//! Nameplates for major encounters and optional V-key plates for enemies in view, drawn like the
//! client plates ([`crate::vplates`]): a hostile bar, name and level over each unit's head.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::enemies::Enemy;
use super::hud::{HudFont, UiArt};
use super::unit_frames::{AuraLayout, AuraPos};
use super::{Phase, RunEntity};
use crate::entities::{mount::MountChild, overhead_anchor, BoneAttach, OverheadFallback};

/// The client's lift over the head anchor (`[0x80abfc]`).
const LIFT: f32 = 2.0 / 3.0;
/// The name's height in gx units, as the client's plate prints it.
const NAME_H: f32 = 0.01;
/// The client's plate reach (`vplates`' 20 yards), squared, for the horde's optional plates.
const NEARBY_SQ: f32 = 20.0 * 20.0;
/// At most this many ordinary plates at once, nearest first.
const MAX_NEARBY: usize = 24;
/// Bosses get first choice; off-screen encounters do not retain UI trees.
const MAX_ENCOUNTER_PLATES: usize = 8;

/// The settings page's equivalent of the two FrameXML nameplate toggles. The Survivors world does
/// not load FrameXML, so these are persisted alongside the client's CVars.
#[derive(Resource, Default)]
pub(super) struct NameplateSettings {
    pub enemies: bool,
    pub friends: bool,
    /// The compact style: a thin flat bar with the name above, in place of the client's art.
    pub compact: bool,
    /// The hero's own health plate under their feet.
    pub hero: bool,
    /// Where the compact plates' buffs and debuffs go, and their sizes: the Auras tab's rows.
    pub auras: AuraLayout,
}

/// Follow the Auras tab: a new layout rebuilds the compact plates in [`enemy_plates`].
pub(super) fn sync_plate_auras(
    cvars: Res<crate::cvars::Cvars>,
    mut settings: ResMut<NameplateSettings>,
) {
    if !cvars.is_changed() {
        return;
    }
    let auras = AuraLayout::plates(&cvars);
    if settings.auras != auras {
        settings.auras = auras;
    }
}

pub(super) fn load_nameplate_settings(
    cvars: Res<crate::cvars::Cvars>,
    mut settings: ResMut<NameplateSettings>,
) {
    settings.enemies = cvars.flag("SurvivorNameplateEnemies").unwrap_or(false);
    settings.friends = cvars.flag("SurvivorNameplateFriends").unwrap_or(true);
    settings.compact = cvars.flag("SurvivorNameplateCompact").unwrap_or(true);
    settings.hero = cvars.flag("SurvivorHeroPlate").unwrap_or(true);
}

pub(super) fn toggle_nameplates(settings: &mut NameplateSettings, control: bool, shift: bool) {
    if control {
        let show_both = !(settings.enemies && settings.friends);
        settings.enemies = show_both;
        settings.friends = show_both;
    } else if shift {
        // As in the client, V and Shift+V each flip only their own side.
        settings.friends = !settings.friends;
    } else {
        settings.enemies = !settings.enemies;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v_and_shift_v_toggle_their_own_side_and_ctrl_v_sets_both() {
        let mut s = NameplateSettings {
            enemies: false,
            friends: true,
            compact: true,
            hero: true,
            auras: AuraLayout::default(),
        };
        toggle_nameplates(&mut s, false, false);
        assert!(s.enemies && s.friends, "V leaves ally plates alone");
        toggle_nameplates(&mut s, false, true);
        assert!(s.enemies && !s.friends, "Shift+V leaves enemy plates alone");
        toggle_nameplates(&mut s, true, false);
        assert!(
            s.enemies && s.friends,
            "Ctrl+V shows both when either is off"
        );
        toggle_nameplates(&mut s, true, false);
        assert!(
            !s.enemies && !s.friends,
            "Ctrl+V hides both when both are on"
        );
    }

    #[test]
    fn the_aura_layout_places_the_rows_and_splits_their_room() {
        // Unset, the plate is what it always was: one 9px row over the bar, 22px in all.
        let shared = AuraLayout::default();
        let g = CompactGeometry::of(shared);
        assert_eq!((g.above, g.bar_top, g.height), (9.0, 11.0, 22.0));
        assert_eq!(rooms(shared, 2, 1), (9, 1));
        // Debuffs under the bar and bigger buffs over it: each row its own, as many as fit.
        let split = AuraLayout {
            debuffs: AuraPos::Below,
            buff_size: 13.0,
            ..shared
        };
        let g = CompactGeometry::of(split);
        assert_eq!(
            (g.above, g.below, g.bar_top, g.height),
            (13.0, 9.0, 15.0, 37.0)
        );
        assert_eq!(rooms(split, 2, 1), (10, 7));
        // A hidden kind takes no room and no row.
        let hidden = AuraLayout {
            buffs: AuraPos::Hidden,
            ..shared
        };
        assert_eq!(rooms(hidden, 2, 5), (10, 0));
        assert_eq!(CompactGeometry::of(hidden).height, 22.0);
    }

    #[test]
    fn a_plates_row_gives_buffs_three_slots_and_counts_what_does_not_fit() {
        // Debuffs keep the row unless the buffs need their three.
        assert_eq!(share(0, 0, 10), (10, 0));
        assert_eq!(share(2, 1, 10), (9, 1));
        assert_eq!(share(7, 8, 10), (7, 3));
        assert_eq!(share(1, 8, 10), (2, 8));
        assert_eq!(share(12, 0, 10), (10, 0));
        // The last slot of a full side becomes its "+n".
        assert_eq!(overflow(7, 7), (7, 0));
        assert_eq!(overflow(8, 3), (2, 6));
        assert_eq!(overflow(12, 10), (9, 3));
    }

    #[test]
    fn every_aura_slot_size_spawns() {
        // A bundle with a component twice panics when it is applied.
        let mut world = World::new();
        let f = Handle::<Font>::default();
        for size in [COMPACT_AURA, 14.0, AURA] {
            let slot = spawn_aura_slot(&mut world.commands(), &f, size);
            world.flush();
            assert!(world.get_entity(slot.frame).is_ok());
        }
    }

    #[test]
    fn bar_glyphs_cover_their_shapes_and_leave_the_eyes_open() {
        assert!(glyph_covers(BarGlyph::Skull, 12.0, 8.0), "cranium");
        assert!(glyph_covers(BarGlyph::Skull, 12.0, 17.5), "jaw");
        assert!(!glyph_covers(BarGlyph::Skull, 9.8, 11.2), "eye");
        assert!(!glyph_covers(BarGlyph::Skull, 2.0, 2.0), "corner");
        assert!(glyph_covers(BarGlyph::Star, 12.0, 12.0), "star centre");
        assert!(glyph_covers(BarGlyph::Star, 12.0, 6.0), "top point");
        assert!(
            !glyph_covers(BarGlyph::Star, 12.0, 16.5),
            "between the lower points"
        );
        let img = glyph_image(BarGlyph::Star);
        let alpha = |x: usize, y: usize| img.data.as_ref().unwrap()[(y * 64 + x) * 4 + 3];
        assert_eq!(alpha(32, 32), 255);
        assert_eq!(alpha(0, 0), 0);
    }
}

pub(super) fn persist_nameplate_settings(
    settings: &NameplateSettings,
    cvars: &mut crate::cvars::Cvars,
    commands: &mut Commands,
) {
    cvars.set(
        "SurvivorNameplateEnemies",
        if settings.enemies { "1" } else { "0" },
    );
    cvars.set(
        "SurvivorNameplateFriends",
        if settings.friends { "1" } else { "0" },
    );
    cvars.set(
        "SurvivorNameplateCompact",
        if settings.compact { "1" } else { "0" },
    );
    cvars.set("SurvivorHeroPlate", if settings.hero { "1" } else { "0" });
    cvars.commit_latched();
    for event in cvars.take_events() {
        commands.trigger(event);
    }
}

/// One plate's root, and the parts it moves and fills.
#[derive(Component)]
pub(super) struct Plate {
    bar: Entity,
    fill: Entity,
    name: Entity,
    name_seat: Entity,
    level: Entity,
    level_seat: Entity,
}

/// A compact plate's health text, its name's clip box and its aura row: your debuffs from the
/// left, the unit's buffs from the right, each side's "+n" for what does not fit. A side's icons
/// are spawned the first time the plate needs them and retained when the plate is recycled.
#[derive(Component)]
pub(super) struct CompactParts {
    health: Entity,
    name_box: Entity,
    rows: [Entity; 2],
    debuffs: Vec<AuraSlot>,
    buffs: Vec<AuraSlot>,
    more: [(Entity, Entity); 2],
    /// The health and format the text was last written for.
    shown: Option<(f32, f32, super::unit_frames::HealthText)>,
    auras_dirty: bool,
}

fn absolute() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

fn unpainted() -> ImageNode {
    ImageNode {
        color: Color::NONE,
        ..default()
    }
}

fn label(f: &Handle<Font>, s: String, color: Color) -> impl Bundle {
    sized_label(f, s, color, 11.0)
}

fn sized_label(f: &Handle<Font>, s: String, color: Color, size: f32) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(1.0, 1.0),
            color: Color::BLACK,
        },
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
    )
}

/// The compact plate, in UI pixels: every plate the same width; an 11px bar with the level, name
/// and health in it, and rows of icons over and under it as the Auras tab places them.
const COMPACT_W: f32 = 100.0;
const COMPACT_BAR_H: f32 = 11.0;
const COMPACT_AURA: f32 = 9.0;
const COMPACT_AURA_GAP: f32 = 1.0;
/// Between the bar and a row of icons.
const COMPACT_ROW_GAP: f32 = 2.0;

/// A compact plate's rows in a layout: the height over the bar, the bar's top, the plate's
/// height, and each kind's row height on each edge.
#[derive(Clone, Copy, PartialEq, Debug)]
struct CompactGeometry {
    above: f32,
    below: f32,
    bar_top: f32,
    height: f32,
}

