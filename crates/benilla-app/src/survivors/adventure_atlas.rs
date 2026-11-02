//! The adventure atlas: the story's road, the dungeons and the raids, each a linear map of
//! location cards joined by the path between them, as on the original ASR's story map.
//! Playable battlegrounds are picked here as on the adventure panel; the rest show as locked.
//! Card art is the original ASR's zone photographs (`assets/atlas`, sources in `sources.json`);
//! a raid without one falls back to its own loading screen from the player's install.
use super::{data, front_end as skin, hud::HudFont, inventory::Equipment, Phase, Selection};
use crate::glue::GlueClicks;
use benilla_assets::{LockRecover, MapCatalogRes, WorldAssets};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Clone, Copy)]
pub(super) enum Action {
    Open(u8),
    Close,
    Tab(u8),
    Pick(usize),
}

#[derive(Resource, Default)]
pub(super) struct Atlas {
    pub open: bool,
    tab: u8,
    /// The card under the cursor, by index into [`PLACES`]; the details strip describes it.
    hover: Option<usize>,
    revision: u64,
}

#[derive(Clone, Copy)]
enum Side {
    Both,
    Alliance,
    Horde,
}

#[derive(Clone, Copy)]
enum Art {
    /// A photograph in `assets/atlas`.
    Photo(&'static str),
    /// The `Map.dbc` row whose loading screen stands in.
    Screen(u32),
}

struct Place {
    name: &'static str,
    /// The battleground's name in [`data`], where it differs from the card's.
    map: &'static str,
    side: Side,
    art: Art,
    line: &'static str,
    blurb: &'static str,
    /// The encounters of a place not yet built as a battleground.
    bosses: &'static [&'static str],
}

const fn place(
    name: &'static str,
    side: Side,
    art: Art,
    line: &'static str,
    blurb: &'static str,
) -> Place {
    Place {
        name,
        map: name,
        side,
        art,
        line,
        blurb,
        bosses: &[],
    }
}

use Art::{Photo, Screen};
use Side::{Alliance, Both, Horde};

