//! The run pick screen: compact race, gender and class buttons on the left,
//! zone buttons on the right, and Enter World at the bottom.
//!
//! The survivors HUD lays out on the 768-unit canvas through `UiScale`, so every glue builder here
//! takes a unit scale.

use bevy::prelude::*;

use super::front_end as skin;
use super::hud::{looks_for, rgb, start_run};
pub(super) use super::menu_layout::build_menu;
use super::races::{race_name, race_tc};
use super::{data, Phase, Rng, Run, Selection};
use crate::glue::art::{tc_rect, GlueArt, GOLD};
use crate::glue::widgets::{GlueDisabled, HoverLabel, LockHighlight};
use crate::glue::GlueClicks;

/// The shown race's bonuses on the pick plate, one per line.
#[derive(Component)]
pub(super) struct PickDescription;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MenuAction {
    Race(u8),
    Sex(u8),
    Class(usize),
    Map(usize),
    Randomize,
    Enter,
    /// Opens the talents screen ([`super::talents`]).
    Talents,
    /// Opens the settings, as Escape does.
    Settings,
    /// Steps FEATURES dial `.0` (`[skin, face, hair style, hair colour, facial hair]`) back or on.
    Feature(usize, bool),
    /// Rolls every FEATURES dial.
    RandomFeatures,
}

/// On [`MenuRoot`] once the glue art is in and the screen is built.
#[derive(Component)]
pub(super) struct MenuBuilt;

/// A race button's face, re-cropped to the picked gender.
#[derive(Component)]
pub(super) struct RaceFace(pub(super) u8);

#[derive(Component)]
pub(super) struct PickName;

#[derive(Component)]
pub(super) struct ClassSlot(pub usize);

/// The saved pick, `benilla-config/survivors-pick.txt` (`localStorage` on the page):
/// `class race sex map`, restored at the menu so a reload keeps the last hero and zone.
fn pick_path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-pick.txt"))
}

fn save_pick(sel: &Selection) {
    let (Some(class), Some(&(_, race, sex))) = (sel.class, sel.look.and_then(|i| sel.looks.get(i)))
    else {
        return;
    };
    if let Some(path) = pick_path() {
        let mut text = format!("{class} {race} {sex} {}", sel.map);
        let mut features: Vec<_> = sel.features.iter().collect();
        features.sort_unstable_by_key(|(k, _)| **k);
        for (&(c, r, s), f) in features {
            text += &format!("\n{c} {r} {s} {} {} {} {} {}", f[0], f[1], f[2], f[3], f[4]);
        }
        let _ = crate::local_state::write_atomic(&path, &text);
    }
}

/// Restore the saved pick once the creature data can resolve its looks; a stale or missing file
/// leaves the default.
pub(super) fn restore_pick(
    creatures: Option<Res<crate::entities::Creatures>>,
    mut selection: ResMut<Selection>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(creatures) = creatures.as_deref() else {
        return;
    };
    *done = true;
    let Some(text) = pick_path().and_then(|p| crate::local_state::read_to_string(&p).ok()) else {
        return;
    };
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    // Each later line is one hero's FEATURES: `class race sex skin face hair_style hair_color
    // facial_hair`.
    for line in lines {
        let f: Vec<u8> = line
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if let [class, race, sex, a, b, c, d, e] = f[..] {
            selection
                .features
                .insert((usize::from(class), race, sex), [a, b, c, d, e]);
        }
    }
    let v: Vec<usize> = first
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    let [class, race, sex, map] = v[..] else {
        return;
    };
    if data::MAPS.get(map).is_some_and(super::dungeon::available) {
        selection.map = map;
        selection.stage = map;
        selection.version += 1;
    }
    if class < data::CLASSES.len() {
        let all = all_looks(creatures);
        if let Some(l) = find(&all[class], Some(race as u8), Some(sex as u8)) {
            set(&mut selection, &all, class, l);
        }
    }
}

/// Every class's looks, `(display, race, sex)`, indexed like [`data::CLASSES`].
fn all_looks(creatures: &crate::entities::Creatures) -> Vec<Vec<(u32, u8, u8)>> {
    data::CLASSES
        .iter()
        .map(|c| looks_for(c, creatures))
        .collect()
}

