//! Morgan's Defias finale: committed blades and gunfire, explosive coin bags,
//! limited reinforcements, and a smoke ambush below forty percent health.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{
    Run,
    boss_cooldowns::Timer,
    boss_pacing::Windups,
    data::{self, Ranged},
    dungeon::Ground,
    enemies::{self, Enemy},
    mechanics::{self, Target},
    telegraphs::Telegraphs,
    units::{Fx, flat, yaw_toward},
};

pub(super) static THUG: data::EnemyDef = data::EnemyDef {
    attack: 1.8,
    ..data::en("Defias Thug", 2323, 500.0, 7.0, 15.0, 0, 12)
};
pub(super) static MARKSMAN: data::EnemyDef = data::EnemyDef {
    attack: 2.5,
    ranged: Some(Ranged {
        spell: 75,
        range: 22.0,
        cooldown: 3.0,
    }),
    ..data::en("Defias Marksman", 2324, 400.0, 6.0, 20.0, 0, 12)
};

const CUT: usize = 0;
const SHOT: usize = 1;
const MONEY: usize = 2;
const CALL: usize = 3;
const SMOKE: usize = 4;
const BOMBS: usize = 5;
const BOMB_RADIUS: f32 = 2.0;
const NAMES: [&str; 6] = [
    "Collector's Cut",
    "Marked for Collection",
    "Dirty Money",
    "Call the Brotherhood",
    "Smoke and Daggers",
    "Collector's Bombs",
];
const SPELLS: [u32; 6] = [1752, 19434, 13812, 6673, 1856, 13812];
const PERIODS: [f32; 6] = [6.0, 12.0, 16.0, 1.0, 18.0, 7.0];
const CUT_RADIUS: f32 = 10.0;
const BAG_RADIUS: f32 = 3.0;
const SHOT_HALF: f32 = 1.3;
const RECOVERY: f32 = 0.8;
const BLEED_SECONDS: f32 = 30.0;
/// Old Blunderbuss: ItemDisplayInfo 6606, weapon class 2 / gun subclass 3.
pub(super) const WEAPONS: &[data::Weapon] = &[(2, 6606, 2, 3, 2, 26, 0)];

#[derive(Default)]
pub(super) struct Bleed {
    pub until: f32,
    pub next: f32,
    pub damage: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) enum Debuff {
    Marked {
        until: f32,
    },
    Bleeding {
        until: f32,
        next: f32,
        damage: f32,
    },
    Bomb {
        until: f32,
        bleed: f32,
        #[serde(default)]
        multiplier: f32,
    },
}

fn apply(run: &mut Run, peer: u32, effect: Debuff) {
    if run.coop_unavailable.contains(&peer) {
        return;
    }
    if peer == 0 {
        receive(run, effect);
    } else {
        run.coop_morgan_debuffs.push((peer, effect));
    }
}

pub(super) fn receive(run: &mut Run, effect: Debuff) {
    if run.dead_t.is_some() || run.hp <= 0.0 || run.immune_t > 0.0 || run.test_god {
        return;
    }
    let now = run.elapsed;
    match effect {
        Debuff::Marked { until } if until.is_finite() && until > now => {
            run.morgan_mark_until = until.min(now + 3.0);
        }
        Debuff::Bleeding {
            until,
            next,
            damage,
        } if until.is_finite()
            && next.is_finite()
            && damage.is_finite()
            && until > now
            && damage > 0.0 =>
        {
            if run.morgan_bleed.until <= now {
                run.morgan_bleed.next = next.max(now);
            }
            run.morgan_bleed.until = until.min(now + BLEED_SECONDS);
            run.morgan_bleed.damage = damage;
        }
        Debuff::Bomb {
            until,
            bleed,
            multiplier,
        } if until.is_finite()
            && until > now
            && bleed.is_finite()
            && bleed > 0.0
            && multiplier.is_finite() =>
        {
            receive(
                run,
                Debuff::Bleeding {
                    until,
                    next: now + 1.0,
                    damage: bleed,
                },
            );
            run.morgan_bomb_slow_until = until.min(now + BLEED_SECONDS);
            // Percentage-health trap damage is exact, independent of armour, shields or party size.
            run.hp -= run.stats().max_hp * 0.9 * multiplier.clamp(1.0, super::boss_enrage::DAMAGE);
        }
        _ => {}
    }
}

