//! The reward card, in the HUD's steel, ink and wine, lettered in its one font. Every card is one
//! size, tall enough for the longest spell text in the game, so a reroll or a banish never moves the
//! buttons under the cursor. Rarity is a ladder: each step keeps what is below it and adds one thing.
//! Approved design: `docs/mocks/reward_cards.html`, whose sizes below are its units at a 1600 wide
//! screen; [`GAME_SCALE`] turns them into the 768 unit canvas.
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::LineHeight;

pub const WIDTH: f32 = 372.0;
pub const HEIGHT: f32 = 400.0;
pub(super) const HAND_GAP: f32 = 22.0;
/// A 1600 wide screen is 900 high, and the canvas 768.
pub const GAME_SCALE: f32 = 768.0 / 900.0;
/// The HUD's type scale from `front_end.rs`, in these units.
pub(super) const CAPTION: f32 = 11.7;
pub(super) const SMALL: f32 = 14.0;
pub(super) const BODY: f32 = 16.4;
pub(super) const LEAD: f32 = 18.75;
pub(super) const HEADING: f32 = 23.4;
pub(super) const TITLE: f32 = 30.5;
pub(super) const BODY_LINE: f32 = BODY * 1.42;
const MIN_BODY_PIXELS: f32 = 13.0;
/// The frame's border and gap and the panel's border.
#[cfg(test)]
const FRAME: f32 = 8.0;
const HEAD_HEIGHT: f32 = 96.0;
const ICON: f32 = 64.0;
const HEAD_GAP: f32 = 14.0;
const PADDING: f32 = 16.0;
const DESC_HEIGHT: f32 = 130.0;
const DESC_BOTTOM: f32 = 12.0;
const STATS_HEIGHT: f32 = 124.0;
const STATS_INSET: f32 = 12.0;
const FOOT_HEIGHT: f32 = 42.0;
pub(super) const ROW_GAP: f32 = 10.0;
pub(super) const ARROW: f32 = 14.0;
pub(super) const ARROW_GAP: f32 = 4.0;
/// The arrow's drawing, near the size it shows at.
const ARROW_ART: (f32, f32) = (24.0, 14.0);
pub(super) const TAG_PADDING: f32 = 6.0;
pub(super) const TAG_GAP: f32 = 8.0;
/// The room inside the card for each part, for the fit test.
#[cfg(test)]
pub(super) const NAME_WIDTH: f32 = WIDTH - FRAME - 2.0 * PADDING - ICON - HEAD_GAP;
#[cfg(test)]
pub(super) const DESC_WIDTH: f32 = WIDTH - FRAME - 2.0 * PADDING;
#[cfg(test)]
pub(super) const DESC_TEXT_HEIGHT: f32 = DESC_HEIGHT - DESC_BOTTOM;
#[cfg(test)]
pub(super) const ROW_WIDTH: f32 = WIDTH - FRAME - 2.0 * STATS_INSET - 2.0 * STATS_INSET - 2.0;
#[cfg(test)]
pub(super) const FOOT_WIDTH: f32 = WIDTH - FRAME - 2.0 * PADDING;
pub(super) const PIP: f32 = 9.0;
pub(super) const PIP_GAP: f32 = 4.0;
/// What the reward screen stacks around the hand: heading, banish, food and action rows.
const SCREEN_HEIGHT: f32 = 722.0;

/// UI coordinates are 768 units high; retain readable text when the hand fits.
pub(super) fn readable_scale(width: f32, height: f32, count: usize) -> f32 {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return GAME_SCALE;
    }
    let ui_scale = height / 768.0;
    let desired = GAME_SCALE.max(MIN_BODY_PIXELS / (BODY * ui_scale));
    let count = count.max(1) as f32;
    let hand_width = WIDTH * count + HAND_GAP * (count - 1.0);
    let width_limit = ((width / ui_scale - 48.0) / hand_width).max(0.01);
    let height_limit = (768.0 - 24.0) / SCREEN_HEIGHT;
    desired.min(width_limit).min(height_limit)
}

pub const TEXT: Color = Color::srgb(0.91, 0.918, 0.941);
pub const MUTED: Color = Color::srgb(0.545, 0.576, 0.651);
pub const POSITIVE: Color = Color::srgb(30.0 / 255.0, 1.0, 0.0);
const BODY_COLOR: Color = Color::srgb(0.722, 0.741, 0.792);
const WINE: Color = Color::srgb(0.541, 0.122, 0.2);
const WINE_HI: Color = Color::srgb(0.722, 0.227, 0.314);
const STEEL: Color = Color::srgb(0.604, 0.639, 0.722);
pub(super) const STEEL_LO: Color = Color::srgb(0.365, 0.4, 0.475);
const CRIMSON: Color = Color::srgb(0.816, 0.337, 0.416);