fn find(looks: &[(u32, u8, u8)], race: Option<u8>, sex: Option<u8>) -> Option<usize> {
    looks
        .iter()
        .position(|&(_, r, s)| race.is_none_or(|x| x == r) && sex.is_none_or(|x| x == s))
}

fn set(sel: &mut Selection, all: &[Vec<(u32, u8, u8)>], class: usize, look: usize) {
    sel.class = Some(class);
    sel.looks = all[class].clone();
    sel.look = Some(look);
    sel.version += 1;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn menu_clicks(
    inventory: Res<super::inventory::Equipment>,
    coop: Res<super::coop::Coop>,
    mut options: ResMut<super::options::Options>,
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &MenuAction)>,
    phase: Res<State<Phase>>,
    creatures: Option<Res<crate::entities::Creatures>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    appearances: Option<Res<super::appearance::Appearances>>,
) {
    if inventory.open || coop.open || coop.in_room() || options.open || *phase.get() != Phase::Menu {
        return;
    }
    let Some(creatures) = creatures.as_deref() else {
        return;
    };
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    let all = all_looks(creatures);
    let cur = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, r, s)| (r, s));
    let (race, sex) = (cur.map(|c| c.0), cur.map_or(0, |c| c.1));
    match action {
        MenuAction::Map(i) => {
            if selection.map != i && data::MAPS.get(i).is_some_and(super::dungeon::available) {
                selection.map = i;
                selection.version += 1;
            }
        }
        MenuAction::Talents => {}
        MenuAction::Feature(dial, forward) => {
            if let Some((key, mut f, ranges)) =
                super::appearance::current(&selection, creatures, appearances.as_deref())
            {
                let opts = ranges.options(dial, &f);
                let at = opts.iter().position(|&v| v == f[dial]).unwrap_or(0);
                let n = opts.len();
                f[dial] = opts[if forward { (at + 1) % n } else { (at + n - 1) % n }];
                if dial == 0 {
                    ranges.fit_face(&mut f);
                }
                selection.features.insert(key, f);
                selection.version += 1;
            }
        }
        MenuAction::RandomFeatures => {
            if let Some((key, mut f, ranges)) =
                super::appearance::current(&selection, creatures, appearances.as_deref())
            {
                for dial in 0..super::appearance::DIALS {
                    let opts = ranges.options(dial, &f);
                    f[dial] = opts[rng.index(opts.len())];
                }
                ranges.fit_face(&mut f);
                selection.features.insert(key, f);
                selection.version += 1;
            }
        }
        // What Escape does on this screen.
        MenuAction::Settings => {
            options.open = true;
            options.settings = true;
        }
        MenuAction::Enter => {
            if start_run(&mut run, &mut selection) {
                next.set(Phase::Playing);
            }
        }
        MenuAction::Randomize => {
            let picks: Vec<(usize, usize)> = all
                .iter()
                .enumerate()
                .flat_map(|(c, looks)| (0..looks.len()).map(move |l| (c, l)))
                .collect();
            if !picks.is_empty() {
                let (c, l) = picks[rng.index(picks.len())];
                set(&mut selection, &all, c, l);
            }
        }
        // A class choice must keep the selected race.
        MenuAction::Class(c) => {
            let looks = &all[c];
            let l = find(looks, race, Some(sex))
                .or_else(|| find(looks, race, None));
            if let Some(l) = l {
                set(&mut selection, &all, c, l);
            }
        }
        // The race keeps the class where it can, else takes the first class it has.
        MenuAction::Race(r) => {
            let classes = selection.class.into_iter().chain(0..all.len());
            for c in classes {
                let l = find(&all[c], Some(r), Some(sex)).or_else(|| find(&all[c], Some(r), None));
                if let Some(l) = l {
                    set(&mut selection, &all, c, l);
                    break;
                }
            }
        }
        MenuAction::Sex(s) => {
            if let Some(c) = selection.class {
                if let Some(l) = find(&all[c], race, Some(s)) {
                    set(&mut selection, &all, c, l);
                }
            }
        }
    }
    save_pick(&selection);
}

