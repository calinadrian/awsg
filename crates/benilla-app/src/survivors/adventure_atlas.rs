//! The adventure atlas: the story's zones, the dungeons and the raids, each on its own tab.
//! Playable battlegrounds are picked here as on the adventure panel; the rest show what
//! unlocks them. Card art is each map's own loading screen, read from the player's install.
use super::{data, front_end as skin, hud::HudFont, inventory::Equipment, Phase, Selection};
use crate::glue::GlueClicks;
use benilla_assets::{LockRecover, MapCatalogRes, WorldAssets};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Story,
    Dungeons,
    Raids,
}

#[derive(Component, Clone, Copy)]
pub(super) enum Action {
    Open,
    Close,
    Tab(u8),
    Pick(usize),
}

#[derive(Resource, Default)]
pub(super) struct Atlas {
    pub open: bool,
    tab: u8,
    revision: u64,
}

#[derive(Clone, Copy)]
enum Side {
    Both,
    Alliance,
    Horde,
}

struct Entry {
    name: &'static str,
    tab: Tab,
    side: Side,
    /// The `Map.dbc` row whose loading screen is the card's art.
    art: u32,
    line: &'static str,
    blurb: &'static str,
    /// The encounters of a raid not yet built as a battleground.
    bosses: &'static [&'static str],
}

const fn entry(
    name: &'static str,
    tab: Tab,
    side: Side,
    art: u32,
    line: &'static str,
    blurb: &'static str,
) -> Entry {
    Entry {
        name,
        tab,
        side,
        art,
        line,
        blurb,
        bosses: &[],
    }
}

/// In the order the road runs: the starting zones, then the contested lands beyond them.
static ATLAS: &[Entry] = &[
    entry("Elwynn Forest", Tab::Story, Side::Alliance, 0, "A hero is born",
        "Northshire's bells fade behind you. Gnolls, kobolds and the Defias test a new hero across the forests of Elwynn."),
    entry("Durotar", Tab::Story, Side::Horde, 1, "Strength and honour",
        "The red canyons around Orgrimmar ring with the cries of quilboar, harpies and the Burning Blade."),
    entry("Teldrassil", Tab::Story, Side::Alliance, 1, "Under the World Tree",
        "Corruption creeps through the boughs of Teldrassil, and the night elves call their sentinels to arms."),
    entry("Dun Morogh", Tab::Story, Side::Alliance, 0, "Snow and steel",
        "Troggs boil up from below Ironforge while frostmane trolls hold the passes."),
    entry("Mulgore", Tab::Story, Side::Horde, 1, "The plains of the tauren",
        "Centaur raiders and the Venture Company threaten the grasslands below Thunder Bluff."),
    entry("Tirisfal Glades", Tab::Story, Side::Horde, 0, "The Forsaken rise",
        "Scarlet zealots and the mindless Scourge haunt the fields above the Undercity."),
    entry("Westfall", Tab::Story, Side::Both, 0, "The Defias brotherhood",
        "Harvest golems and bandits overrun the farms the Stormwind guard abandoned."),
    entry("The Barrens", Tab::Story, Side::Horde, 1, "The long road south",
        "Raptors, quilboar and centaur roam a savanna that never seems to end."),
    entry("Redridge Mountains", Tab::Story, Side::Both, 0, "Blackrock at the gates",
        "Blackrock orcs and gnolls press on Lakeshire from the burning hills."),
    entry("Duskwood", Tab::Story, Side::Both, 0, "Where the night never ends",
        "Worgen, ghouls and the restless dead stalk the darkened woods around Darkshire."),
    entry("Stonetalon Mountains", Tab::Story, Side::Horde, 1, "Peaks of the Venture Company",
        "Goblin shredders cut the ancient forests while elementals stir in the heights."),
    entry("Stranglethorn Vale", Tab::Story, Side::Both, 0, "Jungle of blood",
        "Trolls, pirates and the hunters of Nesingwary contest every clearing of the jungle."),
    entry("Fire Plume Ridge", Tab::Story, Side::Both, 1, "Un'Goro's heart",
        "Devilsaurs and fire elementals guard the smoking crater at the heart of Un'Goro."),
    entry("Mount Hyjal", Tab::Story, Side::Both, 1, "The World Tree's watch",
        "Demons gather once more on the slopes where Archimonde fell."),
    entry("The Dark Portal", Tab::Story, Side::Both, 0, "The doorway to Outland",
        "The Burning Legion pours through the Portal into the Blasted Lands."),
    entry("The Deadmines", Tab::Dungeons, Side::Both, 36, "Van Cleef's hideout",
        "The Defias forge a juggernaut in the mines beneath Moonbrook."),
    entry("Wailing Caverns", Tab::Dungeons, Side::Both, 43, "The druids of the Fang",
        "Nightmare twists the druids who sought to restore the Barrens."),
    entry("Hall of Thanes", Tab::Dungeons, Side::Both, 3065, "The ancient thanes",
        "Restless dead walk the great burial halls of the thanes."),
    entry("Gurubashi Arena", Tab::Dungeons, Side::Both, 0, "Blood on the sand",
        "Every three hours the chest falls, and every brawler in Stranglethorn comes for it."),
    Entry {
        bosses: &["Hakkar the Soulflayer", "High Priest Thekal", "Bloodlord Mandokir", "Jin'do the Hexxer"],
        ..entry("Zul'Gurub", Tab::Raids, Side::Both, 309, "The Soulflayer returns",
            "The Gurubashi priests summon Hakkar into the heart of Stranglethorn.")
    },
    Entry {
        bosses: &["Onyxia"],
        ..entry("Onyxia's Lair", Tab::Raids, Side::Both, 249, "The brood mother",
            "Deathwing's daughter waits in her lair beneath Dustwallow Marsh.")
    },
    Entry {
        bosses: &["Lucifron", "Magmadar", "Golemagg", "Majordomo Executus", "Ragnaros"],
        ..entry("Molten Core", Tab::Raids, Side::Both, 409, "The Firelord's domain",
            "Ragnaros stirs in the molten depths below Blackrock Mountain.")
    },
    Entry {
        bosses: &["Razorgore", "Vaelastrasz", "Broodlord Lashlayer", "Chromaggus", "Nefarian"],
        ..entry("Blackwing Lair", Tab::Raids, Side::Both, 469, "Nefarian's citadel",
            "The black dragonflight breeds its chromatic army atop Blackrock Spire.")
    },
    Entry {
        bosses: &["Kurinnaxx", "General Rajaxx", "Moam", "Ossirian the Unscarred"],
        ..entry("Ruins of Ahn'Qiraj", Tab::Raids, Side::Both, 509, "The scarab wall falls",
            "The qiraji swarm through the ruins beyond the Scarab Wall.")
    },
    entry("Gates of Ahn'Qiraj", Tab::Raids, Side::Both, 531, "The war of the shifting sands",
        "The gong has rung. C'Thun's armies march out of Silithus."),
    entry("Naxxramas", Tab::Raids, Side::Both, 533, "The necropolis",
        "Kel'Thuzad's floating citadel looms over the Eastern Plaguelands."),
];

