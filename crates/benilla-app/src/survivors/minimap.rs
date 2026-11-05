//! The minimap at the head of the right column ([`super::right_column`]): a square map in a steel
//! frame, the close, day and zoom knobs down its left, and a plate under it with the zone and the
//! player's local time.
//!
//! The map is composited here each frame: the ADT tiles through `md5translate.trs` (north up,
//! 533.33 yd per 256-px tile), then the player arrow, then the red `ObjectIcons` cell per live
//! enemy, the dot the reference draws for a tracked creature. Sizes are the engine minimap's
//! ([`crate::minimap`]), measured against its 140.8-px blip basis.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use benilla_assets::{LockRecover, MapCatalogRes, WorldAssets};
use benilla_formats::{tile_to_world, world_to_tile, MinimapTranslate};
use benilla_world::world_map::CurrentMap;
use benilla_world::world_point::WorldPoint;

use super::damage_meter::RightStack;
use super::enemies::Enemy;
use super::hero::Hero;
use super::hud::{HudFont, UiArt};
use super::right_column::{self as col, HudSizes, Slot};
use super::Run;
use crate::area::AreaTableRes;

/// The composite's edge in texels; the 140-unit widget is ~200 px at 1080p.
const SIDE: usize = 192;
const TILE_YARDS: f32 = 533.333_3;
const TILE_PX: usize = 256;
const CHUNK_YARDS: f32 = TILE_YARDS / 16.0;
/// The outdoor zoom table (`0x8116d0`): view diameter in chunks per `Minimap:SetZoom` index.
const ZOOM_CHUNKS: [f32; 6] = [14.0, 12.0, 10.0, 8.0, 6.0, 4.0];
/// Fully in: the packs that matter close in from ~30 yd, which the wider zooms crowd to the middle.
const START_ZOOM: usize = 5;

/// The engine minimap's blip basis: the stock widget at 140.8 px on the 1024×768 screen.
const BLIP_BASIS_PX: f32 = 140.8;
const DOT_PX: f32 = 8.0;
/// `ObjectIcons.blp` cell 1 (left, right, top, bottom): a unit passing creature tracking.
const TRACKED_UNIT_CELL: [f32; 4] = [0.25, 0.5, 0.0, 0.25];
const ARROW_PX: f32 = 33.6;
const ARROW_OFFSET_PX: Vec2 = Vec2::new(0.51, -1.73);

/// `GAMETIME_DAWN` and `GAMETIME_DUSK`, minutes into the day.
const DAWN: u32 = 5 * 60 + 30;
const DUSK: u32 = 21 * 60;

/// `FactionGroup.dbc` masks.
const GROUP_ALLIANCE: u32 = 2;
const GROUP_HORDE: u32 = 4;

#[derive(Component)]
pub(super) struct ZoneText;

/// The local time on the zone plate.
#[derive(Component)]
pub(super) struct ClockText;

/// The cluster's root, built at this scale.
#[derive(Component)]
pub(super) struct MapCluster(f32);

/// A knob's glyph, dimmed when its button can do nothing.
#[derive(Component)]
pub(super) struct KnobGlyph(MapButton);

/// The map and everything but the close knob: hidden by that knob.
#[derive(Component)]
pub(super) struct MapBody;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapButton {
    ZoomIn,
    ZoomOut,
    Toggle,
}

/// Decoded RGBA8 art the composite samples.
struct Rgba {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Rgba {
    fn decode(assets: &mut WorldAssets, path: &str) -> Option<Self> {
        let (w, h, px) = assets.decode_rgba(path)?;
        Some(Self {
            w: w as usize,
            h: h as usize,
            px,
        })
    }