static PLACES: &[Place] = &[
    // 0..=3: the Alliance road.
    place("Elwynn Forest", Alliance, Photo("elwynn"), "A hero is born",
        "Northshire's bells fade behind you. Gnolls, kobolds and the Defias test a new hero across the forests of Elwynn."),
    place("Westfall", Alliance, Photo("westfall"), "The Defias brotherhood",
        "Harvest golems and bandits overrun the farms the Stormwind guard abandoned."),
    place("The Deadmines", Alliance, Photo("deadmines"), "Van Cleef's hideout",
        "The Defias forge a juggernaut in the mines beneath Moonbrook."),
    place("Duskwood", Alliance, Photo("duskwood"), "Where the night never ends",
        "Worgen, ghouls and the restless dead stalk the darkened woods around Darkshire."),
    // 4..=7: the Horde road.
    place("Durotar", Horde, Photo("durotar"), "Strength and honour",
        "The red canyons around Orgrimmar ring with the cries of quilboar, harpies and the Burning Blade."),
    place("The Barrens", Horde, Photo("barrens"), "The long road south",
        "Raptors, quilboar and centaur roam a savanna that never seems to end."),
    place("Wailing Caverns", Horde, Photo("wailing"), "The druids of the Fang",
        "Nightmare twists the druids who sought to restore the Barrens."),
    place("Stonetalon Mountains", Horde, Photo("stonetalon"), "Peaks of the Venture Company",
        "Goblin shredders cut the ancient forests while elementals stir in the heights."),
    // 8..: where the roads meet, and on to the end.
    place("Stranglethorn Vale", Both, Photo("stv"), "Jungle of blood",
        "Trolls, pirates and the hunters of Nesingwary contest every clearing of the jungle."),
    place("Warsong Gulch", Both, Photo("wsg"), "The battle for the forest",
        "Silverwing sentinels and Warsong outriders fight over every felled tree of Ashenvale."),
    Place {
        map: "Fire Plume Ridge",
        ..place("Un'Goro Crater", Both, Photo("ungoro"), "Un'Goro's heart",
            "Devilsaurs and fire elementals guard the smoking crater at the heart of Un'Goro.")
    },
    place("Arathi Basin", Both, Photo("ab"), "Resources of the highlands",
        "The League of Arathor and the Defilers race for the farms, mines and mills of the basin."),
    place("Feralas", Both, Photo("feralas"), "The ruins of Dire Maul",
        "Ogres and the last Highborne guard the shattered halls of Eldre'Thalas."),
    place("Gurubashi Arena", Both, Photo("arena"), "Blood on the sand",
        "Every three hours the chest falls, and every brawler in Stranglethorn comes for it."),
    place("Scarlet Monastery", Both, Photo("sm"), "The Crusade's stronghold",
        "Zealots of the Scarlet Crusade guard their cathedral in the north of Tirisfal."),
    place("Alterac Valley", Both, Photo("av"), "The war in the mountains",
        "Stormpike and Frostwolf clash across the snowbound valley."),
    place("Blackrock Mountain", Both, Photo("brm"), "Fire beneath the mountain",
        "The Dark Iron dwarves and the black dragonflight hold the mountain between them."),
    place("Gates of Ahn'Qiraj", Both, Photo("aq"), "The war of the shifting sands",
        "The gong has rung. C'Thun's armies march out of Silithus."),
    place("Naxxramas", Both, Photo("naxx"), "The necropolis",
        "Kel'Thuzad's floating citadel looms over the Eastern Plaguelands."),
    // 19..: dungeons and raids off the story road.
    place("Hall of Thanes", Both, Screen(3065), "The ancient thanes",
        "Restless dead walk the great burial halls of the thanes."),
    Place {
        bosses: &["Hakkar the Soulflayer", "High Priest Thekal", "Bloodlord Mandokir", "Jin'do the Hexxer"],
        ..place("Zul'Gurub", Both, Screen(309), "The Soulflayer returns",
            "The Gurubashi priests summon Hakkar into the heart of Stranglethorn.")
    },
    Place {
        bosses: &["Onyxia"],
        ..place("Onyxia's Lair", Both, Screen(249), "The brood mother",
            "Deathwing's daughter waits in her lair beneath Dustwallow Marsh.")
    },
    Place {
        bosses: &["Lucifron", "Magmadar", "Golemagg", "Majordomo Executus", "Ragnaros"],
        ..place("Molten Core", Both, Screen(409), "The Firelord's domain",
            "Ragnaros stirs in the molten depths below Blackrock Mountain.")
    },
    Place {
        bosses: &["Razorgore", "Vaelastrasz", "Broodlord Lashlayer", "Chromaggus", "Nefarian"],
        ..place("Blackwing Lair", Both, Photo("brm"), "Nefarian's citadel",
            "The black dragonflight breeds its chromatic army atop Blackrock Spire.")
    },
    Place {
        bosses: &["Kurinnaxx", "General Rajaxx", "Moam", "Ossirian the Unscarred"],
        ..place("Ruins of Ahn'Qiraj", Both, Screen(509), "The scarab wall falls",
            "The qiraji swarm through the ruins beyond the Scarab Wall.")
    },
    place("Teldrassil", Alliance, Screen(1), "The tree of life",
        "Nightsabers and gnarlpine furbolgs prowl beneath the boughs of the great tree."),
    place("Mulgore", Horde, Screen(1), "The wide red land",
        "Plainstriders, prairie wolves and the quilboar roam the Tauren homeland."),
    place("Tirisfal Glades", Horde, Screen(0), "The shadowed beginning",
        "Forsaken and Scarlet patrols stalk the misted fields beyond Deathknell."),
    place("Dun Morogh", Alliance, Screen(0), "The snowy homeland",
        "Defeat Dun Morogh's final boss to unlock Dwarves and Gnomes."),
];

/// A map of cards on a seven-column grid, and the path drawn between them.
struct Road {
    rows: u8,
    /// `(place, column, row)`.
    cards: &'static [(usize, u8, u8)],
    /// Pairs of indices into `cards`.
    path: &'static [(usize, usize)],
}