pub(super) fn hex(rgb: u32) -> Color {
    Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A gradient stop that lands on screen as this colour. UI gradients reach this game's target
/// without sRGB encoding (#1a202b measured #030406), so the stop carries the encoded values itself.
pub(super) fn card_stop(color: Color, at: f32) -> ColorStop {
    let c = color.to_srgba();
    ColorStop::new(LinearRgba::new(c.red, c.green, c.blue, c.alpha), percent(at))
}

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

/// What each step of the rarity ladder paints.
struct Tier {
    frame: Color,
    /// How strongly the rarity tints the panel's top, and the hovered card's.
    wash: f32,
    hover: Color,
    well: Color,
    gem: f32,
    rule: f32,
    /// The glow around the card, and how far it reaches.
    aura: Option<(f32, f32)>,
    corner: Option<f32>,
}

fn tier(rarity: usize) -> Tier {
    let rc = quality(rarity);
    match rarity {
        0 => Tier {
            frame: STEEL,
            wash: 0.0,
            hover: WINE.with_alpha(0.31),
            well: hex(0x29303b),
            gem: 11.0,
            rule: 2.0,
            aura: None,
            corner: None,
        },
        1 => Tier {
            frame: hex(0x3fae35),
            wash: 0.09,
            hover: rc.with_alpha(0.19),
            well: hex(0x2a4a2a),
            gem: 11.0,
            rule: 2.0,
            aura: None,
            corner: None,
        },
        2 => Tier {
            frame: rc,
            wash: 0.15,
            hover: rc.with_alpha(0.31),
            well: hex(0x1e4570),
            gem: 11.0,
            rule: 2.0,
            aura: Some((16.0, 0.47)),
            corner: Some(14.0),
        },
        3 => Tier {
            frame: rc,
            wash: 0.16,
            hover: rc.with_alpha(0.35),
            well: hex(0x4a2a6b),
            gem: 14.0,
            rule: 3.0,
            aura: Some((22.0, 0.5)),
            corner: Some(18.0),
        },
        _ => Tier {
            frame: hex(0xffa64d),
            wash: 0.19,
            hover: rc.with_alpha(0.38),
            well: hex(0x6b4114),
            gem: 17.0,
            rule: 3.0,
            aura: Some((30.0, 0.53)),
            corner: Some(24.0),
        },
    }
}

#[derive(Resource)]
pub struct CardArt {
    pub title: Handle<Font>,
    pub label: Handle<Font>,
    pub body: Handle<Font>,
    pub value: Handle<Font>,
    arrow: Handle<Image>,
}

// The HUD's font has no U+2192. Draw one shared arrow for ranks and stats.
fn comparison_arrow(images: &mut Assets<Image>) -> Handle<Image> {
    let inside = |x: f32, y: f32| {
        let y = (y - ARROW_ART.1 / 2.0).abs();
        let tip = ARROW_ART.0 - 1.0;
        let shaft = (1.0..=tip - 1.0).contains(&x) && y <= 1.0;
        let head = x <= tip && x >= tip - 7.0 && (y - (tip - x)).abs() <= 1.2;
        shaft || head
    };
    let (width, height) = (ARROW_ART.0 as u32, ARROW_ART.1 as u32);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for py in 0..height {
        for px in 0..width {
            // Four samples a side, so the thin lines keep their edge at small sizes.
            let mut covered = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = px as f32 + (sx as f32 + 0.5) / 4.0;
                    let y = py as f32 + (sy as f32 + 0.5) / 4.0;
                    covered += u32::from(inside(x, y));
                }
            }
            pixels.extend_from_slice(&[0xb8, 0xbd, 0xca, (covered * 255 / 16) as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    images.add(image)
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
    /// The cards letter in the HUD's one font, `font`, like every other survivors screen.
    pub fn load(font: Handle<Font>, images: &mut Assets<Image>) -> Self {
        Self {
            label: font.clone(),
            title: font.clone(),
            body: font.clone(),
            value: font,
            arrow: comparison_arrow(images),
        }
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let font = crate::char_select::wow_font(&asset_server);
    commands.insert_resource(CardArt::load(font, &mut images));
}

pub struct StatRow {
    pub label: String,
    pub before: Option<String>,
    pub after: String,
}

/// The state beside the rarity: what is new fills with wine, a rank stays an outline.
pub enum Tag {
    New(String),
    Rank(String),
}

impl Tag {
    pub(super) fn text(&self) -> &str {
        match self {
            Self::New(text) | Self::Rank(text) => text,
        }
    }
}

pub struct Content {
    pub rarity_name: String,
    pub kind: String,
    pub name: String,
    pub tag: Option<Tag>,
    pub rows: Vec<StatRow>,
    pub description: String,
    pub footer: String,
    /// The slots this reward fills: taken, this one, of how many.
    pub pips: Option<(usize, usize, usize)>,
}

pub struct CardNodes {
    pub root: Entity,
    pub icon: Entity,
    /// Shown while the card is hovered, and when another card is picked.
    pub hover: Entity,
    pub dim: Entity,
    pub frame: (Color, Color),
}

/// Re-shape at the propagated render scale, before Bevy measures the text.
/// A newly dealt hand can otherwise retain glyph metrics from the previous window scale.
#[derive(Component)]
pub(super) struct RasterScale(f32);

pub(super) fn refresh_render_scale(
    mut texts: Query<(
        &bevy::ui::ComputedUiRenderTargetInfo,
        &mut RasterScale,
        &mut TextFont,
    )>,
) {
    for (target, mut previous, mut font) in &mut texts {
        let scale = target.scale_factor();
        if scale.is_finite() && scale > 0.0 && previous.0 != scale {
            previous.0 = scale;
            font.set_changed();
        }
    }
}

pub fn text(font: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        RasterScale(f32::NAN),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
    )
}

fn left_text(font: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        RasterScale(f32::NAN),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextLayout::new_with_justify(Justify::Left),
    )
}

/// A description split into plain and emphasised runs: a number, with the unit that follows it.
pub(super) fn emphasis(text: &str) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    let mut after_number = false;
    for word in text.split(' ') {
        let bare = word.trim_end_matches(['.', ',', ';']);
        let number = word.chars().any(|c| c.is_ascii_digit());
        let unit = after_number
            && matches!(bare, "yd" | "sec" | "ticks" | "tick" | "missile" | "missiles");
        let strong = number || unit;
        // Trailing punctuation reads with the plain text after it.
        let (head, tail) = word.split_at(if strong { bare.len() } else { word.len() });
        match runs.last_mut() {
            Some((run, was)) if *was == strong => {
                run.push(' ');
                run.push_str(head);
            }
            Some((run, _)) if strong => {
                run.push(' ');
                runs.push((head.to_owned(), true));
            }
            Some(_) => runs.push((format!(" {head}"), false)),
            None => runs.push((head.to_owned(), strong)),
        }
        if !tail.is_empty() {
            runs.push((tail.to_owned(), false));
        }
        after_number = number && tail.is_empty();
    }
    runs
}

fn absolute(left: Val, top: Val, width: Val, height: Val) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left,
        top,
        width,
        height,
        ..default()
    }
}

