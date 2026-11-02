//! The mode's screens, in bevy_ui since FrameXML is off: the class and hero pick, the in-run HUD,
//! the level-up cards, pause, and the results.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};
use bevy::window::PrimaryWindow;

use super::data::{self, AbilityDef, Effect, Kind};
use super::front_end::{self as skin, gap, size};
use super::hero::{self, Hero};
use super::jewelry::{self, JEWELS};
use super::minimap::{self, MiniMap};
use super::rewards::{Rarity, Upgrade};
use super::{Owned, Phase, Rng, Run, Selection};

const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// `ActionButtonTemplate`: 36 units, 6 apart, the first 8 in from the bar's left and 4 up.
const SLOT: f32 = 36.0;
const BUTTON_GAP: f32 = 6.0;
const BUTTON_LEFT: f32 = 8.0;
const BUTTON_BOTTOM: f32 = 4.0;
/// The bar art's height (`MainMenuBarTexture0`..`3`, 256×43).
const BAR_ART_HEIGHT: f32 = 43.0;
/// The cut-down bar: six buttons on the first strip, two recesses, the right edge.
const BAR_WIDTH: f32 = 352.0;
/// The talents button's face.
const TALENTS_ICON: &str = "Interface\\Icons\\INV_Misc_Book_09";
/// `UI-CharacterCreate-Races`, the character button's portrait sheet.
const RACES_SHEET: &str = "Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Races";
/// A run-jewelry icon in the top-left row.
const PASSIVE_SIZE: f32 = 30.0;
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const BUTTON: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const BUTTON_HOT: Color = Color::srgba(0.36, 0.12, 0.06, 0.98);
const DIM: Color = Color::srgb(0.6, 0.6, 0.6);

#[derive(Component)]
pub(super) struct MenuRoot;
#[derive(Component)]
pub(super) struct HudRoot;
#[derive(Component)]
pub(super) struct LevelUpRoot;
#[derive(Component)]
pub(super) struct PauseRoot;
#[derive(Component)]
pub(super) struct OverRoot;
#[derive(Component)]
pub(super) struct XpBar;
#[derive(Component)]
pub(super) struct XpFill;
#[derive(Component)]
pub(super) struct XpText;
#[derive(Component)]
pub(super) struct LevelText;
#[derive(Component)]
pub(super) struct TimerText;
#[derive(Component)]
pub(super) struct KillText;
#[derive(Component)]
pub(super) struct HpFrame;
#[derive(Component)]
pub(super) struct HpBar;
#[derive(Component)]
pub(super) struct HpLevelSeat;
#[derive(Component)]
pub(super) struct HpLevel;
#[derive(Component)]
pub(super) struct HpFill;
#[derive(Component)]
pub(super) struct ShieldFill;
#[derive(Component)]
pub(super) struct BannerText;
#[derive(Component)]
pub(super) struct ActionBar;
/// The character button's portrait, set to the run's race and sex.
#[derive(Component)]
pub(super) struct CharacterIcon;
#[derive(Component)]
pub(super) struct AbilityRow;
#[derive(Component)]
pub(super) struct PassiveRow;
#[derive(Component)]
pub(super) struct OfferRow;
/// The line under LEVEL UP: which level this is.
#[derive(Component)]
pub(super) struct LevelUpSub;

/// A radial cooldown sweep over one ability icon: `sweep` is the remaining fraction
/// (1.0 = just cast, 0.0 = ready), drawn by `ui_sweep.wgsl`.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct SweepUiMaterial {
    #[uniform(0)]
    pub(super) sweep: f32,
}

impl UiMaterial for SweepUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_app/shaders/ui_sweep.wgsl".into()
    }
}

/// The cooldown overlay on one ability slot, driven every frame.
#[derive(Component)]
pub(super) struct AbilitySweep {
    handle: Handle<SweepUiMaterial>,
    index: usize,
}

#[derive(Component)]
pub(super) struct OverTitle;
#[derive(Component)]
pub(super) struct OverStats;
#[derive(Component)]
pub(super) struct OverLine;
#[derive(Component)]
pub(super) struct OverStat(usize);

/// An icon slot filled from the spell's `SpellIcon` once the client data is up.
#[derive(Component)]
pub(super) struct SpellIcon(pub(super) u32);
#[derive(Component)]
pub(super) struct IconDone;

/// A FrameXML `Texture` region: a client art file, its `TexCoords` (left, right, top, bottom) and
/// vertex colour, drawn once the archives are readable.
#[derive(Component)]
pub(super) struct UiArt {
    path: &'static str,
    coords: [f32; 4],
    tint: Color,
}

impl UiArt {
    pub(super) fn crop(path: &'static str, coords: [f32; 4]) -> Self {
        Self {
            path,
            coords,
            tint: Color::WHITE,
        }
    }

    pub(super) fn tinted(path: &'static str, tint: Color) -> Self {
        Self {
            path,
            coords: [0.0, 1.0, 0.0, 1.0],
            tint,
        }
    }
}

#[derive(Component)]
pub(super) struct ArtDone;

/// What an icon slot's tooltip describes: an index into the run's spells or run jewelry.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TipFor {
    Ability(usize),
    Pool(&'static str),
    Amulet,
    Jewelry(usize),
    Character,
    Talents,
}

/// The one `GameTooltip`, and the column its lines go in.
#[derive(Component)]
pub(super) struct GameTooltip;
#[derive(Component)]
pub(super) struct TipBody;

#[derive(Component, Clone, Copy, PartialEq)]
pub(super) enum UiAction {
    Resume,
    Settings,
    MainMenu,
    Quit,
    /// From the results: back through the menu and straight into the same pick.
    Again,
}

/// Set by the results' Play again: the menu starts the next run as soon as it is up.
#[derive(Resource, Default)]
pub(super) struct Replay(bool);

/// The clock's column at the top centre: the clock, the run's line, and the boss bars that
/// [`super::plates::boss_bar`] hangs under them.
#[derive(Component)]
pub(super) struct TopStack;

/// The colour a button returns to when not hovered.
#[derive(Component)]
pub(super) struct BaseColor(Color);

#[derive(Clone, Copy)]
pub(super) enum Choice {
    NewAbility(&'static AbilityDef, Rarity),
    Upgrade(usize, Rarity, [Option<Upgrade>; 2]),
    Jewelry(usize, Rarity),
    Amulet(usize),
    Gold,
}
impl Choice {
    pub fn rarity(self) -> Rarity {
        match self {
            Self::NewAbility(_, r) | Self::Upgrade(_, r, _) | Self::Jewelry(_, r) => r,
            Self::Amulet(i) => super::amulets::AMULETS[i].rarity,
            Self::Gold => Rarity::Common,
        }
    }
}

/// The level-up cards on offer.
#[derive(Resource, Default)]
pub(super) struct Offer {
    choices: Vec<Choice>,
    pub chest: bool,
}

impl Offer {
    pub(super) fn len(&self) -> usize {
        self.choices.len()
    }

    pub(super) fn choices(&self) -> &[Choice] {
        &self.choices
    }
}

#[derive(Resource)]
pub(super) struct HudFont(pub(super) Handle<Font>);

pub(super) fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::BLACK,
        },
    )
}

fn centered(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        text(font, s, size, color),
        TextLayout::new_with_justify(Justify::Center),
    )
}

fn button(action: UiAction, base: Color) -> impl Bundle {
    edged_button(action, base, EDGE)
}

fn edged_button(action: UiAction, base: Color, edge: Color) -> impl Bundle {
    (
        Button,
        action,
        BaseColor(base),
        BackgroundColor(base),
        BorderColor::all(edge),
    )
}

fn icon(spell: u32, size: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
        ImageNode {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            ..default()
        },
        SpellIcon(spell),
    )
}

/// `UI-Quickslot2`, an action button's 64×64 normal texture, 66 units over the 36-unit button.
fn slot_frame() -> impl Bundle {
    let pad = (66.0 - SLOT) / 2.0;
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-pad),
            top: Val::Px(-pad),
            width: Val::Px(66.0),
            height: Val::Px(66.0),
            ..default()
        },
        ImageNode {
            color: Color::NONE,
            ..default()
        },
        UiArt::crop("Interface\\Buttons\\UI-Quickslot2", [0.0, 1.0, 0.0, 1.0]),
    )
}

pub(super) fn spell_name(spells: Option<&crate::ui_action::Spells>, spell: u32, fallback: &str) -> String {
    spells
        .and_then(|s| s.catalog.get(spell))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| fallback.replace('_', " "))
}