const TABS: [(Tab, &str, &str); 3] = [
    (Tab::Story, "Story", "The road so far"),
    (Tab::Dungeons, "Dungeons", "Five-player instances"),
    (Tab::Raids, "Raids", "Raid instances"),
];

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Art(u32);
#[derive(Component)]
struct Scroll;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Atlas>().add_systems(
        Update,
        (input, draw, art, scroll)
            .chain()
            .after(super::hud::show_panels),
    );
}

/// What a card's entry is in this build: its index into [`data::MAPS`] when playable, or why not.
fn status(e: &Entry) -> Result<usize, &'static str> {
    match data::MAPS.iter().position(|m| m.name == e.name) {
        Some(i) if super::dungeon::available(&data::MAPS[i]) => Ok(i),
        Some(_) => Err("Locked · dungeon pack required"),
        None if e.tab == Tab::Raids => Err("Locked · raid in development"),
        None => Err("Locked · coming in a later update"),
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
        Action::Open => {
            atlas.open = true;
            atlas.tab = ATLAS
                .iter()
                .find(|e| data::MAPS.get(selection.map).is_some_and(|m| m.name == e.name))
                .map_or(0, |e| TABS.iter().position(|t| t.0 == e.tab).unwrap_or(0) as u8);
        }
        Action::Close => atlas.open = false,
        Action::Tab(tab) => atlas.tab = tab,
        Action::Pick(i) => {
            if selection.map != i && data::MAPS.get(i).is_some_and(super::dungeon::available) {
                selection.map = i;
                selection.version += 1;
            }
        }
    }
    atlas.revision += 1;
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
        BorderColor::all(if last {
            skin::GOLD
        } else {
            Color::srgb(0.45, 0.22, 0.14)
        }),
    ))
    .with_children(|p| {
        skin::label(p, f, text, 14., Color::srgb(0.96, 0.66, 0.45));
    });
}