fn square(size: Val) -> Node {
    Node {
        width: size,
        height: size,
        flex_shrink: 0.0,
        ..default()
    }
}

/// A rotated square, the frame's gems and the empty food slot's mark.
pub(super) fn diamond(mut node: Node, color: Color, edge: Color) -> impl Bundle {
    node.border = UiRect::all(px(1.0));
    (
        node,
        BackgroundColor(color),
        BorderColor::all(edge),
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
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
    let u = |v: f32| px(v * scale);
    let rc = quality(rarity);
    let look = tier(rarity);
    let mut shadows = vec![ShadowStyle {
        color: Color::srgba(0.0, 0.0, 0.0, 0.53),
        x_offset: Val::ZERO,
        y_offset: u(8.0),
        spread_radius: Val::ZERO,
        blur_radius: u(20.0),
    }];
    if let Some((reach, alpha)) = look.aura {
        shadows.push(ShadowStyle {
            color: rc.with_alpha(alpha),
            x_offset: Val::ZERO,
            y_offset: Val::ZERO,
            spread_radius: Val::ZERO,
            blur_radius: u(reach),
        });
    }
    let root = commands
        .spawn((
            Button,
            Node {
                width: u(WIDTH),
                height: u(HEIGHT),
                flex_shrink: 0.0,
                padding: UiRect::all(px(2.0)),
                border: UiRect::all(px(1.0)),
                ..default()
            },
            BackgroundColor(hex(0x080b0f)),
            BorderColor::all(look.frame),
            BoxShadow(shadows),
        ))
        .id();
    let mut icon = Entity::PLACEHOLDER;
    let mut hover = Entity::PLACEHOLDER;
    let mut dim = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|c| {
        spawn_behind(c, rarity, scale);
        c.spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect {
                    left: px(1.0),
                    right: px(1.0),
                    bottom: px(1.0),
                    top: u(look.rule),
                },
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::new(
                160f32.to_radians(),
                vec![
                    card_stop(hex(0x1a202b), 0.0),
                    card_stop(hex(0x0b1215), 62.0),
                ],
            )),
            BorderColor {
                top: rc,
                right: hex(0x343c49),
                bottom: hex(0x343c49),
                left: hex(0x343c49),
            },
        ))
        .with_children(|panel| {
            let fill = || absolute(Val::ZERO, Val::ZERO, percent(100.0), percent(100.0));
            if look.wash > 0.0 {
                panel.spawn((
                    fill(),
                    BackgroundGradient::from(LinearGradient::to_bottom(vec![
                        card_stop(rc.with_alpha(look.wash), 0.0),
                        card_stop(rc.with_alpha(0.0), 48.0),
                    ])),
                ));
            }
            spawn_inner_fx(panel, rarity, scale, index);
            hover = panel
                .spawn((
                    fill(),
                    BackgroundGradient::from(LinearGradient::to_bottom(vec![
                        card_stop(look.hover, 0.0),
                        card_stop(look.hover.with_alpha(0.0), 62.0),
                    ])),
                    Visibility::Hidden,
                ))
                .id();
            icon = spawn_head(panel, art, &content, index, rarity, scale);
            spawn_description(panel, art, &content.description, scale);
            spawn_rows(panel, art, &content.rows, look.well, scale);
            spawn_foot(panel, art, &content.footer, content.pips, scale);
            spawn_flash(panel, rarity, index);
            dim = panel
                .spawn((
                    fill(),
                    BackgroundColor(Color::srgba(0.02, 0.027, 0.039, 0.72)),
                    Visibility::Hidden,
                ))
                .id();
        });
        spawn_frame_marks(c, rarity, scale);
    });
    CardNodes {
        root,
        icon,
        hover,
        dim,
        frame: (look.frame, rc),
    }
}

