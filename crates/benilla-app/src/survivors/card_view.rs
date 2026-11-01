//! Native Bevy rendering of the supplied Arcane Codex V6 ornate card format.
//! Coordinates follow the 420x640 handoff; the game scales them onto its 768-high UI canvas.
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub const WIDTH: f32 = 420.;
pub const HEIGHT: f32 = 640.;
pub const GAME_SCALE: f32 = 0.52;
pub const TEXT: Color = Color::srgb(243. / 255., 235. / 255., 221. / 255.);
pub const MUTED: Color = Color::srgb(188. / 255., 180. / 255., 166. / 255.);
pub const POSITIVE: Color = Color::srgb(104. / 255., 240. / 255., 181. / 255.);

pub fn quality(index: usize) -> Color {
    let [r, g, b] = [
        [187, 197, 206],
        [110, 217, 151],
        [76, 174, 255],
        [201, 128, 255],
        [255, 188, 87],
    ][index];
    Color::srgb(r as f32 / 255., g as f32 / 255., b as f32 / 255.)
}

#[derive(Resource)]
pub struct CardArt {
    pub title: Handle<Font>,
    pub label: Handle<Font>,
    pub body: Handle<Font>,
    pub value: Handle<Font>,
    background: [Handle<Image>; 5],
    border: [Handle<Image>; 5],
    diamond: [Handle<Image>; 5],
    stat: [Handle<Image>; 5],
}

// The handoff's stat panel has 7px cut corners and a quiet .75px rarity outline.
// Rasterized from the UI geometry once, so no world shader or SVG loader is needed.
fn stat_panel(rarity: usize, images: &mut Assets<Image>) -> Handle<Image> {
    let color = quality(rarity).to_srgba();
    let foreground = [color.red * 255., color.green * 255., color.blue * 255.];
    let background = [8., 15., 22.];
    let mut pixels = Vec::with_capacity(372 * 112 * 4);
    for y in 0..112 {
        for x in 0..372 {
            let (a, b) = (x as f32 + 0.5, y as f32 + 0.5);
            let (c, d) = (372. - a, 112. - b);
            let distance = [
                a,
                b,
                c,
                d,
                (a + b - 7.) / std::f32::consts::SQRT_2,
                (c + b - 7.) / std::f32::consts::SQRT_2,
                (a + d - 7.) / std::f32::consts::SQRT_2,
                (c + d - 7.) / std::f32::consts::SQRT_2,
            ]
            .into_iter()
            .fold(f32::INFINITY, f32::min);
            let border = (1.25 - distance).clamp(0., 1.) * 0.45;
            for channel in 0..3 {
                pixels.push(
                    (foreground[channel] * border + background[channel] * (1. - border)) as u8,
                );
            }
            pixels.push(((distance + 0.5).clamp(0., 1.) * 255.) as u8);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: 372,
            height: 112,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

pub fn png(bytes: &[u8], images: &mut Assets<Image>) -> Handle<Image> {
    images.add(
        Image::from_buffer(
            bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::linear(),
            RenderAssetUsages::default(),
        )
        .expect("handoff PNG"),
    )
}

impl CardArt {
    pub fn load(fonts: &mut Assets<Font>, images: &mut Assets<Image>) -> Self {
        macro_rules! layers {
            ($kind:literal) => {
                ["common", "uncommon", "rare", "epic", "legendary"].map(|name| {
                    let bytes: &[u8] = match name {
                        "common" => {
                            include_bytes!(concat!("../../assets/cards/", $kind, "-common.png"))
                        }
                        "uncommon" => {
                            include_bytes!(concat!("../../assets/cards/", $kind, "-uncommon.png"))
                        }
                        "rare" => {
                            include_bytes!(concat!("../../assets/cards/", $kind, "-rare.png"))
                        }
                        "epic" => {
                            include_bytes!(concat!("../../assets/cards/", $kind, "-epic.png"))
                        }
                        _ => {
                            include_bytes!(concat!("../../assets/cards/", $kind, "-legendary.png"))
                        }
                    };
                    png(bytes, images)
                })
            };
        }
        Self {
            label: fonts.add(
                Font::try_from_bytes(include_bytes!("../../assets/cards/font-0.ttf").to_vec())
                    .expect("Cinzel regular"),
            ),
            title: fonts.add(
                Font::try_from_bytes(include_bytes!("../../assets/cards/font-1.ttf").to_vec())
                    .expect("Cinzel bold"),
            ),
            body: fonts.add(
                Font::try_from_bytes(include_bytes!("../../assets/cards/font-2.ttf").to_vec())
                    .expect("Crimson Pro regular"),
            ),
            value: fonts.add(
                Font::try_from_bytes(include_bytes!("../../assets/cards/font-3.ttf").to_vec())
                    .expect("Crimson Pro semibold"),
            ),
            background: layers!("background"),
            border: layers!("border"),
            diamond: layers!("diamond"),
            stat: std::array::from_fn(|i| stat_panel(i, images)),
        }
    }
}

pub fn setup(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(CardArt::load(&mut fonts, &mut images));
}

pub struct Content {
    pub rarity_name: String,
    pub name: String,
    pub state: String,
    pub description: String,
    pub stat_label: String,
    pub stat_value: String,
    pub positive: bool,
    pub footer: String,
}

pub struct CardNodes {
    pub root: Entity,
    pub icon: Entity,
}

fn rect(x: f32, y: f32, width: f32, height: f32, scale: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x * scale),
        top: px(y * scale),
        width: px(width * scale),
        height: px(height * scale),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

pub fn text(font: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    weighted_text(font, value, size, color, FontWeight::NORMAL)
}

pub fn weighted_text(
    font: &Handle<Font>,
    value: impl Into<String>,
    size: f32,
    color: Color,
    weight: FontWeight,
) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: size,
            weight,
            ..default()
        },
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
    )
}

