//! Player-hosted matches over the operator's directory. Clients own their hero/build;
//! the host owns enemies, the encounter clock, shared pickups and the result.
use super::{
    data,
    enemies::{self, Enemy},
    hero::{self, Hero},
    units::{self, Fx, Marks},
    Phase, Run, Selection,
};
use crate::net::{FieldChanged, Guid, ObjectStore};
use asr_coop::{Client, Member, Room, Server};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const DEFAULT_SERVER: &str = "ws://hassian.duckdns.org:47771";
pub(super) const REVIVE_SECONDS: f32 = 3.0;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Pose {
    pub peer: u32,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub hp: f32,
    pub maximum: f32,
    pub level: u32,
    pub loaded: bool,
    pub paused: bool,
    #[serde(default)]
    pub revival: u32,
    #[serde(default)]
    pub outfit: Vec<asr_coop::economy::LookItem>,
    /// This player's own damage this run, as their client counts it, for the damage meter.
    #[serde(default)]
    pub damage: f64,
}

// Snapshot rows are the bulk of host traffic: short keys, omitted defaults and
// rounded floats keep a 100-enemy snapshot near half its verbose size.
#[derive(Clone, Serialize, Deserialize)]
struct Foe {
    #[serde(rename = "i")]
    id: u64,
    #[serde(rename = "g", default, skip_serializing_if = "is_zero")]
    group: u8,
    #[serde(rename = "n")]
    index: usize,
    #[serde(rename = "p")]
    pos: [f32; 3],
    #[serde(rename = "y")]
    yaw: f32,
    #[serde(rename = "h")]
    hp: f32,
    #[serde(rename = "m")]
    maximum: f32,
    /// Zero while standing.
    #[serde(rename = "s", default, skip_serializing_if = "is_zero_f32")]
    speed: f32,
    #[serde(rename = "d", default, skip_serializing_if = "is_false")]
    dead: bool,
    #[serde(rename = "l", default, skip_serializing_if = "is_false")]
    lair: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Pickup {
    #[serde(rename = "i")]
    id: u64,
    #[serde(rename = "p")]
    pos: [f32; 3],
    #[serde(rename = "k", default, skip_serializing_if = "is_zero")]
    kind: u8,
    #[serde(rename = "a")]
    amount: f32,
}
fn is_zero(v: &u8) -> bool {
    *v == 0
}
fn is_zero_f32(v: &f32) -> bool {
    *v == 0.
}
fn is_false(v: &bool) -> bool {
    !*v
}
/// Rounds to `1 / per_unit`; dividing last keeps the shortest decimal form.
fn round_to(v: f32, per_unit: f32) -> f32 {
    (v * per_unit).round() / per_unit
}
fn round_pos(p: Vec3) -> [f32; 3] {
    [round_to(p.x, 100.), round_to(p.y, 100.), round_to(p.z, 100.)]
}
#[derive(Clone, Serialize, Deserialize)]
struct Visual {
    peer: Option<u32>,
    caster: u64,
    spell: u32,
    hits: Vec<u64>,
    dest: Option<[f32; 3]>,
}
#[derive(Component)]
struct PickupReplica(u64);

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Blow {
    target: u64,
    damage: f32,
    effect: u8,
    strength: f32,
    seconds: f32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Packet {
    Player {
        pose: Pose,
        hits: Vec<Blow>,
        visuals: Vec<Visual>,
        /// Guest packet counter; the host echoes the last one it applied.
        #[serde(default)]
        seq: u32,
    },
    World {
        foes: Vec<Foe>,
        pickups: Vec<Pickup>,
        players: Vec<Pose>,
        #[serde(default, with = "peer_map")]
        revive_progress: HashMap<u32, f32>,
        #[serde(default)]
        equipment_kills: [u32; 3],
        #[serde(default)]
        boss_loot: Vec<String>,
        /// Each guest's last applied `seq`, so guests can drop acknowledged hits.
        #[serde(default, with = "peer_map")]
        acks: HashMap<u32, u32>,
        elapsed: f32,
        kills: u32,
        bosses: u32,
        guardians: u32,
        xp: u64,
        gold: u64,
        caches: Vec<[f32; 3]>,
        damage: Vec<(u32, f32)>,
        visuals: Vec<Visual>,
        warnings: Vec<[f32; 13]>,
        paused: bool,
        ended: bool,
        won: bool,
    },
}

/// A per-peer map as `[peer, value]` pairs. A tagged packet buffers its fields before reading
/// them, and a buffered JSON object key stays a string a `u32` key will not read from, so a map
/// keyed by peer failed every snapshot that held one.
mod peer_map {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize>(
        map: &HashMap<u32, T>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_seq(map.iter())
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        d: D,
    ) -> Result<HashMap<u32, T>, D::Error> {
        Ok(Vec::<(u32, T)>::deserialize(d)?.into_iter().collect())
    }
}

#[derive(Component)]
pub(super) struct Replica(pub u64);
#[derive(Component)]
pub(super) struct Ally(pub u32);

/// Host snapshots and guest poses are sent at these intervals (seconds).
const HOST_TICK: f32 = 1. / 15.;
const GUEST_TICK: f32 = 1. / 20.;
/// Past the newest snapshot, keep moving along its velocity for this long.
const MAX_EXTRAPOLATION: f32 = 0.15;

/// Snapshot interpolation. Each snapshot starts a glide from where the replica is drawn
/// to the reported position, lasting one measured packet interval, so motion stays
/// continuous instead of lurching toward each update and then standing still.
#[derive(Component)]
pub(super) struct ReplicaPose {
    from: Vec3,
    to: Vec3,
    velocity: Vec3,
    yaw: Option<f32>,
    age: f32,
    span: f32,
}

impl ReplicaPose {
    fn new(pos: Vec3, yaw: Option<f32>, span: f32) -> Self {
        Self {
            from: pos,
            to: pos,
            velocity: Vec3::ZERO,
            yaw,
            age: 0.,
            span,
        }
    }
    fn sample(&self) -> Vec3 {
        if self.age < self.span {
            self.from.lerp(self.to, self.age / self.span)
        } else {
            self.to + self.velocity * (self.age - self.span).min(MAX_EXTRAPOLATION)
        }
    }
    fn push(&mut self, pos: Vec3, yaw: Option<f32>) {
        // Smooth the measured interval so TCP clumping cannot make a glide jump.
        let current = self.sample();
        let observed = self.age.clamp(0.03, 0.3);
        self.span += (observed - self.span) * 0.2;
        if current.distance_squared(pos) > 400. {
            // Teleports, respawns and stray relocations snap.
            *self = Self::new(pos, yaw, self.span);
            return;
        }
        self.velocity = ((pos - self.to) / observed).clamp_length_max(30.);
        self.from = current;
        self.to = pos;
        self.yaw = yaw;
        self.age = 0.;
    }
    /// Planar speed being drawn, for run animations.
    fn speed(&self) -> f32 {
        if self.age < self.span {
            units::flat(self.to - self.from).length() / self.span.max(0.03)
        } else if self.age < self.span + MAX_EXTRAPOLATION {
            units::flat(self.velocity).length()
        } else {
            0.
        }
    }
}

#[derive(Resource)]
pub(super) struct Coop {
    #[cfg(not(target_arch = "wasm32"))]
    connection: Option<std::sync::Mutex<asr_coop::transport::Connection>>,
    pub room: Option<Room>,
    pub rooms: Vec<Room>,
    pub peer: u32,
    pub connected: bool,
    pub connecting: bool,
    pub running: bool,
    pub status: String,
    pub open: bool,
    pub url: String,
    pub name: String,
    pub code: String,
    pub private: bool,
    pub players: HashMap<u32, Pose>,
    outgoing: Vec<Blow>,
    incoming: Vec<Blow>,
    visuals: Vec<Visual>,
    replay: Vec<Visual>,
    xp: u64,
    gold: u64,
    caches: Vec<[f32; 3]>,
    seen_caches: usize,
    tick: f32,
    list_t: f32,
    /// Guest: packets sent, and hits the host has not applied yet as (seq, target, damage).
    seq: u32,
    unacked: std::collections::VecDeque<(u32, u64, f32)>,
    /// Host: each guest's last applied packet.
    acks: HashMap<u32, u32>,
    remote_paused: bool,
    since_host: f32,
    revival: u32,
    pub(super) revive_progress: HashMap<u32, f32>,
}

impl Default for Coop {
    fn default() -> Self {
        let saved = crate::local_state::home()
            .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-coop.txt")).ok());
        let mut lines = saved.as_deref().unwrap_or("").lines();
        let saved_url = lines.next().unwrap_or(DEFAULT_SERVER).trim();
        // Earlier installers saved a loopback address on every player's PC.
        // Move those clients to the shared directory while keeping custom servers.
        let saved_url = match saved_url.trim_end_matches('/') {
            "" | "ws://127.0.0.1:47771" | "ws://localhost:47771" | "ws://[::1]:47771" => {
                DEFAULT_SERVER
            }
            _ => saved_url,
        };
        let url = crate::webenv::var("WOW_ASR_COOP_URL").unwrap_or_else(|| saved_url.to_string());
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            connection: None,
            room: None,
            rooms: Vec::new(),
            peer: 0,
            connected: false,
            connecting: false,
            running: false,
            status: "Connect to browse rooms or host a game.".into(),
            open: false,
            url,
            name: lines.next().unwrap_or("Hero").to_string(),
            code: String::new(),
            private: true,
            players: HashMap::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            visuals: Vec::new(),
            replay: Vec::new(),
            xp: 0,
            gold: 0,
            caches: Vec::new(),
            seen_caches: 0,
            tick: 0.,
            list_t: 0.,
            seq: 0,
            unacked: Default::default(),
            acks: HashMap::new(),
            remote_paused: true,
            since_host: 0.,
            revival: 0,
            revive_progress: HashMap::new(),
        }
    }
}

impl Coop {
    pub fn guest(&self) -> bool {
        self.running && self.peer != 0
    }
    pub fn active(&self) -> bool {
        self.running
    }
    pub fn in_room(&self) -> bool {
        self.room.is_some()
    }
    pub fn send(&mut self, message: Client) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(connection) = &self.connection {
            let sent = connection
                .lock()
                .ok()
                .is_some_and(|c| c.send.try_send(message).is_ok());
            if !sent {
                self.status = "Co-op connection is too slow; leave and reconnect.".into();
                self.connected = false;
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = message;
    }
    pub fn connect(&mut self) {
        self.connected = false;
        self.connecting = false;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.connecting = true;
            self.connection = Some(std::sync::Mutex::new(asr_coop::transport::connect(
                self.url.trim().to_string(),
            )));
            self.status = "Connecting...".into();
            if let Some(home) = crate::local_state::home() {
                let _ = crate::local_state::write_atomic(
                    &home.join("survivors-coop.txt"),
                    &format!("{}\n{}", self.url.trim(), self.name.trim()),
                );
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status = "Co-op currently requires the Windows desktop build.".into();
        }
    }
    pub fn member(&self, selection: &Selection) -> Option<Member> {
        let class = selection.class?;
        let &(display, race, sex) = selection.looks.get(selection.look?)?;
        Some(Member {
            peer: 0,
            name: self.name.trim().chars().take(32).collect(),
            hero: asr_coop::Hero {
                class,
                race,
                sex,
                display,
                features: selection.current_features(),
            },
            ready: false,
        })
    }
    pub fn leave(&mut self) {
        self.send(Client::Leave);
        self.running = false;
        self.room = None;
        self.players.clear();
        self.open = true;
    }
    /// Called only when the host collects a shared pickup.
    pub fn reward(&mut self, xp: u32, gold: u32) {
        if self.running && self.peer == 0 {
            self.xp += u64::from(xp);
            self.gold += u64::from(gold);
        }
    }
    pub fn cache(&mut self, pos: Vec3) {
        if self.running && self.peer == 0 {
            self.caches.push(pos.to_array());
        }
    }
    pub fn submit(&mut self, target: u64, damage: f32, effect: data::Effect) {
        if !self.guest() || self.outgoing.len() >= 2048 {
            return;
        }
        let (effect, strength, seconds) = match effect {
            data::Effect::Slow(m, t) => (1, m, t),
            data::Effect::Root(t) => (2, 0., t),
            data::Effect::Stun(t) => (3, 0., t),
            data::Effect::Fear(t) => (4, 0., t),
            data::Effect::Weaken(m, t) => (5, m, t),
            _ => (0, 0., 0.),
        };
        if self.unacked.len() < 4096 {
            self.unacked.push_back((self.seq + 1, target, damage.max(0.)));
        }
        self.outgoing.push(Blow {
            target,
            damage: damage.max(0.),
            effect,
            strength,
            seconds,
        });
    }
    pub fn living_guests(&self) -> bool {
        self.players
            .values()
            .any(|p| p.peer != 0 && p.loaded && p.hp > 0.)
    }
    /// Enemy targeting returns a remote ally only when it is closer than the local hero.
    pub fn target(
        &self,
        from: Vec3,
        local: (Entity, Vec3),
        local_alive: bool,
        allies: &Query<(Entity, &Ally)>,
    ) -> (Entity, Vec3, u32, bool) {
        let local_alive =
            local_alive && (!self.running || self.players.get(&0).is_none_or(|p| !p.paused));
        let mut result = (local.0, local.1, 0, !local_alive);
        let mut distance = if local_alive {
            units::flat(from - local.1).length_squared()
        } else {
            f32::INFINITY
        };
        if self.running {
            for (entity, ally) in allies {
                if let Some(p) = self
                    .players
                    .get(&ally.0)
                    .filter(|p| p.loaded && !p.paused && p.hp > 0.)
                {
                    let pos = Vec3::from_array(p.pos);
                    let d = units::flat(pos - from).length_squared();
                    if d < distance {
                        distance = d;
                        result = (entity, pos, ally.0, false);
                    }
                }
            }
        }
        result
    }
}

pub(super) fn authority(coop: Res<Coop>) -> bool {
    !coop.guest()
}
pub(super) fn world_running(coop: Res<Coop>, phase: Res<State<Phase>>) -> bool {
    *phase.get() == Phase::Playing
        || (coop.running && matches!(phase.get(), Phase::LevelUp | Phase::Paused))
}
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Coop>()
        .add_systems(PreUpdate, receive)
        .add_systems(
            PostUpdate,
            (
                sync_hero_health,
                sync_allies,
                smooth_replicas,
                play_visuals,
                capture_visuals,
                transmit,
                super::resurrection_ui::refresh,
            )
                .chain(),
        );
}

fn resurrect(run: &mut Run) {
    run.dead_t = None;
    run.hp = (run.stats().max_hp * 0.5).max(1.0);
    run.heal_t = 0.0;
    run.immune_t = run.immune_t.max(2.0);
    run.banner = Some(("Resurrected by a teammate".into(), 3.0));
}

/// Original ASR co-op: three seconds within five yards, then half health.
/// Only the host advances the channel and decides a party wipe.
pub(super) fn revive_teammates(
    time: Res<Time>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut coop: ResMut<Coop>,
    mut run: ResMut<Run>,
    heroes: Query<&Transform, With<Hero>>,
) {
    if !coop.active() || coop.guest() {
        return;
    }
    let Ok(tf) = heroes.single() else { return };
    if let Some(local) = coop.players.get_mut(&0) {
        local.hp = run.hp;
        local.maximum = run.stats().max_hp;
        local.pos = tf.translation.to_array();
    }
    // Loading players are not corpses. Wait until every remaining member is known.
    let ready = coop.room.as_ref().is_some_and(|room| {
        room.members
            .iter()
            .all(|m| coop.players.get(&m.peer).is_some_and(|p| p.loaded))
    });
    if !ready {
        return;
    }
    if coop.players.values().all(|p| p.hp <= 0.0) {
        run.won = false;
        next.set(Phase::GameOver);
        coop.revive_progress.clear();
        return;
    }
    // Menus preserve channel progress without advancing it.
    if *phase.get() != Phase::Playing || coop.players.values().any(|p| p.paused) {
        return;
    }
    let fallen: Vec<_> = coop
        .players
        .values()
        .filter(|p| p.loaded && p.hp <= 0.0)
        .map(|p| (p.peer, Vec3::from_array(p.pos)))
        .collect();
    let dead: HashSet<_> = fallen.iter().map(|(peer, _)| *peer).collect();
    coop.revive_progress.retain(|peer, _| dead.contains(peer));
    for (peer, corpse) in fallen {
        let nearby = coop.players.values().any(|p| {
            p.peer != peer
                && p.loaded
                && !p.paused
                && p.hp > 0.0
                && Vec3::from_array(p.pos).distance_squared(corpse) <= 25.0
        });
        if !nearby {
            coop.revive_progress.remove(&peer);
            continue;
        }
        let progress = coop.revive_progress.entry(peer).or_default();
        *progress += time.delta_secs();
        if *progress < REVIVE_SECONDS {
            continue;
        }
        coop.revive_progress.remove(&peer);
        run.coop_damage.retain(|(target, _)| *target != peer);
        if let Some(p) = coop.players.get_mut(&peer) {
            p.revival += 1;
            p.hp = (p.maximum * 0.5).max(1.0);
        }
        if peer == 0 {
            coop.revival = coop.players[&0].revival;
            resurrect(&mut run);
        }
    }
    if run.slain_last {
        run.won = true;
        next.set(Phase::GameOver);
    }
}

fn sync_hero_health(
    run: Res<Run>,
    mut hero: Query<(Entity, &Guid, &mut ObjectStore), With<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
) {
    for (entity, guid, mut store) in &mut hero {
        let hp = if run.dead_t.is_some() {
            0
        } else {
            run.hp.max(0.0).ceil() as u32
        };
        let old = store
            .0
            .raw_fields()
            .find(|(i, _)| *i == 22)
            .map_or(0, |(_, v)| v);
        store
            .0
            .merge(benilla_protocol::messages::ObjectFields::from_pairs(&[
                (22, hp),
                (28, run.stats().max_hp.ceil() as u32),
            ]));
        if old != hp {
            fields.write(FieldChanged {
                entity,
                guid: guid.0,
                kind: benilla_protocol::messages::ObjectType::Unit,
                index: 22,
                old,
                new: hp,
            });
        }
    }
}

fn events(coop: &Coop) -> Vec<Result<Server, String>> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(c) = &coop.connection {
        if let Ok(c) = c.lock() {
            return c.recv.try_iter().take(256).collect();
        }
    }
    Vec::new()
}