fn spawn_head(
    panel: &mut ChildSpawnerCommands,
    art: &CardArt,
    content: &Content,
    index: usize,
    rarity: usize,
    scale: f32,
) -> Entity {
    let u = |v: f32| px(v * scale);
    let rc = quality(rarity);
    let mut icon = Entity::PLACEHOLDER;
    panel
        .spawn(Node {
            height: u(HEAD_HEIGHT),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: u(HEAD_GAP),
            padding: UiRect::new(u(PADDING), u(PADDING), u(4.0), Val::ZERO),
            ..default()
        })
        .with_children(|head| {
            let mut shadows = vec![ShadowStyle {
                color: Color::BLACK,
                x_offset: Val::ZERO,
                y_offset: Val::ZERO,
                spread_radius: px(1.0),
                blur_radius: Val::ZERO,
            }];
            if rarity >= 2 {
                shadows.push(ShadowStyle {
                    color: rc,
                    x_offset: Val::ZERO,
                    y_offset: Val::ZERO,
                    spread_radius: Val::ZERO,
                    blur_radius: u(12.0),
                });
            }
            head.spawn((
                Node {
                    padding: UiRect::all(u(3.0)),
                    border: UiRect::all(px(1.0)),
                    ..square(u(ICON))
                },
                BackgroundColor(hex(0x11151b)),
                BorderColor::all(rc),
                BoxShadow(shadows),
            ))
            .with_children(|socket| {
                icon = socket
                    .spawn((
                        Node {
                            width: percent(100.0),
                            height: percent(100.0),
                            ..default()
                        },
                        ImageNode::default(),
                    ))
                    .id();
                socket
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: u(-6.0),
                            top: u(-6.0),
                            width: u(20.0),
                            height: u(20.0),
                            border: UiRect::all(px(1.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(hex(0x10141d)),
                        BorderColor::all(STEEL_LO),
                    ))
                    .with_children(|key| {
                        key.spawn(text(
                            &art.label,
                            (index + 1).to_string(),
                            CAPTION * scale,
                            TEXT,
                        ));
                    });
            });
            head.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::ZERO,
                min_width: Val::ZERO,
                row_gap: u(7.0),
                ..default()
            })
            .with_children(|side| {
                side.spawn(left_text(
                    &art.title,
                    content.name.as_str(),
                    HEADING * scale,
                    rc,
                ));
                side.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: u(TAG_GAP),
                    ..default()
                })
                .with_children(|tags| spawn_tags(tags, art, content, rarity, scale));
            });
        });
    icon
}

