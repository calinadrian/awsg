//! ASR's Hogger encounter: pursue, close the gap, commit to a readable blow, recover.
//! Geometry and direction lock when a warning starts; missed blows do not follow the player.
use bevy::prelude::*;

use super::{
    boss_cooldowns::Timer,
    boss_pacing::Windups,
    dungeon::Ground,
    enemies::{Charge, Enemy},
    mechanics::{self, Target},
    telegraphs::Telegraphs,
    units::{flat, yaw_toward, Fx},
    Run,
};

const CLEAVE: usize = 0;
const CHARGE: usize = 1;
const STOMP: usize = 2;
const ROCK: usize = 3;
const SPELLS: [u32; 4] = [15496, 100, 20549, 4164];
const NAMES: [&str; 4] = ["Cleave", "Charge", "War Stomp", "Throw Rock"];
const PERIODS: [f32; 4] = [4.5, 7.5, 12.0, 12.0];
pub(super) const FRENZY_NAME: &str = "Blood Frenzy";
pub(super) const FRENZY_ICON: u32 = 12292;
pub(super) const FRENZY_DESCRIPTION: &str = "Triggered at 50% health: deals 50% more damage, moves 20% faster, uses abilities more often and follows charges with a cleave. Grows in size and turns red. Stacks with the three-minute Berserk.";
const CLEAVE_REACH: f32 = 10.0;
const STOMP_REACH: f32 = 5.5;

#[derive(Component)]
struct EnrageGrowth;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (grow_enraged, tint_enraged)
            .chain()
            .after(super::enemies::run_scripts)
            .after(super::enemies::cap_heights)
            .after(crate::aura_visual::apply_aura_tint),
    );
}

/// Grow once after the normal model-height cap, without changing attack geometry.
#[allow(clippy::type_complexity)]
fn grow_enraged(
    mut commands: Commands,
    mut bosses: Query<(
        Entity,
        &Enemy,
        &mut crate::net::NetEntity,
        &mut crate::net::ObjectStore,
        &mut Transform,
        Has<EnrageGrowth>,
        Option<&crate::entities::StandBoxHeight>,
    )>,
) {
    for (entity, enemy, mut net, mut store, mut tf, grown, stand) in &mut bosses {
        if !enemy.hogger_enraged || grown || stand.is_none() {
            continue;
        }
        net.scale *= 1.15;
        tf.scale = Vec3::splat(net.scale);
        store.0.merge(benilla_protocol::ObjectFields::from_pairs(&[(
            4,
            net.scale.to_bits(),
        )]));
        commands.entity(entity).insert(EnrageGrowth);
    }
}

/// The aura tint publisher resets slots each frame; put Hogger's encounter tint after it.
/// Follow the same model-owner chain so his attached rigged parts receive the red too.
fn tint_enraged(
    bosses: Query<&Enemy>,
    rigs: Query<(Entity, &benilla_world::rig_palette::RigSkin)>,
    parents: Query<&benilla_world::model_fade::ParentModel>,
    mut tints: ResMut<benilla_world::instance_tint::InstanceTints>,
) {
    for (entity, rig) in &rigs {
        let mut owner = entity;
        for _ in 0..benilla_world::model_fade::MAX_MODEL_CHAIN {
            if bosses.get(owner).is_ok_and(|e| e.hogger_enraged) {
                tints.set(rig.slot, benilla_world::instance_tint::pack([255, 95, 85]));
                break;
            }
            let Ok(parent) = parents.get(owner) else {
                break;
            };
            owner = parent.0;
        }
    }
}

struct Blow {
    slot: usize,
    at: f32,
    from: f32,
    center: Vec3,
    direction: Option<Vec3>,
    radius: f32,
    damage: f32,
}

#[derive(Default)]
pub(super) struct Script {
    born: Option<f32>,
    due: [f32; 4],
    pending: Option<Blow>,
    busy: f32,
    charging: bool,
    combo_until: Option<f32>,
    far_since: Option<f32>,
    close_since: Option<f32>,
    enraged: bool,
    charge_from: f32,
    charge_until: f32,
}

