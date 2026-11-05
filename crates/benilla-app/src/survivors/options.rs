//! Escape-menu settings, backed by the same saved CVars as the engine controls: a window in
//! the HUD's frame with the topics down a sidebar under a search box, and the open topic's tabs
//! and rows beside them.
mod preview;

use super::{hud::HudFont, right_column as col, Phase};
use bevy::input::{keyboard::KeyboardInput, ButtonState};
use bevy::prelude::*;
use benilla_world::view::MsaaFormats;

#[derive(Resource, Default)]
pub(super) struct Options {
    pub open: bool,
    pub settings: bool,
    /// The open topic and its tab, indices into [`TOPICS`].
    topic: u8,
    tab: u8,
    /// The search box: its text, whether it takes keys, and its caret.
    search: String,
    search_active: bool,
    cursor: super::text_edit::Cursor,
}

impl Options {
    pub fn escape(&mut self, phase: Phase, next: &mut NextState<Phase>) {
        // A search in progress clears first.
        if self.settings && (self.search_active || !self.search.is_empty()) {
            self.search.clear();
            self.search_active = false;
        } else if self.settings {
            self.close_settings(phase);
        } else if self.open || phase == Phase::Paused {
            self.resume(phase, next);
        } else {
            self.open = true;
            self.settings = phase == Phase::Menu;
            if phase == Phase::Playing {
                next.set(Phase::Paused);
            }
        }
    }

    /// Back out of the settings: to the pause menu in a run, all the way out from the hero menu.
    fn close_settings(&mut self, phase: Phase) {
        self.settings = false;
        self.search.clear();
        self.search_active = false;
        if phase == Phase::Menu {
            self.open = false;
        }
    }

    pub fn resume(&mut self, phase: Phase, next: &mut NextState<Phase>) {
        self.open = false;
        self.settings = false;
        if phase == Phase::Paused {
            next.set(Phase::Playing);
        }
    }
}

struct Setting {
    label: &'static str,
    key: &'static str,
    values: &'static [&'static str],
    labels: &'static [&'static str],
}

const SETTINGS: &[Setting] = &[
    Setting {
        label: "Display mode",
        key: "gxWindow",
        values: &["1", "0"],
        labels: &["Windowed", "Fullscreen"],
    },
    Setting {
        label: "Vertical sync",
        key: "gxVSync",
        values: &["0", "1"],
        labels: &["Off", "On"],
    },
    Setting {
        label: "Render scale",
        key: "renderScale",
        values: &["0.5", "0.59", "0.67", "0.75", "0.77", "0.85", "1", "1.25", "1.5", "2"],
        labels: &["50%", "59%", "67%", "75%", "77%", "85%", "100%", "125%", "150%", "200%"],
    },
    #[cfg(not(target_os = "macos"))]
    Setting {
        label: "Upscaling (<100%)",
        key: "SurvivorUpscaler",
        values: &["0", "1"],
        labels: &["Bilinear", "FSR 1 (EASU)"],
    },
    Setting {
        label: "View distance",
        key: "farclip",
        values: &["177", "300", "500", "777"],
        labels: &["Low", "Medium", "High", "Ultra"],
    },
    Setting {
        label: "Grass and flowers",
        key: "WorldDetail",
        values: &["0", "1", "2"],
        labels: &["Low", "Medium", "High"],
    },
    Setting {
        label: "Bloom",
        key: "ffxGlow",
        values: &["0", "1"],
        labels: &["Off", "On"],
    },
    Setting {
        label: "World color grade",
        key: "SurvivorWorldGrade",
        values: &["0", "1"],
        labels: &["Off", "On"],
    },
    Setting {
        label: "Anti-aliasing (restart)",
        key: "gxMultisample",
        values: &["1", "2", "4", "8"],
        labels: &["Off", "2×", "4×", "8×"],
    },
    Setting {
        label: "Fast anti-aliasing",
        key: "SurvivorFxaa",
        values: &["0", "1"],
        labels: &["Off", "FXAA"],
    },
    Setting {
        label: "Spell effect detail",
        key: "spellEffectLevel",
        values: &["0", "1", "2"],
        labels: &["Low", "Medium", "High"],
    },
    Setting {
        label: "Weather intensity",
        key: "weatherDensity",
        values: &["0", "1", "2", "3"],
        labels: &["10%", "33%", "66%", "100%"],
    },
    Setting {
        label: "Mip filtering (restart)",
        key: "trilinear",
        values: &["0", "1"],
        labels: &["Bilinear", "Trilinear"],
    },
    Setting {
        label: "Anisotropic filtering (restart)",
        key: "anisotropic",
        values: &["1", "2", "4", "8", "16"],
        labels: &["Off", "2×", "4×", "8×", "16×"],
    },
    Setting {
        label: "Sound",
        key: "MasterSoundEffects",
        values: &["0", "1"],
        labels: &["Muted", "On"],
    },
    Setting {
        label: "Master volume",
        key: "MasterVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
    },
    Setting {
        label: "Effects volume",
        key: "SoundVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
    },
    Setting {
        label: "Music volume",
        key: "MusicVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
    },
    Setting {
        label: "Ambience volume",
        key: "AmbienceVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
    },
    Setting {
        label: "Minimap size",
        key: super::right_column::MAP_SIZE,
        values: HUD_SIZES,
        labels: HUD_SIZE_LABELS,
    },
    Setting {
        label: "Run tracker size",
        key: super::right_column::TRACKER_SIZE,
        values: HUD_SIZES,
        labels: HUD_SIZE_LABELS,
    },
    Setting {
        label: "Damage meter size",
        key: super::right_column::METER_SIZE,
        values: HUD_SIZES,
        labels: HUD_SIZE_LABELS,
    },
    Setting {
        label: "Action bar size",
        key: super::right_column::BAR_SIZE,
        values: HUD_SIZES,
        labels: HUD_SIZE_LABELS,
    },
    Setting {
        label: "Health text",
        key: super::unit_frames::HEALTH_TEXT,
        values: &["0", "1", "2", "3", "4", "5"],
        labels: &[
            "Cur / Max · %",
            "Cur / Max",
            "Cur · %",
            "Current",
            "Percent",
            "None",
        ],
    },
    Setting {
        label: "Health bar height",
        key: super::unit_frames::HEALTH_HEIGHT,
        values: &["11", "13", "15", "17", "20", "24", "28"],
        labels: &[
            "11 px", "13 px", "15 px", "17 px", "20 px", "24 px", "28 px",
        ],
    },
    Setting {
        label: "Frame buffs",
        key: super::unit_frames::FRAME_BUFFS,
        values: AURA_PLACES,
        labels: AURA_PLACE_LABELS,
    },
    Setting {
        label: "Frame debuffs",
        key: super::unit_frames::FRAME_DEBUFFS,
        values: AURA_PLACES,
        labels: AURA_PLACE_LABELS,
    },
    Setting {
        label: "Frame buff size",
        key: super::unit_frames::FRAME_BUFF_SIZE,
        values: FRAME_AURA_SIZES,
        labels: FRAME_AURA_SIZE_LABELS,
    },
    Setting {
        label: "Frame debuff size",
        key: super::unit_frames::FRAME_DEBUFF_SIZE,
        values: FRAME_AURA_SIZES,
        labels: FRAME_AURA_SIZE_LABELS,
    },
    Setting {
        label: "Plate debuffs",
        key: super::unit_frames::PLATE_DEBUFFS,
        values: AURA_PLACES,
        labels: AURA_PLACE_LABELS,
    },
    Setting {
        label: "Plate buffs",
        key: super::unit_frames::PLATE_BUFFS,
        values: AURA_PLACES,
        labels: AURA_PLACE_LABELS,
    },
    Setting {
        label: "Plate debuff size",
        key: super::unit_frames::PLATE_DEBUFF_SIZE,
        values: PLATE_AURA_SIZES,
        labels: PLATE_AURA_SIZE_LABELS,
    },
    Setting {
        label: "Plate buff size",
        key: super::unit_frames::PLATE_BUFF_SIZE,
        values: PLATE_AURA_SIZES,
        labels: PLATE_AURA_SIZE_LABELS,
    },
    // An unset row shows its first value, so the plates' default, Cur · %, leads.
    Setting {
        label: "Nameplate health",
        key: super::plates::PLATE_HEALTH_TEXT,
        values: &["2", "0", "1", "3", "4", "5"],
        labels: &[
            "Cur · %",
            "Cur / Max · %",
            "Cur / Max",
            "Current",
            "Percent",
            "None",
        ],
    },
];