/// The original story map: the two faction roads meet in Stranglethorn, climb to Un'Goro and
/// Arathi, and run back west along the bottom to Naxxramas.
const STORY: Road = Road {
    rows: 5,
    cards: &[
        (4, 0, 0), (5, 1, 0), (6, 2, 0), (7, 3, 0),
        (8, 4, 1), (9, 5, 1), (10, 6, 1),
        (0, 0, 2), (1, 1, 2), (2, 2, 2), (3, 3, 2), (11, 6, 2),
        (18, 0, 3), (17, 1, 3), (16, 2, 3), (15, 3, 3), (14, 4, 3), (13, 5, 3), (12, 6, 3),
        (25, 0, 4), (26, 1, 4), (27, 2, 4),
        (28, 3, 4),
    ],
    path: &[
        (0, 1), (1, 2), (2, 3), (3, 4),
        (7, 8), (8, 9), (9, 10), (10, 4),
        (4, 5), (5, 6), (6, 11), (11, 18),
        (18, 17), (17, 16), (16, 15), (15, 14), (14, 13), (13, 12),
        (19, 7), (20, 0), (21, 0),
        (22, 7),
    ],
};

const DUNGEONS: Road = Road {
    rows: 1,
    cards: &[(2, 0, 0), (6, 1, 0), (19, 2, 0), (14, 3, 0), (13, 4, 0), (16, 5, 0)],
    path: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)],
};

const RAIDS: Road = Road {
    rows: 1,
    cards: &[(20, 0, 0), (21, 1, 0), (22, 2, 0), (23, 3, 0), (24, 4, 0), (17, 5, 0), (18, 6, 0)],
    path: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6)],
};

const TABS: [(&str, &str, Road); 3] = [
    ("Story", "The road so far", STORY),
    ("Dungeons", "Five-player instances", DUNGEONS),
    ("Raids", "Raid instances", RAIDS),
];

/// Grid geometry, in percent of the map: a card fills 90% of its column, at the photos' 2:1
/// widened slightly, and a row is a card's height plus a fifth for the path between rows.
const COLUMNS: f32 = 7.;
const CARD_W: f32 = 90. / COLUMNS;
const CARD_ASPECT: f32 = 1.75;
const ROW_GAP: f32 = 1.25;

#[derive(Component)]
struct Root;
#[derive(Component)]
struct CardArt(Art);
#[derive(Component)]
struct Card(usize);

/// The decoded photographs, built on first use.
#[derive(Resource, Default)]
struct Photos(HashMap<&'static str, Handle<Image>>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Atlas>()
        .init_resource::<Photos>()
        .add_systems(
            Update,
            (input, hover, draw, art).chain().after(super::hud::show_panels),
        );
}

fn photo_bytes(key: &str) -> Option<&'static [u8]> {
    macro_rules! photos {
        ($($k:literal),*) => {
            match key {
                $($k => Some(include_bytes!(concat!("../../assets/atlas/", $k, ".png")).as_slice()),)*
                _ => None,
            }
        };
    }
    photos!(
        "ab", "aq", "arena", "av", "barrens", "brm", "deadmines", "durotar", "duskwood",
        "elwynn", "feralas", "naxx", "sm", "stonetalon", "stv", "ungoro", "wailing",
        "westfall", "wsg"
    )
}

/// The place's index into [`data::MAPS`] when it can be played in this build.
fn playable(p: &Place) -> Option<usize> {
    data::MAPS
        .iter()
        .position(|m| m.name == p.map)
        .filter(|&i| super::dungeon::available(&data::MAPS[i]))
}

fn status(p: &Place, selection: &Selection) -> (String, Color) {
    if playable(p).is_some_and(|map| !selection.can_visit(map)) {
        return ("Other faction territory".into(), Color::srgb(0.95, 0.42, 0.36));
    }
    let side = match p.side {
        Both => None,
        Alliance => Some(("Alliance", Color::srgb(0.42, 0.62, 1.0))),
        Horde => Some(("Horde", Color::srgb(0.95, 0.36, 0.3))),
    };
    let locked = Color::srgb(0.95, 0.42, 0.36);
    match (playable(p), side) {
        (Some(_), Some((side, color))) => (side.into(), color),
        (Some(_), None) => ("Open".into(), skin::GREEN),
        (None, _) if data::MAPS.iter().any(|m| m.name == p.map) => {
            ("Dungeon pack required".into(), locked)
        }
        (None, _) => ("Locked".into(), locked),
    }
}

