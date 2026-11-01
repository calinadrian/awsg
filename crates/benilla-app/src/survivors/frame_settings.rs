//! Saved placement and right-click controls for survivors unit frames.
use super::{
    hud::{HudFont, HudRoot},
    Phase,
};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub(super) const SIZE: Vec2 = Vec2::new(240., 76.);
const MENU: Vec2 = Vec2::new(240., 326.);
const ROW: f32 = 28.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    pub pos: Vec2,
    pub scale: f32,
    pub locked: bool,
    pub health: bool,
    pub design: u8,
    pub level: bool,
}
fn defaults(index: usize) -> Placement {
    Placement {
        pos: Vec2::new(16. + index as f32 * 248., 16.),
        scale: 1.,
        locked: true,
        health: true,
        design: 0,
        level: true,
    }
}
impl Placement {
    fn clamp(&mut self, canvas: Vec2) {
        self.scale = if self.scale.is_finite() {
            self.scale.clamp(0.65, 1.6)
        } else {
            1.
        };
        if !self.pos.is_finite() {
            self.pos = Vec2::splat(16.);
        }
        self.pos = self
            .pos
            .clamp(Vec2::ZERO, (canvas - SIZE * self.scale).max(Vec2::ZERO));
    }
    fn bounds(&self) -> Rect {
        Rect::from_corners(self.pos, self.pos + SIZE * self.scale)
    }
}

#[derive(Resource)]
pub(super) struct FrameSettings {
    pub items: [Placement; 2],
    menu: Option<(usize, Vec2)>,
    drag: Option<(usize, Vec2)>,
    captured: bool,
    draft: Option<(usize, u8)>,
}
impl Default for FrameSettings {
    fn default() -> Self {
        Self {
            items: [defaults(0), defaults(1)],
            menu: None,
            drag: None,
            captured: false,
            draft: None,
        }
    }
}
impl FrameSettings {
    fn menu_size(&self) -> Vec2 {
        if self.draft.is_some() {
            Vec2::new(MENU.x, 186.)
        } else {
            MENU
        }
    }
    fn close_menu(&mut self) {
        if let Some((index, original)) = self.draft.take() {
            self.items[index].design = original;
        }
        self.menu = None;
    }
    fn decode(text: &str) -> Self {
        let mut result = Self::default();
        for (index, line) in text.lines().take(2).enumerate() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() != 6 && fields.len() != 7 {
                continue;
            }
            let (Ok(x), Ok(y), Ok(scale)) = (
                fields[0].parse::<f32>(),
                fields[1].parse::<f32>(),
                fields[2].parse::<f32>(),
            ) else {
                continue;
            };
            let mut item = Placement {
                pos: Vec2::new(x, y),
                scale,
                locked: fields[3] != "0",
                health: fields[4] != "0",
                design: fields
                    .get(6)
                    .and_then(|s| s.parse::<u8>().ok())
                    .filter(|v| *v < 3)
                    .unwrap_or(0),
                level: fields[5] != "0",
            };
            item.clamp(Vec2::splat(16384.));
            result.items[index] = item;
        }
        result
    }
    fn encode(&self) -> String {
        self.items
            .iter()
            .map(|p| {
                format!(
                    "{} {} {} {} {} {} {}\n",
                    p.pos.x,
                    p.pos.y,
                    p.scale,
                    u8::from(p.locked),
                    u8::from(p.health),
                    u8::from(p.level),
                    p.design
                )
            })
            .collect()
    }
    fn save(&self) {
        if let Some(path) = crate::local_state::home().map(|h| h.join("survivors-unit-frames.txt"))
        {
            if let Err(error) = crate::local_state::write_atomic(&path, &self.encode()) {
                warn!("Could not save unit-frame settings: {error}");
            }
        }
    }
}

#[derive(Component)]
struct SettingsMenu;
#[derive(Component)]
struct MenuLine(usize);
#[derive(Component)]
struct MenuRow(usize);

pub(super) fn plugin(app: &mut App) {
    let settings = crate::local_state::home()
        .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-unit-frames.txt")).ok())
        .map_or_else(FrameSettings::default, |s| FrameSettings::decode(&s));
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    let settings = if std::env::var_os("WOW_SURVIVORS_FRAME_SETTINGS_PREVIEW").is_some() {
        let mut preview = FrameSettings::default();
        preview.items[0].locked = false;
        preview.items[1].locked = false;
        preview.menu = Some((0, Vec2::new(520., 16.)));
        preview
    } else {
        settings
    };
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    let settings = if let Ok(design) = std::env::var("WOW_SURVIVORS_FRAME_DESIGN") {
        let mut settings = settings;
        let design = design.parse::<u8>().unwrap_or(0).min(2);
        for p in &mut settings.items {
            p.design = design;
        }
        settings
    } else {
        settings
    };
    app.insert_resource(settings)
        .add_systems(
            Update,
            input
                .after(crate::ui_script::UiInput)
                .before(benilla_world::schedule::WorldStage::Input),
        )
        .add_systems(Update, (spawn_menu, paint_menu).chain());
}

