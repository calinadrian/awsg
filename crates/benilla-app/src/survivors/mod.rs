//! Warcraft Survivors: a survival mode on the benilla world, started by [`crate::run_survivors`].
//! There is no server: the hero, the enemies and the spells are local entities dressed by the same
//! attach, animation and spell-visual systems a streamed unit gets, on a battleground picked at
//! the menu ([`data::MAPS`]), with FrameXML off ([`crate::run_mode::CaptureMode`]). In play the
//! hero is driven by WoW's own player controller ([`drive`]).

mod abilities;
mod amulets;
mod auto;
mod caches;
mod card_content;
mod card_view;
mod cards;
mod data;
mod drive;
mod dungeon;
mod elwynn;
mod elwynn_behavior;
mod enemies;
mod fill;
mod frame_settings;
mod gold;
mod grade;
mod guardian_behavior;
mod guardians;
mod hero;
mod hud;
mod jewelry;
mod loot_ground;
mod menu;
mod menu_icons;
mod minimap;
mod options;
mod plates;
mod races;
mod reward_axes;
mod rewards;
mod ring;
pub(crate) mod spell_flight;
mod spell_passives;
mod talents;
mod talents_data;
mod telegraphs;
mod unit_frames;
mod units;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use data::{AbilityDef, ClassDef};

/// Set by [`crate::run_survivors`] before the app builds. wasm32 has no process environment, so
/// the flag lives here; `WOW_SURVIVORS` (`?survivors=1` / `__wenilla_env.survivors` on the web)
/// still turns it on from outside.
pub(crate) static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn active() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
        || crate::webenv::var("WOW_SURVIVORS").is_some()
}

pub(crate) struct SurvivorsPlugin;

impl Plugin for SurvivorsPlugin {
    fn build(&self, app: &mut App) {
        // Present before `OnEnter(InWorld)`, where the UI lifecycle decides not to load FrameXML.
        app.insert_resource(crate::run_mode::CaptureMode)
            .add_plugins(grade::plugin)
            .add_plugins(options::plugin)
            .init_state::<Phase>()
            .init_resource::<Rng>()
            .init_resource::<Selection>()
            .init_resource::<Run>()
            .init_resource::<hero::CamRig>()
            .init_resource::<hero::AimPoint>()
            .init_resource::<drive::EntryPending>()
            .init_resource::<abilities::PendingHits>()
            .add_systems(
                Update,
                spell_flight::prepare.before(crate::entities::EntityVisualsSet),
            )
            .init_resource::<enemies::Spawner>()
            .init_resource::<guardians::Lairs>()
            .init_resource::<talents::TalentBook>()
            .init_resource::<telegraphs::Telegraphs>()
            .init_resource::<talents::TalentScreen>()
            .init_resource::<hud::Offer>()
            .init_resource::<cards::PendingPick>()
            .add_message::<abilities::Hit>()
            .add_message::<enemies::Loot>()
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<hud::SweepUiMaterial>::default())
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<fill::FillMaterial>::default())
            .add_systems(Startup, (card_view::setup, hud::setup_hud).chain())
            .add_plugins(unit_frames::plugin)
            .add_systems(
                Update,
                loot_ground::seat
                    .after(enemies::drop_loot)
                    .before(enemies::collect_gems)
                    .before(caches::collect)
                    .after(WorldStage::Input),
            )
            .add_systems(Update, hero::daylight)
            .add_systems(
                Update,
                (
                    hero::drop_stray_buttons,
                    drive::take_reins,
                    hero::aim_cursor,
                    hero::drive_hero,
                    hero::place_camera,
                )
                    .chain()
                    .after(crate::ui_script::UiInput)
                    .before(crate::player::PlayerControlSet)
                    .in_set(WorldStage::Input),
            )
            // The leash, once the controller has moved the hero this frame.
            .add_systems(
                Update,
                drive::hold_leash
                    .after(crate::player::PlayerControlSet)
                    .in_set(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    enemies::spawn_enemies,
                    guardians::keep_lairs,
                    enemies::move_enemies,
                    enemies::run_scripts,
                    abilities::cast_abilities,
                    abilities::tick_areas,
                    abilities::apply_hits,
                    enemies::tick_marks,
                    enemies::drop_loot,
                    enemies::collect_gems,
                    caches::collect,
                    hero::hero_vitals,
                    ring::light_ring,
                )
                    .chain()
                    .after(WorldStage::Input)
                    .run_if(in_state(Phase::Playing))
                    .run_if(|entry: Res<drive::EntryPending>| !entry.0)
                    // Entering a battleground raises its loading screen: the run waits behind it.
                    .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            )
            .add_systems(
                Update,
                (enemies::reap_dead, enemies::still_gems).after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hero::follow_map,
                    hud::menu_preview,
                    hud::buttons,
                    hud::keys,
                    hud::refresh_menu,
                    menu::restore_pick,
                    menu::build_menu,
                    menu::menu_clicks,
                    menu::refresh_menu_ui.before(crate::glue::GlueVisuals),
                    talents::talent_screen,
                    talents::fill_talent_icons,
                    talents::refresh_progress,
                    (cards::hand_controls, cards::deal_cards).chain(),
                    cards::choose_card,
                    cards::animate_cards,
                    hud::refresh_hud,
                    hud::refresh_results,
                    hud::fill_icons,
                    hud::update_cooldown_sweeps,
                    hud::show_panels,
                )
                    .chain()
                    .after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hud::apply_ui_scale,
                    hud::fill_art,
                    hud::icon_tooltip,
                    hud::hero_plate,
                    plates::enemy_plates,
                    plates::boss_bar,
                    minimap::map_buttons,
                    minimap::zone_text,
                    minimap::compose,
                )
                    .chain()
                    .after(hud::show_panels),
            )
            // The telegraphs' stream push, after the frame's effect-stream clear.
            .add_systems(
                PostUpdate,
                telegraphs::draw_telegraphs
                    .after(benilla_world::particles::buffer::begin_effect_frame),
            )
            .add_systems(OnEnter(Phase::Playing), unpause)
            .add_systems(
                OnTransition {
                    exited: Phase::Menu,
                    entered: Phase::Playing,
                },
                (drive::start_on_center, talents::begin_run, caches::place),
            )
            .add_systems(OnEnter(Phase::LevelUp), (pause, hud::roll_offer))
            .add_systems(OnEnter(Phase::Paused), pause)
            .add_systems(OnEnter(Phase::GameOver), (unpause, talents::pay_run))
            .add_systems(OnEnter(Phase::Menu), (unpause, reset_run));
        if let Some(auto) = auto::from_env() {
            app.insert_resource(auto)
                .add_systems(
                    Update,
                    auto::drive.after(hud::keys).before(hud::refresh_menu),
                )
                .add_systems(Update, auto::plan.before(WorldStage::Input));
        }
    }
}