fn input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    options: Res<super::options::Options>,
    coop: Res<super::coop::Coop>,
    gear: Res<Equipment>,
    actions: Query<(Entity, &Action, &skin::Control)>,
    mut atlas: ResMut<Atlas>,
    mut selection: ResMut<Selection>,
) {
    if *phase.get() != Phase::Menu {
        if atlas.open {
            atlas.open = false;
            atlas.revision += 1;
        }
        return;
    }
    if options.open || coop.open || gear.open {
        return;
    }
    if atlas.open && keys.just_pressed(KeyCode::Escape) {
        atlas.open = false;
        atlas.revision += 1;
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _, c)| c.enabled && clicks.hit(*e))
        .map(|(_, a, _)| *a)
    else {
        return;
    };
    match action {
        Action::Open(tab) => {
            atlas.open = true;
            atlas.tab = tab.min(TABS.len() as u8 - 1);
            atlas.hover = None;
        }
        Action::Close => atlas.open = false,
        Action::Tab(tab) => {
            atlas.tab = tab;
            atlas.hover = None;
        }
        Action::Pick(i) => {
            if selection.map != i && selection.can_visit(i) && data::MAPS.get(i).is_some_and(super::dungeon::available) {
                selection.select_map(i);
            }
        }
    }
    atlas.revision += 1;
}

/// The details strip follows the cursor; leaving every card returns it to the selected map.
fn hover(cards: Query<(&Card, &Interaction)>, mut atlas: ResMut<Atlas>) {
    if !atlas.open {
        return;
    }
    let now = cards
        .iter()
        .find(|(_, i)| **i != Interaction::None)
        .map(|(c, _)| c.0);
    if now.is_some() && now != atlas.hover {
        atlas.hover = now;
        atlas.revision += 1;
    }
}

fn card(p: &mut ChildSpawnerCommands, f: &Handle<Font>, index: usize, at: (f32, f32), height: f32, selected: bool, selection: &Selection) {
    let place = &PLACES[index];
    let open = playable(place).filter(|&map| selection.can_visit(map));
    let (state, color) = status(place, selection);
    let mut card = p.spawn((
        Button,
        Card(index),
        Action::Pick(open.unwrap_or(usize::MAX)),
        skin::Control {
            primary: false,
            selected,
            enabled: open.is_some(),
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(at.0),
            top: Val::Percent(at.1),
            width: Val::Percent(CARD_W),
            height: Val::Percent(height),
            border: UiRect::all(Val::Px(if selected { 2. } else { 1. })),
            padding: UiRect::axes(Val::Px(12.), Val::Px(8.)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(if selected { skin::GOLD } else { skin::EDGE }),
    ));
    if selected {
        card.insert(BoxShadow::new(
            skin::GOLD.with_alpha(0.45),
            Val::Px(0.),
            Val::Px(0.),
            Val::Px(2.),
            Val::Px(10.),
        ));
    }
    card.with_children(|p| {
        let fill = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            top: Val::Px(0.),
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        };
        p.spawn((CardArt(place.art), ImageNode::default(), fill.clone()));
        // Darker toward the name, and darker still while locked.
        p.spawn((
            fill,
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::auto(Color::srgba(0., 0., 0., if open.is_some() { 0.05 } else { 0.35 })),
                ColorStop::auto(Color::srgba(0., 0., 0., if open.is_some() { 0.7 } else { 0.85 })),
            ])),
        ));
        skin::label(p, f, place.name, 19., if open.is_some() { skin::GOLD } else { skin::TEXT });
        skin::label(p, f, state, 12., color);
    });
}

