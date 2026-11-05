//! Equipment catalog and a temporary outfit on an isolated character.
use super::super::{data::CLASSES, races::RACES};
use super::*;

pub(super) fn pick_race(gear: &mut Equipment, id: u8) {
    let Some(race) = RACES.iter().find(|race| race.id == id) else {
        return;
    };
    gear.viewer_race = Some(id);
    if !race.classes.contains(&gear.viewer_class) {
        if let Some(class) = CLASSES
            .iter()
            .find(|class| race.classes.contains(&class.id))
        {
            gear.viewer_class = class.id;
        }
    }
    fit_outfit(gear);
}

pub(super) fn fit_outfit(gear: &mut Equipment) {
    let class = gear.viewer_class;
    let catalog = &gear.catalog;
    gear.viewer_outfit.retain(|slot, id| {
        catalog
            .definition(id)
            .is_some_and(|item| item.fits(class, slot))
    });
    if !matches!(class, 1 | 4 | 7) {
        gear.viewer_off_hand = false;
    }
}

pub(super) fn try_item(gear: &mut Equipment, id: &str) {
    let Some(item) = gear.catalog.definition(id).cloned() else {
        return;
    };
    let mut slot = if gear.viewer_off_hand && item.fits(gear.viewer_class, "off_hand") {
        "off_hand".to_string()
    } else {
        item.slot.clone()
    };
    if !item.fits(gear.viewer_class, &slot) {
        return;
    }
    if matches!(slot.as_str(), "finger_1" | "trinket_1") {
        let worn = gear
            .viewer_outfit
            .iter()
            .find(|(worn, current)| current.as_str() == id && item.fits(gear.viewer_class, worn))
            .map(|(slot, _)| slot.clone());
        if let Some(worn) = worn {
            gear.viewer_outfit.remove(&worn);
            return;
        }
    }
    if gear
        .viewer_outfit
        .get(&slot)
        .is_some_and(|current| current == id)
    {
        gear.viewer_outfit.remove(&slot);
        return;
    }
    if matches!(slot.as_str(), "finger_1" | "trinket_1") && gear.viewer_outfit.contains_key(&slot) {
        slot = slot.replace("_1", "_2");
    }
    if slot == "main_hand" && item.two_handed {
        gear.viewer_outfit.remove("off_hand");
    }
    if slot == "off_hand"
        && gear
            .viewer_outfit
            .get("main_hand")
            .and_then(|id| gear.catalog.definition(id))
            .is_some_and(|item| item.two_handed)
    {
        gear.viewer_outfit.remove("main_hand");
    }
    gear.viewer_outfit.insert(slot, id.to_string());
}