/// The Auras tabs' values: an edge (`unit_frames::AuraPos`), and icon sizes in UI pixels.
const AURA_PLACES: &[&str] = &["0", "1", "2"];
const AURA_PLACE_LABELS: &[&str] = &["Above", "Below", "Hidden"];
const FRAME_AURA_SIZES: &[&str] = &["10", "12", "14", "16", "18", "22"];
const FRAME_AURA_SIZE_LABELS: &[&str] = &["10 px", "12 px", "14 px", "16 px", "18 px", "22 px"];
const PLATE_AURA_SIZES: &[&str] = &["7", "9", "11", "13", "16"];
const PLATE_AURA_SIZE_LABELS: &[&str] = &["7 px", "9 px", "11 px", "13 px", "16 px"];

const HUD_SIZES: &[&str] = &["0.8", "0.9", "1", "1.1", "1.2", "1.35", "1.5"];
const HUD_SIZE_LABELS: &[&str] = &["80%", "90%", "100%", "110%", "120%", "135%", "150%"];

const QUALITY_LEVELS: [&str; 4] = ["Low", "Medium", "High", "Ultra"];
const QUALITY_SETTINGS: &[(&str, [&str; QUALITY_LEVELS.len()])] = &[
    ("renderScale", ["0.67", "0.85", "1", "1"]),
    ("farclip", ["177", "300", "500", "777"]),
    ("WorldDetail", ["0", "1", "2", "2"]),
    ("spellEffectLevel", ["0", "1", "2", "2"]),
    ("weatherDensity", ["0", "1", "2", "3"]),
    ("ffxGlow", ["0", "1", "1", "1"]),
    ("SurvivorFxaa", ["0", "1", "1", "1"]),
    ("trilinear", ["0", "1", "1", "1"]),
    ("anisotropic", ["1", "2", "4", "16"]),
];

fn quality_index(cvars: &crate::cvars::Cvars) -> Option<usize> {
    (0..QUALITY_LEVELS.len()).find(|&i| {
        QUALITY_SETTINGS
            .iter()
            .all(|(key, values)| cvars.num(key) == values[i].parse::<f32>().ok())
    })
}

/// What an unset row shows: its first value, but the health bar's default height mid-list.
fn unset(setting: &Setting) -> &'static str {
    if setting.key == super::unit_frames::HEALTH_HEIGHT {
        "13"
    } else {
        setting.values[0]
    }
}

fn nearest(setting: &Setting, value: &str) -> usize {
    let value = value.parse::<f32>().unwrap_or(0.0);
    setting
        .values
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let distance = |s: &str| (s.parse::<f32>().unwrap_or(0.0) - value).abs();
            distance(a).total_cmp(&distance(b))
        })
        .map_or(0, |(i, _)| i)
}

fn supports_samples(formats: Option<&MsaaFormats>, samples: u32) -> bool {
    formats.is_none_or(|formats| {
        formats.formats.is_empty() || formats.formats.iter().any(|&(_, _, n)| n == samples)
    })
}

fn selected_index(setting: &Setting, value: &str, formats: Option<&MsaaFormats>) -> usize {
    if setting.key != "gxMultisample" {
        return nearest(setting, value);
    }
    let requested = value.parse::<u32>().unwrap_or(1);
    setting
        .values
        .iter()
        .rposition(|value| {
            value
                .parse::<u32>()
                .is_ok_and(|n| n <= requested && supports_samples(formats, n))
        })
        .unwrap_or(0)
}

/// The topics down the sidebar: their group, name, line under the title, icon spell and tabs.
struct Topic {
    group: &'static str,
    name: &'static str,
    about: &'static str,
    /// The PNGs from `tools/export_settings_icons.js`, for the sidebar and the page title.
    icon_small: &'static [u8],
    icon: &'static [u8],
    tabs: &'static [&'static str],
}

