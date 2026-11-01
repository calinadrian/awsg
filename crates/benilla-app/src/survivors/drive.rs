//! The hero under WoW's own player controller ([`crate::player`]): its movement, turning, strafing,
//! jumping, mouselook, camera and its capsule against the world's colliders. Survivors runs with
//! FrameXML off, so `Bindings.xml` never runs; the keys set the controller's input bits here
//! instead, and the hero is made our own body ([`SelfPlayer`]) so the controller drives it.
//!
//! The controller runs only while a run is being played ([`SelfDriven`]): the menu, the level-up
//! pick, the pause and the results keep the cursor and the camera still.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::hero::{self, Hero};
use super::{Phase, Run, Selection};
use crate::bindings::{BindingsState, Input};
use crate::net::{Guid, SelfGuid, SelfPlayer, UnitSpeeds};
use crate::player::Player;
use crate::run_mode::SelfDriven;
use benilla_protocol::events::MoveSpeeds;

/// A new run must stream its destination before handing its hero to the controller.
#[derive(Resource, Default)]
pub(super) struct EntryPending(pub bool);

/// The held bits and the keys that set them, as the stock bindings have it.
const HELD: [(Input, [KeyCode; 2]); 6] = [
    (Input::MoveForward, [KeyCode::KeyW, KeyCode::ArrowUp]),
    (Input::MoveBackward, [KeyCode::KeyS, KeyCode::ArrowDown]),
    (Input::TurnLeft, [KeyCode::KeyA, KeyCode::ArrowLeft]),
    (Input::TurnRight, [KeyCode::KeyD, KeyCode::ArrowRight]),
    (Input::StrafeLeft, [KeyCode::KeyQ, KeyCode::KeyQ]),
    (Input::StrafeRight, [KeyCode::KeyE, KeyCode::KeyE]),
];

/// The client's default speeds (`MoveSpeeds` in the LIVING block) for a run speed of 7.
const WALK: f32 = 2.5;
const RUN_BACK: f32 = 4.5;
const SWIM: f32 = 4.722_222;
const SWIM_BACK: f32 = 2.5;

/// Whether the controller should be driving the hero now: a run in play, alive, by hand.
pub(super) fn driving(phase: &State<Phase>, run: &Run, auto: bool) -> bool {
    *phase.get() == Phase::Playing && run.dead_t.is_none() && !auto
}

/// Hand the hero to the controller while a run is played, and take it back otherwise; feed the
/// keys into its input bits and the run's speed into its speeds.
#[allow(clippy::too_many_arguments)]
pub(super) fn take_reins(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    auto: Option<Res<super::auto::Auto>>,
    driven: Option<Res<SelfDriven>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut binds: ResMut<BindingsState>,
    mut player: ResMut<Player>,
    mut self_guid: ResMut<SelfGuid>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    ground: avian3d::prelude::SpatialQuery,
    mut hero: Query<(Entity, &Guid, Option<&UnitSpeeds>, &mut Transform), With<Hero>>,
    mut seated: Local<Option<Entity>>,
    mut entry: ResMut<EntryPending>,
    progress: Res<benilla_world::terrain_stream::WorldLoadProgress>,
) {
    let want = driving(&phase, &run, auto.is_some());
    let Some((body, guid, speeds, mut tf)) = hero.single_mut().ok().filter(|_| want) else {
        if driven.is_some() {
            commands.remove_resource::<SelfDriven>();
            for (input, _) in HELD {
                binds.set_held(input, false);
            }
            // A look in progress hid and held the cursor: the picker and the menu need it back.
            if let Ok(mut c) = cursor.single_mut() {
                if !c.visible || c.grab_mode != CursorGrabMode::None {
                    c.visible = true;
                    c.grab_mode = CursorGrabMode::None;
                }
            }
        }
        wheel.clear();
        return;
    };
    // A new run's hero: make it our body, and let the controller seat itself on it afresh (its
    // take-control edge adopts the body's pose and puts the camera behind it).
    if *seated != Some(body) || entry.0 {
        let wow = benilla_assets::coords::bevy_to_wow(tf.translation);
        let (tx, ty) = benilla_formats::world_to_tile(wow[0], wow[1]);
        if !progress.focus_resident
            || !progress.presentable()
            || progress.focus_tile != Some((tx as i32, ty as i32))
        {
            return;
        }
        // Stand him on the collider, not the drawn height: the two part by a yard or so, and the
        // ground is one-sided, so a capsule seated under it falls through the world. Looked for
        // from just above his feet, never from the sky, which underground finds the cave's roof.
        // No collider yet means the floor is still attaching; wait for it.
        let from = tf.translation + Vec3::Y * SUNK_REACH;
        let Some(hit) = ground.cast_ray(
            from,
            Dir3::NEG_Y,
            SUNK_REACH * 4.0,
            true,
            &benilla_world::collision::WorldCollision::body_filter(),
        ) else {
            return;
        };
        tf.translation.y = from.y - hit.distance + 0.05;
        *seated = Some(body);
        commands.entity(body).insert(SelfPlayer);
        self_guid.0 = Some(guid.0);
        *player = Player::default();
        entry.0 = false;
    }
    if driven.is_none() {
        commands.insert_resource(SelfDriven);
    }
    for (input, pair) in HELD {
        binds.set_held(input, keys.any_pressed(pair));
    }
    if keys.just_pressed(KeyCode::Space) {
        binds.fire(Input::Jump, 1.0);
    }
    let notches: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y,
            MouseScrollUnit::Pixel => w.y * 0.02,
        })
        .sum();
    if notches > 0.0 {
        binds.fire(Input::CameraZoomIn, notches);
    } else if notches < 0.0 {
        binds.fire(Input::CameraZoomOut, -notches);
    }
    // The run's speed (passives, haste) as the body's own speeds, the rest scaled alongside.
    let run_speed = run.stats().speed;
    let k = run_speed / super::BASE_SPEED;
    let want_speeds = MoveSpeeds {
        walk: WALK * k,
        run: run_speed,
        run_back: RUN_BACK * k,
        swim: SWIM * k,
        swim_back: SWIM_BACK * k,
        turn_rate: std::f32::consts::PI,
    };
    if speeds.is_none_or(|s| s.0 != want_speeds) {
        commands.entity(body).insert(UnitSpeeds(want_speeds));
    }
}

