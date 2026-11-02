//! Boss and elite mechanics past Elwynn's native kits: a library of telegraphed moves, the phases
//! that add and drop them as a boss's health falls, the affixes elites roll, and the interpreter
//! that runs them against every living player. Elwynn's own bosses and guardians keep their ported
//! kits (`enemies::run_scripts`, `guardian_behavior`).
//!
//! Gold telegraphs are stepped out of and turn red in their last moments; green ones are stood
//! in; purple ones are stood in together.

use std::collections::HashMap;

use bevy::prelude::*;

use super::data::{self, EnemyDef};
use super::enemies::{self, Charge, Enemy};
use super::hero::{self, Hero};
use super::telegraphs::{Telegraphs, Tone};
use super::units::{flat, Fx, COLOR_HURT};
use super::{Rng, Run, Selection};

/// What a move does. Radii and lengths are yards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Verb {
    /// A ring around itself; it plants its feet while the circle fills.
    Stomp { radius: f32 },
    /// A blow thrown at the spot its target stands on.
    Throw { radius: f32 },
    /// `count` blasts scattered around its target, the first on it.
    Bombs { count: u8, radius: f32 },
    /// A rush down a marked lane through its target; `twice` rushes again from where it stops.
    Charge { twice: bool },
    /// A cone toward its target, 90 or 80 degrees wide.
    Cone { radius: f32, degrees: f32 },
    /// A pool at its target that burns on for `secs`.
    Pool { radius: f32, secs: f32 },
    /// Calls `count` of the horde to its side.
    Summon { count: u8 },
    /// A purple circle near its target that `need` players must stand in together: enough of them
    /// share a light blow, too few and every player takes a heavy one.
    Soak { radius: f32, need: u8 },
    /// Fire falls on all within `radius` of it but for `gaps` green circles.
    SafeZone { radius: f32, gaps: u8 },
    /// A wave out to `outer`: safe close in, within the green `inner` circle, or beyond it.
    Nova { inner: f32, outer: f32 },
    /// The arena closes in for `secs`: outside a green circle shrinking from `from` to `to`
    /// around where it stood, the ground burns every second.
    Closing { from: f32, to: f32, secs: f32 },
    /// `count` lanes through itself, one after another, each turned `step` degrees on.
    Beam { count: u8, length: f32, step: f32 },
    /// `count` lanes fanned at its target, `spread` degrees apart.
    Volley { count: u8, length: f32, spread: f32 },
    /// Drags every player near toward itself, then stomps.
    Vortex { radius: f32, pull: f32 },
    /// Runs its target down for `secs`, `haste` times as fast and unslowed.
    Fixate { secs: f32, haste: f32 },
    /// Untouchable until the `count` guards it calls are slain.
    Shield { count: u8 },
    /// Stands and heals `rate` of its health a second for `secs`, unless it takes `breaks` of its
    /// health first.
    Mend { rate: f32, secs: f32, breaks: f32 },
    /// Vanishes and steps out behind its target.
    Blink,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Move {
    /// Names the move for a later phase's [`Op::Remove`].
    pub id: u8,
    pub verb: Verb,
    /// The `Spell.dbc` row it plays.
    pub spell: u32,
    /// Seconds between uses, and before the first once its phase begins.
    pub every: f32,
    pub first: f32,
    /// Its blow, as a multiple of the caster's swing.
    pub power: f32,
}

