//! Moving the HUD: right-click a panel to unlock it, drag it on an 8px grid that shows while it
//! moves, and lock it again from the same menu. Whatever is clicked comes to the front and stays
//! there. Moved panels keep their place in a local file; the unit frames keep their own richer
//! menu ([`super::frame_settings`]) and share only the stacking. Windows over the HUD (the Test
//! Grounds) move the same way but keep their own place in the stacking. The menus (pause and
//! settings) draw over everything and need no unlocking: drag them by anything but a button.
use super::{hud::HudFont, right_column as col, Phase};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::BTreeMap;

/// The snap step, and the grid's drawn lines every so many steps.
const GRID: f32 = 8.;
const LINE_EVERY: f32 = 32.;
/// The HUD root draws at 400 and the reward cards from 410: raised panels stack between.
const FRONT_BASE: i32 = 401;
const FRONT_MAX: i32 = 409;
/// The pause and settings menus, over every window: Test Grounds (900), talents (1000), co-op
/// (1200). Only the arranging overlay, its menu and the tooltips draw over them.
pub(super) const MENU_Z: i32 = 1500;
pub(super) const TOOLTIP_Z: i32 = MENU_Z + 30;
const MENU_W: f32 = 180.;
const MENU_ROW: f32 = 24.;
const MENU_ROWS: usize = 4;
const FILE: &str = "survivors-hud-layout.txt";

/// A panel the player can move: its saved name, the name its menu shows, and its layer.
#[derive(Component, Clone, Copy)]
pub(super) struct Movable {
    pub id: &'static str,
    pub name: &'static str,
    layer: Layer,
}

/// Where a movable sits: in the HUD, stacked by clicks; a window over the HUD; or one of the two
/// menus, each the one thing that moves while it is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    Hud,
    Window,
    Settings,
    Pause,
}

impl Layer {
    /// The menus are never locked.
    fn is_menu(self) -> bool {
        matches!(self, Self::Settings | Self::Pause)
    }
}

impl Movable {
    pub(super) const fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            layer: Layer::Hud,
        }
    }

    pub(super) const fn window(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            layer: Layer::Window,
        }
    }

    pub(super) const fn settings(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            layer: Layer::Settings,
        }
    }

    pub(super) const fn pause(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            layer: Layer::Pause,
        }
    }
}

/// A panel that only stacks: it comes to the front when clicked but moves by its own rules.
#[derive(Component, Clone, Copy)]
pub(super) struct Stacked(pub &'static str);

/// A moved panel's own seat, put back when its position is reset.
#[derive(Component, Clone)]
struct Home {
    position_type: PositionType,
    left: Val,
    top: Val,
    right: Val,
    bottom: Val,
    margin: UiRect,
}

#[derive(Resource, Default)]
pub(super) struct HudLayout {
    /// Each moved panel's top-left, in UI units.
    moved: BTreeMap<String, Vec2>,
    /// The unlocked panels; every panel starts a session locked.
    unlocked: Vec<&'static str>,
    /// Panels by when they were last clicked, the front one last.
    front: Vec<&'static str>,
    /// The panel whose menu is open, the menu's top-left and whether the panel is always free.
    menu: Option<(&'static str, Vec2, bool)>,
    /// The dragged panel and the cursor's offset from its top-left.
    drag: Option<(&'static str, Vec2)>,
    captured: bool,
}

impl HudLayout {
    /// Bring a panel to the front.
    pub(super) fn raise(&mut self, id: &'static str) {
        if self.front.last() != Some(&id) {
            self.front.retain(|f| *f != id);
            self.front.push(id);
        }
    }

    /// Whether a panel is the front one.
    #[cfg(test)]
    pub(super) fn is_front(&self, id: &str) -> bool {
        self.front.last() == Some(&id)
    }

    fn unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(&id)
    }

    /// Whether a panel moves right now.
    fn free(&self, m: &Movable) -> bool {
        m.layer.is_menu() || self.unlocked(m.id)
    }

    fn decode(text: &str) -> Self {
        let mut layout = Self::default();
        for line in text.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if let [id, x, y] = fields[..] {
                if let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) {
                    if x.is_finite() && y.is_finite() {
                        layout.moved.insert(id.to_string(), Vec2::new(x, y));
                    }
                }
            }
        }
        layout
    }

    fn encode(&self) -> String {
        self.moved
            .iter()
            .map(|(id, p)| format!("{id} {} {}\n", p.x, p.y))
            .collect()
    }

    fn save(&self) {
        if let Some(path) = crate::local_state::home().map(|h| h.join(FILE)) {
            if let Err(error) = crate::local_state::write_atomic(&path, &self.encode()) {
                warn!("Could not save the HUD layout: {error}");
            }
        }
    }
}

