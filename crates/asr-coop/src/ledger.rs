//! The economy's rules: every transaction the shared inventory accepts, applied to one state
//! held in memory. A ledger never reads a clock, rolls a die or writes a file itself; its [`Host`]
//! does. So the realm service ([`crate::economy_store`], on disk) and a solo game (in the player's
//! own save) run exactly the same rules, on any target.
use crate::economy::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_GOLD: u64 = 1_000_000_000_000;
#[derive(Clone, Default, Serialize, Deserialize)]
struct Account {
    #[serde(default)]
    offline_devices: BTreeMap<String, Inventory>,
    #[serde(default)]
    owner: Option<String>,
    inventory: Inventory,
    receipts: BTreeMap<String, (String, Option<String>)>,
    session: Option<(String, String, u32, u32)>,
    #[serde(default)]
    session_equipment: u32,
    #[serde(default)]
    session_bosses: Vec<String>,
    /// When the run began, which bounds the time it may claim to have survived.
    #[serde(default)]
    session_start: u64,
    /// Whether the run has slain its zone's final boss.
    #[serde(default)]
    session_won: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    generation: u64,
    serial: u64,
    accounts: BTreeMap<String, Account>,
    listings: Vec<Listing>,
}
impl State {
    /// How many transactions have been committed; the newer of two snapshots is the larger.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// What a ledger needs from wherever it runs.
pub trait Host {
    /// Preserve the pre-migration store before accepting the first offline device.
    fn backup_before_sync(&mut self, _state: &[u8]) -> Result<(), String> { Ok(()) }
    /// Seconds since the Unix epoch.
    fn now(&mut self) -> u64;
    /// A uniform roll in `0..limit`; 0 when `limit` is 0.
    fn random(&mut self, limit: u32) -> u32;
    /// An unguessable hero credential or run id.
    fn secret(&mut self) -> String;
    /// Make a committed state durable before the ledger answers. An error leaves the ledger as
    /// it was, and the reply asks the client to retry.
    fn save(&mut self, generation: u64, state: &[u8]) -> Result<(), String>;
}

pub struct Ledger {
    state: State,
    catalog: Catalog,
}
impl Ledger {
    pub fn delivered_snapshot(&self, credential: &str, delivery: &str) -> Option<Inventory> {
        let account = self.state.accounts.get(credential)?;
        let account = account.owner.as_ref().and_then(|owner| self.state.accounts.get(owner)).unwrap_or(account);
        account.receipts.contains_key(&format!("offline-delivery:{delivery}")).then(|| account.inventory.clone())
    }
    /// Install an acknowledged snapshot locally, retaining credentials, receipts and active runs.
    /// The delivery ID makes recovery after a crash between the two save files idempotent.
    pub fn install_snapshot(&mut self, credential: &str, mut inventory: Inventory, delivery: &str, host: &mut impl Host) -> Result<Inventory, String> {
        let mut next = self.state.clone();
        let owner = next.accounts.get(credential).ok_or("Local inventory credential missing")?.owner.clone().unwrap_or_else(|| credential.into());
        let account = next.accounts.get_mut(&owner).ok_or("Local inventory missing")?;
        let receipt = format!("offline-delivery:{delivery}");
        if !account.receipts.contains_key(&receipt) {
            // The owner credential's class stays fixed; views can request any linked class.
            inventory.class = account.inventory.class;
            inventory.name = account.inventory.name.clone();
            account.inventory = inventory;
            account.receipts.insert(receipt, (String::new(), None));
            // Local rewards use a fresh random ID range, avoiding other devices' sequences.
            next.serial = ((u64::from(host.random(u32::MAX)) << 32) | u64::from(host.random(u32::MAX))) & 0x0FFF_FFFF_FFFF_FFFF;
            self.commit(next, host).map_err(|e| format!("Could not save transaction: {e}"))?;
        }
        Ok(self.state.accounts[&owner].inventory.clone())
    }