pub(crate) const fn mv(id: u8, verb: Verb, spell: u32, every: f32, first: f32, power: f32) -> Move {
    Move {
        id,
        verb,
        spell,
        every,
        first,
        power,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Op {
    Add(Move),
    Remove(u8),
}

/// A boss's turn at a share of its health: what it says and how its moves change.
#[derive(Debug)]
pub(crate) struct Phase {
    pub below: f32,
    pub yell: &'static str,
    pub ops: &'static [Op],
}

#[derive(Debug)]
pub(crate) struct Kit {
    pub phases: &'static [Phase],
    /// Seconds after it arrives that it enrages, hitting harder and faster; 0 for never.
    pub enrage: f32,
}

/// The affixes an elite may roll, as bits.
pub(crate) mod affix {
    pub const MOLTEN: u16 = 1 << 0;
    pub const ARCANE: u16 = 1 << 1;
    pub const VORTEX: u16 = 1 << 2;
    pub const HUNTER: u16 = 1 << 3;
    pub const SUMMONER: u16 = 1 << 4;
    pub const BLINKER: u16 = 1 << 5;
    pub const JUGGERNAUT: u16 = 1 << 6;
    pub const VAMPIRIC: u16 = 1 << 7;
    pub const FRENZIED: u16 = 1 << 8;
}

const AFFIXES: [(u16, &str); 9] = [
    (affix::MOLTEN, "Molten"),
    (affix::ARCANE, "Arcane"),
    (affix::VORTEX, "Vortex"),
    (affix::HUNTER, "Hunter"),
    (affix::SUMMONER, "Summoner"),
    (affix::BLINKER, "Blinker"),
    (affix::JUGGERNAUT, "Juggernaut"),
    (affix::VAMPIRIC, "Vampiric"),
    (affix::FRENZIED, "Frenzied"),
];

/// `Spell.dbc` rows the mechanics play.
pub(crate) mod spell {
    pub const WAR_STOMP: u32 = 20549;
    pub const FLAMESTRIKE: u32 = 2120;
    pub const ARCANE_EXPLOSION: u32 = 1449;
    pub const MULTI_SHOT: u32 = 2643;
    pub const BLINK: u32 = 1953;
    pub const HURRICANE: u32 = 16914;
    pub const ENRAGE: u32 = 8599;
    pub const SHIELD: u32 = 17;
}

/// How many affixes a battleground's elites roll, by tier; Elwynn's keep none.
pub(crate) const fn affix_count(tier: u8) -> u8 {
    match tier {
        0..=2 => 1,
        3 | 4 => 2,
        _ => 3,
    }
}

/// `count` distinct affixes for the enemy with this serial: the same serial always rolls the same.
pub(super) fn draw(count: u8, serial: u64) -> u16 {
    let mut bits = 0;
    let mut seed = serial.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    let mut left = usize::from(count).min(AFFIXES.len());
    while left > 0 {
        seed ^= seed >> 33;
        seed = seed.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        seed ^= seed >> 29;
        let (bit, _) = AFFIXES[(seed % AFFIXES.len() as u64) as usize];
        if bits & bit == 0 {
            bits |= bit;
            left -= 1;
        }
    }
    bits
}

/// The affixes by name: "Molten, Arcane".
pub(super) fn affix_names(bits: u16) -> String {
    AFFIXES
        .iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// An encounter's name as its bar shows it, with its affixes or its ward.
pub(super) fn bar_name(e: &Enemy) -> String {
    if e.ward {
        format!("{}  ·  Shielded", e.def.name)
    } else if e.affixes != 0 {
        format!("{}  ·  {}", e.def.name, affix_names(e.affixes))
    } else {
        e.def.name.to_string()
    }
}

/// The moves an affix brings; the rest (Juggernaut, Vampiric, Frenzied) are passive.
fn affix_moves(bits: u16) -> Vec<Move> {
    use spell::*;
    let mut moves = Vec::new();
    for (bit, _) in AFFIXES {
        let m = match bits & bit {
            affix::MOLTEN => mv(0, Verb::Pool { radius: 4.0, secs: 5.0 }, FLAMESTRIKE, 10.0, 4.0, 0.6),
            affix::ARCANE => {
                mv(0, Verb::Bombs { count: 3, radius: 3.5 }, ARCANE_EXPLOSION, 11.0, 6.0, 0.9)
            }
            affix::VORTEX => mv(0, Verb::Vortex { radius: 7.0, pull: 6.0 }, WAR_STOMP, 16.0, 8.0, 1.1),
            affix::HUNTER => mv(0, Verb::Throw { radius: 3.5 }, MULTI_SHOT, 6.0, 3.0, 0.8),
            affix::SUMMONER => mv(0, Verb::Summon { count: 3 }, 0, 16.0, 5.0, 0.0),
            affix::BLINKER => mv(0, Verb::Blink, BLINK, 9.0, 4.0, 0.0),
            _ => continue,
        };
        moves.push(m);
    }
    moves
}

/// One of a boss's shield guards, by the boss it guards.
#[derive(Component)]
pub(super) struct Guard(pub Entity);

#[derive(Clone, Copy)]
struct Slot {
    mv: Move,
    wait: f32,
}

/// Where a blow lands, as the players' feet are tested against it.
#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle { c: Vec3, r: f32 },
    Cone { c: Vec3, dir: Vec3, r: f32, cos: f32 },
    Lane { from: Vec3, dir: Vec3, len: f32, half: f32 },
    /// Between `inner` and `outer` of `c`.
    Band { c: Vec3, inner: f32, outer: f32 },
    /// Within `r` of `c` but outside every gap.
    Field { c: Vec3, r: f32, gaps: [Vec3; 4], count: usize, gap: f32 },
    /// Beyond `r` of `c`.
    Outside { c: Vec3, r: f32 },
}

impl Shape {
    fn hits(&self, p: Vec3) -> bool {
        match *self {
            Shape::Circle { c, r } => flat(p - c).length() <= r,
            Shape::Cone { c, dir, r, cos } => {
                let d = flat(p - c);
                d.length() <= r && (d == Vec3::ZERO || d.normalize().dot(dir) >= cos)
            }
            Shape::Lane { from, dir, len, half } => {
                let d = flat(p - from);
                let along = d.dot(dir);
                (0.0..=len).contains(&along) && (d - dir * along).length() <= half
            }
            Shape::Band { c, inner, outer } => {
                let d = flat(p - c).length();
                d > inner && d <= outer
            }
            Shape::Field { c, r, gaps, count, gap } => {
                flat(p - c).length() <= r
                    && !gaps[..count].iter().any(|g| flat(p - *g).length() <= gap)
            }
            Shape::Outside { c, r } => flat(p - c).length() > r,
        }
    }
}

/// A blow on its way down.
#[derive(Clone, Copy)]
struct Impact {
    at: f32,
    shape: Shape,
    damage: f32,
    /// The visual it lands with, at `spot` or on its caster.
    spell: u32,
    spot: Option<Vec3>,
    /// A soak: how many players must share it.
    need: u8,
    /// Seconds it burns on once landed, a tick a second at `dot`.
    ticks: u32,
    dot: f32,
}

/// A lane marked later, for a beam's turns.
#[derive(Clone, Copy)]
struct Queued {
    at: f32,
    from: Vec3,
    dir: Vec3,
    len: f32,
    damage: f32,
}

struct Closing {
    center: Vec3,
    from: f32,
    to: f32,
    start: f32,
    end: f32,
    next: f32,
    damage: f32,
}

struct Mend {
    until: f32,
    rate: f32,
    need: f32,
    taken: f32,
}

/// A boss's or an affixed elite's script: its kit's phases and the moves they leave running,
/// with the blows on their way.
#[derive(Component)]
pub(super) struct Mechanics {
    kit: Option<&'static Kit>,
    next: usize,
    moves: Vec<Slot>,
    affixes: u16,
    born: Option<f32>,
    enraged: bool,
    frenzied: bool,
    busy: f32,
    second: Option<(f32, u32, f32)>,
    impacts: Vec<Impact>,
    queued: Vec<Queued>,
    closing: Option<Closing>,
    mend: Option<Mend>,
    fixate: Option<f32>,
    pull: Option<(f32, f32, f32)>,
    ward: Option<(f32, f32)>,
    speed: f32,
    last_hp: f32,
}

impl Mechanics {
    pub(super) fn boss(kit: &'static Kit) -> Self {
        Self::new(Some(kit), 0)
    }

    pub(super) fn elite(affixes: u16) -> Self {
        let mut m = Self::new(None, affixes);
        m.moves = affix_moves(affixes)
            .into_iter()
            .map(|mv| Slot { mv, wait: mv.first })
            .collect();
        m
    }

    fn new(kit: Option<&'static Kit>, affixes: u16) -> Self {
        Self {
            kit,
            next: 0,
            moves: Vec::new(),
            affixes,
            born: None,
            enraged: false,
            frenzied: false,
            busy: 0.0,
            second: None,
            impacts: Vec::new(),
            queued: Vec::new(),
            closing: None,
            mend: None,
            fixate: None,
            pull: None,
            ward: None,
            speed: 0.0,
            last_hp: f32::NAN,
        }
    }

    /// Takes every turn its health has reached, returning the lines it yells.
    fn turn(&mut self, share: f32) -> Vec<&'static str> {
        let Some(kit) = self.kit else {
            return Vec::new();
        };
        let mut yells = Vec::new();
        while let Some(phase) = kit.phases.get(self.next).filter(|p| share <= p.below) {
            self.next += 1;
            if !phase.yell.is_empty() {
                yells.push(phase.yell);
            }
            for op in phase.ops {
                match *op {
                    Op::Add(mv) => self.moves.push(Slot { mv, wait: mv.first }),
                    Op::Remove(id) => self.moves.retain(|s| s.mv.id != id),
                }
            }
        }
        yells
    }
}

/// A player a blow can reach.
#[derive(Clone, Copy)]
pub(super) struct Target {
    pub peer: u32,
    pub pos: Vec3,
    pub entity: Entity,
}

/// Every player standing: the local hero unless down, and each loaded guest.
pub(super) fn living(
    run: &Run,
    coop: &super::coop::Coop,
    hero: (Entity, Vec3),
    allies: &Query<(Entity, &super::coop::Ally)>,
) -> Vec<Target> {
    let mut out = Vec::new();
    let local = run.dead_t.is_none()
        && (!coop.running || coop.players.get(&0).is_none_or(|p| !p.paused));
    if local {
        out.push(Target {
            peer: 0,
            pos: hero.1,
            entity: hero.0,
        });
    }
    if coop.running {
        for (entity, ally) in allies {
            if let Some(p) = coop
                .players
                .get(&ally.0)
                .filter(|p| p.peer != 0 && p.loaded && !p.paused && p.hp > 0.0)
            {
                out.push(Target {
                    peer: p.peer,
                    pos: Vec3::from_array(p.pos),
                    entity,
                });
            }
        }
    }
    out
}

/// One blow on one player, routed to a guest through the host as every enemy blow is.
pub(super) fn strike(run: &mut Run, fx: &mut Fx, target: &Target, damage: f32) {
    run.coop_target = target.peer;
    let taken = hero::hurt_hero(run, damage);
    if taken > 0.0 {
        fx.number(target.entity, taken, false, COLOR_HURT);
    }
    run.coop_target = 0;
}

/// Who a landing blow strikes and how hard: a soak's share falls on those inside it when enough
/// stand there, and on everyone, harder, when too few do.
fn struck(shape: &Shape, need: u8, damage: f32, targets: &[Target]) -> Vec<(usize, f32)> {
    let inside: Vec<usize> = (0..targets.len())
        .filter(|&i| shape.hits(targets[i].pos))
        .collect();
    if need == 0 {
        return inside.into_iter().map(|i| (i, damage)).collect();
    }
    let need = usize::from(need).min(targets.len()).max(1);
    if inside.len() >= need {
        // Six tenths of the blow, split between them.
        let share = damage * 3.0 / (5.0 * inside.len() as f32);
        inside.into_iter().map(|i| (i, share)).collect()
    } else {
        (0..targets.len()).map(|i| (i, damage * 2.0)).collect()
    }
}

fn rotate(dir: Vec3, degrees: f32) -> Vec3 {
    Quat::from_rotation_y(degrees.to_radians()) * dir
}

/// A charge down a marked lane through `to`; returns how long it keeps the caster busy.
fn charge(en: &mut Enemy, pos: Vec3, to: Vec3, ground: &super::dungeon::Ground, tele: &mut Telegraphs, now: f32, damage: f32, spell: u32) -> f32 {
    const WINDUP: f32 = 1.1;
    const SPEED: f32 = 20.0;
    let way = flat(to - pos);
    let d = way.length().max(0.1);
    let end = ground.at(pos + way / d * (d + 4.0), to.y);
    let rush = (d + 4.0) / SPEED + 0.3;
    tele.lane(pos, end, en.radius.max(1.0) + 0.6, now, WINDUP, WINDUP + rush);
    en.charge = Some(Charge {
        to: end,
        spell,
        damage,
        windup: WINDUP,
        left: rush,
        hit: false,
    });
    WINDUP + rush
}

/// The horde rows called to a boss's side: the strongest few unlocked by now.
fn callable(roster: &'static data::Roster, now: f32) -> Vec<&'static EnemyDef> {
    let open: Vec<_> = roster.enemies.iter().filter(|d| d.from <= now).collect();
    let skip = open.len().saturating_sub(4);
    open.into_iter().skip(skip).collect()
}

/// Bosses and affixed elites take their turns, keep up their moves and land their blows on every
/// player standing in them.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn run_mechanics(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    ground: super::dungeon::Ground,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Enemy>)>,
    mut casters: Query<(Entity, &mut Transform, &mut Enemy, &mut Mechanics), Without<Hero>>,
    guards: Query<(&Guard, &Enemy), Without<Mechanics>>,
    mut fx: Fx,
    mut tele: ResMut<Telegraphs>,
    coop: Res<super::coop::Coop>,
    allies: Query<(Entity, &super::coop::Ally)>,
) {
    let dt = time.delta_secs();
    let Ok((hero_e, htf)) = hero.single() else {
        return;
    };
    let hpos = htf.translation;
    let now = run.elapsed;
    let roster = data::roster(selection.map);
    let targets = living(&run, &coop, (hero_e, hpos), &allies);
    let mut standing: HashMap<Entity, u32> = HashMap::new();
    for (guard, en) in &guards {
        if !en.dead {
            *standing.entry(guard.0).or_default() += 1;
        }
    }

    for (e, mut tf, mut en, mut m) in &mut casters {
        let m = &mut *m;
        let en = &mut *en;
        let pos = tf.translation;

        // Blows already falling land, their caster standing or not.
        let mut i = 0;
        while i < m.impacts.len() {
            let mut impact = m.impacts[i];
            if now < impact.at {
                i += 1;
                continue;
            }
            if impact.spell != 0 {
                fx.cast(e, impact.spell, &[], impact.spot);
                if impact.ticks > 0 {
                    if let Shape::Circle { c, r } = impact.shape {
                        tele.hazard(c, r, now, impact.ticks as f32);
                    }
                }
            }
            for (who, damage) in struck(&impact.shape, impact.need, impact.damage, &targets) {
                strike(&mut run, &mut fx, &targets[who], damage);
            }
            if impact.ticks > 0 {
                impact.ticks -= 1;
                impact.at += 1.0;
                impact.damage = impact.dot;
                impact.spell = 0;
                m.impacts[i] = impact;
                i += 1;
            } else {
                m.impacts.swap_remove(i);
            }
        }
        if en.dead {
            m.queued.clear();
            m.closing = None;
            m.pull = None;
            continue;
        }
        if en.asleep() {
            for slot in &mut m.moves {
                slot.wait = slot.mv.first;
            }
            m.last_hp = en.hp;
            continue;
        }
        let born = *m.born.get_or_insert(now);
        if m.speed <= 0.0 {
            m.speed = en.speed;
        }
        let lost = if m.last_hp.is_finite() {
            (m.last_hp - en.hp).max(0.0)
        } else {
            0.0
        };

        for line in m.turn(en.hp / en.max_hp) {
            debug!("survivors mechanics: {} turns at {:.0}%: {line}", en.def.name, en.hp / en.max_hp * 100.0);
            run.yell = Some((format!("{} yells: {line}", en.def.name), 4.0));
        }
        if let Some(kit) = m.kit {
            if kit.enrage > 0.0 && !m.enraged && now - born >= kit.enrage {
                // Its swings and its moves hit half again as hard, and its moves come faster.
                m.enraged = true;
                en.damage *= 1.5;
                fx.cast(e, spell::ENRAGE, &[], None);
                run.yell = Some((format!("{} becomes enraged!", en.def.name), 4.0));
            }
        }

        // The passive affixes.
        if m.affixes & affix::JUGGERNAUT != 0 {
            en.stun_t = 0.0;
            en.fear_t = 0.0;
            en.slow = en.slow.max(0.8);
        }
        if m.affixes & affix::FRENZIED != 0 && !m.frenzied && en.hp < en.max_hp * 0.5 {
            m.frenzied = true;
            m.speed *= 1.35;
            en.speed = m.speed;
            en.damage *= 1.25;
            fx.cast(e, spell::ENRAGE, &[], None);
        }

        // A ward holds while its guards stand; a moment's grace lets them arrive.
        if let Some((since, until)) = m.ward {
            let guarded = standing.get(&e).copied().unwrap_or(0);
            if now >= until || (guarded == 0 && now > since + 1.0) {
                debug!(
                    "survivors mechanics: {} ward falls at t={now:.0}, {}",
                    en.def.name,
                    if guarded == 0 { "its guards slain" } else { "timed out" }
                );
                m.ward = None;
                en.ward = false;
                run.yell = Some((format!("{}'s shield shatters!", en.def.name), 4.0));
            } else {
                en.ward = true;
            }
        }
        if let Some(mend) = m.mend.as_mut() {
            mend.taken += lost;
            if mend.taken >= mend.need {
                m.mend = None;
                en.root_t = 0.0;
                en.stun_t = en.stun_t.max(2.0);
                run.yell = Some((format!("{}'s mending is broken!", en.def.name), 4.0));
            } else if now >= mend.until {
                m.mend = None;
            } else {
                en.root_t = en.root_t.max(0.2);
                en.hp = (en.hp + en.max_hp * mend.rate * dt).min(en.max_hp);
            }
        }
        if m.fixate.is_some_and(|until| now >= until) {
            m.fixate = None;
            en.speed = m.speed;
        }
        if m.fixate.is_some() {
            en.slow = 1.0;
            en.slow_t = 0.0;
        }
        if let Some((until, strength, reach)) = m.pull {
            if now >= until {
                m.pull = None;
            } else {
                for t in &targets {
                    let way = flat(pos - t.pos);
                    let d = way.length();
                    if d > reach || d < 2.0 {
                        continue;
                    }
                    let step = way / d * (strength * dt).min(d - 2.0);
                    if t.peer == 0 {
                        run.push += step;
                    } else if let Some(entry) = run.coop_push.iter_mut().find(|(p, _)| *p == t.peer) {
                        entry.1 += step;
                    } else {
                        run.coop_push.push((t.peer, step));
                    }
                }
            }
        }
        if let Some(c) = m.closing.as_mut() {
            if now >= c.end {
                m.closing = None;
            } else if now >= c.next {
                let k = ((now - c.start) / (c.end - c.start)).clamp(0.0, 1.0);
                let r = c.from + (c.to - c.from) * k;
                tele.hazard(c.center, r, now, 1.05);
                tele.tone(Tone::Green);
                m.impacts.push(Impact {
                    at: now + 1.0,
                    shape: Shape::Outside { c: c.center, r },
                    damage: c.damage,
                    spell: 0,
                    spot: None,
                    need: 0,
                    ticks: 0,
                    dot: 0.0,
                });
                c.next += 1.0;
            }
        }
        let mut k = 0;
        while k < m.queued.len() {
            let q = m.queued[k];
            if now < q.at {
                k += 1;
                continue;
            }
            tele.lane(q.from, q.from + q.dir * q.len, 2.2, now, 1.0, 1.0);
            m.impacts.push(Impact {
                at: now + 1.0,
                shape: Shape::Lane {
                    from: q.from,
                    dir: q.dir,
                    len: q.len,
                    half: 2.2,
                },
                damage: q.damage,
                spell: 0,
                spot: None,
                need: 0,
                ticks: 0,
                dot: 0.0,
            });
            m.queued.swap_remove(k);
        }
        m.last_hp = en.hp;

        let (_, tpos, _, down) = coop.target(pos, (hero_e, hpos), run.dead_t.is_none(), &allies);
        if down || targets.is_empty() {
            continue;
        }
        let power = en.damage * en.weaken;
        let pace = if m.enraged { 0.6 } else { 1.0 };
        if let Some((at, spell, damage)) = m.second {
            if now >= at && en.charge.is_none() {
                m.second = None;
                if en.stun_t <= 0.0 {
                    m.busy = now + charge(en, pos, tpos, &ground, &mut tele, now, damage, spell);
                }
            }
        }
        let mut free = en.charge.is_none()
            && now >= m.busy
            && en.stun_t <= 0.0
            && en.fear_t <= 0.0
            && m.mend.is_none();
        let dist = flat(tpos - pos).length();
        let toward = flat(tpos - pos).normalize_or(Vec3::X);
        for slot in 0..m.moves.len() {
            m.moves[slot].wait -= dt;
            if m.moves[slot].wait > 0.0 {
                continue;
            }
            let Move { verb, spell, every, power: scale, .. } = m.moves[slot].mv;
            let support = matches!(verb, Verb::Summon { .. } | Verb::Shield { .. } | Verb::Fixate { .. });
            if !support && !free {
                continue;
            }
            // Out of reach, it tries again shortly.
            let retry = match verb {
                Verb::Stomp { radius } | Verb::Vortex { radius, .. } => dist > radius * 2.0 + 6.0,
                Verb::Cone { radius, .. } => dist > radius + 4.0,
                Verb::Charge { .. } => !(4.0..=30.0).contains(&dist) || en.root_t > 0.0,
                Verb::Blink => dist < 6.0 || dist > 40.0,
                Verb::Throw { .. } | Verb::Bombs { .. } | Verb::Pool { .. } => dist > 32.0,
                Verb::Shield { .. } => m.ward.is_some(),
                Verb::Closing { .. } => m.closing.is_some(),
                Verb::Fixate { .. } => m.fixate.is_some(),
                _ => false,
            };
            if retry {
                m.moves[slot].wait = 1.0;
                continue;
            }
            m.moves[slot].wait = every * pace;
            let damage = power * scale;
            debug!("survivors mechanics: {} {:?} for {damage:.0} at t={now:.0}", en.def.name, verb);
            let land = |m: &mut Mechanics, at: f32, shape: Shape, spell: u32, spot: Option<Vec3>| {
                m.impacts.push(Impact {
                    at,
                    shape,
                    damage,
                    spell,
                    spot,
                    need: 0,
                    ticks: 0,
                    dot: 0.0,
                });
            };
            match verb {
                Verb::Stomp { radius } => {
                    tele.circle(pos, radius, now, 1.2);
                    en.root_t = en.root_t.max(1.3);
                    m.busy = now + 1.3;
                    land(m, now + 1.2, Shape::Circle { c: pos, r: radius }, spell, None);
                }
                Verb::Throw { radius } => {
                    let spot = ground.at(tpos, tpos.y);
                    let flight = (dist / 14.0).clamp(1.2, 1.8);
                    fx.cast(e, spell, &[], Some(spot));
                    tele.circle(spot, radius, now, flight);
                    m.busy = now + 0.5;
                    land(m, now + flight, Shape::Circle { c: spot, r: radius }, 0, None);
                }
                Verb::Bombs { count, radius } => {
                    for n in 0..count {
                        let spot = if n == 0 {
                            tpos
                        } else {
                            let a = rng.range(0.0, std::f32::consts::TAU);
                            tpos + Vec3::new(a.cos(), 0.0, a.sin()) * rng.range(3.0, 7.0)
                        };
                        let spot = ground.at(spot, tpos.y);
                        tele.circle(spot, radius, now, 1.5);
                        land(m, now + 1.5, Shape::Circle { c: spot, r: radius }, spell, Some(spot));
                    }
                    m.busy = now + 0.8;
                }
                Verb::Charge { twice } => {
                    let busy = charge(en, pos, tpos, &ground, &mut tele, now, damage, spell);
                    m.busy = now + busy;
                    if twice {
                        m.second = Some((now + busy + 0.5, spell, damage));
                    }
                }
                Verb::Cone { radius, degrees } => {
                    let half = (degrees * 0.5).to_radians();
                    tele.cone(pos, toward, radius, half, now, 1.1);
                    fx.cast(e, spell, &[], None);
                    en.root_t = en.root_t.max(1.3);
                    m.busy = now + 1.3;
                    let cos = half.cos();
                    land(m, now + 1.1, Shape::Cone { c: pos, dir: toward, r: radius, cos }, 0, None);
                }
                Verb::Pool { radius, secs } => {
                    let spot = ground.at(tpos, tpos.y);
                    tele.circle(spot, radius, now, 1.4);
                    m.busy = now + 0.6;
                    m.impacts.push(Impact {
                        at: now + 1.4,
                        shape: Shape::Circle { c: spot, r: radius },
                        damage,
                        spell,
                        spot: Some(spot),
                        need: 0,
                        ticks: secs.round() as u32,
                        dot: damage * 0.35,
                    });
                }
                Verb::Summon { count } => {
                    let rows = callable(roster, now);
                    for n in 0..count {
                        if rows.is_empty() {
                            break;
                        }
                        let def = rows[rng.index(rows.len())];
                        let a = n as f32 / f32::from(count) * std::f32::consts::TAU;
                        let at = ground.spawn_spot(pos + Vec3::new(a.cos(), 0.0, a.sin()) * 4.0, pos.y);
                        enemies::spawn_enemy(&mut commands, &mut run, def, at, tpos, false);
                    }
                }
                Verb::Soak { radius, need } => {
                    let a = rng.range(0.0, std::f32::consts::TAU);
                    let spot = ground.at(tpos + Vec3::new(a.cos(), 0.0, a.sin()) * 7.0, tpos.y);
                    tele.circle(spot, radius, now, 3.0);
                    tele.tone(Tone::Purple);
                    fx.cast(e, spell, &[], None);
                    m.busy = now + 1.0;
                    m.impacts.push(Impact {
                        at: now + 3.0,
                        shape: Shape::Circle { c: spot, r: radius + 0.5 },
                        damage,
                        spell,
                        spot: Some(spot),
                        need,
                        ticks: 0,
                        dot: 0.0,
                    });
                }
                Verb::SafeZone { radius, gaps } => {
                    const GAP: f32 = 3.0;
                    let count = usize::from(gaps).clamp(1, 4);
                    let mut spots = [Vec3::ZERO; 4];
                    for (n, spot) in spots.iter_mut().take(count).enumerate() {
                        let a = rng.range(0.0, std::f32::consts::TAU);
                        // The first gap is a short run from the target; the rest anywhere inside.
                        let at = if n == 0 {
                            tpos + Vec3::new(a.cos(), 0.0, a.sin()) * rng.range(5.0, 8.0)
                        } else {
                            pos + Vec3::new(a.cos(), 0.0, a.sin()) * rng.range(4.0, radius * 0.8)
                        };
                        *spot = ground.at(at, pos.y);
                    }
                    tele.circle(pos, radius, now, 3.0);
                    for spot in &spots[..count] {
                        tele.circle(*spot, GAP, now, 3.0);
                        tele.tone(Tone::Green);
                    }
                    fx.cast(e, spell, &[], None);
                    m.busy = now + 1.2;
                    let field = Shape::Field { c: pos, r: radius, gaps: spots, count, gap: GAP + 0.5 };
                    land(m, now + 3.0, field, spell, Some(pos));
                }
                Verb::Nova { inner, outer } => {
                    tele.circle(pos, outer, now, 2.0);
                    tele.circle(pos, inner, now, 2.0);
                    tele.tone(Tone::Green);
                    en.root_t = en.root_t.max(2.2);
                    m.busy = now + 2.2;
                    land(m, now + 2.0, Shape::Band { c: pos, inner, outer }, spell, None);
                }
                Verb::Closing { from, to, secs } => {
                    fx.cast(e, spell, &[], None);
                    m.closing = Some(Closing {
                        center: pos,
                        from,
                        to,
                        start: now,
                        end: now + secs,
                        next: now,
                        damage: damage * 0.5,
                    });
                }
                Verb::Beam { count, length, step } => {
                    for n in 0..count {
                        let dir = rotate(toward, step * f32::from(n));
                        m.queued.push(Queued {
                            at: now + 0.8 * f32::from(n),
                            from: pos - dir * length,
                            dir,
                            len: length * 2.0,
                            damage,
                        });
                    }
                    let busy = 0.8 * f32::from(count) + 1.2;
                    fx.cast(e, spell, &[], None);
                    en.root_t = en.root_t.max(busy);
                    m.busy = now + busy;
                }
                Verb::Volley { count, length, spread } => {
                    let first = -spread * (f32::from(count) - 1.0) * 0.5;
                    for n in 0..count {
                        let dir = rotate(toward, first + spread * f32::from(n));
                        tele.lane(pos, pos + dir * length, 1.8, now, 1.3, 1.3);
                        land(m, now + 1.3, Shape::Lane { from: pos, dir, len: length, half: 1.8 }, 0, None);
                    }
                    fx.cast(e, spell, &[], None);
                    en.root_t = en.root_t.max(1.4);
                    m.busy = now + 1.4;
                }
                Verb::Vortex { radius, pull } => {
                    fx.cast(e, spell::HURRICANE, &[], None);
                    tele.circle(pos, radius, now, 1.8);
                    m.pull = Some((now + 1.6, pull, radius * 3.0));
                    en.root_t = en.root_t.max(1.9);
                    m.busy = now + 1.9;
                    land(m, now + 1.8, Shape::Circle { c: pos, r: radius }, spell, None);
                }
                Verb::Fixate { secs, haste } => {
                    m.fixate = Some(now + secs);
                    en.speed = m.speed * haste;
                    fx.cast(e, spell, &[], None);
                    run.yell = Some((format!("{} fixes its eyes on its prey!", en.def.name), 3.0));
                }
                Verb::Shield { count } => {
                    let rows = callable(roster, now);
                    let Some(def) = rows.last().copied() else {
                        continue;
                    };
                    // Guards come whatever the horde's crowd: the ward must have someone to fall.
                    let slots = run.horde_spawn_slots.take();
                    for n in 0..count {
                        let a = n as f32 / f32::from(count) * std::f32::consts::TAU;
                        let at = ground.spawn_spot(pos + Vec3::new(a.cos(), 0.0, a.sin()) * 5.0, pos.y);
                        let guard = enemies::spawn_enemy(&mut commands, &mut run, def, at, tpos, false);
                        if guard != Entity::PLACEHOLDER {
                            commands
                                .entity(guard)
                                .insert(Guard(e))
                                .entry::<Enemy>()
                                .and_modify(|mut g| g.guard = true);
                        }
                    }
                    run.horde_spawn_slots = slots;
                    m.ward = Some((now, now + 30.0));
                    en.ward = true;
                    fx.cast(e, spell::SHIELD, &[], None);
                    run.yell = Some((format!("{} hides behind its guards! Slay them!", en.def.name), 4.0));
                }
                Verb::Mend { rate, secs, breaks } => {
                    m.mend = Some(Mend {
                        until: now + secs,
                        rate,
                        need: en.max_hp * breaks,
                        taken: 0.0,
                    });
                    en.root_t = en.root_t.max(secs);
                    tele.hazard(pos, 4.0, now, secs);
                    tele.tone(Tone::Green);
                    fx.cast(e, spell, &[], None);
                    m.busy = now + secs;
                    run.yell = Some((format!("{} begins to mend its wounds! Break it!", en.def.name), 4.0));
                }
                Verb::Blink => {
                    let behind = flat(tpos - pos).normalize_or(Vec3::X) * 3.5;
                    let spot = ground.at(tpos + behind, tpos.y);
                    fx.cast(e, spell, &[], None);
                    tf.translation = spot;
                    tf.rotation = Quat::from_rotation_y(super::units::yaw_toward(-behind));
                    en.charge = None;
                    en.relocated(spot, tpos);
                    m.busy = now + 0.6;
                }
            }
            if !support {
                free = false;
            }
        }
    }
    run.coop_target = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD: Op = Op::Add(mv(1, Verb::Stomp { radius: 7.0 }, 0, 9.0, 2.0, 1.0));
    const LATER: Op = Op::Add(mv(2, Verb::Blink, 0, 8.0, 0.0, 0.0));
    static KIT: Kit = Kit {
        phases: &[
            Phase { below: 1.0, yell: "", ops: &[ADD] },
            Phase { below: 0.5, yell: "Now!", ops: &[Op::Remove(1), LATER] },
        ],
        enrage: 0.0,
    };

    #[test]
    fn phases_add_and_drop_moves_as_health_falls() {
        let mut m = Mechanics::boss(&KIT);
        assert!(m.turn(1.0).is_empty());
        assert_eq!(m.moves.iter().map(|s| s.mv.id).collect::<Vec<_>>(), [1]);
        assert_eq!(m.moves[0].wait, 2.0);
        assert!(m.turn(0.7).is_empty());
        assert_eq!(m.turn(0.3), ["Now!"]);
        assert_eq!(m.moves.iter().map(|s| s.mv.id).collect::<Vec<_>>(), [2]);
        // A turn is taken once.
        assert!(m.turn(0.1).is_empty());
    }

    #[test]
    fn shapes_test_the_ground_they_mark() {
        let o = Vec3::ZERO;
        let band = Shape::Band { c: o, inner: 4.0, outer: 12.0 };
        assert!(!band.hits(Vec3::X * 3.0) && band.hits(Vec3::X * 8.0) && !band.hits(Vec3::X * 13.0));
        let mut gaps = [Vec3::ZERO; 4];
        gaps[0] = Vec3::X * 6.0;
        let field = Shape::Field { c: o, r: 15.0, gaps, count: 1, gap: 3.5 };
        assert!(!field.hits(Vec3::X * 7.0) && field.hits(Vec3::Z * 7.0) && !field.hits(Vec3::Z * 16.0));
        let lane = Shape::Lane { from: o, dir: Vec3::X, len: 10.0, half: 2.0 };
        assert!(lane.hits(Vec3::new(5.0, 0.0, 1.5)) && !lane.hits(Vec3::new(5.0, 0.0, 2.5)));
        assert!(!lane.hits(Vec3::new(-1.0, 0.0, 0.0)) && !lane.hits(Vec3::new(11.0, 0.0, 0.0)));
        let cone = Shape::Cone { c: o, dir: Vec3::X, r: 8.0, cos: 45f32.to_radians().cos() };
        assert!(cone.hits(Vec3::new(5.0, 0.0, 4.0)) && !cone.hits(Vec3::new(1.0, 0.0, 4.0)));
        assert!(Shape::Outside { c: o, r: 10.0 }.hits(Vec3::Z * 11.0));
    }

    fn at(x: f32) -> Target {
        Target {
            peer: x as u32,
            pos: Vec3::X * x,
            entity: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn a_soak_is_shared_by_enough_players_and_punishes_too_few() {
        let spot = Shape::Circle { c: Vec3::ZERO, r: 3.0 };
        // Alone, the one player standing in it is enough.
        assert_eq!(struck(&spot, 2, 100.0, &[at(0.0)]), [(0, 60.0)]);
        assert_eq!(struck(&spot, 1, 100.0, &[at(9.0)]), [(0, 200.0)]);
        // Two needed, one in: everyone takes the heavy blow.
        let party = [at(0.0), at(9.0), at(20.0)];
        assert_eq!(struck(&spot, 2, 100.0, &party), [(0, 200.0), (1, 200.0), (2, 200.0)]);
        let party = [at(0.0), at(1.0), at(20.0)];
        assert_eq!(struck(&spot, 2, 100.0, &party), [(0, 30.0), (1, 30.0)]);
        // An ordinary blow strikes who stands in it.
        assert_eq!(struck(&spot, 0, 100.0, &party), [(0, 100.0), (1, 100.0)]);
    }

    #[test]
    fn affixes_draw_distinct_and_repeatable() {
        for serial in 0..200u64 {
            for count in 1..=3u8 {
                let bits = draw(count, serial);
                assert_eq!(bits.count_ones(), u32::from(count));
                assert_eq!(bits, draw(count, serial));
            }
        }
        assert_eq!(draw(0, 7), 0);
        assert_eq!(affix_names(affix::MOLTEN | affix::VAMPIRIC), "Molten, Vampiric");
        // Every active affix brings a move; the passive ones bring none.
        assert_eq!(affix_moves(affix::MOLTEN | affix::BLINKER).len(), 2);
        assert!(affix_moves(affix::JUGGERNAUT | affix::VAMPIRIC | affix::FRENZIED).is_empty());
    }
}