/// The path between two cards: straight along a row or column, otherwise out of the first
/// card's side to the second's column and on to its nearest edge.
fn path(p: &mut ChildSpawnerCommands, a: (f32, f32), b: (f32, f32), height: f32) {
    let line = |p: &mut ChildSpawnerCommands, left: f32, top: f32, w: Option<f32>, h: Option<f32>| {
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(left),
                top: Val::Percent(top),
                width: w.map_or(Val::Px(3.), Val::Percent),
                height: h.map_or(Val::Px(3.), Val::Percent),
                ..default()
            },
            BackgroundColor(Color::srgb(0.72, 0.55, 0.24)),
        ));
    };
    let (ax, ay, bx, by) = (a.0, a.1, b.0, b.1);
    let mid = |y: f32| y + height / 2.;
    let center = |x: f32| x + CARD_W / 2.;
    if (ay - by).abs() < 0.01 {
        let (l, r) = if ax < bx { (ax, bx) } else { (bx, ax) };
        line(p, l + CARD_W, mid(ay), Some(r - l - CARD_W), None);
    } else if (ax - bx).abs() < 0.01 {
        let (t, d) = if ay < by { (ay, by) } else { (by, ay) };
        line(p, center(ax), t + height, None, Some(d - t - height));
    } else {
        // Out of the side facing the other card, to the gap before its column, then along it.
        let gap = (100. / COLUMNS - CARD_W) / 2.;
        if bx > ax {
            let x = bx - gap;
            line(p, ax + CARD_W, mid(ay), Some(x - ax - CARD_W), None);
            line(p, x, mid(by), Some(gap), None);
            let (t, d) = (mid(ay).min(mid(by)), mid(ay).max(mid(by)));
            line(p, x, t, None, Some(d - t));
        } else {
            let x = bx + CARD_W + gap;
            line(p, x, mid(ay), Some(ax - x), None);
            line(p, bx + CARD_W, mid(by), Some(gap), None);
            let (t, d) = (mid(ay).min(mid(by)), mid(ay).max(mid(by)));
            line(p, x, t, None, Some(d - t));
        }
    }
}

fn chip(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: String, last: bool) {
    p.spawn((
        Node {
            padding: UiRect::axes(Val::Px(10.), Val::Px(3.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(11.)),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.03, 0.02, 0.85)),
        BorderColor::all(if last { skin::GOLD } else { Color::srgb(0.45, 0.22, 0.14) }),
    ))
    .with_children(|p| {
        skin::label(p, f, text, 13., Color::srgb(0.96, 0.66, 0.45));
    });
}

fn details(p: &mut ChildSpawnerCommands, f: &Handle<Font>, place: &Place, selection: &Selection) {
    let open = playable(place).filter(|&map| selection.can_visit(map));
    let (state, color) = status(place, selection);
    p.spawn((
        Node {
            width: Val::Percent(100.),
            flex_shrink: 0.,
            padding: UiRect::axes(Val::Px(18.), Val::Px(12.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(5.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.),
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        p.spawn(Node { column_gap: Val::Px(14.), align_items: AlignItems::Baseline, ..default() })
            .with_children(|p| {
                skin::label(p, f, place.name, 26., skin::GOLD);
                skin::label(p, f, place.line, 16., Color::srgb(0.96, 0.66, 0.45));
                skin::label(p, f, state, 14., color);
                if let Some(map) = data::all_maps().iter().find(|m| m.name == place.map) {
                    skin::label(p, f, map.zone, 12., skin::MUTED);
                }
            });
        skin::label(p, f, place.blurb, 15., skin::TEXT);
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.),
            row_gap: Val::Px(6.),
            margin: UiRect::top(Val::Px(4.)),
            ..default()
        })
        .with_children(|p| {
            let chips: Vec<String> = match open {
                Some(i) => data::roster(i)
                    .bosses
                    .iter()
                    .map(|b| {
                        let at = b.from as u32;
                        if at == 0 {
                            b.name.to_string()
                        } else {
                            format!("{}:{:02} · {}", at / 60, at % 60, b.name)
                        }
                    })
                    .collect(),
                None => data::all_maps()
                    .iter()
                    .find(|m| m.name == place.map)
                    .map_or_else(
                        || place.bosses.iter().map(|b| b.to_string()).collect(),
                        |m| m.boss_names().map(str::to_string).collect(),
                    ),
            };
            let n = chips.len();
            for (i, text) in chips.into_iter().enumerate() {
                chip(p, f, text, i + 1 == n);
            }
        });
    });
}

