//! Talents, as the old ASR kept them: three trees a class, read out of its tables
//! ([`super::talents_data`]), bought with points that every run's talent experience earns the
//! class, and spent between runs. A row opens at five points spent in its tree; a talent with a
//! prerequisite needs it maxed; respecs are free.
//!
//! In a run the talents count through [`Build`]: stat bonuses, spell and school modifiers, the
//! spells a run starts with and the procs that ready a spell again. The milestones and capstones
//! whose mechanics the mode has no counterpart for yet (`Eff::Other`) are shown, not applied.

use std::collections::HashMap;

use bevy::prelude::*;

use super::data::{AbilityDef, Kind};
use super::hud::{HudFont, MenuRoot};
use super::menu::MenuAction;
use super::{Phase, Run, Selection};
use crate::glue::art::{GlueArt, GOLD};
use crate::glue::widgets::{glue_button, GlueBtnKind};
use crate::glue::GlueClicks;

/// One talent, as the old tables hold it.
pub(super) struct TalentDef {
    pub key: &'static str,
    pub name: &'static str,
    /// `Interface\Icons\<icon>.blp`.
    pub icon: &'static str,
    /// Its text, `{v}` standing for its value at the rank shown.
    pub text: &'static str,
    /// The row (0-6, a row opening at five points spent each) and column (0-3).
    pub tier: u8,
    pub col: u8,
    pub ranks: u8,
    /// A talent that must be maxed first.
    pub prereq: Option<&'static str>,
    /// `{v}` per rank: a share (shown as a percent) unless `flat`.
    pub v: f32,
    pub flat: bool,
    pub effects: &'static [Eff],
}

#[allow(dead_code)]
pub(super) struct TreeDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub talents: &'static [TalentDef],
}

/// What a talent does, per rank.
#[allow(dead_code)]
pub(super) enum Eff {
    /// A stat of the old game's: 0 damage, 1 cooldown recovery, 2 maximum health, 3 critical
    /// chance, 4 critical damage, 5 damage taken, 6 health a second, 7 movement speed, 13 a
    /// spell striking twice, 14 leech, 15 health a kill, 18-21 damage against the crowd
    /// controlled, elites and bosses, the wounded and the afflicted, 22 melee thorns, 24 health
    /// a second as a share of the maximum. Others (pets, buffs, luck) have no counterpart yet.
    Stat(u32, f32),
    /// A modifier on the spells `target` picks: `what` 0 damage, 1 cooldown, 2 targets, 3 area,
    /// 4 duration, 5 extra damage as a share of the hero's maximum health.
    Mod { target: Target, what: u32, per: f32 },
    /// The run starts with this spell.
    Grant(&'static str),
    /// A `chance` (per `every` seconds at most) to ready `cast` again when `trigger` happens:
    /// 2 a critical strike, 3 or 4 a cast of a spell `on`, 7 any cast, 8 a melee blow taken,
    /// 9 health falling below `arg`.
    Proc {
        chance: f32,
        every: f32,
        cast: &'static str,
        trigger: u32,
        on: &'static [&'static str],
        arg: f32,
    },
    /// A milestone mechanic of the old game's, by its tag: not ported.
    Other(u32),
}

pub(super) enum Target {
    All,
    Spells(&'static [&'static str]),
    /// A spell school: 1 holy, 2 fire, 3 nature, 4 frost, 5 shadow, 6 arcane.
    School(u8),
    /// 6 damage over time, 9 heals and shields, 10 totems.
    Kind(u8),
}

pub(super) use super::talents_data::TREES;

/// The most points a class can hold, as in the game it copies.
pub(super) const MAX_POINTS: u32 = 51;

/// Talent experience the `n`th point costs.
fn point_cost(n: u32) -> f32 {
    100.0 + 25.0 * n as f32
}

/// The class's three trees.
pub(super) fn trees(class: u8) -> Option<&'static [TreeDef; 3]> {
    TREES.iter().find(|(c, _)| *c == class).map(|(_, t)| t)
}

/// The old game's spell keys where the mode names the spell otherwise.
fn canonical(key: &str) -> &str {
    match key {
        "sw_pain" => "shadow_word_pain",
        "pw_shield" => "power_word_shield",
        "demo_shout" => "demoralizing_shout",
        k => k,
    }
}