const TOPICS: &[Topic] = &[
    Topic {
        group: "Game",
        name: "Graphics",
        about: "How the world is drawn.",
        icon_small: include_bytes!("../../assets/settings/graphics_small.png"),
        icon: include_bytes!("../../assets/settings/graphics.png"),
        tabs: &["Display", "World", "Textures"],
    },
    Topic {
        group: "Game",
        name: "Audio",
        about: "Volume and sound.",
        icon_small: include_bytes!("../../assets/settings/audio_small.png"),
        icon: include_bytes!("../../assets/settings/audio.png"),
        tabs: &["Volume"],
    },
    Topic {
        group: "Interface",
        name: "HUD layout",
        about: "The sizes of the panels round the screen.",
        icon_small: include_bytes!("../../assets/settings/hud_layout_small.png"),
        icon: include_bytes!("../../assets/settings/hud_layout.png"),
        tabs: &["Sizes"],
    },
    Topic {
        group: "Interface",
        name: "Unit frames",
        about: "Your frame and your target's.",
        icon_small: include_bytes!("../../assets/settings/unit_frames_small.png"),
        icon: include_bytes!("../../assets/settings/unit_frames.png"),
        tabs: &["Health", "Auras"],
    },
    Topic {
        group: "Interface",
        name: "Nameplates",
        about: "The bars and names over units' heads.",
        icon_small: include_bytes!("../../assets/settings/nameplates_small.png"),
        icon: include_bytes!("../../assets/settings/nameplates.png"),
        tabs: &["Plates", "Auras", "Names"],
    },
    Topic {
        group: "Interface",
        name: "Encounter warnings",
        about: "Compact boss instructions, severity filters and cooldown visibility.",
        icon_small: include_bytes!("../../assets/settings/hud_layout_small.png"),
        icon: include_bytes!("../../assets/settings/hud_layout.png"),
        tabs: &["Visibility"],
    },
];

/// One row of the page: a stepped value from [`SETTINGS`] or an on/off switch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Item {
    Quality,
    Value(usize),
    Flag(InterfaceSetting),
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Self::Quality => "Graphics quality",
            Self::Value(i) => SETTINGS[i].label,
            Self::Flag(f) => f.label(),
        }
    }

    /// The topic and tab the row sits on.
    fn place(self) -> (u8, u8) {
        match self {
            Self::Quality => (0, 0),
            Self::Value(i) => match SETTINGS[i].key {
                "gxWindow" | "gxVSync" | "renderScale" | "gxMultisample" | "SurvivorUpscaler"
                | "SurvivorFxaa" => (0, 0),
                "farclip" | "WorldDetail" | "ffxGlow" | "spellEffectLevel" | "weatherDensity"
                | "SurvivorWorldGrade" => (0, 1),
                "trilinear" | "anisotropic" => (0, 2),
                "MasterSoundEffects" | "MasterVolume" | "SoundVolume" | "MusicVolume"
                | "AmbienceVolume" => (1, 0),
                super::unit_frames::HEALTH_TEXT | super::unit_frames::HEALTH_HEIGHT => (3, 0),
                super::unit_frames::FRAME_BUFFS
                | super::unit_frames::FRAME_DEBUFFS
                | super::unit_frames::FRAME_BUFF_SIZE
                | super::unit_frames::FRAME_DEBUFF_SIZE => (3, 1),
                super::unit_frames::PLATE_BUFFS
                | super::unit_frames::PLATE_DEBUFFS
                | super::unit_frames::PLATE_BUFF_SIZE
                | super::unit_frames::PLATE_DEBUFF_SIZE => (4, 1),
                super::plates::PLATE_HEALTH_TEXT => (4, 0),
                _ => (2, 0),
            },
            Self::Flag(
                InterfaceSetting::EnemyPlates
                | InterfaceSetting::FriendlyPlates
                | InterfaceSetting::CompactPlates
                | InterfaceSetting::HeroPlate,
            ) => (4, 0),
            Self::Flag(InterfaceSetting::EncounterWarnings | InterfaceSetting::WarningLethal
                | InterfaceSetting::WarningDangerous | InterfaceSetting::WarningStandard
                | InterfaceSetting::BossCooldowns) => (5, 0),
            Self::Flag(_) => (4, 2),
        }
    }

    /// Shown on the open page, or, while searching, when its name or topic matches.
    fn shown(self, options: &Options) -> bool {
        let query = options.search.trim().to_lowercase();
        if query.is_empty() {
            return self.place() == (options.topic, options.tab);
        }
        let (topic, tab) = self.place();
        let topic = &TOPICS[usize::from(topic)];
        [self.label(), topic.name, topic.tabs[usize::from(tab)]]
            .iter()
            .any(|s| s.to_lowercase().contains(&query))
    }

    fn all() -> impl Iterator<Item = Self> {
        std::iter::once(Self::Quality)
            .chain((0..SETTINGS.len()).map(Self::Value))
            .chain(InterfaceSetting::ALL.into_iter().map(Self::Flag))
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(Item);
/// A row's "Topic · Tab" line, shown only in search results.
#[derive(Component)]
struct RowWhere;
#[derive(Component)]
struct ValueLabel(Item);
/// A switch's knob.
#[derive(Component)]
struct Switch(InterfaceSetting);
#[derive(Component)]
struct TopicIcon(u8);

/// A topic's picture in the sidebar: dimmed until its row is open or under the pointer.
#[derive(Component)]
struct SideIcon(u8);
#[derive(Component)]
enum PageText {
    Title,
    About,
}
#[derive(Component)]
struct SearchText;
#[derive(Component)]
struct SearchBox;
#[derive(Component)]
struct InterfaceHint;
#[derive(Component)]
struct NoMatch;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Adjust(usize, i32),
    AdjustQuality(i32),
    Topic(u8),
    Tab(u8, u8),
    Toggle(InterfaceSetting),
    Search,
    Back,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InterfaceSetting {
    EncounterWarnings,
    WarningLethal,
    WarningDangerous,
    WarningStandard,
    BossCooldowns,
    PlayerNames,
    NpcNames,
    OwnName,
    GuildNames,
    EnemyPlates,
    FriendlyPlates,
    CompactPlates,
    HeroPlate,
}

impl InterfaceSetting {
    const ALL: [Self; 13] = [
        Self::EncounterWarnings,
        Self::WarningLethal,
        Self::WarningDangerous,
        Self::WarningStandard,
        Self::BossCooldowns,
        Self::PlayerNames,
        Self::NpcNames,
        Self::OwnName,
        Self::GuildNames,
        Self::EnemyPlates,
        Self::FriendlyPlates,
        Self::CompactPlates,
        Self::HeroPlate,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::EncounterWarnings => "Show encounter warnings",
            Self::WarningLethal => "Lethal warnings (red skull)",
            Self::WarningDangerous => "Dangerous warnings (white skull)",
            Self::WarningStandard => "Standard warnings (steel diamond)",
            Self::BossCooldowns => "Show boss cooldowns",
            Self::PlayerNames => "Player names",
            Self::NpcNames => "NPC names",
            Self::OwnName => "Your name",
            Self::GuildNames => "Guild names",
            Self::EnemyPlates => "Enemy nameplates",
            Self::FriendlyPlates => "Friendly nameplates",
            Self::CompactPlates => "Compact nameplates",
            Self::HeroPlate => "Your health bar",
        }
    }

    fn cvar(self) -> Option<&'static str> {
        match self {
            Self::EncounterWarnings => Some("SurvivorEncounterWarnings"),
            Self::WarningLethal => Some("SurvivorWarningLethal"),
            Self::WarningDangerous => Some("SurvivorWarningDangerous"),
            Self::WarningStandard => Some("SurvivorWarningStandard"),
            Self::BossCooldowns => Some("SurvivorBossCooldowns"),
            Self::PlayerNames => Some("UnitNamePlayer"),
            Self::NpcNames => Some("UnitNameNPC"),
            Self::OwnName => Some("UnitNameOwn"),
            Self::GuildNames => Some("UnitNamePlayerGuild"),
            Self::EnemyPlates | Self::FriendlyPlates | Self::CompactPlates | Self::HeroPlate => {
                None
            }
        }
    }

    fn enabled(
        self,
        cvars: &crate::cvars::Cvars,
        plates: &super::plates::NameplateSettings,
    ) -> bool {
        match self {
            Self::EnemyPlates => plates.enemies,
            Self::FriendlyPlates => plates.friends,
            Self::CompactPlates => plates.compact,
            Self::HeroPlate => plates.hero,
            _ => self.cvar().and_then(|key| cvars.flag(key)).unwrap_or(false),
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Options>().add_systems(
        Update,
        (
            spawn,
            search_input,
            clicks,
            paint_nav,
            paint_page,
            preview::paint,
        )
            .chain()
            .after(super::hud::buttons)
            .before(super::hud::keys),
    );
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    if let Ok(preview) = std::env::var("WOW_SURVIVORS_OPTIONS_PREVIEW") {
        app.insert_resource(Options {
            open: true,
            settings: preview != "menu",
            topic: if preview == "audio" { 1 } else { 0 },
            ..default()
        });
    }
}

/// The window's own tones on top of the HUD's palette.
const SIDEBAR: Color = Color::srgba(0.035, 0.05, 0.06, 0.97);
pub(super) const ROW: Color = Color::srgb(0.063, 0.086, 0.11);
const WELL: Color = Color::srgb(0.027, 0.035, 0.047);
pub(super) const EDGE_DIM: Color = Color::srgb(0.173, 0.196, 0.239);
const HOVER: Color = Color::srgb(0.114, 0.145, 0.188);
pub(super) const PICKED: Color = Color::srgb(0.165, 0.051, 0.086);

/// A word or line of the window's text, over the panel.
fn text(font: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    col::label(font, value, size, color)
}

/// A topic's picture, unframed.
fn icon(image: Handle<Image>, size: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.,
            ..default()
        },
        ImageNode::new(image),
    )
}

