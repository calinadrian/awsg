//! The four guardians: each sleeps in a lair at the battleground's
//! edge, north, east, south and west, wakes when the hero comes near and goes home when he runs.
//! A slain guardian drops a free cache and lends its strength to the others; after a while another
//! guardian claims the lair again.

use bevy::prelude::*;

use super::enemies::{self, Enemy};
use super::hero::{self, Hero};
use super::{data, Run, Selection};

/// How far out from the centre the lairs lie, as a share of the leash.
const OUT: f32 = 0.7;
/// Native lairs first fill at 0:20 (ASR.exe RVAs 0x5bf95dd and 0x5c663b2).
const FIRST: f32 = 20.0;
/// Seconds an emptied lair waits before a guardian claims it again.
const REFILL: f32 = 180.0;

struct Lair {
    def: &'static data::EnemyDef,
    pos: Vec3,
    held: Option<Entity>,
    spawned: bool,
    /// The run second the empty lair is claimed again.
    refill_at: f32,
}

/// The run's lairs; each guardian waits for its own ground to stream in.
#[derive(Resource, Default)]
pub(super) struct Lairs {
    lairs: Vec<Lair>,
    /// Guardians slain this run.
    pub slain: u32,
}

impl Lairs {
    /// Where the lairs are and whether a guardian holds each, for the map.
    pub fn marks(&self) -> impl Iterator<Item = (Vec3, bool)> + '_ {
        self.lairs.iter().map(|l| (l.pos, l.held.is_some()))
    }
}

/// North, east, south and west from the centre, in the world's axes (WoW north is Bevy -Z).
const SIDES: [Vec3; 4] = [Vec3::NEG_Z, Vec3::X, Vec3::Z, Vec3::NEG_X];

pub(super) fn keep_lairs(
    mut commands: Commands,
    mut lairs: ResMut<Lairs>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    ground: super::dungeon::Ground,
    hero: Query<&Transform, With<Hero>>,
    guardians: Query<&Enemy>,
    coop: Res<super::coop::Coop>,
) {
    let roster = data::roster(selection.map);
    if roster.guardians.is_empty()
        || super::dungeon::is_dungeon(selection.stage_def())
        || (run.dead_t.is_some() && !coop.living_guests())
    {
        return;
    }
    if hero.single().is_err() {
        return;
    }
    let map = selection.stage_def();
    if lairs.lairs.is_empty() {
        let center = hero::arena(map);
        lairs.lairs = roster
            .guardians
            .iter()
            .zip(SIDES)
            .map(|(def, side)| Lair {
                def,
                pos: center + side * map.leash * OUT,
                held: None,
                spawned: false,
                // Fedfennel keeps the source boss's 9:00 entrance in the elite slot.
                refill_at: if def.name == "Fedfennel" {
                    def.from
                } else {
                    FIRST
                },
            })
            .collect();
    }

    let now = run.elapsed;
    let slain = lairs.slain;
    let center = hero::arena(map);
    let mut fallen = Vec::new();
    for (i, lair) in lairs.lairs.iter_mut().enumerate() {
        match lair.held {
            Some(e) => {
                if guardians.get(e).is_ok_and(|g| !g.dead) {
                    continue;
                }
                lair.held = None;
                lair.refill_at = if lair.def.name == "Fedfennel" {
                    f32::INFINITY // The source encounter occurs once per run.
                } else {
                    now + REFILL
                };
                fallen.push(i);
            }
            None if now >= lair.refill_at => {
                // An unloaded lair must not prevent the other elites from appearing.
                let Some(h) = ground.height(lair.pos) else {
                    continue;
                };
                lair.pos.y = h;
                let e = enemies::spawn_guardian(
                    &mut commands,
                    &mut run,
                    lair.def,
                    lair.pos,
                    center,
                    slain,
                );
                if lair.spawned {
                    run.banner =
                        Some((format!("{} claims its lair once more.", lair.def.name), 4.0));
                }
                lair.spawned = true;
                lair.held = Some(e);
            }
            None => {}
        }
    }
    for &i in &fallen {
        let name = lairs.lairs[i].def.name;
        lairs.slain += 1;
        run.banner = Some((
            format!(
                "{name} is slain! Its lair yields treasure and its strength, and the other guardians grow stronger."
            ),
            5.0,
        ));
    }
}