/// A spell's school, by the mode's key.
fn school(key: &str) -> u8 {
    match key {
        "fireball" | "flamestrike" | "fire_blast" | "blast_wave" | "pyroblast" | "immolate"
        | "rain_of_fire" | "hellfire" | "searing_pain" | "explosive_trap" | "flame_shock"
        | "searing_totem" | "magma_totem" => 2,
        "frostbolt" | "frost_nova" | "blizzard" | "cone_of_cold" | "frost_shock" => 4,
        "arcane_missiles" | "arcane_explosion" | "arcane_shot" | "volley" | "moonfire"
        | "starfire" => 6,
        "shadow_word_pain" | "mind_blast" | "psychic_scream" | "shadow_bolt" | "corruption"
        | "curse_of_agony" | "death_coil" | "shadowburn" => 5,
        "smite" | "holy_fire" | "holy_nova" | "power_word_shield" | "hammer_of_wrath"
        | "consecration" | "holy_shock" | "exorcism" | "holy_wrath" | "hammer_of_justice"
        | "divine_shield" | "judgement" => 1,
        "serpent_sting" | "lightning_bolt" | "chain_lightning" | "earth_shock"
        | "lightning_shield" | "wrath" | "hurricane" | "insect_swarm" | "entangling_roots"
        | "thorns" | "rejuvenation" => 3,
        _ => 0,
    }
}

impl Target {
    fn picks(&self, def: &AbilityDef) -> bool {
        match self {
            Self::All => true,
            Self::Spells(keys) => keys.iter().any(|k| canonical(k) == def.key),
            Self::School(s) => {
                (if def.native_kind == 255 {
                    school(def.key)
                } else {
                    def.school
                }) == *s
            }
            Self::Kind(6) => def.dot > 0.0 || def.kind == Kind::Dot,
            Self::Kind(9) => def.native_kind == 9 || matches!(def.kind, Kind::Heal | Kind::Shield),
            Self::Kind(10) => {
                def.native_kind == 10 || (def.native_kind == 255 && def.kind == Kind::Totem)
            }
            Self::Kind(_) => false,
        }
    }
}

/// A talent's `{v}` at `rank` (at least one), ready to read.
pub(super) fn value_text(t: &TalentDef, rank: u8) -> String {
    let v = t.v * f32::from(rank.max(1)) * if t.flat { 1.0 } else { 100.0 };
    let r = (v * 100.0).round() / 100.0;
    if r.fract().abs() < 1e-4 {
        format!("{r:.0}")
    } else {
        format!("{r}")
    }
}

pub(super) fn text_at(t: &TalentDef, rank: u8) -> String {
    t.text.replace("{v}", &value_text(t, rank))
}

/// One class's talents: its experience and the ranks bought.
#[derive(Default, Clone)]
pub(super) struct ClassBook {
    pub exp: f32,
    pub ranks: HashMap<&'static str, u8>,
}

impl ClassBook {
    /// Permanent class level, independent of the level reached inside a run.
    pub fn level(&self) -> u32 {
        self.earned() + 1
    }

    pub fn available(&self) -> u32 {
        self.earned().saturating_sub(self.spent())
    }

    pub fn progress_text(&self) -> String {
        if self.earned() >= MAX_POINTS {
            format!(
                "Class level {} (MAX) | {} talent points available",
                self.level(),
                self.available()
            )
        } else {
            let (xp, next) = self.toward_next();
            format!(
                "Class level {} | {xp:.0} / {next:.0} XP | {} talent points available",
                self.level(),
                self.available()
            )
        }
    }

    /// Points earned from the experience so far.
    pub fn earned(&self) -> u32 {
        let (mut n, mut left) = (0, self.exp);
        while n < MAX_POINTS && left >= point_cost(n) {
            left -= point_cost(n);
            n += 1;
        }
        n
    }

    /// Experience into the next point, and that point's cost.
    pub fn toward_next(&self) -> (f32, f32) {
        let (mut n, mut left) = (0, self.exp);
        while n < MAX_POINTS && left >= point_cost(n) {
            left -= point_cost(n);
            n += 1;
        }
        (left, point_cost(n))
    }

