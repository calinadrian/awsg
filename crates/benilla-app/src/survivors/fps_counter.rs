//! Optional framerate display, averaged over half a second without diagnostic logging.
use super::{hud::HudFont, hud_layout::MENU_Z};
use bevy::prelude::*;
use bevy::time::Real;

#[derive(Resource, Default)]
pub(super) struct FpsSettings {
    pub enabled: bool,
}

impl FpsSettings {
    pub(super) fn toggle(&mut self) {
        self.enabled = !self.enabled;
        if let Some(path) = crate::local_state::home().map(|h| h.join("survivors-fps.txt")) {
            if let Err(error) =
                crate::local_state::write_atomic(&path, if self.enabled { "1\n" } else { "0\n" })
            {
                warn!("Could not save FPS counter setting: {error}");
            }
        }
    }
}

#[derive(Component)]
struct Counter;

pub(super) fn plugin(app: &mut App) {
    let enabled = crate::local_state::home()
        .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-fps.txt")).ok())
        .is_some_and(|s| s.trim() == "1");
    app.insert_resource(FpsSettings { enabled })
        .add_systems(Update, (spawn, update).chain());
}

fn spawn(mut commands: Commands, font: Option<Res<HudFont>>, counter: Query<(), With<Counter>>) {
    if !counter.is_empty() {
        return;
    }
    let Some(font) = font else { return };
    commands.spawn((
        Counter,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.),
            bottom: Val::Px(12.),
            padding: UiRect::axes(Val::Px(7.), Val::Px(3.)),
            display: Display::None,
            ..default()
        },
        Text::new("FPS …"),
        TextFont {
            font: font.0.clone(),
            font_size: 12.,
            ..default()
        },
        TextColor(Color::srgb(0.65, 1., 0.65)),
        BackgroundColor(Color::srgba(0., 0., 0., 0.75)),
        GlobalZIndex(MENU_Z + 80),
        Pickable::IGNORE,
    ));
}

fn update(
    settings: Res<FpsSettings>,
    time: Res<Time<Real>>,
    mut counter: Query<(&mut Node, &mut Text), With<Counter>>,
    mut sample: Local<(f64, u32)>,
) {
    let Ok((mut node, mut text)) = counter.single_mut() else {
        return;
    };
    let display = if settings.enabled {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    if !settings.enabled {
        *sample = (0., 0);
        return;
    }
    let delta = time.delta_secs_f64();
    if delta <= 0. {
        return;
    }
    sample.0 += delta;
    sample.1 += 1;
    if sample.0 >= 0.5 {
        let fps = f64::from(sample.1) / sample.0;
        let ms = sample.0 * 1000. / f64::from(sample.1);
        text.set_if_neq(Text::new(format!("{fps:.0} FPS  ·  {ms:.1} ms")));
        *sample = (0., 0);
    }
}