impl CompactGeometry {
    fn of(layout: AuraLayout) -> Self {
        let edge = |at: AuraPos| {
            [
                (layout.debuffs, layout.debuff_size),
                (layout.buffs, layout.buff_size),
            ]
            .into_iter()
            .filter(|(pos, _)| *pos == at)
            .map(|(_, size)| size)
            .fold(0.0, f32::max)
        };
        let (above, below) = (edge(AuraPos::Above), edge(AuraPos::Below));
        let bar_top = if above > 0.0 {
            above + COMPACT_ROW_GAP
        } else {
            0.0
        };
        let under = if below > 0.0 {
            COMPACT_ROW_GAP + below
        } else {
            0.0
        };
        Self {
            above,
            below,
            bar_top,
            height: bar_top + COMPACT_BAR_H + under,
        }
    }
}

/// How many icons of `size` fit across a plate.
fn row_fit(size: f32) -> usize {
    (((COMPACT_W + COMPACT_AURA_GAP) / (size + COMPACT_AURA_GAP)).floor() as usize).max(1)
}

/// Each side's slots in a layout, debuffs then buffs: a shared row splits by [`share`], a row of
/// its own takes what fits, a hidden kind none.
fn rooms(layout: AuraLayout, debuffs: usize, buffs: usize) -> (usize, usize) {
    let fit = |pos: AuraPos, size: f32| {
        if pos == AuraPos::Hidden {
            0
        } else {
            row_fit(size)
        }
    };
    if layout.debuffs == layout.buffs && layout.buffs != AuraPos::Hidden {
        share(
            debuffs,
            buffs,
            row_fit(layout.debuff_size.max(layout.buff_size)),
        )
    } else {
        (
            fit(layout.debuffs, layout.debuff_size),
            fit(layout.buffs, layout.buff_size),
        )
    }
}
/// The name's size, and the least it shrinks to before its end is clipped; the level and health.
const COMPACT_NAME: f32 = 7.0;
const COMPACT_NAME_MIN: f32 = 5.5;
const COMPACT_SMALL: f32 = 5.5;
/// The most of a shared row the unit's buffs may take.
const COMPACT_BUFFS: usize = 3;
const SILVER: Color = Color::srgb(0.78, 0.82, 0.9);
const ELITE_GOLD: Color = Color::srgb(0.88, 0.7, 0.22);

/// The saved CVar behind the Interface tab's Nameplate health row.
pub(super) const PLATE_HEALTH_TEXT: &str = "SurvivorHudPlateHealth";

/// The compact bar's frame: gold for bosses and lair elites, silver for silvers, else the boss
/// bars' grey. The selected unit's is crimson ([`target_edge`]).
fn compact_edge(en: &Enemy) -> Color {
    if en.boss || en.home.is_some() {
        ELITE_GOLD
    } else if en.silver {
        SILVER
    } else {
        STEEL_LO
    }
}

/// How a shared row's `slots` split: the buffs take at most `COMPACT_BUFFS` unless the debuffs
/// leave more free, the debuffs the rest.
fn share(debuffs: usize, buffs: usize, slots: usize) -> (usize, usize) {
    let b = buffs.min(COMPACT_BUFFS.max(slots.saturating_sub(debuffs)));
    (slots - b, b)
}

/// Of `len` auras in `slots` slots: the icons shown, and how many a "+n" stands for.
fn overflow(len: usize, slots: usize) -> (usize, usize) {
    if len <= slots {
        (len, 0)
    } else {
        let shown = slots.saturating_sub(1);
        (shown, len - shown)
    }
}

/// The compact style: the boss bars' frame round an 11px crimson well with the level, name and
/// health text in it, and the aura icons on the edges `layout` names.
fn spawn_compact_plate(
    commands: &mut Commands,
    f: &Handle<Font>,
    en: &Enemy,
    layout: AuraLayout,
) -> Entity {
    let geometry = CompactGeometry::of(layout);
    use super::right_column::{label as text, STEEL, TEXT_HI, WELL};
    let span = || Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    let hi = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(WINE_HI),
        ))
        .id();
    let fill = commands
        .spawn((span(), BackgroundColor(WINE)))
        .add_child(hi)
        .id();
    let level = commands
        .spawn(text(f, &en.level.to_string(), COMPACT_SMALL, STEEL))
        .id();
    let name = commands
        .spawn(text(
            f,
            en.def.name,
            COMPACT_NAME,
            if en.silver { SILVER } else { TEXT_HI },
        ))
        .id();
    let name_box = commands
        .spawn(Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        })
        .add_child(name)
        .id();
    let health = commands.spawn(text(f, "", COMPACT_SMALL, TEXT_HI)).id();
    let words = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(2.0)),
            column_gap: Val::Px(2.0),
            ..span()
        })
        .add_children(&[level, name_box, health])
        .id();
    let bar = commands
        .spawn((
            Node {
                left: Val::Px(0.0),
                top: Val::Px(geometry.bar_top),
                width: Val::Px(COMPACT_W),
                height: Val::Px(COMPACT_BAR_H),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip(),
                ..absolute()
            },
            BackgroundColor(WELL),
            BorderColor::all(compact_edge(en)),
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
        ))
        .add_children(&[fill, words])
        .id();

    // The rows: the debuffs from the left and the buffs from the right, each on its edge, each
    // side's "+n" after its icons, which [`compact_plates`] adds when the side first needs them.
    let side = |commands: &mut Commands, left: bool| {
        let size = if left {
            layout.debuff_size
        } else {
            layout.buff_size
        };
        let count = commands.spawn(text(f, "", COMPACT_SMALL, TEXT_HI)).id();
        let chip = commands
            .spawn((
                Node {
                    width: Val::Px(size),
                    height: Val::Px(size),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(FRAME_INK),
                BorderColor::all(STEEL_LO),
                Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
            ))
            .add_child(count)
            .id();
        let row = commands
            .spawn(Node {
                flex_direction: if left {
                    FlexDirection::Row
                } else {
                    FlexDirection::RowReverse
                },
                column_gap: Val::Px(COMPACT_AURA_GAP),
                ..default()
            })
            .add_child(chip)
            .id();
        (row, (chip, count))
    };
    let (left_row, more_debuffs) = side(commands, true);
    let (right_row, more_buffs) = side(commands, false);
    // Each edge's row hugs the bar: the icons over it sit on their bottoms, those under on tops.
    let mut edge = |top: f32, height: f32, align: AlignItems| {
        commands
            .spawn(Node {
                left: Val::Px(0.0),
                top: Val::Px(top),
                width: Val::Px(COMPACT_W),
                height: Val::Px(height),
                justify_content: JustifyContent::SpaceBetween,
                align_items: align,
                column_gap: Val::Px(COMPACT_AURA_GAP),
                ..absolute()
            })
            .id()
    };
    let above = edge(0.0, geometry.above, AlignItems::FlexEnd);
    let below = edge(
        geometry.height - geometry.below,
        geometry.below,
        AlignItems::FlexStart,
    );
    // A buffs row alone on its edge still keeps to the right.
    for (row, pos) in [(left_row, layout.debuffs), (right_row, layout.buffs)] {
        match pos {
            AuraPos::Above => commands.entity(above).add_child(row),
            AuraPos::Below => commands.entity(below).add_child(row),
            AuraPos::Hidden => commands
                .entity(row)
                .insert(Node {
                    display: Display::None,
                    ..default()
                })
                .insert(ChildOf(above)),
        };
    }
    for (row, pos) in [(right_row, layout.buffs)] {
        if pos != AuraPos::Hidden && pos != layout.debuffs {
            commands
                .entity(row)
                .entry::<Node>()
                .and_modify(|mut n| n.margin.left = Val::Auto);
        }
    }
    commands
        .spawn((
            RunEntity,
            Node {
                left: Val::Px(0.),
                top: Val::Px(0.),
                width: Val::Px(COMPACT_W),
                height: Val::Px(geometry.height),
                ..absolute()
            },
            GlobalZIndex(-1),
            UiTransform::default(),
            Visibility::Hidden,
            Plate {
                bar,
                fill,
                name,
                name_seat: name_box,
                level,
                level_seat: level,
            },
            CompactParts {
                health,
                name_box,
                rows: [left_row, right_row],
                debuffs: Vec::new(),
                buffs: Vec::new(),
                more: [more_debuffs, more_buffs],
                shown: None,
                auras_dirty: true,
            },
        ))
        .add_children(&[above, below, bar])
        .id()
}

fn spawn_plate(commands: &mut Commands, f: &Handle<Font>, en: &Enemy) -> Entity {
    let seat = || Node {
        width: Val::Px(0.0),
        height: Val::Px(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        overflow: Overflow::visible(),
        ..absolute()
    };
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            unpainted(),
            UiArt::tinted(nameplate::BAR_FILL_TEXTURE, Color::srgb(1.0, 0.0, 0.0)),
        ))
        .id();
    let bar = commands.spawn(absolute()).add_child(fill).id();
    let border = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..absolute()
            },
            unpainted(),
            UiArt::crop(nameplate::BORDER_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
        ))
        .id();
    let name = commands
        .spawn(label(
            f,
            en.def.name.to_string(),
            // A silver's name in silver.
            if en.silver { SILVER } else { Color::WHITE },
        ))
        .id();
    let name_seat = commands
        .spawn(Node {
            align_items: AlignItems::FlexEnd,
            ..seat()
        })
        .add_child(name)
        .id();
    let [r, g, b, _] = crate::vplates::CON_YELLOW;
    let level = commands
        .spawn(label(f, en.level.to_string(), Color::srgb(r, g, b)))
        .id();
    let level_seat = commands.spawn(seat()).add_child(level).id();
    commands
        .spawn((
            RunEntity,
            Node {
                left: Val::Px(0.),
                top: Val::Px(0.),
                ..absolute()
            },
            UiTransform::default(),
            // Under the HUD: a plate near the top of the screen must not cover the clock.
            GlobalZIndex(-1),
            Visibility::Hidden,
            Plate {
                bar,
                fill,
                name,
                name_seat,
                level,
                level_seat,
            },
        ))
        .add_children(&[bar, border, name_seat, level_seat])
        .id()
}