/// The screen the mode is on.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum Phase {
    #[default]
    Menu,
    Playing,
    LevelUp,
    Paused,
    GameOver,
}

/// Everything spawned for a run, despawned when the next begins.
#[derive(Component)]
pub(super) struct RunEntity;

/// The class, look and battleground picked at the menu; the battleground holds for the run.
#[derive(Resource, Default)]
pub(super) struct Selection {
    pub class: Option<usize>,
    /// `(display, race, sex)`, one per race and sex the class's trainers come in.
    pub looks: Vec<(u32, u8, u8)>,
    pub look: Option<usize>,
    /// An index into [`data::MAPS`].
    pub map: usize,
    /// The battleground the world is standing in: the last one entered (or the saved pick at
    /// boot). Picking another at the menu changes only `map`, so the menu hero stays put and no
    /// loading screen rises until Enter World.
    pub stage: usize,
    /// Bumped on every change, so the menu rebuilds once.
    pub version: u32,
}

impl Selection {
    pub fn class_def(&self) -> Option<&'static ClassDef> {
        self.class.map(|i| &data::CLASSES[i])
    }

    pub fn map_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.map.min(data::MAPS.len() - 1)]
    }

    pub fn stage_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.stage.min(data::MAPS.len() - 1)]
    }
}

/// An ability the hero has, with its own cooldown.
#[derive(Clone)]
pub(super) struct Owned {
    pub def: &'static AbilityDef,
    pub level: u8,
    pub cd: f32,
    pub progress: rewards::SpellProgress,
    echoing: bool,
}

