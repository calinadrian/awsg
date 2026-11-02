//! The inventory client, with cached gear and a durable transaction outbox. The outbox drains
//! into one of two economies with the same rules ([`asr_coop::ledger`]): the realm at the co-op
//! server's address, or the player's own solo ledger, which needs no server at all.
use super::{coop::Coop, Phase, Run, Selection};
use asr_coop::{
    economy::{Action, Catalog, Inventory, Listing, Reply, Request},
    ledger::{Host, Ledger, State as LedgerState},
};
#[cfg(not(target_arch = "wasm32"))]
use asr_coop::{Client, Server};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct HeroSave {
    pub token: String,
    pub inventory: Inventory,
    pub run: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Job {
    key: String,
    request: Request,
}
#[derive(Default, Serialize, Deserialize)]
struct Save {
    heroes: BTreeMap<String, HeroSave>,
    pending: Vec<Job>,
    #[serde(default)]
    linked: BTreeMap<String, String>,
    /// Whether the player chose the solo economy; unset, a player with no realm heroes is solo.
    #[serde(default)]
    solo: Option<bool>,
}
/// The solo economy's address in hero keys: `solo:<class>`.
pub(super) const SOLO: &str = "solo";
/// The account window's open tab.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash)]
pub(super) enum Pane {
    #[default]
    Inventory,
    Auction,
    Drops,
    Quests,
    Achievements,
}
#[derive(Resource)]
pub(super) struct Equipment {
    pub catalog: Catalog,
    save: Save,
    pub listings: Vec<Listing>,
    pub status: String,
    pub revision: u64,
    pub open: bool,
    pub pane: Pane,
    pub search: String,
    pub search_active: bool,
    pub search_cursor: super::text_edit::Cursor,
    pub loot_class: Option<u8>,
    pub loot_slot: Option<usize>,
    pub loot_quality: usize,
    pub loot_source: usize,
    pub selected: Option<u64>,
    pub listing: Option<u64>,
    pub filter: usize,
    /// An index into `SLOTS`: the bags and the market show only items for that slot.
    pub slot_filter: Option<usize>,
    pub price: u64,
    pub confirm: Option<Action>,
    /// The quest open in the quest log.
    pub quest: Option<String>,
    /// The economy zone the quest log offers quests from; all the hero's zones when `None`.
    pub quest_zone: Option<String>,
    /// The achievement category shown: 0 for the summary, then one per category.
    pub category: usize,
    #[cfg(not(target_arch = "wasm32"))]
    connection: Option<std::sync::Mutex<asr_coop::transport::Connection>>,
    /// The solo economy, read from its save the first time it is needed.
    ledger: Option<Ledger>,
    server: String,
    connected: bool,
    flight: bool,
    timer: f32,
    refresh_t: f32,
    run_key: String,
    counted: [u32; 3],
    boss_counted: usize,
    /// The run's kill tally as of the last reward batch.
    tallied: std::collections::BTreeMap<String, [u32; 2]>,
    reward_t: f32,
    finished: bool,
    read_only: bool,
    link_failed: bool,
}
fn path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|p| p.join("survivors-inventory.json"))
}
/// The solo ledger's state: every solo hero, their bags and their run sessions.
fn solo_path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|p| p.join("survivors-solo-economy.json"))
}
/// The solo ledger's clock, dice and save.
struct SoloHost {
    seed: u64,
}
impl SoloHost {
    fn new() -> Self {
        let nanos = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self { seed: nanos ^ 0x5DEE_CE66_D1CE_4E5B }
    }
    /// splitmix64: plenty for loot rolls nobody else has to trust.
    fn next(&mut self) -> u64 {
        self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
impl Host for SoloHost {
    fn now(&mut self) -> u64 {
        web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
    fn random(&mut self, limit: u32) -> u32 {
        if limit == 0 {
            return 0;
        }
        (self.next() % u64::from(limit)) as u32
    }
    fn secret(&mut self) -> String {
        format!("{:016x}{:016x}", self.next(), self.next())
    }
    fn save(&mut self, _generation: u64, state: &[u8]) -> Result<(), String> {
        let path = solo_path().ok_or("Could not save transaction: no persistent save folder")?;
        crate::local_state::write_atomic_bytes(&path, state)
            .map_err(|e| format!("Could not save transaction: {e}"))
    }
}
fn id() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        asr_coop::transport::request_id()
    }
    #[cfg(target_arch = "wasm32")]
    {
        format!(
            "{}",
            web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        )
    }
}
impl Default for Equipment {
    fn default() -> Self {
        let loaded = path().and_then(|p| match crate::local_state::read_to_string(&p) {
            Ok(text) => Some(serde_json::from_str::<Save>(&text).map_err(|e| e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => Some(Err(e.to_string())),
        });
        let (save, read_only, status) = match loaded {
            Some(Ok(save)) => (save, false, "Inventory ready".into()),
            Some(Err(error)) => (
                Save::default(),
                true,
                format!("Inventory save could not be read; preserve it for recovery: {error}"),
            ),
            None => (
                Save::default(),
                false,
                "Play to earn equipment, materials and gold".into(),
            ),
        };
        Self {
            catalog: Catalog::load(),
            save,
            listings: Vec::new(),
            status,
            revision: 0,
            open: false,
            pane: Pane::Inventory,
            search: String::new(),
            search_active: false,
            search_cursor: Default::default(),
            loot_class: None,
            loot_slot: None,
            loot_quality: 0,
            loot_source: 0,
            selected: None,
            listing: None,
            filter: 0,
            slot_filter: None,
            price: 10,
            confirm: None,
            quest: None,
            quest_zone: None,
            category: 0,
            #[cfg(not(target_arch = "wasm32"))]
            connection: None,
            ledger: None,
            server: String::new(),
            connected: false,
            flight: false,
            timer: 0.,
            refresh_t: 0.,
            run_key: String::new(),
            counted: [0; 3],
            boss_counted: 0,
            tallied: Default::default(),
            reward_t: 0.,
            finished: true,
            read_only,
            link_failed: false,
        }
    }
}
impl Equipment {
    pub fn key(server: &str, class: u8) -> String {
        format!("{}:{class}", server.trim_end_matches('/'))
    }
    /// Whether this player keeps their bags in the solo economy. The web build has no realm
    /// connection, so it is always solo there.
    pub fn solo(&self) -> bool {
        cfg!(target_arch = "wasm32")
            || self.save.solo.unwrap_or_else(|| {
                let prefix = format!("{SOLO}:");
                self.save.heroes.keys().all(|key| key.starts_with(&prefix))
            })
    }
    /// The economy this player's bags and rewards live in: the solo ledger, or the realm at the
    /// co-op server's address. A co-op room always plays on the realm its players share.
    pub fn realm(&self, coop: &Coop) -> String {
        if self.solo() && !coop.in_room() {
            SOLO.into()
        } else {
            coop.url.clone()
        }
    }
    pub fn set_solo(&mut self, solo: bool) {
        if self.save.solo != Some(solo) {
            self.save.solo = Some(solo);
            self.persist();
            self.listings.clear();
            self.selected = None;
            self.listing = None;
            self.revision += 1;
        }
    }
    /// An economy's answer to the outbox's first job for `server`, solo or realm alike.
    fn receive(&mut self, server: &str, job: Job, reply: Reply) {
        if reply.ok {
            let prefix = format!("{}:", server.trim_end_matches('/'));
            let linked = if let Action::LinkInventory { tokens } = &job.request.action {
                self.save.linked.insert(prefix.clone(), tokens.join("|"));
                true
            } else {
                self.save.linked.contains_key(&prefix)
                    && !matches!(job.request.action, Action::Register { .. })
            };
            let shared = reply.inventory.clone();
            let hero = self.save.heroes.entry(job.key).or_default();
            if let Some(token) = reply.token {
                hero.token = token;
            }
            if let Some(inventory) = reply.inventory {
                hero.inventory = inventory;
            }
            if let Some(run) = reply.run {
                hero.run = run;
            }
            if linked {
                if let Some(inventory) = shared {
                    for (key, hero) in &mut self.save.heroes {
                        if key.starts_with(&prefix) {
                            let class = hero.inventory.class;
                            hero.inventory = inventory.clone();
                            hero.inventory.class = class;
                        }
                    }
                }
            }
            self.listings = reply.listings;
            self.status = if server == SOLO {
                "Solo inventory saved".into()
            } else {
                "Inventory synchronized".into()
            };
        } else {
            if matches!(job.request.action, Action::LinkInventory { .. }) {
                self.link_failed = true;
            }
            self.status = reply.message;
        }
        self.persist();
        self.revision += 1;
    }
    /// The job's credential and run, from the hero the job is for; `None` for a hero this save
    /// has no credential for.
    fn signed(&self, job: &Job) -> Option<Request> {
        let mut request = job.request.clone();
        if matches!(request.action, Action::Register { .. }) {
            return Some(request);
        }
        let hero = self.save.heroes.get(&job.key)?;
        request.token = hero.token.clone();
        if let Action::Reward { run, .. } = &mut request.action {
            if run.is_empty() {
                *run = hero.run.clone();
            }
        }
        Some(request)
    }
    /// Drain the solo jobs through the solo ledger, at once: there is no one to wait for.
    fn solo_tick(&mut self, class: u8) {
        self.server = SOLO.into();
        self.connected = true;
        if self.ledger.is_none() {
            let loaded = solo_path().map(|p| crate::local_state::read_to_string(&p));
            let state = match loaded {
                Some(Ok(text)) => match serde_json::from_str::<LedgerState>(&text) {
                    Ok(state) => state,
                    Err(error) => {
                        self.status = format!("Solo save could not be read; preserve it for recovery: {error}");
                        self.read_only = true;
                        self.revision += 1;
                        return;
                    }
                },
                Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => {
                    self.status = format!("Solo save could not be read: {e}");
                    self.read_only = true;
                    self.revision += 1;
                    return;
                }
                _ => LedgerState::default(),
            };
            self.ledger = Some(Ledger::new(state, Catalog::load()));
        }
        self.link_classes(SOLO, class);
        let Some(mut ledger) = self.ledger.take() else { return };
        let mut host = SoloHost::new();
        let prefix = format!("{SOLO}:");
        for _ in 0..32 {
            let Some(index) = self.save.pending.iter().position(|j| j.key.starts_with(&prefix)) else {
                break;
            };
            let Some(request) = self.signed(&self.save.pending[index]) else {
                self.status = "Missing solo hero credential; restore survivors-inventory.json from backup".into();
                break;
            };
            let reply = ledger.handle(request, &mut host);
            if reply.retry {
                self.status = reply.message;
                self.revision += 1;
                break;
            }
            let job = self.save.pending.remove(index);
            self.receive(SOLO, job, reply);
        }
        self.ledger = Some(ledger);
    }
    pub fn hero(&self, server: &str, class: u8) -> Option<&HeroSave> {
        self.save.heroes.get(&Self::key(server, class)).or_else(|| {
            let prefix = format!("{}:", server.trim_end_matches('/'));
            self.save
                .heroes
                .iter()
                .find(|(key, _)| key.starts_with(&prefix))
                .map(|(_, hero)| hero)
        })
    }
    pub fn view(&self, server: &str, class: u8) -> Option<HeroSave> {
        let mut hero = self.hero(server, class)?.clone();
        let original_class = hero.inventory.class;
        for item in &mut hero.inventory.items {
            if item.slot.is_some() && item.equipped_class.is_none() {
                item.equipped_class = Some(original_class);
            }
        }
        hero.inventory.class = class;
        Some(hero)
    }
    /// The hero's active quests in `zone` as the in-run tracker shows them, with this run's
    /// unsent kills folded in, and the achievements the hero holds.
    pub fn journal(
        &self,
        coop: &Coop,
        run: &Run,
        zone: &str,
    ) -> Option<(Vec<super::quests::Line>, &std::collections::BTreeMap<String, u64>)> {
        let hero = self.hero(&self.realm(coop), run.class.id)?;
        let lines = super::quests::live(&self.catalog, &hero.inventory, zone, run.class.id, run, &self.tallied, self.boss_counted);
        Some((lines, &hero.inventory.achievements))
    }
    pub fn busy(&self) -> bool {
        self.read_only
    }
    pub fn online(&self) -> bool {
        self.connected
    }
    /// Whether an accept, abandon or turn-in for this quest is still waiting on the ledger.
    pub fn quest_busy(&self, quest: &str) -> bool {
        self.read_only
            || self.save.pending.iter().any(|job| match &job.request.action {
                Action::AcceptQuest { quest: pending }
                | Action::AbandonQuest { quest: pending }
                | Action::TurnInQuest { quest: pending } => pending == quest,
                _ => false,
            })
    }
    pub fn item_busy(&self, item: u64) -> bool {
        self.read_only
            || self
                .save
                .pending
                .iter()
                .any(|job| match job.request.action {
                    Action::Equip { item: pending, .. }
                    | Action::Lock { item: pending, .. }
                    | Action::Upgrade { item: pending }
                    | Action::Scrap { item: pending }
                    | Action::List { item: pending, .. } => pending == item,
                    _ => false,
                })
    }
    fn link_classes(&mut self, server: &str, class: u8) {
        if self.link_failed {
            return;
        }
        let prefix = format!("{}:", server.trim_end_matches('/'));
        if self.save.pending.iter().any(|job| {
            job.key.starts_with(&prefix)
                && matches!(
                    job.request.action,
                    Action::Register { .. } | Action::LinkInventory { .. }
                )
        }) {
            return;
        }
        let tokens: Vec<_> = self
            .save
            .heroes
            .iter()
            .filter(|(key, hero)| key.starts_with(&prefix) && !hero.token.is_empty())
            .map(|(_, hero)| hero.token.clone())
            .collect();
        if !tokens.is_empty()
            && self.save.heroes.contains_key(&Self::key(server, class))
            && self.save.linked.get(&prefix) != Some(&tokens.join("|"))
        {
            self.action(server, class, Action::LinkInventory { tokens });
            // Linking must precede unsent item actions against another class's former bags.
            if let Some(job) = self.save.pending.pop() {
                let index = self
                    .save
                    .pending
                    .iter()
                    .rposition(|pending| {
                        matches!(
                            pending.request.action,
                            Action::Register { .. }
                                | Action::BeginRun { .. }
                                | Action::Reward { .. }
                        )
                    })
                    .map_or(0, |i| i + 1);
                self.save.pending.insert(index, job);
            }
            self.persist();
        }
    }
    fn persist(&mut self) -> bool {
        let result = path()
            .ok_or_else(|| "No persistent save folder".to_string())
            .and_then(|p| {
                let text = serde_json::to_string(&self.save).map_err(|e| e.to_string())?;
                crate::local_state::write_atomic(&p, &text).map_err(|e| e.to_string())
            });
        if let Err(error) = result {
            self.status = format!("Could not save inventory: {error}");
            self.read_only = true;
            false
        } else {
            true
        }
    }
    fn queue(&mut self, key: String, action: Action) {
        if self.read_only {
            return;
        }
        self.save.pending.push(Job {
            key,
            request: Request {
                id: id(),
                token: String::new(),
                action,
            },
        });
        self.persist();
        self.revision += 1;
    }
    pub fn action(&mut self, server: &str, class: u8, action: Action) {
        if matches!(action, Action::Refresh) {
            self.link_failed = false;
        }
        self.queue(Self::key(server, class), action);
    }
    fn ensure_hero(&mut self, server: &str, class: u8, name: &str) {
        let key = Self::key(server, class);
        if !self.save.heroes.contains_key(&key)
            && !self
                .save
                .pending
                .iter()
                .any(|j| j.key == key && matches!(j.request.action, Action::Register { .. }))
        {
            let name = if name.trim().chars().count() >= 2 {
                name.trim()
            } else {
                "Hero"
            };
            self.queue(
                key,
                Action::Register {
                    name: name.chars().take(32).collect(),
                    class,
                },
            );
        }
    }
    /// Queue what the run has earned since the last batch; `end` is the party size of a run
    /// that is over, which also reports its gold and best-of progress.
    fn rewards(&mut self, run: &Run, gold: u32, end: Option<u32>) {
        let delta = std::array::from_fn::<_, 3, _>(|i| {
            run.equipment_kills[i].saturating_sub(self.counted[i])
        });
        let boss_names = run.boss_loot.iter().skip(self.boss_counted).cloned().collect::<Vec<_>>();
        let mut progress = super::quests::kill_ticks(run, &mut self.tallied);
        if delta != [0; 3] || gold > 0 || !boss_names.is_empty() || !progress.is_empty() || end.is_some() {
            progress.extend(super::quests::run_ticks(run));
            if let Some(party) = end {
                use asr_coop::quests::{Kind, Tick};
                progress.push(Tick { kind: Kind::Gold, target: String::new(), n: gold });
                if run.won && party > 1 {
                    progress.push(Tick { kind: Kind::Party, target: String::new(), n: party });
                }
            }
            self.queue(
                self.run_key.clone(),
                Action::Reward {
                    run: String::new(),
                    normal: delta[0],
                    elite: delta[1],
                    boss: delta[2],
                    boss_names,
                    gold,
                    progress,
                },
            );
            self.counted = run.equipment_kills;
            self.boss_counted = run.boss_loot.len();
        }
    }
}
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Equipment>()
        .add_systems(PreUpdate, network)
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            begin.before(super::talents::begin_run),
        )
        .add_systems(Update, rewards.after(super::coop::revive_teammates))
        .add_systems(OnEnter(Phase::GameOver), finish)
        .add_systems(Last, settle_exit)
        .add_systems(OnEnter(Phase::Menu), finish.before(super::reset_run));
}
fn begin(
    mut gear: ResMut<Equipment>,
    mut run: ResMut<Run>,
    coop: Res<Coop>,
    selection: Res<Selection>,
) {
    gear.open = false;
    run.spell_damage.clear();
    let url = gear.realm(&coop);
    gear.ensure_hero(&url, run.class.id, &coop.name);
    gear.run_key = Equipment::key(&url, run.class.id);
    gear.counted = [0; 3];
    gear.boss_counted = 0;
    gear.tallied.clear();
    gear.reward_t = 0.;
    gear.finished = false;
    let zone_name = selection.stage_def().name;
    if let Some(zone) = gear
        .catalog
        .zones
        .iter()
        .find(|(_, z)| z.name == zone_name)
        .map(|(id, _)| id.clone())
    {
        gear.action(&url, run.class.id, Action::BeginRun { zone });
    } else {
        gear.finished = true;
        gear.status = "Equipment drops are not available for this map".into();
    }
    if let Some(hero) = gear.view(&url, run.class.id) {
        run.gear_look = hero
            .inventory
            .items
            .iter()
            .filter(|item| item.equipped_for(run.class.id))
            .filter_map(|item| {
                let slot = item.slot.as_ref()?;
                let d = gear.catalog.definition(&item.definition)?;
                if !d.fits(run.class.id, slot) {
                    return None;
                }
                Some(asr_coop::economy::LookItem {
                    slot: slot.clone(),
                    display: d.display_id,
                    category: d.category.clone(),
                })
            })
            .collect();
        let mut inventory = hero.inventory.clone();
        inventory.class = run.class.id;
        run.spell_damage = inventory
            .spell_damage_bonuses(&gear.catalog)
            .into_iter()
            .map(|(spell, value)| (spell, value as f32 / 10_000.))
            .collect();
        let bonuses = inventory.bonuses(&gear.catalog);
        run.gear = std::array::from_fn(|i| {
            let key = [
                "max_health",
                "damage_bp",
                "cooldown_reduction_bp",
                "critical_chance_bp",
                "damage_reduction_bp",
            ][i];
            bonuses.get(key).copied().unwrap_or(0) as f32 / if i == 0 { 1. } else { 10_000. }
        });
    }
}
fn rewards(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    mut gear: ResMut<Equipment>,
) {
    if gear.finished || !matches!(phase.get(), Phase::Playing | Phase::Paused | Phase::LevelUp) {
        return;
    }
    gear.reward_t += time.delta_secs();
    if gear.reward_t >= 10. || run.boss_loot.len() > gear.boss_counted {
        gear.reward_t = 0.;
        gear.rewards(&run, 0, None);
    }
}
/// How many players a run that ends now had.
fn party(coop: &Coop) -> u32 {
    coop.room.as_ref().map_or(1, |room| room.members.len() as u32)
}
fn finish(run: Res<Run>, coop: Res<Coop>, mut gear: ResMut<Equipment>) {
    if !gear.finished {
        gear.rewards(&run, run.gold, Some(party(&coop)));
        gear.finished = true;
    }
}
fn settle_exit(mut exits: MessageReader<AppExit>, run: Res<Run>, coop: Res<Coop>, mut gear: ResMut<Equipment>) {
    if exits.read().next().is_some() && !gear.finished {
        gear.rewards(&run, run.gold, Some(party(&coop)));
        gear.finished = true;
    }
}
fn network(
    time: Res<Time<Real>>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    mut gear: ResMut<Equipment>,
) {
    let Some(class) = selection.class_def().map(|c| c.id) else {
        return;
    };
    let url = gear.realm(&coop);
    if gear.open {
        gear.ensure_hero(&url, class, &coop.name);
    }
    if gear.read_only {
        return;
    }
    if url == SOLO {
        gear.solo_tick(class);
        return;
    }
    gear.timer += time.delta_secs();
    gear.refresh_t += time.delta_secs();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let prefix = format!("{}:", url.trim_end_matches('/'));
        if gear.server != url {
            gear.connection = None;
            gear.connected = false;
            gear.flight = false;
            gear.server = url.clone();
            gear.listings.clear();
            gear.link_failed = false;
        }
        let relevant = gear.save.pending.iter().any(|j| j.key.starts_with(&prefix));
        if gear.connection.is_none() && (gear.open || relevant) && gear.timer >= 3. {
            gear.timer = 0.;
            gear.connection = Some(std::sync::Mutex::new(asr_coop::transport::connect(
                url.clone(),
            )));
            gear.status = "Connecting to shared inventory and auctions…".into();
            gear.revision += 1;
        }
        let events: Vec<_> = gear
            .connection
            .as_ref()
            .and_then(|c| c.lock().ok())
            .map_or_else(Vec::new, |c| c.recv.try_iter().take(64).collect());
        for event in events {
            match event {
                Ok(Server::Welcome) => {
                    gear.connected = true;
                    gear.timer = 0.;
                }
                Ok(Server::Economy { reply }) => {
                    if !gear.flight
                        || gear
                            .save
                            .pending
                            .first()
                            .is_none_or(|j| j.request.id != reply.id)
                    {
                        continue;
                    }
                    if reply.retry {
                        gear.status = reply.message;
                        gear.connection = None;
                        gear.connected = false;
                        gear.flight = false;
                        gear.timer = 0.;
                        gear.revision += 1;
                        continue;
                    }
                    let job = gear.save.pending.remove(0);
                    gear.flight = false;
                    gear.receive(&url, job, reply);
                }
                Err(reason) | Ok(Server::Error { reason }) => {
                    gear.status = format!("Shared economy unavailable: {reason}. Pending transactions are saved for retry.");
                    gear.connection = None;
                    gear.connected = false;
                    gear.flight = false;
                    gear.timer = 0.;
                    gear.revision += 1;
                }
                _ => {}
            }
        }
        if gear.flight && gear.timer > 15. {
            gear.connection = None;
            gear.connected = false;
            gear.flight = false;
            gear.timer = 0.;
            gear.status = "No reply from economy service; retrying the saved transaction".into();
            gear.revision += 1;
        }
        if gear.connected && !gear.flight {
            gear.link_classes(&url, class);
            if gear.save.pending.is_empty() && gear.open && gear.refresh_t > 5. {
                gear.refresh_t = 0.;
                gear.action(&url, class, Action::Refresh);
            }
            if let Some(index) = gear.save.pending.iter().position(|j| j.key.starts_with(&prefix)) {
                if index != 0 {
                    let job = gear.save.pending.remove(index);
                    gear.save.pending.insert(0, job);
                }
                let Some(request) = gear.signed(&gear.save.pending[0]) else {
                    gear.status =
                        "Missing hero credential; restore survivors-inventory.json from backup".into();
                    return;
                };
                gear.save.pending[0].request = request;
                if gear.persist() {
                    let request = gear.save.pending[0].request.clone();
                    let sent = gear
                        .connection
                        .as_ref()
                        .and_then(|c| c.lock().ok())
                        .is_some_and(|c| c.send.try_send(Client::Economy { request }).is_ok());
                    gear.flight = sent;
                    gear.timer = 0.;
                }
            } else if gear.timer > 10. {
                if let Some(c) = gear.connection.as_ref().and_then(|c| c.lock().ok()) {
                    let _ = c.send.try_send(Client::List);
                }
                gear.timer = 0.;
            }
        }
    }
}