/// A ‹ or › arrow of a stepped value.
fn arrow(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, glyph: &str, action: Action) {
    parent
        .spawn((
            Button,
            action,
            Node {
                width: Val::Px(16.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
        ))
        .with_children(|b| {
            b.spawn(text(font, glyph, 13., col::STEEL));
        });
}

/// Two crossed bars, centred in their box: the close button's ×.
fn cross(parent: &mut ChildSpawnerCommands) {
    for turn in [45., -45.] {
        parent
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Px(0.),
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    Node {
                        width: Val::Px(10.),
                        height: Val::Px(1.5),
                        flex_shrink: 0.,
                        ..default()
                    },
                    BackgroundColor(col::STEEL_HI),
                    UiTransform::from_rotation(Rot2::degrees(turn)),
                ));
            });
    }
}

/// A nav button's text, coloured by whether its topic or tab is open.
#[derive(Component)]
struct NavLabel(Action);

fn spawn(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    roots: Query<(), With<Root>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !roots.is_empty() {
        return;
    }
    let Some(font) = font else {
        return;
    };
    let f = &font.0;
    commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
            // Modal: what it covers can't be clicked through it.
            bevy::ui::FocusPolicy::Block,
            GlobalZIndex(super::hud_layout::MENU_Z),
        ))
        .with_children(|root| {
            root.spawn((
                super::hud_layout::Movable::settings("settings", "Settings"),
                col::plate(Node {
                    width: Val::Px(860.),
                    height: Val::Px(520.),
                    max_width: Val::Percent(96.),
                    max_height: Val::Percent(92.),
                    overflow: Overflow::clip(),
                    ..default()
                }),
            ))
            .with_children(|window| {
                // Close, over the top right.
                window
                    .spawn((
                        Button,
                        Action::Back,
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(8.),
                            top: Val::Px(8.),
                            width: Val::Px(20.),
                            height: Val::Px(20.),
                            border: UiRect::all(Val::Px(1.)),
                            ..default()
                        },
                        BackgroundColor(col::KNOB),
                        BorderColor::all(col::STEEL_LO),
                        Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                        ZIndex(2),
                    ))
                    .with_children(cross);

                // The sidebar: the name, the search box and the topics in their groups.
                window
                    .spawn((
                        Node {
                            width: Val::Px(200.),
                            flex_shrink: 0.,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::vertical(Val::Px(12.)),
                            border: UiRect::right(Val::Px(1.)),
                            ..default()
                        },
                        BackgroundColor(SIDEBAR),
                        BorderColor::all(EDGE_DIM),
                    ))
                    .with_children(|side| {
                        side.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.),
                            padding: UiRect::new(
                                Val::Px(16.),
                                Val::Px(14.),
                                Val::ZERO,
                                Val::Px(10.),
                            ),
                            ..default()
                        })
                        .with_children(|brand| {
                            brand.spawn(col::diamond(Node::default(), 11., col::STEEL));
                            brand.spawn(text(f, "Settings", 16., col::STEEL_HI));
                        });
                        side.spawn((
                            Button,
                            Action::Search,
                            SearchBox,
                            Node {
                                height: Val::Px(26.),
                                margin: UiRect::new(
                                    Val::Px(12.),
                                    Val::Px(12.),
                                    Val::ZERO,
                                    Val::Px(8.),
                                ),
                                padding: UiRect::horizontal(Val::Px(8.)),
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip(),
                                border: UiRect::all(Val::Px(1.)),
                                ..default()
                            },
                            BackgroundColor(WELL),
                            BorderColor::all(EDGE_DIM),
                            Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                        ))
                        .with_children(|b| {
                            b.spawn((SearchText, text(f, "", 12., col::TEXT_DIM)));
                        });
                        let mut group = "";
                        for (i, topic) in TOPICS.iter().enumerate() {
                            if topic.group != group {
                                group = topic.group;
                                // A text node ignores its own padding: a box holds the label off
                                // the edge and the row above.
                                side.spawn(Node {
                                    padding: UiRect::new(
                                        Val::Px(14.),
                                        Val::ZERO,
                                        Val::Px(10.),
                                        Val::Px(4.),
                                    ),
                                    ..default()
                                })
                                .with_child(text(
                                    f,
                                    &group.to_uppercase(),
                                    10.,
                                    col::STEEL,
                                ));
                            }
                            let action = Action::Topic(i as u8);
                            side.spawn((
                                Button,
                                action,
                                Node {
                                    height: Val::Px(28.),
                                    padding: UiRect::horizontal(Val::Px(12.)),
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(9.),
                                    border: UiRect::left(Val::Px(2.)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                BorderColor::all(Color::NONE),
                            ))
                            .with_children(|b| {
                                b.spawn((
                                    SideIcon(i as u8),
                                    icon(super::card_view::png(topic.icon_small, &mut images), 18.),
                                ));
                                b.spawn((
                                    NavLabel(action),
                                    text(f, topic.name, 13., col::TEXT_DIM),
                                ));
                            });
                        }
                    });

                // The page: its title, its tabs, its rows and a line of help.
                window
                    .spawn(Node {
                        flex_grow: 1.,
                        min_width: Val::Px(0.),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(20.), Val::Px(16.)),
                        row_gap: Val::Px(12.),
                        ..default()
                    })
                    .with_children(|page| {
                        page.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.),
                            ..default()
                        })
                        .with_children(|title| {
                            title
                                .spawn(Node {
                                    width: Val::Px(34.),
                                    height: Val::Px(34.),
                                    ..default()
                                })
                                .with_children(|seat| {
                                    for (i, topic) in TOPICS.iter().enumerate() {
                                        seat.spawn((
                                            TopicIcon(i as u8),
                                            icon(
                                                super::card_view::png(topic.icon, &mut images),
                                                34.,
                                            ),
                                        ));
                                    }
                                });
                            title
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.),
                                    ..default()
                                })
                                .with_children(|t| {
                                    t.spawn((PageText::Title, text(f, "", 19., col::TEXT_HI)));
                                    t.spawn((PageText::About, text(f, "", 11., col::TEXT_DIM)));
                                });
                        });
                        page.spawn((
                            Node {
                                column_gap: Val::Px(4.),
                                border: UiRect::bottom(Val::Px(1.)),
                                min_height: Val::Px(31.),
                                ..default()
                            },
                            BorderColor::all(EDGE_DIM),
                        ))
                        .with_children(|tabs| {
                            for (t, topic) in TOPICS.iter().enumerate() {
                                for (k, name) in topic.tabs.iter().enumerate() {
                                    let action = Action::Tab(t as u8, k as u8);
                                    tabs.spawn((
                                        Button,
                                        action,
                                        Node {
                                            padding: UiRect::new(
                                                Val::Px(12.),
                                                Val::Px(12.),
                                                Val::Px(6.),
                                                Val::Px(7.),
                                            ),
                                            margin: UiRect::bottom(Val::Px(-1.)),
                                            border: UiRect::bottom(Val::Px(2.)),
                                            display: Display::None,
                                            ..default()
                                        },
                                        BackgroundColor(Color::NONE),
                                        BorderColor::all(Color::NONE),
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            NavLabel(action),
                                            text(f, name, 12., col::TEXT_DIM),
                                        ));
                                    });
                                }
                            }
                        });
                        preview::spawn(page, f);
                        page.spawn(Node {
                            flex_grow: 1.,
                            flex_wrap: FlexWrap::Wrap,
                            align_content: AlignContent::FlexStart,
                            column_gap: Val::Px(10.),
                            row_gap: Val::Px(6.),
                            overflow: Overflow::clip(),
                            ..default()
                        })
                        .with_children(|rows| {
                            for item in Item::all() {
                                rows.spawn((
                                    Row(item),
                                    Node {
                                        width: Val::Percent(48.5),
                                        min_height: Val::Px(34.),
                                        padding: UiRect::axes(Val::Px(10.), Val::Px(4.)),
                                        justify_content: JustifyContent::SpaceBetween,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(10.),
                                        border: UiRect::all(Val::Px(1.)),
                                        display: Display::None,
                                        ..default()
                                    },
                                    BackgroundColor(ROW),
                                    BorderColor::all(EDGE_DIM),
                                    Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                                ))
                                .with_children(|row| {
                                    let (t, k) = item.place();
                                    let topic = &TOPICS[usize::from(t)];
                                    row.spawn(Node {
                                        flex_direction: FlexDirection::Column,
                                        min_width: Val::Px(0.),
                                        flex_shrink: 1.,
                                        ..default()
                                    })
                                    .with_children(|words| {
                                        words.spawn(text(f, item.label(), 12.5, col::TEXT_HI));
                                        words.spawn((
                                            RowWhere,
                                            Node {
                                                display: Display::None,
                                                ..default()
                                            },
                                            text(
                                                f,
                                                &format!(
                                                    "{} · {}",
                                                    topic.name,
                                                    topic.tabs[usize::from(k)]
                                                ),
                                                9.5,
                                                col::TEXT_DIM,
                                            ),
                                        ));
                                    });
                                    match item {
                                        Item::Quality | Item::Value(_) => {
                                            let adjust = |delta| match item {
                                                Item::Value(i) => Action::Adjust(i, delta),
                                                _ => Action::AdjustQuality(delta),
                                            };
                                            row.spawn((
                                                Node {
                                                    height: Val::Px(22.),
                                                    flex_shrink: 0.,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.)),
                                                    ..default()
                                                },
                                                BackgroundColor(WELL),
                                                BorderColor::all(EDGE_DIM),
                                                Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                                            ))
                                            .with_children(|pick| {
                                                arrow(pick, f, "‹", adjust(-1));
                                                // A text node sizes to its words, so a box
                                                // round it centres them.
                                                pick.spawn(Node {
                                                    width: Val::Px(84.),
                                                    justify_content: JustifyContent::Center,
                                                    ..default()
                                                })
                                                .with_child((
                                                    ValueLabel(item),
                                                    text(f, "", 11.5, col::TEXT_HI),
                                                ));
                                                arrow(pick, f, "›", adjust(1));
                                            });
                                        }
                                        Item::Flag(setting) => {
                                            row.spawn((
                                                Button,
                                                Action::Toggle(setting),
                                                Node {
                                                    width: Val::Px(34.),
                                                    height: Val::Px(16.),
                                                    flex_shrink: 0.,
                                                    padding: UiRect::all(Val::Px(1.)),
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.)),
                                                    border_radius: BorderRadius::MAX,
                                                    ..default()
                                                },
                                                BackgroundColor(WELL),
                                                BorderColor::all(EDGE_DIM),
                                                Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
                                            ))
                                            .with_children(|track| {
                                                track.spawn((
                                                    Switch(setting),
                                                    Node {
                                                        width: Val::Px(12.),
                                                        height: Val::Px(12.),
                                                        border_radius: BorderRadius::MAX,
                                                        ..default()
                                                    },
                                                    BackgroundColor(col::STEEL),
                                                ));
                                            });
                                        }
                                    }
                                });
                            }
                            rows.spawn((
                                NoMatch,
                                Node {
                                    display: Display::None,
                                    ..default()
                                },
                                text(f, "No setting matches.", 12., col::TEXT_DIM),
                            ));
                        });
                        page.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: Val::Px(12.),
                            ..default()
                        })
                        .with_children(|foot| {
                            foot.spawn((
                                InterfaceHint,
                                Node {
                                    display: Display::None,
                                    ..default()
                                },
                                text(
                                    f,
                                    "V: nearby enemies · Shift+V: friends · Ctrl+V: both",
                                    11.,
                                    col::TEXT_DIM,
                                ),
                            ));
                            foot.spawn(Node {
                                flex_grow: 1.,
                                ..default()
                            });
                            foot.spawn(text(
                                f,
                                "Changes apply at once and are saved.",
                                11.,
                                col::TEXT_DIM,
                            ));
                        });
                    });
            });
        });
}

