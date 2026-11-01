//! Player-hosted matches over the operator's directory. Clients own their hero/build;
//! the host owns enemies, the encounter clock, shared pickups and the result.
use super::{
    Phase, Run, Selection, data,
    enemies::{self, Enemy},
    hero::{self, Hero},
    units::{self, Fx, Marks},
};
use crate::net::{FieldChanged, Guid, ObjectStore};
use asr_coop::{Client, Member, Room, Server};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const DEFAULT_SERVER: &str = "ws://hassian.duckdns.org:47771";

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
}

#[derive(Clone, Serialize, Deserialize)]
struct Foe {
    id: u64,
    group: u8,
    index: usize,
    pos: [f32; 3],
    yaw: f32,
    hp: f32,
    maximum: f32,
    speed: f32,
    moving: bool,
    dead: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Pickup {
    id: u64,
    pos: [f32; 3],
    kind: u8,
    amount: f32,
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
    },
    World {
        foes: Vec<Foe>,
        pickups: Vec<Pickup>,
        players: Vec<Pose>,
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

#[derive(Component)]
pub(super) struct Replica(pub u64);
#[derive(Component)]
pub(super) struct Ally(u32);

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
    remote_paused: bool,
    since_host: f32,
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
            remote_paused: true,
            since_host: 0.,
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
        let mut result = (local.0, local.1, 0, !local_alive);
        let mut distance = if local_alive {
            units::flat(from - local.1).length_squared()
        } else {
            f32::INFINITY
        };
        if self.running {
            for (entity, ally) in allies {
                if let Some(p) = self.players.get(&ally.0).filter(|p| p.loaded && p.hp > 0.) {
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
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Coop>()
        .add_systems(PreUpdate, receive)
        .add_systems(
            PostUpdate,
            (sync_allies, play_visuals, capture_visuals, transmit).chain(),
        );
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
    )>,
    mut lairs: ResMut<super::guardians::Lairs>,
    mut pickups: Query<(Entity, &PickupReplica, &mut Transform), Without<Replica>>,
    emitters: Query<(Entity, &benilla_world::particles::ParticleEmitter)>,
    mut telegraphs: ResMut<super::telegraphs::Telegraphs>,
) {
    if coop.running {
        coop.since_host += real.delta_secs();
    }
    for event in events(&coop) {
        match event {
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
                coop.xp = 0;
                coop.gold = 0;
                coop.caches.clear();
                coop.seen_caches = 0;
                coop.incoming.clear();
                coop.outgoing.clear();
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
                        coop.players.insert(peer, pose.clone());
                        for mut visual in visuals.into_iter().take(64).filter(valid_visual) {
                            visual.peer = Some(peer);
                            coop.visuals.push(visual.clone());
                            coop.replay.push(visual);
                        }
                        if pose.hp > 0. && pose.loaded && !pose.paused {
                            for h in hits.into_iter().take(2048) {
                                if h.damage.is_finite()
                                    && (0. ..=1_000_000.).contains(&h.damage)
                                    && h.seconds.is_finite()
                                    && h.strength.is_finite()
                                {
                                    // Guests cannot hit arbitrary entities or enemies across the arena.
                                    if foes.iter().any(|(_, id, _, tf, ..)| {
                                        id.0 == h.target
                                            && units::flat(
                                                tf.translation - Vec3::from_array(pose.pos),
                                            )
                                            .length()
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
                        telegraphs.restore(warnings);
                        let me = coop.peer;
                        coop.replay.extend(
                            visuals
                                .into_iter()
                                .take(256)
                                .filter(|v| v.peer != Some(me) && valid_visual(v)),
                        );
                        coop.players = players.into_iter().map(|p| (p.peer, p)).collect();
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
                                run.xp += (add_xp as f32 * run.stats().growth).round().max(1.);
                            }
                            run.gold = run.gold.saturating_add(super::gold::collected(
                                add_gold.min(u64::from(u32::MAX)) as u32,
                                run.jewel_stat(13),
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
                                    run.hp = (run.hp - raw).min(run.stats().max_hp);
                                } else if raw >= 0. {
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
                            let present: HashSet<_> = loot.iter().map(|p| p.id).collect();
                            for (entity, id, _) in &pickups {
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
                                if let Some((_, _, mut tf)) =
                                    pickups.iter_mut().find(|(_, id, _)| id.0 == p.id)
                                {
                                    tf.translation = Vec3::from_array(p.pos);
                                } else {
                                    let kind = match p.kind {
                                        1 => enemies::GemKind::Gold(p.amount as u32),
                                        2 => enemies::GemKind::Heal(p.amount),
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
                                if let Some((_, _, mut e, mut tf, mut store, mut mv)) =
                                    foes.iter_mut().find(|(_, id, ..)| id.0 == f.id)
                                {
                                    tf.translation = Vec3::from_array(f.pos);
                                    tf.rotation = Quat::from_rotation_y(f.yaw);
                                    e.hp = f.hp;
                                    e.max_hp = f.maximum;
                                    if f.group == 2 {
                                        e.home = Some(enemies::Home {
                                            pos: Vec3::from_array(f.pos),
                                            awake: true,
                                        });
                                    }
                                    mv.speed = if f.moving { f.speed } else { 0. };
                                    mv.flags = if f.moving {
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
                                    commands.entity(entity).insert(Replica(f.id));
                                    commands.queue(move |world: &mut World| {
                                        if let Some(mut e) = world.get_mut::<Enemy>(entity) {
                                            e.hp = f.hp;
                                            e.max_hp = f.maximum;
                                            if f.group == 2 {
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
        let local_wait = !matches!(phase.get(), Phase::Playing | Phase::GameOver)
            || entry.0
            || loading.covering();
        let waiting = if coop.peer == 0 {
            coop.room.as_ref().is_some_and(|r| {
                r.members.iter().any(|m| {
                    m.peer != 0
                        && coop
                            .players
                            .get(&m.peer)
                            .is_none_or(|p| !p.loaded || p.paused)
                })
            })
        } else {
            coop.remote_paused || coop.since_host > 2.
        };
        if local_wait || waiting {
            clock.pause();
        } else {
            clock.unpause();
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
        &mut Transform,
        &mut ObjectStore,
        &mut crate::creature_anim::MovementState,
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
    for (entity, ally, mut tf, mut store, mut mv) in &mut allies {
        if !wanted.contains(&ally.0) {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(p) = coop.players.get(&ally.0).filter(|p| p.loaded) {
            let pos = Vec3::from_array(p.pos);
            let delta = units::flat(pos - tf.translation).length();
            tf.translation = pos;
            tf.rotation = Quat::from_rotation_y(p.yaw);
            mv.speed = if delta > 0.01 { 7. } else { 0. };
            mv.flags = if delta > 0.01 {
                crate::creature_anim::move_flags::FORWARD
            } else {
                0
            };
            store
                .0
                .merge(benilla_protocol::messages::ObjectFields::from_pairs(&[
                    (22, p.hp.max(0.) as u32),
                    (28, p.maximum as u32),
                    (34, p.level),
                ]));
            if p.hp <= 0. {
                units::kill_unit(&mut store);
            }
            if let Ok(guid) = guids.get(entity) {
                fields.write(FieldChanged {
                    entity,
                    guid: guid.0,
                    kind: benilla_protocol::messages::ObjectType::Unit,
                    index: 22,
                    old: 0,
                    new: p.hp.max(0.) as u32,
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
            commands
                .entity(e)
                .remove::<Hero>()
                .insert(Ally(member.peer));
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
    )>,
    lairs: Res<super::guardians::Lairs>,
    pickups: Query<(&Guid, &Transform, &enemies::Gem, &Visibility)>,
    telegraphs: Res<super::telegraphs::Telegraphs>,
) {
    coop.tick += real.delta_secs();
    if coop.tick < 0.0667 && !phase.is_changed() {
        return;
    }
    coop.tick = 0.;
    if !coop.running {
        if coop.connected {
            coop.send(Client::List);
        }
        return;
    }
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
    };
    let own_peer = coop.peer;
    coop.players.insert(own_peer, pose.clone());
    let packet = if coop.guest() {
        let hits = std::mem::take(&mut coop.outgoing);
        let visuals = std::mem::take(&mut coop.visuals);
        Packet::Player {
            pose,
            hits,
            visuals,
        }
    } else {
        let roster = data::roster(selection.stage);
        let mut state = Vec::new();
        for (entity, guid, tf, e, mv) in &foes {
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
            commands.entity(entity).insert(Replica(guid.0));
            state.push(Foe {
                id: guid.0,
                group,
                index,
                pos: tf.translation.to_array(),
                yaw: tf.rotation.to_euler(EulerRot::YXZ).0,
                hp: e.hp,
                maximum: e.max_hp,
                speed: e.speed,
                moving: mv.speed > 0.,
                dead: e.dead,
            });
        }
        let paused = coop.room.as_ref().is_some_and(|r| {
            r.members.iter().any(|m| {
                coop.players
                    .get(&m.peer)
                    .is_none_or(|p| !p.loaded || p.paused)
            })
        });
        let pickups = pickups
            .iter()
            .filter(|(_, _, _, visible)| **visible != Visibility::Hidden)
            .map(|(id, tf, gem, _)| {
                let (kind, amount) = match gem.kind {
                    enemies::GemKind::Xp(n) => (0, n as f32),
                    enemies::GemKind::Gold(n) => (1, n as f32),
                    enemies::GemKind::Heal(n) => (2, n),
                };
                Pickup {
                    id: id.0,
                    pos: tf.translation.to_array(),
                    kind,
                    amount,
                }
            })
            .collect();
        Packet::World {
            foes: state,
            pickups,
            players: coop.players.values().cloned().collect(),
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
    for hit in std::mem::take(&mut coop.incoming) {
        let Some((entity, guid, tf, mut enemy, mut store, mut marks)) =
            foes.iter_mut().find(|(_, id, ..)| id.0 == hit.target)
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
            });
            units::kill_unit(&mut store);
            marks.clear();
            marks.flush(entity, guid.0, &mut store, &mut fields);
        }
    }
}