pub fn spawn(
    commands: &mut Commands,
    art: &CardArt,
    content: Content,
    index: usize,
    rarity: usize,
    scale: f32,
) -> CardNodes {
    let rc = quality(rarity);
    let root = commands
        .spawn((
            Button,
            Node {
                width: px(WIDTH * scale),
                height: px(HEIGHT * scale),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    let mut icon = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|c| {
        c.spawn((
            rect(0., 0., WIDTH, HEIGHT, scale),
            ImageNode::new(art.background[rarity].clone()),
        ));
        c.spawn((
            rect(0., 0., WIDTH, HEIGHT, scale),
            ImageNode::new(art.border[rarity].clone()),
        ));
        c.spawn((
            Node {
                border_radius: BorderRadius::all(px(5. * scale)),
                ..rect(24., 64., 32., 36., scale)
            },
            BackgroundColor(TEXT.with_alpha(0.2)),
        ))
        .with_children(|key| {
            key.spawn(text(&art.body, (index + 1).to_string(), 18. * scale, TEXT));
        });
        c.spawn(Node {
            column_gap: px(10. * scale),
            ..rect(110., 64., 200., 36., scale)
        })
        .with_children(|label| {
            label.spawn((
                Node {
                    width: px(14. * scale),
                    height: px(18. * scale),
                    ..default()
                },
                ImageNode::new(art.diamond[rarity].clone()),
            ));
            label.spawn(text(
                &art.label,
                content
                    .rarity_name
                    .chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join("\u{2009}"),
                12. * scale,
                rc,
            ));
        });
        c.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(24. * scale),
                top: px(64. * scale),
                padding: UiRect::axes(px(10. * scale), px(6. * scale)),
                border_radius: BorderRadius::all(px(6. * scale)),
                ..default()
            },
            BackgroundColor(rc),
        ))
        .with_children(|badge| {
            badge.spawn(text(&art.label, content.state, 12. * scale, Color::BLACK));
        });
        c.spawn((
            Node {
                border: UiRect::all(px(1.5 * scale)),
                border_radius: BorderRadius::all(px(9. * scale)),
                ..rect(166., 108., 88., 88., scale)
            },
            BorderColor::all(rc),
            BackgroundColor(Color::srgb(0.043, 0.078, 0.125)),
        ))
        .with_children(|socket| {
            socket.spawn((
                Node {
                    border: UiRect::all(px(scale)),
                    border_radius: BorderRadius::all(px(6. * scale)),
                    ..rect(1.5, 1.5, 82., 82., scale)
                },
                BorderColor::all(rc),
            ));
            icon = socket
                .spawn((
                    Node {
                        width: px(80. * scale),
                        height: px(80. * scale),
                        ..default()
                    },
                    ImageNode::default(),
                ))
                .id();
        });
        c.spawn(rect(24., 204., 372., 36., scale))
            .with_children(|title| {
                let size = if content.name.chars().count() > 24 {
                    23.
                } else {
                    29.
                };
                title.spawn(weighted_text(
                    &art.title,
                    content.name,
                    size * scale,
                    TEXT,
                    FontWeight::BOLD,
                ));
            });
        c.spawn((
            rect(24., 256., 168., 1., scale),
            BackgroundColor(rc.with_alpha(0.55)),
        ));
        c.spawn((
            rect(228., 256., 168., 1., scale),
            BackgroundColor(rc.with_alpha(0.55)),
        ));
        c.spawn((
            rect(205., 251., 10., 12., scale),
            ImageNode::new(art.diamond[rarity].clone()),
        ));
        c.spawn(rect(40., 273., 340., 135., scale))
            .with_children(|body| {
                let size = if content.description.len() > 200 {
                    18.
                } else {
                    21.
                };
                body.spawn(text(&art.body, content.description, size * scale, TEXT));
            });
        c.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8. * scale),
                ..rect(24., 424., 372., 112., scale)
            },
            ImageNode::new(art.stat[rarity].clone()),
        ))
        .with_children(|stat| {
            stat.spawn(text(
                &art.label,
                content
                    .stat_label
                    .chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join("\u{2009}"),
                14. * scale,
                MUTED,
            ));
            let size = if content.stat_value.lines().count() > 5 {
                14.
            } else if content.stat_value.lines().count() > 3 {
                18.
            } else if content.stat_value.lines().count() > 1 {
                23.
            } else if content.stat_value.chars().count() > 17 {
                26.
            } else if content.stat_value.chars().count() > 12 {
                34.
            } else {
                44.
            };
            stat.spawn(weighted_text(
                &art.value,
                content.stat_value,
                size * scale,
                if content.positive { POSITIVE } else { TEXT },
                FontWeight::SEMIBOLD,
            ));
        });
        c.spawn(Node {
            column_gap: px(12. * scale),
            ..rect(24., 552., 372., 24., scale)
        })
        .with_children(|footer| {
            footer.spawn((
                Node {
                    width: px(5. * scale),
                    height: px(7. * scale),
                    ..default()
                },
                ImageNode::new(art.diamond[rarity].clone()),
            ));
            footer.spawn(text(&art.body, content.footer, 18. * scale, MUTED));
            footer.spawn((
                Node {
                    width: px(5. * scale),
                    height: px(7. * scale),
                    ..default()
                },
                ImageNode::new(art.diamond[rarity].clone()),
            ));
        });
    });
    CardNodes { root, icon }
}