/// Fixed-cost seating for a Survivors horde. Dense piles never start a breadth-first search.
#[derive(Default)]
pub(super) struct PlateSeats(Vec<Rect>);

impl PlateSeats {
    fn clear(&mut self) {
        self.0.clear();
    }

    fn claim(&mut self, rect: Rect) {
        self.0.push(rect);
    }

    fn resolve(&self, desired: Rect, viewport: Vec2) -> Rect {
        let size = desired.size();
        let max = (viewport - size).max(Vec2::ZERO);
        let seed = desired.min.clamp(Vec2::ZERO, max).round();
        for row in [0., -1., 1., -2., 2., -3., 3., -4., 4., -5., 5., -6., 6.] {
            for column in [0., -1., 1.] {
                let at = (seed + Vec2::new(column * (size.x + 2.), row * (size.y + 2.)))
                    .clamp(Vec2::ZERO, max)
                    .round();
                let candidate = Rect::from_corners(at, at + size);
                if self
                    .0
                    .iter()
                    .all(|other| other.intersect(candidate).is_empty())
                {
                    return candidate;
                }
            }
        }
        Rect::from_corners(seed, seed + size)
    }
}

/// Keep encounter plates visible and add ordinary enemy plates while the WoW-style V toggle is on.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
#[derive(Default)]
pub(super) struct PlatePool {
    by_unit: HashMap<Entity, Entity>,
    spare: Vec<Entity>,
    style: Option<(bool, AuraLayout)>,
    shown: Vec<Entity>,
}

