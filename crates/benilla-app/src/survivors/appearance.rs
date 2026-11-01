//! The hero's appearance dials: skin colour, face, hair style, hair colour and facial hair, per
//! race and sex, as the menu's FEATURES rows cycle them.
//!
//! The values are counted from the same DBCs as [`benilla_formats::CharCreateCatalog`]'s ranges
//! (CharSections for skin, face and hair colour; CharHairGeosets; CharacterFacialHairStyles), but
//! for every race the tables carry: the upstream catalog stops at the eight playable ones, and the
//! roster's modded races (Worgen, Vulpera, Dracthyr, ...) come from a patch. Each dial keeps the
//! sorted values that exist, so a patch whose variations skip a number still cycles cleanly.

use std::collections::{BTreeSet, HashMap};

use benilla_assets::{LockRecover, WorldAssets};
use bevy::prelude::*;

/// `[skin, face, hair_style, hair_color, facial_hair]`, the wire values the composite and the
/// geoset tables take (not indices).
pub(crate) type Features = [u8; 5];

pub(super) const DIALS: usize = 5;

/// The selectable values per dial for one (race, sex). The face list depends on the skin colour,
/// as CharSections keys faces by it; a skin with no face rows falls back to every face.
///
/// Only looks whose textures the running data ships are offered: the player package keeps the
/// heroes' own CharSections art, and a skin whose base is missing composites to an untextured
/// (white) body, a hair row whose sheet is missing to white hair.
#[derive(Default)]
pub(super) struct Ranges {
    skins: Vec<u8>,
    faces: HashMap<u8, Vec<u8>>,
    all_faces: Vec<u8>,
    hair_styles: Vec<u8>,
    hair_colors: Vec<u8>,
    facial_hair: Vec<u8>,
    /// Each CharSections hair row (style, colour) and whether its mesh sheet ships.
    hair: HashMap<(u8, u8), bool>,
}

impl Ranges {
    /// The values dial `dial` can take with the other dials at `cur`. `cur`'s own value is added
    /// only to a dial the tables leave empty, so a display authored off them shows its look.
    pub(super) fn options(&self, dial: usize, cur: &Features) -> Vec<u8> {
        let mut v: Vec<u8> = match dial {
            0 => self.skins.clone(),
            1 => self.faces_for(cur[0]).to_vec(),
            2 => self.hair_styles.iter().copied().filter(|&s| self.hair_ok(s, cur[3])).collect(),
            3 => self.hair_colors.iter().copied().filter(|&c| self.hair_ok(cur[2], c)).collect(),
            _ => self.facial_hair.clone(),
        };
        if v.is_empty() {
            v.push(cur[dial]);
        }
        v
    }

    fn faces_for(&self, skin: u8) -> &[u8] {
        self.faces
            .get(&skin)
            .filter(|f| !f.is_empty())
            .unwrap_or(&self.all_faces)
    }

    /// Whether the hair mesh sheet for (style, colour) reads: the style's own row, else variation
    /// 1's ([`benilla_formats::CharSections::hair_mesh_texture`]); no row at all draws no sheet.
    fn hair_ok(&self, style: u8, color: u8) -> bool {
        self.hair
            .get(&(style, color))
            .or_else(|| self.hair.get(&(1, color)))
            .copied()
            .unwrap_or(true)
    }

    /// A new skin colour can leave the face without a CharSections row: take that skin's first.
    pub(super) fn fit_face(&self, f: &mut Features) {
        if let Some(faces) = self.faces.get(&f[0]).filter(|v| !v.is_empty()) {
            if !faces.contains(&f[1]) {
                f[1] = faces[0];
            }
        }
    }

    /// Move every dial this data cannot draw to the nearest look it can: a pick saved on a fuller
    /// install, or made before the ranges checked the art. Whether anything changed.
    pub(super) fn fit(&self, f: &mut Features) -> bool {
        let before = *f;
        if !self.skins.is_empty() && !self.skins.contains(&f[0]) {
            f[0] = self.skins[0];
        }
        let faces = self.faces_for(f[0]);
        if !faces.is_empty() && !faces.contains(&f[1]) {
            f[1] = faces[0];
        }
        if !self.hair_ok(f[2], f[3]) {
            if let Some(&c) = self.hair_colors.iter().find(|&&c| self.hair_ok(f[2], c)) {
                f[3] = c;
            } else if let Some(&s) = self.hair_styles.iter().find(|&&s| self.hair_ok(s, f[3])) {
                f[2] = s;
            }
        }
        *f != before
    }
}