impl Owned {
    pub fn new(def: &'static AbilityDef, rarity: rewards::Rarity) -> Self {
        Self {
            def,
            level: 1,
            cd: 0.5,
            echoing: false,
            progress: rewards::SpellProgress::learned(
                rarity,
                reward_axes::axes(def.key).contains(&rewards::Upgrade::Power),
            ),
        }
    }
    pub fn upgrade(&mut self, rarity: rewards::Rarity, axes: [Option<rewards::Upgrade>; 2]) {
        for axis in axes.into_iter().flatten() {
            let rank = 2 + self.progress.ranks.iter().sum::<u32>();
            self.progress.apply(axis, rarity, rank);
            self.level = self.level.saturating_add(1);
        }
    }
    pub fn damage_mult(&self) -> f32 {
        self.progress.power
    }
    pub fn cooldown(&self, stats: &Stats) -> f32 {
        // Native cooldown helper RVA 0x5b9bc40 scales the definition by 1.4 before
        // rank modifiers, then divides by cooldown recovery.
        let base = self.def.cooldown
            * if self.def.native_kind == 255 {
                1.0
            } else {
                1.4
            };
        (base * self.progress.cooldown / (1.0 + stats.haste).max(0.1)).max(0.25)
    }
    pub fn count(&self) -> u32 {
        self.def.count + self.progress.targets
    }
    pub fn radius(&self, stats: &Stats) -> f32 {
        (if matches!(self.def.kind, data::Kind::Melee | data::Kind::Cone) && self.def.radius == 0. {
            self.def.range
        } else {
            self.def.radius
        }) * self.progress.area
            * stats.area
    }
    pub fn duration(&self) -> f32 {
        self.def.duration * self.progress.duration
    }
}

/// The hero's derived numbers, from the base and run jewelry.
#[derive(Clone, Copy)]
pub(super) struct Stats {
    pub might: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub regen: f32,
    pub haste: f32,
    pub area: f32,
    pub magnet: f32,
    pub growth: f32,
    pub armor: f32,
    pub crit: f32,
    /// A critical strike's multiplier.
    pub crit_mult: f32,
    pub extra_targets: u32,
}

pub(super) const BASE_HP: f32 = 120.0;
pub(super) const BASE_SPEED: f32 = 7.0;
/// Original ASR's base pickup radius (RVA 0x5be92f5).
pub(super) const BASE_MAGNET: f32 = 9.0;

/// One run's state.
#[derive(Resource)]
pub(super) struct Run {
    pub class: &'static ClassDef,
    pub race: u8,
    pub elapsed: f32,
    pub kills: u32,
    pub level: u32,
    pub xp: f32,
    pub hp: f32,
    pub abilities: Vec<Owned>,
    pub amulet: Option<amulets::Amulet>,
    pub shield: f32,
    pub shield_t: f32,
    pub immune_t: f32,
    pub thorns: f32,
    pub thorns_t: f32,
    pub thorns_spell: u32,
    pub heal_rate: f32,
    pub heal_t: f32,
    pub pending_levels: u32,
    pub pending_chests: u32,
    pub jewelry: Vec<jewelry::Jewel>,
    pub gold: u32,
    pub chest_cost: u32,
    pub revives_used: u32,
    pub bosses: u32,
    /// A timed roster's last boss has fallen: the run is won.
    pub slain_last: bool,
    pub won: bool,
    /// Seconds since the hero fell, while the death plays out.
    pub dead_t: Option<f32>,
    pub next_guid: u64,
    pub damage_done: f64,
    /// Bumped whenever spells or jewelry change, so the HUD rebuilds the icon rows once.
    pub version: u32,
    /// A centre-screen announcement and its seconds left.
    pub banner: Option<(String, f32)>,
    /// A boss's or guardian's latest yell over the boss bar, and its seconds left.
    pub yell: Option<(String, f32)>,
    /// The class's talents, as this run counts them.
    talents: talents::Build,
    /// The talent experience the run paid and the points it brought.
    pub talent_pay: (f32, u32),
    pub talent_paid: bool,
    hand: cards::Hand,
    /// A melee blow landed on the hero this frame, for the talents that answer one.
    pub struck: bool,
    /// Key, native effect, strength, and remaining seconds of active spell buffs.
    pub buffs: Vec<(&'static str, u32, f32, f32)>,
    passives: spell_passives::State,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            class: &data::CLASSES[0],
            race: 0,
            elapsed: 0.0,
            kills: 0,
            level: 1,
            xp: 0.0,
            hp: BASE_HP,
            abilities: Vec::new(),
            amulet: None,
            shield: 0.0,
            shield_t: 0.0,
            immune_t: 0.0,
            thorns: 0.0,
            thorns_t: 0.0,
            thorns_spell: 0,
            heal_rate: 0.0,
            heal_t: 0.0,
            pending_levels: 0,
            pending_chests: 0,
            jewelry: Vec::new(),
            gold: 0,
            chest_cost: 20,
            revives_used: 0,
            bosses: 0,
            slain_last: false,
            won: false,
            dead_t: None,
            next_guid: 1,
            damage_done: 0.0,
            version: 0,
            banner: None,
            yell: None,
            talents: talents::Build::default(),
            talent_pay: (0.0, 0),
            talent_paid: false,
            hand: cards::Hand::default(),
            struck: false,
            buffs: Vec::new(),
            passives: spell_passives::State::default(),
        }
    }
}

