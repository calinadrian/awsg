//! The player's and the target's unit frames: one flat bar each in the boss bars' frame, with the
//! level, name and health in it, the power under it, and the auras on its edges
//! ([`classic`]). The Interface settings pick the health text.
mod classic;
use super::{
    Run,
    enemies::Enemy,
    frame_settings::{self, FrameSettings},
    hud::{HudFont, HudRoot, compact},
    plates::{AuraSlot, fill_slots, hero_auras, spawn_aura_slot},
};
use bevy::prelude::*;

/// The saved CVar behind the Interface tab's Health text row.
pub(super) const HEALTH_TEXT: &str = "SurvivorHudHealthText";
/// The saved CVar behind the Health bar height row, in UI pixels at 100%.
pub(super) const HEALTH_HEIGHT: &str = "SurvivorHudHealthHeight";

/// The health well's height the setting asks for, within what the text still fits.
pub(super) fn health_height(cvars: &crate::cvars::Cvars) -> f32 {
    cvars
        .get(HEALTH_HEIGHT)
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .map_or(classic::HEALTH_H, |v| v.round().clamp(11., 28.))
}

/// The saved CVars behind the Auras tabs: where each kind sits and how big its icons are.
pub(super) const FRAME_BUFFS: &str = "SurvivorHudFrameBuffs";
pub(super) const FRAME_DEBUFFS: &str = "SurvivorHudFrameDebuffs";
pub(super) const FRAME_BUFF_SIZE: &str = "SurvivorHudFrameBuffSize";
pub(super) const FRAME_DEBUFF_SIZE: &str = "SurvivorHudFrameDebuffSize";
pub(super) const PLATE_BUFFS: &str = "SurvivorHudPlateBuffs";
pub(super) const PLATE_DEBUFFS: &str = "SurvivorHudPlateDebuffs";
pub(super) const PLATE_BUFF_SIZE: &str = "SurvivorHudPlateBuffSize";
pub(super) const PLATE_DEBUFF_SIZE: &str = "SurvivorHudPlateDebuffSize";

/// Which edge of a frame or plate a kind of aura sits on, by its setting's value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AuraPos {
    Above,
    Below,
    Hidden,
}

/// Where a frame's or a plate's buffs and debuffs go, and their icons' sizes in UI pixels.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct AuraLayout {
    pub buffs: AuraPos,
    pub debuffs: AuraPos,
    pub buff_size: f32,
    pub debuff_size: f32,
}

impl Default for AuraLayout {
    fn default() -> Self {
        Self::plates(&crate::cvars::Cvars::default())
    }
}

impl AuraLayout {
    fn read(cvars: &crate::cvars::Cvars, keys: [&str; 4], defaults: Self) -> Self {
        let pos = |key: &str, default: AuraPos| match cvars.get(key).and_then(|v| v.parse::<f32>().ok()) {
            Some(v) if v.round() == 0. => AuraPos::Above,
            Some(v) if v.round() == 1. => AuraPos::Below,
            Some(v) if v.round() == 2. => AuraPos::Hidden,
            _ => default,
        };
        let size = |key: &str, default: f32| {
            cvars
                .get(key)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map_or(default, |v| v.round().clamp(6., 32.))
        };
        Self {
            buffs: pos(keys[0], defaults.buffs),
            debuffs: pos(keys[1], defaults.debuffs),
            buff_size: size(keys[2], defaults.buff_size),
            debuff_size: size(keys[3], defaults.debuff_size),
        }
    }

    /// The unit frames': buffs over the plate and debuffs under it, 14px, unless set otherwise.
    pub(super) fn frames(cvars: &crate::cvars::Cvars) -> Self {
        let keys = [FRAME_BUFFS, FRAME_DEBUFFS, FRAME_BUFF_SIZE, FRAME_DEBUFF_SIZE];
        Self::read(cvars, keys, Self {
            buffs: AuraPos::Above,
            debuffs: AuraPos::Below,
            buff_size: 14.,
            debuff_size: 14.,
        })
    }