/// The picked hero's dial key, its dials (the display's own look until one is set) and its race
/// and sex's ranges.
pub(super) fn current<'a>(
    selection: &super::Selection,
    creatures: &crate::entities::Creatures,
    appearances: Option<&'a Appearances>,
) -> Option<((usize, u8, u8), Features, &'a Ranges)> {
    let class = selection.class?;
    let &(display, race, sex) = selection.looks.get(selection.look?)?;
    let ranges = appearances?.ranges(race, sex)?;
    let f = selection
        .features
        .get(&(class, race, sex))
        .copied()
        .or_else(|| creatures.survivors_appearance(display).map(base))?;
    Some(((class, race, sex), f, ranges))
}

/// A display's own dials.
pub(crate) fn base(npc: &benilla_formats::NpcAppearance) -> Features {
    [npc.skin, npc.face, npc.hair_style, npc.hair_color, npc.facial_hair]
}

/// Every (race, sex)'s [`Ranges`], read once from the patch chain.
#[derive(Resource, Default)]
pub(crate) struct Appearances(HashMap<(u8, u8), Ranges>);

impl Appearances {
    pub(super) fn ranges(&self, race: u8, sex: u8) -> Option<&Ranges> {
        self.0.get(&(race, sex))
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            load.run_if(not(resource_exists::<Appearances>)),
            repair.run_if(resource_exists::<Appearances>),
            // After the menu's own per-frame paint, which leaves the dial arrows alone.
            super::features_ui::refresh.after(super::menu::refresh_menu_ui),
        ),
    );
}

/// Fit every saved pick to what this data can draw ([`Ranges::fit`]), so a hero never wears a
/// look whose art is missing.
fn repair(mut selection: ResMut<super::Selection>, appearances: Res<Appearances>) {
    let mut changed = false;
    for (&(_, race, sex), f) in selection.bypass_change_detection().features.iter_mut() {
        if let Some(ranges) = appearances.ranges(race, sex) {
            changed |= ranges.fit(f);
        }
    }
    if changed {
        selection.version += 1;
    }
}