fn spawn_tags(
    tags: &mut ChildSpawnerCommands,
    art: &CardArt,
    content: &Content,
    rarity: usize,
    scale: f32,
) {
    let u = |v: f32| px(v * scale);
    let rc = quality(rarity);
    let size = CAPTION * scale;
    tags.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: u(4.0),
        ..default()
    })
    .with_children(|label| {
        // Epic and above fill their name with the rarity.
        if rarity >= 3 {
            let ink = if rarity == 3 { Color::WHITE } else { hex(0x140c04) };
            label
                .spawn((
                    Node {
                        padding: UiRect::axes(u(TAG_PADDING), u(2.0)),
                        ..default()
                    },
                    BackgroundColor(rc),
                ))
                .with_children(|chip| {
                    chip.spawn(text(&art.label, content.rarity_name.as_str(), size, ink));
                });
        } else {
            let color = match rarity {
                0 => MUTED,
                // Quality blue is too dark for text this small.
                2 => hex(0x66aaeb),
                _ => rc,
            };
            label.spawn(text(&art.label, content.rarity_name.as_str(), size, color));
        }
        label.spawn(text(&art.label, content.kind.as_str(), size, MUTED));
    });
    let Some(tag) = &content.tag else { return };
    let (background, edge, ink) = match tag {
        Tag::New(_) => (WINE, WINE_HI, Color::WHITE),
        Tag::Rank(_) => (Color::NONE, hex(0x465163), hex(0xd0d4dd)),
    };
    tags.spawn((
        Node {
            padding: UiRect::axes(u(TAG_PADDING), u(2.0)),
            border: UiRect::all(px(1.0)),
            align_items: AlignItems::Center,
            column_gap: u(ARROW_GAP),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(edge),
    ))
    .with_children(|chip| {
        if let Some((before, after)) = tag.text().split_once('→') {
            chip.spawn(text(&art.label, before.trim(), size, ink));
            spawn_arrow(chip, art, scale);
            chip.spawn(text(&art.label, after.trim(), size, ink));
        } else {
            chip.spawn(text(&art.label, tag.text(), size, ink));
        }
    });
}

fn spawn_arrow(parent: &mut ChildSpawnerCommands, art: &CardArt, scale: f32) {
    parent.spawn((
        Node {
            width: px(ARROW * scale),
            height: px(ARROW * scale * ARROW_ART.1 / ARROW_ART.0),
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode::new(art.arrow.clone()),
    ));
}

fn spawn_description(panel: &mut ChildSpawnerCommands, art: &CardArt, description: &str, scale: f32) {
    let u = |v: f32| px(v * scale);
    panel
        .spawn(Node {
            height: u(DESC_HEIGHT),
            flex_shrink: 0.0,
            padding: UiRect::new(u(PADDING), u(PADDING), Val::ZERO, u(DESC_BOTTOM)),
            ..default()
        })
        .with_children(|desc| {
            let font = TextFont {
                font: art.body.clone(),
                font_size: BODY * scale,
                ..default()
            };
            let line = LineHeight::Px(BODY_LINE * scale);
            desc.spawn((
                Text::default(),
                RasterScale(f32::NAN),
                font.clone(),
                line,
                TextColor(BODY_COLOR),
                TextLayout::new_with_justify(Justify::Left),
            ))
            .with_children(|body| {
                for (run, strong) in emphasis(description) {
                    body.spawn((
                        TextSpan::new(run),
                        font.clone(),
                        line,
                        TextColor(if strong { TEXT } else { BODY_COLOR }),
                    ));
                }
            });
        });
}

fn spawn_rows(
    panel: &mut ChildSpawnerCommands,
    art: &CardArt,
    rows: &[StatRow],
    well: Color,
    scale: f32,
) {
    let u = |v: f32| px(v * scale);
    panel
        .spawn((
            Node {
                height: u(STATS_HEIGHT),
                flex_shrink: 0.0,
                margin: UiRect::horizontal(u(STATS_INSET)),
                padding: UiRect::axes(u(STATS_INSET), u(4.0)),
                border: UiRect::all(px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(hex(0x05080b)),
            BorderColor::all(well),
        ))
        .with_children(|stats| {
            let alone = rows.len() == 1;
            for (i, row) in rows.iter().enumerate() {
                stats
                    .spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::ZERO,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: u(ROW_GAP),
                            border: UiRect::top(px(if i > 0 { 1.0 } else { 0.0 })),
                            ..default()
                        },
                        BorderColor::all(hex(0x1d232c)),
                    ))
                    .with_children(|line| {
                        line.spawn((
                            Node {
                                flex_shrink: 0.0,
                                ..default()
                            },
                            left_text(&art.label, row.label.as_str(), CAPTION * scale, MUTED),
                        ));
                        line.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: u(ARROW_GAP),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|values| {
                            if let Some(before) = &row.before {
                                values.spawn(text(
                                    &art.value,
                                    before.as_str(),
                                    SMALL * scale,
                                    MUTED,
                                ));
                                spawn_arrow(values, art, scale);
                                values.spawn(text(
                                    &art.value,
                                    row.after.as_str(),
                                    LEAD * scale,
                                    POSITIVE,
                                ));
                            } else {
                                let size = if alone { TITLE } else { LEAD };
                                values.spawn(text(
                                    &art.value,
                                    row.after.as_str(),
                                    size * scale,
                                    TEXT,
                                ));
                            }
                        });
                    });
            }
        });
}