    /// The compact plates': both kinds sharing the row over the bar, 9px, unless set otherwise.
    pub(super) fn plates(cvars: &crate::cvars::Cvars) -> Self {
        let keys = [PLATE_BUFFS, PLATE_DEBUFFS, PLATE_BUFF_SIZE, PLATE_DEBUFF_SIZE];
        Self::read(cvars, keys, Self {
            buffs: AuraPos::Above,
            debuffs: AuraPos::Above,
            buff_size: 9.,
            debuff_size: 9.,
        })
    }
}

/// What a health text shows, by its setting's value; the nameplates' setting shares it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum HealthText {
    All,
    CurrentMax,
    CurrentPercent,
    Current,
    Percent,
    None,
}

impl HealthText {
    /// The format a setting's value names, `default` when it is unset or unknown.
    pub(super) fn from_setting(value: Option<&str>, default: Self) -> Self {
        match value
            .and_then(|v| v.parse::<f32>().ok())
            .map_or(-1, |v| v.round() as i32)
        {
            0 => Self::All,
            1 => Self::CurrentMax,
            2 => Self::CurrentPercent,
            3 => Self::Current,
            4 => Self::Percent,
            5 => Self::None,
            _ => default,
        }
    }

    /// "225 / 225 · 100%"; past 10,000 the numbers shorten as the boss bars' do, "15.4k".
    pub(super) fn format(self, hp: f32, max: f32) -> String {
        let hp = hp.max(0.).ceil();
        let max = max.max(1.).ceil();
        let (cur, top) = (compact(f64::from(hp)), compact(f64::from(max)));
        let pct = format!("{:.0}%", (hp / max * 100.).clamp(0., 100.).ceil());
        match self {
            Self::All => format!("{cur} / {top} · {pct}"),
            Self::CurrentMax => format!("{cur} / {top}"),
            Self::CurrentPercent => format!("{cur} · {pct}"),
            Self::Current => cur,
            Self::Percent => pct,
            Self::None => String::new(),
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(frame_settings::plugin).add_systems(
        Update,
        (spawn, refresh, refresh_power, refresh_auras).chain(),
    );
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    if std::env::var_os("WOW_SURVIVORS_FRAME_PREVIEW").is_some() {
        app.add_systems(Update, preview_subjects.before(refresh));
    }
}

#[derive(Component)]
struct UnitFrame(bool);
/// The size, health height and auras a frame's designs were laid out with; a change lays them out
/// again.
#[derive(Component)]
struct Built(f32, f32, Option<AuraLayout>);
#[derive(Component)]
struct FrameName(bool);
#[derive(Component)]
struct FrameHealth(bool);
#[derive(Component)]
struct FrameFill(bool);
#[derive(Component)]
struct FrameLevel(bool);
#[derive(Component)]
struct MoveOutline(bool);
#[derive(Component)]
struct DesignRoot(bool, u8);
/// A power well (1) or its fill (2), the player's or the target's.
#[derive(Component)]
struct PowerNode(bool, u8);
/// A frame's buffs and debuffs, on the edges their settings name: each design's plate has its own.
#[derive(Component)]
struct FrameAuras {
    target: bool,
    buffs: Vec<AuraSlot>,
    debuffs: Vec<AuraSlot>,
}

fn seat(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x),
        top: Val::Px(y),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    }
}