pub(super) fn enemy_plates(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    settings: Res<NameplateSettings>,
    font: Option<Res<HudFont>>,
    ui_scale: Res<UiScale>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    hero: Query<&Transform, (With<super::hero::Hero>, Without<Enemy>)>,
    cams: Query<(&Camera, &Transform), (With<WorldCamera>, Without<Enemy>)>,
    mut plates: Query<(&Plate, &mut UiTransform, Option<&mut CompactParts>)>,
    mut nodes: Query<&mut Node>,
    mut vis: Query<(Entity, &mut Visibility), With<Plate>>,
    mut texts: Query<&mut TextFont>,
    anchors: (
        Query<&BoneAttach>,
        Query<&benilla_world::rig_anim::RigPose>,
        Query<&OverheadFallback>,
        Query<&GlobalTransform>,
        Query<(), With<MountChild>>,
    ),
    mut pool: Local<PlatePool>,
    mut bucket: Local<PlateSeats>,
    hud: Query<(&ComputedNode, &bevy::ui::UiGlobalTransform), With<super::hud::TopStack>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    let PlatePool {
        by_unit,
        spare,
        style,
        shown,
    } = &mut *pool;
    let in_run = matches!(
        *phase.get(),
        Phase::Playing | Phase::Paused | Phase::LevelUp
    );
    let want_style = Some((settings.compact, settings.auras));
    if !in_run || *style != want_style {
        *style = want_style;
        for plate in by_unit
            .drain()
            .map(|(_, plate)| plate)
            .chain(spare.drain(..))
        {
            if let Ok(mut c) = commands.get_entity(plate) {
                c.despawn();
            }
        }
    }
    let compact = settings.compact;
    bucket.clear();
    if !in_run {
        for (_, mut visibility) in &mut vis {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    }
    let (Some(font), Ok((cam, cam_tf))) = (font, cams.single()) else {
        for (_, mut visibility) in &mut vis {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let Some(viewport) = cam.logical_viewport_size() else {
        for (_, mut visibility) in &mut vis {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let cam_g = GlobalTransform::from(*cam_tf);
    let s = ui_scale.0;
    let basis = plate_basis(viewport);
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = if compact {
        (COMPACT_W, CompactGeometry::of(settings.auras).height)
    } else {
        (gx(PLATE_W), gx(PLATE_H))
    };
    let bar_top = ph - gx(BAR_OFF_Y) - gx(BAR_H);
    let ui_viewport = viewport / s;
    // The clock, the run's line and the boss bars: a plate under them is unreadable and makes
    // them so, and every boss there already has its name in a bar.
    let covered = hud.single().ok().map(|(node, at)| {
        let inv = node.inverse_scale_factor();
        Rect::from_center_size(at.translation * inv, node.size() * inv)
    });
    let near = hero.single().ok().map(|h| h.translation);
    let mut encounters = Vec::new();
    let mut nearby = Vec::new();
    for (unit, tf, en) in &enemies {
        let encounter = en.boss || en.home.is_some();
        if en.dead || !(encounter || settings.enemies) {
            continue;
        }
        // Ordinary plates keep the client's 20-yard reach, measured from the hero.
        let dist_sq = near.map_or(f32::INFINITY, |h| h.distance_squared(tf.translation));
        if !encounter && dist_sq > NEARBY_SQ {
            continue;
        }
        let head = overhead_anchor(
            unit, tf, &anchors.0, &anchors.1, &anchors.2, &anchors.3, &anchors.4,
        ) + Vec3::Y * LIFT;
        let Some(screen) = cam
            .world_to_viewport(&cam_g, head)
            .ok()
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= viewport.x && p.y <= viewport.y)
        else {
            continue;
        };
        if encounter {
            encounters.push((dist_sq, (unit, en, screen)));
        } else {
            nearby.push((dist_sq, (unit, en, screen)));
        }
    }
    // A packed horde still shows only its nearest few; each plate costs the overlap solver a
    // pass over every plate seated before it.
    nearby.sort_by(|a, b| a.0.total_cmp(&b.0));
    nearby.truncate(MAX_NEARBY);
    encounters.sort_by(|a, b| {
        b.1 .1
            .boss
            .cmp(&a.1 .1.boss)
            .then_with(|| a.0.total_cmp(&b.0))
    });
    encounters.truncate(MAX_ENCOUNTER_PLATES);
    // Entity order makes the overlap solver's priority stable; query iteration order cannot
    // reshuffle a packed group from frame to frame. Encounters claim their seats first, so the
    // horde's plates make way for them rather than the other way round.
    encounters.sort_by_key(|(_, (unit, _, _))| unit.index());
    nearby.sort_by_key(|(_, (unit, _, _))| unit.index());
    spare.retain(|plate| plates.contains(*plate));
    by_unit.retain(|unit, plate| {
        if !plates.contains(*plate) {
            return false;
        }
        let keep = encounters
            .iter()
            .chain(&nearby)
            .any(|(_, (u, _, _))| u == unit);
        if !keep {
            commands.entity(*plate).remove::<PlateOf>();
            spare.push(*plate);
        }
        keep
    });

    shown.clear();
    for (_, (unit, en, screen)) in encounters.into_iter().chain(nearby) {
        let Some(&plate) = by_unit.get(&unit) else {
            let p = if let Some(p) = spare.pop() {
                let (parts, _, compact_parts) = plates.get_mut(p).expect("live spare plate");
                set_display(&mut nodes, p, true);
                let name_color = if en.silver {
                    SILVER
                } else if compact {
                    super::right_column::TEXT_HI
                } else {
                    Color::WHITE
                };
                commands
                    .entity(parts.name)
                    .insert((Text::new(en.def.name), TextColor(name_color)));
                commands.entity(parts.level).insert(Text::new(en.level.to_string()));
                if let Some(mut compact_parts) = compact_parts {
                    compact_parts.auras_dirty = true;
                    commands.entity(parts.bar).insert(BorderColor::all(compact_edge(en)));
                    if let Ok(mut font) = texts.get_mut(parts.name) {
                        if font.font_size != COMPACT_NAME {
                            font.font_size = COMPACT_NAME;
                        }
                    }
                }
                p
            } else if compact {
                spawn_compact_plate(&mut commands, &font.0, en, settings.auras)
            } else {
                spawn_plate(&mut commands, &font.0, en)
            };
            commands.entity(p).insert(PlateOf(unit));
            by_unit.insert(unit, p);
            continue;
        };
        let Ok((parts, mut position, _)) = plates.get_mut(plate) else {
            continue;
        };
        let desired = Rect::new(
            screen.x / s - pw * 0.5,
            screen.y / s,
            screen.x / s + pw * 0.5,
            screen.y / s + ph,
        );
        let seated = bucket.resolve(desired, ui_viewport);
        if covered.is_some_and(|hud| !hud.intersect(seated).is_empty()) {
            continue;
        }
        bucket.claim(seated);
        shown.push(plate);
        let translation = Val2::px(seated.min.x.round(), seated.min.y.round());
        if position.translation != translation {
            position.translation = translation;
        }
        if let Ok(mut n) = nodes.get_mut(plate) {
            if n.width != Val::Px(pw) {
                n.width = Val::Px(pw);
            }
            if n.height != Val::Px(ph) {
                n.height = Val::Px(ph);
            }
        }
        if compact {
            set_width(
                &mut nodes,
                parts.fill,
                (en.hp / en.max_hp.max(1.)).clamp(0., 1.),
            );
            continue;
        }
        if let Ok(mut n) = nodes.get_mut(parts.bar) {
            let (left, top, width, height) = (
                Val::Px(gx(BAR_OFF_X)),
                Val::Px(bar_top),
                Val::Px(gx(BAR_W)),
                Val::Px(gx(BAR_H)),
            );
            if (n.left, n.top, n.width, n.height) != (left, top, width, height) {
                n.left = left;
                n.top = top;
                n.width = width;
                n.height = height;
            }
        }
        set_width(
            &mut nodes,
            parts.fill,
            (en.hp / en.max_hp.max(1.)).clamp(0., 1.),
        );
        // The name's bottom on the plate's centre; the level's centre in the border's seat.
        if let Ok(mut n) = nodes.get_mut(parts.name_seat) {
            let (left, top) = (Val::Px(pw / 2.), Val::Px(ph / 2.));
            if (n.left, n.top) != (left, top) {
                n.left = left;
                n.top = top;
            }
        }
        if let Ok(mut n) = nodes.get_mut(parts.level_seat) {
            let (left, top) = (Val::Px(pw - gx(LEVEL_OFF_X)), Val::Px(ph - gx(LEVEL_OFF_Y)));
            if (n.left, n.top) != (left, top) {
                n.left = left;
                n.top = top;
            }
        }
        for (text, h) in [(parts.name, NAME_H), (parts.level, LEVEL_H)] {
            if let Ok(mut f) = texts.get_mut(text) {
                let size = text_px(h, basis) / s;
                if f.font_size != size {
                    f.font_size = size;
                }
            }
        }
    }
    for &plate in spare.iter() {
        set_display(&mut nodes, plate, false);
    }
    // Write visibility once, without hiding and re-showing every live plate each frame.
    for (plate, mut visibility) in &mut vis {
        visibility.set_if_neq(if shown.contains(&plate) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

// --- The boss bar -------------------------------------------------------------------------------

/// The bar's outer size; its well runs from `WELL_LEFT` to `BAR_W - WELL_RIGHT`.
const BAR_W: f32 = 300.0;
const BAR_H: f32 = 22.0;
const WELL_LEFT: f32 = 18.0;
const WELL_RIGHT: f32 = 3.0;
/// The aura columns either side of a bar; more icons than fit spill outward.
const SIDE_W: f32 = 88.0;
const AURA: f32 = 22.0;
/// Icons per side of a bar.
const MAX_AURAS: usize = 6;
/// Phase lines per bar.
const MAX_PHASES: usize = 4;
/// At most this many bars stack under the clock; bosses take the first rows.
const MAX_BARS: usize = 4;

const STEEL: Color = Color::srgb(0.604, 0.639, 0.722);
const STEEL_HI: Color = Color::srgb(0.831, 0.851, 0.894);
const STEEL_LO: Color = Color::srgb(0.365, 0.4, 0.475);
const ELITE_EDGE: Color = Color::srgb(0.247, 0.275, 0.337);
const FRAME_INK: Color = Color::srgb(0.051, 0.082, 0.09);
const WINE: Color = Color::srgb(0.541, 0.122, 0.2);
const WINE_HI: Color = Color::srgb(0.722, 0.227, 0.314);
const WINE_LO: Color = Color::srgb(0.298, 0.051, 0.106);
const SHIELD: Color = Color::srgb(0.373, 0.424, 0.533);
const SHIELD_HI: Color = Color::srgb(0.522, 0.573, 0.678);
const SHIELD_LO: Color = Color::srgb(0.173, 0.204, 0.282);
const LAG: Color = Color::srgba(0.82, 0.5, 0.56, 0.55);
const TICK: Color = Color::srgba(0.5, 0.56, 0.69, 0.55);
const TEXT_HI: Color = Color::srgb(0.91, 0.918, 0.941);
const TEXT_ELITE: Color = Color::srgb(0.788, 0.8, 0.839);
const TEXT_SMALL: Color = Color::srgb(0.722, 0.741, 0.792);
const CAST_TEXT: Color = Color::srgb(0.788, 0.839, 0.949);

/// The boss bars under the clock, one per engaged boss or elite however far off it is: a ringed
/// medallion, the name and level and health inside a crimson well framed in steel, the script's
/// phase lines, the move it winds up, and its buffs to the left and debuffs to the right, each
/// with a tooltip. Yells use the separate announcement panel.
#[derive(Component)]
pub(super) struct BossBar {
    rows: Vec<BarRow>,
}

struct BarRow {
    row: Entity,
    bar: Entity,
    fill: Entity,
    /// The fill's top line and the four bands that darken its foot.
    bands: [Entity; 5],
    lag: Entity,
    cast: Entity,
    phases: Vec<Entity>,
    caps: [Entity; 2],
    ring: Entity,
    skull: Entity,
    star: Entity,
    name: Entity,
    level: Entity,
    small: Entity,
    pct: Entity,
    buffs: Vec<AuraSlot>,
    debuffs: Vec<AuraSlot>,
    /// The encounter it shows.
    shows: Option<Entity>,
}

pub(super) struct AuraSlot {
    pub(super) frame: Entity,
    image: Entity,
    band: Entity,
    timer: Entity,
    badge: Entity,
    stacks: Entity,
}

/// What an aura icon's tooltip says; [`super::hud::icon_tooltip`] draws it. `rev` moves on
/// whenever the aura does.
#[derive(Component, Default)]
pub(super) struct AuraTip {
    pub aura: Option<super::mechanics::Aura>,
    pub rev: u32,
}

fn bar_text(f: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: f.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(0.0, 1.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.95),
        },
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
    )
}

/// A steel diamond of `size`, its centre at (`x`, `y`) in its parent's padding box.
fn diamond(size: f32, x: f32, y: f32) -> impl Bundle {
    (
        Node {
            left: Val::Px(x - size / 2.0),
            top: Val::Px(y - size / 2.0),
            width: Val::Px(size),
            height: Val::Px(size),
            ..absolute()
        },
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
        BackgroundColor(STEEL_LO),
        Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
    )
}

/// Where the fill's foot starts darkening, and the share each of its four bands takes.
const FOOT: f32 = 45.0;
const FOOT_BAND: f32 = (100.0 - FOOT) / 4.0;

/// A top-lit fill as flat colours: the body, its top line, and four bands from the body down to
/// `lo`. Flat, because Bevy's `BackgroundGradient` lands darker than the same colours flat.
fn fill_tones(hi: Color, mid: Color, lo: Color) -> [Color; 6] {
    let band = |t: f32| mid.mix(&lo, t);
    [mid, hi, band(0.125), band(0.375), band(0.625), band(0.875)]
}

/// A horizontal band across its parent, from `top` percent, `height` percent tall.
fn band(top: f32, height: f32, color: Color) -> impl Bundle {
    (
        Node {
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Percent(top),
            height: Val::Percent(height),
            ..absolute()
        },
        BackgroundColor(color),
    )
}

/// The medallion's mark: the mock's skull and star, drawn into textures by [`fill_glyphs`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum BarGlyph {
    Skull,
    Star,
}

#[derive(Component)]
pub(super) struct GlyphDone;

/// Coverage of a glyph at a point of its 24-unit box: the skull's cranium and jaw less its eyes
/// and nose, or the five-point star about the box's centre.
fn glyph_covers(glyph: BarGlyph, x: f32, y: f32) -> bool {
    let circle = |cx: f32, cy: f32, r: f32| (x - cx).powi(2) + (y - cy).powi(2) <= r * r;
    let inside = |pts: &[(f32, f32)]| {
        let mut odd = false;
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let (xi, yi) = pts[i];
            let (xj, yj) = pts[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                odd = !odd;
            }
            j = i;
        }
        odd
    };
    match glyph {
        BarGlyph::Skull => {
            let jaw = (9.0..=15.0).contains(&x) && (14.5..=18.5).contains(&y);
            let eyes = circle(9.8, 11.2, 1.5) || circle(14.2, 11.2, 1.5);
            let nose = inside(&[(12.0, 13.2), (11.2, 14.5), (12.8, 14.5)]);
            (circle(12.0, 11.0, 5.5) || jaw) && !eyes && !nose
        }
        BarGlyph::Star => inside(&[
            (12.0, 5.5),
            (13.53, 9.9),
            (18.18, 9.99),
            (14.47, 12.8),
            (15.82, 17.26),
            (12.0, 14.6),
            (8.18, 17.26),
            (9.53, 12.8),
            (5.82, 9.99),
            (10.47, 9.9),
        ]),
    }
}

/// A glyph as a white 64×64 texture with 4×4 supersampled edges, tinted by its `ImageNode`.
fn glyph_image(glyph: BarGlyph) -> Image {
    const SIZE: u32 = 64;
    const SAMPLES: u32 = 4;
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for py in 0..SIZE {
        for px in 0..SIZE {
            let mut hits = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let at = |p: u32, s: u32| {
                        (p as f32 + (s as f32 + 0.5) / SAMPLES as f32) * 24.0 / SIZE as f32
                    };
                    if glyph_covers(glyph, at(px, sx), at(py, sy)) {
                        hits += 1;
                    }
                }
            }
            let a = (hits * 255 / (SAMPLES * SAMPLES)) as u8;
            data.extend_from_slice(&[255, 255, 255, a]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}

/// Give each medallion mark its texture, drawing the two once.
pub(super) fn fill_glyphs(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut marks: Query<(Entity, &BarGlyph, &mut ImageNode), Without<GlyphDone>>,
    mut made: Local<Option<[Handle<Image>; 2]>>,
) {
    if marks.is_empty() {
        return;
    }
    let [skull, star] = made
        .get_or_insert_with(|| {
            [
                images.add(glyph_image(BarGlyph::Skull)),
                images.add(glyph_image(BarGlyph::Star)),
            ]
        })
        .clone();
    for (e, glyph, mut img) in &mut marks {
        img.image = match glyph {
            BarGlyph::Skull => skull.clone(),
            BarGlyph::Star => star.clone(),
        };
        commands.entity(e).insert(GlyphDone);
    }
}

pub(super) fn spawn_aura_slot(commands: &mut Commands, f: &Handle<Font>, size: f32) -> AuraSlot {
    if size <= COMPACT_AURA {
        return spawn_tiny_slot(commands, f, size);
    }
    let small = size < AURA;
    let type_size = if small { 7.0 } else { 8.0 };
    let image = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                ..default()
            },
            super::hud::SpellIcon(0),
            super::hud::IconDone,
        ))
        .id();
    let timer = commands.spawn(bar_text(f, type_size, TEXT_HI)).id();
    let band = commands
        .spawn((
            Node {
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(if small { 7.0 } else { 9.0 }),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..absolute()
            },
            BackgroundColor(Color::srgba(0.02, 0.027, 0.035, 0.75)),
        ))
        .add_child(timer)
        .id();
    let stacks = commands.spawn(bar_text(f, type_size, TEXT_HI)).id();
    // The count of a stacking aura, on the middle of its top edge.
    let badge = commands
        .spawn((
            Node {
                left: Val::Px((size - 2.0 - 13.0) / 2.0),
                top: Val::Px(-7.0),
                width: Val::Px(13.0),
                height: Val::Px(11.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..absolute()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(WINE_HI),
            ZIndex(1),
        ))
        .add_child(stacks)
        .id();
    let frame = commands
        .spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::BLACK),
            Outline::new(Val::Px(2.0), Val::ZERO, STEEL_LO),
            Interaction::default(),
            super::hud::TipFor::Aura,
            AuraTip::default(),
        ))
        .add_children(&[image, band, badge])
        .id();
    AuraSlot {
        frame,
        image,
        band,
        timer,
        badge,
        stacks,
    }
}

