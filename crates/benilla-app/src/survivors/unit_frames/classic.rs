//! Stock PlayerFrame.xml geometry. The player's frame mirrors the target artwork.
use super::*;

#[derive(Component)]
pub(super) struct RacePortrait;

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
        frame.spawn((
            RacePortrait,
            face,
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            UiArt::crop(
                "Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Races",
                [0., 0.25, 0., 0.25],
            ),
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
        .spawn(Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..seat(x(20., 22.), 51., 22., 16.)
        })
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
    mut commands: Commands,
    run: Res<Run>,
    selection: Res<super::super::Selection>,
    portraits: Query<Entity, With<RacePortrait>>,
    mut last: Local<Option<(u8, u8)>>,
) {
    let sex = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(0, |l| l.2);
    let identity = (run.race, sex);
    if *last == Some(identity) || portraits.is_empty() {
        return;
    }
    if let Some(coords) = super::super::races::race_tc(run.race, sex) {
        for entity in &portraits {
            commands
                .entity(entity)
                .remove::<super::super::hud::ArtDone>()
                .insert(UiArt::crop(
                    "Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Races",
                    coords,
                ));
        }
    }
    *last = Some(identity);
}