fn spawn_foot(
    panel: &mut ChildSpawnerCommands,
    art: &CardArt,
    footer: &str,
    pips: Option<(usize, usize, usize)>,
    scale: f32,
) {
    let u = |v: f32| px(v * scale);
    panel
        .spawn(Node {
            height: u(FOOT_HEIGHT),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(u(PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: u(ROW_GAP),
            ..default()
        })
        .with_children(|foot| {
            foot.spawn(left_text(&art.body, footer, SMALL * scale, MUTED));
            let Some((taken, next, total)) = pips else {
                return;
            };
            foot.spawn(Node {
                column_gap: u(PIP_GAP),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|row| {
                for i in 0..total {
                    let (fill, edge) = if i < taken {
                        (hex(0x8d97a9), hex(0x5d6778))
                    } else if i < taken + next {
                        (CRIMSON, hex(0xf0a3b1))
                    } else {
                        (hex(0x0a0d11), hex(0x5d6778))
                    };
                    row.spawn((
                        Node {
                            border: UiRect::all(px(1.0)),
                            ..square(u(PIP))
                        },
                        BackgroundColor(fill),
                        BorderColor::all(edge),
                    ));
                }
            });
        });
}

/// The moving parts of the rarity ladder.
#[derive(Component)]
pub(super) enum CardFx {
    /// Epic and up: the light behind the card, breathing at this period.
    Halo { color: Color, alpha: f32, period: f32, t: f32 },
    /// Legendary: a band of light crossing the panel.
    Sweep { t: f32 },
    /// Legendary: a spark rising through the panel, by this much.
    Ember { rise: f32, t: f32 },
    /// Epic and up: the flash as the card lands.
    Flash { color: Color, t: f32 },
}

const SWEEP_SECS: f32 = 4.5;
const EMBER_SECS: f32 = 3.4;
const FLASH_SECS: f32 = 0.6;
const DEAL_STAGGER: f32 = 0.055;

fn halo_gradient(color: Color, alpha: f32) -> BackgroundGradient {
    BackgroundGradient::from(RadialGradient::new(
        UiPosition::CENTER,
        RadialGradientShape::ClosestSide,
        vec![
            card_stop(color.with_alpha(alpha), 0.0),
            card_stop(color.with_alpha(0.0), 100.0),
        ],
    ))
}

/// What shines from behind the card, past its edges.
fn spawn_behind(card: &mut ChildSpawnerCommands, rarity: usize, scale: f32) {
    if rarity < 3 {
        return;
    }
    let u = |v: f32| px(v * scale);
    let color = quality(rarity);
    let (alpha, period) = if rarity == 3 { (0.36, 2.8) } else { (0.58, 2.0) };
    card.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: u(-34.0),
            right: u(-34.0),
            top: u(-44.0),
            bottom: u(-24.0),
            ..default()
        },
        halo_gradient(color, alpha),
        CardFx::Halo {
            color,
            alpha,
            period,
            t: 0.0,
        },
    ));
    if rarity >= 4 {
        card.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.0),
                top: u(-58.0),
                width: u(260.0),
                height: u(116.0),
                margin: UiRect::left(u(-130.0)),
                ..default()
            },
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::ClosestSide,
                vec![
                    card_stop(hex(0xffc477).with_alpha(0.56), 0.0),
                    card_stop(hex(0xffc477).with_alpha(0.0), 100.0),
                ],
            )),
        ));
    }
}

