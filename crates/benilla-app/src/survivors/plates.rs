//! Nameplates for major encounters and optional V-key plates for enemies in view, drawn like the
//! client plates ([`crate::vplates`]): a hostile bar, name and level over each unit's head.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;

use super::enemies::Enemy;
use super::hud::{HudFont, UiArt};
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

/// The settings page's equivalent of the two FrameXML nameplate toggles. The Survivors world does
/// not load FrameXML, so these are persisted alongside the client's CVars.
#[derive(Resource, Default)]
pub(super) struct NameplateSettings {
    pub enemies: bool,
    pub friends: bool,
}

pub(super) fn load_nameplate_settings(
    cvars: Res<crate::cvars::Cvars>,
    mut settings: ResMut<NameplateSettings>,
) {
    settings.enemies = cvars.flag("SurvivorNameplateEnemies").unwrap_or(false);
    settings.friends = cvars.flag("SurvivorNameplateFriends").unwrap_or(true);
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
        };
        toggle_nameplates(&mut s, false, false);
        assert!(s.enemies && s.friends, "V leaves ally plates alone");
        toggle_nameplates(&mut s, false, true);
        assert!(s.enemies && !s.friends, "Shift+V leaves enemy plates alone");
        toggle_nameplates(&mut s, true, false);
        assert!(s.enemies && s.friends, "Ctrl+V shows both when either is off");
        toggle_nameplates(&mut s, true, false);
        assert!(!s.enemies && !s.friends, "Ctrl+V hides both when both are on");
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