    /// Bilinear RGBA at texel-space `(x, y)`, clamped to the edge.
    fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        let x = (x - 0.5).clamp(0.0, (self.w - 1) as f32);
        let y = (y - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let at = |x: usize, y: usize| {
            let i = (y * self.w + x) * 4;
            [0, 1, 2, 3].map(|c| f32::from(self.px[i + c]))
        };
        let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
        [0, 1, 2, 3].map(|k| {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bottom = c[k] + (d[k] - c[k]) * fx;
            top + (bottom - top) * fy
        })
    }
}

struct Art {
    translate: MinimapTranslate,
    arrow: Option<Rgba>,
    icons: Option<Rgba>,
}

#[derive(Resource)]
pub(super) struct MiniMap {
    zoom: usize,
    shown: bool,
    image: Handle<Image>,
    art: Option<Art>,
    /// Decoded 256² tiles by ADT index, `None` where no art was authored, for the map `dir` names.
    tiles: HashMap<(u32, u32), Option<Vec<u8>>>,
    dir: String,
    buf: Vec<u8>,
}

impl MiniMap {
    pub(super) fn new(images: &mut Assets<Image>) -> Self {
        let image = Image::new(
            Extent3d {
                width: SIDE as u32,
                height: SIDE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0; SIDE * SIDE * 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        Self {
            zoom: START_ZOOM,
            shown: true,
            image: images.add(image),
            art: None,
            tiles: HashMap::new(),
            dir: String::new(),
            buf: vec![0; SIDE * SIDE * 4],
        }
    }

    fn view_radius(&self) -> f32 {
        ZOOM_CHUNKS[self.zoom] * 0.5 * CHUNK_YARDS
    }
}

/// Build the cluster at the head of the right column, again when its size setting changes.
pub(super) fn build(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    sizes: Res<HudSizes>,
    map: Option<Res<MiniMap>>,
    stacks: Query<Entity, With<RightStack>>,
    clusters: Query<(Entity, &MapCluster)>,
) {
    let (Some(font), Some(map)) = (font, map) else {
        return;
    };
    let Ok(stack) = stacks.single() else { return };
    let k = sizes.map;
    if let Some((e, c)) = clusters.iter().next() {
        if c.0 == k {
            return;
        }
        commands.entity(e).despawn();
    }
    let root = spawn_cluster(&mut commands, &font.0, &map, k);
    commands.entity(root).insert((
        ChildOf(stack),
        Slot(0),
        super::hud_layout::Movable::new("minimap", "Minimap"),
    ));
}

/// The cluster's height in units: the framed map and the zone plate under it, or the close knob
/// alone while the map is hidden.
fn cluster_height(shown: bool) -> f32 {
    if shown {
        188.0
    } else {
        20.0
    }
}

/// The 210-unit cluster at scale `k`: the framed 164-unit map at its right, the knobs down its left.
fn spawn_cluster(commands: &mut Commands, f: &Handle<Font>, map: &MiniMap, k: f32) -> Entity {
    let px = |v: f32| Val::Px(v * k);
    let at = |left: f32, top: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(w),
        height: px(h),
        ..default()
    };
    let knob = |c: &mut ChildSpawnerCommands, node: Node, size: f32| {
        c.spawn((
            Node {
                width: px(size),
                height: px(size),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::MAX,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..node
            },
            BackgroundColor(col::KNOB),
            BorderColor::all(col::STEEL_LO),
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
        ))
        .id()
    };
    // A knob's glyph as bars, not a character, so it sits on the knob's exact centre: `len`
    // long and 1.5 thick, one bar for "-", two crossed for "+", the cross turned for "x".
    let glyph = |c: &mut ChildSpawnerCommands, button: MapButton, len: f32, bars: &[f32]| {
        for &turn in bars {
            c.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|centre| {
                centre.spawn((
                    KnobGlyph(button),
                    Node {
                        width: px(len),
                        height: px(1.5),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(col::STEEL_HI),
                    UiTransform::from_rotation(Rot2::degrees(turn)),
                ));
            });
        }
    };
    commands
        .spawn((
            MapCluster(k),
            Node {
                width: px(210.0),
                height: px(cluster_height(map.shown)),
                ..default()
            },
        ))
        .with_children(|c| {
            // The boss bars' frame: a black line, the grey edge, a dark inset, then the map.
            c.spawn((
                MapBody,
                col::plate(Node {
                    padding: UiRect::all(Val::Px(2.0)),
                    ..at(46.0, 4.0, 164.0, 164.0)
                }),
            ))
            .with_children(|frame| {
                frame.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(map.image.clone()),
                    Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
                ));
            });
            // North: a steel diamond on the frame's top edge.
            c.spawn((
                MapBody,
                col::diamond(at(121.5, -2.5, 13.0, 13.0), 13.0 * k, col::STEEL_LO),
            ));
            c.spawn((
                MapBody,
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..at(121.5, -2.5, 13.0, 13.0)
                },
            ))
            .with_children(|n| {
                n.spawn((
                    Text::new("N"),
                    TextFont {
                        font: f.clone(),
                        font_size: 8.0 * k,
                        ..default()
                    },
                    TextColor(Color::srgb(0.043, 0.055, 0.078)),
                ));
            });
            // The zone and the local time.
            c.spawn((
                MapBody,
                col::plate(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(px(6.0)),
                    column_gap: px(6.0),
                    overflow: Overflow::clip(),
                    ..at(46.0, 172.0, 164.0, 16.0)
                }),
            ))
            .with_children(|plate| {
                plate.spawn((ZoneText, col::label(f, "", 10.0 * k, col::TEXT_HI)));
                plate.spawn((ClockText, col::label(f, "", 10.0 * k, col::STEEL_HI)));
            });
            let x = knob(c, at(28.0, 6.0, 14.0, 14.0), 14.0);
            c.commands().entity(x).insert((MapButton::Toggle, Button));
            c.commands()
                .entity(x)
                .with_children(|b| glyph(b, MapButton::Toggle, 7.0, &[45.0, -45.0]));
            // `GameTimeFrame`'s sun or moon, by the run's hour.
            let tod = if (DAWN..DUSK).contains(&super::hero::MINUTE_OF_DAY) {
                0.0
            } else {
                0.5
            };
            let day = knob(c, at(26.0, 28.0, 18.0, 18.0), 18.0);
            c.commands().entity(day).insert(MapBody).with_children(|b| {
                b.spawn((
                    Node {
                        width: px(16.0),
                        height: px(16.0),
                        border_radius: BorderRadius::MAX,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::crop(
                        "Interface\\Minimap\\UI-TOD-Indicator",
                        [
                            tod + 0.06,
                            tod + 50.0 / 128.0 - 0.06,
                            0.1,
                            50.0 / 64.0 - 0.1,
                        ],
                    ),
                ));
            });
            for (button, top, bars) in [
                (MapButton::ZoomIn, 128.0, &[0.0, 90.0][..]),
                (MapButton::ZoomOut, 150.0, &[0.0][..]),
            ] {
                let e = knob(c, at(26.0, top, 18.0, 18.0), 18.0);
                c.commands().entity(e).insert((button, Button, MapBody));
                c.commands()
                    .entity(e)
                    .with_children(|b| glyph(b, button, 8.0, bars));
            }
        })
        .id()
}