/// A nameplate's icon: no time left on it, only the stack count, small in its bottom-right corner.
/// Its `band` and `timer` are an empty node that never shows.
fn spawn_tiny_slot(commands: &mut Commands, f: &Handle<Font>, size: f32) -> AuraSlot {
    let image = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                ..default()
            },
            super::hud::SpellIcon(0),
            super::hud::IconDone,
        ))
        .id();
    let band = commands
        .spawn(Node {
            width: Val::Px(0.0),
            height: Val::Px(0.0),
            ..absolute()
        })
        .id();
    let stacks = commands
        .spawn(super::right_column::label(f, "", 5.0, TEXT_HI))
        .id();
    let badge = commands
        .spawn((
            Node {
                right: Val::Px(-1.0),
                bottom: Val::Px(-2.0),
                display: Display::None,
                ..absolute()
            },
            ZIndex(1),
        ))
        .add_child(stacks)
        .id();
    let frame = commands
        .spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(STEEL_LO),
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
            Interaction::default(),
            super::hud::TipFor::Aura,
            AuraTip::default(),
        ))
        .add_children(&[image, badge, band])
        .id();
    AuraSlot {
        frame,
        image,
        band,
        timer: band,
        badge,
        stacks,
    }
}

/// The ringed medallion over the bar's left end: black, a steel ring, black, a blue-grey inner
/// ring and a dark core, with a skull for a boss or a star for an elite.
fn spawn_medallion(commands: &mut Commands) -> (Entity, Entity, Entity, Entity) {
    let disc = |commands: &mut Commands, inset: f32, color: Color| {
        commands
            .spawn((
                Node {
                    left: Val::Px(inset),
                    top: Val::Px(inset),
                    right: Val::Px(inset),
                    bottom: Val::Px(inset),
                    border_radius: BorderRadius::MAX,
                    ..absolute()
                },
                BackgroundColor(color),
            ))
            .id()
    };
    let ring = disc(commands, 1.0, STEEL_LO);
    let gap = disc(commands, 2.0, Color::srgb(0.043, 0.055, 0.078));
    let inner = disc(commands, 6.0, Color::srgb(0.173, 0.208, 0.314));
    let core = disc(commands, 7.5, Color::srgb(0.102, 0.122, 0.169));
    // The mark's 24-unit box drawn 18 across, centred on the medallion.
    let glyph = |commands: &mut Commands, glyph: BarGlyph, tint: Color| {
        commands
            .spawn((
                Node {
                    left: Val::Px(4.0),
                    top: Val::Px(4.0),
                    width: Val::Px(18.0),
                    height: Val::Px(18.0),
                    ..absolute()
                },
                ImageNode {
                    color: tint,
                    ..default()
                },
                glyph,
            ))
            .id()
    };
    let skull = glyph(commands, BarGlyph::Skull, STEEL_HI);
    let star = glyph(commands, BarGlyph::Star, Color::srgb(0.725, 0.741, 0.784));
    let medallion = commands
        .spawn((
            Node {
                // Centred on the bar's left end, (6, 11) from its outer corner.
                left: Val::Px(6.0 - 1.0 - 13.0),
                top: Val::Px(BAR_H / 2.0 - 1.0 - 13.0),
                width: Val::Px(26.0),
                height: Val::Px(26.0),
                border_radius: BorderRadius::MAX,
                ..absolute()
            },
            BackgroundColor(Color::BLACK),
            ZIndex(2),
        ))
        .add_children(&[ring, gap, inner, core, skull, star])
        .id();
    (medallion, ring, skull, star)
}

fn spawn_bar_row(commands: &mut Commands, f: &Handle<Font>) -> BarRow {
    let span = || Node {
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        width: Val::Percent(100.0),
        ..absolute()
    };
    let lag = commands.spawn((span(), BackgroundColor(LAG))).id();
    let edge = commands
        .spawn((
            Node {
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(1.0),
                ..absolute()
            },
            BackgroundColor(Color::srgba(1.0, 0.78, 0.82, 0.7)),
        ))
        .id();
    let [body, top, b1, b2, b3, b4] = fill_tones(WINE_HI, WINE, WINE_LO);
    let line = commands
        .spawn((
            Node {
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                height: Val::Px(1.0),
                ..absolute()
            },
            BackgroundColor(top),
        ))
        .id();
    let feet: Vec<Entity> = [b1, b2, b3, b4]
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            commands
                .spawn(band(FOOT + FOOT_BAND * i as f32, FOOT_BAND, c))
                .id()
        })
        .collect();
    let fill = commands
        .spawn((span(), BackgroundColor(body)))
        .add_children(&[line, feet[0], feet[1], feet[2], feet[3], edge])
        .id();
    let bands = [line, feet[0], feet[1], feet[2], feet[3]];
    // Short ticks along the foot every tenth, clear of the text.
    let ticks: Vec<Entity> = (1..10)
        .map(|i| {
            commands
                .spawn((
                    Node {
                        left: Val::Percent(i as f32 * 10.0),
                        bottom: Val::Px(0.0),
                        width: Val::Px(1.0),
                        height: Val::Percent(26.0),
                        ..absolute()
                    },
                    BackgroundColor(TICK),
                ))
                .id()
        })
        .collect();
    let phases: Vec<Entity> = (0..MAX_PHASES)
        .map(|_| {
            commands
                .spawn((
                    Node {
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        width: Val::Px(1.0),
                        display: Display::None,
                        ..absolute()
                    },
                    BackgroundColor(STEEL_HI),
                    BoxShadow::new(
                        Color::srgba(0.83, 0.85, 0.89, 0.6),
                        Val::ZERO,
                        Val::ZERO,
                        Val::ZERO,
                        Val::Px(3.0),
                    ),
                ))
                .id()
        })
        .collect();
    let cast = commands
        .spawn((
            Node {
                left: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(2.0),
                width: Val::Percent(0.0),
                display: Display::None,
                ..absolute()
            },
            BackgroundColor(Color::srgb(0.7, 0.76, 0.9)),
            BoxShadow::new(
                Color::srgba(0.78, 0.84, 0.96, 0.7),
                Val::ZERO,
                Val::ZERO,
                Val::ZERO,
                Val::Px(4.0),
            ),
        ))
        .id();

    let name = commands.spawn(bar_text(f, 12.0, TEXT_HI)).id();
    let level = commands.spawn(bar_text(f, 9.0, STEEL)).id();
    let who = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(5.0),
            ..default()
        })
        .add_children(&[name, level])
        .id();
    let small = commands.spawn(bar_text(f, 9.0, TEXT_SMALL)).id();
    let pct = commands
        .spawn((
            bar_text(f, 12.0, TEXT_HI),
            Node {
                min_width: Val::Px(26.0),
                ..default()
            },
        ))
        .id();
    let hp = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .add_children(&[small, pct])
        .id();
    let text = commands
        .spawn(Node {
            padding: UiRect::horizontal(Val::Px(6.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..span()
        })
        .add_children(&[who, hp])
        .id();

    // The well darkens toward its top, in flat bands like the fill.
    let shade = [
        commands
            .spawn(band(0.0, 20.0, Color::srgb(0.075, 0.027, 0.047)))
            .id(),
        commands
            .spawn(band(20.0, 20.0, Color::srgb(0.09, 0.033, 0.056)))
            .id(),
        commands
            .spawn(band(70.0, 30.0, Color::srgb(0.13, 0.047, 0.08)))
            .id(),
    ];
    let mut well_children = shade.to_vec();
    well_children.extend([lag, fill]);
    well_children.extend(&ticks);
    well_children.extend(&phases);
    well_children.extend([cast, text]);
    let well = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.106, 0.039, 0.067)),
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
        ))
        .add_children(&well_children)
        .id();

    // Positions in the bar's padding box, which starts one pixel in, past its border.
    let mid = WELL_LEFT + (BAR_W - WELL_LEFT - WELL_RIGHT) / 2.0 - 1.0;
    let crest = commands.spawn(diamond(7.0, mid, -1.0)).id();
    let cap = commands
        .spawn(diamond(9.0, BAR_W - 1.0, BAR_H / 2.0 - 1.0))
        .id();
    let (medallion, ring, skull, star) = spawn_medallion(commands);
    let bar = commands
        .spawn((
            Node {
                width: Val::Px(BAR_W),
                height: Val::Px(BAR_H),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect {
                    left: Val::Px(WELL_LEFT - 1.0),
                    right: Val::Px(WELL_RIGHT - 1.0),
                    top: Val::Px(2.0),
                    bottom: Val::Px(2.0),
                },
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(FRAME_INK),
            BorderColor::all(STEEL_LO),
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.6),
                Val::ZERO,
                Val::Px(2.0),
                Val::ZERO,
                Val::Px(6.0),
            ),
        ))
        .add_children(&[well, crest, cap, medallion])
        .id();

    let buffs: Vec<AuraSlot> = (0..MAX_AURAS)
        .map(|_| spawn_aura_slot(commands, f, AURA))
        .collect();
    let debuffs: Vec<AuraSlot> = (0..MAX_AURAS)
        .map(|_| spawn_aura_slot(commands, f, AURA))
        .collect();
    let side = |commands: &mut Commands, slots: &[AuraSlot], left: bool| {
        // Buffs are laid right to left, so the first sits next to the bar.
        let mut frames: Vec<Entity> = slots.iter().map(|s| s.frame).collect();
        if left {
            frames.reverse();
        }
        commands
            .spawn(Node {
                width: Val::Px(SIDE_W),
                height: Val::Px(BAR_H),
                flex_shrink: 0.0,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                justify_content: if left {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                },
                // Clear of the medallion, which reaches past the bar's left end.
                padding: UiRect::right(Val::Px(if left { 9.0 } else { 0.0 })),
                ..default()
            })
            .add_children(&frames)
            .id()
    };
    let buff_side = side(commands, &buffs, true);
    let debuff_side = side(commands, &debuffs, false);
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                display: Display::None,
                ..default()
            },
            Visibility::Hidden,
        ))
        .add_children(&[buff_side, bar, debuff_side])
        .id();
    BarRow {
        row,
        bar,
        fill,
        bands,
        lag,
        cast,
        phases,
        caps: [crest, cap],
        ring,
        skull,
        star,
        name,
        level,
        small,
        pct,
        buffs,
        debuffs,
        shows: None,
    }
}

