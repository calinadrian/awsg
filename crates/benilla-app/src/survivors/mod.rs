//! Warcraft Survivors: a survival mode on the benilla world, started by [`crate::run_survivors`].
//! Solo and player-hosted co-op use local entities dressed by the same
//! attach, animation and spell-visual systems a streamed unit gets, on a battleground picked at
//! the menu ([`data::MAPS`]), with FrameXML off ([`crate::run_mode::CaptureMode`]). In play the
//! hero is driven by WoW's own player controller ([`drive`]).

mod abilities;
mod almighty_strength;
mod amulets;
pub(crate) mod appearance;
mod auto;
mod auto_attack;
mod boss_scripts;
mod boss_cooldowns;
mod boss_pacing;
mod boss_enrage;
mod hogger;
mod mother_fang;
mod princess;
mod morgan;
mod encounter_announcer;
mod hud_notices;
mod caches;
mod card_content;
mod card_view;
mod cards;
mod coop;
mod party_frames;
mod coop_ui;
mod damage_meter;
mod data;
mod difficulty;
#[cfg(test)]
mod difficulty_sim;
mod drive;
mod dungeon;
mod elwynn;
mod elwynn_behavior;
mod enemies;
mod adventure_atlas;
mod features_ui;
mod fill;
mod frame_settings;
mod front_end;
mod gold;
#[cfg(test)]
mod golden;
mod grade;
mod guardian_behavior;
mod guardians;
mod hero;
mod horde_spawning;
mod low_memory;
mod hud;
mod hud_layout;
mod ui_edit;
mod fps_counter;
mod food;
mod jewelry;
mod journal_ui;
mod inventory;
mod inventory_ui;
pub(crate) mod inventory_outfit;
mod loot_ground;
mod magnets;
mod mechanics;
mod menu;
mod menu_icons;
mod menu_layout;
mod menu_skin;
mod menu_preview;
mod item_preview;
mod minimap;
mod options;
mod plates;
mod progress;
mod quests;
mod right_column;
mod races;
mod reward_axes;
mod rewards;
mod resurrection_ui;
mod ring;
pub(crate) mod spell_flight;
mod spell_passives;
mod text_edit;
mod talents;
mod talents_data;
mod talent_combat;
mod companions;
mod telegraphs;
mod test_grounds;
mod ui_audit;
mod unit_frames;
mod units;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use data::{AbilityDef, ClassDef};
use races::Bonus;

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
            .add_plugins((coop::plugin, coop_ui::plugin, progress::plugin, party_frames::plugin))
            .add_plugins((
                damage_meter::plugin,
                text_edit::plugin,
                quests::plugin,
                right_column::plugin,
            ))
            .init_state::<Phase>()
            .init_resource::<Rng>()
            .init_resource::<caches::Spawner>()
            .init_resource::<Selection>()
            .init_resource::<Run>()
            .init_resource::<boss_pacing::Windups>()
            .init_resource::<hero::CamRig>()
            .init_resource::<hero::AimPoint>()
            .init_resource::<auto_attack::AutoAttack>()
            .init_resource::<drive::EntryPending>()
            .init_resource::<abilities::PendingHits>()
            .add_systems(
                Update,
                spell_flight::prepare.before(crate::entities::EntityVisualsSet),
            )
            .init_resource::<enemies::Spawner>()
            .init_resource::<enemies::Bolts>()
            .init_resource::<guardians::Lairs>()
            .init_resource::<talents::TalentBook>()
            .init_resource::<telegraphs::Telegraphs>()
            .init_resource::<plates::NameplateSettings>()
            .init_resource::<talents::TalentScreen>()
            .init_resource::<hud::Offer>()
            .init_resource::<hud::Replay>()
            .init_resource::<cards::PendingPick>()
            .add_message::<abilities::Hit>()
            .add_message::<enemies::Loot>()
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<hud::SweepUiMaterial>::default())
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<fill::FillMaterial>::default())
            .add_systems(
                Startup,
                (
                    plates::load_nameplate_settings.after(crate::cvars::CvarLoad),
                    card_view::setup,
                    hud::setup_hud,
                )
                    .chain(),
            )
            .add_plugins(unit_frames::plugin)
            .add_plugins(hud_layout::plugin)
            .add_plugins((ui_edit::plugin, fps_counter::plugin))
            .add_plugins(boss_cooldowns::plugin)
            .add_plugins(boss_enrage::plugin)
            .add_plugins(hogger::plugin)
            .add_plugins(mother_fang::plugin)
            .add_plugins(morgan::plugin)
            .add_plugins(encounter_announcer::plugin)
            .add_plugins(hud_notices::plugin)
            .add_plugins(magnets::plugin)
            .add_plugins(test_grounds::plugin)
            .add_plugins(appearance::plugin)
            .add_plugins((inventory::plugin, inventory_ui::plugin, inventory_outfit::plugin))
            .add_plugins(adventure_atlas::plugin)
            .add_plugins(menu_preview::plugin)
            .add_plugins(item_preview::plugin)
            .add_systems(Update, menu_skin::hover.after(front_end::hover).after(menu::refresh_menu_ui))
            .add_plugins(ui_audit::plugin)
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
                    enemies::spawn_enemies.run_if(coop::authority),
                    guardians::keep_lairs.run_if(coop::authority),
                    enemies::move_enemies.run_if(coop::authority),
                    enemies::run_scripts.run_if(coop::authority),
                    mechanics::run_mechanics.run_if(coop::authority),
                    (enemies::cap_heights,
                        (talent_combat::forms, companions::tick).chain().run_if(in_state(Phase::Playing))).chain(),
                    abilities::cast_abilities.run_if(in_state(Phase::Playing)),
                    (
                        auto_attack::start_on_right_click,
                        auto_attack::swing,
                        auto_attack::follow_focus.after(crate::target::TargetUpdate),
                    )
                        .chain()
                        .run_if(in_state(Phase::Playing)),
                    abilities::tick_areas.run_if(in_state(Phase::Playing)),
                    abilities::apply_hits,
                    coop::apply_guest_hits,
                    morgan::cleanup.run_if(coop::authority),
                    enemies::tick_marks,
                    enemies::drop_loot.run_if(coop::authority),
                    enemies::collect_gems.run_if(coop::authority),
                    caches::replenish.run_if(in_state(Phase::Playing)),
                    caches::collect.run_if(in_state(Phase::Playing)),
                    hero::hero_vitals,
                    coop::revive_teammates,
                    ring::light_ring,
                )
                    .chain()
                    .after(WorldStage::Input)
                    .run_if(coop::world_running)
                    .run_if(|time: Res<Time>| time.delta_secs() > 0.0)
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
                    (hud::apply_ui_scale, hero::follow_map).chain(),
                    hud::menu_preview,
                    hud::buttons,
                    hud::keys,
                    hud::refresh_menu,
                    menu::restore_pick,
                    menu::build_menu,
                    menu::menu_clicks,
                    (
                        menu::refresh_menu_ui.before(crate::glue::GlueVisuals),
                        menu_layout::refresh_adventure,
                        menu_layout::refresh_pick_plate,
                        menu_layout::refresh_class,
                    ).chain(),
                    talents::talent_screen,
                    talents::fill_talent_icons,
                    talents::refresh_progress,
                    (cards::hand_controls, cards::deal_cards).chain(),
                    cards::choose_card,
                    (cards::animate_cards, card_view::animate_fx),
                    (hud::refresh_hud, hud::size_action_bar),
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
                    hud::icon_tooltip.run_if(ui_edit::inactive),
                    hud::hero_plate,
                    plates::sync_plate_auras,
                    plates::enemy_plates,
                    plates::compact_plates,
                    plates::boss_bar,
                    plates::boss_auras,
                    plates::target_edge,
                    plates::fill_glyphs,
                    minimap::map_buttons.run_if(ui_edit::inactive),
                    minimap::show_cluster,
                    minimap::clock,
                    minimap::zone_text,
                    minimap::compose,
                )
                    .chain()
                    .after(hud::show_panels),
            )
            .add_systems(
                PostUpdate,
                card_view::refresh_render_scale
                    .after(bevy::ui::UiSystems::Propagate)
                    .before(bevy::text::detect_text_needs_rerender::<Text>),
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
            .add_systems(OnEnter(Phase::Menu), (unpause, reset_run, auto_attack::reset));
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
    pub low_memory: bool,
    pub unlocked_races: Vec<u8>,
    pub class: Option<usize>,
    /// `(display, race, sex)`, one per race and sex the class's trainers come in.
    pub looks: Vec<(u32, u8, u8)>,
    pub look: Option<usize>,
    /// An index into [`data::MAPS`].
    pub map: usize,
    /// The battleground shown behind the menu or used for the run.
    pub stage: usize,
    /// Bumped on every change, so the menu rebuilds once.
    pub version: u32,
    /// The FEATURES dials, kept per class (and the race and sex they were set on); a hero with no
    /// entry wears its display's own look.
    pub features: std::collections::HashMap<(usize, u8, u8), appearance::Features>,
    /// The zones whose final boss this player has beaten, kept by [`progress`].
    pub cleared: std::collections::BTreeSet<String>,
    /// The selected class and permanent level used for raid eligibility.
    pub story_class: Option<u8>,
    pub class_level: u32,
}