/// Type into the search box while it has focus.
fn search_input(
    mut keyboard: MessageReader<KeyboardInput>,
    mut options: ResMut<Options>,
    keys: Res<ButtonInput<KeyCode>>,
    mut clipboard: NonSendMut<crate::textinput::HostClipboard>,
    windows: Query<&bevy::window::RawHandleWrapper, With<bevy::window::PrimaryWindow>>,
) {
    let events: Vec<_> = keyboard.read().cloned().collect();
    if !options.settings || !options.search_active {
        return;
    }
    let control = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let wl = crate::textinput::wayland_display(windows.single().ok());
    let options = &mut *options;
    for event in events
        .into_iter()
        .filter(|e| e.state == ButtonState::Pressed)
    {
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => options.search_active = false,
            KeyCode::Escape => {}
            KeyCode::KeyC | KeyCode::KeyX if control => {
                if !options.cursor.selected(&options.search).is_empty() {
                    clipboard.write(wl, options.cursor.selected(&options.search));
                    if event.key_code == KeyCode::KeyX {
                        options
                            .cursor
                            .key(&mut options.search, KeyCode::Backspace, false, false);
                    }
                }
            }
            KeyCode::KeyV if control => {
                if let Some(value) = clipboard.read(wl) {
                    options.cursor.insert(&mut options.search, &value, 32);
                }
            }
            key => {
                if !options.cursor.key(&mut options.search, key, shift, control) && !control {
                    if let Some(text) = event.text {
                        options.cursor.insert(&mut options.search, &text, 32);
                    }
                }
            }
        }
    }
}

