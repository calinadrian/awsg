//! Stock PlayerFrame.xml geometry. The player's frame mirrors the target artwork.
use super::*;

#[derive(Component)]
pub(super) struct RacePortrait(Handle<PortraitMaterial>);

pub(super) fn spawn(
    frame: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    target: bool,
    design: u8,
    materials: &mut Assets<PortraitMaterial>,
) {
    let x = |left: f32, width: f32| if target { 240. - left - width } else { left };
    frame.spawn((
        seat(x(82., 119.), 14., 119., 43.),
        BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
    ));
    frame.spawn((
        seat(x(-24., 232.), -8., 232., 100.),
        ImageNode {
            color: Color::NONE,
            flip_x: !target,
            ..default()
        },
        UiArt::crop(
            "Interface\\TargetingFrame\\UI-TargetingFrame",
            [0.09375, 1., 0., 0.78125],
        ),
        ZIndex(3),
    ));
    let face = seat(x(18., 64.), 4., 64., 64.);
    if target {
        let material = materials.add(PortraitMaterial {
            image: Handle::default(),
        });
        frame.spawn((
            FramePortrait(true, material.clone()),
            face,
            MaterialNode(material),
            ZIndex(2),
        ));
    } else {
        // The race atlas gives a complete Classic portrait without depending on
        // a live model camera, which can crop the head out of this small frame.
        let material = materials.add(PortraitMaterial {
            image: Handle::default(),
        });
        frame.spawn((
            RacePortrait(material.clone()),
            face,
            MaterialNode(material),
            ZIndex(2),
        ));
    }
    let gold = Color::srgb(1., 0.82, 0.);
    frame
        .spawn(Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..seat(x(82., 119.), 14., 119., 17.)
        })
        .with_children(|n| {
            n.spawn((
                FrameName(target),
                label(font, 11., if design == 1 { Color::WHITE } else { gold }),
            ));
        });
    frame
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..seat(x(18., 22.), 50., 22., 16.)
            },
            ZIndex(5),
        ))
        .with_children(|n| {
            n.spawn((FrameLevel(target), label(font, 11., gold)));
        });
    frame
        .spawn((
            seat(x(82., 119.), 33., 119., 12.),
            BackgroundColor(Color::srgb(0.03, 0.08, 0.03)),
        ))
        .with_children(|bar| {
            bar.spawn((
                FrameFill(target),
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ImageNode {
                    color: Color::NONE,
                    ..default()
                },
                UiArt::tinted(
                    "Interface\\TargetingFrame\\UI-StatusBar",
                    Color::srgb(0.1, 0.8, 0.12),
                ),
            ));
            bar.spawn(Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..seat(0., 0., 119., 12.)
            })
            .with_children(|n| {
                n.spawn((FrameHealth(target), label(font, 10., Color::WHITE)));
            });
        });
    frame
        .spawn((
            PowerNode(target, 1, false),
            seat(x(82., 119.), 45., 119., 12.),
            BackgroundColor(Color::srgb(0.01, 0.02, 0.06)),
        ))
        .with_children(|bar| {
            bar.spawn((
                PowerNode(target, 2, false),
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.02, 0.3, 0.95)),
            ));
            bar.spawn(Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..seat(0., 0., 119., 12.)
            })
            .with_children(|n| {
                n.spawn((FramePowerText(target), label(font, 9., Color::WHITE)));
            });
        });
}

pub(super) fn refresh_portraits(
    run: Res<Run>,
    selection: Res<super::super::Selection>,
    portraits: Query<&RacePortrait>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<PortraitMaterial>>,
    mut cached: Local<Option<((u8, u8), Handle<Image>)>>,
) {
    let sex = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(0, |l| l.2);
    let identity = (run.race, sex);
    if portraits.is_empty() {
        return;
    }
    if cached.as_ref().is_none_or(|(key, _)| *key != identity) {
        let Some(coords) = super::super::races::race_tc(run.race, sex) else {
            return;
        };
        let Some(assets) = assets.as_deref_mut() else {
            return;
        };
        let Some((width, height, rgba)) =
            assets.decode_rgba("Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Races")
        else {
            return;
        };
        let [left, right, top, bottom] = coords;
        let x = (left * width as f32).round() as usize;
        let y = (top * height as f32).round() as usize;
        let w = ((right - left) * width as f32).round() as usize;
        let h = ((bottom - top) * height as f32).round() as usize;
        if w == 0 || h == 0 || x + w > width as usize || y + h > height as usize {
            return;
        }
        let mut pixels = Vec::with_capacity(w * h * 4);
        for row in y..y + h {
            let start = (row * width as usize + x) * 4;
            pixels.extend_from_slice(&rgba[start..start + w * 4]);
        }
        // Give the circular portrait shader a standalone atlas cell, so its UVs
        // describe the face rather than the entire rectangular race sheet.
        let image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: w as u32,
                height: h as u32,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            pixels,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        *cached = Some((identity, images.add(image)));
    }
    if let Some((_, image)) = cached.as_ref() {
        for portrait in &portraits {
            if let Some(material) = materials.get_mut(&portrait.0) {
                if material.image != *image {
                    material.image = image.clone();
                }
            }
        }
    }
}