/// A top-left on the grid within the canvas; within a step of centring, centred.
fn snap(v: f32, size: f32, span: f32) -> f32 {
    let mid = (span - size) / 2.;
    let at = if (v - mid).abs() < GRID {
        mid.round()
    } else {
        (v / GRID).round() * GRID
    };
    at.clamp(0., (span - size).max(0.))
}

/// The right-click menu's size: an always-free panel has only its name and the reset.
fn menu_room(free: bool) -> Vec2 {
    let rows = if free { 2 } else { MENU_ROWS };
    Vec2::new(MENU_W, MENU_ROW * rows as f32 + 8.)
}

pub(super) fn plugin(app: &mut App) {
    let layout = crate::local_state::home()
        .and_then(|h| crate::local_state::read_to_string(&h.join(FILE)).ok())
        .map_or_else(HudLayout::default, |s| HudLayout::decode(&s));
    app.insert_resource(layout)
        .add_systems(
            Update,
            input
                .after(super::frame_settings::input)
                .after(crate::ui_script::UiInput)
                .before(benilla_world::schedule::WorldStage::Input),
        )
        .add_systems(Update, (spawn, paint).chain())
        .add_systems(PostUpdate, apply.before(bevy::ui::UiSystems::Layout));
}

/// A panel's boxes in UI units: as drawn (its transform's scale and shift in), and as laid out.
struct Boxes {
    drawn: Rect,
    laid: Rect,
}