fn card(p: &mut ChildSpawnerCommands, f: &Handle<Font>, e: &Entry, selected: bool) {
    let status = status(e);
    let open = status.is_ok();
    p.spawn((
        Button,
        Action::Pick(*status.as_ref().unwrap_or(&usize::MAX)),
        skin::Control {
            primary: false,
            selected,
            enabled: open,
        },
        Node {
            width: Val::Percent(100.),
            min_height: Val::Px(if selected { 230. } else { 120. }),
            padding: UiRect::axes(Val::Px(22.), Val::Px(16.)),
            border: UiRect::all(Val::Px(if selected { 2. } else { 1. })),
            flex_direction: FlexDirection::Column,
            justify_content: if selected {
                JustifyContent::FlexStart
            } else {
                JustifyContent::Center
            },
            row_gap: Val::Px(4.),
            overflow: Overflow::clip(),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(if selected { skin::GOLD } else { skin::EDGE }),
    ))
    .with_children(|p| {
        // Loading screens are 4:3; the card shows a band across their middle.
        p.spawn((
            Art(e.art),
            ImageNode::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Percent(-60.),
                width: Val::Percent(100.),
                ..default()
            },
        ));
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Px(0.),
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., if open { 0.45 } else { 0.62 })),
        ));
        skin::label(
            p,
            f,
            e.name,
            if selected { 34. } else { 28. },
            if open { skin::GOLD } else { skin::TEXT },
        );
        let side = match e.side {
            Side::Both => None,
            Side::Alliance => Some(("Alliance", Color::srgb(0.45, 0.65, 1.0))),
            Side::Horde => Some(("Horde", Color::srgb(0.95, 0.35, 0.3))),
        };
        match status {
            Ok(_) => skin::label(
                p,
                f,
                match side {
                    Some((side, _)) => format!("{side}: {}", e.line),
                    None => e.line.to_string(),
                },
                17.,
                side.map_or(Color::srgb(0.96, 0.66, 0.45), |s| s.1),
            ),
            Err(why) => skin::label(p, f, why, 16., Color::srgb(0.95, 0.42, 0.36)),
        };
        if !selected {
            return;
        }
        skin::label(p, f, e.blurb, 16., skin::TEXT);
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.),
            row_gap: Val::Px(6.),
            margin: UiRect::top(Val::Px(10.)),
            ..default()
        })
        .with_children(|p| {
            let chips: Vec<String> = match status {
                Ok(i) => data::roster(i)
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
                Err(_) => data::all_maps()
                    .iter()
                    .find(|m| m.name == e.name)
                    .map_or_else(
                        || e.bosses.iter().map(|b| b.to_string()).collect(),
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
    let tab = TABS[usize::from(atlas.tab).min(TABS.len() - 1)].0;
    let current = data::MAPS.get(selection.map).map(|m| m.name);
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.8)),
            GlobalZIndex(400),
        ))
        .with_children(|root| {
            root.spawn(skin::panel(Node {
                width: Val::Percent(70.),
                max_width: Val::Px(1120.),
                height: Val::Percent(92.),
                padding: UiRect::all(Val::Px(18.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.),
                overflow: Overflow::clip(),
                ..default()
            }))
            .with_children(|p| {
                p.spawn(Node {
                    column_gap: Val::Px(12.),
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|p| {
                    for (i, (t, title, detail)) in TABS.iter().enumerate() {
                        p.spawn(Node {
                            flex_grow: 1.,
                            flex_basis: Val::Px(0.),
                            ..default()
                        })
                        .with_children(|p| {
                            skin::button(
                                p,
                                f,
                                title,
                                detail,
                                Action::Tab(i as u8),
                                true,
                                *t == tab,
                                true,
                            );
                        });
                    }
                    p.spawn(Node {
                        width: Val::Px(110.),
                        flex_shrink: 0.,
                        ..default()
                    })
                    .with_children(|p| {
                        skin::button(p, f, "Close", "", Action::Close, false, false, true);
                    });
                });
                p.spawn((
                    Scroll,
                    ScrollPosition::default(),
                    bevy::ui::RelativeCursorPosition::default(),
                    Node {
                        flex_grow: 1.,
                        min_height: Val::Px(0.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    for e in ATLAS.iter().filter(|e| e.tab == tab) {
                        card(p, f, e, current == Some(e.name));
                    }
                });
            });
        });
}

/// Each card's loading screen, through `Map.dbc` → `LoadingScreens.dbc`, as the world's own cover.
fn art(
    maps: Option<Res<MapCatalogRes>>,
    mut assets: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut screens: Local<Option<benilla_formats::LoadingScreenCatalog>>,
    mut nodes: Query<(&Art, &mut ImageNode), Added<Art>>,
) {
    if nodes.is_empty() {
        return;
    }
    let (Some(maps), Some(assets)) = (maps, assets.as_mut()) else {
        return;
    };
    if screens.is_none() {
        *screens = benilla_formats::load_loading_screens(&mut assets.chain.lock_recover()).ok();
    }
    let Some(screens) = screens.as_ref() else {
        return;
    };
    for (map, mut node) in &mut nodes {
        let path = maps.0.loading_screen_id(map.0).and_then(|id| screens.path(id));
        if let Some(image) = path.and_then(|p| assets.sprite_texture(p, &mut images)) {
            node.image = image;
        }
    }
}

fn scroll(
    atlas: Res<Atlas>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut columns: Query<
        (
            &mut ScrollPosition,
            &ComputedNode,
            &bevy::ui::RelativeCursorPosition,
        ),
        With<Scroll>,
    >,
) {
    if !atlas.open {
        return;
    }
    let delta: f32 = wheel
        .read()
        .map(|e| {
            e.y * if e.unit == bevy::input::mouse::MouseScrollUnit::Line {
                40.
            } else {
                1.
            }
        })
        .sum();
    for (mut scroll, node, cursor) in &mut columns {
        if cursor
            .normalized
            .is_some_and(|p| (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y))
        {
            let limit = (node.content_size.y - node.size.y).max(0.) * node.inverse_scale_factor();
            scroll.0.y = (scroll.0.y - delta).clamp(0., limit);
        }
    }
}
