//! Native Bevy rendering of the approved Classic ornate card format.
//! Coordinates follow the 420x640 handoff; the game scales them onto its 768-high UI canvas.
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub const WIDTH: f32 = 420.0;
pub const HEIGHT: f32 = 640.0;
pub const GAME_SCALE: f32 = 0.52;
pub const TEXT: Color = Color::srgb(243.0 / 255.0, 235.0 / 255.0, 221.0 / 255.0);
pub const MUTED: Color = Color::srgb(188.0 / 255.0, 180.0 / 255.0, 166.0 / 255.0);
pub const POSITIVE: Color = Color::srgb(30.0 / 255.0, 1.0, 0.0);

pub fn quality(index: usize) -> Color {
    let [r, g, b] = [
        [255, 255, 255],
        [30, 255, 0],
        [0, 112, 221],
        [163, 53, 238],
        [255, 128, 0],
    ][index];
    Color::srgb((r as f32) / 255.0, (g as f32) / 255.0, (b as f32) / 255.0)
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

// Flat inset panel; the outer frame carries the item-quality color.
fn stat_panel(_rarity: usize, images: &mut Assets<Image>) -> Handle<Image> {
    let foreground = [92.0, 87.0, 72.0];
    let background = [8.0, 8.0, 7.0];
    let mut pixels = Vec::with_capacity(372 * 112 * 4);
    for y in 0..112 {
        for x in 0..372 {
            let (a, b) = ((x as f32) + 0.5, (y as f32) + 0.5);
            let (c, d) = (372.0 - a, 112.0 - b);
            let distance = [a, b, c, d].into_iter().fold(f32::INFINITY, f32::min);
            let border = (1.25 - distance).clamp(0.0, 1.0) * 0.45;
            for channel in 0..3 {
                pixels.push(
                    (foreground[channel] * border + background[channel] * (1.0 - border)) as u8,
                );
            }
            pixels.push(((distance + 0.5).clamp(0.0, 1.0) * 255.0) as u8);
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

pub struct StatRow {
    pub label: String,
    pub before: Option<String>,
    pub after: String,
}

pub struct Content {
    pub rarity_name: String,
    pub name: String,
    pub state: String,
    pub rank: Option<String>,
    pub rows: Vec<StatRow>,
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
    let divider_y = if content.rank.is_some() { 280.0 } else { 256.0 };
    let body_height = if content.rank.is_some() { 111.0 } else { 135.0 };
    let root = commands
        .spawn((
            Button,
            Node {
                width: px(WIDTH * scale),
                height: px(HEIGHT * scale),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let mut icon = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|c| {
        c.spawn((
            rect(0.0, 0.0, WIDTH, HEIGHT, scale),
            ImageNode::new(art.background[rarity].clone()),
        ));
        c.spawn((
            rect(0.0, 0.0, WIDTH, HEIGHT, scale),
            ImageNode::new(art.border[rarity].clone()),
        ));
        c.spawn((
            Node {
                border: UiRect::all(px(scale)),
                border_radius: BorderRadius::all(px(2.0 * scale)),
                ..rect(24.0, 64.0, 32.0, 36.0, scale)
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.12)),
            BorderColor::all(Color::srgb(0.38, 0.37, 0.31)),
        ))
        .with_children(|key| {
            key.spawn(text(
                &art.body,
                (index + 1).to_string(),
                18.0 * scale,
                Color::srgb(0.91, 0.8, 0.5),
            ));
        });
        c.spawn(Node {
            column_gap: px(10.0 * scale),
            ..rect(110.0, 64.0, 200.0, 36.0, scale)
        })
        .with_children(|label| {
            label.spawn((
                Node {
                    width: px(14.0 * scale),
                    height: px(18.0 * scale),
                    ..default()
                },
                ImageNode::new(art.diamond[rarity].clone()),
            ));
            label.spawn(text(&art.label, content.rarity_name, 12.0 * scale, rc));
        });
        if content.state == "NEW" {
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(24.0 * scale),
                    top: px(64.0 * scale),
                    padding: UiRect::axes(px(10.0 * scale), px(6.0 * scale)),
                    border_radius: BorderRadius::all(px(2.0 * scale)),
                    ..default()
                },
                BackgroundColor(rc),
            ))
            .with_children(|badge| {
                badge.spawn(text(&art.label, "NEW", 12.0 * scale, Color::BLACK));
            });
        }
        c.spawn((
            Node {
                border: UiRect::all(px(3.0 * scale)),
                border_radius: BorderRadius::all(px(4.0 * scale)),
                ..rect(166.0, 108.0, 88.0, 88.0, scale)
            },
            BorderColor::all(Color::srgb(0.62, 0.59, 0.52)),
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|socket| {
            icon = socket
                .spawn((
                    Node {
                        width: px(80.0 * scale),
                        height: px(80.0 * scale),
                        ..default()
                    },
                    ImageNode::default(),
                ))
                .id();
        });
        c.spawn(rect(24.0, 204.0, 372.0, 36.0, scale))
            .with_children(|title| {
                let size = if content.name.chars().count() > 32 {
                    18.0
                } else if content.name.chars().count() > 24 {
                    22.0
                } else {
                    27.0
                };
                title.spawn(weighted_text(
                    &art.title,
                    content.name,
                    size * scale,
                    rc,
                    FontWeight::BOLD,
                ));
            });
        if let Some(rank) = content.rank {
            c.spawn(rect(24.0, 244.0, 372.0, 20.0, scale))
                .with_children(|label| {
                    label.spawn(text(&art.label, rank, 12.0 * scale, MUTED));
                });
        }
        let rule = Color::srgb(0.46, 0.41, 0.29);
        c.spawn((
            rect(24.0, divider_y, 168.0, 1.0, scale),
            BackgroundColor(rule),
        ));
        c.spawn((
            rect(228.0, divider_y, 168.0, 1.0, scale),
            BackgroundColor(rule),
        ));
        c.spawn((
            rect(205.0, divider_y - 5.0, 10.0, 12.0, scale),
            ImageNode::new(art.diamond[rarity].clone()),
        ));
        c.spawn(rect(40.0, divider_y + 17.0, 340.0, body_height, scale))
            .with_children(|body| {
                let size = if content.description.len() > 360 {
                    12.0
                } else if content.description.len() > 240 {
                    14.0
                } else if content.description.len() > 160 {
                    17.0
                } else {
                    21.0
                };
                body.spawn(text(&art.body, content.description, size * scale, TEXT));
            });
        c.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px((if content.rows.is_empty() { 8.0 } else { 0.0 }) * scale),
                ..rect(24.0, 424.0, 372.0, 112.0, scale)
            },
            ImageNode::new(art.stat[rarity].clone()),
        ))
        .with_children(|stat| {
            if content.rows.is_empty() {
                stat.spawn(text(
                    &art.label,
                    content.stat_label,
                    14.0 * scale,
                    Color::srgb(0.84, 0.74, 0.5),
                ));
                let size = if content.stat_value.chars().count() > 17 {
                    26.0
                } else if content.stat_value.chars().count() > 12 {
                    34.0
                } else {
                    44.0
                };
                stat.spawn(weighted_text(
                    &art.value,
                    content.stat_value,
                    size * scale,
                    if content.positive { POSITIVE } else { TEXT },
                    FontWeight::SEMIBOLD,
                ));
            } else {
                let count = content.rows.len();
                let value_size: f32 = match count {
                    1 => 36.0,
                    2 => 29.0,
                    3 => 23.0,
                    4 => 18.0,
                    _ => 14.0,
                };
                for row in content.rows {
                    let characters = row.before.as_ref().map_or(0, |value| value.chars().count())
                        + row.after.chars().count();
                    let row_size = value_size.min(if characters > 17 {
                        14.0
                    } else if characters > 11 {
                        20.0
                    } else {
                        29.0
                    });
                    stat.spawn(Node {
                        width: px(340.0 * scale),
                        height: px((94.0 / (count as f32)) * scale),
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|line| {
                        line.spawn((
                            Node {
                                width: px(126.0 * scale),
                                ..default()
                            },
                            text(
                                &art.label,
                                row.label,
                                (if count > 3 { 9.0 } else { 12.0 }) * scale,
                                Color::srgb(0.84, 0.74, 0.5),
                            ),
                        ));
                        line.spawn(Node {
                            column_gap: px(10.0 * scale),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|values| {
                            if let Some(before) = row.before {
                                values.spawn(weighted_text(
                                    &art.value,
                                    before,
                                    row_size * scale,
                                    Color::srgb(0.57, 0.58, 0.55),
                                    FontWeight::SEMIBOLD,
                                ));
                                values.spawn(text(&art.body, "→", 22.0 * scale, MUTED));
                            }
                            values.spawn(weighted_text(
                                &art.value,
                                row.after,
                                row_size * scale,
                                if content.positive { POSITIVE } else { TEXT },
                                FontWeight::SEMIBOLD,
                            ));
                        });
                    });
                }
            }
        });
        c.spawn(rect(32.0, 552.0, 356.0, 24.0, scale))
            .with_children(|footer| {
                let size = if content.footer.chars().count() > 42 {
                    14.0
                } else {
                    16.0
                };
                footer.spawn(text(&art.body, content.footer, size * scale, MUTED));
            });
    });
    CardNodes { root, icon }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ornate_frames_keep_transparent_centers_and_full_card_geometry() {
        let borders: [&[u8]; 5] = [
            include_bytes!("../../assets/cards/border-common.png"),
            include_bytes!("../../assets/cards/border-uncommon.png"),
            include_bytes!("../../assets/cards/border-rare.png"),
            include_bytes!("../../assets/cards/border-epic.png"),
            include_bytes!("../../assets/cards/border-legendary.png"),
        ];
        for bytes in borders {
            let image = image::load_from_memory(bytes)
                .expect("ornate border PNG")
                .to_rgba8();
            assert_eq!(image.dimensions(), (420, 640));
            assert_eq!(image.get_pixel(210, 320).0[3], 0);
            assert_eq!(image.get_pixel(0, 0).0[3], 0);
        }
    }
}