pub(super) fn tick_bleed(run: &mut Run, hero: Entity, fx: &mut Fx, now: f32) {
    while run.morgan_bleed.next > 0.0
        && run.morgan_bleed.next <= now
        && run.morgan_bleed.next <= run.morgan_bleed.until
    {
        let damage = run.morgan_bleed.damage;
        run.morgan_bleed.next += 1.0;
        let previous = run.coop_target;
        run.coop_target = 0;
        let taken = super::hero::hurt_hero(run, damage);
        run.coop_target = previous;
        if taken > 0.0 {
            fx.number(hero, taken, false, super::units::COLOR_HURT);
        }
    }
    if now >= run.morgan_bleed.until {
        run.morgan_bleed = Bleed::default();
    }
}

/// Every damaging Morgan ability shares one refreshing bleed and healing lockout.
fn strike(run: &mut Run, fx: &mut Fx, target: &Target, damage: f32) {
    mechanics::strike(run, fx, target, damage);
    apply(
        run,
        target.peer,
        Debuff::Bleeding {
            until: run.elapsed + BLEED_SECONDS,
            next: run.elapsed + 1.0,
            damage: damage * 0.1,
        },
    );
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Bomb {
    id: u64,
    pos: [f32; 3],
    armed: f32,
    #[serde(skip)]
    active: bool,
    #[serde(skip)]
    contacts: HashSet<u32>,
}

#[derive(Component, Default)]
pub(super) struct ReplicaBombs(pub Vec<Bomb>);

#[derive(Component)]
struct BombVisual {
    owner: Entity,
    id: u64,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        draw_bombs.after(super::enemies::run_scripts).after(cleanup),
    );
}

fn draw_bombs(
    mut commands: Commands,
    phase: Res<State<super::Phase>>,
    coop: Res<super::coop::Coop>,
    mut run: ResMut<Run>,
    bosses: Query<(
        Entity,
        &Enemy,
        Option<&enemies::Script>,
        Option<&ReplicaBombs>,
    )>,
    visuals: Query<(Entity, &BombVisual)>,
) {
    let mut bombs = Vec::new();
    if matches!(
        phase.get(),
        super::Phase::Playing | super::Phase::Paused | super::Phase::LevelUp
    ) {
        for (owner, enemy, script, replica) in &bosses {
            if !enemy.boss
                || enemy.dead
                || enemy.hp <= 0.0
                || enemy.def.name != "Morgan the Collector"
            {
                continue;
            }
            let state = if coop.guest() {
                replica.map(|r| r.0.as_slice()).unwrap_or_default()
            } else {
                script
                    .map(|s| s.morgan.bombs.as_slice())
                    .unwrap_or_default()
            };
            bombs.extend(state.iter().map(|b| (owner, b)));
        }
    }
    for (entity, visual) in &visuals {
        if !bombs
            .iter()
            .any(|(owner, b)| *owner == visual.owner && b.id == visual.id)
        {
            commands.entity(entity).despawn();
        }
    }
    for (owner, bomb) in bombs {
        if visuals
            .iter()
            .any(|(_, v)| v.owner == owner && v.id == bomb.id)
        {
            continue;
        }
        // GameObjectDisplayInfo 5791: World/Goober/LandMineAlteracValley.mdx.
        // The player's own MPQ chain supplies its native mesh and textures.
        let serial = run.guid();
        let mine = super::units::spawn_object(
            &mut commands,
            serial,
            5791,
            1.0,
            Vec3::from_array(bomb.pos),
        );
        commands
            .entity(mine)
            .insert(BombVisual { owner, id: bomb.id });
    }
}

enum Attack {
    Cut {
        center: Vec3,
        dir: Vec3,
    },
    Mark {
        peer: u32,
        center: Vec3,
    },
    Shot {
        center: Vec3,
        dir: Vec3,
        length: f32,
    },
    Bags {
        centers: Vec<Vec3>,
        echo: bool,
    },
    Call,
    DropBomb,
    Smoke {
        peer: u32,
    },
}
struct Pending {
    at: f32,
    damage: f32,
    attack: Attack,
}

#[derive(Default)]
pub(super) struct Script {
    born: bool,
    due: [f32; 6],
    cast_from: [f32; 6],
    cast_until: [f32; 6],
    busy: f32,
    smoke_phase: bool,
    last_stand: bool,
    waves: u8,
    adds: Vec<Entity>,
    last_target: Option<u32>,
    pending: Option<Pending>,
    pub(super) bombs: Vec<Bomb>,
}