fn label(font: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(0., 1.),
            color: Color::srgba(0., 0., 0., 0.95),
        },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
    )
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, Added<HudRoot>>,
    frames: Query<(Entity, &UnitFrame, &Built)>,
    font: Option<Res<HudFont>>,
    settings: Res<FrameSettings>,
    cvars: Res<crate::cvars::Cvars>,
) {
    let Some(font) = font else { return };
    let health_h = health_height(&cvars);
    let auras = AuraLayout::frames(&cvars);
    for root in &roots {
        for target in [false, true] {
            let placement = settings.items[usize::from(target)];
            commands.spawn((
                ChildOf(root),
                UnitFrame(target),
                super::hud_layout::Stacked(frame_settings::STACK_IDS[usize::from(target)]),
                Built(f32::NAN, f32::NAN, None),
                seat(placement.pos.x, placement.pos.y, 0., 0.),
            ));
        }
    }
    // Text under a scaled transform outgrew the box it was measured in, so the size setting lays
    // the frame out again at its size instead.
    for (frame, unit, built) in &frames {
        let scale = settings.items[usize::from(unit.0)].scale;
        if built.0 == scale && built.1 == health_h && built.2 == Some(auras) {
            continue;
        }
        commands
            .entity(frame)
            .despawn_related::<Children>()
            .insert(Built(scale, health_h, Some(auras)))
            .with_children(|frame| build(frame, &font.0, unit.0, scale, health_h, auras));
    }
}

fn build(
    frame: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    target: bool,
    scale: f32,
    health_h: f32,
    auras: AuraLayout,
) {
    let gold = Color::srgb(1., 0.82, 0.);
    let tall = classic::HEIGHT - classic::HEALTH_H + health_h;
    let (w, h) = (classic::WIDTH * scale, tall * scale);
    for design in 0..3u8 {
        // The third design is the same frame a size down, centred where the others sit.
        let s = if design == 2 { scale * 0.85 } else { scale };
        let inset = ((w - classic::WIDTH * s) / 2.).round();
        frame
            .spawn((
                DesignRoot(target, design),
                seat(
                    inset,
                    ((h - tall * s) / 2.).round(),
                    classic::WIDTH * s,
                    tall * s,
                ),
            ))
            .with_children(|skin| classic::spawn(skin, font, target, design, s, health_h, auras));
    }
    frame.spawn((
        MoveOutline(target),
        Node {
            border: UiRect::all(Val::Px(1.)),
            display: Display::None,
            ..seat(0., 0., w, h)
        },
        BorderColor::all(gold),
        ZIndex(8),
    ));
}

