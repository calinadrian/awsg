//! The dungeon battlegrounds' ground: The Deadmines and Wailing Caverns stand on cave floors the
//! terrain height lookup knows nothing of. Each carries the old ASR's checked floor grid (2 yd
//! cells of the lowest dry floor reachable from the start, built from the vanilla WMOs by its
//! `deadmines_ground.py` and `wailing_ground.py`) and its play area, so spawns land on the floor
//! of the arena and never in rock, on a roof or past a barricade.

use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_world::world_point::WorldPoint;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::{Selection, data::MapDef};

/// No nearest spawnable cell.
const NONE: u16 = 0xffff;

/// A floor grid in WoW coordinates: `x0 + tx * cell`, `y0 + ty * cell`, row-major by `tx`.
struct Grid {
    x0: f32,
    y0: f32,
    cell: f32,
    w: usize,
    h: usize,
    /// The floor height per cell, NaN off the floor; then its target's height answers instead.
    heights: Vec<f32>,
    /// Per cell, the nearest spawnable cell (`(tx, ty)`); a spawnable cell names itself.
    targets: Vec<(u16, u16)>,
}

impl Grid {
    /// Unpack an old ASR ground file: `w * h` f32 heights, then `w * h` u16 pairs.
    fn parse(raw: &[u8], x0: f32, y0: f32, cell: f32, w: usize, h: usize) -> Self {
        let n = w * h;
        assert_eq!(raw.len(), n * 8, "ground grid size");
        let f32_at = |i: usize| f32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap());
        let u16_at = |i: usize| {
            let o = n * 4 + i * 2;
            u16::from_le_bytes(raw[o..o + 2].try_into().unwrap())
        };
        Self {
            x0,
            y0,
            cell,
            w,
            h,
            heights: (0..n).map(f32_at).collect(),
            targets: (0..n).map(|i| (u16_at(i * 2), u16_at(i * 2 + 1))).collect(),
        }
    }

    fn index(&self, x: f32, y: f32) -> Option<usize> {
        let tx = ((x - self.x0) / self.cell).floor();
        let ty = ((y - self.y0) / self.cell).floor();
        (tx >= 0.0 && ty >= 0.0 && (tx as usize) < self.w && (ty as usize) < self.h)
            .then(|| tx as usize * self.h + ty as usize)
    }

    fn target(&self, i: usize) -> Option<usize> {
        let (tx, ty) = self.targets[i];
        (tx != NONE && ty != NONE).then(|| tx as usize * self.h + ty as usize)
    }

    /// The floor under a WoW `(x, y)`; off the floor, its nearest spawnable cell's.
    fn height(&self, x: f32, y: f32) -> Option<f32> {
        let i = self.index(x, y)?;
        let z = self.heights[i];
        if z.is_finite() {
            return Some(z);
        }
        self.target(i).map(|t| self.heights[t])
    }

    /// The centre and floor of the spawnable cell nearest a WoW `(x, y)`.
    fn spawn_spot(&self, x: f32, y: f32) -> Option<[f32; 3]> {
        let t = self.target(self.index(x, y)?)?;
        let (tx, ty) = (t / self.h, t % self.h);
        Some([
            self.x0 + (tx as f32 + 0.5) * self.cell,
            self.y0 + (ty as f32 + 0.5) * self.cell,
            self.heights[t],
        ])
    }
}