fn spawn_menu(
    mut commands: Commands,
    roots: Query<Entity, Added<HudRoot>>,
    font: Option<Res<HudFont>>,
) {
    let Some(font) = font else { return };
    for root in &roots {
        commands.entity(root).with_children(|parent| {
            parent
                .spawn((
                    SettingsMenu,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(MENU.x),
                        height: Val::Px(MENU.y),
                        display: Display::None,
                        border: UiRect::all(Val::Px(1.)),
                        padding: UiRect::all(Val::Px(8.)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.025, 0.035, 0.98)),
                    BorderColor::all(Color::srgb(0.7, 0.55, 0.2)),
                    GlobalZIndex(900),
                ))
                .with_children(|panel| {
                    for line in 0..11 {
                        panel
                            .spawn((
                                MenuRow(line),
                                BackgroundColor(Color::NONE),
                                Node {
                                    width: Val::Percent(100.),
                                    height: Val::Px(ROW),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|row| {
                                row.spawn((
                                    MenuLine(line),
                                    Text::new(""),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1., 0.85, 0.4)),
                                ));
                            });
                    }
                });
        });
    }
}

fn paint_menu(
    settings: Res<FrameSettings>,
    mut menus: Query<&mut Node, (With<SettingsMenu>, Without<MenuRow>)>,
    mut lines: Query<(&MenuLine, &mut Text)>,
    mut rows: Query<(&MenuRow, &mut BackgroundColor, &mut Node), Without<SettingsMenu>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
) {
    for mut node in &mut menus {
        node.height = Val::Px(settings.menu_size().y);
        node.display = if settings.menu.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some((_, pos)) = settings.menu {
            node.left = Val::Px(pos.x);
            node.top = Val::Px(pos.y);
        }
    }
    let Some((index, _)) = settings.menu else {
        return;
    };
    let cursor = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .map(|p| p / scale.0.max(0.01));
    let menu_pos = settings.menu.unwrap().1;
    for (row, mut color, mut node) in &mut rows {
        node.display = if settings.draft.is_some() && row.0 >= 6 {
            Display::None
        } else {
            Display::Flex
        };
        let hot = row.0 > 0
            && cursor.is_some_and(|p| {
                Rect::from_corners(
                    menu_pos + Vec2::new(9., 9. + ROW * row.0 as f32),
                    menu_pos + Vec2::new(MENU.x - 9., 9. + ROW * (row.0 + 1) as f32),
                )
                .contains(p)
            });
        color.0 = if hot {
            Color::srgb(0.24, 0.19, 0.08)
        } else if row.0 == 0 {
            Color::srgb(0.10, 0.08, 0.04)
        } else {
            Color::NONE
        };
    }
    let p = settings.items[index];
    let labels: Vec<String> = if settings.draft.is_some() {
        vec![
            "Preview design".into(),
            format!("{} Classic", if p.design == 0 { "[x]" } else { "[ ]" }),
            format!(
                "{} Classic Silver",
                if p.design == 1 { "[x]" } else { "[ ]" }
            ),
            format!("{} Compact", if p.design == 2 { "[x]" } else { "[ ]" }),
            "Apply design".into(),
            "Cancel".into(),
        ]
    } else {
        vec![
            format!("{} frame", if index == 0 { "Player" } else { "Target" }),
            if p.locked {
                "Unlock — drag to move"
            } else {
                "Lock position"
            }
            .into(),
            format!("Smaller   (scale {:.0}%)", p.scale * 100.),
            "Larger".into(),
            format!("Bar numbers: {}", if p.health { "On" } else { "Off" }),
            format!("Level: {}", if p.level { "On" } else { "Off" }),
            "Reset this frame".into(),
            "Unlock both frames".into(),
            "Lock both frames".into(),
            format!(
                "Design: {} >",
                ["Classic", "Classic Silver", "Compact"][p.design as usize]
            ),
            "Close".into(),
        ]
    };
    for (line, mut text) in &mut lines {
        if line.0 >= labels.len() {
            continue;
        }
        if text.0 != labels[line.0] {
            text.0 = labels[line.0].clone();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn input(
    mut settings: ResMut<FrameSettings>,
    phase: Res<State<Phase>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    target: Res<crate::target::Selection>,
    enemies: Query<&super::enemies::Enemy>,
    mut hover: ResMut<crate::ui_script::PlayerUiHover>,
    mut pointer: ResMut<crate::ui_script::PointerOverUi>,
    mut panel: ResMut<crate::ui_script::PointerOverUiPanel>,
    mut consumed: ResMut<crate::ui_script::PlayerUiClickConsumed>,
) {
    const TOKEN: u32 = u32::MAX - 64;
    if hover.0 == Some(TOKEN) {
        hover.0 = None;
        pointer.0 = false;
        panel.0 = false;
    }
    if matches!(phase.get(), Phase::Menu | Phase::GameOver) {
        settings.drag = None;
        settings.captured = false;
        return;
    }
    let Ok(window) = windows.single() else { return };
    let scale = ui_scale.0.max(0.01);
    let canvas = Vec2::new(window.width(), window.height()) / scale;
    for item in &mut settings.items {
        item.clamp(canvas);
    }
    let Some(cursor) = window.cursor_position().map(|p| p / scale) else {
        if settings.drag.take().is_some() {
            settings.save();
        }
        return;
    };
    let has_target = target
        .target
        .and_then(|e| enemies.get(e).ok())
        .is_some_and(|e| !e.dead && e.hp > 0.);
    let hit = (0..2).rev().find(|&i| {
        (i == 0 || has_target || !settings.items[i].locked)
            && settings.items[i].bounds().contains(cursor)
    });
    let menu_hit = settings
        .menu
        .is_some_and(|(_, p)| Rect::from_corners(p, p + settings.menu_size()).contains(cursor));
    let blocked = hit.is_some() || menu_hit || settings.drag.is_some() || settings.captured;
    if blocked {
        hover.0 = Some(TOKEN);
        pointer.0 = true;
        panel.0 = true;
        consumed.0 = true;
    }
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(index) = hit {
            settings.close_menu();
            settings.menu = Some((index, cursor.min((canvas - MENU).max(Vec2::ZERO))));
            settings.captured = true;
        } else if !menu_hit {
            settings.close_menu();
        }
    }
    if mouse.just_pressed(MouseButton::Left) {
        if let Some((index, pos)) = settings.menu.filter(|_| menu_hit) {
            let row = ((cursor.y - pos.y - 9.) / ROW).floor() as i32;
            if settings.draft.is_some() {
                match row {
                    1..=3 => settings.items[index].design = (row - 1) as u8,
                    4 => {
                        settings.draft = None;
                        settings.menu = None;
                        settings.save();
                    }
                    5 => settings.close_menu(),
                    _ => {}
                }
                settings.captured = true;
                return;
            }
            let p = &mut settings.items[index];
            match row {
                1 => p.locked = !p.locked,
                2 => p.scale = (p.scale - 0.05).max(0.65),
                3 => p.scale = (p.scale + 0.05).min(1.6),
                4 => p.health = !p.health,
                5 => p.level = !p.level,
                6 => settings.items[index] = defaults(index),
                7 => {
                    for p in &mut settings.items {
                        p.locked = false;
                    }
                }
                8 => {
                    for p in &mut settings.items {
                        p.locked = true;
                    }
                }
                9 => settings.draft = Some((index, settings.items[index].design)),
                10 => settings.close_menu(),
                _ => {}
            }
            if matches!(row, 1 | 6 | 7 | 8) {
                settings.menu = None;
            }
            settings.captured = true;
            if settings.draft.is_none() {
                settings.save();
            }
        } else {
            settings.close_menu();
            if let Some(index) = hit.filter(|&i| !settings.items[i].locked) {
                settings.drag = Some((index, cursor - settings.items[index].pos));
                settings.captured = true;
            }
        }
    }
    if let Some((index, offset)) = settings.drag {
        if mouse.pressed(MouseButton::Left) {
            settings.items[index].pos = ((cursor - offset) / 2.).round() * 2.;
            settings.items[index].clamp(canvas);
        } else {
            settings.drag = None;
            settings.save();
        }
    }
    if !mouse.pressed(MouseButton::Left) && !mouse.pressed(MouseButton::Right) {
        settings.captured = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        settings.close_menu();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_keeps_independent_positions_and_switches() {
        let mut settings = FrameSettings::default();
        settings.items[1] = Placement {
            pos: Vec2::new(500., 250.),
            scale: 1.35,
            locked: false,
            health: false,
            design: 2,
            level: true,
        };
        assert_eq!(
            FrameSettings::decode(&settings.encode()).items,
            settings.items
        );
    }
    #[test]
    fn invalid_settings_and_resize_keep_frames_reachable() {
        let mut settings = FrameSettings::decode("NaN inf -5 0 1 1\nbroken");
        settings.items[0].clamp(Vec2::new(800., 600.));
        assert!(settings.items[0].pos.is_finite());
        assert_eq!(settings.items[0].scale, 0.65);
        assert_eq!(settings.items[1], defaults(1));
        settings.items[0].pos = Vec2::splat(9000.);
        settings.items[0].clamp(Vec2::new(800., 600.));
        assert!(settings.items[0]
            .bounds()
            .max
            .cmple(Vec2::new(800., 600.))
            .all());
    }
    #[test]
    fn right_click_unlock_drag_lock_and_scale_use_the_live_input_path() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(FrameSettings::default());
        world.insert_resource(State::new(Phase::Playing));
        world.insert_resource(UiScale(1.));
        world.insert_resource(ButtonInput::<MouseButton>::default());
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(crate::target::Selection::default());
        world.insert_resource(crate::ui_script::PlayerUiHover::default());
        world.insert_resource(crate::ui_script::PointerOverUi::default());
        world.insert_resource(crate::ui_script::PointerOverUiPanel::default());
        world.insert_resource(crate::ui_script::PlayerUiClickConsumed::default());
        let mut window = Window::default();
        window.resolution.set(1200., 768.);
        let window = world.spawn((window, PrimaryWindow)).id();
        let step = |world: &mut World, pos: Vec2, button: Option<MouseButton>| {
            world
                .get_mut::<Window>(window)
                .unwrap()
                .set_cursor_position(Some(pos));
            world.resource_mut::<ButtonInput<MouseButton>>().reset_all();
            if let Some(button) = button {
                world
                    .resource_mut::<ButtonInput<MouseButton>>()
                    .press(button);
            }
            world.run_system_once(input).unwrap();
        };
        step(&mut world, Vec2::new(50., 40.), Some(MouseButton::Right));
        assert_eq!(world.resource::<FrameSettings>().menu.unwrap().0, 0);
        assert!(world.resource::<crate::ui_script::PointerOverUi>().0);
        step(&mut world, Vec2::new(70., 90.), Some(MouseButton::Left));
        assert!(!world.resource::<FrameSettings>().items[0].locked);
        step(&mut world, Vec2::new(50., 40.), None);
        step(&mut world, Vec2::new(50., 40.), Some(MouseButton::Left));
        // Keep the same press held while the pointer moves.
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(350., 240.)));
        world.resource_mut::<ButtonInput<MouseButton>>().clear();
        world.run_system_once(input).unwrap();
        assert_eq!(
            world.resource::<FrameSettings>().items[0].pos,
            Vec2::new(316., 216.)
        );
        step(&mut world, Vec2::new(350., 240.), None);
        step(&mut world, Vec2::new(350., 240.), Some(MouseButton::Right));
        step(&mut world, Vec2::new(370., 290.), Some(MouseButton::Left));
        assert!(world.resource::<FrameSettings>().items[0].locked);
        step(&mut world, Vec2::new(350., 240.), None);
        step(&mut world, Vec2::new(350., 240.), Some(MouseButton::Left));
        assert!(world.resource::<FrameSettings>().drag.is_none());
        assert_eq!(world.resource::<FrameSettings>().items[1], defaults(1));
        step(&mut world, Vec2::new(350., 240.), Some(MouseButton::Right));
        step(&mut world, Vec2::new(370., 347.), Some(MouseButton::Left));
        assert!((world.resource::<FrameSettings>().items[0].scale - 1.05).abs() < 0.001);
        // Open Design, preview Silver, then cancel without altering the previous design.
        step(&mut world, Vec2::new(370., 510.), Some(MouseButton::Left));
        assert!(world.resource::<FrameSettings>().draft.is_some());
        step(&mut world, Vec2::new(370., 310.), Some(MouseButton::Left));
        assert_eq!(world.resource::<FrameSettings>().items[0].design, 1);
        step(&mut world, Vec2::new(370., 395.), Some(MouseButton::Left));
        assert_eq!(world.resource::<FrameSettings>().items[0].design, 0);
        step(&mut world, Vec2::new(350., 240.), Some(MouseButton::Right));
        step(&mut world, Vec2::new(370., 510.), Some(MouseButton::Left));
        step(&mut world, Vec2::new(370., 340.), Some(MouseButton::Left));
        assert_eq!(world.resource::<FrameSettings>().items[0].design, 2);
        step(&mut world, Vec2::new(370., 367.), Some(MouseButton::Left));
        assert!(world.resource::<FrameSettings>().draft.is_none());
        assert_eq!(world.resource::<FrameSettings>().items[0].design, 2);
        assert_eq!(
            FrameSettings::decode(&world.resource::<FrameSettings>().encode()).items[0].design,
            2
        );
    }
    #[test]
    fn old_settings_migrate_and_cancel_restores_design() {
        let mut settings = FrameSettings::decode("100 200 1.2 1 0 1\n400 200 1 1 1 1 99");
        assert_eq!(settings.items[0].pos, Vec2::new(100., 200.));
        assert_eq!(settings.items[0].design, 0);
        assert_eq!(settings.items[1].design, 0);
        settings.draft = Some((0, 0));
        settings.items[0].design = 2;
        settings.close_menu();
        assert_eq!(settings.items[0].design, 0);
    }
}
