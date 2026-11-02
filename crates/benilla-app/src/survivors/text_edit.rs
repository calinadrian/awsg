//! Shared single-line editing for the mode's native fields. Indices are UTF-8 byte boundaries.
use bevy::{prelude::*, text::TextLayoutInfo};

#[derive(Clone, Debug, Default)]
pub(super) struct Cursor {
    pub at: usize,
    pub anchor: Option<usize>,
}
impl Cursor {
    fn clamp(&mut self, value: &str) {
        self.at = boundary(value, self.at);
        self.anchor = self
            .anchor
            .map(|i| boundary(value, i))
            .filter(|&i| i != self.at);
    }
    pub fn place(&mut self, value: &str, at: usize, extend: bool) {
        self.clamp(value);
        if extend {
            self.anchor.get_or_insert(self.at);
        } else {
            self.anchor = None;
        }
        self.at = boundary(value, at);
    }
    pub fn select_all(&mut self, value: &str) {
        self.anchor = Some(0);
        self.at = value.len();
    }
    pub fn selected<'a>(&self, value: &'a str) -> &'a str {
        let a = boundary(value, self.anchor.unwrap_or(self.at));
        let b = boundary(value, self.at);
        &value[a.min(b)..a.max(b)]
    }
    fn remove_selection(&mut self, value: &mut String) -> bool {
        self.clamp(value);
        let anchor = self.anchor.take().unwrap_or(self.at);
        if anchor == self.at {
            return false;
        }
        let start = anchor.min(self.at);
        value.replace_range(start..anchor.max(self.at), "");
        self.at = start;
        true
    }
    pub fn insert(&mut self, value: &mut String, text: &str, limit: usize) {
        if !text.chars().any(|c| !c.is_control()) {
            return;
        }
        self.remove_selection(value);
        let remaining = limit.saturating_sub(value.chars().count());
        let text: String = text
            .chars()
            .filter(|c| !c.is_control())
            .take(remaining)
            .collect();
        value.insert_str(self.at, &text);
        self.at += text.len();
    }
    pub fn key(&mut self, value: &mut String, key: KeyCode, shift: bool, control: bool) -> bool {
        self.clamp(value);
        let move_to = match key {
            KeyCode::KeyA if control => {
                self.select_all(value);
                return true;
            }
            KeyCode::Home => Some(0),
            KeyCode::End => Some(value.len()),
            KeyCode::ArrowLeft => Some(if !shift && self.anchor.is_some() {
                self.at.min(self.anchor.unwrap_or(self.at))
            } else if control {
                word_left(value, self.at)
            } else {
                previous(value, self.at)
            }),
            KeyCode::ArrowRight => Some(if !shift && self.anchor.is_some() {
                self.at.max(self.anchor.unwrap_or(self.at))
            } else if control {
                word_right(value, self.at)
            } else {
                next(value, self.at)
            }),
            KeyCode::Backspace | KeyCode::Delete => {
                if !self.remove_selection(value) {
                    let other = if key == KeyCode::Backspace {
                        if control {
                            word_left(value, self.at)
                        } else {
                            previous(value, self.at)
                        }
                    } else if control {
                        word_right(value, self.at)
                    } else {
                        next(value, self.at)
                    };
                    value.replace_range(self.at.min(other)..self.at.max(other), "");
                    self.at = self.at.min(other);
                }
                return true;
            }
            _ => None,
        };
        if let Some(at) = move_to {
            self.place(value, at, shift);
            true
        } else {
            false
        }
    }
}
fn boundary(value: &str, mut i: usize) -> usize {
    i = i.min(value.len());
    while !value.is_char_boundary(i) {
        i -= 1;
    }
    i
}
fn previous(value: &str, i: usize) -> usize {
    value[..i].char_indices().next_back().map_or(0, |(i, _)| i)
}
fn next(value: &str, i: usize) -> usize {
    value[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}
fn word_left(value: &str, mut i: usize) -> usize {
    while i > 0
        && value[..i]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        i = previous(value, i);
    }
    while i > 0
        && value[..i]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_whitespace())
    {
        i = previous(value, i);
    }
    i
}
fn word_right(value: &str, mut i: usize) -> usize {
    while i < value.len()
        && value[i..]
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace())
    {
        i = next(value, i);
    }
    while i < value.len() && value[i..].chars().next().is_some_and(char::is_whitespace) {
        i = next(value, i);
    }
    i
}
#[derive(Component)]
pub(super) struct Field {
    pub id: u8,
    cursor: Cursor,
    focused: bool,
}
#[derive(Component)]
struct Decoration(bool);
pub(super) type FieldQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Field,
        &'static TextLayoutInfo,
        &'static ComputedNode,
        &'static bevy::ui::UiGlobalTransform,
    ),
