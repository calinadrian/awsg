//! Ground telegraphs, as the old ASR warned of its bosses' blows: a gold circle where a stomp, a
//! throw, a whirlwind or a bomb will land, filling from the centre until it does, and a gold lane
//! down a charge's path, filling through the wind-up; a bright edge over a faint fill. Projected
//! onto the ground through the decal projector the client's AoE targeting reticle uses
//! ([`crate::target`]'s reticle), in art drawn here: the client has no plain warning circle (its
//! reticle art is a "no" sign).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use benilla_world::decal::{DecalFrame, WorldDecal};
use benilla_world::particles::buffer::{EffectVertex, WorldEffectDraw};
use benilla_world::sky_order::Rung;
use benilla_world::view::WorldCamera;

use super::Run;

/// The art's side, in texels.
const ART: u32 = 128;
/// What a telegraph asks of the player, by its colour: gold to step out of (red in its last
/// moments), green to stand in, purple to stand in together.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Tone {
    #[default]
    Gold,
    Green,
    Purple,
}

/// The art's colours, edge and fill, by palette: the old telegraphs' gold (the edge pale and
/// bright, the fill deeper), the red gold turns before it lands, green and purple.
const PALETTES: [([u8; 3], [u8; 3]); 4] = [
    ([255, 236, 140], [255, 196, 40]),
    ([255, 160, 140], [235, 45, 30]),
    ([180, 255, 180], [50, 220, 80]),
    ([232, 180, 255], [172, 70, 240]),
];
/// How long before it lands a gold telegraph turns red.
const RED_BEFORE: f32 = 0.4;

