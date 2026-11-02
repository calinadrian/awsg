//! The directory serializes transactions. Two alternating, synced snapshots preserve
//! the previous generation if a write is interrupted, on Windows and Unix.
use crate::economy::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_GOLD: u64 = 1_000_000_000_000;
#[derive(Clone, Default, Serialize, Deserialize)]
struct Account {
    #[serde(default)]
    owner: Option<String>,
    inventory: Inventory,
    receipts: BTreeMap<String, (String, Option<String>)>,
    session: Option<(String, String, u32, u32)>,
    #[serde(default)]
    session_equipment: u32,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct State {
    generation: u64,
    serial: u64,
    accounts: BTreeMap<String, Account>,
    listings: Vec<Listing>,
}
pub struct Store {
    path: PathBuf,
    state: State,
    catalog: Catalog,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn secret() -> String {
    format!(
        "{}{}",
        tungstenite::handshake::client::generate_key(),
        tungstenite::handshake::client::generate_key()
    )
}
fn random(limit: u32) -> u32 {
    if limit == 0 {
        return 0;
    }
    let key = secret();
    let value = key
        .bytes()
        .fold(0u64, |v, c| v.wrapping_mul(257).wrapping_add(u64::from(c)));
    (value % u64::from(limit)) as u32
}
impl Store {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let mut states = Vec::new();
        let mut existed = false;
        for slot in [0, 1] {
            let p = path.with_extension(format!("{slot}.json"));
            match std::fs::read(&p) {
                Ok(bytes) => {
                    existed = true;
                    if let Ok(state) = serde_json::from_slice::<State>(&bytes) {
                        states.push(state);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Cannot read economy store: {e}")),
            }
        }
        if existed && states.is_empty() {
            return Err("Both economy snapshots are unreadable; restore a backup".into());
        }
        let state = states
            .into_iter()
            .max_by_key(|s| s.generation)
            .unwrap_or_default();
        Ok(Self {
            path,
            state,
            catalog: Catalog::load(),
        })
    }
    fn commit(&mut self, mut state: State) -> Result<(), String> {
        state.generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or("Store generation exhausted")?;
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let path = self
            .path
            .with_extension(format!("{}.json", state.generation % 2));
        let bytes = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Could not save transaction: {e}"))?;
        #[cfg(unix)]
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
        }
        self.state = state;
        Ok(())
    }
    pub fn handle(&mut self, request: Request) -> Reply {
        match self.apply(&request) {
            Ok(reply) => reply,
            Err(message) => Reply {
                id: request.id,
                retry: message.starts_with("Could not save transaction:"),
                message,
                ..Reply::default()
            },
        }
    }
    fn apply(&mut self, req: &Request) -> Result<Reply, String> {
        if req.id.is_empty() || req.id.len() > 100 {
            return Err("Invalid transaction ID".into());
        }
        let mut next = self.state.clone();
        let expired: Vec<_> = next
            .listings
            .iter()
            .filter(|l| l.expires <= now())
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
        next.listings.retain(|l| l.expires > now());
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
                let token = secret();
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
        let fingerprint = serde_json::to_string(&req.action).map_err(|e| e.to_string())?;
        let receipt = next.accounts[&token].receipts.get(&req.id).cloned();
        if let Some((old, result)) = receipt {
            if old != fingerprint {
                return Err("Transaction ID reused with different contents".into());
            }
            run = result;
        } else {
            match &req.action {
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
                        let items = std::mem::take(&mut old.inventory.items);
                        let materials = std::mem::take(&mut old.inventory.materials);
                        let gold = std::mem::take(&mut old.inventory.gold);
                        let receipts = old.receipts.clone();
                        let shared = next
                            .accounts
                            .get_mut(&token)
                            .ok_or("Inventory unavailable")?;
                        shared.inventory.gold = shared
                            .inventory
                            .gold
                            .checked_add(gold)
                            .filter(|&v| v <= MAX_GOLD)
                            .ok_or("Combined gold exceeds limit")?;
                        shared.inventory.items.extend(items);
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
                Action::BeginRun { zone } => {
                    if !self.catalog.zones.contains_key(zone) {
                        return Err("No equipment table for this zone".into());
                    }
                    let id = secret();
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    account.session = Some((id.clone(), zone.clone(), 0, 0));
                    account.session_equipment = 0;
                    run = Some(id);
                }
                Action::Reward {
                    run,
                    normal,
                    elite,
                    boss,
                    gold,
                } => {
                    let account = next.accounts.get_mut(&token).ok_or("Hero unavailable")?;
                    let (active, zone, kills, paid_gold) = account
                        .session
                        .as_mut()
                        .ok_or("Start an online run to earn equipment")?;
                    if active != run
                        || *normal > 2000
                        || *elite > 100
                        || *boss > 20
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
                    let zone_id = zone.clone();
                    let table = self.catalog.zones.get(&zone_id).ok_or("Unknown zone")?;
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
                                .filter(|i| i.zone == zone_id && i.can_use(class))
                                .map(|i| (i, d.weight))
                        })
                        .collect();
                    let weight: u32 = pool.iter().map(|(_, w)| *w).sum();
                    for (kind, count) in [("normal", *normal), ("elite", *elite), ("boss", *boss)] {
                        for _ in 0..count {
                            if random(10_000) < table.material_chance_bp[kind]
                                && !table.material_pool.is_empty()
                            {
                                let material = &table.material_pool
                                    [random(table.material_pool.len() as u32) as usize];
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
                                && account.session_equipment < MAX_EQUIPMENT_PER_RUN
                                && weight > 0
                                && account.inventory.items.len() < 5000
                                && random(10_000) < table.equipment_chance_bp[kind]
                            {
                                let mut roll = random(weight);
                                if let Some((definition, _)) = pool.iter().find(|(_, w)| {
                                    if roll < *w {
                                        true
                                    } else {
                                        roll -= w;
                                        false
                                    }
                                }) {
                                    next.serial += 1;
                                    account.inventory.items.push(Item {
                                        id: next.serial,
                                        definition: definition.id.clone(),
                                        ..Item::default()
                                    });
                                    account.session_equipment += 1;
                                }
                            }
                        }
                    }
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
                                expires: now() + 48 * 3600,
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
        if !matches!(req.action, Action::Refresh) || !expired.is_empty() {
            self.commit(next)
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