/// The screen's per-frame state: the locked highlights, hover labels, which races and classes
/// the pick allows, the gender on the race faces, the name line and Enter World.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_menu_ui(
    selection: Res<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    art: Res<GlueArt>,
    mut buttons: Query<(
        &MenuAction,
        &Interaction,
        &Children,
        Option<&mut LockHighlight>,
        Option<&mut GlueDisabled>,
        &mut Visibility,
    )>,
    mut labels: Query<&mut Visibility, (With<HoverLabel>, Without<MenuAction>)>,
    mut faces: Query<(&RaceFace, &mut ImageNode)>,
    mut name: Query<(&mut Text, &mut TextColor), With<PickName>>,
    mut styles: Query<(&MenuAction, &mut skin::Control)>,
    mut class_slots: Query<(&ClassSlot, &mut Node)>,
    mut cache: Local<(Option<u32>, bool, Vec<Vec<(u32, u8, u8)>>)>,
) {
    let have = creatures.is_some();
    if cache.0 != Some(selection.version) || cache.1 != have {
        *cache = (
            Some(selection.version),
            have,
            creatures.as_deref().map(all_looks).unwrap_or_default(),
        );
    }
    let all = &cache.2;
    let cur = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, r, s)| (r, s));

    for (slot, mut node) in &mut class_slots {
        let available = cur.is_none_or(|(race, _)| {
            all.get(slot.0)
                .is_some_and(|looks| find(looks, Some(race), None).is_some())
        });
        node.display = if available {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (action, interaction, children, lock, disabled, mut vis) in &mut buttons {
        let selected = match *action {
            MenuAction::Race(r) => cur.is_some_and(|c| c.0 == r),
            MenuAction::Sex(s) => cur.is_some_and(|c| c.1 == s),
            MenuAction::Class(i) => selection.class == Some(i),
            MenuAction::Map(i) => selection.map == i,
            MenuAction::Randomize
            | MenuAction::Feature(..)
            | MenuAction::RandomFeatures
            | MenuAction::Enter
            | MenuAction::Talents
            | MenuAction::Settings => false,
        };
        if let Some(mut lock) = lock {
            if lock.0 != selected {
                lock.0 = selected;
            }
        }
        if let Some(mut disabled) = disabled {
            let off = match *action {
                MenuAction::Enter => cur.is_none() || !super::dungeon::available(&data::MAPS[selection.map]),
                MenuAction::Map(i) => !super::dungeon::available(&data::MAPS[i]),
                _ => false,
            };
            if disabled.0 != off {
                disabled.0 = off;
            }
        }
        match *action {
            MenuAction::Race(r) => {
                let ok = all.is_empty() || all.iter().any(|l| find(l, Some(r), None).is_some());
                let want = if ok {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *vis != want {
                    *vis = want;
                }
            }
            _ => {}
        }
        let show = *interaction != Interaction::None || art.races.is_none();
        for child in children {
            if let Ok(mut label) = labels.get_mut(*child) {
                let want = if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *label != want {
                    *label = want;
                }
            }
        }
    }

    if let (Some((_, size)), Some((_, sex))) = (&art.races, cur) {
        for (face, mut img) in &mut faces {
            if let Some(tc) = race_tc(face.0, sex) {
                let rect = Some(tc_rect(*size, tc));
                if img.rect != rect {
                    img.rect = rect;
                }
            }
        }
    }

    for (action, mut style) in &mut styles {
        // Painted by [`super::features_ui::refresh`] from the dials' ranges.
        if matches!(action, MenuAction::Feature(..) | MenuAction::RandomFeatures) {
            continue;
        }
        let selected = match action {
            MenuAction::Sex(s) => cur.is_some_and(|c| c.1 == *s),
            MenuAction::Map(i) => selection.map == *i,
            _ => false,
        };
        let enabled = match action {
            MenuAction::Enter => {
                cur.is_some() && super::dungeon::available(&data::MAPS[selection.map])
            }
            MenuAction::Map(i) => super::dungeon::available(&data::MAPS[*i]),
            _ => true,
        };
        if style.selected != selected || style.enabled != enabled {
            style.selected = selected;
            style.enabled = enabled;
        }
    }
    let class = selection.class_def();
    if let Ok((mut t, mut color)) = name.single_mut() {
        let want = match (class, cur) {
            (Some(c), Some((r, _))) => format!("{} {}", race_name(r), c.name),
            _ => String::new(),
        };
        if t.0 != want {
            t.0 = want;
        }
        // In the class's own colour, as the game's raid frames and chat show it.
        let want = class.map_or(GOLD, |c| rgb(c.color));
        if color.0 != want {
            color.0 = want;
        }
    }
}
