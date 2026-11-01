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
#[derive(Default)]
pub(super) struct Ranges {
    skins: Vec<u8>,
    faces: HashMap<u8, Vec<u8>>,
    all_faces: Vec<u8>,
    hair_styles: Vec<u8>,
    hair_colors: Vec<u8>,
    facial_hair: Vec<u8>,
}

impl Ranges {
    /// The values dial `dial` can take with the other dials at `cur`; `cur`'s own value is always
    /// in it, so a display authored off the tables still shows its look as one step.
    pub(super) fn options(&self, dial: usize, cur: &Features) -> Vec<u8> {
        let base: &[u8] = match dial {
            0 => &self.skins,
            1 => self
                .faces
                .get(&cur[0])
                .filter(|f| !f.is_empty())
                .unwrap_or(&self.all_faces),
            2 => &self.hair_styles,
            3 => &self.hair_colors,
            _ => &self.facial_hair,
        };
        let mut v = base.to_vec();
        if !v.contains(&cur[dial]) {
            v.push(cur[dial]);
            v.sort_unstable();
        }
        v
    }

    /// A new skin colour can leave the face without a CharSections row: take that skin's first.
    pub(super) fn fit_face(&self, f: &mut Features) {
        if let Some(faces) = self.faces.get(&f[0]).filter(|v| !v.is_empty()) {
            if !faces.contains(&f[1]) {
                f[1] = faces[0];
            }
        }
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

/// Whether a race's body skin can be composited from CharSections: the eight 1.12 races. A
/// modded race wears its display's baked atlas, so only its geoset dials change it.
pub(crate) fn composites(race: u8) -> bool {
    (1..=8).contains(&race)
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
            // After the menu's own per-frame paint, which leaves the dial arrows alone.
            super::features_ui::refresh.after(super::menu::refresh_menu_ui),
        ),
    );
}

fn load(mut commands: Commands, assets: Option<Res<WorldAssets>>) {
    let Some(assets) = assets else { return };
    let mut chain = assets.chain.lock_recover();
    let mut read = |name: &str| chain.read_file(name).ok().and_then(|b| Wdbc::new(b));
    let sections = read("DBFilesClient\\CharSections.dbc");
    let hair = read("DBFilesClient\\CharHairGeosets.dbc");
    let facial = read("DBFilesClient\\CharacterFacialHairStyles.dbc");
    drop(chain);

    type Sets = (
        BTreeSet<u8>,
        HashMap<u8, BTreeSet<u8>>,
        BTreeSet<u8>,
        BTreeSet<u8>,
        BTreeSet<u8>,
        BTreeSet<u8>,
    );
    let mut acc: HashMap<(u8, u8), Sets> = HashMap::new();
    // CharSections: ID, Race, Sex, SectionType, Variation, Color, Tex0-2, Flags; flag 0x1 is a row
    // the client never offers.
    if let Some(t) = sections.filter(|t| t.fields >= 10) {
        for r in 0..t.records {
            let [race, sex, ty, var, color] = [1, 2, 3, 4, 5].map(|f| t.u32(r, f) as u8);
            if t.u32(r, 9) & 1 != 0 {
                continue;
            }
            let e = acc.entry((race, sex)).or_default();
            match ty {
                0 if var == 0 => {
                    e.0.insert(color);
                }
                1 => {
                    e.1.entry(color).or_default().insert(var);
                    e.2.insert(var);
                }
                3 => {
                    e.4.insert(color);
                }
                _ => {}
            }
        }
    }
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
        .map(|(key, (mut skins, mut faces, mut all_faces, styles, mut colors, facial))| {
            // Texture dials have nothing to paint on a race that keeps its baked atlas.
            if !composites(key.0) {
                skins.clear();
                faces.clear();
                all_faces.clear();
                colors.clear();
            }
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