fn receive(
    mut commands: Commands,
    mut coop: ResMut<Coop>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    real: Res<Time<Real>>,
    mut clock: ResMut<Time<Virtual>>,
    entry: Res<super::drive::EntryPending>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut foes: Query<(
        Entity,
        &Replica,
        &mut Enemy,
        &mut Transform,
        &mut ObjectStore,
        &mut crate::creature_anim::MovementState,
        Option<&mut ReplicaPose>,
    )>,
    mut lairs: ResMut<super::guardians::Lairs>,
    mut pickups: Query<
        (Entity, &PickupReplica, &mut Transform, Option<&mut ReplicaPose>),
        Without<Replica>,
    >,
    emitters: Query<(Entity, &benilla_world::particles::ParticleEmitter)>,
    mut telegraphs: ResMut<super::telegraphs::Telegraphs>,
) {
    if coop.running {
        coop.since_host += real.delta_secs();
    }
    for event in events(&coop) {
        match event {
            Ok(Server::Economy { .. }) => {}
            Err(reason) => {
                coop.connected = false;
                coop.connecting = false;
                coop.status = format!("Connection lost: {reason}");
                if coop.running && *phase.get() != Phase::GameOver {
                    run.won = false;
                    next.set(Phase::GameOver);
                }
                coop.running = false;
                coop.room = None;
                coop.open = true;
            }
            Ok(Server::Welcome) => {
                coop.connected = true;
                coop.connecting = false;
                coop.status = "Connected. Host a room or join friends.".into();
                coop.send(Client::List);
            }
            Ok(Server::Rooms { rooms }) => {
                coop.rooms = rooms;
            }
            Ok(Server::Room { room, peer }) => {
                let members: HashSet<_> = room.members.iter().map(|m| m.peer).collect();
                coop.players.retain(|p, _| members.contains(p));
                coop.peer = peer;
                coop.room = Some(room);
                coop.status = "Ready up when your hero is chosen.".into();
            }
            Ok(Server::Error { reason }) => {
                coop.status = reason;
            }
            Ok(Server::Closed { reason }) => {
                coop.status = reason;
                if coop.running && *phase.get() != Phase::GameOver {
                    run.won = false;
                    next.set(Phase::GameOver);
                }
                coop.running = false;
                coop.room = None;
                coop.open = true;
                coop.players.clear();
            }
            Ok(Server::Left { peer }) => {
                coop.players.remove(&peer);
            }
            Ok(Server::Start) => {
                if *phase.get() != Phase::Menu {
                    coop.leave();
                    continue;
                }
                let Some(room) = coop.room.clone() else {
                    continue;
                };
                let Some(member) = room.members.iter().find(|m| m.peer == coop.peer) else {
                    continue;
                };
                if !selection.race_unlocked(member.hero.race)
                    || !data::MAPS.get(room.map).is_some_and(|map| super::races::can_enter(member.hero.race, map.name))
                {
                    coop.leave();
                    coop.status = "This race is locked or the zone belongs to the other faction.".into();
                    continue;
                }
                selection.class = Some(member.hero.class);
                selection.looks = vec![(member.hero.display, member.hero.race, member.hero.sex)];
                selection.look = Some(0);
                selection.map = room.map;
                selection.version += 1;
                if !super::hud::start_run(&mut run, &mut selection) {
                    coop.leave();
                    continue;
                }
                run.next_guid = (u64::from(coop.peer) + 1) << 40;
                run.coop_run = true;
                coop.running = true;
                coop.open = false;
                coop.remote_paused = true;
                coop.since_host = 0.;
                coop.revival = 0;
                coop.revive_progress.clear();
                coop.xp = 0;
                coop.gold = 0;
                coop.caches.clear();
                coop.seen_caches = 0;
                coop.incoming.clear();
                coop.outgoing.clear();
                coop.seq = 0;
                coop.unacked.clear();
                coop.acks.clear();
                coop.players.clear();
                coop.visuals.clear();
                coop.replay.clear();
                for m in &room.members {
                    coop.players.insert(
                        m.peer,
                        Pose {
                            peer: m.peer,
                            ..default()
                        },
                    );
                }
                next.set(Phase::Playing);
            }
            Ok(Server::Data { peer, payload }) => {
                if !coop.running {
                    continue;
                }
                let Ok(packet) = serde_json::from_value::<Packet>(payload) else {
                    continue;
                };
                match packet {
                    Packet::Player {
                        mut pose,
                        hits,
                        visuals,
                        seq,
                    } if coop.peer == 0 && peer != 0 => {
                        if !coop
                            .room
                            .as_ref()
                            .is_some_and(|r| r.members.iter().any(|m| m.peer == peer))
                        {
                            continue;
                        }
                        if !pose.pos.iter().all(|p| p.is_finite())
                            || !pose.yaw.is_finite()
                            || !pose.hp.is_finite()
                            || !pose.maximum.is_finite()
                            || !pose.damage.is_finite()
                            || pose.damage < 0.
                            || pose.outfit.len() > 19
                            || pose.outfit.iter().any(|i| i.display > 2_000_000 || i.slot.len() > 32 || i.category.len() > 32)
                        {
                            continue;
                        }
                        let center = hero::arena(selection.stage_def());
                        if units::flat(Vec3::from_array(pose.pos) - center).length()
                            > selection.stage_def().leash + 10.
                        {
                            continue;
                        }
                        pose.peer = peer;
                        // Repeat the host's new life until the guest acknowledges it; stale
                        // packets must neither undo a resurrection nor resurrect an old corpse.
                        if coop
                            .players
                            .get(&peer)
                            .is_some_and(|p| p.revival != pose.revival)
                        {
                            continue;
                        }
                        coop.players.insert(peer, pose.clone());
                        for mut visual in visuals.into_iter().take(64).filter(valid_visual) {
                            visual.peer = Some(peer);
                            coop.visuals.push(visual.clone());
                            coop.replay.push(visual);
                        }
                        coop.acks.insert(peer, seq);
                        if pose.hp > 0. && pose.loaded && !pose.paused && !hits.is_empty() {
                            let reach: HashMap<_, _> = foes
                                .iter()
                                .map(|(_, id, _, tf, ..)| (id.0, tf.translation))
                                .collect();
                            for h in hits.into_iter().take(2048) {
                                if h.damage.is_finite()
                                    && (0. ..=1_000_000.).contains(&h.damage)
                                    && h.seconds.is_finite()
                                    && h.strength.is_finite()
                                {
                                    // Guests cannot hit arbitrary entities or enemies across the arena.
                                    if reach.get(&h.target).is_some_and(|at| {
                                        units::flat(*at - Vec3::from_array(pose.pos)).length()
                                            <= 70.
                                    }) {
                                        coop.incoming.push(h);
                                    }
                                }
                            }
                        }
                    }
                    Packet::World {
                        foes: state,
                        pickups: loot,
                        players,
                        revive_progress,
                        equipment_kills,
                        boss_loot,
                        acks,
                        elapsed,
                        kills,
                        bosses,
                        guardians,
                        xp,
                        gold,
                        caches,
                        damage,
                        visuals,
                        warnings,
                        paused,
                        ended,
                        won,
                    } if coop.guest() && peer == 0 => {
                        coop.since_host = 0.;
                        coop.remote_paused = paused;
                        let applied = acks.get(&coop.peer).copied().unwrap_or(0);
                        coop.unacked.retain(|(seq, ..)| *seq > applied);
                        let mut predicted: HashMap<u64, f32> = HashMap::new();
                        for (_, target, damage) in &coop.unacked {
                            *predicted.entry(*target).or_default() += damage;
                        }
                        coop.revive_progress = revive_progress;
                        run.equipment_kills = equipment_kills;
                        run.boss_loot = boss_loot;
                        telegraphs.restore(warnings);
                        let me = coop.peer;
                        coop.replay.extend(
                            visuals
                                .into_iter()
                                .take(256)
                                .filter(|v| v.peer != Some(me) && valid_visual(v)),
                        );
                        coop.players = players.into_iter().map(|p| (p.peer, p)).collect();
                        if let Some(p) = coop.players.get(&me).cloned() {
                            if p.revival > coop.revival {
                                coop.revival = p.revival;
                                resurrect(&mut run);
                            }
                        }
                        run.elapsed = elapsed;
                        run.kills = kills;
                        run.bosses = bosses;
                        lairs.slain = guardians;
                        let add_xp = xp.saturating_sub(coop.xp);
                        let add_gold = gold.saturating_sub(coop.gold);
                        coop.xp = xp;
                        coop.gold = gold;
                        if run.dead_t.is_none() {
                            if add_xp > 0 {
                                run.xp += (add_xp as f32 * run.stats().growth
                                    / run.party_factor(0.48))
                                    .round()
                                    .max(1.);
                            }
                            run.gold = run.gold.saturating_add(super::gold::collected(
                                add_gold.min(u64::from(u32::MAX)) as u32,
                                run.gold_find(),
                                run.party_factor(0.48),
                            ));
                            while run.level < data::MAX_LEVEL
                                && run.xp >= data::xp_to_next(run.level)
                            {
                                run.xp -= data::xp_to_next(run.level);
                                run.level += 1;
                                run.pending_levels += 1;
                            }
                            if run.pending_levels > 0 && *phase.get() == Phase::Playing {
                                next.set(Phase::LevelUp);
                            }
                        }
                        for (target, raw) in damage {
                            if target == coop.peer {
                                if raw < 0. && run.dead_t.is_none() {
                                    run.heal(-raw);
                                } else if raw >= 0. && *phase.get() == Phase::Playing {
                                    hero::hurt_hero(&mut run, raw);
                                    run.struck = true;
                                }
                            }
                        }
                        for pos in caches.iter().skip(coop.seen_caches) {
                            let id = run.guid();
                            super::caches::spawn(&mut commands, id, Vec3::from_array(*pos), false);
                        }
                        coop.seen_caches = caches.len();
                        if !entry.0 && !loading.covering() {
                            let pickup_entities: HashMap<_, _> = pickups
                                .iter()
                                .map(|(entity, id, ..)| (id.0, entity))
                                .collect();
                            let foe_entities: HashMap<_, _> =
                                foes.iter().map(|(entity, id, ..)| (id.0, entity)).collect();
                            let present: HashSet<_> = loot.iter().map(|p| p.id).collect();
                            for (entity, id, ..) in &pickups {
                                if !present.contains(&id.0) {
                                    for (emitter_entity, emitter) in &emitters {
                                        if emitter.emit_host() == Some(entity)
                                            || emitter.anchor() == Some(entity)
                                        {
                                            commands.entity(emitter_entity).despawn();
                                        }
                                    }
                                    commands.entity(entity).despawn();
                                }
                            }
                            for p in loot {
                                if let Some((entity, _, mut tf, pose)) = pickup_entities
                                    .get(&p.id)
                                    .and_then(|entity| pickups.get_mut(*entity).ok())
                                {
                                    match pose {
                                        Some(mut pose) => {
                                            pose.push(Vec3::from_array(p.pos), None)
                                        }
                                        None => {
                                            tf.translation = Vec3::from_array(p.pos);
                                            commands.entity(entity).insert(ReplicaPose::new(
                                                tf.translation,
                                                None,
                                                HOST_TICK,
                                            ));
                                        }
                                    }
                                } else {
                                    let kind = match p.kind {
                                        1 => enemies::GemKind::Gold(p.amount as u32),
                                        2 => enemies::GemKind::Heal(p.amount),
                                        3 => enemies::GemKind::Magnet,
                                        _ => enemies::GemKind::Xp(p.amount as u32),
                                    };
                                    let serial = run.guid();
                                    let e = enemies::spawn_gem(
                                        &mut commands,
                                        serial,
                                        Vec3::from_array(p.pos),
                                        kind,
                                    );
                                    // The host has already seated/attracted this pickup.
                                    // Local terrain seating would overwrite its replicated Y
                                    // every frame, leaving the wisp at a different position.
                                    commands
                                        .entity(e)
                                        .remove::<super::loot_ground::GroundedLoot>()
                                        .insert((PickupReplica(p.id), Visibility::Inherited));
                                }
                            }
                            let roster = data::roster(selection.stage);
                            let present: HashSet<_> = state.iter().map(|f| f.id).collect();
                            for (entity, id, ..) in &foes {
                                if !present.contains(&id.0) {
                                    commands.entity(entity).despawn();
                                }
                            }
                            for f in state {
                                let moving = f.speed > 0.;
                                if let Some((entity, _, mut e, _tf, mut store, mut mv, pose)) =
                                    foe_entities
                                        .get(&f.id)
                                        .and_then(|entity| foes.get_mut(*entity).ok())
                                {
                                    match pose {
                                        Some(mut pose) => {
                                            pose.push(Vec3::from_array(f.pos), Some(f.yaw))
                                        }
                                        None => {
                                            commands.entity(entity).insert(ReplicaPose::new(
                                                Vec3::from_array(f.pos),
                                                Some(f.yaw),
                                                HOST_TICK,
                                            ));
                                        }
                                    }
                                    // Keep this guest's hits the host has not applied yet,
                                    // instead of letting each snapshot undo them.
                                    e.hp = if f.dead {
                                        f.hp
                                    } else {
                                        (f.hp - predicted.get(&f.id).copied().unwrap_or(0.))
                                            .max(0.001)
                                    };
                                    e.max_hp = f.maximum;
                                    if f.lair {
                                        e.home = Some(enemies::Home {
                                            pos: Vec3::from_array(f.pos),
                                            awake: true,
                                        });
                                    }
                                    mv.speed = f.speed;
                                    mv.flags = if moving {
                                        crate::creature_anim::move_flags::FORWARD
                                    } else {
                                        0
                                    };
                                    if f.dead && !e.dead {
                                        e.dead = true;
                                        units::kill_unit(&mut store);
                                    }
                                } else if !f.dead {
                                    let defs = match f.group {
                                        1 => &roster.bosses,
                                        2 => &roster.guardians,
                                        _ => &roster.enemies,
                                    };
                                    let Some(def) = defs.get(f.index) else {
                                        continue;
                                    };
                                    let entity = enemies::spawn_enemy(
                                        &mut commands,
                                        &mut run,
                                        def,
                                        Vec3::from_array(f.pos),
                                        Vec3::ZERO,
                                        f.group == 1,
                                    );
                                    commands.entity(entity).insert((
                                        Replica(f.id),
                                        ReplicaPose::new(
                                            Vec3::from_array(f.pos),
                                            Some(f.yaw),
                                            HOST_TICK,
                                        ),
                                    ));
                                    commands.queue(move |world: &mut World| {
                                        if let Some(mut e) = world.get_mut::<Enemy>(entity) {
                                            e.hp = f.hp;
                                            e.max_hp = f.maximum;
                                            if f.lair {
                                                e.home = Some(enemies::Home {
                                                    pos: Vec3::from_array(f.pos),
                                                    awake: true,
                                                });
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        if ended {
                            run.won = won;
                            next.set(Phase::GameOver);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    run.coop_unavailable.clear();
    run.coop_living = if coop.running {
        coop.players
            .values()
            .filter(|p| p.loaded && p.hp > 0.)
            .count()
            .max(1)
    } else {
        1
    };
    if coop.running {
        let choosing = matches!(phase.get(), Phase::LevelUp | Phase::Paused);
        let own = coop.peer;
        if let Some(pose) = coop.players.get_mut(&own) {
            pose.paused = choosing;
        }
        for pose in coop.players.values() {
            if (own == 0 || pose.peer == own) && (pose.paused || !pose.loaded || pose.hp <= 0.) {
                run.coop_unavailable.insert(pose.peer);
            }
        }
        if choosing {
            run.coop_unavailable.insert(own);
            run.coop_unavailable.insert(0);
        }
    }
    if coop.running && !coop.connected {
        run.won = false;
        next.set(Phase::GameOver);
        coop.running = false;
        coop.room = None;
        coop.open = true;
    }
    if coop.running {
        if *phase.get() == Phase::Menu && run.elapsed > 0. {
            coop.leave();
            clock.unpause();
            return;
        }
        let local_wait = *phase.get() == Phase::Menu || entry.0 || loading.covering();
        let waiting = if coop.peer == 0 {
            coop.room.as_ref().is_some_and(|r| {
                r.members
                    .iter()
                    .any(|m| m.peer != 0 && coop.players.get(&m.peer).is_none_or(|p| !p.loaded))
            })
        } else {
            coop.remote_paused || coop.since_host > 2.
        };
        // This hero's own entry needs the clock: it seats only once physics has stepped the new
        // ground's colliders in. The run itself is held by its entry and loading-screen gates, so
        // pausing here deadlocked a guest whose host was waiting for it to load.
        let entering = entry.0 || loading.covering();
        if *phase.get() == Phase::Menu || (waiting && !entering) {
            clock.pause();
        } else {
            clock.unpause();
        }
        // A party wait freezes this hero too: say who it is waiting on. Renewed every frame,
        // so it lapses a moment after the wait ends.
        if waiting && !local_wait {
            let loading: Vec<&str> = coop.room.as_ref().map_or_else(Vec::new, |r| {
                r.members
                    .iter()
                    .filter(|m| m.peer != coop.peer)
                    .filter(|m| coop.players.get(&m.peer).is_none_or(|p| !p.loaded))
                    .map(|m| m.name.as_str())
                    .collect()
            });
            let text = if coop.guest() && coop.since_host > 2. {
                "Waiting for the host…".to_string()
            } else if loading.is_empty() {
                "Waiting for the party…".to_string()
            } else {
                format!("Waiting for {} to finish loading…", loading.join(", "))
            };
            run.banner = Some((text, 0.25));
        }
        if coop.guest() && coop.since_host > 15. {
            coop.status = "The host stopped responding.".into();
            coop.leave();
            run.won = false;
            next.set(Phase::GameOver);
        }
    }
}

fn sync_allies(
    mut commands: Commands,
    coop: Res<Coop>,
    mut run: ResMut<Run>,
    mut allies: Query<(
        Entity,
        &Ally,
        &mut ObjectStore,
        &mut crate::creature_anim::MovementState,
        &mut ReplicaPose,
    )>,
    mut fields: MessageWriter<FieldChanged>,
    guids: Query<&Guid>,
) {
    let wanted: HashSet<_> = if coop.running {
        coop.players
            .keys()
            .copied()
            .filter(|p| *p != coop.peer)
            .collect()
    } else {
        HashSet::new()
    };
    let existing: HashSet<_> = allies.iter().map(|(_, a, ..)| a.0).collect();
    let span = if coop.guest() { HOST_TICK } else { GUEST_TICK };
    for (entity, ally, mut store, mut mv, mut glide) in &mut allies {
        if !wanted.contains(&ally.0) {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(p) = coop.players.get(&ally.0).filter(|p| p.loaded) {
            // A pose is pushed only when it changes, so a guest relayed through the
            // host at a different rate than its own sends still glides evenly.
            let pos = Vec3::from_array(p.pos);
            if pos != glide.to || glide.yaw != Some(p.yaw) {
                glide.push(pos, Some(p.yaw));
            }
            // Animate at the speed actually drawn; standing below a slow walk.
            let speed = glide.speed();
            let moving = speed > 0.5 && p.hp > 0.;
            mv.speed = if moving { speed.min(14.) } else { 0. };
            mv.flags = if moving {
                crate::creature_anim::move_flags::FORWARD
            } else {
                0
            };
            let hp = p.hp.max(0.).ceil() as u32;
            let changed = store
                .0
                .raw_fields()
                .find(|(index, _)| *index == 22)
                .map_or(0, |(_, value)| value)
                != hp;
            store
                .0
                .merge(benilla_protocol::messages::ObjectFields::from_pairs(&[
                    (22, hp),
                    (28, p.maximum as u32),
                    (34, p.level),
                ]));
            if p.hp <= 0. {
                units::kill_unit(&mut store);
            }
            if let Some(guid) = guids.get(entity).ok().filter(|_| changed) {
                fields.write(FieldChanged {
                    entity,
                    guid: guid.0,
                    kind: benilla_protocol::messages::ObjectType::Unit,
                    index: 22,
                    old: 0,
                    new: hp,
                });
            }
        }
    }
    if let Some(room) = &coop.room {
        for member in &room.members {
            if member.peer == coop.peer || existing.contains(&member.peer) || !coop.running {
                continue;
            }
            let Some(pose) = coop.players.get(&member.peer).filter(|p| p.loaded) else {
                continue;
            };
            let Some(class) = data::CLASSES.get(member.hero.class) else {
                continue;
            };
            let serial = run.guid();
            let e = hero::spawn_hero(
                &mut commands,
                class,
                member.hero.display,
                member.hero.race,
                member.hero.sex,
                serial,
                Vec3::from_array(pose.pos),
            );
            commands.entity(e).remove::<Hero>().insert((
                Ally(member.peer),
                ReplicaPose::new(Vec3::from_array(pose.pos), Some(pose.yaw), span),
            ));
        }
    }
}

fn smooth_replicas(time: Res<Time<Real>>, mut units: Query<(&mut ReplicaPose, &mut Transform)>) {
    let dt = time.delta_secs();
    let blend = 1. - (-15. * dt).exp();
    for (mut pose, mut transform) in &mut units {
        pose.age += dt;
        transform.translation = pose.sample();
        if let Some(yaw) = pose.yaw {
            transform.rotation = transform
                .rotation
                .slerp(Quat::from_rotation_y(yaw), blend);
        }
    }
}

fn transmit(
    mut commands: Commands,
    mut coop: ResMut<Coop>,
    real: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    entry: Res<super::drive::EntryPending>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    heroes: Query<&Transform, With<Hero>>,
    foes: Query<(
        Entity,
        &Guid,
        &Transform,
        &Enemy,
        &crate::creature_anim::MovementState,
        Has<Replica>,
    )>,
    lairs: Res<super::guardians::Lairs>,
    pickups: Query<(&Guid, &Transform, &enemies::Gem, &Visibility)>,
    telegraphs: Res<super::telegraphs::Telegraphs>,
) {
    let dt = real.delta_secs();
    if !coop.running {
        // Browsing refreshes the room list once a second.
        coop.list_t += dt;
        if coop.list_t >= 1. || phase.is_changed() {
            coop.list_t = 0.;
            if coop.connected {
                coop.send(Client::List);
            }
        }
        coop.tick = 0.;
        return;
    }
    // Carry the remainder so packets leave at an even rate rather than drifting with
    // the frame time; never bank more than one extra packet after a stall.
    let period = if coop.guest() { GUEST_TICK } else { HOST_TICK };
    coop.tick += dt;
    if coop.tick < period && !phase.is_changed() {
        return;
    }
    coop.tick = (coop.tick - period).clamp(0., period);
    let Ok(tf) = heroes.single() else { return };
    let pose = Pose {
        peer: coop.peer,
        pos: tf.translation.to_array(),
        yaw: tf.rotation.to_euler(EulerRot::YXZ).0,
        hp: run.hp,
        maximum: run.stats().max_hp,
        level: run.level,
        loaded: !entry.0 && !loading.covering() && *phase.get() != Phase::Menu,
        paused: matches!(phase.get(), Phase::LevelUp | Phase::Paused),
        revival: coop.revival,
        outfit: run.gear_look.clone(),
        damage: run.damage_done,
    };
    let own_peer = coop.peer;
    coop.players.insert(own_peer, pose.clone());
    let packet = if coop.guest() {
        let hits = std::mem::take(&mut coop.outgoing);
        let visuals = std::mem::take(&mut coop.visuals);
        coop.seq += 1;
        Packet::Player {
            pose,
            hits,
            visuals,
            seq: coop.seq,
        }
    } else {
        let roster = data::roster(selection.stage);
        let mut state = Vec::new();
        for (entity, guid, tf, e, mv, replica) in &foes {
            let definition = [&roster.enemies, &roster.bosses, &roster.guardians]
                .iter()
                .enumerate()
                .find_map(|(group, defs)| {
                    defs.iter()
                        .position(|d| std::ptr::eq(d, e.def))
                        .map(|i| (group as u8, i))
                });
            let Some((group, index)) = definition else {
                continue;
            };
            if !replica {
                commands.entity(entity).insert(Replica(guid.0));
            }
            state.push(Foe {
                id: guid.0,
                group,
                index,
                pos: round_pos(tf.translation),
                yaw: round_to(tf.rotation.to_euler(EulerRot::YXZ).0, 100.),
                hp: round_to(e.hp, 10.).max(if e.dead { 0. } else { 0.1 }),
                maximum: round_to(e.max_hp, 10.),
                speed: if mv.speed > 0. { round_to(e.speed, 100.) } else { 0. },
                dead: e.dead,
                lair: e.home.is_some(),
            });
        }
        let paused = coop.room.as_ref().is_some_and(|r| {
            r.members
                .iter()
                .any(|m| coop.players.get(&m.peer).is_none_or(|p| !p.loaded))
        });
        let pickups = pickups
            .iter()
            .filter(|(_, _, _, visible)| **visible != Visibility::Hidden)
            .map(|(id, tf, gem, _)| {
                let (kind, amount) = match gem.kind {
                    enemies::GemKind::Xp(n) => (0, n as f32),
                    enemies::GemKind::Gold(n) => (1, n as f32),
                    enemies::GemKind::Heal(n) => (2, n),
                    enemies::GemKind::Magnet => (3, 0.),
                };
                Pickup {
                    id: id.0,
                    pos: round_pos(tf.translation),
                    kind,
                    amount,
                }
            })
            .collect();
        Packet::World {
            foes: state,
            pickups,
            players: coop.players.values().cloned().collect(),
            revive_progress: coop.revive_progress.clone(),
            equipment_kills: run.equipment_kills,
            boss_loot: run.boss_loot.clone(),
            acks: coop.acks.clone(),
            elapsed: run.elapsed,
            kills: run.kills,
            bosses: run.bosses,
            guardians: lairs.slain,
            xp: coop.xp,
            gold: coop.gold,
            caches: coop.caches.clone(),
            damage: std::mem::take(&mut run.coop_damage),
            visuals: std::mem::take(&mut coop.visuals),
            warnings: telegraphs.snapshot(),
            paused,
            ended: *phase.get() == Phase::GameOver,
            won: run.won,
        }
    };
    if let Ok(payload) = serde_json::to_value(packet) {
        coop.send(Client::Data { payload });
    }
}

fn valid_visual(v: &Visual) -> bool {
    v.spell > 0
        && v.spell < 200_000
        && v.hits.len() <= 32
        && v.dest.is_none_or(|p| p.iter().all(|v| v.is_finite()))
}
fn capture_visuals(
    mut coop: ResMut<Coop>,
    mut casts: MessageReader<crate::creature_anim::SpellGoTargets>,
    local: Query<Entity, With<Hero>>,
    foes: Query<&Guid, With<Enemy>>,
    replicas: Query<&Replica>,
) {
    if !coop.running {
        casts.clear();
        return;
    }
    for cast in casts.read() {
        let peer = if local.contains(cast.caster) {
            Some(coop.peer)
        } else if !coop.guest() && foes.contains(cast.caster) {
            None
        } else {
            continue;
        };
        let caster = foes.get(cast.caster).map_or(0, |g| g.0);
        let hits = cast
            .hits
            .iter()
            .filter_map(|e| replicas.get(*e).map(|r| r.0).ok())
            .collect();
        if coop.visuals.len() < 256 {
            coop.visuals.push(Visual {
                peer,
                caster,
                spell: cast.spell_id,
                hits,
                dest: cast.dest.map(|p| p.to_array()),
            });
        }
    }
}
fn play_visuals(
    mut coop: ResMut<Coop>,
    allies: Query<(Entity, &Ally)>,
    foes: Query<(Entity, &Replica)>,
    mut fx: Fx,
) {
    for v in std::mem::take(&mut coop.replay) {
        let caster = if let Some(peer) = v.peer {
            allies.iter().find(|(_, a)| a.0 == peer).map(|(e, _)| e)
        } else {
            foes.iter().find(|(_, id)| id.0 == v.caster).map(|(e, _)| e)
        };
        if let Some(caster) = caster {
            let hits: Vec<_> = v
                .hits
                .iter()
                .filter_map(|id| foes.iter().find(|(_, r)| r.0 == *id).map(|(e, _)| e))
                .collect();
            fx.cast(caster, v.spell, &hits, v.dest.map(Vec3::from_array));
        }
    }
}

pub(super) fn apply_guest_hits(
    mut commands: Commands,
    mut coop: ResMut<Coop>,
    mut run: ResMut<Run>,
    mut foes: Query<(
        Entity,
        &Guid,
        &Transform,
        &mut Enemy,
        &mut ObjectStore,
        &mut Marks,
    )>,
    mut loot: MessageWriter<enemies::Loot>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
) {
    if coop.guest() {
        return;
    }
    let targets: HashMap<_, _> = foes
        .iter()
        .map(|(entity, guid, ..)| (guid.0, entity))
        .collect();
    for hit in std::mem::take(&mut coop.incoming) {
        let Some((entity, guid, tf, mut enemy, mut store, mut marks)) = targets
            .get(&hit.target)
            .and_then(|entity| foes.get_mut(*entity).ok())
        else {
            continue;
        };
        if enemy.dead {
            continue;
        }
        enemy.hp -= hit.damage;
        fx.number(entity, hit.damage, false, units::COLOR_SPELL);
        let boss = enemy.boss || enemy.home.is_some();
        let seconds = hit.seconds.clamp(0., 30.) * if boss { 0.25 } else { 1. };
        match hit.effect {
            1 => {
                enemy.slow = hit.strength.clamp(if boss { 0.75 } else { 0.1 }, 1.);
                enemy.slow_t = enemy.slow_t.max(seconds);
            }
            2 => enemy.root_t = enemy.root_t.max(seconds),
            3 => enemy.stun_t = enemy.stun_t.max(seconds),
            4 if !boss => enemy.fear_t = enemy.fear_t.max(seconds),
            5 => {
                enemy.weaken = hit.strength.clamp(0.3, 1.);
                enemy.weaken_t = seconds;
            }
            _ => {}
        }
        if enemy.hp <= 0. {
            enemy.hp = 0.;
            enemy.dead = true;
            enemy.dots.clear();
            enemies::mark_dying(&mut commands, entity);
            run.kills += 1;
            run.slain_last |= enemy.def.last;
            loot.write(enemies::Loot {
                pos: tf.translation,
                xp: enemy.xp,
                boss: enemy.boss,
                boss_name: enemy.boss.then(|| enemy.def.name.to_owned()),
                spawn_cost: if enemy.home.is_none() {
                    super::horde_spawning::cost(enemy.def)
                } else {
                    0.0
                },
                roaming_elite: enemy.home.is_none() && super::elwynn::is_roaming_elite(enemy.def),
                elite: enemy.home.is_some() || super::elwynn::is_roaming_elite(enemy.def),
            });
            units::kill_unit(&mut store);
            marks.clear();
            marks.flush(entity, guid.0, &mut store, &mut fields);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glides_continuously_between_snapshots() {
        let mut pose = ReplicaPose::new(Vec3::ZERO, None, 0.1);
        pose.age = 0.1;
        pose.push(Vec3::X, None);
        assert_eq!(pose.sample(), Vec3::ZERO);
        pose.age = 0.05;
        assert!((pose.sample().x - 0.5).abs() < 1e-4);
        // A late snapshot keeps moving along the last velocity, then holds.
        pose.age = 0.15;
        assert!((pose.sample().x - 1.5).abs() < 1e-3);
        pose.age = 5.;
        assert!((pose.sample().x - 2.5).abs() < 1e-3);
        // The next glide starts from where the replica is drawn.
        pose.age = 0.12;
        let drawn = pose.sample();
        pose.push(Vec3::X * 2., None);
        assert_eq!(pose.sample(), drawn);
        // Teleports snap.
        pose.push(Vec3::X * 100., None);
        assert_eq!(pose.sample(), Vec3::X * 100.);
    }

    #[test]
    fn compact_rows_round_trip_and_omit_defaults() {
        let foe = Foe {
            id: 7,
            group: 0,
            index: 3,
            pos: round_pos(Vec3::new(-8943.1234, 12.3456, 0.004)),
            yaw: 1.5,
            hp: 10.,
            maximum: 20.,
            speed: 0.,
            dead: false,
            lair: false,
        };
        let text = serde_json::to_string(&foe).unwrap();
        assert_eq!(
            text,
            r#"{"i":7,"n":3,"p":[-8943.12,12.35,0.0],"y":1.5,"h":10.0,"m":20.0}"#
        );
        let back: Foe = serde_json::from_str(&text).unwrap();
        assert_eq!((back.group, back.speed, back.dead), (0, 0., false));
    }

    /// A snapshot that acknowledges a guest still reaches it: the per-peer maps travel through
    /// the tagged packet and the relay's JSON value intact.
    #[test]
    fn world_with_acks_and_revivals_round_trips() {
        let packet = Packet::World {
            foes: Vec::new(),
            pickups: Vec::new(),
            players: Vec::new(),
            revive_progress: HashMap::from([(1, 0.5)]),
            equipment_kills: [0; 3],
            boss_loot: Vec::new(),
            acks: HashMap::from([(1, 42)]),
            elapsed: 3.,
            kills: 0,
            bosses: 0,
            guardians: 0,
            xp: 0,
            gold: 0,
            caches: Vec::new(),
            damage: Vec::new(),
            visuals: Vec::new(),
            warnings: Vec::new(),
            paused: false,
            ended: false,
            won: false,
        };
        let value = serde_json::to_value(packet).unwrap();
        let text = serde_json::to_string(&value).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let Packet::World { acks, revive_progress, .. } = serde_json::from_value(value).unwrap()
        else {
            panic!("a world packet");
        };
        assert_eq!(acks.get(&1), Some(&42));
        assert_eq!(revive_progress.get(&1), Some(&0.5));
    }
}