fn spawn_boss_bar(commands: &mut Commands, f: &Handle<Font>) -> Entity {
    let rows: Vec<BarRow> = (0..MAX_BARS).map(|_| spawn_bar_row(commands, f)).collect();
    let children: Vec<Entity> = rows.iter().map(|row| row.row).collect();
    // In the top column ([`super::hud::TopStack`]), one row per engaged enemy.
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            Visibility::Hidden,
            BossBar { rows },
        ))
        .add_children(&children)
        .id()
}

/// Health as the bar prints it: `713`, `9.8k`, `15.4k`, `1.2M`.
fn short(n: f32) -> String {
    let n = n.max(0.0);
    if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e4 {
        format!("{:.1}k", n / 1e3)
    } else if n >= 1e3 {
        format!("{:.2}k", n / 1e3)
    } else {
        format!("{}", n.round() as u32)
    }
}

/// Whether an enemy is in a fight worth a bar: a boss, an awake lair guardian, or a roaming
/// elite that has been hit.
fn engaged(e: &Enemy) -> bool {
    !e.silver
        && (e.boss || e.home.is_some_and(|h| h.awake) || (e.def.roaming_elite && e.hp < e.max_hp))
}

fn set_text(texts: &mut Query<&mut Text>, e: Entity, s: String) {
    if let Ok(mut t) = texts.get_mut(e) {
        if t.0 != s {
            t.0 = s;
        }
    }
}

fn set_display(nodes: &mut Query<&mut Node>, e: Entity, on: bool) {
    if let Ok(mut n) = nodes.get_mut(e) {
        let want = if on { Display::Flex } else { Display::None };
        if n.display != want {
            n.display = want;
        }
    }
}

fn set_width(nodes: &mut Query<&mut Node>, e: Entity, share: f32) {
    if let Ok(mut n) = nodes.get_mut(e) {
        let want = Val::Percent(share * 100.0);
        if n.width != want {
            n.width = want;
        }
    }
}

/// The bar's paint for this frame: the parts that change with focus, rank and the fight's state.
#[derive(SystemParam)]
pub(super) struct BarPaint<'w, 's> {
    borders: Query<'w, 's, &'static mut BorderColor>,
    shadows: Query<'w, 's, &'static mut BoxShadow>,
    colors: Query<'w, 's, &'static mut BackgroundColor>,
    tints: Query<'w, 's, &'static mut TextColor>,
}

impl BarPaint<'_, '_> {
    fn border(&mut self, e: Entity, color: Color) {
        if let Ok(mut b) = self.borders.get_mut(e) {
            let want = BorderColor::all(color);
            if *b != want {
                *b = want;
            }
        }
    }
    fn shadow(&mut self, e: Entity, want: BoxShadow) {
        if let Ok(mut s) = self.shadows.get_mut(e) {
            if *s != want {
                *s = want;
            }
        }
    }
    fn color(&mut self, e: Entity, color: Color) {
        if let Ok(mut c) = self.colors.get_mut(e) {
            if c.0 != color {
                c.0 = color;
            }
        }
    }
    fn tint(&mut self, e: Entity, color: Color) {
        if let Ok(mut c) = self.tints.get_mut(e) {
            if c.0 != color {
                c.0 = color;
            }
        }
    }
}

fn brighter(c: Color, k: f32) -> Color {
    let l = c.to_linear();
    Color::linear_rgba(l.red * k, l.green * k, l.blue * k, l.alpha)
}