impl Selection {
    pub fn race_unlocked(&self, race: u8) -> bool {
        matches!(race, 1..=8) || self.unlocked_races.contains(&race)
    }
    pub fn can_visit(&self, map: usize) -> bool {
        let race = self.look.and_then(|i| self.looks.get(i)).map(|look| look.1).unwrap_or(1);
        data::MAPS.get(map).is_some_and(|map| races::can_enter(race, map.name))
    }
    /// Whether the account has cleared the road to a battleground (co-op guests follow their host).
    pub fn unlocked(&self, map: usize) -> bool {
        data::MAPS
            .get(map)
            .is_some_and(|map| difficulty::unlocked(map.name, &self.cleared) && self.raid_ready(map.name))
    }

    pub fn raid_ready(&self, zone: &str) -> bool {
        !difficulty::is_raid(zone) || (
            self.class_def().is_some_and(|class| self.story_class == Some(class.id))
                && self.class_level >= difficulty::RAID_LEVEL
        )
    }

    pub fn unlock_hint(&self, zone: &str) -> String {
        let mut needs = Vec::new();
        if !difficulty::unlocked(zone, &self.cleared) {
            needs.push(difficulty::unlock_hint(zone));
        }
        if difficulty::is_raid(zone) && self.class_level < difficulty::RAID_LEVEL {
            needs.push(format!("Class level {} (current {})", difficulty::RAID_LEVEL, self.class_level));
        }
        needs.join(" · ")
    }
    /// Keep the world preview on the same battleground the menu and next run select.
    pub fn select_map(&mut self, map: usize) {
        self.map = map;
        self.stage = map;
        self.version += 1;
    }