fn clicks(
    mut options: ResMut<Options>,
    phase: Res<State<Phase>>,
    mut cvars: ResMut<crate::cvars::Cvars>,
    mut nameplates: ResMut<super::plates::NameplateSettings>,
    mut commands: Commands,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    msaa: Option<Res<MsaaFormats>>,
) {
    if !options.settings {
        return;
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        options.search_active = *action == Action::Search;
        match *action {
            Action::Search => {
                let end = options.search.len();
                let search = options.search.clone();
                options.cursor.place(&search, end, false);
            }
            Action::Topic(topic) => {
                options.topic = topic;
                options.tab = 0;
                options.search.clear();
            }
            Action::Tab(topic, tab) => {
                options.topic = topic;
                options.tab = tab;
            }
            Action::Back => {
                options.close_settings(*phase.get());
            }
            Action::AdjustQuality(delta) => {
                if !Item::Quality.shown(&options) {
                    continue;
                }
                let index = quality_index(&cvars).map_or(usize::from(delta > 0), |at| {
                    (at as i32 + delta).clamp(0, QUALITY_LEVELS.len() as i32 - 1) as usize
                });
                for (key, values) in QUALITY_SETTINGS {
                    cvars.set(key, values[index]);
                }
                for event in cvars.take_events() {
                    commands.trigger(event);
                }
            }
            Action::Adjust(i, delta) => {
                if !Item::Value(i).shown(&options) {
                    continue;
                }
                let s = &SETTINGS[i];
                let at = selected_index(s, cvars.get(s.key).unwrap_or(unset(s)), msaa.as_deref());
                let mut next = at as i32 + delta;
                let value = loop {
                    let Some(value) = s.values.get(next as usize).copied() else {
                        break None;
                    };
                    if s.key != "gxMultisample"
                        || value
                            .parse::<u32>()
                            .is_ok_and(|n| supports_samples(msaa.as_deref(), n))
                    {
                        break Some(value);
                    }
                    next += delta;
                };
                let Some(value) = value else { continue };
                cvars.set(s.key, value);
                if s.key == "SurvivorUpscaler"
                    && value == "1"
                    && cvars.num("renderScale").unwrap_or(1.0) >= 1.0
                {
                    cvars.set("renderScale", "0.67");
                }
                cvars.commit_latched();
                for event in cvars.take_events() {
                    commands.trigger(event);
                }
            }
            Action::Toggle(setting) => {
                if let Some(key) = setting.cvar() {
                    let value = if !cvars.flag(key).unwrap_or(false) {
                        "1"
                    } else {
                        "0"
                    };
                    cvars.set(key, value);
                    cvars.commit_latched();
                    for event in cvars.take_events() {
                        commands.trigger(event);
                    }
                } else {
                    match setting {
                        InterfaceSetting::EnemyPlates => nameplates.enemies = !nameplates.enemies,
                        InterfaceSetting::FriendlyPlates => {
                            nameplates.friends = !nameplates.friends
                        }
                        InterfaceSetting::CompactPlates => nameplates.compact = !nameplates.compact,
                        InterfaceSetting::HeroPlate => nameplates.hero = !nameplates.hero,
                        _ => unreachable!("CVar-backed interface setting"),
                    }
                    super::plates::persist_nameplate_settings(
                        &nameplates,
                        &mut cvars,
                        &mut commands,
                    );
                }
            }
        }
    }
}