/// The player's buffs and the target's buffs and debuffs, as the boss bars show them.
#[allow(clippy::too_many_arguments)]
fn refresh_auras(
    mut commands: Commands,
    run: Res<Run>,
    selection: Res<crate::target::Selection>,
    spells: Option<Res<crate::ui_action::Spells>>,
    frames: Query<(&FrameAuras, &InheritedVisibility)>,
    enemies: Query<(&Enemy, Option<&super::mechanics::Mechanics>)>,
    mut slots: Query<(&mut super::plates::AuraTip, &mut Outline)>,
    mut icons: Query<(&mut super::hud::SpellIcon, &mut ImageNode)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    time: Res<Time>,
    mut since: Local<f32>,
) {
    // Ten times a second is enough for timers shown to a tenth; only the design on screen fills.
    *since += time.delta_secs();
    if *since < 0.1 {
        return;
    }
    *since = 0.0;
    let spell_name = |spell: u32| {
        spells
            .as_deref()
            .and_then(|s| s.catalog.get(spell))
            .map(|d| d.name.clone())
    };
    for (frame, shown) in &frames {
        if !shown.get() {
            continue;
        }
        let all = if frame.target {
            selection
                .target
                .and_then(|unit| {
                    let (en, mech) = enemies.get(unit).ok()?;
                    (!en.dead && en.hp > 0.).then_some((unit, en, mech))
                })
                .map(|(unit, en, mech)| {
                    let mark = run.passives.marks.get(&unit);
                    super::mechanics::auras(en, mech, mark, run.elapsed, spell_name)
                })
                .unwrap_or_default()
        } else {
            hero_auras(&run, |id, fallback| {
                spell_name(id).unwrap_or_else(|| fallback.replace('_', " "))
            })
        };
        let (buffs, debuffs): (Vec<_>, Vec<_>) = all.into_iter().partition(|a| a.buff);
        let edge = super::right_column::STEEL_LO;
        let red = super::right_column::WINE_HI;
        fill_slots(
            &mut commands,
            &frame.buffs,
            buffs,
            edge,
            &mut slots,
            &mut icons,
            &mut nodes,
            &mut texts,
        );
        fill_slots(
            &mut commands,
            &frame.debuffs,
            debuffs,
            red,
            &mut slots,
            &mut icons,
            &mut nodes,
            &mut texts,
        );
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh(
    run: Res<Run>,
    coop: Res<super::coop::Coop>,
    selection: Res<crate::target::Selection>,
    enemies: Query<&Enemy>,
    mut frames: Query<
        (&UnitFrame, &mut Node),
        (
            Without<FrameFill>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
    mut fills: Query<
        (&FrameFill, &mut Node),
        (
            Without<UnitFrame>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
    mut outlines: Query<
        (&MoveOutline, &mut Node),
        (Without<UnitFrame>, Without<FrameFill>, Without<DesignRoot>),
    >,
    mut designs: Query<
        (&DesignRoot, &mut Node),
        (Without<UnitFrame>, Without<FrameFill>, Without<MoveOutline>),
    >,
    mut names: Query<(&FrameName, &mut Text), (Without<FrameHealth>, Without<FrameLevel>)>,
    mut health: Query<
        (&FrameHealth, &mut Text, &mut Visibility),
        (Without<FrameName>, Without<FrameLevel>),
    >,
    mut levels: Query<
        (&FrameLevel, &mut Text, &mut Visibility),
        (Without<FrameName>, Without<FrameHealth>),
    >,
    settings: Res<FrameSettings>,
    cvars: Res<crate::cvars::Cvars>,
) {
    let format = HealthText::from_setting(cvars.get(HEALTH_TEXT), HealthText::All);
    let target = selection
        .target
        .and_then(|e| enemies.get(e).ok())
        .filter(|e| !e.dead && e.hp > 0.);
    let max = run.stats().max_hp;
    for (frame, mut node) in &mut frames {
        let placement = settings.items[usize::from(frame.0)];
        node.left = Val::Px(placement.pos.x);
        node.top = Val::Px(placement.pos.y);
        node.display = if frame.0 && target.is_none() && placement.locked {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (design, mut node) in &mut designs {
        node.display = if settings.items[usize::from(design.0)].design == design.1 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (outline, mut node) in &mut outlines {
        node.display = if settings.items[usize::from(outline.0)].locked {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (fill, mut node) in &mut fills {
        let (hp, max) = if fill.0 {
            target.map_or((0., 1.), |e| (e.hp, e.max_hp))
        } else {
            (run.hp, max)
        };
        node.width = Val::Percent((hp / max.max(1.)).clamp(0., 1.) * 100.);
    }
    for (name, mut text) in &mut names {
        let value = if name.0 {
            target.map_or("Select a target", |e| e.def.name)
        } else {
            if coop.running && !coop.name.trim().is_empty() {
                coop.name.trim()
            } else {
                run.class.name
            }
        };
        if text.0 != value {
            text.0 = value.to_string();
        }
    }
    for (value, mut text, mut visible) in &mut health {
        *visible = if settings.items[usize::from(value.0)].health {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let (hp, max) = if value.0 {
            target.map_or((0., 1.), |e| (e.hp, e.max_hp))
        } else {
            (run.hp, max)
        };
        let want = format.format(hp, max);
        if text.0 != want {
            text.0 = want;
        }
    }
    for (level, mut text, mut visible) in &mut levels {
        *visible = if settings.items[usize::from(level.0)].level {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        text.0 = if level.0 {
            target.map_or(String::new(), |e| e.level.to_string())
        } else {
            run.level.to_string()
        };
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh_power(
    selection: Res<crate::target::Selection>,
    enemies: Query<&Enemy>,
    hero: Query<&crate::net::ObjectStore, With<super::hero::Hero>>,
    stores: Query<&crate::net::ObjectStore>,
    mut power_nodes: Query<
        (&PowerNode, &mut Node, Option<&mut BackgroundColor>),
        (
            Without<UnitFrame>,
            Without<FrameFill>,
            Without<MoveOutline>,
            Without<DesignRoot>,
        ),
    >,
) {
    let target = selection
        .target
        .and_then(|e| enemies.get(e).ok())
        .filter(|e| !e.dead && e.hp > 0.);
    let resource = |store: Option<&crate::net::ObjectStore>| {
        let Some(store) = store else {
            return (0, 0, Color::NONE);
        };
        let kind = store.0.unit_power_type();
        let current = store.0.unit_shown_power(kind).unwrap_or(0);
        let maximum = store.0.unit_shown_max_power(kind).unwrap_or(0);
        let color = match kind {
            1 => Color::srgb(0.70, 0.14, 0.10),
            2 => Color::srgb(0.85, 0.45, 0.14),
            3 => Color::srgb(0.89, 0.72, 0.11),
            _ => Color::srgb(0.11, 0.36, 0.88),
        };
        (current, maximum, color)
    };
    let player_power = resource(hero.single().ok());
    let target_power = resource(
        selection
            .target
            .filter(|_| target.is_some())
            .and_then(|e| stores.get(e).ok()),
    );
    for (part, mut node, color) in &mut power_nodes {
        let (value, maximum, tint) = if part.0 { target_power } else { player_power };
        // A unit with no power keeps its well, empty, so both frames are one height.
        match part.1 {
            1 => {}
            _ => {
                node.width = Val::Percent(if maximum > 0 {
                    (value as f32 / maximum as f32).clamp(0., 1.) * 100.
                } else {
                    0.
                });
                if let Some(mut color) = color {
                    color.0 = tint;
                }
            }
        }
    }
}

/// Opt-in visual QA with real local units, usable with the existing screenshot runner.
#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
fn preview_subjects(
    mut commands: Commands,
    mut selection: ResMut<crate::target::Selection>,
    hero: Query<Entity, (With<super::hero::Hero>, Without<crate::net::SelfPlayer>)>,
    enemies: Query<(Entity, &Enemy, &crate::net::Guid)>,
) {
    for body in &hero {
        commands.entity(body).insert(crate::net::SelfPlayer);
    }
    if let Some((entity, _, guid)) = enemies
        .iter()
        .filter(|(_, e, _)| !e.dead && e.hp > 0.)
        .max_by(|(_, a, _), (_, b, _)| a.max_hp.total_cmp(&b.max_hp))
    {
        selection.target = Some(entity);
        selection.guid = Some(guid.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_text_follows_the_setting_and_shortens_big_numbers() {
        let all = HealthText::from_setting(None, HealthText::All);
        assert_eq!(all, HealthText::All);
        assert_eq!(all.format(225., 225.), "225 / 225 · 100%");
        assert_eq!(all.format(15435., 16974.), "15.4k / 17.0k · 91%");
        assert_eq!(
            HealthText::from_setting(Some("1"), HealthText::All).format(131., 182.),
            "131 / 182"
        );
        assert_eq!(
            HealthText::from_setting(Some("2"), HealthText::All).format(131., 182.),
            "131 · 72%"
        );
        assert_eq!(
            HealthText::from_setting(Some("3"), HealthText::All).format(131.4, 182.),
            "132"
        );
        assert_eq!(HealthText::from_setting(Some("4"), HealthText::All).format(0.2, 182.), "1%");
        assert_eq!(HealthText::from_setting(Some("5"), HealthText::All).format(131., 182.), "");
        assert_eq!(HealthText::from_setting(Some("9"), HealthText::All), HealthText::All);
    }
}