/// Where a dungeon's enemies may stand, beside its leash circle.
enum PlayArea {
    /// Inside this outline (WoW `(x, y)`): the Mast Room and the western cave, whose two exits
    /// carry the timber barricades.
    Outline(&'static [[f32; 2]]),
    /// North of this line (WoW `y`): the Pit of Fangs, on the arena side of the lower barricade.
    NorthOf(f32),
}

impl PlayArea {
    fn contains(&self, x: f32, y: f32, margin: f32) -> bool {
        match self {
            Self::NorthOf(line) => y >= line + margin,
            Self::Outline(points) => {
                let (mut odd, mut nearest) = (false, f32::INFINITY);
                for (i, a) in points.iter().enumerate() {
                    let b = points[(i + 1) % points.len()];
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let t =
                        (((x - a[0]) * dx + (y - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                    nearest = nearest.min((x - a[0] - t * dx).hypot(y - a[1] - t * dy));
                    if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) * dx / dy {
                        odd = !odd;
                    }
                }
                odd && nearest >= margin
            }
        }
    }
}

/// The Deadmines' outline as the old ASR's zone table carries it (`deadmines_arena.outline()`).
const DEADMINES_OUTLINE: [[f32; 2]; 21] = [
    [-264.228, -473.687],
    [-317.31, -472.76],
    [-318.453, -538.26],
    [-305.455, -538.487],
    [-305.006, -575.791],
    [-293.456, -584.494],
    [-276.283, -586.194],
    [-275.46, -539.011],
    [-265.371, -539.187],
    [-264.542, -491.684],
    [-255.633, -491.84],
    [-251.721, -496.909],
    [-247.669, -493.979],
    [-233.672, -494.223],
    [-226.832, -498.303],
    [-213.788, -490.19],
    [-221.446, -481.435],
    [-230.34, -475.279],
    [-243.269, -471.052],
    [-251.232, -468.913],
    [-258.301, -472.79],
];

struct Dungeon {
    grid: Grid,
    area: PlayArea,
}

static DEADMINES: std::sync::LazyLock<Dungeon> = std::sync::LazyLock::new(|| Dungeon {
    grid: Grid::parse(
        include_bytes!("ground/deadmines.ground"),
        -340.0,
        -1020.0,
        2.0,
        200,
        370,
    ),
    area: PlayArea::Outline(&DEADMINES_OUTLINE),
});

static WAILING: std::sync::LazyLock<Dungeon> = std::sync::LazyLock::new(|| Dungeon {
    grid: Grid::parse(
        include_bytes!("ground/wailing.ground"),
        -180.0,
        340.0,
        2.0,
        130,
        120,
    ),
    area: PlayArea::NorthOf(372.0),
});

// The generated floor stays in the user's ignored MPQ, never in the source tree.
static THANES: std::sync::LazyLock<Option<Dungeon>> = std::sync::LazyLock::new(|| {
    let data = benilla_formats::wow_data()?;
    let mut chain = benilla_formats::Chain::open(&data).ok()?;
    let raw = chain
        .read_file("World\\ASRForeverThanes\\arena.ground")
        .ok()?;
    if raw.len() != 110 * 120 * 8 {
        return None;
    }
    let grid = Grid::parse(&raw, 0.0, -60.0, 1.0, 110, 120);
    // Validate target indices before the grid may be queried.
    if grid.targets.iter().any(|&(x, y)| x >= 110 || y >= 120)
        || grid
            .targets
            .iter()
            .any(|&(x, y)| !grid.heights[x as usize * 120 + y as usize].is_finite())
    {
        return None;
    }
    Some(Dungeon {
        grid,
        area: PlayArea::Outline(&[[10.0, -50.0], [98.0, -50.0], [98.0, 40.0], [10.0, 40.0]]),
    })
});

pub(super) fn available(map: &MapDef) -> bool {
    map.map != 3065 || THANES.is_some()
}

/// The `Map.dbc` rows of the dungeon battlegrounds.
const DEADMINES_MAP: u32 = 36;
const WAILING_MAP: u32 = 43;

fn of(map: &MapDef) -> Option<&'static Dungeon> {
    match map.map {
        DEADMINES_MAP => Some(&DEADMINES),
        WAILING_MAP => Some(&WAILING),
        3065 => THANES.as_ref(),
        _ => None,
    }
}

/// Whether a battleground is fought underground, with no terrain and no sky to ring with fire.
pub(super) fn is_dungeon(map: &MapDef) -> bool {
    map.map == 3065 || story_instance(map) || of(map).is_some()
}

fn story_instance(map: &MapDef) -> bool {
    matches!(map.map, 189 | 209 | 249 | 409)
}

/// The ground of the battleground being stood in: a dungeon's floor grid, else the terrain.
#[derive(SystemParam)]
pub(super) struct Ground<'w, 's> {
    world: WorldPoint<'w, 's>,
    selection: Res<'w, Selection>,
    collision: avian3d::prelude::SpatialQuery<'w, 's>,
}

impl Ground<'_, '_> {
    /// The ground height under a Bevy point.
    pub fn height(&self, p: Vec3) -> Option<f32> {
        if story_instance(self.selection.stage_def()) {
            return self.story_floor(p).map(|spot| spot.y);
        }
        match of(self.selection.stage_def()) {
            Some(d) => {
                let w = bevy_to_wow(p);
                d.grid.height(w[0], w[1])
            }
            None => self.world.terrain_height_under(p),
        }
    }

    /// `p` on the ground, or at `fallback` where there is none.
    pub fn at(&self, mut p: Vec3, fallback: f32) -> Vec3 {
        if story_instance(self.selection.stage_def()) {
            return self.spawn_spot(p, fallback);
        }
        p.y = self.height(p).unwrap_or(fallback);
        p
    }

    /// Where an enemy wanted at `p` may appear: on a dungeon's floor, inside its play area and its
    /// leash; on the open ground anywhere.
    pub fn spawn_spot(&self, p: Vec3, fallback: f32) -> Vec3 {
        let map = self.selection.stage_def();
        if story_instance(map) {
            let center = wow_to_bevy(map.center);
            for k in 0..=12 {
                let wish = p.lerp(center, k as f32 / 12.0);
                let Some(spot) = self.story_floor(wish) else { continue };
                if spot.distance_squared(center) > map.leash * map.leash { continue; }
                let from = center + Vec3::Y * 1.5;
                let to = spot + Vec3::Y * 1.5;
                let delta = to - from;
                if let Ok(dir) = Dir3::new(delta) {
                    if self.collision.cast_ray(from, dir, delta.length(), true,
                        &benilla_world::collision::WorldCollision::body_filter()).is_some() { continue; }
                }
                return spot;
            }
            return center;
        }
        let Some(d) = of(map) else {
            return self.at(p, fallback);
        };
        let center = map.center;
        let w = bevy_to_wow(p);
        // Pull the wish toward the centre until it is in play, then onto the nearest floor cell.
        for k in 0..=8 {
            let t = k as f32 / 8.0;
            let x = w[0] + (center[0] - w[0]) * t;
            let y = w[1] + (center[1] - w[1]) * t;
            let inside =
                d.area.contains(x, y, 2.0) && (x - center[0]).hypot(y - center[1]) <= map.leash;
            if !inside {
                continue;
            }
            if let Some(s) = d.grid.spawn_spot(x, y) {
                if d.area.contains(s[0], s[1], 2.0)
                    && (s[0] - center[0]).hypot(s[1] - center[1]) <= map.leash
                {
                    return wow_to_bevy(s);
                }
            }
        }
        wow_to_bevy(map.center)
    }

    /// Nearby instance floor, traced below roof height. Reject steep walls and missing floors.
    fn story_floor(&self, p: Vec3) -> Option<Vec3> {
        let mut from = p;
        from.y = self.selection.stage_def().center[2] + 8.0;
        let hit = self.collision.cast_ray(from, Dir3::NEG_Y, 24.0, true,
            &benilla_world::collision::WorldCollision::body_filter())?;
        (hit.normal.y >= 0.64).then(|| from - Vec3::Y * hit.distance)
    }
}