fn boxes(node: &ComputedNode, at: &UiGlobalTransform, shift: Option<&UiTransform>) -> Boxes {
    let inv = node.inverse_scale_factor();
    let size = node.size() * inv;
    let center = at.translation * inv;
    let scale = at.matrix2.x_axis.length();
    // The drawn centre carries the transform's own shift; the laid-out box is without it.
    let moved = shift.map_or(Vec2::ZERO, |t| {
        let px = |v: Val| if let Val::Px(p) = v { p } else { 0. };
        Vec2::new(px(t.translation.x), px(t.translation.y))
    });
    Boxes {
        drawn: Rect::from_center_size(center, size * scale),
        laid: Rect::from_center_size(center - moved, size),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn input(
    mut layout: ResMut<HudLayout>,
    frames: Res<super::frame_settings::FrameSettings>,
    options: Res<super::options::Options>,
    phase: Res<State<Phase>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mouse: Res<ButtonInput<MouseButton>>,
    panels: Query<(
        &Movable,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
        Option<&UiTransform>,
        Entity,
    )>,
    kids: Query<&Children>,
    buttons: Query<&Interaction, With<Button>>,
    mut hover: ResMut<crate::ui_script::PlayerUiHover>,
    mut pointer: ResMut<crate::ui_script::PointerOverUi>,
    mut panel: ResMut<crate::ui_script::PointerOverUiPanel>,
    mut consumed: ResMut<crate::ui_script::PlayerUiClickConsumed>,
) {
    const TOKEN: u32 = u32::MAX - 65;
    if hover.0 == Some(TOKEN) {
        hover.0 = None;
        pointer.0 = false;
        panel.0 = false;
    }
    // An open menu is the only thing that moves, on the front end too; otherwise the HUD and its
    // windows in a run, but not under the reward cards, nor while a unit frame has the pointer.
    let settings = options.open && options.settings;
    let paused =
        !settings && *phase.get() != Phase::Menu && (options.open || *phase.get() == Phase::Paused);
    let off = !settings
        && !paused
        && matches!(phase.get(), Phase::Menu | Phase::LevelUp | Phase::GameOver);
    if off || frames.busy() {
        if layout.drag.take().is_some() {
            layout.save();
        }
        layout.menu = None;
        layout.captured = false;
        return;
    }
    let Ok(window) = windows.single() else { return };
    let scale = ui_scale.0.max(0.01);
    let canvas = Vec2::new(window.width(), window.height()) / scale;
    let Some(cursor) = window.cursor_position().map(|p| p / scale) else {
        if layout.drag.take().is_some() {
            layout.save();
        }
        return;
    };
    // The front-most panel under the cursor.
    let rank = |layout: &HudLayout, id: &str| {
        layout
            .front
            .iter()
            .position(|f| *f == id)
            .map_or(-1, |i| i as i32)
    };
    let live = |m: &Movable| match m.layer {
        Layer::Settings => settings,
        Layer::Pause => paused,
        Layer::Hud | Layer::Window => !settings && !paused,
    };
    let hit = panels
        .iter()
        .filter(|(m, node, _, shown, ..)| live(m) && shown.get() && node.size().x > 0.)
        .map(|(m, node, at, _, shift, entity)| (*m, boxes(node, at, shift), entity))
        .filter(|(_, b, _)| b.drawn.contains(cursor))
        .max_by_key(|(m, ..)| rank(&layout, m.id));
    let menu_rect = layout
        .menu
        .map(|(_, p, free)| Rect::from_corners(p, p + menu_room(free)));
    let menu_hit = menu_rect.is_some_and(|r| r.contains(cursor));
    // A locked panel lets clicks through to the world, except to raise it or open its menu.
    let over_free = hit.as_ref().is_some_and(|(m, ..)| layout.free(m));
    if over_free || menu_hit || layout.drag.is_some() || layout.captured {
        hover.0 = Some(TOKEN);
        pointer.0 = true;
        panel.0 = true;
        consumed.0 = true;
    }

    if mouse.just_pressed(MouseButton::Right) {
        if let Some((m, ..)) = &hit {
            let id = m.id;
            layout.raise(id);
            let free = m.layer.is_menu();
            let room = menu_room(free);
            layout.menu = Some((id, cursor.min((canvas - room).max(Vec2::ZERO)), free));
            layout.captured = true;
            consumed.0 = true;
        } else if !menu_hit {
            layout.menu = None;
        }
    }
    if mouse.just_pressed(MouseButton::Left) {
        if let Some((id, pos, free)) = layout.menu.filter(|_| menu_hit) {
            let row = ((cursor.y - pos.y - 4.) / MENU_ROW).floor() as i32;
            // An always-free panel's one row is the reset.
            let row = if free && row == 1 { 2 } else { row };
            match row {
                1 => {
                    if layout.unlocked(id) {
                        layout.unlocked.retain(|u| *u != id);
                    } else {
                        layout.unlocked.push(id);
                    }
                }
                2 => {
                    layout.moved.remove(id);
                    layout.save();
                }
                3 => {
                    if layout.unlocked.is_empty() {
                        layout.unlocked = panels
                            .iter()
                            .filter(|(m, ..)| live(m))
                            .map(|(m, ..)| m.id)
                            .collect();
                    } else {
                        layout.unlocked.clear();
                    }
                }
                _ => {}
            }
            if row >= 1 {
                layout.menu = None;
            }
            layout.captured = true;
        } else {
            layout.menu = None;
            if let Some((m, b, entity)) = &hit {
                layout.raise(m.id);
                // A menu's buttons stay buttons; the rest of it is the handle.
                let on_button = m.layer.is_menu()
                    && kids
                        .iter_descendants(*entity)
                        .any(|e| buttons.get(e).is_ok_and(|i| *i != Interaction::None));
                if layout.free(m) && !on_button {
                    layout.drag = Some((m.id, cursor - b.laid.min));
                    layout.captured = true;
                }
            }
        }
    }
    if let Some((id, offset)) = layout.drag {
        if mouse.pressed(MouseButton::Left) {
            let size = panels
                .iter()
                .find(|(m, ..)| m.id == id)
                .map_or(Vec2::ZERO, |(_, node, at, _, shift, _)| {
                    boxes(node, at, shift).laid.size()
                });
            let want = cursor - offset;
            let at = Vec2::new(
                snap(want.x, size.x, canvas.x),
                snap(want.y, size.y, canvas.y),
            );
            layout.moved.insert(id.to_string(), at);
        } else {
            layout.drag = None;
            layout.save();
        }
    }
    if !mouse.pressed(MouseButton::Left) && !mouse.pressed(MouseButton::Right) {
        layout.captured = false;
    }
}

/// Seat every moved panel at its place within its parent, put reset ones back, and stack the
/// clicked ones in front.
#[allow(clippy::type_complexity)]
fn apply(
    mut commands: Commands,
    layout: Res<HudLayout>,
    mut panels: Query<(Entity, &Movable, &mut Node, Option<&Home>, Option<&ChildOf>)>,
    stacked: Query<(Entity, &Stacked, Option<&GlobalZIndex>)>,
    zs: Query<&GlobalZIndex, With<Movable>>,
    parents: Query<(&ComputedNode, &UiGlobalTransform)>,
) {
    let front = |id: &str| {
        layout
            .front
            .iter()
            .position(|f| *f == id)
            .map(|i| (FRONT_BASE + i as i32).min(FRONT_MAX))
    };
    for (entity, movable, mut node, home, parent) in &mut panels {
        if let Some(z) = front(movable.id).filter(|_| movable.layer == Layer::Hud) {
            if zs.get(entity).map_or(true, |g| g.0 != z) {
                commands.entity(entity).insert(GlobalZIndex(z));
            }
        }
        let Some(&at) = layout.moved.get(movable.id) else {
            if let Some(home) = home {
                node.position_type = home.position_type;
                node.left = home.left;
                node.top = home.top;
                node.right = home.right;
                node.bottom = home.bottom;
                node.margin = home.margin;
                commands.entity(entity).remove::<Home>();
            }
            continue;
        };
        if home.is_none() {
            commands.entity(entity).insert(Home {
                position_type: node.position_type,
                left: node.left,
                top: node.top,
                right: node.right,
                bottom: node.bottom,
                margin: node.margin,
            });
        }
        // A top-level window is placed on the screen itself.
        let origin = match parent.map(|p| parents.get(p.parent())) {
            None => Vec2::ZERO,
            Some(Ok((pnode, pat))) => {
                let inv = pnode.inverse_scale_factor();
                pat.translation * inv - pnode.size() * inv / 2.
            }
            Some(Err(_)) => continue,
        };
        let (left, top) = (
            Val::Px((at.x - origin.x).round()),
            Val::Px((at.y - origin.y).round()),
        );
        if node.position_type != PositionType::Absolute {
            node.position_type = PositionType::Absolute;
        }
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
        if node.right != Val::Auto || node.bottom != Val::Auto || node.margin != UiRect::ZERO {
            node.right = Val::Auto;
            node.bottom = Val::Auto;
            node.margin = UiRect::ZERO;
        }
    }
    for (entity, stacked, z) in &stacked {
        if let Some(want) = front(stacked.0) {
            if z.map_or(true, |g| g.0 != want) {
                commands.entity(entity).insert(GlobalZIndex(want));
            }
        }
    }
}

// --- What the player sees while arranging ------------------------------------------------------

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct GridLines;
#[derive(Component)]
struct Hint;
#[derive(Component)]
struct Menu;
#[derive(Component)]
struct MenuLine(usize);
/// An unlocked panel's edge and name tag.
#[derive(Component)]
struct Edge(&'static str);
#[derive(Component)]
struct EdgeTag;

/// The overlay and the menu, once: top-level, so they show over the front end's settings too.
fn spawn(mut commands: Commands, font: Option<Res<HudFont>>, spawned: Query<(), With<Menu>>) {
    let Some(font) = font else { return };
    if !spawned.is_empty() {
        return;
    }
    let f = &font.0;
    let hud = &mut commands;
    hud.spawn((
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        Pickable::IGNORE,
        GlobalZIndex(MENU_Z + 10),
    ))
    .with_children(|o| {
        // The grid: a line every 32px and the centre lines in crimson, shown while dragging.
        o.spawn((
            GridLines,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|g| {
            let line =
                |g: &mut ChildSpawnerCommands, across: bool, at: Val, color: Color, thick: f32| {
                    g.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: if across { Val::Px(0.) } else { at },
                            top: if across { at } else { Val::Px(0.) },
                            width: if across {
                                Val::Percent(100.)
                            } else {
                                Val::Px(thick)
                            },
                            height: if across {
                                Val::Px(thick)
                            } else {
                                Val::Percent(100.)
                            },
                            ..default()
                        },
                        BackgroundColor(color),
                    ));
                };
            let faint = Color::srgba(1., 1., 1., 0.08);
            for i in 1..=(4096. / LINE_EVERY) as i32 {
                line(g, false, Val::Px(i as f32 * LINE_EVERY), faint, 1.);
            }
            for i in 1..=(2304. / LINE_EVERY) as i32 {
                line(g, true, Val::Px(i as f32 * LINE_EVERY), faint, 1.);
            }
            let centre = Color::srgba(0.72, 0.23, 0.31, 0.55);
            line(g, false, Val::Percent(50.), centre, 2.);
            line(g, true, Val::Percent(50.), centre, 2.);
        });
        o.spawn((
            Hint,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(120.),
                left: Val::Percent(50.),
                margin: UiRect::left(Val::Px(-120.)),
                width: Val::Px(240.),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(10.), Val::Px(4.)),
                border: UiRect::all(Val::Px(1.)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.027, 0.035, 0.047, 0.9)),
            BorderColor::all(col::STEEL_LO),
            Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
        ))
        .with_child(col::label(
            f,
            "Drag to move · right-click to lock",
            11.,
            col::STEEL_HI,
        ));
    });
    hud.spawn((
        Menu,
        col::plate(Node {
            position_type: PositionType::Absolute,
            width: Val::Px(MENU_W),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(4.)),
            display: Display::None,
            ..default()
        }),
        bevy::ui::FocusPolicy::Block,
        GlobalZIndex(MENU_Z + 20),
    ))
    .with_children(|menu| {
        for line in 0..MENU_ROWS {
            menu.spawn((
                Node {
                    height: Val::Px(MENU_ROW),
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(8.)),
                    border: if line == 0 {
                        UiRect::bottom(Val::Px(1.))
                    } else {
                        UiRect::left(Val::Px(2.))
                    },
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(if line == 0 {
                    col::STEEL_LO
                } else {
                    Color::NONE
                }),
                MenuLine(line),
            ))
            .with_child(col::label(
                f,
                "",
                12.,
                if line == 0 {
                    col::STEEL_HI
                } else {
                    col::TEXT_HI
                },
            ));
        }
    });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn paint(
    mut commands: Commands,
    layout: Res<HudLayout>,
    font: Option<Res<HudFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    panels: Query<(
        &Movable,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
        Option<&UiTransform>,
    )>,
    overlays: Query<Entity, With<Overlay>>,
    mut nodes: Query<
        (&mut Node, Has<GridLines>, Has<Hint>),
        (
            Or<(With<GridLines>, With<Hint>, With<Menu>)>,
            Without<MenuLine>,
        ),
    >,
    mut lines: Query<
        (
            &MenuLine,
            &Children,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
        ),
        (
            Without<Edge>,
            Without<GridLines>,
            Without<Hint>,
            Without<Menu>,
        ),
    >,
    mut texts: Query<&mut Text>,
    mut edges: Query<
        (Entity, &Edge, &mut Node, &mut BorderColor, &Children),
        (Without<GridLines>, Without<Hint>, Without<Menu>),
    >,
    mut tags: Query<
        (&mut BackgroundColor, &mut BorderColor),
        (With<EdgeTag>, Without<Edge>, Without<MenuLine>),
    >,
) {
    let any = !layout.unlocked.is_empty();
    for (mut node, grid, hint) in &mut nodes {
        let on = if grid {
            layout.drag.is_some()
        } else if hint {
            any
        } else {
            if let Some((_, at, _)) = layout.menu {
                node.left = Val::Px(at.x);
                node.top = Val::Px(at.y);
            }
            layout.menu.is_some()
        };
        let want = if on { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }

    // The menu's words, and the row under the cursor in crimson.
    if let Some((id, at, free)) = layout.menu {
        let name = panels
            .iter()
            .find(|(m, ..)| m.id == id)
            .map_or("", |(m, ..)| m.name);
        let words = if free {
            [name, "Reset position", "", ""]
        } else {
            [
                name,
                if layout.unlocked(id) {
                    "Lock"
                } else {
                    "Unlock — drag to move"
                },
                "Reset position",
                if any { "Lock all" } else { "Unlock all" },
            ]
        };
        let cursor = windows
            .single()
            .ok()
            .and_then(Window::cursor_position)
            .map(|p| p / ui_scale.0.max(0.01));
        for (line, children, mut bg, mut edge, mut node) in &mut lines {
            let want = if free && line.0 > 1 {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != want {
                node.display = want;
            }
            let top = at.y + 4. + MENU_ROW * line.0 as f32;
            let hot = line.0 > 0
                && cursor.is_some_and(|c| {
                    Rect::new(at.x, top, at.x + MENU_W, top + MENU_ROW).contains(c)
                });
            bg.0 = if hot {
                col::WINE.with_alpha(0.35)
            } else {
                Color::NONE
            };
            if line.0 > 0 {
                *edge = BorderColor::all(if hot { col::WINE_HI } else { Color::NONE });
            }
            if let Some(mut text) = children.first().and_then(|c| texts.get_mut(*c).ok()) {
                if text.0 != words[line.0] {
                    text.0 = words[line.0].to_string();
                }
            }
        }
    }

    // Each unlocked panel's edge: steel, crimson while it moves, with its name over it.
    let (Some(font), Ok(overlay)) = (font, overlays.single()) else {
        return;
    };
    let mut seen = Vec::new();
    for (entity, edge, mut node, mut border, children) in &mut edges {
        let Some((m, cn, at, shown, shift)) = panels.iter().find(|(m, ..)| m.id == edge.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        if !layout.unlocked(m.id) || !shown.get() {
            commands.entity(entity).despawn();
            continue;
        }
        seen.push(m.id);
        let r = boxes(cn, at, shift).drawn;
        node.left = Val::Px(r.min.x - 4.);
        node.top = Val::Px(r.min.y - 4.);
        node.width = Val::Px(r.width() + 8.);
        node.height = Val::Px(r.height() + 8.);
        let moving = layout.drag.is_some_and(|(d, _)| d == m.id);
        let tone = if moving { col::WINE_HI } else { col::STEEL };
        *border = BorderColor::all(tone);
        if let Some(Ok((mut bg, mut tag_edge))) = children.first().map(|c| tags.get_mut(*c)) {
            bg.0 = if moving {
                col::WINE.with_alpha(0.6)
            } else {
                Color::srgba(0.027, 0.035, 0.047, 0.92)
            };
            *tag_edge = BorderColor::all(tone);
        }
    }
    for (m, _, _, shown, _) in &panels {
        if layout.unlocked(m.id) && shown.get() && !seen.contains(&m.id) {
            commands.entity(overlay).with_children(|o| {
                o.spawn((
                    Edge(m.id),
                    Node {
                        position_type: PositionType::Absolute,
                        border: UiRect::all(Val::Px(1.)),
                        ..default()
                    },
                    BorderColor::all(col::STEEL),
                ))
                .with_children(|e| {
                    e.spawn((
                        EdgeTag,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-1.),
                            bottom: Val::Percent(100.),
                            margin: UiRect::bottom(Val::Px(3.)),
                            padding: UiRect::axes(Val::Px(6.), Val::Px(1.)),
                            border: UiRect::all(Val::Px(1.)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.027, 0.035, 0.047, 0.92)),
                        BorderColor::all(col::STEEL),
                    ))
                    .with_child(col::label(
                        &font.0,
                        m.name,
                        10.,
                        col::STEEL_HI,
                    ));
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_keeps_to_the_grid_the_canvas_and_the_centre() {
        assert_eq!(snap(13., 100., 1365.), 16.);
        assert_eq!(snap(-40., 100., 1365.), 0.);
        assert_eq!(snap(1400., 100., 1365.), 1265.);
        // Within a step of centred, centred: (1365 - 100) / 2 = 632.5.
        assert_eq!(snap(628., 100., 1365.), 633.);
        assert_eq!(snap(600., 100., 1365.), 600.);
    }

    #[test]
    fn the_layout_file_roundtrips_and_skips_bad_lines() {
        let mut layout = HudLayout::default();
        layout.moved.insert("minimap".into(), Vec2::new(24., 640.));
        layout
            .moved
            .insert("action_bar".into(), Vec2::new(480., 696.));
        let back = HudLayout::decode(&(layout.encode() + "broken line\nmeter x 3\n"));
        assert_eq!(back.moved, layout.moved);
    }

    #[test]
    fn raising_moves_a_panel_to_the_front_once() {
        let mut layout = HudLayout::default();
        layout.raise("minimap");
        layout.raise("meter");
        layout.raise("minimap");
        assert_eq!(layout.front, ["meter", "minimap"]);
    }

    /// One left press through [`input`] on a screen with this panel under the cursor, and what
    /// it starts dragging. `open` is the menu state: `Some(true)` for the settings.
    fn press_on(
        phase: Phase,
        open: Option<bool>,
        panel: Movable,
        button: Option<Interaction>,
    ) -> Option<&'static str> {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let mut options = super::super::options::Options::default();
        if let Some(settings) = open {
            options.open = true;
            options.settings = settings;
        }
        world.insert_resource(options);
        world.insert_resource(HudLayout::default());
        world.insert_resource(super::super::frame_settings::FrameSettings::default());
        world.insert_resource(State::new(phase));
        world.insert_resource(UiScale(1.));
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        world.insert_resource(mouse);
        world.insert_resource(crate::ui_script::PlayerUiHover::default());
        world.insert_resource(crate::ui_script::PointerOverUi::default());
        world.insert_resource(crate::ui_script::PointerOverUiPanel::default());
        world.insert_resource(crate::ui_script::PlayerUiClickConsumed::default());
        let mut window = Window::default();
        window.resolution.set(1200., 768.);
        window.set_cursor_position(Some(Vec2::new(100., 100.)));
        world.spawn((window, PrimaryWindow));
        let entity = world
            .spawn((
                panel,
                ComputedNode {
                    size: Vec2::new(200., 200.),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::new(100., 100.)),
                InheritedVisibility::VISIBLE,
            ))
            .id();
        if let Some(interaction) = button {
            world.spawn((Button, interaction, ChildOf(entity)));
        }
        world.run_system_once(input).unwrap();
        world.resource::<HudLayout>().drag.map(|(id, _)| id)
    }

    #[test]
    fn the_menus_drag_without_unlocking_and_their_buttons_stay_buttons() {
        let settings = Movable::settings("settings", "Settings");
        let pause = Movable::pause("pause", "Pause menu");
        assert_eq!(
            press_on(Phase::Menu, Some(true), settings, None),
            Some("settings"),
            "the front end's settings"
        );
        assert_eq!(
            press_on(Phase::Paused, None, pause, None),
            Some("pause"),
            "the pause menu"
        );
        assert_eq!(
            press_on(Phase::LevelUp, Some(false), pause, None),
            Some("pause"),
            "the pause menu over the reward cards"
        );
        assert_eq!(
            press_on(
                Phase::Paused,
                Some(true),
                settings,
                Some(Interaction::Pressed)
            ),
            None,
            "a press on one of its buttons is the button's"
        );
    }

    #[test]
    fn locked_panels_stay_put_and_only_the_open_menu_moves() {
        let hud = Movable::new("minimap", "Minimap");
        let window = Movable::window("test_grounds", "Test Grounds");
        let settings = Movable::settings("settings", "Settings");
        let pause = Movable::pause("pause", "Pause menu");
        assert_eq!(press_on(Phase::Playing, None, hud, None), None, "locked");
        assert_eq!(press_on(Phase::Playing, None, window, None), None, "locked");
        assert_eq!(
            press_on(Phase::Paused, None, hud, None),
            None,
            "under the pause menu"
        );
        assert_eq!(
            press_on(Phase::Paused, None, settings, None),
            None,
            "the settings are closed"
        );
        assert_eq!(
            press_on(Phase::Paused, Some(true), pause, None),
            None,
            "behind the settings"
        );
        assert_eq!(
            press_on(Phase::Playing, None, pause, None),
            None,
            "not paused"
        );
    }

    #[test]
    fn the_overlay_and_menu_spawn() {
        // A bundle with a component twice panics when it is applied.
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(HudFont(Handle::default()));
        world.run_system_once(spawn).unwrap();
        world.run_system_once(spawn).unwrap();
        assert_eq!(
            world.query::<&MenuLine>().iter(&world).count(),
            MENU_ROWS,
            "spawned once"
        );
        // Its queries share `Node`: overlapping access panics when the system first runs.
        let mut layout = HudLayout::default();
        layout.menu = Some(("settings", Vec2::ZERO, true));
        world.insert_resource(layout);
        world.insert_resource(UiScale(1.));
        world.run_system_once(paint).unwrap();
        world.flush();
        let shown = world
            .query::<(&MenuLine, &Node)>()
            .iter(&world)
            .filter(|(_, n)| n.display != Display::None)
            .count();
        assert_eq!(
            shown, 2,
            "an always-free panel's menu: its name and the reset"
        );
    }
}
