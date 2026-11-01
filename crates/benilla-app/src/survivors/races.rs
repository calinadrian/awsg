//! Local ASR roster; models and textures live in the user patch chain.
use super::data::ClassDef;
use crate::entities::Creatures;
pub(super) struct Race {
    pub id: u8,
    pub name: &'static str,
    pub faction: u8,
    pub classes: &'static [u8],
    pub portrait: (u16, u16),
}
include!("races_data.rs");

/// Original race effect tag 8, from the table at RVA 0x954c0b0 (88-byte rows).
pub(super) fn xp_bonus(id: u8) -> f32 {
    match id {
        1 | 86 | 24 => 0.10,
        84 => 0.12,
        27 => 0.08,
        _ => 0.0,
    }
}

/// Original race effect tag 7, applied to the base pickup radius.
pub(super) fn pickup_bonus(id: u8) -> f32 {
    match id {
        3 => 0.30,
        52 | 86 => 0.25,
        35 => 0.35,
        _ => 0.0,
    }
}

pub(super) fn race_name(id: u8) -> &'static str {
    RACES
        .iter()
        .find(|r| r.id == id)
        .map_or("Unknown", |r| r.name)
}
pub(super) fn race_tc(id: u8, sex: u8) -> Option<[f32; 4]> {
    let r = RACES.iter().find(|r| r.id == id)?;
    let (x, y) = (
        f32::from(r.portrait.0),
        f32::from(r.portrait.1) + f32::from(sex.min(1)) * 128.0,
    );
    Some([
        x / 1024.0,
        (x + 64.0) / 1024.0,
        y / 256.0,
        (y + 64.0) / 256.0,
    ])
}
pub(super) fn looks_for(class: &ClassDef, creatures: &Creatures) -> Vec<(u32, u8, u8)> {
    let mut looks = Vec::new();
    for r in RACES.iter().filter(|r| r.classes.contains(&class.id)) {
        for sex in 0..=1 {
            let display = 300000 + u32::from(class.id) * 200 + u32::from(r.id) * 2 + u32::from(sex);
            if creatures.display_race_sex(display) == Some((r.id, sex)) {
                looks.push((display, r.id, sex));
            } else if let Some(&display) = class
                .looks
                .iter()
                .find(|&&d| creatures.display_race_sex(d) == Some((r.id, sex)))
            {
                looks.push((display, r.id, sex));
            }
        }
    }
    looks
}