impl Run {
    pub fn stats(&self) -> Stats {
        let mut s = Stats {
            might: 1.0,
            max_hp: BASE_HP,
            speed: BASE_SPEED,
            regen: 0.0,
            haste: 0.0,
            area: 1.0,
            magnet: BASE_MAGNET,
            growth: 1.0,
            armor: 0.0,
            crit: 0.05,
            crit_mult: 2.0,
            extra_targets: self
                .amulet
                .as_ref()
                .map_or(0, |a| amulets::AMULETS[a.index].targets),
        };
        // Tanky classes start tougher, cloth starts quicker.
        match self.class.id {
            1 | 2 => s.max_hp += 40.0,
            7 | 11 | 3 | 4 => s.max_hp += 15.0,
            _ => s.speed += 0.5,
        }
        s.might += self.jewel_stat(0);
        s.haste += self.jewel_stat(1);
        s.max_hp *= 1. + self.jewel_stat(2) + self.jewel_stat(6);
        s.crit += self.jewel_stat(3);
        s.crit_mult += self.jewel_stat(4);
        s.regen += self.jewel_stat(7);
        s.area += self.jewel_stat(8);
        s.speed *= 1. + self.jewel_stat(10);
        s.magnet *= 1. + self.jewel_stat(11) + races::pickup_bonus(self.race);
        s.growth += self.jewel_stat(12) + races::xp_bonus(self.race);
        let t = &self.talents;
        s.might += t.stat(0);
        s.haste += t.stat(1);
        s.max_hp *= 1.0 + t.stat(2);
        s.crit += t.stat(3);
        s.crit_mult += t.stat(4);
        s.regen += t.stat(6) + t.stat(24) * s.max_hp;
        s.speed *= 1.0 + t.stat(7);
        for &(_, effect, strength, _) in &self.buffs {
            match effect {
                8 | 10 => s.might += strength,
                9 => s.haste += strength,
                _ => {}
            }
        }
        s
    }

    pub fn jewel_stat(&self, stat: u8) -> f32 {
        self.jewelry
            .iter()
            .map(|j| jewelry::stat(j.index, j.strength, stat))
            .sum::<f32>()
            + self.amulet.as_ref().map_or(0., |a| {
                amulets::AMULETS[a.index]
                    .stats
                    .iter()
                    .filter(|(tag, _)| *tag == stat)
                    .map(|(_, v)| *v * a.scale())
                    .sum::<f32>()
            })
    }
    pub fn guid(&mut self) -> u64 {
        self.next_guid += 1;
        self.next_guid
    }
}

/// xorshift64*: the mode's dice, seeded from the clock.
#[derive(Resource)]
pub(super) struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        let seed = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(seed | 1)
    }
}

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len.max(1) as u64) as usize
    }
}

fn pause(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn unpause(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Back at the menu: everything the last run spawned goes, and the run starts over.
fn reset_run(
    mut commands: Commands,
    spawned: Query<Entity, With<RunEntity>>,
    mut run: ResMut<Run>,
    mut pending: ResMut<abilities::PendingHits>,
    mut spawner: ResMut<enemies::Spawner>,
    mut lairs: ResMut<guardians::Lairs>,
    mut selection: ResMut<Selection>,
    mut book: ResMut<talents::TalentBook>,
) {
    if run.elapsed > 0.0 || run.kills > 0 {
        talents::settle_run(&mut run, &mut book, lairs.slain, selection.stage);
    }
    for e in &spawned {
        commands.entity(e).despawn();
    }
    // Guids stay unique across runs: caches keyed by guid may outlive a despawn by a frame.
    let next_guid = run.next_guid;
    *run = Run {
        next_guid,
        ..Run::default()
    };
    pending.0.clear();
    *spawner = enemies::Spawner::default();
    *lairs = guardians::Lairs::default();
    // The preview hero is respawned for the look still picked.
    selection.version += 1;
}
