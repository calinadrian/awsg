//! Nameplates over the guardians and bosses, drawn as the client draws its V-key plates
//! ([`crate::vplates`]): the hostile red bar under the plate border, the name above it and the
//! level in the border's seat, over the unit's head and sized from the screen diagonal. The
//! horde goes without, so the field stays readable.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;

use super::enemies::Enemy;
use super::hud::{HudFont, UiArt};
use super::{Phase, RunEntity};
use crate::entities::{mount::MountChild, overhead_anchor, BoneAttach, OverheadFallback};

/// How far off a plate still shows.
const PLATE_RANGE: f32 = 70.0;
/// The client's lift over the head anchor (`[0x80abfc]`).
const LIFT: f32 = 2.0 / 3.0;
/// The name's height in gx units, as the client's plate prints it.
const NAME_H: f32 = 0.01;

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
    (
        Text::new(s),
        TextFont {
            font: f.clone(),
            font_size: 11.0,
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
        .spawn(label(f, en.def.name.to_string(), Color::WHITE))
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
        .spawn(label(f, en.def.level.to_string(), Color::srgb(r, g, b)))
        .id();
    let level_seat = commands.spawn(seat()).add_child(level).id();
    commands
        .spawn((
            RunEntity,
            absolute(),
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

/// Give every guardian and boss a plate, seat it over the head and fill its bar; take it away
/// when the unit dies or the run ends.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn enemy_plates(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    ui_scale: Res<UiScale>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    cams: Query<(&Camera, &Transform), (With<WorldCamera>, Without<Enemy>)>,
    plates: Query<(Entity, &Plate)>,
    mut nodes: Query<&mut Node>,
    mut vis: Query<&mut Visibility, With<Plate>>,
    mut texts: Query<&mut TextFont>,
    anchors: (
        Query<&BoneAttach>,
        Query<&benilla_world::rig_anim::RigPose>,
        Query<&OverheadFallback>,
        Query<&GlobalTransform>,
        Query<(), With<MountChild>>,
    ),
    mut by_unit: Local<HashMap<Entity, Entity>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    // Plates whose unit fell or left (or whose run ended) go.
    let in_run = *phase.get() != Phase::Menu;
    by_unit.retain(|unit, plate| {
        let live = in_run && enemies.get(*unit).is_ok_and(|(_, _, e)| !e.dead);
        if !live {
            if let Ok(mut c) = commands.get_entity(*plate) {
                c.despawn();
            }
        }
        live && plates.contains(*plate)
    });
    if !in_run {
        return;
    }
    let (Some(font), Ok((cam, cam_tf))) = (font, cams.single()) else {
        return;
    };
    let Some(viewport) = cam.logical_viewport_size() else {
        return;
    };
    let cam_g = GlobalTransform::from(*cam_tf);
    let s = ui_scale.0;
    let basis = plate_basis(viewport);
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = (gx(PLATE_W), gx(PLATE_H));
    let bar_top = ph - gx(BAR_OFF_Y) - gx(BAR_H);

    for (unit, tf, en) in &enemies {
        if en.dead || !(en.boss || en.home.is_some()) {
            continue;
        }
        let Some(&plate) = by_unit.get(&unit) else {
            let p = spawn_plate(&mut commands, &font.0, en);
            by_unit.insert(unit, p);
            continue;
        };
        let Ok((_, parts)) = plates.get(plate) else {
            continue;
        };
        let head = overhead_anchor(
            unit, tf, &anchors.0, &anchors.1, &anchors.2, &anchors.3, &anchors.4,
        ) + Vec3::Y * LIFT;
        let near = cam_tf.translation.distance(tf.translation) <= PLATE_RANGE;
        let seat = cam
            .world_to_viewport(&cam_g, head)
            .ok()
            .filter(|p| near && p.x >= 0.0 && p.y >= 0.0 && p.x <= viewport.x && p.y <= viewport.y);
        let Ok(mut v) = vis.get_mut(plate) else {
            continue;
        };
        let Some(seat) = seat else {
            *v = Visibility::Hidden;
            continue;
        };
        *v = Visibility::Inherited;
        if let Ok(mut n) = nodes.get_mut(plate) {
            n.width = Val::Px(pw);
            n.height = Val::Px(ph);
            n.left = Val::Px(seat.x / s - pw / 2.0);
            n.top = Val::Px(seat.y / s);
        }
        if let Ok(mut n) = nodes.get_mut(parts.bar) {
            n.left = Val::Px(gx(BAR_OFF_X));
            n.top = Val::Px(bar_top);
            n.width = Val::Px(gx(BAR_W));
            n.height = Val::Px(gx(BAR_H));
        }
        if let Ok(mut n) = nodes.get_mut(parts.fill) {
            n.width = Val::Percent((en.hp / en.max_hp).clamp(0.0, 1.0) * 100.0);
        }
        // The name's bottom on the plate's centre; the level's centre in the border's seat.
        if let Ok(mut n) = nodes.get_mut(parts.name_seat) {
            n.left = Val::Px(pw / 2.0);
            n.top = Val::Px(ph / 2.0);
        }
        if let Ok(mut n) = nodes.get_mut(parts.level_seat) {
            n.left = Val::Px(pw - gx(LEVEL_OFF_X));
            n.top = Val::Px(ph - gx(LEVEL_OFF_Y));
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
}

// --- The boss bar -------------------------------------------------------------------------------

/// How near a guardian or boss is before its bar shows at the top of the screen.
const BAR_RANGE: f32 = 60.0;
const BOSS_BAR_W: f32 = 380.0;
const BOSS_BAR_H: f32 = 20.0;
const YELL_RED: Color = Color::srgb(1.0, 0.25, 0.2);

/// The old ASR's boss bar under the clock: a skull, the name, the health as numbers and a
/// percent, and the latest yell in red above it.
#[derive(Component)]
pub(super) struct BossBar {
    frame: Entity,
    fill: Entity,
    name: Entity,
    health: Entity,
    yell: Entity,
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
            offset: Vec2::new(1.0, 1.0),
            color: Color::BLACK,
        },
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
    )
}

fn spawn_boss_bar(commands: &mut Commands, f: &Handle<Font>) -> Entity {
    let yell = commands.spawn(bar_text(f, 15.0, YELL_RED)).id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.75, 0.05, 0.05)),
        ))
        .id();
    let name = commands.spawn(bar_text(f, 14.0, Color::WHITE)).id();
    let name_box = commands
        .spawn(Node {
            left: Val::Px(8.0),
            top: Val::Px(1.0),
            ..absolute()
        })
        .add_child(name)
        .id();
    let health = commands.spawn(bar_text(f, 14.0, Color::WHITE)).id();
    let health_box = commands
        .spawn(Node {
            right: Val::Px(8.0),
            top: Val::Px(1.0),
            ..absolute()
        })
        .add_child(health)
        .id();
    let frame = commands
        .spawn((
            Node {
                width: Val::Px(BOSS_BAR_W),
                height: Val::Px(BOSS_BAR_H),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.02, 0.02, 0.85)),
            BorderColor::all(Color::srgb(0.72, 0.6, 0.3)),
        ))
        .add_children(&[fill, name_box, health_box])
        .id();
    let skull = commands
        .spawn((
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                margin: UiRect::right(Val::Px(4.0)),
                ..default()
            },
            unpainted(),
            UiArt::crop(nameplate::SKULL_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
        ))
        .id();
    let row = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[skull, frame])
        .id();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(66.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                ..default()
            },
            Visibility::Hidden,
            BossBar {
                frame: row,
                fill,
                name,
                health,
                yell,
            },
        ))
        .add_children(&[yell, row])
        .id()
}