/// One telegraph texture: each texel's colour and alpha from its place, `[-1, 1]` on both axes.
fn art(images: &mut Assets<Image>, texel: impl Fn(Vec2) -> ([u8; 3], f32)) -> Handle<Image> {
    let mut data = Vec::with_capacity((ART * ART * 4) as usize);
    let half = ART as f32 / 2.0;
    for y in 0..ART {
        for x in 0..ART {
            let p = Vec2::new(x as f32 + 0.5 - half, y as f32 + 0.5 - half) / half;
            let (rgb, alpha) = texel(p);
            let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: ART,
            height: ART,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// A smooth step from 1 inside `edge` to 0 a `soft` beyond it.
fn inside(d: f32, edge: f32, soft: f32) -> f32 {
    ((edge + soft - d) / soft).clamp(0.0, 1.0)
}

/// The telegraph textures.
pub(super) struct Art {
    cones: [(Handle<Image>, Handle<Image>); 2],
    /// The circle's outline: a bright edge with a soft glow inward, over a faint fill.
    rim: Handle<Image>,
    /// The fill that grows from the centre to meet the edge.
    disc: Handle<Image>,
    /// The lane's outline: bright edges down both sides and across the far end.
    lane: Handle<Image>,
    /// The lane's fill, which runs down it.
    solid: Handle<Image>,
}

impl Art {
    fn new(images: &mut Assets<Image>, (lit, body): ([u8; 3], [u8; 3])) -> Self {
        Self {
            cones: [45.0_f32, 40.0].map(|degrees| {
                let half = degrees.to_radians();
                let outline = art(images, |p| {
                    let d = p.length();
                    let angle = p.x.atan2(p.y).abs();
                    let mask = inside(d, 0.96, 0.03) * inside(angle, half, 0.02);
                    let edge =
                        (1.0 - inside(d, 0.9, 0.03)).max(1.0 - inside(angle, half - 0.05, 0.02));
                    (lit, mask * (0.14 + 0.81 * edge))
                });
                let fill = art(images, |p| {
                    (
                        body,
                        0.28 * inside(p.length(), 0.95, 0.03)
                            * inside(p.x.atan2(p.y).abs(), half, 0.02),
                    )
                });
                (outline, fill)
            }),
            rim: art(images, |p| {
                let d = p.length();
                let edge = inside(d, 0.96, 0.03) * (1.0 - inside(d, 0.9, 0.03));
                let glow = inside(d, 0.96, 0.03) * ((d - 0.6) / 0.33).clamp(0.0, 1.0);
                if edge > 0.0 {
                    (lit, 0.95 * edge + 0.3 * glow)
                } else {
                    (body, 0.14 * inside(d, 0.92, 0.03) + 0.3 * glow)
                }
            }),
            disc: art(images, |p| (body, 0.28 * inside(p.length(), 0.95, 0.05))),
            // x runs across the lane, y down it (to its far end at +1).
            lane: art(images, |p| {
                let side = 1.0 - inside(p.x.abs(), 0.9, 0.05);
                let end = 1.0 - inside(p.y, 0.93, 0.04);
                let edge = side.max(end);
                if edge > 0.0 {
                    (lit, 0.95 * edge)
                } else {
                    (body, 0.14)
                }
            }),
            solid: art(images, |_| (body, 0.3)),
        }
    }
}

/// How far above and below its centre a telegraph drapes over the ground.
const DRAPE: f32 = 3.0;

enum Shape {
    Cone {
        center: Vec3,
        dir: Vec3,
        radius: f32,
        half_angle: f32,
    },
    Circle {
        center: Vec3,
        radius: f32,
    },
    /// From `from` along the unit `dir` for `len`, `half` to each side.
    Lane {
        from: Vec3,
        dir: Vec3,
        len: f32,
        half: f32,
    },
}

struct Telegraph {
    shape: Shape,
    tone: Tone,
    /// The run seconds it appears, finishes filling and goes.
    start: f32,
    full: f32,
    end: f32,
    /// The outline, projected once.
    outline: Vec<EffectVertex>,
}

/// The run's telegraphs on the ground.
#[derive(Resource, Default)]
pub(super) struct Telegraphs(Vec<Telegraph>);

impl Telegraphs {
    pub(super) fn remove_cone(&mut self, center: Vec3, dir: Vec3, radius: f32) {
        self.0.retain(|t| !matches!(t.shape, Shape::Cone { center: c, dir: d, radius: r, .. }
            if c.distance_squared(center) < 0.01 && d.distance_squared(dir) < 0.01
                && (r - radius).abs() < 0.01));
    }

    pub(super) fn remove_lane(&mut self, from: Vec3, to: Vec3, half: f32) {
        let way = super::units::flat(to - from);
        let dir = way.normalize_or_zero();
        self.0.retain(|t| !matches!(t.shape, Shape::Lane { from: f, dir: d, len, half: h }
            if f.distance_squared(from) < 0.01 && d.distance_squared(dir) < 0.01
                && (len - way.length()).abs() < 0.01 && (h - half).abs() < 0.01));
    }

    /// Remove an egg's step marker as soon as that egg is crushed or its owner dies.
    pub(super) fn remove_circle(&mut self, center: Vec3, radius: f32) {
        self.0.retain(|t| !matches!(t.shape, Shape::Circle { center: c, radius: r }
            if c.distance_squared(center) < 0.01 && (r - radius).abs() < 0.01));
    }
    /// The host sends warnings as shapes and encounter times; each guest projects its own art.
    pub(super) fn snapshot(&self) -> Vec<[f32; 14]> {
        self.0
            .iter()
            .map(|t| {
                let (kind, a, b, radius, half, len) = match t.shape {
                    Shape::Circle { center, radius } => (0., center, Vec3::ZERO, radius, 0., 0.),
                    Shape::Cone {
                        center,
                        dir,
                        radius,
                        half_angle,
                    } => (1., center, dir, radius, half_angle, 0.),
                    Shape::Lane {
                        from,
                        dir,
                        len,
                        half,
                    } => (2., from, dir, 0., half, len),
                };
                [
                    kind, a.x, a.y, a.z, b.x, b.y, b.z, radius, half, len, t.start, t.full, t.end,
                    t.tone as u8 as f32,
                ]
            })
            .collect()
    }
    pub(super) fn restore(&mut self, warnings: Vec<[f32; 14]>) {
        let old = self.snapshot();
        let mut next = Vec::new();
        for w in warnings
            .into_iter()
            .take(256)
            .filter(|w| w.iter().all(|v| v.is_finite()))
        {
            let a = Vec3::new(w[1], w[2], w[3]);
            let b = Vec3::new(w[4], w[5], w[6]);
            let shape = match w[0] as u8 {
                0 => Shape::Circle {
                    center: a,
                    radius: w[7],
                },
                1 => Shape::Cone {
                    center: a,
                    dir: b,
                    radius: w[7],
                    half_angle: w[8],
                },
                2 => Shape::Lane {
                    from: a,
                    dir: b,
                    len: w[9],
                    half: w[8],
                },
                _ => continue,
            };
            let outline = old
                .iter()
                .position(|p| *p == w)
                .map_or_else(Vec::new, |i| std::mem::take(&mut self.0[i].outline));
            next.push(Telegraph {
                shape,
                tone: match w[13] as u8 {
                    1 => Tone::Green,
                    2 => Tone::Purple,
                    _ => Tone::Gold,
                },
                start: w[10],
                full: w[11],
                end: w[12],
                outline,
            });
        }
        self.0 = next;
    }
    /// Colours the telegraph last marked.
    pub fn tone(&mut self, tone: Tone) {
        if let Some(t) = self.0.last_mut() {
            t.tone = tone;
        }
    }
    /// A ground hazard stays filled while it can still deal damage.
    pub fn hazard(&mut self, center: Vec3, radius: f32, now: f32, secs: f32) {
        self.0.push(Telegraph {
            tone: Tone::Gold,
            shape: Shape::Circle { center, radius },
            start: now,
            full: now,
            end: now + secs,
            outline: Vec::new(),
        });
    }
    pub fn cone(
        &mut self,
        center: Vec3,
        dir: Vec3,
        radius: f32,
        half_angle: f32,
        now: f32,
        secs: f32,
    ) {
        self.0.push(Telegraph {
            tone: Tone::Gold,
            shape: Shape::Cone {
                center,
                dir,
                radius,
                half_angle,
            },
            start: now,
            full: now + secs,
            end: now + secs,
            outline: Vec::new(),
        });
    }
    /// A circle of `radius` at `center`, filling for `secs` until what it warns of lands.
    pub fn circle(&mut self, center: Vec3, radius: f32, now: f32, secs: f32) {
        self.0.push(Telegraph {
            tone: Tone::Gold,
            shape: Shape::Circle { center, radius },
            start: now,
            full: now + secs,
            end: now + secs,
            outline: Vec::new(),
        });
    }

    /// A lane `half` wide to each side from `from` to `to`, filling for `fill` seconds and
    /// staying `secs` in all.
    pub fn lane(&mut self, from: Vec3, to: Vec3, half: f32, now: f32, fill: f32, secs: f32) {
        let way = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
        let len = way.length();
        if len < 0.1 {
            return;
        }
        self.0.push(Telegraph {
            tone: Tone::Gold,
            shape: Shape::Lane {
                from,
                dir: way / len,
                len,
                half,
            },
            start: now,
            full: now + fill,
            end: now + secs,
            outline: Vec::new(),
        });
    }
}

fn circle_frame(center: Vec3, r: f32) -> DecalFrame {
    DecalFrame {
        center,
        sin: 0.0,
        cos: 1.0,
        min_x: -r,
        max_x: r,
        min_z: -r,
        max_z: r,
        min_y: -DRAPE,
        max_y: DRAPE,
    }
}

/// The lane's box: the frame's z runs down the lane (the decal's rotation, `sin`/`cos` the
/// direction's x and z).
fn lane_frame(from: Vec3, dir: Vec3, len: f32, half: f32) -> DecalFrame {
    DecalFrame {
        center: from,
        sin: dir.x,
        cos: dir.z,
        min_x: -half,
        max_x: half,
        min_z: 0.0,
        max_z: len,
        min_y: -DRAPE,
        max_y: DRAPE,
    }
}

fn project(decals: &WorldDecal, frame: DecalFrame) -> Vec<EffectVertex> {
    let mut out = Vec::new();
    decals.project(
        &mut out,
        &frame,
        |p| ((DRAPE - p.y.abs()) / DRAPE).clamp(0.0, 1.0),
        |x, z| frame.rect_uv(x, z),
    );
    for v in &mut out {
        v.color = [1.0, 1.0, 1.0, v.color[3]];
    }
    out
}

/// Drop the spent telegraphs and draw the rest, each filling toward its moment.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_telegraphs(
    run: Res<Run>,
    phase: Res<State<super::Phase>>,
    mut images: ResMut<Assets<Image>>,
    mut textures: Local<Option<[Art; 4]>>,
    mut tele: ResMut<Telegraphs>,
    decals: WorldDecal,
    cam: Query<Entity, With<WorldCamera>>,
    mut draw: WorldEffectDraw,
) {
    let now = run.elapsed;
    // A new run's clock starts over: the old run's telegraphs go with it.
    tele.0.retain(|t| now < t.end && now >= t.start);
    if *phase.get() == super::Phase::Menu {
        tele.0.clear();
        return;
    }
    if tele.0.is_empty() {
        return;
    }
    let palettes = textures.get_or_insert_with(|| PALETTES.map(|p| Art::new(&mut images, p)));
    let Ok(cam) = cam.single() else { return };
    for t in &mut tele.0 {
        let fill = ((now - t.start) / (t.full - t.start).max(1e-3)).clamp(0.0, 1.0);
        let palette = match t.tone {
            Tone::Gold if t.full > t.start && t.full - now < RED_BEFORE => 1,
            Tone::Gold => 0,
            Tone::Green => 2,
            Tone::Purple => 3,
        };
        let art = &palettes[palette];
        let (anchor, outline, texture, inner) = match t.shape {
            Shape::Cone {
                center,
                dir,
                radius,
                half_angle,
            } => {
                let frame = |radius| {
                    let mut frame = circle_frame(center, radius);
                    frame.sin = dir.x;
                    frame.cos = dir.z;
                    frame
                };
                if t.outline.is_empty() {
                    t.outline = project(&decals, frame(radius));
                }
                let inner = project(&decals, frame((radius * fill).max(0.2)));
                let (rim, disc) = &art.cones[usize::from(half_angle < 44.0_f32.to_radians())];
                (center, rim.id(), disc.id(), inner)
            }
            Shape::Circle { center, radius } => {
                if t.outline.is_empty() {
                    t.outline = project(&decals, circle_frame(center, radius));
                }
                let inner = project(&decals, circle_frame(center, (radius * fill).max(0.2)));
                (center, art.rim.id(), art.disc.id(), inner)
            }
            Shape::Lane {
                from,
                dir,
                len,
                half,
            } => {
                if t.outline.is_empty() {
                    t.outline = project(&decals, lane_frame(from, dir, len, half));
                }
                let inner = project(&decals, lane_frame(from, dir, (len * fill).max(0.1), half));
                (from, art.lane.id(), art.solid.id(), inner)
            }
        };
        for (verts, texture) in [(&t.outline, outline), (&inner, texture)] {
            if verts.is_empty() {
                continue;
            }
            let mut batch = draw
                .batch(cam, texture)
                .anchored(anchor)
                .rung(Rung::RETICLE, Rung::DECAL_RASTER);
            batch.extend(verts.iter().copied());
            batch.tris();
        }
    }
}