/// Show a bar for every engaged boss and elite wherever it is, bosses first and then the
/// nearest. Yells appear in the compact announcement panel.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn boss_bar(
    mut commands: Commands,
    time: Res<Time>,
    phase: (Res<State<Phase>>, Res<super::ui_edit::UiEditor>),
    font: Option<Res<HudFont>>,
    mut run: ResMut<super::Run>,
    selection: Res<crate::target::Selection>,
    spells: Option<Res<crate::ui_action::Spells>>,
    hero: Query<&Transform, (With<super::hero::Hero>, Without<Enemy>)>,
    enemies: Query<(
        Entity,
        &Transform,
        &Enemy,
        Option<&super::mechanics::Mechanics>,
    )>,
    mut bars: Query<(&mut BossBar, &mut Visibility)>,
    mut shown: Query<&mut Visibility, Without<BossBar>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    mut paint: BarPaint,
    stack: Query<Entity, With<super::hud::TopStack>>,
    mut lags: Local<Vec<f32>>,
) {
    let (phase, editor) = phase;
    if let Some((_, left)) = run.yell.as_mut() {
        *left -= time.delta_secs();
        if *left <= 0.0 {
            run.yell = None;
        }
    }
    let Ok((mut bar, mut vis)) = bars.single_mut() else {
        if let (Some(font), Ok(stack)) = (font, stack.single()) {
            let bar = spawn_boss_bar(&mut commands, &font.0);
            commands.entity(stack).add_child(bar);
        }
        return;
    };
    let playing = editor.active || !matches!(*phase.get(), Phase::Menu | Phase::GameOver);
    let at = hero.single().map(|h| h.translation).unwrap_or_default();
    let mut fights: Vec<_> = enemies
        .iter()
        .filter(|(_, _, e, _)| !e.dead && engaged(e))
        .map(|(id, tf, e, m)| (!e.boss, tf.translation.distance(at), id, e, m))
        .collect();
    fights.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    if editor.active && fights.is_empty() {
        vis.set_if_neq(Visibility::Inherited);
        for (i, row) in bar.rows.iter_mut().enumerate() {
            row.shows = None;
            set_display(&mut nodes, row.row, i == 0);
            if let Ok(mut v) = shown.get_mut(row.row) {
                v.set_if_neq(if i == 0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            if i != 0 {
                continue;
            }
            set_width(&mut nodes, row.fill, 0.75);
            set_width(&mut nodes, row.lag, 0.8);
            set_text(&mut texts, row.name, "Hogger".into());
            set_text(&mut texts, row.level, "12".into());
            set_text(&mut texts, row.pct, "75%".into());
            set_text(&mut texts, row.small, "75k / 100k".into());
            paint.border(row.bar, STEEL_LO);
            paint.color(row.ring, STEEL_LO);
            paint.tint(row.name, TEXT_HI);
            paint.tint(row.small, TEXT_SMALL);
            let tones = fill_tones(WINE_HI, WINE, WINE_LO);
            paint.color(row.fill, tones[0]);
            for (band, tone) in row.bands.iter().zip(&tones[1..]) {
                paint.color(*band, *tone);
            }
            set_display(&mut nodes, row.skull, true);
            set_display(&mut nodes, row.star, false);
            set_display(&mut nodes, row.cast, false);
            for &line in &row.phases {
                set_display(&mut nodes, line, false);
            }
            for slot in row.buffs.iter().chain(&row.debuffs) {
                set_display(&mut nodes, slot.frame, false);
            }
        }
        return;
    }
    // A bar keeps its row while its fight lasts, so the bars never trade places as the hero moves
    // or turns to another target; a new fight takes the first free row, bosses before elites.
    let mut seats: Vec<Option<usize>> = bar
        .rows
        .iter()
        .map(|r| r.shows.and_then(|id| fights.iter().position(|f| f.2 == id)))
        .collect();
    for k in 0..fights.len() {
        if !seats.contains(&Some(k)) {
            if let Some(free) = seats.iter_mut().find(|s| s.is_none()) {
                *free = Some(k);
            }
        }
    }
    // Yells use the compact announcement panel instead of a second line at the boss bars.
    *vis = if playing && !fights.is_empty() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    lags.resize(bar.rows.len(), 1.0);
    let dt = time.delta_secs();
    let now = run.elapsed;
    // One beat for the enrage glow and the low-health throb.
    let beat = 0.5 - 0.5 * (time.elapsed_secs() * std::f32::consts::TAU / 1.2).cos();
    for (i, row) in bar.rows.iter_mut().enumerate() {
        let entry = seats[i].map(|k| &fights[k]);
        set_display(&mut nodes, row.row, entry.is_some());
        if let Ok(mut v) = shown.get_mut(row.row) {
            let want = if entry.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *v != want {
                *v = want;
            }
        }
        let Some(&(_, _, id, e, mech)) = entry else {
            row.shows = None;
            continue;
        };
        let share = (e.hp / e.max_hp).clamp(0.0, 1.0);
        // The pale strip holds a moment's damage, then eases down to the health.
        let lag = &mut lags[i];
        if row.shows != Some(id) || *lag < share {
            *lag = share;
        } else {
            *lag += (share - *lag) * (1.0 - (-dt * 2.5).exp());
        }
        row.shows = Some(id);
        set_width(&mut nodes, row.fill, share);
        set_width(&mut nodes, row.lag, *lag);

        // Rank: a boss in steel with a skull, an elite in darker steel with a star.
        let elite = !e.boss;
        let focus = selection.target == Some(id);
        let raging = e.hogger_enraged || mech.is_some_and(|m| m.raging());
        paint.border(row.bar, if elite { ELITE_EDGE } else { STEEL_LO });
        let drop = (Color::srgba(0.0, 0.0, 0.0, 0.6), 0.0, 2.0, 0.0, 6.0);
        let (color, x, y, spread, blur) = if raging {
            (
                Color::srgba(0.75, 0.12, 0.22, 0.35 + 0.4 * beat),
                0.0,
                0.0,
                1.0 + 2.0 * beat,
                8.0 + 4.0 * beat,
            )
        } else if focus {
            (Color::srgba(0.365, 0.4, 0.475, 0.45), 0.0, 0.0, 0.0, 6.0)
        } else {
            drop
        };
        paint.shadow(
            row.bar,
            BoxShadow::new(
                color,
                Val::Px(x),
                Val::Px(y),
                Val::Px(spread),
                Val::Px(blur),
            ),
        );
        for cap in row.caps {
            paint.color(cap, if elite { ELITE_EDGE } else { STEEL_LO });
        }
        paint.color(row.ring, if elite { ELITE_EDGE } else { STEEL_LO });
        set_display(&mut nodes, row.skull, !elite);
        set_display(&mut nodes, row.star, elite);
        paint.tint(row.name, if elite { TEXT_ELITE } else { TEXT_HI });

        // The fill: crimson, slate while shielded, throbbing below a fifth.
        let (hi, mid, lo) = if e.ward {
            (SHIELD_HI, SHIELD, SHIELD_LO)
        } else {
            (WINE_HI, WINE, WINE_LO)
        };
        let k = if share < 0.2 && !e.ward {
            1.0 + 0.45 * beat
        } else {
            1.0
        };
        let tones = fill_tones(brighter(hi, k), brighter(mid, k), brighter(lo, k));
        paint.color(row.fill, tones[0]);
        for (band, tone) in row.bands.iter().zip(&tones[1..]) {
            paint.color(*band, *tone);
        }

        // The script's turns, as lines across the well.
        let marks: Vec<f32> = mech.map(|m| m.phase_marks().collect()).unwrap_or_default();
        for (n, &line) in row.phases.iter().enumerate() {
            let mark = marks.get(n);
            set_display(&mut nodes, line, mark.is_some());
            if let (Some(&m), Ok(mut node)) = (mark, nodes.get_mut(line)) {
                let want = Val::Percent(m * 100.0);
                if node.left != want {
                    node.left = want;
                }
            }
        }

        set_text(&mut texts, row.name, e.def.name.to_string());
        set_text(&mut texts, row.level, e.level.to_string());
        set_text(
            &mut texts,
            row.pct,
            format!("{:.0}%", (share * 100.0).ceil()),
        );
        // A wind-up takes the numbers' place, its line filling along the foot.
        let cast = mech.and_then(|m| m.casting(now));
        set_display(&mut nodes, row.cast, cast.is_some());
        match cast {
            Some((spell, fallback, progress)) => {
                set_width(&mut nodes, row.cast, progress);
                let name = spells
                    .as_deref()
                    .and_then(|s| s.catalog.get(spell))
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| fallback.to_string());
                set_text(&mut texts, row.small, name);
                paint.tint(row.small, CAST_TEXT);
            }
            None => {
                set_text(
                    &mut texts,
                    row.small,
                    format!("{} / {}", short(e.hp), short(e.max_hp)),
                );
                paint.tint(row.small, TEXT_SMALL);
            }
        }
    }
}

/// A timer as an aura icon prints it: `2m`, `45`, `3.2`.
fn short_time(secs: f32) -> String {
    if secs >= 60.0 {
        format!("{}m", (secs / 60.0).ceil())
    } else if secs >= 10.0 {
        format!("{}", secs.ceil())
    } else {
        format!("{secs:.1}")
    }
}

/// Fill each bar's aura columns: buffs out from the bar's left, debuffs out from its right, each
/// icon with its time left, its stack count and its [`AuraTip`].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn boss_auras(
    mut commands: Commands,
    run: Res<super::Run>,
    spells: Option<Res<crate::ui_action::Spells>>,
    bars: Query<&BossBar>,
    enemies: Query<(&Enemy, Option<&super::mechanics::Mechanics>)>,
    mut slots: Query<(&mut AuraTip, &mut Outline)>,
    mut icons: Query<(&mut super::hud::SpellIcon, &mut ImageNode)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
) {
    let Ok(bar) = bars.single() else { return };
    let name = |spell: u32| {
        spells
            .as_deref()
            .and_then(|s| s.catalog.get(spell))
            .map(|d| d.name.clone())
    };
    for row in &bar.rows {
        let Some((en, mech)) = row.shows.and_then(|id| enemies.get(id).ok()) else {
            continue;
        };
        let mark = row.shows.and_then(|id| run.passives.marks.get(&id));
        let all = super::mechanics::auras(en, mech, mark, run.elapsed, name);
        let (buffs, debuffs): (Vec<_>, Vec<_>) = all.into_iter().partition(|a| a.buff);
        fill_slots(
            &mut commands,
            &row.buffs,
            buffs,
            STEEL_LO,
            &mut slots,
            &mut icons,
            &mut nodes,
            &mut texts,
        );
        fill_slots(
            &mut commands,
            &row.debuffs,
            debuffs,
            WINE_HI,
            &mut slots,
            &mut icons,
            &mut nodes,
            &mut texts,
        );
    }
}

/// Put `list`'s auras in `side`'s slots, in order, and hide the slots left over.
#[allow(clippy::too_many_arguments)]
pub(super) fn fill_slots(
    commands: &mut Commands,
    side: &[AuraSlot],
    list: Vec<super::mechanics::Aura>,
    edge: Color,
    slots: &mut Query<(&mut AuraTip, &mut Outline)>,
    icons: &mut Query<(&mut super::hud::SpellIcon, &mut ImageNode)>,
    nodes: &mut Query<&mut Node>,
    texts: &mut Query<&mut Text>,
) {
    let mut list = list.into_iter();
    for slot in side {
        let aura = list.next();
        set_display(nodes, slot.frame, aura.is_some());
        let Some(mut aura) = aura else {
            if let Ok((mut tip, _)) = slots.get_mut(slot.frame) {
                if tip.aura.is_some() {
                    tip.aura = None;
                }
            }
            continue;
        };
        set_display(nodes, slot.band, aura.left.is_some());
        set_text(
            texts,
            slot.timer,
            aura.left.map(short_time).unwrap_or_default(),
        );
        set_display(nodes, slot.badge, aura.stacks > 1);
        set_text(texts, slot.stacks, aura.stacks.to_string());
        if let Ok((mut icon, mut img)) = icons.get_mut(slot.image) {
            if icon.0 != aura.icon {
                icon.0 = aura.icon;
                img.color = Color::srgba(0.0, 0.0, 0.0, 0.6);
                commands.entity(slot.image).remove::<super::hud::IconDone>();
            }
        }
        // The tooltip counts down whole seconds, so it redraws once a second at most.
        aura.left = aura.left.map(f32::ceil);
        if let Ok((mut tip, mut outline)) = slots.get_mut(slot.frame) {
            if tip.aura.as_ref() != Some(&aura) {
                tip.aura = Some(aura);
                tip.rev = tip.rev.wrapping_add(1);
            }
            if outline.color != edge {
                outline.color = edge;
            }
        }
    }
}

/// One side of a plate's aura row: `count` icons of `size`, edged in its colour, ahead of its "+n".
fn spawn_plate_slots(
    commands: &mut Commands,
    f: &Handle<Font>,
    row: Entity,
    edge: Color,
    size: f32,
    count: usize,
    start: usize,
) -> Vec<AuraSlot> {
    let slots: Vec<AuraSlot> = (0..count)
        .map(|_| spawn_aura_slot(commands, f, size))
        .collect();
    for slot in &slots {
        commands.entity(slot.frame).insert(BorderColor::all(edge));
    }
    let frames: Vec<Entity> = slots.iter().map(|s| s.frame).collect();
    commands.entity(row).insert_children(start, &frames);
    slots
}

/// How often the plates' auras are read: they carry no timers, so a tenth of a second is fresh.
const PLATE_AURA_EVERY: f32 = 0.1;