/// The buttons' tones: the open topic and tab in crimson, hover a step lighter, a switch crimson
/// when on with its knob at the right.
#[allow(clippy::type_complexity)]
fn paint_nav(
    options: Res<Options>,
    cvars: Res<crate::cvars::Cvars>,
    nameplates: Res<super::plates::NameplateSettings>,
    mut buttons: Query<(
        &Action,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
    )>,
    mut knobs: Query<(&Switch, &mut BackgroundColor), Without<Action>>,
    mut side_icons: Query<(&SideIcon, &mut ImageNode)>,
) {
    if !options.settings {
        return;
    }
    let searching = !options.search.trim().is_empty();
    let mut lit = [false; TOPICS.len()];
    for (action, interaction, mut bg, mut border, mut node) in &mut buttons {
        let hot = *interaction != Interaction::None;
        if let Action::Topic(t) = *action {
            lit[usize::from(t)] = hot || (t == options.topic && !searching);
        }
        let (fill, edge) = match *action {
            Action::Topic(t) if t == options.topic && !searching => (PICKED, col::WINE_HI),
            Action::Topic(_) => (if hot { HOVER } else { Color::NONE }, Color::NONE),
            Action::Tab(t, k) => {
                let open = t == options.topic && k == options.tab;
                (
                    Color::NONE,
                    if open {
                        col::WINE_HI
                    } else if hot {
                        col::STEEL_LO
                    } else {
                        Color::NONE
                    },
                )
            }
            Action::Toggle(s) => {
                let on = s.enabled(&cvars, &nameplates);
                let want = if on {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                };
                if node.justify_content != want {
                    node.justify_content = want;
                }
                if on {
                    (col::WINE, col::WINE_HI)
                } else {
                    (WELL, if hot { col::STEEL_LO } else { EDGE_DIM })
                }
            }
            Action::Adjust(..) | Action::AdjustQuality(_) => {
                (if hot { PICKED } else { Color::NONE }, Color::NONE)
            }
            Action::Search => (
                WELL,
                if options.search_active || hot {
                    col::STEEL
                } else {
                    EDGE_DIM
                },
            ),
            Action::Back => (
                if hot { HOVER } else { col::KNOB },
                if hot { col::WINE_HI } else { col::STEEL_LO },
            ),
        };
        bg.set_if_neq(BackgroundColor(fill));
        let edge = BorderColor::all(edge);
        if *border != edge {
            *border = edge;
        }
    }
    for (switch, mut bg) in &mut knobs {
        let on = switch.0.enabled(&cvars, &nameplates);
        bg.set_if_neq(BackgroundColor(if on { Color::WHITE } else { col::STEEL }));
    }
    for (side, mut image) in &mut side_icons {
        let tint = Color::WHITE.with_alpha(if lit[usize::from(side.0)] { 1. } else { 0.82 });
        if image.color != tint {
            image.color = tint;
        }
    }
}