    pub fn spent(&self) -> u32 {
        self.ranks.values().map(|&r| u32::from(r)).sum()
    }

    pub fn rank(&self, key: &str) -> u8 {
        self.ranks.get(key).copied().unwrap_or(0)
    }

    fn tree_spent(&self, tree: &TreeDef, below: u8) -> u32 {
        tree.talents
            .iter()
            .filter(|t| t.tier < below)
            .map(|t| u32::from(self.rank(t.key)))
            .sum()
    }

    /// Whether every bought rank stands: its row open and its prerequisite maxed.
    fn valid(&self, trees: &[TreeDef; 3]) -> bool {
        trees.iter().all(|tree| {
            tree.talents.iter().all(|t| {
                self.rank(t.key) == 0
                    || (self.tree_spent(tree, t.tier) >= u32::from(t.tier) * 5
                        && t.prereq.is_none_or(|p| {
                            tree.talents
                                .iter()
                                .find(|q| q.key == p)
                                .is_none_or(|q| self.rank(q.key) >= q.ranks)
                        }))
            })
        }) && self.spent() <= self.earned()
    }

    /// Buy a rank if the rules allow it.
    pub fn learn(&mut self, trees: &[TreeDef; 3], t: &'static TalentDef) -> bool {
        if self.spent() >= self.earned() || self.rank(t.key) >= t.ranks {
            return false;
        }
        *self.ranks.entry(t.key).or_default() += 1;
        if self.valid(trees) {
            true
        } else {
            self.unlearn_raw(t.key);
            false
        }
    }

    /// Take a rank back if nothing above leans on it.
    pub fn unlearn(&mut self, trees: &[TreeDef; 3], t: &'static TalentDef) -> bool {
        if self.rank(t.key) == 0 {
            return false;
        }
        self.unlearn_raw(t.key);
        if self.valid(trees) {
            true
        } else {
            *self.ranks.entry(t.key).or_default() += 1;
            false
        }
    }

    fn unlearn_raw(&mut self, key: &str) {
        if let Some(r) = self.ranks.get_mut(key) {
            *r -= 1;
            if *r == 0 {
                self.ranks.remove(key);
            }
        }
    }
}

/// Every class's talents, kept in `benilla-config/survivors-talents.txt`, one class a line:
/// `class exp key=rank key=rank ...`.
#[derive(Resource, Default)]
pub(super) struct TalentBook {
    pub classes: HashMap<u8, ClassBook>,
    loaded: bool,
}

fn book_path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-talents.txt"))
}

/// The talent's own `&'static` key, so a book holds no strings of its own.
fn key_of(class: u8, key: &str) -> Option<&'static str> {
    trees(class)?
        .iter()
        .flat_map(|t| t.talents.iter())
        .find(|t| t.key == key)
        .map(|t| t.key)
}

impl TalentBook {
    pub fn class(&mut self, class: u8) -> &mut ClassBook {
        self.load();
        self.classes.entry(class).or_default()
    }

    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(text) = book_path().and_then(|p| crate::local_state::read_to_string(&p).ok())
        else {
            return;
        };
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let (Some(Ok(class)), Some(Ok(exp))) = (
                words.next().map(str::parse::<u8>),
                words.next().map(str::parse::<f32>),
            ) else {
                continue;
            };
            let mut book = ClassBook {
                exp,
                ranks: HashMap::new(),
            };
            for w in words {
                if let Some((k, r)) = w.split_once('=') {
                    if let (Some(k), Ok(r)) = (key_of(class, k), r.parse::<u8>()) {
                        book.ranks.insert(k, r);
                    }
                }
            }
            self.classes.insert(class, book);
        }
    }

    pub fn save(&self) {
        let Some(path) = book_path() else { return };
        let mut lines: Vec<String> = self
            .classes
            .iter()
            .map(|(c, b)| {
                let mut ranks: Vec<String> =
                    b.ranks.iter().map(|(k, r)| format!("{k}={r}")).collect();
                ranks.sort();
                format!("{c} {} {}", b.exp, ranks.join(" "))
            })
            .collect();
        lines.sort();
        let _ = crate::local_state::write_atomic(&path, &lines.join("\n"));
    }
}