/// Keep encounter plates visible and add ordinary enemy plates while the WoW-style V toggle is on.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn enemy_plates(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    settings: Res<NameplateSettings>,
    font: Option<Res<HudFont>>,
    ui_scale: Res<UiScale>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    hero: Query<&Transform, (With<super::hero::Hero>, Without<Enemy>)>,
    cams:Query<(&Camera, &Transform), (With<WorldCamera>, Without<Enemy>)>,
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
    mut bucket: Local<crate::smart_rect::SmartBucket>,
    hud: Query<(&ComputedNode, &bevy::ui::UiGlobalTransform), With<super::hud::TopStack>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    // Plates whose unit fell or left (or whose run ended) go.
    bucket.clear();
    let in_run = *phase.get() != Phase::Menu;
    by_unit.retain(|unit, plate| {
        let live = in_run
            && enemies
                .get(*unit)
                .is_ok_and(|(_, _, e)| !e.dead && (e.boss || e.home.is_some() || settings.enemies));
        if !live {
            if let Ok(mut c) = commands.get_entity(*plate) {
                c.despawn();
            }
        }
        live && plates.contains(*plate)
    });
    // Start hidden and reveal only this frame's projected candidates; otherwise an off-screen or
    // temporarily unprojectable unit leaves its last frame's plate stranded at stale coordinates.
    for mut visibility in &mut vis {
        *visibility = Visibility::Hidden;
    }
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
            encounters.push((unit, en, screen));
        } else {
            nearby.push((dist_sq, (unit, en, screen)));
        }
    }
    // A packed horde still shows only its nearest few; each plate costs the overlap solver a
    // pass over every plate seated before it.
    nearby.sort_by(|a, b| a.0.total_cmp(&b.0));
    nearby.truncate(MAX_NEARBY);
    let mut nearby: Vec<_> = nearby.into_iter().map(|(_, c)| c).collect();
    // Entity order makes the overlap solver's priority stable; query iteration order cannot
    // reshuffle a packed group from frame to frame. Encounters claim their seats first, so the
    // horde's plates make way for them rather than the other way round.
    encounters.sort_by_key(|(unit, _, _)| unit.index());
    nearby.sort_by_key(|(unit, _, _)| unit.index());
    // An ordinary unit that left the reach or the nearest few gives its plate back.
    by_unit.retain(|unit, plate| {
        let keep = enemies
            .get(*unit)
            .is_ok_and(|(_, _, e)| e.boss || e.home.is_some())
            || nearby.iter().any(|(u, _, _)| u == unit);
        if !keep {
            if let Ok(mut c) = commands.get_entity(*plate) {
                c.despawn();
            }
        }
        keep
    });

    for (unit, en, screen) in encounters.into_iter().chain(nearby) {
        let Some(&plate) = by_unit.get(&unit) else {
            let p = spawn_plate(&mut commands, &font.0, en);
            by_unit.insert(unit, p);
            continue;
        };
        let Ok((_, parts)) = plates.get(plate) else {
            continue;
        };
        let Ok(mut v) = vis.get_mut(plate) else {
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
        *v = Visibility::Inherited;
        if let Ok(mut n) = nodes.get_mut(plate) {
            n.width = Val::Px(pw);
            n.height = Val::Px(ph);
            n.left = Val::Px(seated.min.x);
            n.top = Val::Px(seated.min.y);
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

/// The old ASR's boss bar under the clock, one row per engaged boss or elite: a skull, the name,
/// the health as numbers and a percent, and the latest yell in red above them.
#[derive(Component)]
pub(super) struct BossBar {
    rows: Vec<BarRow>,
    yell: Entity,
}

struct BarRow {
    frame: Entity,
    fill: Entity,
    name: Entity,
    health: Entity,
}

/// At most this many bars stack under the clock; bosses take the first rows.
const MAX_BARS: usize = 5;

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

fn spawn_bar_row(commands: &mut Commands, f: &Handle<Font>) -> BarRow {
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
    let bar = commands
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
    let frame = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .add_children(&[skull, bar])
        .id();
    BarRow {
        frame,
        fill,
        name,
        health,
    }
}

fn spawn_boss_bar(commands: &mut Commands, f: &Handle<Font>) -> Entity {
    let yell = commands.spawn(bar_text(f, 15.0, YELL_RED)).id();
    let rows: Vec<BarRow> = (0..MAX_BARS).map(|_| spawn_bar_row(commands, f)).collect();
    let mut children = vec![yell];
    children.extend(rows.iter().map(|row| row.frame));
    // In the clock's column ([`super::hud::TopStack`]), so it always starts under the run's line.
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                ..default()
            },
            Visibility::Hidden,
            BossBar { rows, yell },
        ))
        .add_children(&children)
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

/// Whether an enemy is in a fight worth a bar: a boss, an awake lair guardian, or a roaming
/// elite that has been hit.
fn engaged(e: &Enemy) -> bool {
    e.boss
        || e.home.is_some_and(|h| h.awake)
        || (e.def.roaming_elite && e.hp < e.max_hp)
}

/// Show a bar for every engaged boss and elite in range, bosses first and then the nearest,
/// and the latest yell above them.
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
    stack: Query<Entity, With<super::hud::TopStack>>,
) {
    if let Some((_, left)) = run.yell.as_mut() {
        *left -= time.delta_secs();
        if *left <= 0.0 {
            run.yell = None;
        }
    }
    let Ok((bar, mut vis)) = bars.single_mut() else {
        if let (Some(font), Ok(stack)) = (font, stack.single()) {
            let bar = spawn_boss_bar(&mut commands, &font.0);
            commands.entity(stack).add_child(bar);
        }
        return;
    };
    let playing = !matches!(*phase.get(), Phase::Menu | Phase::GameOver);
    let mut near: Vec<(bool, f32, &Enemy)> = hero
        .single()
        .map(|h| {
            enemies
                .iter()
                .filter(|(_, e)| !e.dead && engaged(e))
                .map(|(tf, e)| (!e.boss, tf.translation.distance(h.translation), e))
                .filter(|(_, d, _)| *d <= BAR_RANGE)
                .collect()
        })
        .unwrap_or_default();
    near.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    near.truncate(bar.rows.len());
    let yell = run.yell.as_ref().map(|(s, _)| s.clone());
    *vis = if playing && (!near.is_empty() || yell.is_some()) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let set = |texts: &mut Query<&mut Text>, e: Entity, s: String| {
        if let Ok(mut t) = texts.get_mut(e) {
            if t.0 != s {
                t.0 = s;
            }
        }
    };
    set(&mut texts, bar.yell, yell.unwrap_or_default());
    for (i, row) in bar.rows.iter().enumerate() {
        let entry = near.get(i);
        if let Ok(mut v) = shown.get_mut(row.frame) {
            let want = if entry.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *v != want {
                *v = want;
            }
        }
        if let Ok(mut n) = nodes.get_mut(row.frame) {
            let want = if entry.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if n.display != want {
                n.display = want;
            }
        }
        let Some((_, _, e)) = entry else { continue };
        let share = (e.hp / e.max_hp).clamp(0.0, 1.0);
        if let Ok(mut n) = nodes.get_mut(row.fill) {
            n.width = Val::Percent(share * 100.0);
        }
        set(&mut texts, row.name, super::mechanics::bar_name(e));
        set(
            &mut texts,
            row.health,
            format!(
                "{} / {}  {:.0}%",
                grouped(e.hp),
                grouped(e.max_hp),
                (share * 100.0).ceil()
            ),
        );
    }
}