pub(super) fn build(p: &mut ChildSpawnerCommands, f: &Handle<Font>, gear: &Equipment, inner: f32) {
    let race = RACES.iter().find(|race| Some(race.id) == gear.viewer_race);
    let catalog = &gear.catalog;
    // Race browsing scrolls separately, keeping gender and class choices in view.
    p.spawn(Node {
        width: Val::Px(if inner < 1100. { 190. } else { 220. }),
        flex_basis: Val::Auto,
        flex_shrink: 0.,
        ..pane_node(0.)
    })
    .with_children(|p| {
        skin::label(
            p,
            f,
            race.map_or("Choose a race", |race| race.name),
            size::HEADING,
            skin::GOLD,
        );
        row(p, |p| {
            choice(p, f, "Male", Action::ViewerSex(0), gear.viewer_sex == 0);
            choice(p, f, "Female", Action::ViewerSex(1), gear.viewer_sex == 1);
        });
        skin::label(p, f, "CLASS", size::CAPTION, skin::MUTED);
        grid(p, 2, |p| {
            for class in CLASSES
                .iter()
                .filter(|class| race.is_some_and(|race| race.classes.contains(&class.id)))
            {
                choice(
                    p,
                    f,
                    class.name,
                    Action::ViewerClass(class.id),
                    gear.viewer_class == class.id,
                );
            }
        });
        skin::rule(p);
        skin::label(p, f, "RACE", size::CAPTION, skin::MUTED);
        p.spawn(scroller(6)).with_children(|p| {
            grid(p, 2, |p| {
                for race in RACES {
                    choice(
                        p,
                        f,
                        race.name,
                        Action::ViewerRace(race.id),
                        gear.viewer_race == Some(race.id),
                    );
                }
            })
        });
    });
    p.spawn(Node {
        width: Val::Px(if inner < 1100. { 290. } else { 360. }),
        flex_basis: Val::Auto,
        flex_shrink: 0.,
        ..pane_node(0.)
    })
    .with_children(|p| {
        skin::label(p, f, "Character preview", size::HEADING, skin::TEXT);
        super::super::item_preview::build(p, f);
        row(p, |p| {
            button(p, f, "Left", Action::ViewerRotate(-0.4), true);
            button(p, f, "Right", Action::ViewerRotate(0.4), true);
            button(p, f, "+", Action::ViewerZoom(-0.1), true);
            button(p, f, "−", Action::ViewerZoom(0.1), true);
        });
        if matches!(gear.viewer_class, 1 | 4 | 7) {
            row(p, |p| {
                choice(
                    p,
                    f,
                    "Main hand",
                    Action::ViewerHand(false),
                    !gear.viewer_off_hand,
                );
                choice(
                    p,
                    f,
                    "Off hand",
                    Action::ViewerHand(true),
                    gear.viewer_off_hand,
                );
            });
        }
        button(
            p,
            f,
            "Clear outfit",
            Action::ViewerClear,
            !gear.viewer_outfit.is_empty(),
        );
        skin::label(
            p,
            f,
            "TRYING ON · click a slot to remove",
            size::CAPTION,
            skin::MUTED,
        );
        p.spawn(Node {
            height: Val::Px(90.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|p| {
            p.spawn(scroller(7)).with_children(|p| {
                if gear.viewer_outfit.is_empty() {
                    skin::label(
                        p,
                        f,
                        "Click an item to dress this character. Your saved gear stays unchanged.",
                        size::SMALL,
                        skin::MUTED,
                    );
                }
                for slot in SLOTS {
                    let Some(item) = gear
                        .viewer_outfit
                        .get(*slot)
                        .and_then(|id| catalog.definition(id))
                    else {
                        continue;
                    };
                    button(
                        p,
                        f,
                        &format!("{} · {}", slot_title(slot), item.name),
                        Action::ViewerRemove((*slot).into()),
                        true,
                    );
                }
            });
        });
        skin::label(
            p,
            f,
            "Jewelry has no visible model.",
            size::SMALL,
            skin::MUTED,
        );
    });
    p.spawn(pane_node(1.)).with_children(|p| {
        skin::label(
            p,
            f,
            format!(
                "{} · {}",
                race.map_or("Choose a race", |race| race.name),
                class_name(gear.viewer_class)
            ),
            size::HEADING,
            skin::GOLD,
        );
        search_box(p, f, gear);
        row(p, |p| {
            let slot = gear
                .viewer_slot
                .and_then(|i| SLOTS.get(i))
                .map_or("All slots".into(), |slot| slot_title(slot));
            button(p, f, &slot, Action::ViewerSlot, true);
            button(
                p,
                f,
                ["All qualities", "Common", "Uncommon", "Rare", "Epic"][gear.viewer_quality],
                Action::ViewerQuality,
                true,
            );
            let zone = gear
                .viewer_zone
                .as_ref()
                .and_then(|id| catalog.zones.get(id))
                .map_or("All zones", |zone| zone.name.as_str());
            button(p, f, zone, Action::ViewerZone, true);
            button(
                p,
                f,
                if gear.viewer_sort_zone {
                    "Sort: Zone"
                } else {
                    "Sort: Slot"
                },
                Action::ViewerSort,
                true,
            );
            button(p, f, "Reset filters", Action::ViewerReset, true);
        });
        let query = gear.search.trim().to_lowercase();
        let quality = ["", "common", "uncommon", "rare", "epic"][gear.viewer_quality];
        let mut items: Vec<_> = catalog
            .items
            .iter()
            .filter(|item| {
                let zone = catalog
                    .zones
                    .get(&item.zone)
                    .map_or(item.zone.as_str(), |zone| zone.name.as_str());
                !item.is_starter()
                    && item.can_use(gear.viewer_class)
                    && slot_matches(gear.viewer_slot, &item.slot)
                    && (quality.is_empty() || item.quality.eq_ignore_ascii_case(quality))
                    && gear.viewer_zone.as_ref().is_none_or(|id| *id == item.zone)
                    && (query.is_empty()
                        || format!(
                            "{} {} {} {}",
                            item.name,
                            item.category,
                            label_slot(&item.slot),
                            zone
                        )
                        .to_lowercase()
                        .contains(&query))
            })
            .collect();
        items.sort_by(|a, b| {
            let zone_order = if gear.viewer_sort_zone {
                let zone_a = catalog.zones.get(&a.zone);
                let zone_b = catalog.zones.get(&b.zone);
                zone_a
                    .map_or(0, |zone| zone.tier)
                    .cmp(&zone_b.map_or(0, |zone| zone.tier))
                    .then_with(|| {
                        zone_a
                            .map_or(a.zone.as_str(), |zone| zone.name.as_str())
                            .cmp(zone_b.map_or(b.zone.as_str(), |zone| zone.name.as_str()))
                    })
            } else {
                std::cmp::Ordering::Equal
            };
            zone_order
                .then_with(|| a.slot.cmp(&b.slot))
                .then_with(|| a.item_level(catalog, 0).cmp(&b.item_level(catalog, 0)))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });
        skin::label(
            p,
            f,
            format!(
                "{} · click to try on, click again to remove",
                skin::plural(items.len() as u32, "item")
            ),
            size::SMALL,
            skin::MUTED,
        );
        p.spawn(scroller(4)).with_children(|p| {
            if items.is_empty() {
                skin::label(
                    p,
                    f,
                    "No items match. Clear the search or reset the filters.",
                    size::BODY,
                    skin::TEXT,
                );
                return;
            }
            grid(p, if inner < 1100. { 1 } else { 2 }, |p| {
                for item in items {
                    let color = item_color(item);
                    let selected = gear.viewer_outfit.values().any(|id| *id == item.id);
                    p.spawn((
                        Button,
                        skin::MenuControl,
                        Action::ViewerTry(item.id.clone()),
                        skin::Control {
                            primary: false,
                            selected,
                            enabled: true,
                        },
                        ItemTip(Some(Item {
                            definition: item.id.clone(),
                            ..Item::default()
                        })),
                        Node {
                            min_width: Val::Px(0.),
                            padding: UiRect::all(Val::Px(gap::S)),
                            border: UiRect::left(Val::Px(3.)),
                            border_radius: BorderRadius::all(Val::Px(4.)),
                            column_gap: Val::Px(gap::S),
                            align_items: AlignItems::Start,
                            ..default()
                        },
                        BackgroundColor(if selected {
                            Color::srgb(0.16, 0.08, 0.12)
                        } else {
                            skin::SURFACE
                        }),
                        BorderColor::all(color),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            ItemIcon(item.icon.clone()),
                            ImageNode::default(),
                            Node {
                                width: Val::Px(36.),
                                height: Val::Px(36.),
                                flex_shrink: 0.,
                                ..default()
                            },
                        ));
                        p.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.,
                            min_width: Val::Px(0.),
                            row_gap: Val::Px(3.),
                            ..default()
                        })
                        .with_children(|p| {
                            skin::label(p, f, &item.name, size::BODY, color);
                            if selected {
                                skin::label(p, f, "Trying on", size::CAPTION, skin::GOLD);
                            }
                            skin::label(
                                p,
                                f,
                                format!(
                                    "iLv {} · {} · {}",
                                    item.item_level(catalog, 0),
                                    slot_title(&item.slot),
                                    item.quality
                                ),
                                size::CAPTION,
                                skin::MUTED,
                            );
                            let stats = amounts(&item.base_bonuses).replace('\n', " · ");
                            if !stats.is_empty() {
                                skin::label(p, f, stats, size::SMALL, skin::GREEN);
                            }
                            if let Some(set) = &item.set_name {
                                skin::label(p, f, format!("Set: {set}"), size::SMALL, skin::GOLD);
                            }
                            let zone = catalog.zones.get(&item.zone);
                            skin::label(
                                p,
                                f,
                                zone.map_or(item.zone.as_str(), |zone| zone.name.as_str()),
                                size::SMALL,
                                skin::MUTED,
                            );
                            if let Some(zone) = zone {
                                let bosses: Vec<_> = zone
                                    .boss_drops
                                    .iter()
                                    .filter(|drop| drop.item == item.id)
                                    .map(|drop| drop.boss.as_str())
                                    .collect();
                                if zone
                                    .equipment
                                    .iter()
                                    .any(|drop| drop.item == item.id && drop.weight > 0)
                                {
                                    skin::label(p, f, "Elite drop", size::CAPTION, skin::MUTED);
                                }
                                if !bosses.is_empty() {
                                    skin::label(
                                        p,
                                        f,
                                        format!("Boss: {}", bosses.join(", ")),
                                        size::CAPTION,
                                        skin::GOLD,
                                    );
                                }
                            }
                        });
                    });
                }
            });
        });
    });
}

fn grid(
    p: &mut ChildSpawnerCommands,
    columns: u16,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    p.spawn(Node {
        width: Val::Percent(100.),
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::flex(columns, 1.),
        column_gap: Val::Px(gap::XS),
        row_gap: Val::Px(gap::XS),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(children);
}