>;
pub(super) fn clicked(id: u8, value: &str, point: Option<Vec2>, fields: &FieldQuery) -> usize {
    let Some(point) = point else {
        return value.len();
    };
    let Some((_, layout, size, at)) = fields.iter().find(|(field, ..)| field.id == id) else {
        return value.len();
    };
    let x = point.x - (at.translation.x - size.size().x / 2.);
    let index = layout
        .glyphs
        .iter()
        .filter(|g| g.line_index == 0)
        .find(|g| x < g.position.x)
        .map_or(value.len(), |g| g.byte_index);
    boundary(value, index)
}
pub(super) fn mark(
    p: &mut ChildSpawnerCommands,
    entity: Entity,
    id: u8,
    cursor: &Cursor,
    focused: bool,
) {
    p.commands()
        .entity(entity)
        .insert((
            Field {
                id,
                cursor: cursor.clone(),
                focused,
            },
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ))
        .with_children(|p| {
            for selection in [true, false] {
                p.spawn((
                    Decoration(selection),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Visibility::Hidden,
                    BackgroundColor(if selection {
                        Color::srgba(0.8, 0.6, 0.2, 0.28)
                    } else {
                        Color::srgb(1., 0.85, 0.4)
                    }),
                    bevy::ui::FocusPolicy::Pass,
                ));
            }
        });
}
fn x_at(layout: &TextLayoutInfo, at: usize) -> f32 {
    layout
        .glyphs
        .iter()
        .find(|g| g.byte_index >= at)
        .map_or_else(
            || {
                layout
                    .glyphs
                    .last()
                    .map_or(0., |g| g.position.x + g.size.x / 2.)
            },
            |g| g.position.x - g.size.x / 2.,
        )
}
pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, paint.after(bevy::ui::UiSystems::Layout));
}
fn paint(
    mut fields: Query<
        (
            Entity,
            &Field,
            &TextLayoutInfo,
            &ComputedNode,
            &ChildOf,
            &mut Node,
        ),
        Without<Decoration>,
    >,
    parents: Query<&ComputedNode>,
    mut decorations: Query<(&ChildOf, &Decoration, &mut Node, &mut Visibility), Without<Field>>,
) {
    for (entity, field, layout, size, parent, mut node) in &mut fields {
        let inv = size.inverse_scale_factor();
        let x = x_at(layout, field.cursor.at) * inv;
        let available = parents
            .get(parent.parent())
            .map_or(0., |p| p.size().x * inv - 24.)
            .max(1.);
        let old = match node.left {
            Val::Px(x) => -x,
            _ => 0.,
        };
        let offset = if !field.focused {
            0.
        } else {
            old.min(x).max(x - available).max(0.)
        };
        node.left = Val::Px(-offset);
        for (parent, decoration, mut node, mut visibility) in &mut decorations {
            if parent.parent() != entity {
                continue;
            }
            let anchor = field.cursor.anchor.unwrap_or(field.cursor.at);
            let show = field.focused && (!decoration.0 || anchor != field.cursor.at);
            *visibility = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            let ax = x_at(layout, anchor) * inv;
            node.left = Val::Px(if decoration.0 { x.min(ax) } else { x });
            node.top = Val::Px(0.);
            node.height = Val::Px((size.size().y * inv).max(14.));
            node.width = Val::Px(if decoration.0 { (x - ax).abs() } else { 1. });
        }
    }
}
