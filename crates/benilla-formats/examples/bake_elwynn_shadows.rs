//! Bake imported scenery into vanilla MCSH masks. Run via scripts/bake_elwynn_shadows.py.
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use benilla_formats::{Chain, ModelBlend, RenderSubmesh, adt_to_tile_mesh, load_object_model};
use glam::{Quat, Vec2, Vec3};

// Survivors fixes daylight at minute 780; match lighting/daynight.rs at that minute.
fn shear() -> Vec2 {
    let phi: f32 = 2.2165682 + (1.9198623 - 2.2165682) / 6.0;
    let theta: f32 = 3.926991;
    Vec2::new(phi.sin() * theta.cos(), phi.sin() * theta.sin()) / phi.cos()
}

struct Cutout {
    width: usize,
    height: usize,
    alpha: Vec<u8>,
}
struct Triangle {
    p: [Vec3; 3],
    uv: [Vec2; 3],
    cutout: Option<Arc<Cutout>>,
    wrap: [bool; 2],
}

impl Triangle {
    fn weights(&self, q: Vec2) -> Option<[f32; 3]> {
        let [a, b, c] = self.p.map(|p| p.truncate());
        let det = (b - a).perp_dot(c - a);
        if det.abs() < 1e-7 {
            return None;
        }
        let v = (q - a).perp_dot(c - a) / det;
        let w = (b - a).perp_dot(q - a) / det;
        let u = 1.0 - v - w;
        (u >= 0.0 && v >= 0.0 && w >= 0.0).then_some([u, v, w])
    }

    fn occludes(&self, q: Vec2, z: f32) -> bool {
        let Some(w) = self.weights(q) else {
            return false;
        };
        if self.p.iter().zip(w).map(|(p, w)| p.z * w).sum::<f32>() <= z + 0.15 {
            return false;
        }
        let Some(mask) = &self.cutout else {
            return true;
        };
        let uv = self.uv.iter().zip(w).map(|(uv, w)| *uv * w).sum::<Vec2>();
        let pixel = |v: f32, n: usize, wrap: bool| {
            let v = if wrap {
                v.rem_euclid(1.0)
            } else {
                v.clamp(0.0, 1.0)
            };
            ((v * n as f32) as usize).min(n - 1)
        };
        let x = pixel(uv.x, mask.width, self.wrap[0]);
        let y = pixel(uv.y, mask.height, self.wrap[1]);
        mask.alpha[y * mask.width + x] >= 128
    }
}

fn rotation(deg: [f32; 3]) -> Quat {
    use std::f32::consts::{FRAC_PI_2, PI};
    let [rx, ry, rz] = deg.map(f32::to_radians);
    // Same placement matrix as benilla_assets::coords, kept in source WoW coordinates.
    Quat::from_rotation_x(FRAC_PI_2)
        * Quat::from_rotation_y(ry - PI)
        * Quat::from_rotation_z(-rx)
        * Quat::from_rotation_x(rz - FRAC_PI_2)
}