    /// The picked hero's dials, if any were set.
    pub fn current_features(&self) -> Option<appearance::Features> {
        let &(_, race, sex) = self.looks.get(self.look?)?;
        self.features.get(&(self.class?, race, sex)).copied()
    }

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
    damage_source: Option<damage_meter::Source>,
    next_source: Option<damage_meter::Source>,
}

impl Owned {
    pub fn new(def: &'static AbilityDef, rarity: rewards::Rarity) -> Self {
        Self {
            def,
            level: 1,
            cd: 0.5,
            echoing: false,
            damage_source: None,
            next_source: None,
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
    /// The fraction of every blow the hero shrugs off (native Hero+0x4e8).
    pub reduction: f32,
    pub crit: f32,
    /// A critical strike's multiplier.
    pub crit_mult: f32,
    pub extra_targets: u32,
}

/// Health and damage reduction from the native class rows (RVA 0x76bc450, +0x80 and +0x84);
/// warriors and the unlisted ids share the default row at 0x6fe8ae0.
pub(super) fn class_base(id: u8) -> (f32, f32) {
    match id {
        2 => (245.0, 0.20),
        3 => (240.0, 0.16),
        4 => (190.0, 0.18),
        5 => (170.0, 0.08),
        7 => (195.0, 0.17),
        8 => (215.0, 0.08),
        9 => (165.0, 0.06),
        11 => (205.0, 0.12),
        _ => (230.0, 0.22),
    }
}
/// Native stat rebuild 0x5be8f80: each level past the first adds 9 health and 3% damage.
const HP_PER_LEVEL: f32 = 9.0;
const MIGHT_PER_LEVEL: f32 = 0.03;
/// Damage reduction stops at 60% (0x5be95ca).
const MAX_REDUCTION: f32 = 0.6;
pub(super) const BASE_SPEED: f32 = 7.0;
/// Bonuses from every source together stop at 15% above the base run speed. Slows still apply.
pub(super) const MAX_SPEED: f32 = BASE_SPEED * 1.15;
/// Original ASR's base pickup radius (RVA 0x5be92f5).
pub(super) const BASE_MAGNET: f32 = 9.0;

/// One run's state.
#[derive(Resource)]
pub(super) struct Run {
    pub low_memory: bool,
    pub class: &'static ClassDef,
    pub race: u8,
    pub elapsed: f32,
    pub kills: u32,
    pub level: u32,
    pub xp: f32,
    pub hp: f32,
    pub abilities: Vec<Owned>,
    /// The race's proc, cast with the spells but off the six-slot bar.
    pub racial: Option<Owned>,
    pub amulet: Option<amulets::Amulet>,
    pub shield: f32,
    pub shield_t: f32,
    pub shield_spell: u32,
    pub immune_t: f32,
    pub gear: [f32; 5],
    /// Item affixes keyed by spell, as fractions (1000 basis points = 10%).
    pub spell_damage: std::collections::HashMap<String, f32>,
    pub gear_look: Vec<asr_coop::economy::LookItem>,
    pub equipment_kills: [u32; 3],
    /// Boss names whose kills are ready to resolve through the zone's named loot table.
    pub boss_loot: Vec<String>,
    /// Kills by creature name, `[normal, elite]`, toward achievements.
    pub tally: std::collections::BTreeMap<String, [u32; 2]>,
    pub thorns: f32,
    pub thorns_t: f32,
    pub thorns_spell: u32,
    thorns_source: Option<damage_meter::Source>,
    pub heal_rate: f32,
    pub heal_t: f32,
    pub pending_levels: u32,
    pub pending_chests: u32,
    /// A Test Grounds run: nothing spawns by itself and nothing it earns is saved.
    pub test: bool,
    /// Test Grounds: the hero takes no damage.
    pub test_god: bool,
    /// Test Grounds: enemies hold at 1 health instead of dying.
    pub test_immortal: bool,
    pub jewelry: Vec<jewelry::Jewel>,
    /// The chest food carried, one slot per kind.
    pub food: food::Slots,
    pub full_belly: u32,
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
    damage_by_source: std::collections::HashMap<damage_meter::Source, f64>,
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
    /// Enemy attacks route through the host to this guest; zero means the local hero.
    pub coop_target: u32,
    pub coop_damage: Vec<(u32, f32)>,
    brood_poison: mother_fang::Poison,
    morgan_bleed: morgan::Bleed,
    morgan_mark_until: f32,
    morgan_bomb_slow_until: f32,
    coop_morgan_debuffs: Vec<(u32, morgan::Debuff)>,
    stun_until: f32,
    healing_blocked_until: f32,
    coop_boar_impacts: Vec<(u32, f32, f32)>,
    pub coop_poison: Vec<(u32, f32)>,
    /// A boss's pull still to move the local hero, and each guest by peer.
    pub push: Vec3,
    pub coop_push: Vec<(u32, Vec3)>,
    pub coop_run: bool,
    pub coop_unavailable: std::collections::HashSet<u32>,
    /// Living party members (native Game+0xc88, rebuilt at 0x5bd91a0); 1 when solo.
    pub coop_living: usize,
    /// This update's ordinary-spawn reservations, including deferred entities and summons.
    pub horde_spawn_slots: Option<usize>,
    /// Key, native effect, strength, and remaining seconds of active spell buffs.
    pub buffs: Vec<(&'static str, u32, f32, f32)>,
    passives: spell_passives::State,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            low_memory: false,
            class: &data::CLASSES[0],
            race: 0,
            elapsed: 0.0,
            kills: 0,
            level: 1,
            xp: 0.0,
            hp: class_base(data::CLASSES[0].id).0,
            abilities: Vec::new(),
            racial: None,
            amulet: None,
            shield: 0.0,
            shield_t: 0.0,
            shield_spell: 0,
            immune_t: 0.0,
            gear: [0.; 5],
            spell_damage: Default::default(),
            gear_look: Vec::new(),
            equipment_kills: [0; 3],
            boss_loot: Vec::new(),
            tally: Default::default(),
            thorns: 0.0,
            thorns_t: 0.0,
            thorns_spell: 0,
            thorns_source: None,
            heal_rate: 0.0,
            heal_t: 0.0,
            pending_levels: 0,
            pending_chests: 0,
            test: false,
            test_god: false,
            test_immortal: false,
            jewelry: Vec::new(),
            food: [None; food::SLOTS],
            full_belly: 0,
            gold: 0,
            chest_cost: 20,
            revives_used: 0,
            bosses: 0,
            slain_last: false,
            won: false,
            dead_t: None,
            next_guid: 1,
            damage_done: 0.0,
            damage_by_source: Default::default(),
            version: 0,
            banner: None,
            yell: None,
            talents: talents::Build::default(),
            talent_pay: (0.0, 0),
            talent_paid: false,
            hand: cards::Hand::default(),
            struck: false,
            coop_target: 0,
            coop_damage: Vec::new(),
            brood_poison: Default::default(),
            morgan_bleed: Default::default(),
            morgan_mark_until: 0.0,
            morgan_bomb_slow_until: 0.0,
            coop_morgan_debuffs: Vec::new(),
            stun_until: 0.0,
            healing_blocked_until: 0.0,
            coop_boar_impacts: Vec::new(),
            coop_poison: Vec::new(),
            push: Vec3::ZERO,
            coop_push: Vec::new(),
            coop_run: false,
            coop_unavailable: Default::default(),
            coop_living: 1,
            horde_spawn_slots: None,
            buffs: Vec::new(),
            passives: spell_passives::State::default(),
        }
    }
}

impl Run {
    fn record_damage(&mut self, source: Option<damage_meter::Source>, spell: u32, damage: f32) {
        if damage > 0.0 && damage.is_finite() {
            let damage = f64::from(damage);
            self.damage_done += damage;
            let source = source.or_else(|| self.abilities.iter().find(|o| o.def.spell == spell).and_then(|o| o.damage_source))
                .unwrap_or(damage_meter::Source::Spell(spell));
            *self.damage_by_source.entry(source).or_default() += damage;
        }
    }