pub(super) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut fills: ResMut<Assets<super::fill::FillMaterial>>,
    card_art: Res<super::card_view::CardArt>,
) {
    let font = crate::char_select::wow_font(&asset_server);
    let number_font: Handle<Font> = asset_server.load("mpq://Fonts/ARIALN.ttf");
    commands.insert_resource(HudFont(font.clone()));
    let f = &font;
    let map = MiniMap::new(&mut images);

    // --- The in-run HUD. ---
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(400),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // `MainMenuBar` cut down to what the mode uses: the six spell buttons on the bar art's
            // first strip, the character and talents buttons in the next two recesses, the right
            // edge, and the gryphon end caps over both ends. The XP bar runs along its top.
            root.spawn((
                ActionBar,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(0.0),
                    margin: UiRect::left(Val::Px(-BAR_WIDTH / 2.0)),
                    width: Val::Px(BAR_WIDTH),
                    height: Val::Px(BAR_ART_HEIGHT),
                    ..default()
                },
            ))
            .with_children(|bar| {
                // (left, width, u0, u1, the strip's top v) in `UI-MainMenuBar-Dwarf`.
                const STRIP: f32 = 0.167_968_75;
                for (left, width, u0, u1, top) in [
                    (0.0, 256.0, 0.0, 1.0, 0.832_031_25),
                    (256.0, 84.0, 0.0, 84.0 / 256.0, 0.582_031_25),
                    (340.0, 12.0, 244.0 / 256.0, 1.0, 0.082_031_25),
                ] {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left),
                            bottom: Val::Px(0.0),
                            width: Val::Px(width),
                            height: Val::Px(BAR_ART_HEIGHT),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop(
                            "Interface\\MainMenuBar\\UI-MainMenuBar-Dwarf",
                            [u0, u1, top, top + STRIP],
                        ),
                    ));
                }
                bar.spawn((
                    AbilityRow,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(BUTTON_LEFT),
                        bottom: Val::Px(BUTTON_BOTTOM),
                        column_gap: Val::Px(BUTTON_GAP),
                        ..default()
                    },
                ));
                // The character and talents buttons; the portrait follows the run's race.
                for (i, tip) in [TipFor::Character, TipFor::Talents].into_iter().enumerate() {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(260.0 + i as f32 * (SLOT + BUTTON_GAP)),
                            bottom: Val::Px(BUTTON_BOTTOM),
                            width: Val::Px(SLOT),
                            height: Val::Px(SLOT),
                            ..default()
                        },
                        Interaction::default(),
                        tip,
                    ))
                    .with_children(|slot| {
                        slot.spawn(slot_frame());
                        let face = (
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ImageNode {
                                color: Color::NONE,
                                ..default()
                            },
                        );
                        if tip == TipFor::Character {
                            slot.spawn((face, CharacterIcon));
                        } else {
                            slot.spawn((face, UiArt::crop(TALENTS_ICON, [0.0, 1.0, 0.0, 1.0])));
                        }
                    });
                }
                // `MainMenuBarLeftEndCap` / `RightEndCap`: 128×128, 32 units over the bar's ends,
                // the right one the left mirrored.
                for (left, flip_x) in [(-96.0, false), (BAR_WIDTH - 32.0, true)] {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left),
                            bottom: Val::Px(0.0),
                            width: Val::Px(128.0),
                            height: Val::Px(128.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            flip_x,
                            ..default()
                        },
                        UiArt::crop(
                            "Interface\\MainMenuBar\\UI-MainMenuBar-EndCap-Human",
                            [0.0, 1.0, 0.0, 1.0],
                        ),
                    ));
                }
            });
            // `MainMenuExpBar` along the action bar's top edge, its numbers always shown.
            root.spawn((
                XpBar,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(BAR_ART_HEIGHT - 3.0),
                    margin: UiRect::left(Val::Px(-BAR_WIDTH / 2.0)),
                    width: Val::Px(BAR_WIDTH),
                    height: Val::Px(13.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    XpFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::tinted(
                        "Interface\\TargetingFrame\\UI-StatusBar",
                        Color::srgb(0.58, 0.0, 0.55),
                    ),
                ));
                // `MainMenuXPBarTexture0`..`3`: the segment rules, a quarter of the bar each.
                for (i, top) in [0.792_968_75, 0.542_968_75, 0.292_968_75, 0.042_968_75]
                    .into_iter()
                    .enumerate()
                {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(25.0 * i as f32),
                            top: Val::Px(0.0),
                            width: Val::Percent(25.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop(
                            "Interface\\MainMenuBar\\UI-MainMenuBar-Dwarf",
                            [0.0, 1.0, top, top + 0.039_062_5],
                        ),
                    ));
                }
                // `MainMenuBarExpText`, centred one unit up.
                bar.spawn(Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    top: Val::Px(-1.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|n| {
                    n.spawn((
                        XpText,
                        Text::new(""),
                        TextFont {
                            font: number_font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::BLACK,
                        },
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    ));
                });
            });
            minimap::spawn_cluster(root, f, &map);
            root.spawn(Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|row| {
                // The jewelry, small along the top left; the spells are on the action bar.
                row.spawn(Node {
                    width: Val::Px(300.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        PassiveRow,
                        Node {
                            column_gap: Val::Px(4.0),
                            margin: UiRect::top(Val::Px(96.0)),
                            ..default()
                        },
                    ));
                });
                // The clock, with the level and the run's counts on one line under it, and the
                // boss bars under that, all one column so none can run into the next.
                row.spawn((
                    TopStack,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                ))
                .with_children(|col| {
                    col.spawn((TimerText, centered(f, "00:00", 34.0, Color::WHITE)));
                    col.spawn(Node {
                        column_gap: Val::Px(8.0),
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    })
                    .with_children(|line| {
                        line.spawn((LevelText, text(f, "Level 1", 14.0, GOLD)));
                        line.spawn((KillText, text(f, "", 14.0, Color::WHITE)));
                    });
                });
                // The minimap cluster's column.
                row.spawn(Node {
                    width: Val::Px(300.0),
                    ..default()
                });
            });
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(22.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|n| {
                n.spawn((BannerText, centered(f, "", 30.0, GOLD)));
            });
            // The hero's health as a nameplate (`CGNamePlateFrame`): the bar under the border that
            // caps it, the level in the border's seat; [`hero_plate`] sizes and seats it.
            let absolute = || Node {
                position_type: PositionType::Absolute,
                ..default()
            };
            let unpainted = || ImageNode {
                color: Color::NONE,
                ..default()
            };
            let [r, g, b, _] = crate::vplates::PLATE_FRIENDLY;
            root.spawn((HpFrame, absolute(), Visibility::Hidden))
                .with_children(|plate| {
                    plate.spawn((HpBar, absolute())).with_children(|bar| {
                        bar.spawn((
                            HpFill,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            unpainted(),
                            UiArt::tinted(nameplate::BAR_FILL_TEXTURE, Color::srgb(r, g, b)),
                        ));
                        bar.spawn((
                            ShieldFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..absolute()
                            },
                            unpainted(),
                            UiArt::tinted(
                                nameplate::BAR_FILL_TEXTURE,
                                Color::srgba(0.85, 0.9, 1.0, 0.75),
                            ),
                        ));
                    });
                    plate.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..absolute()
                        },
                        unpainted(),
                        UiArt::crop(nameplate::BORDER_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
                    ));
                    let [r, g, b, _] = crate::vplates::CON_YELLOW;
                    plate
                        .spawn((
                            HpLevelSeat,
                            Node {
                                width: Val::Px(0.0),
                                height: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..absolute()
                            },
                        ))
                        .with_children(|seat| {
                            seat.spawn((
                                HpLevel,
                                Text::new("1"),
                                TextFont {
                                    font: f.clone(),
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(r, g, b)),
                                TextShadow {
                                    offset: Vec2::new(1.0, 1.0),
                                    color: Color::BLACK,
                                },
                                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                            ));
                        });
                });
        });
    commands.insert_resource(map);

    // --- The run's pick screen; [`super::menu`] builds into it once the glue art is in. ---
    commands.spawn((
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(410),
        bevy::ui::FocusPolicy::Pass,
    ));

    // --- Level up. ---
    commands
        .spawn((
            LevelUpRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            // Dims the world, the plates and the meters under the cards.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(420),
            Visibility::Hidden,
            // The build's icons stay hoverable under the cards.
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn(super::card_view::text(
                &card_art.title,
                "CHOOSE YOUR REWARD",
                32.0,
                super::card_view::TEXT,
            ));
            root.spawn((
                LevelUpSub,
                super::card_view::text(
                    &card_art.body,
                    "Choose your power",
                    18.0,
                    super::card_view::MUTED,
                ),
            ));
            root.spawn((
                OfferRow,
                Node {
                    column_gap: Val::Px(32.0 * super::card_view::GAME_SCALE),
                    // Room for a hovered card's lift.
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(28.0), Val::Px(16.0)),
                    ..default()
                },
            ));
            root.spawn(super::card_view::text(
                &card_art.body,
                "Click a card, or press 1, 2 or 3",
                18.0,
                super::card_view::MUTED,
            ));
            super::cards::spawn_controls(root, &card_art);
        });

    // --- Pause. ---
    commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(430),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn((
                centered(f, "Paused", size::DISPLAY, skin::GOLD),
                Node {
                    margin: UiRect::bottom(Val::Px(gap::S)),
                    ..default()
                },
            ));
            for (label, action) in [
                ("Resume", UiAction::Resume),
                ("Settings", UiAction::Settings),
                ("Abandon run", UiAction::MainMenu),
                ("Quit game", UiAction::Quit),
            ] {
                root.spawn((
                    button(action, BUTTON),
                    Node {
                        width: Val::Px(240.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn(text(f, label, size::LEAD + 2.0, skin::TEXT));
                });
            }
        });

    // --- Results. ---
    let results_w = 640.0;
    commands
        .spawn((
            OverRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(gap::M),
                ..default()
            },
            MaterialNode(fills.add(super::fill::FillMaterial::radial(
                Color::srgba(0.07, 0.02, 0.0, 0.88),
                Color::srgba(0.0, 0.0, 0.0, 0.97),
            ))),
            GlobalZIndex(440),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((OverTitle, centered(f, "", size::HERO, skin::GOLD)));
            root.spawn((OverLine, centered(f, "", size::LEAD, skin::TEXT)));
            root.spawn((OverStats, centered(f, "", size::SMALL, skin::MUTED)));
            // The run in numbers: equal tiles across the column's width.
            root.spawn(Node {
                width: Val::Px(results_w),
                column_gap: Val::Px(gap::S),
                margin: UiRect::top(Val::Px(gap::S)),
                ..default()
            })
            .with_children(|row| {
                for (i, label) in ["SURVIVED", "LEVEL", "KILLS", "DAMAGE", "BOSSES"]
                    .into_iter()
                    .enumerate()
                {
                    row.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(gap::S), Val::Px(gap::M)),
                            row_gap: Val::Px(gap::XS),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundGradient::from(LinearGradient::to_bottom(vec![
                            ColorStop::percent(Color::srgba(0.2, 0.14, 0.05, 0.92), 0.0),
                            ColorStop::percent(Color::srgba(0.04, 0.03, 0.02, 0.94), 100.0),
                        ])),
                        BorderColor::all(skin::EDGE),
                    ))
                    .with_children(|tile| {
                        tile.spawn((OverStat(i), centered(f, "", size::TITLE, skin::TEXT)));
                        tile.spawn(centered(f, label, size::CAPTION, skin::MUTED));
                    });
                }
            });
            // What the run paid the class: the XP, the bar toward the next point, the level.
            root.spawn(skin::panel(Node {
                width: Val::Px(results_w),
                padding: UiRect::axes(Val::Px(gap::L), Val::Px(gap::M)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gap::S),
                ..default()
            }))
            .with_children(|panel| {
                skin::heading(panel, f, "REWARDS");
                let headline = skin::label(panel, f, "", size::LEAD, skin::GOLD);
                panel
                    .commands()
                    .entity(headline)
                    .insert(super::talents::ResultsProgress);
                let fill = skin::progress_bar(panel, 0.0, skin::GOLD.darker(0.15), 8.0);
                panel
                    .commands()
                    .entity(fill)
                    .insert(super::talents::ResultsXpFill);
                let level = skin::label(panel, f, "", size::SMALL, skin::MUTED);
                panel
                    .commands()
                    .entity(level)
                    .insert(super::talents::ResultsLevel);
                skin::heading(panel, f, "QUESTS");
                panel.spawn((
                    super::quests::ResultsQuests,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(gap::XS),
                        ..default()
                    },
                ));
            });
            root.spawn(Node {
                column_gap: Val::Px(gap::M),
                margin: UiRect::top(Val::Px(gap::S)),
                ..default()
            })
            .with_children(|row| {
                for (title, action, primary) in [
                    ("Play again", UiAction::Again, true),
                    ("Main menu", UiAction::MainMenu, false),
                ] {
                    let base = if primary { BUTTON } else { skin::SURFACE };
                    let mut b = row.spawn((
                        edged_button(action, base, if primary { skin::GOLD } else { skin::EDGE }),
                        Node {
                            width: Val::Px(220.0),
                            justify_content: JustifyContent::Center,
                            padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                            border: UiRect::all(Val::Px(if primary { 2.0 } else { 1.0 })),
                            border_radius: BorderRadius::all(Val::Px(5.0)),
                            ..default()
                        },
                    ));
                    if primary {
                        b.insert(BoxShadow::new(
                            Color::srgba(1.0, 0.7, 0.2, 0.35),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(16.0),
                        ));
                    }
                    b.with_children(|b| {
                        b.spawn(text(
                            f,
                            title,
                            if primary { size::HEADING } else { size::LEAD },
                            if primary { skin::GOLD } else { skin::TEXT },
                        ));
                    });
                }
            });
            root.spawn(Node {
                column_gap: Val::Px(gap::XL),
                ..default()
            })
            .with_children(|row| {
                skin::keycap(row, f, "ENTER", "Main menu");
            });
        });
}

