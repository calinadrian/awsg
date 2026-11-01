//! Validate an imported Elwynn overlay with the game's terrain, model and texture readers.
//! `cargo run -p benilla-formats --example verify_retail_elwynn -- path/to/patch-Q.MPQ`
use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use benilla_formats::{
    Chain, adt_to_tile_mesh, load_ground_effect_catalog, load_object_model, parse_wmo_root,
    scatter_ground_doodads, terrain_height_at,
};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("expected MPQ path")?;
    let mut overlay = Chain::open(Path::new(&path))?;
    let listfile = String::from_utf8(overlay.read_file("(listfile)")?)?;
    // Optional Data directory proves patch priority resolves to the converted bytes.
    let mut chain = match std::env::args().nth(2) {
        Some(data) => Chain::open(Path::new(&data))?,
        None => Chain::open(Path::new(&path))?,
    };
    let mut counts = [0; 4];
    let ground = load_ground_effect_catalog(&mut chain, true)?;
    let mut arena_tufts = 0;
    let mut checked = HashSet::new();
    let mut check_texture = |chain: &mut Chain, path: &str| -> Result<()> {
        if checked.insert(path.to_ascii_lowercase()) {
            benilla_blp::decode_native(&chain.read_file(path)?)
                .with_context(|| format!("texture {path}"))?;
        }
        Ok(())
    };
    let mut arena = None;
    for name in listfile.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if name.ends_with(".adt") {
            let bytes = chain.read_file(name)?;
            ensure!(
                bytes == overlay.read_file(name)?,
                "{name}: a later patch hides retail terrain"
            );
            let tile = adt_to_tile_mesh(&bytes).with_context(|| name.to_owned())?;
            ensure!(tile.chunks.len() == 256, "{name}: incomplete tile");
            ensure!(
                tile.chunks
                    .iter()
                    .all(|c| c.shadow.as_ref().is_some_and(|s| s.len() == 4096)),
                "{name}: missing baked scenery shadows"
            );
            ensure!(
                tile.chunks
                    .iter()
                    .any(|c| c.shadow.as_ref().is_some_and(|s| s.contains(&255))),
                "{name}: scenery shadows are empty"
            );
            for c in &tile.chunks {
                ensure!(c.positions.len() == 145, "{name}: missing heights");
                ensure!(c.positions.iter().flatten().all(|p| p.is_finite()));
                for tex in &c.layer_textures {
                    check_texture(&mut chain, tex)?;
                }
            }
            for model in &tile.doodads {
                chain.read_file(&model.model)?;
            }
            for wmo in &tile.wmos {
                chain.read_file(&wmo.model)?;
            }
            if name.ends_with("azeroth_33_49.adt") {
                arena = terrain_height_at(&tile.chunks, [-9396.0, -633.0, 68.0]);
                ensure!(arena.is_some(), "Elwynn spawn is in a terrain hole");
                for c in &tile.chunks {
                    for tuft in scatter_ground_doodads(c, &ground, 33, 49, 2.) {
                        chain.read_file(&tuft.model)?;
                        if (tuft.position[0] + 9396.).hypot(tuft.position[1] + 633.) < 120. {
                            arena_tufts += 1;
                        }
                    }
                }
            }
            counts[0] += 1;
        } else if name.ends_with(".m2") || name.ends_with(".wmo") {
            // Group files are validated when their parent building is loaded.
            let stem = name.trim_end_matches(".wmo");
            if name.ends_with(".wmo")
                && stem
                    .rsplit('_')
                    .next()
                    .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
            {
                continue;
            }
            if name.ends_with(".wmo") {
                let root = parse_wmo_root(&chain.read_file(name)?)?;
                for prop in root.doodads() {
                    ensure!(!prop.model.is_empty(), "{name}: unresolved building prop");
                    chain.read_file(&prop.model)?;
                }
            }
            for mesh in load_object_model(&mut chain, name).with_context(|| name.to_owned())? {
                if name.ends_with("107037.wmo") && !mesh.interior {
                    ensure!(
                        mesh.vertex_colors.is_empty()
                            || mesh.vertex_colors.iter().all(|c| c[0] + c[1] + c[2] > 0.1),
                        "Mage tower exterior has a black retail lighting multiplier"
                    );
                }
                ensure!(mesh.positions.iter().flatten().all(|p| p.is_finite()));
                ensure!(
                    mesh.indices
                        .iter()
                        .all(|&i| (i as usize) < mesh.positions.len())
                );
                if let Some(texture) = mesh.texture {
                    check_texture(&mut chain, &texture)?;
                }
            }
            counts[if name.ends_with(".m2") { 1 } else { 2 }] += 1;
        } else if name.ends_with(".blp") {
            check_texture(&mut chain, name)?;
            counts[3] += 1;
        }
    }
    ensure!(counts[0] == 9, "expected nine retail terrain tiles");
    ensure!(
        arena_tufts > 100,
        "retail arena ground cover is missing: {arena_tufts}"
    );
    println!("Retail ground cover: {arena_tufts} tufts in the playable circle at Medium detail");
    let minimaps = benilla_formats::load_minimap_translate(&mut chain)?;
    for y in 48..=50 {
        for x in 32..=34 {
            let expected = format!("asr-retail-elwynn-{x}-{y}.blp");
            ensure!(
                minimaps.tile("Azeroth", x, y) == Some(expected.as_str()),
                "retail minimap is hidden"
            );
        }
    }
    println!(
        "Validated {} terrain tiles, {} models, {} buildings, {} textures; Elwynn spawn floor {:?}",
        counts[0], counts[1], counts[2], counts[3], arena
    );
    Ok(())
}