    /// Independent effects keep their owner; stat increases stay on the spell they boost.
    fn effect_sources(&self, talent_stat: u32, item_stat: u8) -> Vec<(damage_meter::Source, f32)> {
        let mut sources: Vec<_> = self.talents.sources.iter().filter(|(_, stat, _)| *stat == talent_stat)
            .map(|&(key, _, value)| (damage_meter::Source::Talent(key), value)).collect();
        for j in &self.jewelry {
            let value = jewelry::stat(j.index, j.strength, item_stat);
            if value > 0.0 { sources.push((damage_meter::Source::Jewelry(j.index), value)); }
        }
        if let Some(a) = &self.amulet {
            let value: f32 = amulets::AMULETS[a.index].stats.iter().filter(|(stat, _)| *stat == item_stat)
                .map(|(_, value)| value * a.scale()).sum();
            if value > 0.0 { sources.push((damage_meter::Source::Amulet(a.index), value)); }
        }
        for held in self.food.iter().flatten() {
            let value = food::food_stat(*held, item_stat);
            if value > 0.0 { sources.push((damage_meter::Source::Food(*held), value)); }
        }
        sources
    }
    /// Native online scaling `1 + (living - 1) * chapter * coefficient`; the patched
    /// chapter lookup (0x9dc2c70) gives 0.7 below chapter three. Co-op is Elwynn only.
    /// Coefficients: 0.48 density and pickup shares, 0.72 ordinary health
    /// (0x5c00785), 1.04 boss and lair health (0x5c134be, 0x5c6798a).
    pub fn party_factor(&self, coefficient: f32) -> f32 {
        if !self.coop_run {
            return 1.0;
        }
        1.0 + self.coop_living.saturating_sub(1) as f32 * 0.7 * coefficient
    }
    pub fn stunned(&self) -> bool {
        self.stun_until > self.elapsed
    }

