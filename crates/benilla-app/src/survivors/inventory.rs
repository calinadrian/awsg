//! Native inventory client, with cached gear and a durable transaction outbox.
use super::{coop::Coop, Phase, Run, Selection};
use asr_coop::{
    economy::{Action, Catalog, Inventory, Listing, Request},
    Client, Server,
};
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
}
#[derive(Resource)]
pub(super) struct Equipment {
    pub catalog: Catalog,
    save: Save,
    pub listings: Vec<Listing>,
    pub status: String,
    pub revision: u64,
    pub open: bool,
    pub market: bool,
    pub search: String,
    pub search_active: bool,
    pub search_cursor: super::text_edit::Cursor,
    pub show_loot: bool,
    pub show_journal: bool,
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
    #[cfg(not(target_arch = "wasm32"))]
    connection: Option<std::sync::Mutex<asr_coop::transport::Connection>>,
    server: String,
    connected: bool,
    flight: bool,
    timer: f32,
    refresh_t: f32,
    run_key: String,
    counted: [u32; 3],
    boss_counted: usize,
    reward_t: f32,
    finished: bool,
    read_only: bool,
    link_failed: bool,
}
fn path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|p| p.join("survivors-inventory.json"))
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
            market: false,
            search: String::new(),
            search_active: false,
            search_cursor: Default::default(),
            show_loot: false,
            show_journal: false,
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
            #[cfg(not(target_arch = "wasm32"))]
            connection: None,
            server: String::new(),
            connected: false,
            flight: false,
            timer: 0.,
            refresh_t: 0.,
            run_key: String::new(),
            counted: [0; 3],
            boss_counted: 0,
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
    pub fn busy(&self) -> bool {
        self.read_only
    }
    pub fn online(&self) -> bool {
        self.connected
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
    fn rewards(&mut self, run: &Run, gold: u32) {
        let delta = std::array::from_fn::<_, 3, _>(|i| {
            run.equipment_kills[i].saturating_sub(self.counted[i])
        });
        let boss_names = run.boss_loot.iter().skip(self.boss_counted).cloned().collect::<Vec<_>>();
        if delta != [0; 3] || gold > 0 || !boss_names.is_empty() {
            self.queue(
                self.run_key.clone(),
                Action::Reward {
                    run: String::new(),
                    normal: delta[0],
                    elite: delta[1],
                    boss: delta[2],
                    boss_names,
                    gold,
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
    gear.ensure_hero(&coop.url, run.class.id, &coop.name);
    gear.run_key = Equipment::key(&coop.url, run.class.id);
    gear.counted = [0; 3];
    gear.boss_counted = 0;
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
        gear.action(&coop.url, run.class.id, Action::BeginRun { zone });
    } else {
        gear.finished = true;
        gear.status = "Equipment drops are not available for this map".into();
    }
    if let Some(hero) = gear.view(&coop.url, run.class.id) {
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
        gear.rewards(&run, 0);
    }
}
fn finish(run: Res<Run>, mut gear: ResMut<Equipment>) {
    if !gear.finished {
        gear.rewards(&run, run.gold);
        gear.finished = true;
    }
}
fn settle_exit(mut exits: MessageReader<AppExit>, run: Res<Run>, mut gear: ResMut<Equipment>) {
    if exits.read().next().is_some() && !gear.finished {
        gear.rewards(&run, run.gold);
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
    if gear.open {
        gear.ensure_hero(&coop.url, class, &coop.name);
    }
    if gear.read_only {
        return;
    }
    gear.timer += time.delta_secs();
    gear.refresh_t += time.delta_secs();
    #[cfg(not(target_arch = "wasm32"))]
    {
        if gear.server != coop.url {
            gear.connection = None;
            gear.connected = false;
            gear.flight = false;
            gear.server = coop.url.clone();
            gear.listings.clear();
            gear.link_failed = false;
        }
        let relevant = gear.save.pending.iter().any(|j| {
            j.key
                .starts_with(&format!("{}:", coop.url.trim_end_matches('/')))
        });
        if gear.connection.is_none() && (gear.open || relevant) && gear.timer >= 3. {
            gear.timer = 0.;
            gear.connection = Some(std::sync::Mutex::new(asr_coop::transport::connect(
                coop.url.clone(),
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
                    if reply.ok {
                        let prefix = format!("{}:", coop.url.trim_end_matches('/'));
                        let linked = if let Action::LinkInventory { tokens } = &job.request.action {
                            gear.save.linked.insert(prefix.clone(), tokens.join("|"));
                            true
                        } else {
                            gear.save.linked.contains_key(&prefix)
                                && !matches!(job.request.action, Action::Register { .. })
                        };
                        let shared = reply.inventory.clone();
                        let hero = gear.save.heroes.entry(job.key).or_default();
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
                                for (key, hero) in &mut gear.save.heroes {
                                    if key.starts_with(&prefix) {
                                        let class = hero.inventory.class;
                                        hero.inventory = inventory.clone();
                                        hero.inventory.class = class;
                                    }
                                }
                            }
                        }
                        gear.listings = reply.listings;
                        gear.status = "Inventory synchronized".into();
                    } else {
                        if matches!(job.request.action, Action::LinkInventory { .. }) {
                            gear.link_failed = true;
                        }
                        gear.status = reply.message;
                    }
                    gear.persist();
                    gear.revision += 1;
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
            gear.link_classes(&coop.url, class);
            if gear.save.pending.is_empty() && gear.open && gear.refresh_t > 5. {
                gear.refresh_t = 0.;
                gear.action(&coop.url, class, Action::Refresh);
            }
            if let Some(index) = gear.save.pending.iter().position(|j| {
                j.key
                    .starts_with(&format!("{}:", coop.url.trim_end_matches('/')))
            }) {
                if index != 0 {
                    let job = gear.save.pending.remove(index);
                    gear.save.pending.insert(0, job);
                }
                let key = gear.save.pending[0].key.clone();
                if !matches!(gear.save.pending[0].request.action, Action::Register { .. }) {
                    if let Some(hero) = gear.save.heroes.get(&key).cloned() {
                        gear.save.pending[0].request.token = hero.token;
                        if let Action::Reward { run, .. } = &mut gear.save.pending[0].request.action
                        {
                            if run.is_empty() {
                                *run = hero.run;
                            }
                        }
                    } else {
                        gear.status =
                            "Missing hero credential; restore survivors-inventory.json from backup"
                                .into();
                        return;
                    }
                }
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
    #[cfg(target_arch = "wasm32")]
    if gear.open {
        gear.status = "Shared inventory currently requires the desktop build".into();
    }
}