/// A proc in a run, with the run second it may fire again.
pub(super) struct ProcState {
    pub chance: f32,
    pub every: f32,
    pub cast: &'static str,
    pub trigger: u32,
    pub on: &'static [&'static str],
    pub arg: f32,
    pub ready_at: f32,
}

/// The hero's talents as a run counts them.
#[derive(Default)]
pub(super) struct Build {
    /// [`Eff::Stat`] totals, by the old game's stat number.
    pub stats: HashMap<u32, f32>,
    mods: Vec<(&'static Target, u32, f32)>,
    pub grants: Vec<&'static str>,
    pub procs: Vec<ProcState>,
}

impl Build {
    pub fn of(class: u8, book: &ClassBook) -> Self {
        let mut b = Self::default();
        let Some(trees) = trees(class) else { return b };
        for t in trees.iter().flat_map(|t| t.talents.iter()) {
            let rank = book.rank(t.key);
            if rank == 0 {
                continue;
            }
            let r = f32::from(rank);
            for e in t.effects {
                match e {
                    Eff::Stat(s, per) => *b.stats.entry(*s).or_default() += per * r,
                    Eff::Mod { target, what, per } => b.mods.push((target, *what, per * r)),
                    Eff::Grant(k) => b.grants.push(canonical(k)),
                    Eff::Proc {
                        chance,
                        every,
                        cast,
                        trigger,
                        on,
                        arg,
                    } => b.procs.push(ProcState {
                        chance: chance * if t.ranks > 1 { r } else { 1.0 },
                        every: *every,
                        cast: canonical(cast),
                        trigger: *trigger,
                        on,
                        arg: *arg,
                        ready_at: 0.0,
                    }),
                    Eff::Other(_) => {}
                }
            }
        }
        b
    }

    pub fn stat(&self, s: u32) -> f32 {
        self.stats.get(&s).copied().unwrap_or(0.0)
    }