/// Fill each compact plate on screen: the health text in the setting's format when it changed,
/// the name shrunk to fit beside it, and ten times a second the aura row.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn compact_plates(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<super::Run>,
    cvars: Res<crate::cvars::Cvars>,
    font: Option<Res<HudFont>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut plates: Query<(&PlateOf, &Plate, &mut CompactParts, &Visibility)>,
    enemies: Query<(&Enemy, Option<&super::mechanics::Mechanics>)>,
    sizes: Query<&ComputedNode>,
    mut fonts: Query<&mut TextFont>,
    mut slots: Query<(&mut AuraTip, &mut Outline)>,
    mut icons: Query<(&mut super::hud::SpellIcon, &mut ImageNode)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    settings: Res<NameplateSettings>,
    mut since: Local<f32>,
) {
    let Some(font) = font else { return };
    let format = super::unit_frames::HealthText::from_setting(
        cvars.get(PLATE_HEALTH_TEXT),
        super::unit_frames::HealthText::CurrentPercent,
    );
    let layout = settings.auras;
    *since += time.delta_secs();
    let auras_due = *since >= PLATE_AURA_EVERY;
    if auras_due {
        *since = 0.0;
    }
    let name = |spell: u32| {
        spells
            .as_deref()
            .and_then(|s| s.catalog.get(spell))
            .map(|d| d.name.clone())
    };
    for (of, plate, mut parts, vis) in &mut plates {
        if *vis == Visibility::Hidden {
            continue;
        }
        let Ok((en, mech)) = enemies.get(of.0) else {
            continue;
        };
        let now = Some((en.hp, en.max_hp, format));
        if parts.shown != now {
            parts.shown = now;
            set_text(&mut texts, parts.health, format.format(en.hp, en.max_hp));
        }

        // Text widths scale with their size, so one step lands the name on its box's width.
        if let (Ok(text), Ok(clip)) = (sizes.get(plate.name), sizes.get(parts.name_box)) {
            if let Ok(mut font) = fonts.get_mut(plate.name) {
                let (have, room) = (text.size().x, clip.size().x);
                if have > 0.0 && room > 0.0 {
                    let want = (font.font_size * room / have).clamp(COMPACT_NAME_MIN, COMPACT_NAME);
                    let want = (want * 4.0).floor() / 4.0;
                    if have > room + 0.5 && want < font.font_size {
                        font.font_size = want;
                    }
                }
            }
        }

        if !auras_due && !parts.auras_dirty {
            continue;
        }
        parts.auras_dirty = false;
        let mark = run.passives.marks.get(&of.0);
        let all = super::mechanics::auras(en, mech, mark, run.elapsed, name);
        let (buffs, debuffs): (Vec<_>, Vec<_>) = all.into_iter().partition(|a| a.buff);
        // Grow only to the number actually displayed, rather than allocating a full row per debuff.
        let (nd, nb) = rooms(layout, debuffs.len(), buffs.len());
        let need_debuffs = overflow(debuffs.len(), nd).0;
        let need_buffs = overflow(buffs.len(), nb).0;
        let [debuff_row, buff_row] = parts.rows;
        if need_debuffs > parts.debuffs.len() {
            let start = parts.debuffs.len();
            parts.debuffs.extend(spawn_plate_slots(
                &mut commands,
                &font.0,
                debuff_row,
                WINE_HI,
                layout.debuff_size,
                need_debuffs - start,
                start,
            ));
        }
        if need_buffs > parts.buffs.len() {
            let start = parts.buffs.len();
            parts.buffs.extend(spawn_plate_slots(
                &mut commands,
                &font.0,
                buff_row,
                STEEL_LO,
                layout.buff_size,
                need_buffs - start,
                start,
            ));
        }
        let [more_debuffs, more_buffs] = parts.more;
        for (list, side, room, (chip, count)) in [
            (debuffs, &parts.debuffs, nd, more_debuffs),
            (buffs, &parts.buffs, nb, more_buffs),
        ] {
            if side.is_empty() {
                continue;
            }
            let (shown, hidden) = overflow(list.len(), room);
            let mut list = list;
            list.truncate(shown);
            fill_slots(
                &mut commands,
                &side[..shown],
                list,
                Color::BLACK,
                &mut slots,
                &mut icons,
                &mut nodes,
                &mut texts,
            );
            fill_slots(
                &mut commands,
                &side[shown..],
                Vec::new(),
                Color::BLACK,
                &mut slots,
                &mut icons,
                &mut nodes,
                &mut texts,
            );
            set_display(&mut nodes, chip, hidden > 0);
            if hidden > 0 {
                set_text(&mut texts, count, format!("+{hidden}"));
            }
        }
    }
}

// --- The selected unit's plate ---------------------------------------------------------------

/// The unit a plate belongs to.
#[derive(Component)]
pub(super) struct PlateOf(pub Entity);

/// The hero's buffs as auras: the spells they cast on themselves, their shield, immunity, thorns
/// and healing over time, plus encounter debuffs such as broodling venom.
pub(super) fn hero_auras(
    run: &super::Run,
    name: impl Fn(u32, &str) -> String,
) -> Vec<super::mechanics::Aura> {
    let aura = |icon: u32, fallback: &str, text: String, left: f32| super::mechanics::Aura {
        icon,
        name: name(icon, fallback),
        text,
        left: Some(left),
        buff: true,
        stacks: 1,
    };
    let mut out: Vec<_> = run
        .buffs
        .iter()
        .map(|&(key, _, _, left)| {
            let spell = super::data::ABILITIES
                .iter()
                .find(|a| a.key == key)
                .map_or(0, |a| a.spell);
            aura(spell, key, String::new(), left)
        })
        .collect();
    if run.full_belly > 0 {
        let step = u64::from(super::food::FULL_BELLY_DAMAGE_PERCENT);
        let total = u64::from(run.full_belly) * step;
        out.insert(0, super::mechanics::Aura {
            icon: super::food::FULL_BELLY_ICON,
            name: "Full Belly".into(),
            text: format!("+{step}% damage per stack.\nTotal damage bonus: +{total}%.\nLasts for the rest of this run."),
            left: None,
            buff: true,
            stacks: run.full_belly,
        });
    }
    if run.shield_t > 0.0 && run.shield > 0.0 {
        out.push(aura(
            run.shield_spell,
            "Absorb Shield",
            format!("Absorbs {:.0} damage.", run.shield),
            run.shield_t,
        ));
    }
    if run.immune_t > 0.0 {
        out.push(aura(
            642,
            "Immune",
            "Immune to all damage.".into(),
            run.immune_t,
        ));
    }
    if run.thorns_t > 0.0 {
        out.push(aura(
            run.thorns_spell,
            "Thorns",
            "Attackers take damage.".into(),
            run.thorns_t,
        ));
    }
    if run.heal_t > 0.0 {
        out.push(aura(
            139,
            "Healing",
            format!("Heals {:.0} health a second.", run.heal_rate),
            run.heal_t,
        ));
    }
    if run.morgan_bleed.until > run.elapsed {
        out.push(super::mechanics::Aura {
            icon: 703, name: "Collector's Bleeding".into(),
            text: format!("Bleeding for {:.0} damage each second. Healing received reduced by 100%. Hits from Morgan's spells refresh the 30-second duration.", run.morgan_bleed.damage),
            left: Some(run.morgan_bleed.until - run.elapsed), buff: false, stacks: 1,
        });
    }
    if run.morgan_bomb_slow_until > run.elapsed {
        out.push(super::mechanics::Aura {
            icon: 13812, name: "Collector's Bomb: Slowed".into(),
            text: "Movement speed reduced by 50%. Bomb hits also cause bleeding and prevent healing for 30 seconds.".into(),
            left: Some(run.morgan_bomb_slow_until - run.elapsed), buff: false, stacks: 1,
        });
    }
    if run.morgan_mark_until > run.elapsed {
        out.push(super::mechanics::Aura {
            icon: 1130,
            name: "Marked for Collection".into(),
            text: "Morgan is aiming at you. Sidestep after the gunshot lane locks.".into(),
            left: Some(run.morgan_mark_until - run.elapsed),
            buff: false,
            stacks: 1,
        });
    }
    if run.stunned() {
        out.push(super::mechanics::Aura {
            icon: 853,
            name: "Stunned".into(),
            text: "Cannot move, attack or cast spells.".into(),
            left: Some(run.stun_until - run.elapsed),
            buff: false,
            stacks: 1,
        });
    }
    if run.healing_blocked_until > run.elapsed {
        out.push(super::mechanics::Aura {
            icon: 12294, name: "Boar Ambush: Healing Reduced".into(),
            text: "Healing received reduced by 100%. Further ambush hits refresh the 30-second duration.".into(),
            left: Some(run.healing_blocked_until - run.elapsed), buff: false, stacks: 1,
        });
    }
    if run.brood_poison.until > run.elapsed {
        out.push(super::mechanics::Aura {
            icon: 6917, name: "Brood Venom".into(),
            text: format!("Poisoned: {:.0} damage per second. Movement speed reduced by 30%. Further bites refresh the 3-second duration.", run.brood_poison.damage),
            left: Some(run.brood_poison.until - run.elapsed), buff: false, stacks: 1,
        });
    }
    out
}

/// Edge the selected unit's plate in crimson; the plate it leaves gets its own edge back.
pub(super) fn target_edge(
    selection: Res<crate::target::Selection>,
    plates: Query<(&PlateOf, &Plate), With<CompactParts>>,
    enemies: Query<&Enemy>,
    mut borders: Query<&mut BorderColor>,
    mut edged: Local<Option<(Entity, Entity)>>,
) {
    let now = selection
        .target
        .and_then(|unit| plates.iter().find(|(of, _)| of.0 == unit))
        .map(|(of, plate)| (of.0, plate.bar));
    if *edged == now {
        return;
    }
    if let Some((unit, bar)) = edged.take() {
        if plates.iter().any(|(of, plate)| of.0 == unit && plate.bar == bar) {
            if let (Ok(en), Ok(mut b)) = (enemies.get(unit), borders.get_mut(bar)) {
                *b = BorderColor::all(compact_edge(en));
            }
        }
    }
    if let Some((_, bar)) = now {
        if let Ok(mut b) = borders.get_mut(bar) {
            *b = BorderColor::all(WINE_HI);
        }
    }
    *edged = now;
}
