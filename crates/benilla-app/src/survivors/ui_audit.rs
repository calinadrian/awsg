//! A layout audit for the survivors screens, on with `WOW_SURVIVORS_UI_AUDIT=1`: once the layout
//! settles it logs every visible text that runs out of its parent or off the window, and every
//! pair of sibling texts that overlap. Each finding is logged once, so a screen that is left open
//! does not repeat itself; a new screen's findings follow as it opens.
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::PrimaryWindow;
use std::collections::HashSet;

pub(super) fn plugin(app: &mut App) {
    if crate::webenv::var("WOW_SURVIVORS_UI_AUDIT").is_some_and(|v| v != "0") {
        app.add_systems(PostUpdate, audit.after(bevy::ui::UiSystems::Layout));
    }
}

/// Slack for sub-pixel rounding and anti-aliased edges, in physical pixels.
const TOLERANCE: f32 = 1.5;

fn rect(node: &ComputedNode, at: &UiGlobalTransform) -> Rect {
    Rect::from_center_size(at.translation, node.size())
}

/// The first 40 characters of a text, for the log.
fn quote(text: &Text) -> String {
    text.0.chars().take(40).collect::<String>().replace('\n', " / ")
}

#[allow(clippy::type_complexity)]
fn audit(
    windows: Query<&Window, With<PrimaryWindow>>,
    texts: Query<(Entity, &Text, &ComputedNode, &UiGlobalTransform, &InheritedVisibility, Option<&ChildOf>)>,
    nodes: Query<(&Node, &ComputedNode, &UiGlobalTransform)>,
    mut seen: Local<HashSet<String>>,
    mut frame: Local<u32>,
) {
    // Every half second or so: findings stay for as long as a screen is open.
    *frame += 1;
    if *frame % 30 != 0 {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let screen = Rect::new(0., 0., window.physical_width() as f32, window.physical_height() as f32);
    let mut found = Vec::new();
    let mut by_parent: std::collections::HashMap<Entity, Vec<(Entity, Rect, String)>> = default();
    for (entity, text, node, at, visible, parent) in &texts {
        if !visible.get() || text.0.trim().is_empty() || node.size().min_element() <= 0. {
            continue;
        }
        let r = rect(node, at);
        let what = quote(text);
        let inner = r.inflate(-TOLERANCE);
        if !screen.contains(inner.min) || !screen.contains(inner.max) {
            found.push(format!("off the window: \"{what}\" at {:.0?}", r));
        }
        if let Some(parent) = parent.map(ChildOf::parent) {
            if let Ok((style, pnode, pat)) = nodes.get(parent) {
                // A scroll area's children run past it by design.
                let scrolls = matches!(style.overflow.y, OverflowAxis::Scroll)
                    || matches!(style.overflow.x, OverflowAxis::Scroll);
                let p = rect(pnode, pat).inflate(TOLERANCE);
                if !scrolls && pnode.size().min_element() > 0. && (!p.contains(r.min) || !p.contains(r.max)) {
                    found.push(format!("out of its parent: \"{what}\" {:.0?} in {:.0?}", r, p));
                }
            }
            by_parent.entry(parent).or_default().push((entity, r, what));
        }
    }
    for siblings in by_parent.values() {
        for (i, (_, a, ta)) in siblings.iter().enumerate() {
            for (_, b, tb) in &siblings[i + 1..] {
                let overlap = a.intersect(*b);
                if overlap.width() > TOLERANCE && overlap.height() > TOLERANCE {
                    found.push(format!("overlapping: \"{ta}\" and \"{tb}\""));
                }
            }
        }
    }
    for finding in found {
        if seen.insert(finding.clone()) {
            warn!(target: "ui_audit", "{finding}");
        }
    }
}