fn draw(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    atlas: Res<Atlas>,
    selection: Res<Selection>,
    phase: Res<State<Phase>>,
    roots: Query<Entity, With<Root>>,
    mut last: Local<Option<(u64, u32, bool)>>,
) {
    let show = atlas.open && *phase.get() == Phase::Menu;
    let key = (atlas.revision, selection.version, show);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    for e in &roots {
        commands.entity(e).despawn();
    }
    if !show {
        return;
    }
    let Some(font) = font else {
        *last = None;
        return;
    };
    let f = &font.0;
    let tab = usize::from(atlas.tab).min(TABS.len() - 1);
    let road = &TABS[tab].2;
    let current = data::MAPS.get(selection.map).map(|m| m.name);
    let shown = atlas
        .hover
        .filter(|h| road.cards.iter().any(|c| c.0 == *h))
        .or_else(|| road.cards.iter().map(|c| c.0).find(|&i| current == Some(PLACES[i].map)))
        .unwrap_or(road.cards[0].0);
    // A row's height in percent of the map, and a card's within it.
    let row = 100. / f32::from(road.rows);
    let card_h = row / ROW_GAP;
    // The map keeps the cards' aspect at any width: its height is rows of cards plus gaps.
    let aspect = COLUMNS / (0.9 / CARD_ASPECT * ROW_GAP * f32::from(road.rows));
    let place_at = |(_, c, r): (usize, u8, u8)| {
        (f32::from(c) * 100. / COLUMNS, f32::from(r) * row + (row - card_h) / 2.)
    };
    commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.82)),
            GlobalZIndex(400),
        ))
        .with_children(|root| {
            root.spawn(skin::panel(Node {
                width: Val::Percent(96.),
                max_width: Val::Px(1600.),
                max_height: Val::Percent(96.),
                padding: UiRect::all(Val::Px(18.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.),
                ..default()
            }))
            .with_children(|p| {
                p.spawn(Node { column_gap: Val::Px(12.), flex_shrink: 0., ..default() })
                    .with_children(|p| {
                        for (i, (title, detail, _)) in TABS.iter().enumerate() {
                            p.spawn(Node { width: Val::Px(260.), ..default() }).with_children(|p| {
                                skin::button(p, f, title, detail, Action::Tab(i as u8), true, i == tab, true);
                            });
                        }
                        p.spawn(Node { flex_grow: 1., ..default() });
                        p.spawn(Node { width: Val::Px(120.), ..default() }).with_children(|p| {
                            skin::button(p, f, "Close", "Esc", Action::Close, false, false, true);
                        });
                    });
                p.spawn(Node {
                    width: Val::Percent(100.),
                    aspect_ratio: Some(aspect),
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|map| {
                    for &(a, b) in road.path {
                        path(map, place_at(road.cards[a]), place_at(road.cards[b]), card_h);
                    }
                    for &c in road.cards {
                        card(map, f, c.0, place_at(c), card_h, current == Some(PLACES[c.0].map), &selection);
                    }
                });
                details(p, f, &PLACES[shown], &selection);
            });
        });
}

/// Photographs from the bundle; the loading screen through `Map.dbc` → `LoadingScreens.dbc`.
fn art(
    maps: Option<Res<MapCatalogRes>>,
    mut assets: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut photos: ResMut<Photos>,
    mut screens: Local<Option<benilla_formats::LoadingScreenCatalog>>,
    mut nodes: Query<(&CardArt, &mut ImageNode), Added<CardArt>>,
) {
    for (art, mut node) in &mut nodes {
        match art.0 {
            Art::Photo(key) => {
                let handle = photos.0.entry(key).or_insert_with(|| {
                    photo_bytes(key).map_or_else(Handle::default, |b| super::card_view::png(b, &mut images))
                });
                node.image = handle.clone();
            }
            Art::Screen(map) => {
                let (Some(maps), Some(assets)) = (maps.as_ref(), assets.as_mut()) else {
                    continue;
                };
                if screens.is_none() {
                    *screens = benilla_formats::load_loading_screens(&mut assets.chain.lock_recover()).ok();
                }
                let path = screens
                    .as_ref()
                    .zip(maps.0.loading_screen_id(map))
                    .and_then(|(s, id)| s.path(id));
                if let Some(image) = path.and_then(|p| assets.sprite_texture(p, &mut images)) {
                    node.image = image;
                }
            }
        }
    }
}