/// The menu's hero is the run's hero: respawned whenever the pick changes.
pub(super) fn menu_preview(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    heroes: Query<Entity, With<Hero>>,
    world: super::dungeon::Ground,
    mut shown: Local<Option<(usize, u32, usize)>>,
) {
    if *phase.get() != Phase::Menu {
        return;
    }
    let want = selection
        .class
        .zip(selection.look.and_then(|i| selection.looks.get(i)).copied());
    let key = want.map(|(c, l)| (c, l.0, selection.stage));
    if key == *shown && (key.is_none() || !heroes.is_empty()) {
        return;
    }
    for e in &heroes {
        commands.entity(e).despawn();
    }
    *shown = key;
    let Some((class, (display, race, sex))) = want else {
        return;
    };
    let mut pos = hero::arena(selection.stage_def());
    if let Some(h) = world.height(pos) {
        pos.y = h;
    }
    let serial = run.guid();
    hero::spawn_hero(
        &mut commands,
        &data::CLASSES[class],
        display,
        race,
        sex,
        serial,
        pos,
    );
}

pub(super) fn start_run(run: &mut Run, selection: &mut Selection) -> bool {
    if !data::MAPS.get(selection.map).is_some_and(super::dungeon::available) {
        return false;
    }
    let (Some(class), Some(&(_, race, _))) = (
        selection.class_def(),
        selection.look.and_then(|i| selection.looks.get(i)),
    ) else {
        return false;
    };
    if !selection.race_unlocked(race) || !selection.can_visit(selection.map) {
        return false;
    }
    // The world goes to the battleground picked; until now it stood in the last one.
    selection.stage = selection.map;
    let next_guid = run.next_guid;
    *run = Run {
        class,
        race,
        next_guid,
        ..Run::default()
    };
    run.abilities
        .push(Owned::new(data::ability(class.start), Rarity::Common));
    run.hp = run.stats().max_hp;
    run.version += 1;
    true
}

fn choices_for(run: &Run, rng: &mut Rng, chest: bool) -> Vec<Choice> {
    let mut pool = Vec::new();
    if chest {
        for (i, d) in JEWELS.iter().enumerate() {
            let owned = run.jewelry.iter().find(|j| j.index == i);
            let slots = run
                .jewelry
                .iter()
                .filter(|j| JEWELS[j.index].ring == d.ring)
                .count();
            if owned.is_none() && slots >= 2 {
                continue;
            }
            let rarity = Rarity::roll(rng.f32(), run.luck());
            let gain = owned.map_or_else(
                || super::rewards::jewel_gain(d.stat, d.base, rarity, None),
                |j| j.gain(rarity),
            );
            if gain > 0.000001 {
                pool.push(Choice::Jewelry(i, rarity));
            }
        }
        let rarity = Rarity::roll(rng.f32(), run.luck());
        for (i, a) in super::amulets::AMULETS.iter().enumerate() {
            if a.rarity == rarity {
                pool.push(Choice::Amulet(i));
            }
        }
    } else {
        for (i, o) in run.abilities.iter().enumerate() {
            let rarity = Rarity::roll(rng.f32(), run.luck());
            let mut axes = super::reward_axes::axes(o.def.key).to_vec();
            axes.retain(|a| match a {
                Upgrade::Targets => o.progress.targets < 12,
                Upgrade::Area => o.progress.area < 4.999,
                Upgrade::Duration => o.progress.duration < 5.999,
                Upgrade::Cooldown => o.progress.cooldown > 0.20001,
                _ => true,
            });
            if axes.is_empty() {
                continue;
            }
            let first = axes.swap_remove(rng.index(axes.len()));
            let second = if rarity == Rarity::Legendary && !axes.is_empty() {
                Some(axes.swap_remove(rng.index(axes.len())))
            } else {
                None
            };
            pool.push(Choice::Upgrade(i, rarity, [Some(first), second]));
        }
        if run.abilities.len() < data::MAX_ABILITIES {
            for key in run.class.pool {
                if !run.abilities.iter().any(|o| o.def.key == *key) {
                    pool.push(Choice::NewAbility(
                        data::ability(key),
                        Rarity::roll(rng.f32(), run.luck()),
                    ));
                }
            }
        }
    }
    pool.retain(|choice| {
        !run.hand
            .banned
            .contains(&super::cards::reward_key(*choice, run))
    });
    let mut out = Vec::new();
    while out.len() < 3 && !pool.is_empty() {
        out.push(pool.swap_remove(rng.index(pool.len())));
    }
    if out.is_empty() {
        out.push(Choice::Gold);
    }
    out
}
pub(super) fn roll_offer(run: Res<Run>, mut rng: ResMut<Rng>, mut offer: ResMut<Offer>) {
    offer.chest = run.pending_chests > 0;
    offer.choices = choices_for(&run, &mut rng, offer.chest);
}
pub(super) fn redeal(run: &Run, rng: &mut Rng, offer: &mut Offer) {
    offer.choices = choices_for(run, rng, offer.chest);
}