impl Script {
    pub(super) fn enrage_pending(&mut self) {
        if let Some(pending) = self.pending.as_mut() {
            pending.damage *= super::boss_enrage::DAMAGE;
        }
    }
    pub(super) fn hidden(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|p| matches!(p.attack, Attack::Smoke { .. }))
    }

    fn period(&self, slot: usize) -> f32 {
        PERIODS[slot] * if self.last_stand { 0.8 } else { 1.0 }
    }

    fn waves_due(&self, share: f32) -> u8 {
        if self.last_stand {
            0
        } else if share <= 0.4 {
            2
        } else if share <= 0.7 {
            1
        } else {
            0
        }
    }

    pub(super) fn timers(&self, now: f32, share: f32) -> Vec<Timer> {
        if !self.born {
            return Vec::new();
        }
        (0..6)
            .filter(|&slot| {
                (slot != SMOKE || self.smoke_phase)
                    && (slot != CALL
                        || self.cast_until[CALL] > now
                        || (self.waves < self.waves_due(share) && self.adds.is_empty()))
            })
            .map(|slot| Timer {
                spell: SPELLS[slot],
                name: if slot == MONEY
                    && self.last_stand
                    && self
                        .pending
                        .as_ref()
                        .is_some_and(|p| matches!(p.attack, Attack::Bags { echo: false, .. }))
                {
                    "Dirty Money: Second Blast".into()
                } else {
                    NAMES[slot].into()
                },
                due: if slot == CALL { now } else { self.due[slot] },
                period: self.period(slot),
                cast_until: self.cast_until[slot],
                cast_duration: (self.cast_until[slot] - self.cast_from[slot]).max(0.1),
                ready: self.due[slot] <= now,
            })
            .collect()
    }

    fn target(&mut self, players: &[Target]) -> Option<Target> {
        let next = players
            .iter()
            .filter(|p| self.last_target.is_none_or(|last| p.peer > last))
            .min_by_key(|p| p.peer)
            .or_else(|| players.iter().min_by_key(|p| p.peer))
            .copied();
        if let Some(next) = next {
            self.last_target = Some(next.peer);
        }
        next
    }

    fn start(&mut self, slot: usize, now: f32, duration: f32, damage: f32, attack: Attack) {
        self.due[slot] = now + self.period(slot);
        self.cast_from[slot] = now;
        self.cast_until[slot] = now + duration;
        self.busy = now + duration + RECOVERY;
        self.pending = Some(Pending {
            at: now + duration,
            damage,
            attack,
        });
    }

    fn clear(&mut self, commands: &mut Commands, entity: Entity, tele: &mut Telegraphs) {
        if let Some(p) = self.pending.take() {
            p.cancel(tele);
        }
        for bomb in self.bombs.drain(..) {
            tele.remove_circle(Vec3::from_array(bomb.pos), BOMB_RADIUS);
        }
        self.cast_until = [0.0; 6];
        commands.entity(entity).insert(Visibility::Inherited);
        for add in self.adds.drain(..) {
            commands.entity(add).despawn();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn tick(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        enemy: &mut Enemy,
        tf: &mut Transform,
        players: &[Target],
        living_adds: &[Entity],
        now: f32,
        run: &mut Run,
        ground: &Ground,
        fx: &mut Fx,
        tele: &mut Telegraphs,
        windups: &mut Windups,
        rng: &mut super::Rng,
    ) {
        self.adds.retain(|add| living_adds.contains(add));
        if enemy.dead || enemy.hp <= 0.0 {
            self.clear(commands, entity, tele);
            return;
        }
        if !self.born {
            self.born = true;
            self.due = [now + 2.0, now + 5.0, now + 9.0, now, f32::MAX, now + 4.0];
        }
        // Bombs remain armed after triggering; a player must leave and touch one again
        // to trigger another hit, rather than taking damage every simulation frame.
        for bomb in &mut self.bombs {
            if now < bomb.armed {
                continue;
            }
            let pos = Vec3::from_array(bomb.pos);
            if !bomb.active {
                bomb.active = true;
                tele.hazard(pos, BOMB_RADIUS, now, f32::MAX / 4.0);
            }
            let inside: HashSet<u32> = players
                .iter()
                .filter(|p| flat(p.pos - pos).length() <= BOMB_RADIUS)
                .map(|p| p.peer)
                .collect();
            for player in players
                .iter()
                .filter(|p| inside.contains(&p.peer) && !bomb.contacts.contains(&p.peer))
            {
                let hp = run.hp;
                apply(
                    run,
                    player.peer,
                    Debuff::Bomb {
                        until: now + BLEED_SECONDS,
                        bleed: enemy.damage * 0.1,
                        multiplier: enemy.enrage_damage(),
                    },
                );
                fx.cast(entity, SPELLS[BOMBS], &[player.entity], Some(pos));
                if player.peer == 0 && run.hp < hp {
                    fx.number(player.entity, hp - run.hp, false, super::units::COLOR_HURT);
                }
            }
            bomb.contacts = inside;
        }
        let share = enemy.hp / enemy.max_hp;
        if !self.smoke_phase && share <= 0.4 {
            self.smoke_phase = true;
            self.due[SMOKE] = now + 3.0;
            run.yell = Some((
                "Morgan yells: You can't collect what you can't catch!".into(),
                4.0,
            ));
        }
        if !self.last_stand && share <= 0.2 {
            self.last_stand = true;
            if let Some(Pending {
                attack: Attack::Bags { echo, .. },
                ..
            }) = self.pending.as_mut()
            {
                *echo = true;
            }
            for slot in [CUT, SHOT, MONEY, SMOKE, BOMBS] {
                self.due[slot] = now + (self.due[slot] - now).max(0.0) * 0.8;
            }
            run.yell = Some(("Morgan yells: You'll never take my collection!".into(), 4.0));
            fx.cast(entity, 8599, &[], None);
        }
        if players.is_empty() || enemy.stun_t > 0.0 || enemy.fear_t > 0.0 {
            if let Some(p) = self.pending.take() {
                p.cancel(tele);
            }
            commands.entity(entity).insert(Visibility::Inherited);
            self.cast_until = [0.0; 6];
            enemy.script_hold_until = 0.0;
            self.busy = now + RECOVERY;
            return;
        }
        if self.pending.as_ref().is_some_and(|p| now >= p.at) {
            let p = self.pending.take().unwrap();
            match p.attack {
                Attack::Cut { center, dir } => {
                    fx.cast(entity, SPELLS[CUT], &[], None);
                    for target in players.iter().filter(|p| in_cone(p.pos, center, dir)) {
                        strike(run, fx, target, p.damage);
                    }
                }
                Attack::Mark { peer, center } => {
                    if let Some(target) = players.iter().find(|p| p.peer == peer) {
                        let delta = flat(target.pos - center);
                        let dir = delta.normalize_or(Vec3::Z);
                        let length = (delta.length() + 8.0).max(16.0);
                        let to = ground.at(center + dir * length, center.y);
                        tele.lane(center, to, SHOT_HALF, now, 1.0, 1.0);
                        self.cast_from[SHOT] = now;
                        self.cast_until[SHOT] = now + 1.0;
                        self.pending = Some(Pending {
                            at: now + 1.0,
                            damage: p.damage,
                            attack: Attack::Shot {
                                center,
                                dir,
                                length,
                            },
                        });
                        self.busy = now + 1.0 + RECOVERY;
                        tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
                    }
                }
                Attack::Shot {
                    center,
                    dir,
                    length,
                } => {
                    fx.cast(entity, 75, &[], Some(center + dir * length));
                    for target in players {
                        let delta = flat(target.pos - center);
                        let along = delta.dot(dir);
                        if (0.0..=length).contains(&along)
                            && (delta - dir * along).length() <= SHOT_HALF
                        {
                            strike(run, fx, target, p.damage);
                        }
                    }
                }
                Attack::Bags { centers, echo } => {
                    for center in &centers {
                        fx.cast(entity, SPELLS[MONEY], &[], Some(*center));
                    }
                    // Overlapping bags count as one hit per wave.
                    for target in players.iter().filter(|p| {
                        centers
                            .iter()
                            .any(|c| flat(p.pos - *c).length() <= BAG_RADIUS)
                    }) {
                        strike(run, fx, target, p.damage);
                    }
                    if echo {
                        for center in &centers {
                            tele.circle(*center, BAG_RADIUS, now, 1.25);
                        }
                        self.cast_from[MONEY] = now;
                        self.cast_until[MONEY] = now + 1.25;
                        self.pending = Some(Pending {
                            at: now + 1.25,
                            damage: p.damage,
                            attack: Attack::Bags {
                                centers,
                                echo: false,
                            },
                        });
                        self.busy = now + 1.25 + RECOVERY;
                    }
                }
                Attack::DropBomb => {}
                Attack::Call => {
                    if !self.last_stand {
                        let pos = tf.translation;
                        for (index, def) in [&THUG, &THUG, &MARKSMAN].into_iter().enumerate() {
                            let angle = index as f32 * std::f32::consts::TAU / 3.0;
                            let at = ground.spawn_spot(
                                pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * 12.0,
                                pos.y,
                            );
                            let add =
                                enemies::spawn_enemy(commands, run, def, at, players[0].pos, false);
                            let marksman = index == 2;
                            let health = enemy.max_hp * if marksman { 0.025 } else { 0.035 };
                            let damage = enemy.damage * if marksman { 0.55 } else { 0.35 };
                            commands
                                .entity(add)
                                .entry::<Enemy>()
                                .and_modify(move |mut add| {
                                    add.hp = health;
                                    add.max_hp = health;
                                    add.damage = damage;
                                    add.boss_minion = true;
                                    add.attack_cd = 2.0;
                                });
                            self.adds.push(add);
                        }
                    }
                }
                Attack::Smoke { peer } => {
                    commands.entity(entity).insert(Visibility::Inherited);
                    if let Some(target) = players.iter().find(|p| p.peer == peer) {
                        let dir = flat(target.pos - tf.translation).normalize_or(Vec3::Z);
                        let center = ground.spawn_spot(target.pos + dir * 8.0, target.pos.y);
                        let dir = flat(target.pos - center).normalize_or(-dir);
                        tf.translation = center;
                        tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
                        fx.cast(entity, 1856, &[], None);
                        tele.cone(
                            center,
                            dir,
                            CUT_RADIUS,
                            std::f32::consts::FRAC_PI_4,
                            now,
                            1.5,
                        );
                        self.cast_from[SMOKE] = now;
                        self.cast_until[SMOKE] = now + 1.5;
                        self.pending = Some(Pending {
                            at: now + 1.5,
                            damage: p.damage,
                            attack: Attack::Cut { center, dir },
                        });
                        self.busy = now + 1.5 + RECOVERY;
                    }
                }
            }
        }
        // Recovery delays the next ability, but lets Morgan chase and swing again.
        enemy.script_hold_until = self.pending.as_ref().map_or(0.0, Pending::hold_until);
        if self.pending.is_some() || now < self.busy {
            return;
        }
        if enemy.root_t > 0.0 || !windups.available(now) {
            return;
        }
        let pos = tf.translation;
        let damage = enemy.damage * enemy.weaken;
        let next = [CUT, SHOT, MONEY, SMOKE, BOMBS]
            .into_iter()
            .filter(|&slot| self.due[slot] <= now && (slot != SMOKE || self.smoke_phase))
            .filter(|&slot| {
                slot != CUT
                    || players
                        .iter()
                        .any(|p| flat(p.pos - pos).length() <= CUT_RADIUS)
            })
            .min_by(|&a, &b| self.due[a].total_cmp(&self.due[b]));
        if self.waves < self.waves_due(share) && self.adds.is_empty() {
            self.waves += 1;
            run.yell = Some(("Morgan yells: Brotherhood! Make them pay!".into(), 4.0));
            fx.cast(entity, SPELLS[CALL], &[], None);
            self.start(CALL, now, 1.0, 0.0, Attack::Call);
        } else if let Some(slot) = next {
            let target = if slot == CUT {
                players
                    .iter()
                    .min_by(|a, b| {
                        flat(a.pos - pos)
                            .length_squared()
                            .total_cmp(&flat(b.pos - pos).length_squared())
                    })
                    .copied()
            } else {
                self.target(players)
            };
            let Some(target) = target else { return };
            match slot {
                CUT => {
                    let dir = flat(target.pos - pos).normalize_or(Vec3::Z);
                    tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
                    tele.cone(pos, dir, CUT_RADIUS, std::f32::consts::FRAC_PI_4, now, 1.3);
                    self.start(
                        CUT,
                        now,
                        1.3,
                        damage * 1.1,
                        Attack::Cut { center: pos, dir },
                    );
                }
                SHOT => {
                    apply(run, target.peer, Debuff::Marked { until: now + 2.5 });
                    fx.cast(entity, 1130, &[target.entity], None);
                    self.start(
                        SHOT,
                        now,
                        1.5,
                        damage * 1.4,
                        Attack::Mark {
                            peer: target.peer,
                            center: pos,
                        },
                    );
                }
                MONEY => {
                    let count = if players.len() == 1 { 2 } else { 3 };
                    let mut ordered = players.to_vec();
                    ordered.sort_by_key(|p| p.peer.wrapping_sub(target.peer));
                    let centers = (0..count)
                        .map(|i| {
                            let p = ordered[i % ordered.len()].pos;
                            let angle = i as f32 * std::f32::consts::TAU / count as f32;
                            let offset = if i < ordered.len() {
                                Vec3::ZERO
                            } else {
                                Vec3::new(angle.cos(), 0.0, angle.sin()) * 7.0
                            };
                            ground.spawn_spot(p + offset, p.y)
                        })
                        .collect::<Vec<_>>();
                    for center in &centers {
                        tele.circle(*center, BAG_RADIUS, now, 2.0);
                    }
                    self.start(
                        MONEY,
                        now,
                        2.0,
                        damage,
                        Attack::Bags {
                            centers,
                            echo: self.last_stand,
                        },
                    );
                }
                BOMBS => {
                    let before = self.bombs.len();
                    // Scatter off the players' feet, with separation between persistent hazards.
                    for i in 0..if players.len() > 1 { 2 } else { 1 } {
                        let focus = players[i % players.len()].pos;
                        for _ in 0..16 {
                            let angle = rng.range(0.0, std::f32::consts::TAU);
                            let radius = rng.range(5.0, 14.0);
                            let pos = ground.spawn_spot(
                                focus + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius,
                                focus.y,
                            );
                            if players.iter().any(|p| flat(p.pos - pos).length() < 4.0)
                                || self
                                    .bombs
                                    .iter()
                                    .any(|b| flat(Vec3::from_array(b.pos) - pos).length() < 5.0)
                            {
                                continue;
                            }
                            tele.circle(pos, BOMB_RADIUS, now, 1.5);
                            self.bombs.push(Bomb {
                                id: run.guid(),
                                pos: pos.to_array(),
                                armed: now + 1.5,
                                active: false,
                                contacts: HashSet::new(),
                            });
                            break;
                        }
                    }
                    if self.bombs.len() == before {
                        // A crowded arena can reject every candidate. Retry while pursuing.
                        self.due[BOMBS] = now + 2.0;
                        return;
                    }
                    self.start(BOMBS, now, 1.5, 0.0, Attack::DropBomb);
                }
                SMOKE => {
                    fx.cast(entity, SPELLS[SMOKE], &[], None);
                    commands.entity(entity).insert(Visibility::Hidden);
                    self.start(
                        SMOKE,
                        now,
                        0.8,
                        damage * 1.3,
                        Attack::Smoke { peer: target.peer },
                    );
                }
                _ => unreachable!(),
            }
        } else {
            return;
        }
        windups.take(now);
        enemy.script_hold_until = self.pending.as_ref().map_or(0.0, Pending::hold_until);
    }
}

/// Hits can kill Morgan after his script has already run this frame. Clear his
/// hazards before the hero's death/victory step, including while he is vanished.
pub(super) fn cleanup(
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut bosses: Query<(Entity, &Enemy, &mut enemies::Script)>,
    mut tele: ResMut<Telegraphs>,
) {
    for (entity, enemy, mut script) in &mut bosses {
        if enemy.boss && enemy.def.name == "Morgan the Collector" && (enemy.dead || enemy.hp <= 0.0)
        {
            script.morgan.clear(&mut commands, entity, &mut tele);
            script.cooldowns.clear();
            run.morgan_bleed = Bleed::default();
            run.morgan_mark_until = 0.0;
            run.morgan_bomb_slow_until = 0.0;
        }
    }
}

fn in_cone(pos: Vec3, center: Vec3, dir: Vec3) -> bool {
    let delta = flat(pos - center);
    delta.length() <= CUT_RADIUS
        && (delta.length_squared() < 0.001
            || delta.normalize().dot(dir) >= std::f32::consts::FRAC_1_SQRT_2)
}

impl Pending {
    fn hold_until(&self) -> f32 {
        self.at
            + match &self.attack {
                Attack::Mark { .. } => 1.0,
                Attack::Smoke { .. } => 1.5,
                Attack::Bags { echo: true, .. } => 1.25,
                _ => 0.0,
            }
    }
    fn cancel(self, tele: &mut Telegraphs) {
        match self.attack {
            Attack::Cut { center, dir, .. } => tele.remove_cone(center, dir, CUT_RADIUS),
            Attack::Shot {
                center,
                dir,
                length,
            } => tele.remove_lane(center, center + dir * length, SHOT_HALF),
            Attack::Bags { centers, .. } => {
                for center in centers {
                    tele.remove_circle(center, BAG_RADIUS);
                }
            }
            _ => {}
        }
    }
}