    /// The summed modifier of kind `what` on a spell.
    pub fn modifier(&self, def: &AbilityDef, what: u32) -> f32 {
        self.mods
            .iter()
            .filter(|(t, w, _)| *w == what && t.picks(def))
            .map(|(_, _, v)| v)
            .sum()
    }
}

/// Fire the procs a `trigger` sets off (`spell` the key cast, for the cast trigger 3, which also
/// answers the old game's 4): each that rolls readies its spell again, if the hero has it.
pub(super) fn trigger(run: &mut Run, rng: &mut super::Rng, trigger: u32, spell: Option<&str>) {
    let now = run.elapsed;
    let mut ready = Vec::new();
    for p in &mut run.talents.procs {
        let hit = match p.trigger {
            3 | 4 => trigger == 3 && spell.is_some_and(|k| p.on.iter().any(|o| canonical(o) == k)),
            t => t == trigger,
        };
        if !hit || now < p.ready_at || rng.f32() >= p.chance {
            continue;
        }
        p.ready_at = now + p.every;
        ready.push(p.cast);
    }
    for key in ready {
        if let Some(o) = run.abilities.iter_mut().find(|o| o.def.key == key) {
            o.cd = o.cd.min(0.0);
        }
    }
}

/// The health-share trigger (9): fired once the hero's health falls below a proc's `arg`.
pub(super) fn low_health(run: &mut Run, rng: &mut super::Rng) {
    let share = run.hp / run.stats().max_hp;
    let due = run
        .talents
        .procs
        .iter()
        .any(|p| p.trigger == 9 && share < p.arg && run.elapsed >= p.ready_at);
    if due {
        trigger(run, rng, 9, None);
    }
}

/// Talent experience a finished run pays its class: time survived, kills, bosses, guardians and
/// a win, more in a harder zone.
pub(super) fn run_pay(run: &Run, guardians: u32, map: usize) -> f32 {
    let zone = 1.0 + map as f32 * 0.02;
    let base = run.elapsed / 6.0
        + run.kills as f32 / 20.0
        + run.bosses as f32 * 25.0
        + guardians as f32 * 15.0
        + if run.won { 100.0 } else { 0.0 };
    (base * zone).round()
}

/// At a run's end, pay its class and keep the book.
pub(super) fn pay_run(
    mut run: ResMut<Run>,
    mut book: ResMut<TalentBook>,
    lairs: Res<super::guardians::Lairs>,
    selection: Res<Selection>,
) {
    settle_run(&mut run, &mut book, lairs.slain, selection.stage);
}

/// Shared by defeat/victory and returning to the menu; each run pays only once.
pub(super) fn settle_run(run: &mut Run, book: &mut TalentBook, guardians: u32, map: usize) {
    if run.talent_paid {
        return;
    }
    let pay = run_pay(run, guardians, map);
    let class = run.class.id;
    let page = book.class(class);
    let before = page.earned();
    page.exp += pay;
    let gained = page.earned() - before;
    book.save();
    run.talent_pay = (pay, gained);
    run.talent_paid = true;
}

#[derive(Component)]
pub(super) struct MenuProgress;

#[derive(Component)]
pub(super) struct ResultsProgress;

/// Keep the reward receipt and the selected class's saved progress visible between runs.
pub(super) fn refresh_progress(
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    run: Res<Run>,
    mut book: ResMut<TalentBook>,
    mut texts: ParamSet<(
        Query<&mut Text, With<MenuProgress>>,
        Query<&mut Text, With<ResultsProgress>>,
    )>,
) {
    match phase.get() {
        Phase::Menu => {
            let value = selection
                .class_def()
                .map_or_else(String::new, |class| book.class(class.id).progress_text());
            for mut text in &mut texts.p0() {
                if text.0 != value {
                    text.0 = value.clone();
                }
            }
        }
        Phase::GameOver => {
            let page = book.class(run.class.id);
            let (xp, points) = run.talent_pay;
            let earned = if points > 0 {
                format!("Level up! +{points} talent points")
            } else if page.earned() >= MAX_POINTS {
                "Maximum class level reached".into()
            } else {
                "Earn class XP each run to unlock talent points".into()
            };
            let value = format!(
                "+{xp:.0} {} class XP | {earned}\n{}\nSpend your points in Talents from the main menu.",
                run.class.name, page.progress_text()
            );
            for mut text in &mut texts.p1() {
                if text.0 != value {
                    text.0 = value.clone();
                }
            }
        }
        _ => {}
    }
}

/// As a run starts, count the class's talents and hand it the spells they grant.
pub(super) fn begin_run(mut run: ResMut<Run>, mut book: ResMut<TalentBook>) {
    let class = run.class.id;
    run.talents = Build::of(class, book.class(class));
    let grants = run.talents.grants.clone();
    for key in grants {
        let Some(def) = super::data::ABILITIES.iter().find(|a| a.key == key) else {
            continue;
        };
        if run.abilities.len() < super::data::MAX_ABILITIES
            && !run.abilities.iter().any(|o| o.def.key == key)
        {
            run.abilities
                .push(super::Owned::new(def, super::rewards::Rarity::Common));
        }
    }
    run.hp = run.stats().max_hp;
    run.version += 1;
}

// --- The talents screen --------------------------------------------------------------------

#[derive(Component, Clone, Copy)]
pub(super) struct CloseTalents;

#[derive(Component, Clone, Copy)]
pub(super) struct ResetTalents;

/// The open screen.
#[derive(Component)]
pub(super) struct TalentPanel;

/// A talent's button: its tree and place in it.
#[derive(Component, Clone, Copy)]
pub(super) struct Cell(usize, usize);

/// An icon slot filled from `Interface\Icons` once the client data is up.
#[derive(Component)]
pub(super) struct IconPath(&'static str);

/// The screen's state: open, and bumped on every change so it rebuilds once.
#[derive(Resource, Default)]
pub(super) struct TalentScreen {
    open: bool,
    version: u32,
    built: u32,
    hovered: Option<(usize, usize)>,
}

const CELL: f32 = 46.0;
const ICON: f32 = 38.0;
const TREE_W: f32 = 4.0 * CELL + 24.0;

/// Open, close, spend and refund; rebuild the screen when anything changed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn talent_screen(
    mut commands: Commands,
    clicks: Res<GlueClicks>,
    phase: Res<State<Phase>>,
    mouse: Res<ButtonInput<MouseButton>>,
    selection: Res<Selection>,
    mut screen: ResMut<TalentScreen>,
    mut book: ResMut<TalentBook>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
    roots: Query<Entity, With<MenuRoot>>,
    panels: Query<Entity, With<TalentPanel>>,
    opens: Query<(Entity, &MenuAction)>,
    closes: Query<Entity, With<CloseTalents>>,
    resets: Query<Entity, With<ResetTalents>>,
    cells: Query<(&Cell, &Interaction)>,
) {
    let in_menu = *phase.get() == Phase::Menu;
    let class = selection.class_def().map(|c| c.id);
    if opens
        .iter()
        .any(|(e, a)| *a == MenuAction::Talents && clicks.hit(e))
    {
        screen.open = true;
        screen.version += 1;
    }
    if closes.iter().any(|e| clicks.hit(e)) || !in_menu {
        if screen.open {
            screen.open = false;
            screen.version += 1;
        }
    }
    let trees_now = class.and_then(trees);
    if let (Some(class), Some(trees_now)) = (class, trees_now) {
        if resets.iter().any(|e| clicks.hit(e)) {
            book.class(class).ranks.clear();
            book.save();
            screen.version += 1;
        }
        let hovered = cells
            .iter()
            .find(|(_, i)| **i != Interaction::None)
            .map(|(c, _)| (c.0, c.1));
        if hovered != screen.hovered {
            screen.hovered = hovered;
            screen.version += 1;
        }
        if let Some((tree, idx)) = hovered {
            let t = &trees_now[tree].talents[idx];
            let left = cells
                .iter()
                .any(|(c, i)| (c.0, c.1) == (tree, idx) && *i == Interaction::Pressed)
                && mouse.just_pressed(MouseButton::Left);
            let right = mouse.just_pressed(MouseButton::Right);
            let page = book.class(class);
            let changed = if left {
                page.learn(trees_now, t)
            } else if right {
                page.unlearn(trees_now, t)
            } else {
                false
            };
            if changed {
                book.save();
                screen.version += 1;
            }
        }
    }

    if screen.built == screen.version && (panels.is_empty() != screen.open) {
        return;
    }
    screen.built = screen.version;
    for p in &panels {
        commands.entity(p).despawn();
    }
    if !screen.open {
        return;
    }
    let (Some(class), Some(trees_now), Ok(root)) = (class, trees_now, roots.single()) else {
        return;
    };
    let hovered = screen.hovered;
    let page = book.class(class).clone();
    let f = font.0.clone();
    commands.entity(root).with_children(|ui| {
        build_panel(ui, &art, &f, class, trees_now, &page, hovered);
    });
}

fn text(
    ui: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) {
    ui.spawn((
        Text::new(s),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    ));
}

fn build_panel(
    ui: &mut ChildSpawnerCommands,
    art: &GlueArt,
    f: &Handle<Font>,
    class: u8,
    trees: &'static [TreeDef; 3],
    page: &ClassBook,
    hovered: Option<(usize, usize)>,
) {
    let class_name = super::data::CLASSES
        .iter()
        .find(|c| c.id == class)
        .map_or("Hero", |c| c.name);
    let (into, next) = page.toward_next();
    ui.spawn((
        TalentPanel,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Interaction::default(),
        bevy::ui::FocusPolicy::Block,
        GlobalZIndex(40),
    ))
    .with_children(|back| {
        back.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(2.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.95)),
            BorderColor::all(Color::srgb(0.55, 0.45, 0.25)),
        ))
        .with_children(|frame| {
            text(
                frame,
                f,
                format!(
                    "{class_name} | Class level {} | {} / {} talent points ({} to spend)",
                    page.level(), page.earned(), MAX_POINTS, page.available()
                ),
                20.0,
                GOLD,
            );
            text(
                frame,
                f,
                if page.earned() >= MAX_POINTS {
                    "Every talent point earned.".to_string()
                } else {
                    format!("Talent experience {into:.0} / {next:.0} to the next point")
                },
                14.0,
                Color::srgb(0.85, 0.85, 0.85),
            );
            frame
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(14.0),
                    ..default()
                })
                .with_children(|row| {
                    for (ti, tree) in trees.iter().enumerate() {
                        tree_column(row, f, ti, tree, page);
                    }
                });
            let info = hovered
                .and_then(|(t, i)| trees.get(t).and_then(|tr| tr.talents.get(i)))
                .map(|t| {
                    let rank = page.rank(t.key);
                    let mut s = format!("{}  (Rank {rank}/{})\n{}", t.name, t.ranks, text_at(t, rank));
                    if rank > 0 && rank < t.ranks {
                        s += &format!("\nNext rank: {}", text_at(t, rank + 1));
                    }
                    if t.tier > 0 {
                        s += &format!("\nRequires {} points in the tree.", u32::from(t.tier) * 5);
                    }
                    if let Some(p) = t.prereq {
                        if let Some(q) = trees.iter().flat_map(|tr| tr.talents.iter()).find(|q| q.key == p) {
                            s += &format!("\nRequires {} maxed.", q.name);
                        }
                    }
                    s
                })
                .unwrap_or_else(|| {
                    "Point at a talent to read it. Click to spend a point, right-click to take one back. A row opens at 5 points spent in its tree. Every run pays your class talent experience: time survived, kills, bosses, guardians and a win. Respecs are free.".into()
                });
            frame
                .spawn(Node {
                    width: Val::Px(3.0 * TREE_W + 28.0),
                    min_height: Val::Px(84.0),
                    ..default()
                })
                .with_children(|b| text(b, f, info, 14.0, Color::srgb(1.0, 0.95, 0.8)));
            frame
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|row| {
                    glue_button(row, art, f, ResetTalents, "Reset", 140.0, 36.0, GlueBtnKind::Small, 1.0);
                    glue_button(row, art, f, CloseTalents, "Close", 140.0, 36.0, GlueBtnKind::Small, 1.0);
                });
        });
    });
}