/// What plays inside the panel, under the text.
fn spawn_inner_fx(panel: &mut ChildSpawnerCommands, rarity: usize, scale: f32, index: usize) {
    let u = |v: f32| px(v * scale);
    if rarity >= 3 {
        panel.spawn((
            absolute(u(-48.0), u(-52.0), u(220.0), u(220.0)),
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::ClosestSide,
                vec![
                    card_stop(quality(rarity).with_alpha(0.42), 0.0),
                    card_stop(quality(rarity).with_alpha(0.0), 100.0),
                ],
            )),
        ));
    }
    if rarity >= 4 {
        let light = hex(0xffe2b0);
        panel.spawn((
            absolute(Val::ZERO, Val::ZERO, percent(100.0), percent(100.0)),
            BackgroundGradient::from(LinearGradient::new(
                105f32.to_radians(),
                vec![
                    card_stop(light.with_alpha(0.0), 38.0),
                    card_stop(light.with_alpha(0.21), 50.0),
                    card_stop(light.with_alpha(0.0), 62.0),
                ],
            )),
            UiTransform::from_translation(Val2::percent(-110.0, 0.0)),
            CardFx::Sweep {
                t: -(index as f32) * DEAL_STAGGER,
            },
        ));
        for (n, x) in [12.0, 27.0, 41.0, 58.0, 72.0, 88.0].into_iter().enumerate() {
            panel.spawn((
                diamond_ember(percent(x), u(5.0)),
                CardFx::Ember {
                    rise: 400.0 * scale,
                    t: -(n as f32) * 0.57,
                },
            ));
        }
    }
}

fn diamond_ember(left: Val, size: Val) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left,
            bottom: Val::ZERO,
            width: size,
            height: size,
            ..default()
        },
        BackgroundColor(hex(0xffc477).with_alpha(0.0)),
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
    )
}

fn spawn_flash(panel: &mut ChildSpawnerCommands, rarity: usize, index: usize) {
    if rarity < 3 {
        return;
    }
    let color = quality(rarity).mix(&Color::WHITE, 0.55);
    panel.spawn((
        absolute(Val::ZERO, Val::ZERO, percent(100.0), percent(100.0)),
        BackgroundColor(color.with_alpha(0.0)),
        CardFx::Flash {
            color,
            t: -(index as f32) * DEAL_STAGGER,
        },
    ));
}

/// The diamond on the frame's top edge, and from Rare up its brackets and gems.
fn spawn_frame_marks(card: &mut ChildSpawnerCommands, rarity: usize, scale: f32) {
    let u = |v: f32| px(v * scale);
    let rc = quality(rarity);
    let look = tier(rarity);
    let edge = hex(0x0a0c10);
    card.spawn(diamond(
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            top: u(-look.gem / 2.0 - 1.0),
            width: u(look.gem),
            height: u(look.gem),
            margin: UiRect::left(u(-look.gem / 2.0)),
            ..default()
        },
        rc,
        edge,
    ));
    let gems: &[f32] = match rarity {
        3 => &[-22.0, 14.0],
        4 => &[-44.0, -26.0, 18.0, 36.0],
        _ => &[],
    };
    for &x in gems {
        card.spawn(diamond(
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.0),
                top: u(-4.5),
                width: u(8.0),
                height: u(8.0),
                margin: UiRect::left(u(x)),
                ..default()
            },
            rc,
            edge,
        ));
    }
    let Some(corner) = look.corner else { return };
    let width = u(2.0);
    let out = u(-3.0);
    for (left, right, top, bottom) in [
        (true, false, true, false),
        (false, true, true, false),
        (true, false, false, true),
        (false, true, false, true),
    ] {
        card.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: if left { out } else { Val::Auto },
                right: if right { out } else { Val::Auto },
                top: if top { out } else { Val::Auto },
                bottom: if bottom { out } else { Val::Auto },
                width: u(corner),
                height: u(corner),
                border: UiRect {
                    left: if left { width } else { Val::ZERO },
                    right: if right { width } else { Val::ZERO },
                    top: if top { width } else { Val::ZERO },
                    bottom: if bottom { width } else { Val::ZERO },
                },
                ..default()
            },
            BorderColor::all(rc),
        ));
    }
}