/// The knobs: a press acts, hover lights the ring, and a zoom at its end dims.
#[allow(clippy::type_complexity)]
pub(super) fn map_buttons(
    mut map: ResMut<MiniMap>,
    mut buttons: Query<(Ref<Interaction>, &MapButton, &mut BorderColor)>,
    mut glyphs: Query<(&KnobGlyph, &mut BackgroundColor)>,
    mut bodies: Query<&mut Node, (With<MapBody>, Without<MapCluster>)>,
    mut clusters: Query<(&MapCluster, &mut Node), Without<MapBody>>,
) {
    for (interaction, button, mut border) in &mut buttons {
        if *interaction == Interaction::Pressed && interaction.is_changed() {
            match button {
                MapButton::ZoomIn => map.zoom = (map.zoom + 1).min(ZOOM_CHUNKS.len() - 1),
                MapButton::ZoomOut => map.zoom = map.zoom.saturating_sub(1),
                MapButton::Toggle => map.shown = !map.shown,
            }
        }
        let want = BorderColor::all(if *interaction == Interaction::None {
            col::STEEL_LO
        } else {
            col::STEEL_HI
        });
        if *border != want {
            *border = want;
        }
    }
    for (glyph, mut color) in &mut glyphs {
        let want = match glyph.0 {
            MapButton::ZoomIn if map.zoom == ZOOM_CHUNKS.len() - 1 => col::STEEL_LO,
            MapButton::ZoomOut if map.zoom == 0 => col::STEEL_LO,
            MapButton::Toggle => col::CRIMSON,
            _ => col::STEEL_HI,
        };
        color.set_if_neq(BackgroundColor(want));
    }
    let display = if map.shown {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut bodies {
        if node.display != display {
            node.display = display;
        }
    }
    for (cluster, mut node) in &mut clusters {
        let want = Val::Px(cluster_height(map.shown) * cluster.0);
        if node.height != want {
            node.height = want;
        }
    }
}

/// The cluster shows over the run, the pause menu included, as the HUD does.
pub(super) fn show_cluster(
    editor: Res<super::ui_edit::UiEditor>,
    phase: Res<State<super::Phase>>,
    mut clusters: Query<&mut Visibility, With<MapCluster>>,
) {
    let p = *phase.get();
    let want = if editor.active || (p != super::Phase::Menu && p != super::Phase::GameOver) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut clusters {
        v.set_if_neq(want);
    }
}

/// The player's local time, "21:47", read once a second.
pub(super) fn clock(
    time: Res<Time<Real>>,
    mut texts: Query<&mut Text, With<ClockText>>,
    mut next_at: Local<f32>,
) {
    let now = time.elapsed_secs();
    if now < *next_at && texts.iter().all(|t| !t.0.is_empty()) {
        return;
    }
    *next_at = now + 1.0;
    let [_, _, hour, minute, _, _] = crate::ui_chat::logging::local_now();
    let want = format!("{hour:02}:{minute:02}");
    for mut text in &mut texts {
        if text.0 != want {
            text.0.clone_from(&want);
        }
    }
}

/// `Minimap_Update`: `GetMinimapZoneText` (the subzone, else the zone) in `GetZonePVPInfo`'s
/// colour, the zone's owner against the hero's faction.
pub(super) fn zone_text(
    world: WorldPoint,
    areas: Option<Res<AreaTableRes>>,
    run: Res<Run>,
    mut texts: Query<(&mut Text, &mut TextColor), With<ZoneText>>,
) {
    let Some(areas) = areas else { return };
    let Some(leaf) = world.area() else { return };
    let Some(row) = areas.0.get(leaf) else { return };
    let zone = if row.zone_id == 0 { leaf } else { row.zone_id };
    let name = if leaf == zone {
        areas.0.name(zone).unwrap_or_default().to_string()
    } else {
        row.name.clone()
    };
    let (friend, enemy) = match run.race {
        1 | 3 | 4 | 7 => (GROUP_ALLIANCE, GROUP_HORDE),
        _ => (GROUP_HORDE, GROUP_ALLIANCE),
    };
    let owner = areas.0.get(zone).map_or(0, |r| r.faction_group_mask);
    let color = if row.flags & 0x80 != 0 || owner & enemy != 0 {
        Color::srgb(1.0, 0.1, 0.1)
    } else if owner & friend != 0 {
        Color::srgb(0.1, 1.0, 0.1)
    } else {
        Color::srgb(1.0, 0.7, 0.0)
    };
    for (mut text, mut tc) in &mut texts {
        if text.0 != name {
            text.0 = name.clone();
        }
        if tc.0 != color {
            tc.0 = color;
        }
    }
}

/// Recomposite the map around the hero: tiles, mask, arrow, then the enemy dots.
#[allow(clippy::too_many_arguments)]
pub(super) fn compose(
    editor: Res<super::ui_edit::UiEditor>,
    mut map: ResMut<MiniMap>,
    assets: Option<ResMut<WorldAssets>>,
    catalog: Option<Res<MapCatalogRes>>,
    current: Option<Res<CurrentMap>>,
    phase: Res<State<super::Phase>>,
    hero: Query<&Transform, With<Hero>>,
    cameras: Query<&Transform, With<benilla_world::view::WorldCamera>>,
    enemies: Query<(&Transform, &Enemy)>,
    lairs: Res<super::guardians::Lairs>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Real>>,
    mut next_at: Local<f32>,
) {
    if (!editor.active && *phase.get() == super::Phase::Menu) || !map.shown {
        return;
    }
    // A CPU composite plus a texture upload: on the browser's one thread it was the single
    // costliest survivors system per frame (~5 %), and the blips read the same at 12 Hz.
    let now = time.elapsed_secs();
    if now < *next_at {
        return;
    }
    *next_at = now + 1.0 / 12.0;
    let (Some(mut assets), Some(catalog), Some(current)) = (assets, catalog, current) else {
        return;
    };
    let Ok(hero) = hero.single().or_else(|_| cameras.single()) else {
        return;
    };
    let Some(dir) = catalog.0.directory(current.0).map(str::to_string) else {
        return;
    };
    let map = &mut *map;
    if map.dir != dir {
        map.tiles.clear();
        map.dir.clone_from(&dir);
    }
    if map.art.is_none() {
        let translate = {
            let mut chain = assets.chain.lock_recover();
            benilla_formats::load_minimap_translate(&mut chain)
        };
        let Ok(translate) = translate else {
            warn!("survivors: md5translate.trs failed, no minimap");
            map.shown = false;
            return;
        };
        map.art = Some(Art {
            translate,
            arrow: Rgba::decode(&mut assets, crate::minimap::blips::PLAYER_ARROW_TEXTURE),
            icons: Rgba::decode(&mut assets, "Interface\\Minimap\\ObjectIcons"),
        });
    }
    let art = map.art.as_ref().expect("art loaded above");

    // The hero in WoW coordinates; screen right is east (-y), screen down south (-x).
    let p = hero.translation;
    let (wx, wy) = (-p.z, -p.x);
    let radius = map.view_radius();
    let yd_per_px = 2.0 * radius / SIDE as f32;

    // The 2×2 tile window under the view: its diameter (at most 14 chunks) is under one tile.
    let (tx0, ty0) = world_to_tile(wx + radius + 1.0, wy + radius + 1.0);
    for ty in ty0..=ty0 + 1 {
        for tx in tx0..=tx0 + 1 {
            map.tiles.entry((tx, ty)).or_insert_with(|| {
                let hash = art.translate.tile(&dir, tx, ty)?;
                let tile = Rgba::decode(&mut assets, &format!("textures\\Minimap\\{hash}"))?;
                if tile.w == TILE_PX && tile.h == TILE_PX {
                    return Some(tile.px);
                }
                // Retail replacement tiles are 512². The compositor uses a 256²
                // geographic grid, so normalize decoded art instead of rejecting it.
                let mut pixels = Vec::with_capacity(TILE_PX * TILE_PX * 4);
                for y in 0..TILE_PX {
                    for x in 0..TILE_PX {
                        pixels.extend(
                            tile.sample(
                                (x as f32 + 0.5) * tile.w as f32 / TILE_PX as f32,
                                (y as f32 + 0.5) * tile.h as f32 / TILE_PX as f32,
                            )
                            .map(|c| c.round() as u8),
                        );
                    }
                }
                Some(pixels)
            });
        }
    }
    let window: [[Option<&[u8]>; 2]; 2] = [0, 1].map(|j| {
        [0, 1].map(|i| {
            map.tiles
                .get(&(tx0 + i, ty0 + j))
                .and_then(|t| t.as_deref())
        })
    });
    // Window-local tile texels: x grows east from the window's west edge, y south from its north.
    let (north, west) = tile_to_world(tx0, ty0);
    let tex_per_yd = TILE_PX as f32 / TILE_YARDS;
    let origin = Vec2::new((west - wy) * tex_per_yd, (north - wx) * tex_per_yd);
    let tex_per_px = yd_per_px * tex_per_yd;
    let texel = |x: i32, y: i32| -> [f32; 3] {
        let (cx, cy) = (x.div_euclid(TILE_PX as i32), y.div_euclid(TILE_PX as i32));
        if !(0..2).contains(&cx) || !(0..2).contains(&cy) {
            return [0.0; 3];
        }
        match window[cy as usize][cx as usize] {
            Some(px) => {
                let (lx, ly) = (x.rem_euclid(TILE_PX as i32), y.rem_euclid(TILE_PX as i32));
                let i = (ly as usize * TILE_PX + lx as usize) * 4;
                [f32::from(px[i]), f32::from(px[i + 1]), f32::from(px[i + 2])]
            }
            None => [0.0; 3],
        }
    };

    let half = SIDE as f32 / 2.0;
    let buf = &mut map.buf;
    for v in 0..SIDE {
        for u in 0..SIDE {
            let o = (v * SIDE + u) * 4;
            let mask = 255u8;
            let g = origin + (Vec2::new(u as f32, v as f32) + 0.5 - half) * tex_per_px - 0.5;
            let (x0, y0) = (g.x.floor() as i32, g.y.floor() as i32);
            let (fx, fy) = (g.x - x0 as f32, g.y - y0 as f32);
            let (a, b, c, d) = (
                texel(x0, y0),
                texel(x0 + 1, y0),
                texel(x0, y0 + 1),
                texel(x0 + 1, y0 + 1),
            );
            for k in 0..3 {
                let top = a[k] + (b[k] - a[k]) * fx;
                let bottom = c[k] + (d[k] - c[k]) * fx;
                buf[o + k] = (top + (bottom - top) * fy) as u8;
            }
            buf[o + 3] = mask;
        }
    }

    let scale = SIDE as f32 / BLIP_BASIS_PX;
    let center = Vec2::splat(half);
    // The arrow turns with the hero's facing: counter-clockwise from north, as the yaw runs.
    if let Some(arrow) = &art.arrow {
        let yaw = hero.rotation.to_euler(EulerRot::YXZ).0;
        let (s, c) = yaw.sin_cos();
        let to_screen = |t: Vec2| Vec2::new(c * t.x + s * t.y, -s * t.x + c * t.y);
        let to_tex = |q: Vec2| Vec2::new(c * q.x - s * q.y, s * q.x + c * q.y);
        let size = ARROW_PX * scale;
        blit(
            buf,
            center + to_screen(ARROW_OFFSET_PX * scale),
            size,
            |q| {
                let t = to_tex(q) / size + 0.5;
                (0.0..1.0).contains(&t.x).then_some(())?;
                (0.0..1.0).contains(&t.y).then_some(())?;
                Some(arrow.sample(t.x * arrow.w as f32, t.y * arrow.h as f32))
            },
        );
    }
    if let Some(icons) = &art.icons {
        let size = DOT_PX * scale;
        let [l, r, t, b] = TRACKED_UNIT_CELL;
        for (tf, enemy) in &enemies {
            if enemy.dead {
                continue;
            }
            let off = Vec2::new(tf.translation.x - p.x, tf.translation.z - p.z);
            if off.x.abs() > radius || off.y.abs() > radius {
                continue;
            }
            blit(buf, center + off / yd_per_px, size, |q| {
                let f = q / size + 0.5;
                let x = (l + (r - l) * f.x) * icons.w as f32;
                let y = (t + (b - t) * f.y) * icons.h as f32;
                Some(icons.sample(x, y))
            });
        }
        // The held lairs, big and gold; one out of view sits on the edge, pointing the way.
        let size = DOT_PX * scale * 2.2;
        for (at, held) in lairs.marks() {
            if !held {
                continue;
            }
            let mut off = Vec2::new(at.x - p.x, at.z - p.z) / yd_per_px;
            let rim = half - size * 0.6;
            let far = off.x.abs().max(off.y.abs());
            if far > rim {
                off *= rim / far;
            }
            blit(buf, center + off, size, |q| {
                let f = q / size + 0.5;
                let x = (l + (r - l) * f.x) * icons.w as f32;
                let y = (t + (b - t) * f.y) * icons.h as f32;
                let [_, g, _, a] = icons.sample(x, y);
                let lit = g.max(128.0);
                Some([255.0, lit * 0.82, 40.0, a])
            });
        }
    }

    if let Some(image) = images.get_mut(&map.image) {
        if let Some(data) = image.data.as_mut() {
            data.copy_from_slice(&map.buf);
        }
    }
}

/// Alpha-blend a `size`-px square centred at `at`; `shade` maps a pixel's offset from the centre
/// to its RGBA. The map's own alpha (the mask) is kept.
fn blit(buf: &mut [u8], at: Vec2, size: f32, shade: impl Fn(Vec2) -> Option<[f32; 4]>) {
    let half = size / 2.0;
    let x_lo = (at.x - half).floor().max(0.0) as usize;
    let y_lo = (at.y - half).floor().max(0.0) as usize;
    let x_hi = ((at.x + half).ceil() as usize).min(SIDE);
    let y_hi = ((at.y + half).ceil() as usize).min(SIDE);
    for y in y_lo..y_hi {
        for x in x_lo..x_hi {
            let o = (y * SIDE + x) * 4;
            if buf[o + 3] == 0 {
                continue;
            }
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - at;
            let Some(src) = shade(q) else { continue };
            let a = src[3] / 255.0;
            for k in 0..3 {
                let dst = f32::from(buf[o + k]);
                buf[o + k] = (dst + (src[k] - dst) * a) as u8;
            }
        }
    }
}