/// How far into the ground the feet may sink before [`hold_leash`] lifts them back out (yd).
const SUNK_REACH: f32 = 2.0;
/// The steepest ground [`hold_leash`] lifts the feet onto: a normal's up part, about 50 degrees.
const WALKABLE: f32 = 0.64;
/// How far under a hero below a dungeon's floor grid [`hold_leash`] looks for a lower floor (yd).
const FALL_REACH: f32 = 200.0;

/// Keep the hero inside the battleground's leash after the controller has moved him, and on top of
/// the ground: a tile's collider can attach a yard above where he stands, and the one-sided ground
/// then lets him fall through the world.
pub(super) fn hold_leash(
    driven: Option<Res<SelfDriven>>,
    selection: Res<Selection>,
    ground: avian3d::prelude::SpatialQuery,
    world: super::dungeon::Ground,
    mut player: ResMut<Player>,
    mut hero: Query<&mut Transform, With<Hero>>,
) {
    // Not before the controller has taken the body: until then `player.pos` is the reset default,
    // which reads as far outside the leash and would drag the hero to its edge.
    if driven.is_none() || !player.active {
        return;
    }
    let map = selection.stage_def();
    let center = hero::arena(map);
    let off = super::units::flat(player.pos - center);
    if off.length() > map.leash {
        let held = center + off.normalize() * map.leash;
        player.pos.x = held.x;
        player.pos.z = held.z;
    }
    let filter = benilla_world::collision::WorldCollision::body_filter();
    // Fallen out of reach: back over the drawn ground, to land on whatever is there. Underground
    // the floor grid holds one floor per spot, so a floor below it is real ground he stands on:
    // there only nothing at all under him is a fall.
    if let Some(h) = world.height(player.pos) {
        let fell = player.pos.y < h - SUNK_REACH * 2.0
            && (!super::dungeon::is_dungeon(map)
                || ground
                    .cast_ray(player.pos, Dir3::NEG_Y, FALL_REACH, true, &filter)
                    .is_none());
        if fell {
            player.pos.y = h + SUNK_REACH;
        }
    }
    // Sunk into the ground: up onto its surface. Only when nothing holds his feet (the ground is
    // one-sided, so from under it the floor is not found) and only onto walkable ground: pressed
    // against a leaning cave wall the ray from above his head meets the wall, and lifting onto it
    // each frame would climb him to its top.
    let standing = ground
        .cast_ray(player.pos + Vec3::Y * 0.1, Dir3::NEG_Y, 0.4, true, &filter)
        .is_some();
    let from = player.pos + Vec3::Y * SUNK_REACH;
    if let Some(hit) = ground
        .cast_ray(from, Dir3::NEG_Y, SUNK_REACH, true, &filter)
        .filter(|hit| !standing && hit.normal.y >= WALKABLE)
    {
        let top = from.y - hit.distance;
        if top > player.pos.y + 0.02 {
            player.pos.y = top + 0.05;
        }
    }
    if let Ok(mut tf) = hero.single_mut() {
        if tf.translation != player.pos {
            tf.translation = player.pos;
        }
    }
}

/// A run starts on the battleground's centre, on its floor: between runs the menu only pulls the
/// hero back to the leash, which underground can leave him in rock.
pub(super) fn start_on_center(
    mut commands: Commands,
    selection: Res<Selection>,
    world: super::dungeon::Ground,
    mut hero: Query<&mut Transform, With<Hero>>,
    mut current: ResMut<benilla_world::world_map::CurrentMap>,
    mut player: ResMut<Player>,
    mut focus: ResMut<benilla_world::terrain_stream::ViewFocus>,
    mut progress: ResMut<benilla_world::terrain_stream::WorldLoadProgress>,
    mut entry: ResMut<EntryPending>,
    auto: Option<Res<super::auto::Auto>>,
) {
    let center = hero::arena(selection.stage_def());
    // Never sample the departed map's terrain at the new map's coordinates.
    let pos = if current.0 == selection.stage_def().map {
        world.at(center, center.y)
    } else {
        center
    };
    current.0 = selection.stage_def().map;
    *player = Player::default();
    player.active = true;
    player.pos = pos;
    *focus = benilla_world::terrain_stream::ViewFocus::body(
        benilla_assets::coords::bevy_to_wow(pos),
        false,
    );
    *progress = benilla_world::terrain_stream::WorldLoadProgress::default();
    entry.0 = auto.is_none();
    commands.remove_resource::<SelfDriven>();
    if let Ok(mut tf) = hero.single_mut() {
        tf.translation = pos;
    }
}