    /// Ordinary healing cannot bring back a fallen hero or bypass encounter healing blocks.
    pub fn heal(&mut self, amount: f32) {
        if self.dead_t.is_none() && self.hp > 0.0
            && self.healing_blocked_until <= self.elapsed && self.morgan_bleed.until <= self.elapsed
        {
            let max = self.stats().max_hp;
            let overheal = (self.hp + amount - max).max(0.0);
            self.hp = (self.hp + amount).min(max);
            if overheal > 0.0 && self.talents.has("tree_of_life") {
                talent_combat::shield(self, overheal, max * 0.2, 774);
            }
        }
    }

    pub fn stats(&self) -> Stats {
        let (base_hp, base_reduction) = class_base(self.class.id);
        let levels = self.level.saturating_sub(1) as f32;
        let mut s = Stats {
            might: 1.0 + MIGHT_PER_LEVEL * levels,
            max_hp: base_hp + HP_PER_LEVEL * levels,
            speed: BASE_SPEED,
            // No base regeneration: the native 0.8/s (+0.08 a level) outhealed the early horde.
            // Racials, talents and jewelry still add theirs below.
            regen: 0.0,
            haste: 0.0,
            area: 1.0,
            magnet: BASE_MAGNET,
            growth: 1.0,
            reduction: base_reduction,
            crit: 0.05,
            crit_mult: 2.0,
            extra_targets: self
                .amulet
                .as_ref()
                .map_or(0, |a| amulets::AMULETS[a.index].targets)
                + food::targets(&self.food),
        };
        s.max_hp += self.gear[0];
        s.haste += self.gear[2];
        s.crit += self.gear[3];
        // Cloth starts quicker.
        if !matches!(self.class.id, 1 | 2 | 7 | 11 | 3 | 4) {
            s.speed += 0.5;
        }
        let race = |kind| races::bonus(self.race, kind);
        s.might += self.jewel_stat(0) + race(Bonus::Damage);
        s.haste += self.jewel_stat(1) + race(Bonus::Cooldowns);
        s.max_hp *= 1. + self.jewel_stat(2) + self.jewel_stat(6) + race(Bonus::Health);
        s.crit += self.jewel_stat(3) + race(Bonus::Crit);
        s.crit_mult += self.jewel_stat(4);
        s.regen += self.jewel_stat(7) + race(Bonus::Regen);
        s.area += self.jewel_stat(8);
        s.speed *= 1. + self.jewel_stat(10) + race(Bonus::Speed);
        s.magnet *= 1. + self.jewel_stat(11) + race(Bonus::Pickup);
        s.growth += self.jewel_stat(12) + race(Bonus::Experience);
        let t = &self.talents;
        s.might += t.stat(0);
        s.haste += t.stat(1);
        s.max_hp *= 1.0 + t.stat(2);
        // Gear multiplies damage, so a higher tier's weapons keep pace with its tougher enemies.
        s.might *= 1.0 + self.gear[1];
        s.crit += t.stat(3);
        s.crit_mult += t.stat(4);
        s.regen += t.stat(6) + t.stat(24) * s.max_hp;
        s.speed = (s.speed * (1.0 + t.stat(7))).min(MAX_SPEED);
        let reduction_cap = MAX_REDUCTION + t.stat(27);
        let raw_reduction = s.reduction
            + t.stat(5)
            + self.jewel_stat(5)
            + self.gear[4]
            + race(Bonus::Reduction);
        s.reduction = raw_reduction.min(reduction_cap);
        s.might *= 1.0 + t.stat(25) * s.reduction * 10.0
            + t.stat(26) * (raw_reduction - reduction_cap).max(0.0) * 100.0;
        if self.shield > 0.0 && self.shield_t > 0.0 { s.might *= 1.0 + t.stat(28); }
        if t.combat.flurry_until > self.elapsed { s.haste += t.value("flurry_shaman"); }
        for &(_, effect, strength, _) in &self.buffs {
            match effect {
                8 | 10 => s.might += strength,
                9 => s.haste += strength,
                _ => {}
            }
        }
        if self.brood_poison.until > self.elapsed { s.speed *= 0.7; }
        if self.morgan_bomb_slow_until > self.elapsed { s.speed *= 0.5; }
        if self.stunned() { s.speed = 0.0; }
        s
    }

    pub fn full_belly_multiplier(&self) -> f32 {
        1.0 + self.full_belly as f32 * (food::FULL_BELLY_DAMAGE_PERCENT as f32 / 100.0)
    }

    /// Gold find: jewelry and the race.
    pub fn gold_find(&self) -> f32 {
        self.jewel_stat(13) + races::bonus(self.race, Bonus::Gold)
    }

    /// Luck, which lifts reward rarity.
    pub fn luck(&self) -> f32 {
        self.jewel_stat(14) + races::bonus(self.race, Bonus::Luck) + self.talents.stat(12)
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
            + food::stat(&self.food, stat)
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

fn pause(mut time: ResMut<Time<Virtual>>, coop: Res<coop::Coop>) {
    if !coop.running { time.pause(); }
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
    mut progress: ResMut<progress::Progress>,
    mut coop: ResMut<coop::Coop>,
) {
    progress.settle(&run, selection.stage_def().name, run.coop_run);
    if coop.active() {
        coop.leave();
    }
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