fn tree_column(
    row: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    ti: usize,
    tree: &'static TreeDef,
    page: &ClassBook,
) {
    let spent = page.tree_spent(tree, 7);
    row.spawn((
        Node {
            width: Val::Px(TREE_W),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(6.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.9)),
        BorderColor::all(Color::srgb(0.35, 0.3, 0.2)),
    ))
    .with_children(|col| {
        text(col, f, format!("{}  ({spent})", tree.name), 17.0, GOLD);
        col.spawn(Node {
            width: Val::Px(4.0 * CELL),
            height: Val::Px(7.0 * CELL),
            ..default()
        })
        .with_children(|grid| {
            for (i, t) in tree.talents.iter().enumerate() {
                let rank = page.rank(t.key);
                let open = page.tree_spent(tree, t.tier) >= u32::from(t.tier) * 5;
                let tint = if rank > 0 {
                    Color::WHITE
                } else if open {
                    Color::srgb(0.75, 0.75, 0.75)
                } else {
                    Color::srgb(0.3, 0.3, 0.3)
                };
                let edge = if rank >= t.ranks {
                    GOLD
                } else if rank > 0 {
                    Color::srgb(0.3, 0.9, 0.3)
                } else {
                    Color::srgb(0.25, 0.25, 0.25)
                };
                grid.spawn((
                    Cell(ti, i),
                    Button,
                    Interaction::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(f32::from(t.col) * CELL + (CELL - ICON) / 2.0),
                        top: Val::Px(f32::from(t.tier) * CELL + (CELL - ICON) / 2.0),
                        width: Val::Px(ICON),
                        height: Val::Px(ICON),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(edge),
                ))
                .with_children(|b| {
                    b.spawn((
                        ImageNode {
                            color: tint,
                            ..default()
                        },
                        IconPath(t.icon),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                    b.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(-4.0),
                            bottom: Val::Px(-6.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                    ))
                    .with_children(|r| {
                        text(r, f, format!("{rank}/{}", t.ranks), 11.0, edge);
                    });
                });
            }
        });
    });
}

/// Fill talent icons from `Interface\Icons` once the asset reader is up.
pub(super) fn fill_talent_icons(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &IconPath)>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(assets) = assets.as_deref_mut() else {
        return;
    };
    for (e, mut img, path) in &mut slots {
        if let Some(h) =
            assets.sprite_texture(&format!(r"Interface\Icons\{}.blp", path.0), &mut images)
        {
            img.image = h;
        }
        commands.entity(e).remove::<IconPath>();
    }
}