/// `2810` as `2,810`.
fn grouped(n: f32) -> String {
    let s = (n.max(0.0).round() as u64).to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Show the nearest awake guardian's or boss's bar, and the latest yell above it.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn boss_bar(
    mut commands: Commands,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    mut run: ResMut<super::Run>,
    hero: Query<&Transform, (With<super::hero::Hero>, Without<Enemy>)>,
    enemies: Query<(&Transform, &Enemy)>,
    mut bars: Query<(&BossBar, &mut Visibility)>,
    mut shown: Query<&mut Visibility, Without<BossBar>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
) {
    if let Some((_, left)) = run.yell.as_mut() {
        *left -= time.delta_secs();
        if *left <= 0.0 {
            run.yell = None;
        }
    }
    let Ok((bar, mut vis)) = bars.single_mut() else {
        if let Some(font) = font {
            spawn_boss_bar(&mut commands, &font.0);
        }
        return;
    };
    let playing = !matches!(*phase.get(), Phase::Menu | Phase::GameOver);
    let near = hero.single().ok().and_then(|h| {
        enemies
            .iter()
            .filter(|(_, e)| !e.dead && (e.boss || e.home.is_some_and(|h| h.awake)))
            .map(|(tf, e)| (tf.translation.distance(h.translation), e))
            .filter(|(d, _)| *d <= BAR_RANGE)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, e)| e)
    });
    let yell = run.yell.as_ref().map(|(s, _)| s.clone());
    *vis = if playing && (near.is_some() || yell.is_some()) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Ok(mut v) = shown.get_mut(bar.frame) {
        *v = if near.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let set = |texts: &mut Query<&mut Text>, e: Entity, s: String| {
        if let Ok(mut t) = texts.get_mut(e) {
            if t.0 != s {
                t.0 = s;
            }
        }
    };
    set(&mut texts, bar.yell, yell.unwrap_or_default());
    if let Some(e) = near {
        let share = (e.hp / e.max_hp).clamp(0.0, 1.0);
        if let Ok(mut n) = nodes.get_mut(bar.fill) {
            n.width = Val::Percent(share * 100.0);
        }
        set(&mut texts, bar.name, e.def.name.to_string());
        set(
            &mut texts,
            bar.health,
            format!(
                "{} / {}  {:.0}%",
                grouped(e.hp),
                grouped(e.max_hp),
                (share * 100.0).ceil()
            ),
        );
    }
}
