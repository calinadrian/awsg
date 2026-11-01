//! Verify the local ASR race import through the same loaders used by the game.
//! `cargo run -p benilla-formats --example verify_asr_races -- wow-data/Data`
use std::collections::HashSet;
use std::path::Path;

use anyhow::{ensure, Context, Result};
use benilla_formats::{load_creature_catalog, load_object_model, Chain, CharSections};

fn moving<const N: usize>(keys: &[(f32, [f32; N])]) -> bool {
    keys.windows(2).any(|pair| {
        pair[0]
            .1
            .iter()
            .zip(pair[1].1)
            .any(|(a, b)| (a - b).abs() > 1e-5)
    })
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("expected Data directory")?;
    let mut chain = Chain::open(Path::new(&path))?;
    let catalog = load_creature_catalog(&mut chain)?;
    let sections = CharSections::load(&mut chain)?;
    let mut bodies = HashSet::new();
    let mut appearances = 0;
    for (display, _) in catalog
        .display_models()
        .filter(|(d, _)| (300000..303000).contains(d))
    {
        let (race, sex) = catalog
            .display_race_sex(display)
            .context("missing appearance identity")?;
        let class = (display - 300000) / 200;
        ensure!(
            display == 300000 + class * 200 + u32::from(race) * 2 + u32::from(sex),
            "identity mismatch"
        );
        let model = catalog.model(display).context("missing display model")?;
        ensure!(
            model.scale.is_finite() && model.scale > 0.0,
            "invalid scale"
        );
        let appearance = model
            .npc_appearance
            .context("missing character appearance")?;
        ensure!(appearance.bake_name.is_none(), "foreign baked skin");
        appearances += 1;
        if bodies.insert((race, sex)) {
            let path = format!(
                "{}.m2",
                model
                    .model_path
                    .rsplit_once('.')
                    .map_or(model.model_path.as_str(), |p| p.0)
            );
            let bytes = chain.read(&path)?;
            let animations = benilla_formats::parse_m2_animations(&bytes);
            for id in [0, 4, 5] {
                let clip = animations
                    .iter()
                    .find(|a| a.anim_id == id)
                    .with_context(|| format!("missing animation {id} for {race}/{sex}"))?;
                ensure!(
                    clip.bones
                        .iter()
                        .any(|b| moving(&b.translation) || moving(&b.rotation) || moving(&b.scale)),
                    "static animation {id} for {race}/{sex}"
                );
            }
            let meshes = load_object_model(&mut chain, &model.model_path)
                .with_context(|| format!("race {race}/{sex}: {}", model.model_path))?;
            ensure!(!meshes.is_empty(), "empty body {race}/{sex}");
            for mesh in meshes {
                if let Some(texture) = mesh.texture {
                    benilla_formats::read_texture_mip_chain(&mut chain, &texture)
                        .with_context(|| format!("body {race}/{sex}: {texture}"))?;
                }
            }
            ensure!(
                sections
                    .composite_body(&mut chain, race, sex, 0, 0, 0, 0, 0, [None; 8], None, false)?
                    .is_some(),
                "no skin {race}/{sex}"
            );
            for texture in [
                sections.hair_texture(race, sex, 0, 0),
                sections.skin_extra_texture(race, sex, 0),
            ]
            .into_iter()
            .flatten()
            {
                benilla_formats::read_texture_mip_chain(&mut chain, texture)
                    .with_context(|| format!("hair/extra {race}/{sex}: {texture}"))?;
            }
        }
    }
    ensure!(
        bodies.len() == 52,
        "expected 52 bodies, got {}",
        bodies.len()
    );
    for race in [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 52, 84,
        86,
    ] {
        ensure!(
            bodies.contains(&(race, 0)) && bodies.contains(&(race, 1)),
            "missing race {race}"
        );
    }
    println!("Verified 26 races, 52 body models/skins with moving idle/walk/run clips and {appearances} class appearances.");
    Ok(())
}