    pub fn new(state: State, catalog: Catalog) -> Self {
        Self { state, catalog }
    }
    pub fn handle(&mut self, request: Request, host: &mut impl Host) -> Reply {
        match self.apply(&request, host) {
            Ok(reply) => reply,
            Err(message) => Reply {
                id: request.id,
                retry: message.starts_with("Could not save transaction:"),
                message,
                ..Reply::default()
            },
        }
    }
    fn commit(&mut self, mut state: State, host: &mut impl Host) -> Result<(), String> {
        state.generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or("Store generation exhausted")?;
        let bytes = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
        host.save(state.generation, &bytes)?;
        self.state = state;
        Ok(())
    }
    /// One of the best-quality items these zones drop that `class` can use, at random.
    fn best_item<'a>(
        &self,
        zones: impl Iterator<Item = &'a String>,
        class: u8,
        host: &mut impl Host,
    ) -> Option<&Definition> {
        let mut pool: Vec<_> = zones
            .filter_map(|zone| self.catalog.zones.get(zone))
            .flat_map(|zone| zone.equipment.iter())
            .filter_map(|drop| self.catalog.definition(&drop.item))
            .filter(|item| item.can_use(class))
            .collect();
        let rank = |item: &&Definition| match item.quality.as_str() {
            "epic" => 3,
            "rare" => 2,
            "uncommon" => 1,
            _ => 0,
        };
        let highest = pool.iter().map(rank).max()?;
        pool.retain(|item| rank(item) == highest);
        Some(pool[host.random(pool.len() as u32) as usize])
    }
    fn apply(&mut self, req: &Request, host: &mut impl Host) -> Result<Reply, String> {
        if let Action::Online { device, action } = &req.action {
            if device.is_empty() || device.len() > 100 || !matches!(action.as_ref(), Action::Buy { .. } | Action::List { .. } | Action::Cancel { .. } | Action::Refresh) {
                return Err("Invalid online inventory action".into());
            }
            let inner = Request { id: req.id.clone(), token: req.token.clone(), action: *action.clone() };
            let reply = self.apply(&inner, host)?;
            if let Some(inventory) = &reply.inventory {
                let mut next = self.state.clone();
                let credential = next.accounts.get(&req.token).ok_or("Credential unavailable")?;
                let owner = credential.owner.clone().unwrap_or_else(|| req.token.clone());
                let account = next.accounts.get_mut(&owner).ok_or("Inventory unavailable")?;
                if account.offline_devices.len() >= 64 && !account.offline_devices.contains_key(device) { return Err("Too many offline devices".into()); }
                if account.offline_devices.get(device).is_none_or(|old| !crate::offline::same_inventory(old, inventory)) {
                    account.offline_devices.insert(device.clone(), inventory.clone());
                    self.commit(next, host).map_err(|e| format!("Could not save transaction: {e}"))?;
                }
            }
            return Ok(reply);
        }
        if req.id.is_empty() || req.id.len() > 100 {
            return Err("Invalid transaction ID".into());
        }
        let now = host.now();
        let mut next = self.state.clone();
        let expired: Vec<_> = next
            .listings
            .iter()
            .filter(|l| l.expires <= now)
            .cloned()
            .collect();
        for listing in &expired {
            if let Some(account) = next.accounts.get_mut(&listing.seller) {
                if let Some(item) = account
                    .inventory
                    .items
                    .iter_mut()
                    .find(|i| i.id == listing.item.id)
                {
                    item.listed = false;
                }
            }
        }
        next.listings.retain(|l| l.expires > now);
        let mut run = None;
        let token = if let Action::Register { name, class } = &req.action {
            if !req.token.is_empty() {
                return Err("Already registered".into());
            }
            if ![1, 2, 3, 4, 5, 7, 8, 9, 11].contains(class)
                || !(2..=32).contains(&name.chars().count())
                || name.chars().any(char::is_control)
            {
                return Err("Choose a hero name of 2–32 characters".into());
            }
            if next.accounts.len() >= 100_000 {
                return Err("Hero registration is full".into());
            }
            // Registration is also replay-safe if the reply is lost.
            let key = format!("registration:{}", req.id);
            let existing = next
                .accounts
                .iter()
                .find(|(_, a)| a.receipts.contains_key(&key))
                .map(|(t, _)| t.clone());
            if let Some(token) = existing {
                token
            } else {
                let token = host.secret();
                let mut account = Account {
                    inventory: Inventory {
                        name: name.trim().into(),
                        class: *class,
                        ..Inventory::default()
                    },
                    ..Account::default()
                };
                account
                    .receipts
                    .insert(key, (format!("{name}:{class}"), None));
                next.accounts.insert(token.clone(), account);
                token
            }
        } else {
            if !next.accounts.contains_key(&req.token) {
                return Err("Hero credential not recognized by this server".into());
            }
            req.token.clone()
        };
        let class = next.accounts[&token].inventory.class;
        let credential = token.clone();
        let token = next.accounts[&token].owner.clone().unwrap_or(token);
        let migrated = next
            .accounts
            .get_mut(&token)
            .is_some_and(|account| self.catalog.quests.migrate(&mut account.inventory, now));
        // A sync brings the device's own set, and handing out a second one first would duplicate it.
        let geared = !matches!(req.action, Action::SyncOffline { .. })
            && next.accounts.get_mut(&token).is_some_and(|account| {
                crate::starter::grant(&mut account.inventory, class, &mut next.serial, &self.catalog)
            });
        let migrated = migrated || geared;
        let fingerprint = serde_json::to_string(&req.action).map_err(|e| e.to_string())?;
        let receipt = next.accounts[&token].receipts.get(&req.id).cloned();
        if let Some((old, result)) = receipt {
            if old != fingerprint {
                return Err("Transaction ID reused with different contents".into());
            }
            run = result;
        } else {
            match &req.action {
                Action::SyncOffline { device, base, inventory } => {
                    if !next.accounts[&token].offline_devices.contains_key(device) {
                        let bytes = serde_json::to_vec(&self.state).map_err(|e| e.to_string())?;
                        host.backup_before_sync(&bytes).map_err(|e| format!("Could not save transaction: {e}"))?;
                    }
                    if device.is_empty() || device.len() > 100 { return Err("Invalid offline device identity".into()); }
                    let account = next.accounts.get_mut(&token).ok_or("Inventory unavailable")?;
                    let expected = account.offline_devices.get(device);
                    let empty = Inventory::default();
                    if !crate::offline::same_inventory(base.as_ref(), expected.unwrap_or(&empty))
                        && !crate::offline::same_inventory(base.as_ref(), &account.inventory) {
                        return Err("Sync checkpoint changed; offline progress retained for recovery".into());
                    }
                    if account.offline_devices.len() >= 64 && !account.offline_devices.contains_key(device) { return Err("Too many offline devices".into()); }
                    let mut merged = crate::offline::merge(&account.inventory, base, inventory, &self.catalog)?;
                    self.catalog.quests.earn(&mut merged, now);
                    merged.name = account.inventory.name.clone();
                    merged.class = account.inventory.class;
                    for item in &merged.items {
                        let d = self.catalog.definition(&item.definition).ok_or("Offline item is missing from this server's catalog")?;
                        if item.level > 5 || item.slot.as_ref().is_some_and(|slot| !d.fits(item.equipped_class.unwrap_or(inventory.class), slot)) {
                            return Err("Invalid offline equipment".into());
                        }
                    }
                    next.serial = next.serial.max(merged.items.iter().map(|i| i.id).max().unwrap_or(0));
                    account.inventory = merged.clone();
                    // Checkpoints preserve the requesting class, as replies do.
                    merged.class = class;
                    account.offline_devices.insert(device.clone(), merged);
                }
                Action::LinkInventory { tokens } => {
                    if tokens.len() > 9 {
                        return Err("Too many class inventories".into());
                    }
                    let mut owners = vec![token.clone()];
                    for credential in tokens {
                        let account = next
                            .accounts
                            .get(credential)
                            .ok_or("Class credential not recognized")?;
                        let owner = account.owner.clone().unwrap_or_else(|| credential.clone());
                        if !owners.contains(&owner) {
                            owners.push(owner);
                        }
                    }
                    for owner in &owners {
                        let account = next
                            .accounts
                            .get_mut(owner)
                            .ok_or("Inventory unavailable")?;
                        for item in &mut account.inventory.items {
                            if item.slot.is_some() && item.equipped_class.is_none() {
                                item.equipped_class = Some(account.inventory.class);
                            }
                            if item.equipped_class.zip(item.slot.as_deref()).is_some_and(
                                |(class, slot)| {
                                    self.catalog
                                        .definition(&item.definition)
                                        .is_none_or(|d| !d.fits(class, slot))
                                },
                            ) {
                                item.slot = None;
                                item.equipped_class = None;
                            }
                        }
                    }
                    for owner in owners.iter().filter(|owner| **owner != token) {
                        let old = next
                            .accounts
                            .get_mut(owner)
                            .ok_or("Inventory unavailable")?;
                        self.catalog.quests.migrate(&mut old.inventory, now);
                        let items = std::mem::take(&mut old.inventory.items);
                        let materials = std::mem::take(&mut old.inventory.materials);
                        let gold = std::mem::take(&mut old.inventory.gold);
                        let points = std::mem::take(&mut old.inventory.achievement_points);
                        let total = std::mem::take(&mut old.inventory.achievement_total);
                        let cleared = std::mem::take(&mut old.inventory.cleared_zones);
                        let quests = std::mem::take(&mut old.inventory.quests);
                        let done = std::mem::take(&mut old.inventory.quests_done);
                        let achievements = std::mem::take(&mut old.inventory.achievements);
                        let progress = std::mem::take(&mut old.inventory.achievement_progress);
                        let offline_progress = std::mem::take(&mut old.inventory.offline_progress);
                        let starter_classes = std::mem::take(&mut old.inventory.starter_classes);
                        let offline_devices = std::mem::take(&mut old.offline_devices);
                        let receipts = old.receipts.clone();
                        let shared = next
                            .accounts
                            .get_mut(&token)
                            .ok_or("Inventory unavailable")?;
                        shared.inventory.starter_classes.extend(starter_classes);
                        for (key, value) in offline_progress {
                            let current = shared.inventory.offline_progress.entry(key).or_default();
                            *current = (*current).max(value);
                        }
                        for (device, checkpoint) in offline_devices {
                            shared.offline_devices.entry(device).or_insert(checkpoint);
                        }
                        shared.inventory.gold = shared
                            .inventory
                            .gold
                            .checked_add(gold)
                            .filter(|&v| v <= MAX_GOLD)
                            .ok_or("Combined gold exceeds limit")?;
                        shared.inventory.items.extend(items);
                        // An achievement both earned was paid twice; both cleared zones are
                        // migrated conquests by now, so this also covers the old first-clear pay.
                        let overlap = self.catalog.quests.overlap(&achievements, &shared.inventory.achievements);
                        shared.inventory.achievement_points = shared.inventory.achievement_points.saturating_add(points).saturating_sub(overlap);
                        shared.inventory.achievement_total = shared.inventory.achievement_total.saturating_add(total).saturating_sub(overlap);
                        shared.inventory.cleared_zones.extend(cleared);
                        for (id, when) in achievements {
                            let entry = shared.inventory.achievements.entry(id).or_insert(when);
                            *entry = (*entry).min(when);
                        }
                        for (id, values) in progress {
                            if shared.inventory.achievements.contains_key(&id) {
                                continue;
                            }
                            let mine = shared.inventory.achievement_progress.entry(id).or_default();
                            mine.resize(mine.len().max(values.len()), 0);
                            for (a, b) in mine.iter_mut().zip(values) {
                                *a = (*a).max(b);
                            }
                        }
                        let earned = shared.inventory.achievements.clone();
                        shared.inventory.achievement_progress.retain(|id, _| !earned.contains_key(id));
                        for (id, times) in done {
                            let entry = shared.inventory.quests_done.entry(id).or_default();
                            *entry = (*entry).max(times);
                        }
                        for quest in quests {
                            match shared.inventory.quests.iter_mut().find(|q| q.id == quest.id) {
                                Some(mine) => {
                                    mine.progress.resize(mine.progress.len().max(quest.progress.len()), 0);
                                    for (a, b) in mine.progress.iter_mut().zip(quest.progress) {
                                        *a = (*a).max(b);
                                    }
                                }
                                None => shared.inventory.quests.push(quest),
                            }
                        }
                        for (material, count) in materials {
                            let value = shared.inventory.materials.entry(material).or_default();
                            *value = value
                                .checked_add(count)
                                .ok_or("Combined materials exceed limit")?;
                        }
                        for (id, receipt) in receipts {
                            if !id.starts_with("registration:") {
                                shared.receipts.entry(id).or_insert(receipt);
                            }
                        }
                        for listing in &mut next.listings {
                            if listing.seller == *owner {
                                listing.seller = token.clone();
                            }
                        }
                    }
                    for (credential, account) in &mut next.accounts {
                        let owner = account.owner.as_ref().unwrap_or(credential);
                        if owners.contains(owner) && *credential != token {
                            account.owner = Some(token.clone());
                        }
                    }
                    let mut occupied = std::collections::BTreeSet::new();
                    for item in &mut next
                        .accounts
                        .get_mut(&token)
                        .ok_or("Inventory unavailable")?
                        .inventory
                        .items
                    {
                        if let (Some(class), Some(slot)) = (item.equipped_class, item.slot.as_ref())
                        {
                            if !occupied.insert((class, slot.clone())) {
                                item.slot = None;
                                item.equipped_class = None;
                            }
                        }
                    }
                }
                Action::Register { .. } | Action::Refresh => {}
                Action::Buy { listing } => {
                    let index = next
                        .listings
                        .iter()
                        .position(|l| l.id == *listing)
                        .ok_or("Listing already sold, cancelled or expired")?;
                    let sale = next.listings[index].clone();
                    if sale.seller == token {
                        return Err("Use Cancel to return your own listing".into());
                    }
                    let buyer = &next.accounts[&token].inventory;
                    if buyer.gold < sale.price {
                        return Err("Not enough gold".into());
                    }
                    if buyer.items.len() >= 5000 {
                        return Err("Your bags are full".into());
                    }
                    let seller = next
                        .accounts
                        .get_mut(&sale.seller)
                        .ok_or("Seller unavailable")?;
                    if seller.inventory.gold > MAX_GOLD - sale.price {
                        return Err("Seller gold limit reached".into());
                    }
                    let index = seller
                        .inventory
                        .items
                        .iter()
                        .position(|i| i.id == sale.item.id && i.listed)
                        .ok_or("Listed item unavailable")?;
                    let mut item = seller.inventory.items.remove(index);
                    seller.inventory.gold += sale.price;
                    item.listed = false;
                    item.locked = false;
                    let buyer = next.accounts.get_mut(&token).ok_or("Buyer unavailable")?;
                    buyer.inventory.gold -= sale.price;
                    buyer.inventory.items.push(item);
                    next.listings.retain(|l| l.id != *listing);
                }
                Action::Cancel { listing } => {
                    let sale = next
                        .listings
                        .iter()
                        .find(|l| l.id == *listing && l.seller == token)
                        .ok_or("Your listing was not found")?
                        .clone();
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    if let Some(item) = account
                        .inventory
                        .items
                        .iter_mut()
                        .find(|i| i.id == sale.item.id)
                    {
                        item.listed = false;
                    }
                    next.listings.retain(|l| l.id != *listing);
                }
                Action::RedeemAchievement { gear } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let price = if *gear { 30 } else { 10 };
                    if account.inventory.achievement_points < price {
                        return Err("Not enough achievement points".into());
                    }
                    if *gear {
                        if account.inventory.items.len() >= 5000 { return Err("Inventory is full".into()); }
                        let item = self
                            .best_item(account.inventory.cleared_zones.iter(), class, host)
                            .ok_or("Clear a zone first")?;
                        next.serial += 1;
                        account.inventory.items.push(Item { id: next.serial, definition: item.id.clone(), ..Item::default() });
                    } else {
                        for material in ["cloth", "leather", "iron", "wood"] {
                            let amount = account.inventory.materials.entry(material.into()).or_default();
                            *amount = amount.saturating_add(20);
                        }
                    }
                    account.inventory.achievement_points -= price;
                }
                Action::BeginRun { zone } => {
                    if !self.catalog.zones.contains_key(zone) {
                        return Err("No equipment table for this zone".into());
                    }
                    let id = host.secret();
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    account.session = Some((id.clone(), zone.clone(), 0, 0));
                    account.session_equipment = 0;
                    account.session_bosses.clear();
                    account.session_start = now;
                    account.session_won = false;
                    run = Some(id);
                }
                Action::AcceptQuest { quest } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let def = self.catalog.quests.offer(&account.inventory, quest)?;
                    account.inventory.quests.push(crate::quests::QuestState {
                        id: def.id.clone(),
                        progress: vec![0; def.objectives.len()],
                    });
                }
                Action::AbandonQuest { quest } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let index = account
                        .inventory
                        .quests
                        .iter()
                        .position(|q| q.id == *quest)
                        .ok_or("That quest is not in your log")?;
                    account.inventory.quests.remove(index);
                }
                Action::TurnInQuest { quest } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let index = account
                        .inventory
                        .quests
                        .iter()
                        .position(|q| q.id == *quest)
                        .ok_or("That quest is not in your log")?;
                    let def = self.catalog.quests.quest(quest).ok_or("That quest is no longer offered")?;
                    if !self.catalog.quests.complete(&account.inventory.quests[index]) {
                        return Err("Finish every objective first".into());
                    }
                    let item = if def.reward.item {
                        if account.inventory.items.len() >= 5000 {
                            return Err("Your bags are full".into());
                        }
                        self.best_item(std::iter::once(&def.zone), class, host)
                    } else {
                        None
                    };
                    let inventory = &mut account.inventory;
                    inventory.quests.remove(index);
                    *inventory.quests_done.entry(def.id.clone()).or_default() += 1;
                    inventory.gold = inventory.gold.saturating_add(def.reward.gold).min(MAX_GOLD);
                    inventory.achievement_points = inventory.achievement_points.saturating_add(def.reward.points);
                    inventory.achievement_total = inventory.achievement_total.saturating_add(def.reward.points);
                    if def.reward.materials > 0 {
                        for material in self.catalog.zones.get(&def.zone).map_or(&[][..], |z| &z.material_pool[..]) {
                            let amount = inventory.materials.entry(material.clone()).or_default();
                            *amount = amount.saturating_add(def.reward.materials);
                        }
                    }
                    if let Some(item) = item {
                        next.serial += 1;
                        inventory.items.push(Item { id: next.serial, definition: item.id.clone(), ..Item::default() });
                    }
                }
                Action::Reward {
                    run,
                    normal,
                    elite,
                    boss,
                    boss_names,
                    gold,
                    progress,
                } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let started = account.session_start;
                    let (active, zone, kills, paid_gold) = account
                        .session
                        .as_mut()
                        .ok_or("Start a run to earn equipment")?;
                    if active != run
                        || *normal > 2000
                        || *elite > 100
                        || *boss > 20
                        || boss_names.len() > *boss as usize
                        || *gold > 100_000
                    {
                        return Err("Invalid run reward batch".into());
                    }
                    let count = normal.saturating_add(*elite).saturating_add(*boss);
                    if kills.saturating_add(count) > 100_000
                        || paid_gold.saturating_add(*gold) > 1_000_000
                    {
                        return Err("Run reward limit reached".into());
                    }
                    *kills += count;
                    *paid_gold += gold;
                    let run_gold = *paid_gold;
                    let zone_id = zone.clone();
                    let table = self.catalog.zones.get(&zone_id).ok_or("Unknown zone")?;
                    if boss_names.iter().any(|name| {
                        !table.boss_names().any(|entry| entry == name)
                    }) {
                        return Err("Unknown boss loot entry".into());
                    }
                    let has_named_bosses = !boss_names.is_empty();
                    let fresh_boss_names: Vec<_> = boss_names.iter()
                        .filter(|name| !account.session_bosses.contains(name))
                        .cloned()
                        .collect();
                    account.session_bosses.extend(fresh_boss_names.iter().cloned());
                    account.inventory.gold = account
                        .inventory
                        .gold
                        .saturating_add(u64::from(*gold))
                        .min(MAX_GOLD);
                    let pool: Vec<_> = table
                        .equipment
                        .iter()
                        .filter_map(|d| {
                            self.catalog
                                .definition(&d.item)
                                .filter(|i| i.can_use(class))
                                .map(|i| (i, d.weight))
                        })
                        .collect();
                    let weight: u32 = pool.iter().map(|(_, w)| *w).sum();
                    let mut boss_names = fresh_boss_names.iter();
                    for (kind, count) in [("normal", *normal), ("elite", *elite), ("boss", *boss)] {
                        for _ in 0..count {
                            let boss_name = if kind == "boss" { boss_names.next() } else { None };
                            if host.random(10_000) < table.material_chance_bp[kind]
                                && !table.material_pool.is_empty()
                            {
                                let material = &table.material_pool
                                    [host.random(table.material_pool.len() as u32) as usize];
                                let entry = account
                                    .inventory
                                    .materials
                                    .entry(material.clone())
                                    .or_default();
                                *entry = entry.saturating_add(table.materials[kind]);
                            }
                            // Ordinary enemies never award equipment; their row remains in the
                            // shared table for materials, while elites and bosses use this zone's pool.
                            if kind != "normal"
                                && weight > 0
                                && account.inventory.items.len() < 5000
                                && !(kind == "boss" && has_named_bosses && boss_name.is_none())
                                && host.random(10_000) < table.equipment_chance_bp[kind]
                            {
                                let selected_boss_item = boss_name
                                    .and_then(|name| table.boss_drops.iter().find(|entry| entry.boss == *name))
                                    .and_then(|entry| self.catalog.definition(&entry.item));
                                let named_boss_drop = selected_boss_item.is_some();
                                if !named_boss_drop && account.session_equipment >= MAX_EQUIPMENT_PER_RUN {
                                    continue;
                                }
                                let random_pool_item = || {
                                    let mut roll = host.random(weight);
                                    pool.iter().find(|(_, w)| {
                                        if roll < *w { true } else { roll -= w; false }
                                    }).map(|(definition, _)| *definition)
                                };
                                if let Some(definition) = selected_boss_item.or_else(random_pool_item) {
                                    next.serial += 1;
                                    account.inventory.items.push(Item {
                                        id: next.serial,
                                        definition: definition.id.clone(),
                                        ..Item::default()
                                    });
                                    if !named_boss_drop {
                                        account.session_equipment += 1;
                                    }
                                }
                            }
                        }
                    }
                    let final_slain = table.boss_names().next_back().is_some_and(|boss| account.session_bosses.iter().any(|name| name == boss));
                    let won_now = final_slain && !account.session_won;
                    account.session_won |= final_slain;
                    if final_slain && !account.inventory.cleared_zones.contains(&zone_id) {
                        account.inventory.cleared_zones.insert(zone_id.clone());
                        // The zone's conquest achievement pays the first clear's points below.
                        if self.catalog.quests.conquest(&zone_id).is_none() {
                            account.inventory.achievement_points = account.inventory.achievement_points.saturating_add(10);
                        }
                        account.inventory.gold = account.inventory.gold.saturating_add(100).min(MAX_GOLD);
                        for material in &table.material_pool {
                            let amount = account.inventory.materials.entry(material.clone()).or_default();
                            *amount = amount.saturating_add(5);
                        }
                        if let Some((item, _)) = pool.first() {
                            next.serial += 1;
                            account.inventory.items.push(Item { id: next.serial, definition: item.id.clone(), ..Item::default() });
                        }
                    }
                    // Progress is bounded by what this batch paid for: no more kills than it
                    // counted, no longer than the run has lasted, a party only for a clear.
                    // Bosses and the clear come from the ledger's own count.
                    let survived = now.saturating_sub(started).saturating_mul(11) / 10 + 30;
                    let (mut normal_left, mut elite_left) = (*normal, *elite);
                    let mut ticks = Vec::new();
                    for tick in progress.iter().take(crate::quests::MAX_TICKS) {
                        use crate::quests::Kind;
                        let n = match tick.kind {
                            Kind::Kill => {
                                let n = tick.n.min(normal_left);
                                normal_left -= n;
                                n
                            }
                            Kind::Elite => {
                                let n = tick.n.min(elite_left);
                                elite_left -= n;
                                n
                            }
                            Kind::Survive => tick.n.min(u32::try_from(survived).unwrap_or(u32::MAX)),
                            Kind::Level => tick.n.min(99),
                            Kind::Gold => tick.n.min(run_gold),
                            Kind::Party if account.session_won => tick.n.min(crate::MAX_PLAYERS as u32),
                            Kind::Party | Kind::Boss | Kind::Clear => 0,
                        };
                        if n > 0 {
                            ticks.push(crate::quests::Tick {
                                kind: tick.kind,
                                target: tick.target.chars().take(64).collect(),
                                n,
                            });
                        }
                    }
                    ticks.extend(fresh_boss_names.iter().map(|name| crate::quests::Tick {
                        kind: crate::quests::Kind::Boss,
                        target: name.clone(),
                        n: 1,
                    }));
                    if won_now {
                        ticks.push(crate::quests::Tick { kind: crate::quests::Kind::Clear, target: String::new(), n: 1 });
                    }
                    self.catalog.quests.progress(&mut account.inventory, &zone_id, class, &ticks, now);
                }
                action => {
                    let item_id = match action {
                        Action::Equip { item, .. }
                        | Action::Lock { item, .. }
                        | Action::Upgrade { item }
                        | Action::Scrap { item }
                        | Action::List { item, .. } => *item,
                        _ => return Err("Unsupported action".into()),
                    };
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let index = account
                        .inventory
                        .items
                        .iter()
                        .position(|i| i.id == item_id)
                        .ok_or("Item is no longer in your bags")?;
                    let item = account.inventory.items[index].clone();
                    let definition = self
                        .catalog
                        .definition(&item.definition)
                        .ok_or("Unknown item definition")?;
                    if item.listed {
                        return Err("Cancel the auction before changing this item".into());
                    }
                    match action {
                        Action::Equip { slot, .. } => {
                            if item.slot.is_some()
                                && item.equipped_class.is_some_and(|owner| owner != class)
                            {
                                return Err("Unequip this item on its other class first".into());
                            }
                            if let Some(slot) = slot {
                                if !definition.fits(class, slot) || !SLOTS.contains(&slot.as_str())
                                {
                                    return Err(
                                        "Your class cannot equip this item in that slot".into()
                                    );
                                }
                                for other in &mut account.inventory.items {
                                    if !other.equipped_for(class) {
                                        continue;
                                    }
                                    let two_handed = self
                                        .catalog
                                        .definition(&other.definition)
                                        .is_some_and(|d| d.two_handed);
                                    if other.slot.as_ref() == Some(slot)
                                        || (definition.two_handed
                                            && other.slot.as_deref() == Some("off_hand"))
                                        || (slot == "off_hand" && two_handed)
                                    {
                                        other.slot = None;
                                        other.equipped_class = None;
                                    }
                                }
                            }
                            account.inventory.items[index].slot = slot.clone();
                            account.inventory.items[index].equipped_class =
                                slot.as_ref().map(|_| class);
                        }
                        Action::Lock { locked, .. } => {
                            account.inventory.items[index].locked = *locked
                        }
                        Action::Upgrade { .. } => {
                            if item.locked {
                                return Err("Unlock the item before upgrading".into());
                            }
                            if definition.is_starter() {
                                return Err("Starter gear cannot be upgraded".into());
                            }
                            let recipe = definition
                                .upgrades
                                .get(usize::from(item.level))
                                .ok_or("Already upgraded to +5")?;
                            for (material, cost) in &recipe.cost {
                                if account
                                    .inventory
                                    .materials
                                    .get(material)
                                    .copied()
                                    .unwrap_or(0)
                                    < *cost
                                {
                                    return Err(format!("Not enough {material}"));
                                }
                            }
                            for (material, cost) in &recipe.cost {
                                *account
                                    .inventory
                                    .materials
                                    .entry(material.clone())
                                    .or_default() -= cost;
                            }
                            account.inventory.items[index].level += 1;
                        }
                        Action::Scrap { .. } => {
                            if item.locked || item.slot.is_some() {
                                return Err("Unequip and unlock before scrapping".into());
                            }
                            for (material, amount) in &definition.scrap {
                                let entry = account
                                    .inventory
                                    .materials
                                    .entry(material.clone())
                                    .or_default();
                                *entry = entry.saturating_add(*amount + u32::from(item.level));
                            }
                            account.inventory.items.remove(index);
                        }
                        Action::List { price, .. } => {
                            if definition.is_starter() {
                                return Err("Starter gear cannot be sold".into());
                            }
                            if *price == 0
                                || *price > MAX_GOLD
                                || item.locked
                                || item.slot.is_some()
                            {
                                return Err(
                                    "Unequip and unlock the item, and choose a positive gold price"
                                        .into(),
                                );
                            }
                            if next.listings.iter().filter(|l| l.seller == token).count() >= 20 {
                                return Err("You can list up to 20 items".into());
                            }
                            next.serial += 1;
                            next.listings.push(Listing {
                                id: next.serial,
                                seller: token.clone(),
                                seller_name: account.inventory.name.clone(),
                                item,
                                price: *price,
                                expires: now + 48 * 3600,
                            });
                            account.inventory.items[index].listed = true;
                        }
                        _ => return Err("Unsupported item action".into()),
                    }
                }
            }
            if !matches!(req.action, Action::Refresh) {
                next.accounts
                    .get_mut(&token)
                    .ok_or("Hero unavailable")?
                    .receipts
                    .insert(req.id.clone(), (fingerprint, run.clone()));
            }
        }
        if !matches!(req.action, Action::Refresh) || !expired.is_empty() || migrated {
            self.commit(next, host)
                .map_err(|e| format!("Could not save transaction: {e}"))?;
        }
        let mut inventory = self
            .state
            .accounts
            .get(&token)
            .ok_or("Hero unavailable")?
            .inventory
            .clone();
        inventory.class = class;
        let listings = self
            .state
            .listings
            .iter()
            .filter(|l| l.seller == token)
            .chain(self.state.listings.iter().filter(|l| l.seller != token))
            .take(1000)
            .cloned()
            .map(|mut l| {
                // Never publish another player's bearer credential.
                l.seller = if l.seller == token {
                    "self".into()
                } else {
                    String::new()
                };
                l
            })
            .collect();
        Ok(Reply {
            id: req.id.clone(),
            token: matches!(req.action, Action::Register { .. }).then_some(credential),
            inventory: Some(inventory),
            listings,
            run,
            message: "Saved".into(),
            ok: true,
            retry: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ledger host in memory: a fixed clock, dice that always roll low, and a save that can
    /// be made to fail.
    #[derive(Default)]
    struct Memory {
        rolls: u64,
        saved: Vec<u64>,
        fail: bool,
    }
    impl Host for Memory {
        fn now(&mut self) -> u64 {
            1_700_000_000
        }
        fn random(&mut self, _limit: u32) -> u32 {
            self.rolls += 1;
            0
        }
        fn secret(&mut self) -> String {
            self.rolls += 1;
            format!("secret-{}", self.rolls)
        }
        fn save(&mut self, generation: u64, _state: &[u8]) -> Result<(), String> {
            if self.fail {
                return Err("Could not save transaction: disk full".into());
            }
            self.saved.push(generation);
            Ok(())
        }
    }

    fn send(ledger: &mut Ledger, host: &mut Memory, id: &str, token: &str, action: Action) -> Reply {
        ledger.handle(
            Request {
                id: id.into(),
                token: token.into(),
                action,
            },
            host,
        )
    }

    /// A zone with a final boss, and that boss's name.
    fn zone(catalog: &Catalog) -> (String, String) {
        catalog
            .zones
            .iter()
            .find_map(|(id, zone)| Some((id.clone(), zone.boss_drops.last()?.boss.clone())))
            .expect("the catalog has a zone with bosses")
    }

    #[test]
    fn a_solo_run_pays_drops_and_the_first_clear() {
        let catalog = Catalog::load();
        let (zone, boss) = zone(&catalog);
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let hero = send(&mut ledger, &mut host, "r", "", Action::Register { name: "Solo".into(), class: 9 });
        assert!(hero.ok, "{}", hero.message);
        let token = hero.token.expect("a credential");
        let begun = send(&mut ledger, &mut host, "b", &token, Action::BeginRun { zone: zone.clone() });
        let run = begun.run.expect("a run id");
        let reward = Action::Reward {
            run,
            normal: 10,
            elite: 2,
            boss: 1,
            boss_names: vec![boss],
            gold: 40,
            progress: Vec::new(),
        };
        let paid = send(&mut ledger, &mut host, "p", &token, reward.clone());
        assert!(paid.ok, "{}", paid.message);
        let bags = paid.inventory.expect("the bags");
        assert!(bags.cleared_zones.contains(&zone));
        assert_eq!(bags.achievement_points, 10);
        assert!(bags.gold >= 140, "run gold plus the first-clear purse");
        assert!(!bags.items.is_empty(), "elite, boss and first-clear equipment");
        assert!(!bags.materials.is_empty());
        // The same transaction again, as after a lost reply, pays nothing twice.
        let again = send(&mut ledger, &mut host, "p", &token, reward);
        let replayed = again.inventory.expect("the bags");
        assert_eq!(replayed.gold, bags.gold);
        assert_eq!(replayed.items.len(), bags.items.len());
    }

    #[test]
    fn rewards_need_a_run_and_a_failed_save_changes_nothing() {
        let catalog = Catalog::load();
        let (zone, _) = zone(&catalog);
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let token = send(&mut ledger, &mut host, "r", "", Action::Register { name: "Solo".into(), class: 1 })
            .token
            .expect("a credential");
        let unbegun = send(&mut ledger, &mut host, "x", &token, Action::Reward {
            run: String::new(),
            normal: 1,
            elite: 0,
            boss: 0,
            boss_names: Vec::new(),
            gold: 5,
            progress: Vec::new(),
        });
        assert!(!unbegun.ok);
        host.fail = true;
        let failed = send(&mut ledger, &mut host, "b", &token, Action::BeginRun { zone: zone.clone() });
        assert!(!failed.ok && failed.retry);
        host.fail = false;
        let generation = ledger.state.generation();
        let retried = send(&mut ledger, &mut host, "b", &token, Action::BeginRun { zone });
        assert!(retried.ok, "{}", retried.message);
        assert_eq!(ledger.state.generation(), generation + 1);
    }

    #[test]
    fn saved_state_reads_back() {
        let catalog = Catalog::load();
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        send(&mut ledger, &mut host, "r", "", Action::Register { name: "Solo".into(), class: 4 });
        let bytes = serde_json::to_vec(&ledger.state).expect("serializes");
        let back: State = serde_json::from_slice(&bytes).expect("reads back");
        assert_eq!(back.generation(), ledger.state.generation());
        assert_eq!(back.accounts.len(), 1);
    }

    fn hero(ledger: &mut Ledger, host: &mut Memory, class: u8) -> String {
        send(ledger, host, "register", "", Action::Register { name: "Quester".into(), class })
            .token
            .expect("a credential")
    }
    fn tick(kind: crate::quests::Kind, target: &str, n: u32) -> crate::quests::Tick {
        crate::quests::Tick { kind, target: target.into(), n }
    }
    /// A reward batch for the hero's current run.
    fn reward(normal: u32, elite: u32, boss_names: Vec<String>, progress: Vec<crate::quests::Tick>) -> Action {
        Action::Reward {
            run: String::new(),
            normal,
            elite,
            boss: boss_names.len() as u32,
            boss_names,
            gold: 0,
            progress,
        }
    }
    fn begin(ledger: &mut Ledger, host: &mut Memory, token: &str, id: &str, zone: &str) -> String {
        send(ledger, host, id, token, Action::BeginRun { zone: zone.into() }).run.expect("a run")
    }
    fn with_run(action: Action, run: &str) -> Action {
        match action {
            Action::Reward { normal, elite, boss, boss_names, gold, progress, .. } => {
                Action::Reward { run: run.into(), normal, elite, boss, boss_names, gold, progress }
            }
            other => other,
        }
    }

    #[test]
    fn quests_are_accepted_advanced_and_turned_in() {
        use crate::quests::Kind;
        let catalog = Catalog::load();
        let story = catalog.quests.quest("elwynn.story").expect("the Elwynn story").clone();
        let first = story.objectives[0].target.clone();
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let token = hero(&mut ledger, &mut host, 1);

        let finale = send(&mut ledger, &mut host, "a0", &token, Action::AcceptQuest { quest: "elwynn.finale".into() });
        assert!(!finale.ok && finale.message.contains(&story.title), "{}", finale.message);
        let accepted = send(&mut ledger, &mut host, "a1", &token, Action::AcceptQuest { quest: "elwynn.story".into() });
        assert!(accepted.ok, "{}", accepted.message);
        let twice = send(&mut ledger, &mut host, "a2", &token, Action::AcceptQuest { quest: "elwynn.story".into() });
        assert!(!twice.ok);

        // Progress needs a run in the quest's zone; other zones' kills do not count.
        let run = begin(&mut ledger, &mut host, &token, "b0", "westfall");
        let elsewhere = send(&mut ledger, &mut host, "p0", &token, with_run(reward(200, 0, vec![], vec![tick(Kind::Kill, "Coyote", 200)]), &run));
        assert!(elsewhere.ok, "{}", elsewhere.message);
        assert_eq!(elsewhere.inventory.expect("bags").quests[0].progress, vec![0, 0]);

        let run = begin(&mut ledger, &mut host, &token, "b1", "elwynn");
        // Claims beyond the batch's own kill count are cut to it.
        let paid = send(&mut ledger, &mut host, "p1", &token, with_run(reward(100, 0, vec![first.clone()], vec![tick(Kind::Kill, "Kobold Vermin", 500)]), &run));
        assert!(paid.ok, "{}", paid.message);
        assert_eq!(paid.inventory.expect("bags").quests[0].progress, vec![1, 100]);
        let early = send(&mut ledger, &mut host, "t0", &token, Action::TurnInQuest { quest: "elwynn.story".into() });
        assert!(!early.ok);
        send(&mut ledger, &mut host, "p2", &token, with_run(reward(80, 0, vec![], vec![tick(Kind::Kill, "", 80)]), &run));
        let done = send(&mut ledger, &mut host, "t1", &token, Action::TurnInQuest { quest: "elwynn.story".into() });
        assert!(done.ok, "{}", done.message);
        let bags = done.inventory.expect("bags");
        assert!(bags.quests.is_empty());
        assert_eq!(bags.quests_done.get("elwynn.story"), Some(&1));
        assert_eq!(bags.gold, story.reward.gold);
        let again = send(&mut ledger, &mut host, "a3", &token, Action::AcceptQuest { quest: "elwynn.story".into() });
        assert!(!again.ok, "not repeatable");
        let finale = send(&mut ledger, &mut host, "a4", &token, Action::AcceptQuest { quest: "elwynn.finale".into() });
        assert!(finale.ok, "{}", finale.message);
        let dropped = send(&mut ledger, &mut host, "d0", &token, Action::AbandonQuest { quest: "elwynn.finale".into() });
        assert!(dropped.ok && dropped.inventory.expect("bags").quests.is_empty());
    }

    #[test]
    fn the_log_holds_five_and_bounties_repeat() {
        use crate::quests::{Kind, MAX_ACTIVE};
        let catalog = Catalog::load();
        let offered: Vec<_> = catalog.quests.quests.iter().filter(|q| q.requires.is_empty()).map(|q| q.id.clone()).collect();
        let bounty = catalog.quests.quest("elwynn.bounty").expect("a bounty").clone();
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let token = hero(&mut ledger, &mut host, 9);
        let mut taken = vec![bounty.id.clone()];
        assert!(send(&mut ledger, &mut host, "a", &token, Action::AcceptQuest { quest: bounty.id.clone() }).ok);
        for quest in offered.iter().filter(|q| **q != bounty.id).take(MAX_ACTIVE - 1) {
            assert!(send(&mut ledger, &mut host, &format!("a{quest}"), &token, Action::AcceptQuest { quest: quest.clone() }).ok);
            taken.push(quest.clone());
        }
        let extra = offered.iter().find(|q| !taken.contains(q)).expect("more quests").clone();
        let full = send(&mut ledger, &mut host, "full", &token, Action::AcceptQuest { quest: extra });
        assert!(!full.ok && full.message.contains("full"), "{}", full.message);

        let target = bounty.objectives[0].target.clone();
        let count = bounty.objectives[0].count;
        for round in 0..2u32 {
            let run = begin(&mut ledger, &mut host, &token, &format!("b{round}"), "elwynn");
            send(&mut ledger, &mut host, &format!("p{round}"), &token, with_run(reward(count, 0, vec![], vec![tick(Kind::Kill, &target, count)]), &run));
            let paid = send(&mut ledger, &mut host, &format!("t{round}"), &token, Action::TurnInQuest { quest: bounty.id.clone() });
            assert!(paid.ok, "{}", paid.message);
            assert_eq!(paid.inventory.expect("bags").quests_done.get(&bounty.id), Some(&(round + 1)));
            if round == 0 {
                assert!(send(&mut ledger, &mut host, "again", &token, Action::AcceptQuest { quest: bounty.id.clone() }).ok);
            }
        }
    }

    #[test]
    fn a_clear_earns_its_conquest_and_bounds_hold() {
        use crate::quests::Kind;
        let catalog = Catalog::load();
        let bosses: Vec<_> = catalog.zones["elwynn"].boss_drops.iter().map(|b| b.boss.clone()).collect();
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let token = hero(&mut ledger, &mut host, 4);
        let run = begin(&mut ledger, &mut host, &token, "b", "elwynn");
        // A party is only believed for a clear, and survival only for the time the run has had.
        let early = send(&mut ledger, &mut host, "p0", &token, with_run(reward(0, 0, vec![], vec![tick(Kind::Party, "", 4), tick(Kind::Survive, "", 5000)]), &run));
        let bags = early.inventory.expect("bags");
        assert!(!bags.achievements.keys().any(|id| id.starts_with("party.")));
        assert!(bags.achievement_progress.get("survive.600").is_none_or(|v| v[0] <= 30));
        let cleared = send(&mut ledger, &mut host, "p1", &token, with_run(reward(0, 0, bosses, vec![tick(Kind::Party, "", 9)]), &run));
        assert!(cleared.ok, "{}", cleared.message);
        let bags = cleared.inventory.expect("bags");
        for id in ["conquer.elwynn", "wanted.elwynn", "party.2", "party.3", "party.4"] {
            assert!(bags.achievements.contains_key(id), "{id}");
        }
        assert_eq!(bags.achievement_points, bags.achievement_total);
        assert_eq!(bags.achievement_total, 10 + 10 + 10 + 10 + 15);
        // Another batch from the same run does not clear it again.
        let replay = send(&mut ledger, &mut host, "p2", &token, with_run(reward(0, 0, vec![], vec![]), &run));
        assert_eq!(replay.inventory.expect("bags").achievement_progress.get("clears.10"), Some(&vec![1]));
    }

    #[test]
    fn old_requests_and_heroes_still_read() {
        let old: Action = serde_json::from_str(r#"{"Reward":{"run":"x","normal":1,"elite":0,"boss":0,"gold":0}}"#).expect("an old reward");
        assert!(matches!(old, Action::Reward { ref progress, .. } if progress.is_empty()));
        let hero: Inventory = serde_json::from_str(r#"{"achievement_points":20,"cleared_zones":["elwynn"],"name":"Old","class":1,"gold":5,"materials":{},"items":[]}"#).expect("an old hero");
        assert!(hero.quests.is_empty() && hero.achievements.is_empty() && hero.quest_rev == 0);
    }

    #[test]
    fn linking_two_heroes_pays_shared_achievements_once() {
        let catalog = Catalog::load();
        let bosses: Vec<_> = catalog.zones["elwynn"].boss_drops.iter().map(|b| b.boss.clone()).collect();
        let mut ledger = Ledger::new(State::default(), catalog);
        let mut host = Memory::default();
        let mut tokens = Vec::new();
        for (i, class) in [1u8, 8].into_iter().enumerate() {
            let token = send(&mut ledger, &mut host, &format!("r{i}"), "", Action::Register { name: "Twin".into(), class }).token.expect("a credential");
            let run = begin(&mut ledger, &mut host, &token, &format!("b{i}"), "elwynn");
            send(&mut ledger, &mut host, &format!("p{i}"), &token, with_run(reward(0, 0, bosses.clone(), vec![]), &run));
            tokens.push(token);
        }
        let linked = send(&mut ledger, &mut host, "link", &tokens[0], Action::LinkInventory { tokens: vec![tokens[1].clone()] });
        assert!(linked.ok, "{}", linked.message);
        let bags = linked.inventory.expect("bags");
        assert_eq!(bags.achievement_total, 20, "conquest and most-wanted, once");
        assert_eq!(bags.achievement_points, 20);
    }
}