const GRID: f32 = 8.0;
fn bucket(p: Vec2) -> (i32, i32) {
    ((p.x / GRID).floor() as i32, (p.y / GRID).floor() as i32)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() == 3, "expected input MPQ and mask output folder");
    let mut chain = Chain::open(Path::new(&args[1]))?;
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out)?;
    let mut tiles = Vec::new();
    for y in 48..=50 {
        for x in 32..=34 {
            let name = format!("world/maps/azeroth/azeroth_{x}_{y}.adt");
            tiles.push((x, y, adt_to_tile_mesh(&chain.read_file(&name)?)?));
        }
    }
    let mut seen = HashSet::new();
    let mut models: HashMap<String, Vec<RenderSubmesh>> = HashMap::new();
    let mut masks: HashMap<String, Arc<Cutout>> = HashMap::new();
    let mut triangles = Vec::new();
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let slope = shear();
    for (_, _, tile) in &tiles {
        let placements = tile
            .doodads
            .iter()
            .map(|d| (&d.model, d.unique_id, d.position, d.rotation, d.scale))
            .chain(
                tile.wmos
                    .iter()
                    .map(|w| (&w.model, w.unique_id, w.position, w.rotation, 1.0)),
            );
        for (name, id, at, deg, scale) in placements {
            if !seen.insert((name.ends_with(".wmo"), id)) {
                continue;
            }
            if !models.contains_key(name) {
                models.insert(
                    name.clone(),
                    load_object_model(&mut chain, name).with_context(|| name.clone())?,
                );
            }
            let rot = rotation(deg);
            for mesh in &models[name] {
                if !matches!(mesh.blend, ModelBlend::Opaque | ModelBlend::AlphaTest) {
                    continue;
                }
                let cutout = if mesh.blend == ModelBlend::AlphaTest {
                    let name = mesh.texture.as_ref().context("cutout has no texture")?;
                    if !masks.contains_key(name) {
                        let image = benilla_blp::decode(&chain.read_file(name)?)?;
                        masks.insert(
                            name.clone(),
                            Arc::new(Cutout {
                                width: image.width as usize,
                                height: image.height as usize,
                                alpha: image.mips[0]
                                    .rgba
                                    .as_chunks::<4>()
                                    .0
                                    .iter()
                                    .map(|p| p[3])
                                    .collect(),
                            }),
                        );
                    }
                    Some(masks[name].clone())
                } else {
                    None
                };
                for idx in mesh.indices.as_chunks::<3>().0 {
                    let p = std::array::from_fn(|i| {
                        let p = rot * Vec3::from_array(mesh.positions[idx[i] as usize]) * scale
                            + Vec3::from_array(at);
                        let q = p.truncate() - slope * p.z;
                        Vec3::new(q.x, q.y, p.z)
                    });
                    let uv = std::array::from_fn(|i| Vec2::from_array(mesh.uvs[idx[i] as usize]));
                    let lo = p
                        .iter()
                        .fold(Vec2::splat(f32::INFINITY), |a, p| a.min(p.truncate()));
                    let hi = p
                        .iter()
                        .fold(Vec2::splat(f32::NEG_INFINITY), |a, p| a.max(p.truncate()));
                    let (lx, ly) = bucket(lo);
                    let (hx, hy) = bucket(hi);
                    let n = triangles.len();
                    triangles.push(Triangle {
                        p,
                        uv,
                        cutout: cutout.clone(),
                        wrap: [mesh.wrap_x, mesh.wrap_y],
                    });
                    for x in lx..=hx {
                        for y in ly..=hy {
                            grid.entry((x, y)).or_default().push(n);
                        }
                    }
                }
            }
        }
    }
    println!(
        "Baking {} scenery placements, {} triangles",
        seen.len(),
        triangles.len()
    );
    for (x, y, tile) in &tiles {
        let mut bits = vec![0u8; 256 * 512];
        let mut shaded = 0usize;
        for (i, chunk) in tile.chunks.iter().enumerate() {
            let nw = chunk.positions[0];
            for row in 0..64 {
                for col in 0..64 {
                    let px = nw[0] - (row as f32 + 0.5) * benilla_formats::CHUNK_SIZE / 64.0;
                    let py = nw[1] - (col as f32 + 0.5) * benilla_formats::CHUNK_SIZE / 64.0;
                    let Some(z) = chunk.height_at([px, py, 0.0]) else {
                        continue;
                    };
                    let q = Vec2::new(px, py) - slope * z;
                    if grid
                        .get(&bucket(q))
                        .is_some_and(|list| list.iter().any(|&t| triangles[t].occludes(q, z)))
                    {
                        bits[i * 512 + row * 8 + col / 8] |= 1 << (col % 8);
                        shaded += 1;
                    }
                }
            }
        }
        std::fs::write(out.join(format!("azeroth_{x}_{y}.mcsh")), bits)?;
        println!("Tile {x},{y}: {shaded} shadowed texels");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ray_respects_height_triangle_edges_and_leaf_alpha() {
        let mut t = Triangle {
            p: [
                Vec3::new(0., 0., 5.),
                Vec3::new(2., 0., 5.),
                Vec3::new(0., 2., 5.),
            ],
            uv: [Vec2::ZERO; 3],
            cutout: None,
            wrap: [false; 2],
        };
        assert!(t.occludes(Vec2::splat(0.5), 0.));
        assert!(!t.occludes(Vec2::splat(2.), 0.));
        assert!(!t.occludes(Vec2::splat(0.5), 6.));
        t.cutout = Some(Arc::new(Cutout {
            width: 1,
            height: 1,
            alpha: vec![0],
        }));
        assert!(!t.occludes(Vec2::splat(0.5), 0.));
    }
}