pub(super) fn skip_offer(
    run: &mut Run,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
) {
    run.gold = run.gold.saturating_add(super::cards::skip_gold(run));
    run.version += 1;
    finish_offer(run, offer, rng, next);
}
pub(super) fn pick(
    i: usize,
    run: &mut Run,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
) {
    let Some(&choice) = offer.choices.get(i) else {
        return;
    };
    match choice {
        Choice::NewAbility(def, r) => run.abilities.push(Owned::new(def, r)),
        Choice::Upgrade(k, r, axes) => {
            if let Some(o) = run.abilities.get_mut(k) {
                o.upgrade(r, axes);
            }
        }
        Choice::Jewelry(index, r) => {
            if let Some(j) = run.jewelry.iter_mut().find(|j| j.index == index) {
                j.strength += j.gain(r);
                j.picks += 1;
                j.rarity = r;
            } else {
                let d = &JEWELS[index];
                run.jewelry.push(jewelry::Jewel {
                    index,
                    strength: super::rewards::jewel_gain(d.stat, d.base, r, None),
                    picks: 1,
                    rarity: r,
                });
            }
        }
        Choice::Amulet(index) => {
            if let Some(a) = run.amulet.as_mut().filter(|a| a.index == index) {
                a.rank = a.rank.saturating_add(1);
            } else {
                run.amulet = Some(super::amulets::Amulet { index, rank: 1 });
            }
        }
        Choice::Gold => run.gold = run.gold.saturating_add(25),
    }
    run.version += 1;
    finish_offer(run, offer, rng, next);
}

fn finish_offer(run: &mut Run, offer: &mut Offer, rng: &mut Rng, next: &mut NextState<Phase>) {
    if let Some(chest) = super::rewards::consume_pending(
        offer.chest,
        &mut run.pending_levels,
        &mut run.pending_chests,
    ) {
        offer.chest = chest;
        offer.choices = choices_for(run, rng, chest);
    } else {
        offer.choices.clear();
        next.set(Phase::Playing);
    }
}