pub(super) fn animate_fx(
    time: Res<Time<Real>>,
    mut fx: Query<(
        &mut CardFx,
        Option<&mut BackgroundGradient>,
        Option<&mut BackgroundColor>,
        Option<&mut UiTransform>,
    )>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut fx, gradient, background, transform) in &mut fx {
        match &mut *fx {
            CardFx::Halo {
                color,
                alpha,
                period,
                t,
            } => {
                *t = (*t + dt) % *period;
                let breath = 0.5 - 0.5 * (*t / *period * std::f32::consts::TAU).cos();
                if let Some(mut gradient) = gradient {
                    *gradient = halo_gradient(*color, *alpha * (0.55 + 0.45 * breath));
                }
            }
            CardFx::Sweep { t } => {
                *t += dt;
                if *t >= SWEEP_SECS {
                    *t -= SWEEP_SECS;
                }
                // It waits for the first half of the cycle, then crosses.
                let cross = ((*t / SWEEP_SECS - 0.55) / 0.45).clamp(0.0, 1.0);
                if let Some(mut transform) = transform {
                    transform.translation = Val2::percent(-110.0 + 220.0 * cross, 0.0);
                }
            }
            CardFx::Ember { rise, t } => {
                *t += dt;
                if *t >= EMBER_SECS {
                    *t -= EMBER_SECS;
                }
                let along = (*t / EMBER_SECS).max(0.0);
                let alpha = if *t < 0.0 {
                    0.0
                } else if along < 0.12 {
                    along / 0.12 * 0.9
                } else {
                    (1.0 - along) / 0.88 * 0.9
                };
                if let Some(mut background) = background {
                    background.0 = background.0.with_alpha(alpha);
                }
                if let Some(mut transform) = transform {
                    transform.translation = Val2::px(0.0, -*rise * along);
                }
            }
            CardFx::Flash { color, t } => {
                if *t > FLASH_SECS {
                    continue;
                }
                *t += dt;
                let alpha = if *t < 0.0 {
                    0.0
                } else {
                    0.8 * (1.0 - *t / FLASH_SECS).max(0.0)
                };
                if let Some(mut background) = background {
                    background.0 = color.with_alpha(alpha);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_hand_preserves_body_size_at_standard_resolutions() {
        for (width, height) in [(1366.0, 768.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
            let scale = readable_scale(width, height, 3);
            let screen_scale = height / 768.0;
            assert!(BODY * scale * screen_scale >= 13.0 - 0.001);
            assert!(
                (WIDTH * 3.0 + HAND_GAP * 2.0) * scale * screen_scale
                    <= width - 48.0 * screen_scale
            );
            assert!(SCREEN_HEIGHT * scale <= 768.0);
        }
    }

    #[test]
    fn narrow_viewports_prioritize_fitting_the_whole_hand() {
        let scale = readable_scale(480.0, 768.0, 3);
        assert!(scale < GAME_SCALE);
        assert!((WIDTH * 3.0 + HAND_GAP * 2.0) * scale <= 432.001);
        assert_eq!(readable_scale(0.0, 0.0, 3), GAME_SCALE);
    }

    #[test]
    fn card_sections_add_up_to_the_card() {
        assert_eq!(
            HEAD_HEIGHT + DESC_HEIGHT + STATS_HEIGHT + FOOT_HEIGHT + FRAME,
            HEIGHT
        );
    }

    #[test]
    fn emphasis_marks_numbers_with_their_units_and_keeps_every_word() {
        let text = "Fires 2 missiles for 94.7 damage each within 22 yd. Heals 3% of health.";
        let runs = emphasis(text);
        let joined: String = runs.iter().map(|(run, _)| run.as_str()).collect();
        assert_eq!(joined, text);
        let strong: Vec<&str> = runs
            .iter()
            .filter(|(_, strong)| *strong)
            .map(|(run, _)| run.as_str())
            .collect();
        assert_eq!(strong, ["2 missiles", "94.7", "22 yd", "3%"]);
    }
}