impl Script {
    pub(super) fn enrage_pending(&mut self) {
        if let Some(pending) = self.pending.as_mut() { pending.damage *= super::boss_enrage::DAMAGE; }
    }
    pub(super) fn timers(&self, now: f32) -> Vec<Timer> {
        if self.born.is_none() {
            return Vec::new();
        }
        (0..4)
            .map(|slot| {
                let cast = self
                    .pending
                    .as_ref()
                    .filter(|b| b.slot == slot && b.at > now);
                let charging = slot == CHARGE && self.charging && self.charge_until > now;
                let due = if slot == CLEAVE && self.combo_until.is_some() {
                    now
                } else {
                    self.due[slot]
                };
                Timer {
                    spell: SPELLS[slot],
                    name: NAMES[slot].to_string(),
                    due,
                    period: self.period(slot),
                    cast_until: cast
                        .map_or(if charging { self.charge_until } else { 0.0 }, |b| b.at),
                    cast_duration: cast.map_or(
                        if charging {
                            self.charge_until - self.charge_from
                        } else {
                            0.0
                        },
                        |b| b.at - b.from,
                    ),
                    ready: due <= now,
                }
            })
            .collect()
    }

    fn period(&self, slot: usize) -> f32 {
        PERIODS[slot]
            * if self.enraged && slot != ROCK {
                0.8
            } else {
                1.0
            }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn tick(
        &mut self,
        entity: Entity,
        enemy: &mut Enemy,
        tf: &mut Transform,
        target: Vec3,
        players: &[Target],
        now: f32,
        scale: f32,
        run: &mut Run,
        ground: &Ground,
        fx: &mut Fx,
        tele: &mut Telegraphs,
        windups: &mut Windups,
    ) {
        if enemy.dead {
            self.pending = None;
            self.charging = false;
            return;
        }
        if self.born.is_none() {
            self.born = Some(now);
            self.due = [now + 2.0, now + 3.0, now + 8.0, now + 10.0];
        }
        if !self.enraged && enemy.hp <= enemy.max_hp * 0.5 {
            self.enraged = true;
            enemy.hogger_enraged = true;
            enemy.damage *= 1.5;
            if let Some(blow) = self.pending.as_mut() {
                blow.damage *= 1.5;
            }
            if let Some(charge) = enemy.charge.as_mut() {
                charge.damage *= 1.5;
            }
            fx.cast(entity, 8599, &[], None);
            run.yell = Some(("Hogger yells: Grrrr! You no run from Hogger!".into(), 4.0));
            // The new phase accelerates existing clocks, without interrupting a committed blow.
            for slot in 0..3 {
                self.due[slot] = now + (self.due[slot] - now).max(0.0).min(self.period(slot));
            }
        }
        // Enrage affects every attack, including the custom kit as well as ordinary swings.
        let scale = scale * if self.enraged { 1.5 } else { 1.0 };
        // The chase applies 0.8x: 7.4 yd/s normally, 8.88 yd/s enraged.
        enemy.speed = enemy.def.speed * if self.enraged { 1.2 } else { 1.0 };
        let pos = tf.translation;
        let distance = flat(target - pos).length();
        if distance >= 18.0 && !players.is_empty() {
            self.far_since.get_or_insert(now);
        } else {
            self.far_since = None;
        }
        if distance <= STOMP_REACH && !players.is_empty() {
            self.close_since.get_or_insert(now);
        } else {
            self.close_since = None;
        }

        // Stuns and fears interrupt a pending melee blow; its old warning deals no damage.
        if enemy.stun_t > 0.0 || enemy.fear_t > 0.0 {
            self.pending = None;
            self.combo_until = None;
            self.charge_until = 0.0;
            self.charging = false;
            return;
        }
        if self.pending.as_ref().is_some_and(|b| now >= b.at) {
            let blow = self.pending.take().unwrap();
            if blow.slot != ROCK {
                fx.cast(entity, SPELLS[blow.slot], &[], None);
            }
            let previous = run.coop_target;
            for player in players {
                let offset = flat(player.pos - blow.center);
                let in_shape = offset.length() <= blow.radius
                    && blow.direction.is_none_or(|dir| {
                        offset.length_squared() < 0.001
                            || offset.normalize().dot(dir) >= std::f32::consts::FRAC_1_SQRT_2
                    });
                if in_shape {
                    mechanics::strike(run, fx, player, blow.damage);
                }
            }
            run.coop_target = previous;
        }
        if self.charging && enemy.charge.is_none() {
            self.charging = false;
            self.charge_until = 0.0;
            let near = distance <= CLEAVE_REACH;
            self.busy = now + 0.15;
            // Resume pursuit immediately; a close landing sets up the enraged combo.
            enemy.attack_cd = enemy.attack_cd.max(0.2);
            self.combo_until = (self.enraged && near).then_some(self.busy + 1.0);
        }
        if enemy.charge.is_some() || self.pending.is_some() || now < self.busy || players.is_empty()
        {
            return;
        }
        if self
            .combo_until
            .is_some_and(|until| now > until || distance > CLEAVE_REACH)
        {
            self.combo_until = None;
        }
        if enemy.root_t > 0.0 || !windups.available(now) {
            return;
        }

        let nearby = players
            .iter()
            .filter(|p| flat(p.pos - pos).length() <= STOMP_REACH)
            .count();
        let crowded = nearby >= 2;
        let camping = self.close_since.is_some_and(|since| now - since >= 4.0);
        let slot = if self.combo_until.is_some() && distance <= CLEAVE_REACH {
            CLEAVE
        } else if self.due[CHARGE] <= now && (8.0..=26.0).contains(&distance) {
            CHARGE
        } else if self.due[STOMP] <= now && nearby > 0 && (crowded || camping) {
            STOMP
        } else if self.due[CLEAVE] <= now && distance <= CLEAVE_REACH {
            CLEAVE
        } else if self.due[ROCK] <= now
            && (18.0..=32.0).contains(&distance)
            && self.far_since.is_some_and(|since| now - since >= 3.0)
        {
            ROCK
        } else {
            return;
        }; // Chase and auto-attack normally while no appropriate move is ready.

        windups.take(now);
        self.due[slot] = now + self.period(slot);
        if slot == CHARGE {
            let dir = flat(target - pos).normalize_or(Vec3::Z);
            let to = ground.at(pos + dir * (distance + 2.5), target.y);
            let windup = 1.1;
            let rush = (distance + 2.5) / 20.0 + 0.3;
            tele.lane(
                pos,
                to,
                enemy.radius.max(1.0) + 0.5,
                now,
                windup,
                windup + rush,
            );
            enemy.charge = Some(Charge {
                boar_impact: None,
                to,
                spell: SPELLS[CHARGE],
                damage: 20.0 * scale,
                windup,
                left: rush,
                hit: false,
            });
            self.charging = true;
            self.charge_from = now;
            self.charge_until = now + windup;
            self.combo_until = None;
            self.close_since = None;
            return;
        }
        let (windup, radius, damage, direction, center) = match slot {
            CLEAVE => {
                let dir = flat(target - pos).normalize_or(Vec3::Z);
                let windup = if self.combo_until.take().is_some() {
                    1.0
                } else {
                    1.15
                };
                tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
                tele.cone(
                    pos,
                    dir,
                    CLEAVE_REACH,
                    std::f32::consts::FRAC_PI_4,
                    now,
                    windup,
                );
                (windup, CLEAVE_REACH, 24.0, Some(dir), pos)
            }
            STOMP => {
                tele.circle(pos, STOMP_REACH, now, 1.3);
                self.close_since = None;
                (1.3, STOMP_REACH, 20.0, None, pos)
            }
            _ => {
                let at = ground.at(target, target.y);
                let flight = (distance / 14.0).clamp(1.3, 1.9);
                fx.cast(entity, SPELLS[ROCK], &[], Some(at));
                tele.circle(at, 3.5, now, flight);
                self.far_since = Some(now);
                (flight, 3.5, 18.0, None, at)
            }
        };
        let recovery = if self.enraged { 0.25 } else { 0.45 };
        self.busy = now + windup + recovery;
        // Commit to melee warnings, then chase during recovery. A released rock does not
        // hold him stationary for its whole flight.
        enemy.root_t = enemy.root_t.max(if slot == ROCK { 0.3 } else { windup });
        enemy.attack_cd = enemy.attack_cd.max(self.busy - now);
        self.pending = Some(Blow {
            slot,
            at: now + windup,
            from: now,
            center,
            direction,
            radius,
            damage: damage * scale,
        });
    }
}