pub(super) fn looks_for(
    class: &data::ClassDef,
    creatures: &crate::entities::Creatures,
) -> Vec<(u32, u8, u8)> {
    super::races::looks_for(class, creatures)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn buttons(
    mut presses: Query<
        (&Interaction, &UiAction, &BaseColor, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next: ResMut<NextState<Phase>>,
    mut exit: MessageWriter<AppExit>,
    phase: Res<State<Phase>>,
    mut options: ResMut<super::options::Options>,
    mut replay: ResMut<Replay>,
) {
    for (interaction, action, base, mut bg) in &mut presses {
        match interaction {
            Interaction::Hovered => bg.0 = BUTTON_HOT,
            Interaction::None => bg.0 = base.0,
            Interaction::Pressed => {
                bg.0 = BUTTON_HOT;
                match (*action, *phase.get()) {
                    (UiAction::Resume, _) if options.open || *phase.get() == Phase::Paused => {
                        options.resume(*phase.get(), &mut next)
                    }
                    (UiAction::Settings, _) if options.open || *phase.get() == Phase::Paused => {
                        options.settings = true
                    }
                    (UiAction::MainMenu, _)
                        if (*phase.get() != Phase::Menu && options.open)
                            || matches!(*phase.get(), Phase::Paused | Phase::GameOver) =>
                    {
                        options.open = false;
                        options.settings = false;
                        next.set(Phase::Menu);
                    }
                    (UiAction::Again, Phase::GameOver) if !options.open => {
                        replay.0 = true;
                        next.set(Phase::Menu);
                    }
                    (UiAction::Quit, _)
                        if options.open
                            || matches!(*phase.get(), Phase::Paused | Phase::GameOver) =>
                    {
                        exit.write(AppExit::Success);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(super) fn keys(
    inventory: Res<super::inventory::Equipment>,
    atlas: Res<super::adventure_atlas::Atlas>,
    coop: Res<super::coop::Coop>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut options: ResMut<super::options::Options>,
    mut nameplates: ResMut<super::plates::NameplateSettings>,
    mut cvars: ResMut<crate::cvars::Cvars>,
    mut commands: Commands,
    mut replay: ResMut<Replay>,
) {
    // Play again: the menu is up, so start the same pick as Enter would. A co-op room starts
    // its runs from the host, so there it is only the way back to the menu.
    if *phase.get() == Phase::Menu && std::mem::take(&mut replay.0) {
        if !coop.in_room() && selection.unlocked(selection.map) && start_run(&mut run, &mut selection) {
            next.set(Phase::Playing);
        }
        return;
    }
    if inventory.open || atlas.open || coop.open || (*phase.get() == Phase::Menu && coop.in_room()) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        options.escape(*phase.get(), &mut next);
        return;
    }
    if options.open {
        return;
    }
    if *phase.get() == Phase::Playing && keys.just_pressed(KeyCode::KeyV) {
        let control = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        super::plates::toggle_nameplates(&mut nameplates, control, shift);
        super::plates::persist_nameplate_settings(&nameplates, &mut cvars, &mut commands);
    }
    match phase.get() {
        Phase::Menu => {
            if keys.just_pressed(KeyCode::Enter)
                && selection.unlocked(selection.map)
                && start_run(&mut run, &mut selection)
            {
                next.set(Phase::Playing);
            }
        }
        Phase::Playing | Phase::Paused => {}
        Phase::LevelUp => {}
        Phase::GameOver => {
            if keys.just_pressed(KeyCode::Enter) {
                next.set(Phase::Menu);
            }
        }
    }
}

/// The class's looks once the creature data is in, defaulting to the first. The auto-runner picks
/// a class with no look; the menu itself always sets both.
pub(super) fn refresh_menu(
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
) {
    if let (Some(class), Some(creatures)) = (selection.class_def(), creatures.as_deref()) {
        if selection.looks.is_empty() {
            let looks = looks_for(class, creatures);
            if !looks.is_empty() {
                selection.looks = looks;
                selection.look = Some(0);
                selection.version += 1;
            }
        }
    }
}

/// What a level-up card shows.
pub(super) struct CardText {
    pub spell: u32,
    pub icon: Option<&'static str>,
    pub name: String,
    /// The pre-ornate badge; the ornate card derives its state and footer itself.
    #[allow(dead_code)]
    pub tag: String,
    pub desc: String,
}

pub(super) fn describe_card(
    choice: Choice,
    run: &Run,
    spells: Option<&crate::ui_action::Spells>,
) -> CardText {
    match choice {
        Choice::NewAbility(def, r) => {
            let o = Owned::new(def, r);
            let c = cast_run(&o, run);
            CardText {
                spell: def.spell,
                icon: None,
                name: spell_name(spells, def.spell, def.key),
                tag: "LEARN SPELL".into(),
                desc: format!(
                    "{}\n\nCooldown {} sec",
                    ability_body(def, &c),
                    num(c.cooldown)
                ),
            }
        }
        Choice::Upgrade(k, r, axes) => {
            let o = &run.abilities[k];
            let mut next = o.clone();
            next.upgrade(r, axes);
            let (a, b) = (cast_run(o, run), cast_run(&next, run));
            let mut lines = Vec::new();
            let what = match o.def.kind {
                Kind::Shield => "Absorb",
                Kind::Heal => "Healing",
                _ => "Power",
            };
            if b.dmg > a.dmg + 0.001 {
                lines.push(format!("{what}   {} > {}", num(a.dmg), num(b.dmg)));
            }
            if b.dot > a.dot + 0.001 {
                lines.push(format!("Damage/sec   {} > {}", num(a.dot), num(b.dot)));
            }
            if b.cooldown < a.cooldown - 0.001 {
                lines.push(format!(
                    "Cooldown   {} > {} sec",
                    num(a.cooldown),
                    num(b.cooldown)
                ));
            }
            if b.count > a.count {
                lines.push(format!("Targets   {} > {}", a.count, b.count));
            }
            if b.radius > a.radius + 0.001 {
                lines.push(format!("Radius   {} > {} yd", num(a.radius), num(b.radius)));
            }
            if b.duration > a.duration + 0.001 {
                lines.push(format!(
                    "Duration   {} > {} sec",
                    num(a.duration),
                    num(b.duration)
                ));
            }
            CardText {
                spell: o.def.spell,
                icon: None,
                name: spell_name(spells, o.def.spell, o.def.key),
                tag: format!("RANK {} > {}", o.level, next.level),
                desc: lines.join("\n\n"),
            }
        }
        Choice::Jewelry(i, r) => {
            let d = &JEWELS[i];
            let old = run.jewelry.iter().find(|j| j.index == i);
            let current = old.map_or(0., |j| j.strength);
            let gain = old.map_or_else(
                || super::rewards::jewel_gain(d.stat, d.base, r, None),
                |j| j.gain(r),
            );
            CardText {
                spell: 0,
                icon: Some(d.icon),
                name: d.name.into(),
                tag: format!(
                    "{} {}",
                    if old.is_some() { "UPGRADE" } else { "NEW" },
                    if d.ring { "RING" } else { "TRINKET" }
                ),
                desc: format!(
                    "{}\n\nApplies for this run",
                    jewelry::delta(i, current, current + gain)
                ),
            }
        }
        Choice::Amulet(i) => {
            let d = &super::amulets::AMULETS[i];
            let old = run.amulet.as_ref().filter(|a| a.index == i);
            let a = super::amulets::Amulet {
                index: i,
                rank: old.map_or(1, |a| a.rank + 1),
            };
            let mut lines: Vec<String> = d
                .stats
                .iter()
                .map(|(stat, v)| {
                    format!(
                        "{}   +{}",
                        jewelry::label(*stat),
                        jewelry::amount(*stat, *v * a.scale())
                    )
                })
                .collect();
            if d.targets > 0 {
                lines.push(format!("All spells   +{}", skin::plural(d.targets, "target")));
            }
            CardText {
                spell: 0,
                icon: Some(d.icon),
                name: d.name.into(),
                tag: if old.is_some() {
                    "EMPOWER AMULET"
                } else if run.amulet.is_some() {
                    "REPLACE AMULET"
                } else {
                    "NEW AMULET"
                }
                .into(),
                desc: lines.join("\n\n"),
            }
        }
        Choice::Gold => CardText {
            spell: 0,
            icon: Some("Interface\\Icons\\INV_Misc_Coin_02"),
            name: "A Pouch of Gold".into(),
            tag: "RUN REWARD".into(),
            desc: "+25 gold\n\nNo eligible upgrades remain in this reward pool.".into(),
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_hud(
    mut commands: Commands,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    font: Res<HudFont>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LevelText>>,
        Query<&mut Text, With<TimerText>>,
        Query<&mut Text, With<KillText>>,
        Query<&mut Text, With<BannerText>>,
        Query<&mut Text, With<OverTitle>>,
        Query<&mut Text, With<OverStats>>,
        Query<&mut Text, With<XpText>>,
    )>,
    mut xp_fill: Query<&mut Node, With<XpFill>>,
    rows: Query<(Entity, Has<AbilityRow>), Or<(With<AbilityRow>, With<PassiveRow>)>>,
    portrait: Query<Entity, With<CharacterIcon>>,
    mut sweeps: ResMut<Assets<SweepUiMaterial>>,
    mut last_version: Local<Option<u32>>,
) {
    let need = data::xp_to_next(run.level);
    if let Ok(mut n) = xp_fill.single_mut() {
        n.width = Val::Percent((run.xp / need * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut t) = texts.p6().single_mut() {
        let xp = format!("{} / {}", run.xp as u32, need as u32);
        if **t != xp {
            **t = xp;
        }
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        **t = format!("Level {}", run.level);
    }
    let secs = run.elapsed as u32;
    if let Ok(mut t) = texts.p1().single_mut() {
        **t = format!("{:02}:{:02}", secs / 60, secs % 60);
    }
    let kills = run.kills;
    if let Ok(mut t) = texts.p2().single_mut() {
        **t = format!(
            "·   Kills {kills}   ·   Gold {}   ·   Cache {}",
            run.gold, run.chest_cost
        );
    }
    let banner = match run.banner.as_mut() {
        Some((msg, t)) => {
            *t -= time.delta_secs();
            if *t > 0.0 {
                msg.clone()
            } else {
                String::new()
            }
        }
        None => String::new(),
    };
    if banner.is_empty() {
        run.banner = None;
    }
    // The picker owns the screen: an announcement waits until it closes.
    let banner = if *phase.get() == Phase::LevelUp {
        String::new()
    } else {
        banner
    };
    if let Ok(mut t) = texts.p3().single_mut() {
        if **t != banner {
            **t = banner;
        }
    }
    let title = if run.won {
        "Victory!"
    } else {
        "You have fallen"
    }
    .to_string();
    if let Ok(mut t) = texts.p4().single_mut() {
        **t = title;
    }
    let summary = format!(
        "{} {}  ·  {}",
        super::races::race_name(run.race),
        run.class.name,
        selection.map_def().name,
    );
    if let Ok(mut t) = texts.p5().single_mut() {
        if **t != summary {
            **t = summary;
        }
    }

    // The spell and jewelry icons, rebuilt when the build changes.
    if *last_version == Some(run.version) {
        return;
    }
    *last_version = Some(run.version);
    // The character button's portrait: this run's race and sex.
    let sex = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(0, |l| l.2);
    if let Some(tc) = super::races::race_tc(run.race, sex) {
        for e in &portrait {
            commands
                .entity(e)
                .remove::<ArtDone>()
                .insert(UiArt::crop(RACES_SHEET, tc));
        }
    }
    let f = &font.0;
    for (row, is_abilities) in &rows {
        commands.entity(row).despawn_related::<Children>();
        commands.entity(row).with_children(|row| {
            // Spells fill the bar's six buttons, empty ones left bare; jewelry sits small and
            // bordered at the top left, only those learned.
            let (entries, empties, size): (Vec<(u32, u8, TipFor)>, usize, f32) = if is_abilities {
                let e = run.abilities.iter().enumerate();
                let e: Vec<_> = e
                    .map(|(i, o)| (o.def.spell, o.level, TipFor::Ability(i)))
                    .collect();
                let empties = data::MAX_ABILITIES.saturating_sub(e.len());
                (e, empties, SLOT)
            } else {
                let e = run.jewelry.iter().enumerate();
                (
                    e.map(|(i, j)| (0, j.picks.min(255) as u8, TipFor::Jewelry(i)))
                        .chain(
                            run.amulet
                                .as_ref()
                                .map(|a| (0, a.rank.min(255) as u8, TipFor::Amulet)),
                        )
                        .collect(),
                    0,
                    PASSIVE_SIZE,
                )
            };
            for (spell, level, tip) in entries {
                let ability = match &tip {
                    TipFor::Ability(i) => Some(*i),
                    _ => None,
                };
                let edge = if ability.is_some() { 0.0 } else { 1.0 };
                row.spawn((
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        border: UiRect::all(Val::Px(edge)),
                        ..default()
                    },
                    BorderColor::all(EDGE),
                    Interaction::default(),
                    tip,
                ))
                .with_children(|slot| {
                    if ability.is_some() {
                        slot.spawn(slot_frame());
                    }
                    if let TipFor::Jewelry(i) = tip {
                        slot.spawn((
                            Node {
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                ..default()
                            },
                            ImageNode::default(),
                            UiArt::crop(JEWELS[run.jewelry[i].index].icon, [0., 1., 0., 1.]),
                        ));
                    } else if let TipFor::Amulet = tip {
                        if let Some(a) = &run.amulet {
                            slot.spawn((
                                Node {
                                    width: Val::Percent(100.),
                                    height: Val::Percent(100.),
                                    ..default()
                                },
                                ImageNode::default(),
                                UiArt::crop(
                                    super::amulets::AMULETS[a.index].icon,
                                    [0., 1., 0., 1.],
                                ),
                            ));
                        }
                    } else {
                        slot.spawn(icon(spell, size - 2.0 * edge));
                    }
                    // The cooldown sweep sits over the icon, under the level text.
                    if let Some(i) = ability {
                        let handle = sweeps.add(SweepUiMaterial { sweep: 0.0 });
                        slot.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            MaterialNode(handle.clone()),
                            AbilitySweep { handle, index: i },
                        ));
                    }
                    slot.spawn(Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(2.0),
                        bottom: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|n| {
                        n.spawn(text(f, level.to_string(), 12.0, Color::WHITE));
                    });
                });
            }
            // Every button shows, the empty ones as the bar's bare slots, so it never shifts.
            for _ in 0..empties {
                row.spawn(Node {
                    width: Val::Px(SLOT),
                    height: Val::Px(SLOT),
                    ..default()
                });
            }
        });
    }
}

/// Fill icon slots from `SpellIcon.dbc` once the client data and the asset reader are up.
pub(super) fn fill_icons(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &SpellIcon), Without<IconDone>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<HashMap<u32, Handle<Image>>>,
) {
    let (Some(spells), Some(assets)) = (spells.as_deref(), assets.as_deref_mut()) else {
        return;
    };
    for (e, mut img, icon) in &mut slots {
        let handle = match cache.get(&icon.0) {
            Some(h) => Some(h.clone()),
            None => {
                let path = spells
                    .catalog
                    .get(icon.0)
                    .and_then(|d| d.icon.clone())
                    .unwrap_or_else(|| "Interface\\Icons\\INV_Misc_QuestionMark".into());
                let h = assets.sprite_texture(&format!("{path}.blp"), &mut images);
                if let Some(h) = &h {
                    cache.insert(icon.0, h.clone());
                }
                h
            }
        };
        if let Some(h) = handle {
            img.image = h;
            img.color = Color::WHITE;
        }
        commands.entity(e).insert(IconDone);
    }
}

/// Drive each ability icon's cooldown sweep from its remaining cooldown.
pub(super) fn update_cooldown_sweeps(
    run: Res<Run>,
    mut mats: ResMut<Assets<SweepUiMaterial>>,
    sweeps: Query<&AbilitySweep>,
) {
    let stats = run.stats();
    for s in &sweeps {
        let Some(o) = run.abilities.get(s.index) else {
            continue;
        };
        let total = o.cooldown(&stats).max(0.01);
        let f = (o.cd / total).clamp(0.0, 1.0);
        if let Some(m) = mats.get_mut(&s.handle) {
            m.sweep = f;
        }
    }
}

/// Draw each [`UiArt`] region once its file can be read.
pub(super) fn fill_art(
    mut commands: Commands,
    mut regions: Query<(Entity, &mut ImageNode, &UiArt), Without<ArtDone>>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut assets) = assets else { return };
    for (e, mut img, art) in &mut regions {
        commands.entity(e).insert(ArtDone);
        let Some(h) = assets.sprite_texture(art.path, &mut images) else {
            warn!("survivors: no UI art {}", art.path);
            continue;
        };
        let [l, r, t, b] = art.coords;
        if let Some(size) = images.get(&h).map(Image::size_f32) {
            img.rect = Some(Rect::new(l * size.x, t * size.y, r * size.x, b * size.y));
        }
        img.image = h;
        img.color = art.tint;
    }
}

/// The reference lays its UI out on a screen 768 units tall at any resolution (`useUiScale` off).
pub(super) fn apply_ui_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else { return };
    let want = window.height() / 768.0;
    if want > 0.0 && (scale.0 - want).abs() > 1e-4 {
        scale.0 = want;
    }
}

/// Between the hero's feet and the top of its plate's bar, in gx units.
const PLATE_FEET_GAP: f32 = 0.004;
/// A couple of screen pixels lower than the gap alone sets it.
const PLATE_NUDGE: f32 = 2.0;

/// The hero's nameplate under its feet, sized from the screen diagonal as [`crate::vplates`]
/// sizes a plate, outside the UI scale. The border's top half seats a name the hero has none of,
/// so the plate rises until its bar sits just under the feet.
#[allow(clippy::type_complexity)]
pub(super) fn hero_plate(
    run: Res<Run>,
    phase: Res<State<Phase>>,
    ui_scale: Res<UiScale>,
    hero: Query<&Transform, With<Hero>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut nodes: ParamSet<(
        Query<(&mut Node, &mut Visibility), With<HpFrame>>,
        Query<&mut Node, With<HpBar>>,
        Query<&mut Node, With<HpFill>>,
        Query<&mut Node, With<ShieldFill>>,
        Query<&mut Node, With<HpLevelSeat>>,
    )>,
    mut level: Query<(&mut Text, &mut TextFont), With<HpLevel>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    let seat = hero.single().ok().and_then(|h| {
        let (cam, tf) = cams.single().ok()?;
        let viewport = cam.logical_viewport_size()?;
        let feet = cam
            .world_to_viewport(&GlobalTransform::from(*tf), h.translation)
            .ok()?;
        Some((feet, viewport))
    });
    let mut frames = nodes.p0();
    let Ok((mut frame, mut vis)) = frames.single_mut() else {
        return;
    };
    // Under the cards and the pause menu the plate would sit among their buttons.
    let Some((feet, viewport)) = seat.filter(|_| *phase.get() == Phase::Playing) else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Inherited;
    let s = ui_scale.0;
    let basis = plate_basis(viewport);
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = (gx(PLATE_W), gx(PLATE_H));
    let bar_top = ph - gx(BAR_OFF_Y) - gx(BAR_H);
    frame.width = Val::Px(pw);
    frame.height = Val::Px(ph);
    frame.left = Val::Px(feet.x / s - pw / 2.0);
    frame.top = Val::Px(feet.y / s + gx(PLATE_FEET_GAP) - bar_top + PLATE_NUDGE / s);
    if let Ok(mut bar) = nodes.p1().single_mut() {
        bar.left = Val::Px(gx(BAR_OFF_X));
        bar.top = Val::Px(bar_top);
        bar.width = Val::Px(gx(BAR_W));
        bar.height = Val::Px(gx(BAR_H));
    }
    let stats = run.stats();
    let hp = (run.hp / stats.max_hp).clamp(0.0, 1.0);
    let shield = if run.shield_t > 0.0 {
        (run.shield / stats.max_hp).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if let Ok(mut n) = nodes.p2().single_mut() {
        n.width = Val::Percent(hp * 100.0);
    }
    if let Ok(mut n) = nodes.p3().single_mut() {
        n.width = Val::Percent(shield * 100.0);
    }
    if let Ok(mut seat) = nodes.p4().single_mut() {
        seat.left = Val::Px(pw - gx(LEVEL_OFF_X));
        seat.top = Val::Px(ph - gx(LEVEL_OFF_Y));
    }
    if let Ok((mut text, mut font)) = level.single_mut() {
        let n = run.level.to_string();
        if text.0 != n {
            text.0 = n;
        }
        let size = text_px(LEVEL_H, basis) / s;
        if font.font_size != size {
            font.font_size = size;
        }
    }
}

// ── The build icons' `GameTooltip` ──────────────────────────────────────────────────────────────

/// `TOOLTIP_DEFAULT_BACKGROUND_COLOR`, the `GameTooltip` backdrop tint (`GameTooltip.lua`).
const TIP_FILL: Color = Color::srgb(0.09, 0.09, 0.19);
/// The grey a spell tooltip prints its rank in, right of the name.
const TIP_GREY: Color = Color::srgb(0.5, 0.5, 0.5);

/// One tooltip line: the left text, an optional right-aligned one, the colour and font size
/// (`GameTooltipHeaderText` 14, `GameTooltipText` 12).
struct TipLine {
    left: String,
    right: Option<(String, Color)>,
    color: Color,
    size: f32,
}

impl TipLine {
    fn new(left: impl Into<String>, color: Color) -> Self {
        Self {
            left: left.into(),
            right: None,
            color,
            size: 12.0,
        }
    }

    fn right(mut self, text: impl Into<String>, color: Color) -> Self {
        self.right = Some((text.into(), color));
        self
    }

    fn header(mut self) -> Self {
        self.size = 14.0;
        self
    }
}

/// The results screen's tiles and its line under the title.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_results(
    run: Res<Run>,
    selection: Res<Selection>,
    mut over_texts: ParamSet<(
        Query<(&OverStat, &mut Text)>,
        Query<&mut Text, With<OverLine>>,
    )>,
) {
    let secs = run.elapsed as u32;
    let stats_line = if run.won {
        selection.map_def().victory
    } else {
        "The horde was too much this time."
    };
    let values = [
        format!("{:02}:{:02}", secs / 60, secs % 60),
        run.level.to_string(),
        run.kills.to_string(),
        compact(run.damage_done),
        // Killed, not met: one name per boss down.
        run.boss_loot.len().to_string(),
    ];
    for (stat, mut t) in &mut over_texts.p0() {
        if let Some(v) = values.get(stat.0) {
            if **t != *v {
                **t = v.clone();
            }
        }
    }
    if let Ok(mut t) = over_texts.p1().single_mut() {
        if **t != stats_line {
            **t = stats_line.to_string();
        }
    }
}

/// 12.3k, 4.5M.
pub(super) fn compact(v: f64) -> String {
    if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e4 {
        format!("{:.1}k", v / 1e3)
    } else {
        format!("{}", v.round() as u64)
    }
}

/// Whole from 100 up ("1480", not "1480.2"), else one decimal unless it is whole.
fn num(v: f32) -> String {
    if v.abs() >= 100.0 || (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn pct(v: f32) -> String {
    format!("{}%", num(v * 100.0))
}

/// The numbers an ability casts with at a level, as [`super::abilities::cast_abilities`] takes
/// them from [`Owned`].
struct Cast {
    dmg: f32,
    dot: f32,
    count: u32,
    radius: f32,
    duration: f32,
    cooldown: f32,
}

fn cast_run(o: &Owned, run: &Run) -> Cast {
    let p = super::abilities::parameters(o, run);
    Cast {
        dmg: p.dmg,
        dot: p.dot,
        count: p.count as u32,
        radius: p.radius,
        duration: p.duration,
        cooldown: p.cooldown,
    }
}

fn effect_text(effect: Effect) -> Option<String> {
    Some(match effect {
        Effect::None => return None,
        Effect::Slow(m, t) => format!("slowing them by {} for {} sec", pct(1.0 - m), num(t)),
        Effect::Root(t) => format!("freezing them in place for {} sec", num(t)),
        Effect::Stun(t) => format!("stunning them for {} sec", num(t)),
        Effect::Fear(t) => format!("sending them fleeing for {} sec", num(t)),
        Effect::Weaken(m, t) => format!(
            "cutting their damage by {} for {} sec",
            pct(1.0 - m),
            num(t)
        ),
        Effect::Drain(f) => format!("healing you for {} of the damage", pct(f)),
        Effect::Heal(f) => format!("healing you for {} of maximum health per enemy hit", pct(f)),
        Effect::Shield(f) => format!(
            "shielding you for {} of maximum health per enemy hit",
            pct(f)
        ),
    })
}

/// Reward cards describe the same current cast used by combat.
pub(super) fn ability_description(owned: &Owned, run: &Run) -> String {
    ability_body(owned.def, &cast_run(owned, run))
}

/// What an ability does at these numbers, in the voice of a spell description.
fn ability_body(def: &AbilityDef, c: &Cast) -> String {
    let d = num(c.dmg);
    let burst = def.duration <= 0.0;
    let mut s = match def.kind {
        Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Execute => {
            let melee = def.kind == Kind::Melee;
            let who = match (melee, c.count) {
                (true, 1) => "the closest enemy".into(),
                (true, n) => format!("the {n} closest enemies"),
                (false, 1) => "the nearest enemy".into(),
                (false, n) => format!("the {n} nearest enemies"),
            };
            let mut s = format!("{} {who} for {d} damage", if melee { "Strikes" } else { "Hits" });
            if c.radius > 0.0 && !melee {
                s += &format!(", and everything within {} yd of the hit", num(c.radius));
            }
            s += ". Bosses and elites are targeted first";
            if def.kind == Kind::Execute { s += "; damage doubles below 30% health"; }
            s
        }
        Kind::Missiles => format!("Fires {} for {d} damage each at random enemies within {} yd. Bosses and elites take priority; missiles can hit the same enemy repeatedly", skin::plural(c.count, "missile"), num(def.range)),
        Kind::Buff => format!("Increases {} by {} for {} sec when enemies are within 30 yd", if def.self_effect == 9 { "cooldown recovery" } else { "damage" }, pct(c.dmg), num(c.duration)),
        Kind::Passive => super::spell_passives::description(def.key).into(),
        Kind::Nova => format!("Deals {d} damage to every enemy within {} yd of you", num(c.radius)),
        Kind::Cone => format!("Deals {d} damage to every enemy in a {} yd cone toward the closest", num(c.radius)),
        Kind::Ground | Kind::GroundSelf => {
            let at = if def.kind == Kind::GroundSelf { "around you" } else { "on the thickest pack" };
            if burst {
                format!("Strikes a {} yd area {at} for {d} damage", num(c.radius))
            } else {
                format!(
                    "Covers a {} yd area {at} for {} sec, dealing {d} total damage in {} ticks",
                    num(c.radius),
                    num(c.duration),
                    def.ticks.max(1)
                )
            }
        }
        Kind::Chain => format!(
            "Strikes an enemy for {d} damage, prioritizing bosses and elites, then jumps to {} more within {} yd of each other, 8% weaker with every jump",
            c.count,
            num(c.radius)
        ),
        Kind::Dot => format!(
            "Afflicts {} in range with {} damage per second for {} sec. Bosses and elites are targeted first; avoids reapplying the same spell",
            skin::plural(c.count, "enemy"),
            num(c.dot),
            num(c.duration)
        ),
        Kind::Aura => format!(
            "Every {} sec, deals {d} damage to every enemy within {} yd of you",
            num(c.cooldown),
            num(c.radius)
        ),
        Kind::Shield => format!("Absorbs the next {d} damage you take. Cast again once the shield breaks"),
        Kind::Immune => format!(
            "Below 35% health, makes you immune to all damage for {} sec",
            num(c.duration)
        ),
        Kind::Thorns => format!(
            "When enemies close in, deals {d} damage to every attacker for {} sec",
            num(c.duration)
        ),
        Kind::Heal if c.duration <= 0.0 => format!("Below 85% health, instantly heals you for {d}"),
        Kind::Heal => format!("Below 85% health, heals you for {d} over {} sec", num(c.duration)),
        Kind::Totem if def.key == "magma_totem" => format!(
            "Drops a totem for {} sec that burns every enemy within {} yd for {d} damage every 2 sec",
            num(c.duration),
            num(c.radius)
        ),
        Kind::Totem => format!(
            "Drops a totem for {} sec that hits the nearest enemy within {} yd for {d} damage every second",
            num(c.duration),
            num(def.range)
        ),
    };
    // Areas that tick carry no over-time part; a burst leaves a 5 second one.
    let ground = matches!(def.kind, Kind::Ground | Kind::GroundSelf);
    if c.dot > 0.0 && def.kind != Kind::Dot && (!ground || burst) {
        let secs = if ground { 5.0 } else { c.duration };
        s += &format!(
            ", then {} damage every second for {} sec",
            num(c.dot),
            num(secs)
        );
    }
    if let Some(e) = effect_text(def.effect) {
        s = s.trim_end_matches('.').to_string();
        s += ", ";
        s += &e;
    }
    // A passive's own text may already end its sentence: never "range..".
    if !s.ends_with('.') {
        s.push('.');
    }
    s
}

fn ability_tip(o: &Owned, run: &Run, spells: Option<&crate::ui_action::Spells>) -> Vec<TipLine> {
    let def = o.def;
    let stats = run.stats();
    let now = cast_run(o, run);
    let mut lines = vec![
        TipLine::new(spell_name(spells, def.spell, def.key), Color::WHITE)
            .header()
            .right(format!("Rank {}", o.level), TIP_GREY),
    ];
    let range = match def.kind {
        Kind::Melee => Some("Melee Range".to_string()),
        Kind::Bolt | Kind::Strike | Kind::Chain | Kind::Dot | Kind::Ground | Kind::Totem => {
            Some(format!("{} yd range", num(def.range)))
        }
        _ => None,
    };
    let mut cast = TipLine::new("Casts itself", Color::WHITE);
    if let Some(range) = range {
        cast = cast.right(range, Color::WHITE);
    }
    lines.push(cast);
    lines.push(
        TipLine::new("Instant", Color::WHITE)
            .right(format!("{} sec cooldown", num(now.cooldown)), Color::WHITE),
    );
    lines.push(TipLine::new(ability_body(def, &now), GOLD));
    if !matches!(
        def.kind,
        Kind::Shield | Kind::Immune | Kind::Heal | Kind::Dot
    ) {
        lines.push(TipLine::new(
            format!(
                "{} chance to critically strike for double damage",
                pct(stats.crit)
            ),
            DIM,
        ));
    }
    lines.push(TipLine::new(
        "Level-up cards roll individual stat bonuses by rarity.",
        DIM,
    ));
    lines
}

/// The `GameTooltip` frame: `UI-Tooltip-Background` tiled under `UI-Tooltip-Border`, 16-unit edges
/// and 5-unit insets, its lines 10 units in. Returns the frame and its line column.
fn spawn_tooltip(
    commands: &mut Commands,
    assets: &mut benilla_assets::WorldAssets,
    images: &mut Assets<Image>,
    ui_scale: f32,
) -> (Entity, Entity) {
    use crate::glue::backdrop::{backdrop_border, backdrop_edges, tiled_bg_node};
    let bg = assets.sprite_texture("Interface\\Tooltips\\UI-Tooltip-Background", images);
    let edges = backdrop_edges(assets, "Interface\\Tooltips\\UI-Tooltip-Border", images);
    let mut body = Entity::PLACEHOLDER;
    let mut root = commands.spawn((
        GameTooltip,
        Node {
            position_type: PositionType::Absolute,
            max_width: Val::Px(300.0),
            ..default()
        },
        GlobalZIndex(1000),
        Visibility::Hidden,
        bevy::ui::FocusPolicy::Pass,
    ));
    if bg.is_none() || edges.is_none() {
        root.insert(BackgroundColor(TIP_FILL.with_alpha(0.95)));
    }
    root.with_children(|t| {
        if let Some(bg) = bg {
            t.spawn((
                tiled_bg_node(bg, 16.0, ui_scale, TIP_FILL),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(5.0),
                    right: Val::Px(5.0),
                    top: Val::Px(5.0),
                    bottom: Val::Px(5.0),
                    ..default()
                },
            ));
        }
        if let Some(edges) = &edges {
            backdrop_border(t, edges, 16.0, Color::WHITE);
        }
        body = t
            .spawn((
                TipBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
            ))
            .id();
    });
    (root.id(), body)
}

/// Hovering a build icon raises its `GameTooltip` under the icon, every number from the current
/// rank and stats, and what the next rank brings.
#[allow(clippy::too_many_arguments)]
pub(super) fn icon_tooltip(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    windows: Query<&Window, With<PrimaryWindow>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    font: Res<HudFont>,
    ui_scale: Res<UiScale>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    slots: Query<(
        &Interaction,
        &TipFor,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut frames: Query<(&mut Node, &mut Visibility, &ComputedNode), With<GameTooltip>>,
    mut built: Local<Option<(Entity, Entity)>>,
    mut shown: Local<Option<(TipFor, u32, u32, bool)>>,
) {
    let Some((frame, body)) = *built else {
        if let Some(assets) = assets.as_deref_mut() {
            *built = Some(spawn_tooltip(
                &mut commands,
                assets,
                &mut images,
                ui_scale.0,
            ));
        }
        return;
    };
    let Ok((mut node, mut vis, frame_size)) = frames.get_mut(frame) else {
        return;
    };
    let hovered = slots
        .iter()
        .find(|(i, tip, ..)| {
            **i != Interaction::None
                && (matches!(tip, TipFor::Pool(_)) == (*phase.get() == Phase::Menu))
        });
    let Some((_, &tip, computed, at)) = hovered else {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
        *shown = None;
        return;
    };
    // Physical pixels to the 768-unit canvas the `Val`s are on.
    let inv = computed.inverse_scale_factor;
    let mut x = (at.translation.x - computed.size().x / 2.0) * inv;
    let mut y = (at.translation.y + computed.size().y / 2.0) * inv + 4.0;
    if let Ok(window) = windows.single() {
        let viewport = Vec2::new(window.physical_width() as f32, window.physical_height() as f32) * inv;
        let size = frame_size.size() * inv;
        x = x.clamp(4.0, (viewport.x - size.x - 4.0).max(4.0));
        if y + size.y > viewport.y - 4.0 {
            y = (at.translation.y - computed.size().y / 2.0) * inv - size.y - 4.0;
        }
        y = y.max(4.0);
    }
    let (left, top) = (Val::Px(x), Val::Px(y));
    if node.left != left || node.top != top {
        node.left = left;
        node.top = top;
    }
    let content_key = (tip, run.version, selection.version, spells.is_some());
    if *shown == Some(content_key) {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        return;
    }
    let lines = match tip {
        TipFor::Ability(i) => run
            .abilities
            .get(i)
            .map(|o| ability_tip(o, &run, spells.as_deref())),
        TipFor::Pool(key) => selection.class_def().map(|class| {
            let preview = Run { class, ..Run::default() };
            let mut lines = ability_tip(&Owned::new(data::ability(key), Rarity::Common), &preview, spells.as_deref());
            lines.push(TipLine::new("Base values before racial bonuses, talents and equipment.", DIM));
            lines
        }),
        TipFor::Jewelry(i) => run.jewelry.get(i).map(|j| {
            let d = &JEWELS[j.index];
            let mut lines = vec![TipLine::new(d.name, Color::WHITE).header()];
            for (stat, v) in jewelry::effects(j.index, j.strength) {
                if stat != 255 {
                    lines.push(TipLine::new(
                        format!("{}: {}", jewelry::label(stat), jewelry::amount(stat, v)),
                        GOLD,
                    ));
                }
            }
            lines
        }),
        TipFor::Amulet => run.amulet.as_ref().map(|a| {
            let d = &super::amulets::AMULETS[a.index];
            let mut lines = vec![TipLine::new(d.name, Color::WHITE).header()];
            for &(stat, v) in d.stats {
                lines.push(TipLine::new(
                    format!(
                        "{}: {}",
                        jewelry::label(stat),
                        jewelry::amount(stat, v * a.scale())
                    ),
                    GOLD,
                ));
            }
            if d.targets > 0 {
                lines.push(TipLine::new(format!("+{} spell target", d.targets), GOLD));
            }
            lines
        }),
        TipFor::Character => Some(vec![
            TipLine::new("Character", Color::WHITE).header(),
            TipLine::new(
                format!(
                    "Level {} {} {}",
                    run.level,
                    super::races::race_name(run.race),
                    run.class.name
                ),
                Color::WHITE,
            ),
            TipLine::new("Equipment is managed from Inventory on the main menu.", DIM),
        ]),
        TipFor::Talents => Some(vec![
            TipLine::new("Talents", Color::WHITE).header(),
            TipLine::new("Spend class points from Talents on the main menu.", DIM),
        ]),
    };
    let Some(lines) = lines else {
        *vis = Visibility::Hidden;
        *shown = None;
        return;
    };
    *shown = Some(content_key);
    // Shown from the next frame, once the new lines have laid out.
    *vis = Visibility::Hidden;
    let f = &font.0;
    commands.entity(body).despawn_related::<Children>();
    commands.entity(body).with_children(|b| {
        for line in lines {
            let face = |s: String, size: f32, color: Color| {
                (
                    Text::new(s),
                    TextFont {
                        font: f.clone(),
                        font_size: size,
                        ..default()
                    },
                    TextColor(color),
                    TextShadow {
                        offset: Vec2::splat(1.0),
                        color: Color::BLACK,
                    },
                )
            };
            b.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(face(line.left, line.size, line.color));
                if let Some((text, color)) = line.right {
                    row.spawn((
                        face(text, line.size, color),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    ));
                }
            });
        }
    });
}

#[allow(clippy::type_complexity)]
pub(super) fn show_panels(
    phase: Res<State<Phase>>,
    coop: Res<super::coop::Coop>,
    inventory: Res<super::inventory::Equipment>,
    atlas: Res<super::adventure_atlas::Atlas>,
    options: Res<super::options::Options>,
    mut roots: ParamSet<(
        Query<(&mut Visibility, &mut Node), With<MenuRoot>>,
        Query<(&mut Visibility, &mut Node), With<HudRoot>>,
        Query<(&mut Visibility, &mut Node), With<LevelUpRoot>>,
        Query<(&mut Visibility, &mut Node), With<PauseRoot>>,
        Query<(&mut Visibility, &mut Node), With<OverRoot>>,
    )>,
) {
    let p = *phase.get();
    // A hidden panel leaves the layout too: a merely invisible button still takes clicks.
    let set = |(mut v, mut node): (Mut<Visibility>, Mut<Node>), on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    for root in &mut roots.p0() {
        // Inventory and the auction house draw under the menu, so it steps aside for them.
        set(
            root,
            p == Phase::Menu && !options.open && !coop.open && !inventory.open && !atlas.open,
        );
    }
    for root in &mut roots.p1() {
        set(root, p != Phase::Menu && p != Phase::GameOver);
    }
    for root in &mut roots.p2() {
        set(root, p == Phase::LevelUp && !options.open);
    }
    for root in &mut roots.p3() {
        set(
            root,
            p != Phase::Menu && (p == Phase::Paused || options.open) && !options.settings,
        );
    }
    for root in &mut roots.p4() {
        set(root, p == Phase::GameOver && !options.open);
    }
}