fn load(mut commands: Commands, assets: Option<Res<WorldAssets>>) {
    let Some(assets) = assets else { return };
    let mut chain = assets.chain.lock_recover();
    let mut read = |name: &str| chain.read_file(name).ok().and_then(|b| Wdbc::new(b));
    let sections = read("DBFilesClient\\CharSections.dbc");
    let hair = read("DBFilesClient\\CharHairGeosets.dbc");
    let facial = read("DBFilesClient\\CharacterFacialHairStyles.dbc");

    type Sets = (
        BTreeSet<u8>,
        HashMap<u8, BTreeSet<u8>>,
        BTreeSet<u8>,
        BTreeSet<u8>,
        BTreeSet<u8>,
        BTreeSet<u8>,
        HashMap<(u8, u8), bool>,
    );
    let mut acc: HashMap<(u8, u8), Sets> = HashMap::new();
    // CharSections: ID, Race, Sex, SectionType, Variation, Color, Tex0-2, Flags; flag 0x1 is a row
    // the client never offers. A row counts only if its art is in the chain.
    if let Some(t) = sections.filter(|t| t.fields >= 10) {
        let ships = |r: usize, cols: &[usize]| {
            // The web host answers an unindexed name with a request each; it serves the full set.
            if cfg!(target_arch = "wasm32") {
                return true;
            }
            cols.iter()
                .map(|&c| t.str(r, 6 + c))
                .filter(|p| !p.is_empty())
                .all(|p| chain.contains(p))
        };
        for r in 0..t.records {
            let [race, sex, ty, var, color] = [1, 2, 3, 4, 5].map(|f| t.u32(r, f) as u8);
            if t.u32(r, 9) & 1 != 0 {
                continue;
            }
            let e = acc.entry((race, sex)).or_default();
            match ty {
                // The base skin and the tauren fur, both bound whole.
                0 if var == 0 && ships(r, &[0, 1]) => {
                    e.0.insert(color);
                }
                1 if ships(r, &[0, 1]) => {
                    e.1.entry(color).or_default().insert(var);
                    e.2.insert(var);
                }
                3 => {
                    let shipped = ships(r, &[0]);
                    e.6.insert((var, color), shipped);
                    if shipped {
                        e.4.insert(color);
                    }
                }
                _ => {}
            }
        }
    }
    drop(chain);
    // CharHairGeosets: ID, Race, Sex, Variation, ...
    if let Some(t) = hair.filter(|t| t.fields >= 4) {
        for r in 0..t.records {
            let key = (t.u32(r, 1) as u8, t.u32(r, 2) as u8);
            acc.entry(key).or_default().3.insert(t.u32(r, 3) as u8);
        }
    }
    // CharacterFacialHairStyles: Race, Sex, Variation, ...
    if let Some(t) = facial.filter(|t| t.fields >= 3) {
        for r in 0..t.records {
            let key = (t.u32(r, 0) as u8, t.u32(r, 1) as u8);
            acc.entry(key).or_default().5.insert(t.u32(r, 2) as u8);
        }
    }
    let ranges: HashMap<(u8, u8), Ranges> = acc
        .into_iter()
        .map(|(key, (skins, faces, all_faces, styles, colors, facial, hair))| {
            (
                key,
                Ranges {
                    skins: skins.into_iter().collect(),
                    faces: faces
                        .into_iter()
                        .map(|(k, v)| (k, v.into_iter().collect()))
                        .collect(),
                    all_faces: all_faces.into_iter().collect(),
                    hair_styles: styles.into_iter().collect(),
                    hair_colors: colors.into_iter().collect(),
                    facial_hair: facial.into_iter().collect(),
                    hair,
                },
            )
        })
        .collect();
    info!("survivors appearances: {} race/sex ranges", ranges.len());
    commands.insert_resource(Appearances(ranges));
}

/// A WDBC file read by index, every field a `u32`.
struct Wdbc {
    bytes: Vec<u8>,
    records: usize,
    fields: usize,
    size: usize,
}

impl Wdbc {
    fn new(bytes: Vec<u8>) -> Option<Self> {
        if bytes.len() < 20 || &bytes[0..4] != b"WDBC" {
            return None;
        }
        let word = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
        let (records, fields, size) = (word(4), word(8), word(12));
        (size >= fields * 4 && 20 + records * size <= bytes.len()).then_some(Self {
            bytes,
            records,
            fields,
            size,
        })
    }

    fn u32(&self, record: usize, field: usize) -> u32 {
        let o = 20 + record * self.size + field * 4;
        u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap())
    }

    /// A string field: its offset into the block after the records, read to the NUL.
    fn str(&self, record: usize, field: usize) -> &str {
        let start = 20 + self.records * self.size + self.u32(record, field) as usize;
        let tail = self.bytes.get(start..).unwrap_or_default();
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        std::str::from_utf8(&tail[..end]).unwrap_or_default()
    }
}

/// A dial's row label: the race's own word for it where the create catalog has one (tusks,
/// horns, piercings, ...), else the generic name.
pub(super) fn dial_label(
    catalog: Option<&crate::entities::CharCreate>,
    dial: usize,
    race: u8,
    sex: u8,
) -> &'static str {
    let token = match dial {
        2 => catalog.and_then(|c| c.0.hair_customization(race)),
        4 => catalog.and_then(|c| c.0.facial_hair_customization(race, sex)),
        _ => None,
    };
    match (dial, token.map(str::to_ascii_uppercase).as_deref()) {
        (0, _) => "Skin Color",
        (1, _) => "Face",
        (2, Some("HORNS")) => "Horns",
        (2, _) => "Hair Style",
        (3, _) => "Hair Color",
        (_, Some("EARRINGS")) => "Earrings",
        (_, Some("PIERCINGS")) => "Piercings",
        (_, Some("MARKINGS")) => "Markings",
        (_, Some("FEATURES")) => "Features",
        (_, Some("HORNS")) => "Horns",
        (_, Some("TUSKS")) => "Tusks",
        (_, Some("HAIR")) => "Hair",
        _ => "Facial Hair",
    }
}
