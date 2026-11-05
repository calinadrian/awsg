//! Development helper: `item_display_audit <data-dir> <items.tsv> [<out.rgba> <display id>...]` walks
//! every catalog item through what the client's dress path reads and prints each one that would not
//! show on a character; the optional tail writes a body atlas wearing those shirt, chest... displays.
//!
//! Make the table with `cargo run -p asr-coop --example catalog_table`: each line is `id, name, slot,
//! display id, category`, tab separated. A body slot's
//! textures go through [`equip_region_candidates`] and the real BLP decoder for both sexes, because
//! `CompositePlan::run` skips an overlay that reads nowhere after the underwear is already gone.

use std::collections::BTreeMap;
use std::path::Path;

use benilla_formats::{
    blp_bytes_to_mip_chain, equip_region_candidates, equip_tex_dir, load_item_display_catalog,
    Chain, CharSections, ItemDisplay,
};

/// The layers a body slot paints, from the compositor's `EQUIP_LAYER_COLUMN`.
fn body_layers(slot: &str) -> Option<&'static [usize]> {
    Some(match slot {
        "shirt" => &[0, 1, 3, 4],
        "chest" => &[0, 1, 3, 4, 5, 6],
        "belt" => &[5],
        "legs" => &[5, 6],
        "feet" => &[6, 7],
        "wrists" => &[1],
        "hands" => &[1, 2],
        "tabard" => &[3, 4],
        _ => return None,
    })
}

/// A texture's fate for one sex: `None` when a candidate decodes, else why none did.
fn texture_fault(chain: &Chain, layer: usize, name: &str, sex: u8) -> Option<String> {
    let mut seen = None;
    for path in equip_region_candidates(layer, name, sex) {
        if !chain.contains(&path) {
            continue;
        }
        match chain.read(&path).and_then(|b| blp_bytes_to_mip_chain(&b)) {
            Ok(tex) => {
                // A nearly empty TorsoUpper still removes the underwear and so leaves a bare chest.
                if layer == 3 {
                    let alpha = tex.mips[0].chunks_exact(4).filter(|px| px[3] > 128).count();
                    let total = (tex.width * tex.height).max(1) as usize;
                    if alpha * 100 / total < 25 {
                        return Some(format!("{path}: only {}% opaque", alpha * 100 / total));
                    }
                }
                return None;
            }
            Err(e) => seen = Some(format!("{path}: {e:#}")),
        }
    }
    Some(seen.unwrap_or_else(|| "no file".into()))
}

/// Every reason `display` would not show on a body in `slot`.
fn faults(chain: &Chain, display: &ItemDisplay, slot: &str, category: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(layers) = body_layers(slot) {
        let mut painted = 0;
        for &layer in layers {
            let Some(name) = display.region_textures[layer].as_deref() else {
                continue;
            };
            painted += 1;
            for sex in 0..2u8 {
                if let Some(why) = texture_fault(chain, layer, name, sex) {
                    let dir = equip_tex_dir(layer).unwrap_or("?");
                    let who = if sex == 1 { "female" } else { "male" };
                    out.push(format!("{dir}/{name} {who}: {why}"));
                }
            }
        }
        // A chest with no torso art is the naked case even when every file it names reads.
        if slot == "chest" && display.region_textures[3].is_none() {
            out.push("chest without TorsoUpper art".into());
        }
        if painted == 0 && display.geoset_groups == [0; 3] {
            out.push("no textures and no geosets".into());
        }
        return out;
    }
    let (dir, columns): (&str, &[usize]) = match (slot, category) {
        ("head", _) => ("Head", &[0]),
        ("shoulders", _) => ("Shoulder", &[0, 1]),
        ("off_hand", "shield") => ("Shield", &[0]),
        ("main_hand" | "off_hand" | "ranged", _) => ("Weapon", &[0]),
        ("back", _) => {
            match display.model_texture[0].as_deref() {
                Some(tex) => {
                    let path = format!("Item\\ObjectComponents\\Cape\\{tex}.blp");
                    if !chain.contains(&path) {
                        out.push(format!("no file {path}"));
                    }
                }
                None => out.push("cape without a texture".into()),
            }
            return out;
        }
        _ => return out, // Jewelry has no visible model.
    };
    for &col in columns {
        let Some(model) = display.model[col].as_deref() else {
            out.push(format!("no {dir} model in column {col}"));
            continue;
        };
        // A helm's file carries the wearer: test the human pair, the two the base data all ship.
        let paths: Vec<String> = if slot == "head" {
            let stem = model.strip_suffix(".m2").unwrap_or(model);
            ["HuM", "HuF"]
                .map(|who| format!("Item\\ObjectComponents\\{dir}\\{stem}_{who}.m2"))
                .into()
        } else {
            vec![format!("Item\\ObjectComponents\\{dir}\\{model}")]
        };
        for path in paths {
            if !chain.contains(&path) {
                out.push(format!("no file {path}"));
            }
        }
        if let Some(tex) = display.model_texture[col].as_deref() {
            let path = format!("Item\\ObjectComponents\\{dir}\\{tex}.blp");
            if !chain.contains(&path) {
                out.push(format!("no file {path}"));
            }
        }
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (data, table) = (
        args.next().expect("data dir"),
        args.next().expect("items table"),
    );
    let mut chain = Chain::open(Path::new(&data)).expect("open patch chain");
    let catalog = load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
    let text = std::fs::read_to_string(table).expect("read items table");
    let (mut total, mut broken) = (0u32, 0u32);
    let mut per_slot: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let [id, name, slot, display, category, ..] = f[..] else {
            continue;
        };
        let display_id: u32 = display.parse().expect("display id");
        let found = match catalog.get(display_id) {
            Some(row) => faults(&chain, row, slot, category),
            None => vec!["display id not in ItemDisplayInfo".into()],
        };
        total += 1;
        let tally = per_slot.entry(slot.to_string()).or_default();
        tally.0 += 1;
        if !found.is_empty() {
            broken += 1;
            tally.1 += 1;
            println!(
                "BROKEN {id}\t{name}\t{slot}\t{display_id}\t{}",
                found.join(" | ")
            );
        }
    }
    for (slot, (all, bad)) in per_slot {
        println!("SLOT {slot}: {bad} of {all} broken");
    }
    println!("TOTAL {broken} of {total} broken");
    // `<out.rgba> <display id>...`: a Human female's 256² body atlas wearing those chest displays.
    let Some(out) = args.next() else { return };
    let sections = CharSections::load(&mut chain).expect("CharSections");
    let mut equipment: [Option<&ItemDisplay>; 8] = [None; 8];
    for (slot, id) in args.enumerate() {
        equipment[slot] = catalog.get(id.parse().expect("display id"));
    }
    let atlas = sections
        .composite_plan(1, 1, 0, 0, 0, 0, 0, equipment, None, false)
        .expect("a base skin")
        .run(|path| blp_bytes_to_mip_chain(&chain.read(path)?))
        .expect("composite");
    std::fs::write(&out, &atlas.mips[0]).expect("write atlas");
    println!("ATLAS {}x{} {out}", atlas.width, atlas.height);
}