/// What the page shows: the open topic's title, tabs and rows, or every row the search matches.
#[allow(clippy::type_complexity)]
fn paint_page(
    options: Res<Options>,
    cvars: Res<crate::cvars::Cvars>,
    msaa: Option<Res<MsaaFormats>>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&Row>,
            Option<&TopicIcon>,
            Option<&Action>,
            Has<Root>,
            Has<RowWhere>,
            Has<InterfaceHint>,
            Has<NoMatch>,
        ),
        Or<(
            With<Root>,
            With<Row>,
            With<TopicIcon>,
            With<Action>,
            With<RowWhere>,
            With<InterfaceHint>,
            With<NoMatch>,
        )>,
    >,
    mut texts: Query<
        (
            &mut Text,
            &mut TextColor,
            Option<&PageText>,
            Option<&ValueLabel>,
            Option<&NavLabel>,
            Has<SearchText>,
        ),
        Or<(
            With<PageText>,
            With<ValueLabel>,
            With<NavLabel>,
            With<SearchText>,
        )>,
    >,
) {
    let query = options.search.trim();
    let searching = !query.is_empty();
    let matches = searching && Item::all().any(|item| item.shown(&options));
    for (mut node, row, topic_icon, action, root, place, hint, none) in &mut nodes {
        let shown = if root {
            options.settings
        } else if let Some(row) = row {
            row.0.shown(&options)
        } else if let Some(icon) = topic_icon {
            !searching && icon.0 == options.topic
        } else if let Some(Action::Tab(t, _)) = action {
            !searching && *t == options.topic
        } else if place {
            searching
        } else if hint {
            !searching && options.topic == 4
        } else if none {
            searching && !matches
        } else {
            continue;
        };
        let want = if shown { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    if !options.settings {
        return;
    }
    let topic = &TOPICS[usize::from(options.topic)];
    for (mut text, mut color, page, value, nav, search) in &mut texts {
        let (want, tone) = if let Some(page) = page {
            match (page, searching) {
                (PageText::Title, false) => (topic.name.to_string(), col::TEXT_HI),
                (PageText::Title, true) => ("Search".to_string(), col::TEXT_HI),
                (PageText::About, false) => (topic.about.to_string(), col::TEXT_DIM),
                (PageText::About, true) => {
                    (format!("Settings matching \"{query}\""), col::TEXT_DIM)
                }
            }
        } else if let Some(value) = value {
            let label = match value.0 {
                Item::Quality => quality_index(&cvars).map_or("Custom", |i| QUALITY_LEVELS[i]),
                Item::Value(i) => {
                    let s = &SETTINGS[i];
                    let at =
                        selected_index(s, cvars.get(s.key).unwrap_or(unset(s)), msaa.as_deref());
                    s.labels[at]
                }
                Item::Flag(_) => unreachable!("toggle rows have no value label"),
            };
            (label.to_string(), col::TEXT_HI)
        } else if let Some(nav) = nav {
            let open = match nav.0 {
                Action::Topic(t) => t == options.topic && !searching,
                Action::Tab(t, k) => t == options.topic && k == options.tab,
                _ => false,
            };
            (
                text.0.clone(),
                if open { col::TEXT_HI } else { col::TEXT_DIM },
            )
        } else if search {
            if options.search_active {
                let at = options.cursor.at.min(options.search.len());
                let (a, b) = options.search.split_at(at);
                (format!("{a}|{b}"), col::TEXT_HI)
            } else if options.search.is_empty() {
                ("Search settings…".to_string(), col::TEXT_DIM)
            } else {
                (options.search.clone(), col::TEXT_HI)
            }
        } else {
            continue;
        };
        if text.0 != want {
            text.0 = want;
        }
        color.set_if_neq(TextColor(tone));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_closes_settings_before_resuming_the_paused_run() {
        let mut o = Options::default();
        let mut next = NextState::default();
        o.escape(Phase::Playing, &mut next);
        assert!(o.open);
        assert!(matches!(next, NextState::Pending(Phase::Paused)));
        o.settings = true;
        next = NextState::default();
        o.escape(Phase::Paused, &mut next);
        assert!(o.open && !o.settings);
        assert!(matches!(next, NextState::Unchanged));
        o.escape(Phase::Paused, &mut next);
        assert!(!o.open);
        assert!(matches!(next, NextState::Pending(Phase::Playing)));
    }
    #[test]
    fn the_settings_window_spawns() {
        // A bundle with a component twice panics when it is applied, at startup.
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(HudFont(Handle::default()));
        world.init_resource::<Assets<Image>>();
        world.run_system_once(spawn).unwrap();
        assert_eq!(
            world.query::<&Row>().iter(&world).count(),
            Item::all().count()
        );
    }

    #[test]
    fn every_topic_icon_is_a_transparent_picture_at_its_size() {
        for topic in TOPICS {
            for (bytes, size) in [(topic.icon_small, 48), (topic.icon, 96)] {
                let image = image::load_from_memory(bytes)
                    .expect("topic icon PNG")
                    .to_rgba8();
                assert_eq!(image.dimensions(), (size, size), "{}", topic.name);
                assert_eq!(image.get_pixel(0, 0).0[3], 0, "{}", topic.name);
                assert!(image.pixels().any(|p| p.0[3] == 255), "{}", topic.name);
            }
        }
    }

    #[test]
    fn escape_clears_a_search_before_closing_the_settings() {
        let mut o = Options {
            open: true,
            settings: true,
            search: "size".into(),
            ..default()
        };
        let mut next = NextState::default();
        o.escape(Phase::Paused, &mut next);
        assert!(o.settings && o.search.is_empty());
        o.escape(Phase::Paused, &mut next);
        assert!(o.open && !o.settings);
    }

    #[test]
    fn every_row_has_a_tab_and_search_finds_rows_across_topics() {
        for item in Item::all() {
            let (topic, tab) = item.place();
            assert!(
                usize::from(tab) < TOPICS[usize::from(topic)].tabs.len(),
                "{item:?}"
            );
        }
        let o = Options {
            search: "SIZE".into(),
            ..default()
        };
        let found: Vec<_> = Item::all()
            .filter(|i| i.shown(&o))
            .map(Item::label)
            .collect();
        assert_eq!(
            found,
            [
                "Minimap size",
                "Run tracker size",
                "Damage meter size",
                "Action bar size",
                "Frame buff size",
                "Frame debuff size",
                "Plate debuff size",
                "Plate buff size",
            ]
        );
        // A topic's name finds its rows too.
        let o = Options {
            search: "nameplates".into(),
            ..default()
        };
        assert!(Item::all().filter(|i| i.shown(&o)).count() >= 7);
    }

    #[test]
    fn opening_menu_during_reward_selection_does_not_discard_the_reward() {
        let mut o = Options::default();
        let mut next = NextState::default();
        o.escape(Phase::LevelUp, &mut next);
        o.escape(Phase::LevelUp, &mut next);
        assert!(!o.open);
        assert!(matches!(next, NextState::Unchanged));
    }
    #[test]
    fn numeric_values_match_saved_cvars_and_controls_stay_in_range() {
        for setting in SETTINGS {
            assert_eq!(setting.values.len(), setting.labels.len());
            for (i, v) in setting.values.iter().enumerate() {
                assert_eq!(nearest(setting, v), i);
            }
        }
        assert_eq!(SETTINGS[2].values[nearest(&SETTINGS[2], "1.00")], "1");
    }

    /// The CVar store refuses writes to a name it does not know, so an unregistered row's arrows
    /// do nothing.
    #[test]
    fn every_row_writes_a_registered_cvar() {
        let known: Vec<&str> = crate::cvars::registered_pairs().map(|(n, _)| n).collect();
        let keys = SETTINGS.iter().map(|s| s.key).chain([
            "SurvivorNameplateEnemies",
            "SurvivorNameplateFriends",
            "SurvivorNameplateCompact",
            "SurvivorHeroPlate",
        ]);
        for key in keys.chain(InterfaceSetting::ALL.iter().filter_map(|f| f.cvar())) {
            assert!(known.contains(&key), "{key} is not registered");
        }
    }

    #[test]
    fn setting_buttons_update_the_live_audio_and_graphics_backends() {
        let mut app = App::new();
        app.init_resource::<crate::cvars::Cvars>()
            .init_resource::<crate::sound::SoundConfig>()
            .insert_resource(crate::world_backdrop::RenderScale(1.))
            .insert_resource(Options {
                open: true,
                settings: true,
                topic: 1,
                ..default()
            })
            .init_resource::<crate::survivors::plates::NameplateSettings>()
            .insert_resource(State::new(Phase::Paused))
            .add_observer(crate::sound::on_cvar)
            .add_observer(crate::world_backdrop::on_cvar)
            .add_systems(Update, clicks);
        let volume = SETTINGS
            .iter()
            .position(|s| s.key == "MasterVolume")
            .unwrap();
        let button = app
            .world_mut()
            .spawn((Interaction::Pressed, Action::Adjust(volume, -1)))
            .id();
        app.update();
        assert!((app.world().resource::<crate::sound::SoundConfig>().master - 0.9).abs() < 0.001);
        assert_eq!(
            app.world()
                .resource::<crate::cvars::Cvars>()
                .get("MasterVolume"),
            Some("0.9")
        );
        app.world_mut().despawn(button);
        app.world_mut().resource_mut::<Options>().topic = 0;
        let render = SETTINGS
            .iter()
            .position(|s| s.key == "renderScale")
            .unwrap();
        app.world_mut()
            .spawn((Interaction::Pressed, Action::Adjust(render, 1)));
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::world_backdrop::RenderScale>()
                .0,
            1.25
        );
    }
}
