//! Inventory, equipment and shared auctions in the character menu.
use super::{coop::Coop, front_end as skin, hud::HudFont, inventory::Equipment, Phase, Selection};
use crate::glue::GlueClicks;
use asr_coop::economy::{Action as Trade, Definition, Item, SLOTS};
use bevy::prelude::*;

#[derive(Component, Clone)]
pub(super) enum Action {
    Open(bool),
    Close,
    Tab(bool),
    Select(u64),
    Listing(u64),
    Filter(usize),
    SlotFilter(Option<usize>),
    Page(i32),
    Equip(String),
    Unequip,
    Lock,
    Upgrade,
    Scrap,
    Sell,
    Price(i32),
    Buy,
    Cancel,
    Confirm,
    Keep,
    Refresh,
}
#[derive(Component)]
struct Root;
#[derive(Component)]
struct ItemIcon(String);
#[derive(Component)]
struct ScrollColumn;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (input, draw, icons, scroll)
            .chain()
            .after(super::hud::show_panels),
    );
}
fn button(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    action: Action,
    enabled: bool,
) {
    p.spawn((
        Button,
        action,
        skin::Control {
            primary: false,
            selected: false,
            enabled,
        },
        Node {
            padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
            min_height: Val::Px(29.),
            border: UiRect::all(Val::Px(1.)),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(skin::EDGE),
    ))
    .with_children(|p| {
        skin::label(
            p,
            f,
            title,
            13.,
            if enabled { skin::TEXT } else { skin::MUTED },
        );
    });
}
fn row(p: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        column_gap: Val::Px(6.),
        row_gap: Val::Px(5.),
        flex_wrap: FlexWrap::Wrap,
        ..default()
    })
    .with_children(children);
}
fn label_slot(slot: &str) -> String {
    slot.replace('_', " ")
}
/// The character sheet's name and empty-slot art for an economy slot, as
/// `PaperDollItemFrame.dbc` pairs them: the back shows the chest art.
fn slot_info(slot: &str) -> (&'static str, &'static str) {
    match slot {
        "head" => ("Head", "Head"),
        "neck" => ("Neck", "Neck"),
        "shoulders" => ("Shoulder", "Shoulder"),
        "back" => ("Back", "Chest"),
        "chest" => ("Chest", "Chest"),
        "shirt" => ("Shirt", "Shirt"),
        "tabard" => ("Tabard", "Tabard"),
        "wrists" => ("Wrist", "Wrists"),
        "hands" => ("Hands", "Hands"),
        "belt" => ("Waist", "Waist"),
        "legs" => ("Legs", "Legs"),
        "feet" => ("Feet", "Feet"),
        "finger_1" | "finger_2" => ("Finger", "Finger"),
        "trinket_1" | "trinket_2" => ("Trinket", "Trinket"),
        "main_hand" => ("Main Hand", "MainHand"),
        "off_hand" => ("Off Hand", "SecondaryHand"),
        "ranged" => ("Ranged", "Ranged"),
        _ => ("Equipment", "Chest"),
    }
}
/// A slot as a button names it, numbering the second ring and trinket.
fn slot_title(slot: &str) -> String {
    match slot {
        "finger_1" | "trinket_1" => format!("{} 1", slot_info(slot).0),
        "finger_2" | "trinket_2" => format!("{} 2", slot_info(slot).0),
        _ => slot_info(slot).0.into(),
    }
}
fn slot_art(slot: &str) -> String {
    format!(
        "Interface\\PaperDoll\\UI-PaperDoll-Slot-{}",
        slot_info(slot).1
    )
}
/// Both rings, and both trinkets, are one kind of item.
fn slot_kind(slot: &str) -> &str {
    match slot {
        "finger_2" => "finger_1",
        "trinket_2" => "trinket_1",
        other => other,
    }
}
/// `PaperDollFrame.xml`: eight slots down the left, eight down the right, weapons below.
const DOLL_LEFT: [&str; 8] = [
    "head",
    "neck",
    "shoulders",
    "back",
    "chest",
    "shirt",
    "tabard",
    "wrists",
];
const DOLL_RIGHT: [&str; 8] = [
    "hands",
    "belt",
    "legs",
    "feet",
    "finger_1",
    "finger_2",
    "trinket_1",
    "trinket_2",
];
const DOLL_WEAPONS: [&str; 3] = ["main_hand", "off_hand", "ranged"];
/// One filter per kind of item, in character-sheet order.
fn filter_slots() -> impl Iterator<Item = (usize, &'static str)> {
    SLOTS
        .iter()
        .enumerate()
        .filter(|(_, s)| !matches!(**s, "finger_2" | "trinket_2"))
        .map(|(i, s)| (i, *s))
}
fn slot_matches(filter: Option<usize>, slot: &str) -> bool {
    filter.is_none_or(|i| SLOTS.get(i).is_some_and(|s| slot_kind(s) == slot_kind(slot)))
}
#[allow(clippy::too_many_arguments)]
fn doll_slot(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    slot: &str,
    item: Option<(&Item, &Definition)>,
    selected: bool,
    filtered: bool,
    label_right: bool,
) {
    let (name, _) = slot_info(slot);
    let action = item.map_or_else(
        || Action::SlotFilter(SLOTS.iter().position(|s| *s == slot)),
        |(i, _)| Action::Select(i.id),
    );
    p.spawn(Node {
        column_gap: Val::Px(6.),
        align_items: AlignItems::Center,
        flex_direction: if label_right {
            FlexDirection::Row
        } else {
            FlexDirection::RowReverse
        },
        ..default()
    })
    .with_children(|p| {
        p.spawn((
            Button,
            action,
            skin::Control {
                primary: false,
                selected: selected || filtered,
                enabled: true,
            },
            Node {
                width: Val::Px(40.),
                height: Val::Px(40.),
                padding: UiRect::all(Val::Px(1.)),
                border: UiRect::all(Val::Px(1.)),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(if selected || filtered {
                skin::GOLD
            } else {
                skin::EDGE
            }),
        ))
        .with_children(|p| {
            p.spawn((
                ItemIcon(item.map_or_else(|| slot_art(slot), |(_, d)| d.icon.clone())),
                ImageNode::default(),
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
            ));
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: if label_right {
                AlignItems::FlexStart
            } else {
                AlignItems::FlexEnd
            },
            min_width: Val::Px(0.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, name, 10., skin::MUTED);
            if let Some((i, d)) = item {
                skin::label(p, f, format!("{} +{}", d.name, i.level), 11., skin::TEXT);
            }
        });
    });
}
fn slot_strip(p: &mut ChildSpawnerCommands, f: &Handle<Font>, current: Option<usize>) {
    p.spawn(Node {
        column_gap: Val::Px(3.),
        row_gap: Val::Px(3.),
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|p| {
        button(p, f, "All slots", Action::SlotFilter(None), true);
        for (i, slot) in filter_slots() {
            let on = current == Some(i);
            p.spawn((
                Button,
                Action::SlotFilter(Some(i)),
                skin::Control {
                    primary: false,
                    selected: on,
                    enabled: true,
                },
                Node {
                    width: Val::Px(29.),
                    height: Val::Px(29.),
                    border: UiRect::all(Val::Px(1.)),
                    flex_shrink: 0.,
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                BorderColor::all(if on { skin::GOLD } else { skin::EDGE }),
            ))
            .with_children(|p| {
                p.spawn((
                    ItemIcon(slot_art(slot)),
                    ImageNode::default(),
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                ));
            });
        }
    });
    if let Some(slot) = current.and_then(|i| SLOTS.get(i)) {
        skin::label(
            p,
            f,
            format!("Showing {} items", slot_info(slot).0),
            12.,
            skin::GOLD,
        );
    }
}
fn amounts(values: &std::collections::BTreeMap<String, u32>) -> String {
    values
        .iter()
        .map(|(k, v)| {
            if let Some(name) = k.strip_suffix("_bp") {
                format!("+{:.2}% {}", *v as f32 / 100., name.replace('_', " "))
            } else {
                format!("+{v} {}", k.replace('_', " "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    options: Res<super::options::Options>,
    atlas: Res<super::adventure_atlas::Atlas>,
    actions: Query<(Entity, &Action, &skin::Control)>,
    mut gear: ResMut<Equipment>,
) {
    if *phase.get() != Phase::Menu {
        if gear.open {
            gear.open = false;
            gear.revision += 1;
        }
        return;
    }
    if options.open || coop.open || atlas.open {
        return;
    }
    if gear.open && keys.just_pressed(KeyCode::Escape) {
        gear.open = false;
        gear.confirm = None;
        gear.revision += 1;
        return;
    }
    let Some(class) = selection.class_def().map(|c| c.id) else {
        return;
    };
    let Some(action) = actions
        .iter()
        .find(|(e, _, c)| c.enabled && clicks.hit(*e))
        .map(|(_, a, _)| a.clone())
    else {
        return;
    };
    let item = gear.selected.and_then(|id| {
        gear.hero(&coop.url, class)?
            .inventory
            .items
            .iter()
            .find(|i| i.id == id)
            .cloned()
    });
    let sale = gear
        .listing
        .and_then(|id| gear.listings.iter().find(|l| l.id == id).cloned());
    let mut trade = None;
    match action {
        Action::Open(market) => {
            gear.open = true;
            gear.market = market;
            gear.confirm = None;
            gear.slot_filter = None;
            gear.page = 0;
            gear.filter = 0;
        }
        Action::Close => {
            gear.open = false;
            gear.confirm = None;
        }
        Action::Tab(market) => {
            gear.market = market;
            gear.page = 0;
            gear.filter = 0;
            gear.confirm = None;
        }
        Action::Select(id) => {
            gear.selected = Some(id);
            gear.listing = None;
            gear.confirm = None;
        }
        Action::Listing(id) => {
            gear.listing = Some(id);
            gear.selected = None;
            gear.confirm = None;
        }
        Action::Filter(filter) => {
            gear.filter = filter;
            gear.page = 0;
        }
        Action::SlotFilter(slot) => {
            gear.slot_filter = slot;
            gear.page = 0;
        }
        Action::Page(delta) => gear.page = gear.page.saturating_add_signed(delta as isize),
        Action::Price(delta) => {
            gear.price = gear
                .price
                .saturating_add_signed(i64::from(delta))
                .clamp(1, 1_000_000_000)
        }
        Action::Equip(slot) => {
            if let Some(item) = item {
                trade = Some(Trade::Equip {
                    item: item.id,
                    slot: Some(slot),
                });
            }
        }
        Action::Unequip => {
            if let Some(item) = item {
                trade = Some(Trade::Equip {
                    item: item.id,
                    slot: None,
                });
            }
        }
        Action::Lock => {
            if let Some(item) = item {
                trade = Some(Trade::Lock {
                    item: item.id,
                    locked: !item.locked,
                });
            }
        }
        Action::Upgrade => {
            if let Some(item) = item {
                gear.confirm = Some(Trade::Upgrade { item: item.id });
            }
        }
        Action::Scrap => {
            if let Some(item) = item {
                gear.confirm = Some(Trade::Scrap { item: item.id });
            }
        }
        Action::Sell => {
            if let Some(item) = item {
                gear.confirm = Some(Trade::List {
                    item: item.id,
                    price: gear.price,
                });
            }
        }
        Action::Buy => {
            if let Some(sale) = sale {
                gear.confirm = Some(Trade::Buy { listing: sale.id });
            }
        }
        Action::Cancel => {
            if let Some(sale) = sale {
                gear.confirm = Some(Trade::Cancel { listing: sale.id });
            }
        }
        Action::Confirm => trade = gear.confirm.take(),
        Action::Keep => gear.confirm = None,
        Action::Refresh => trade = Some(Trade::Refresh),
    }
    if let Some(trade) = trade {
        if !gear.busy() {
            gear.action(&coop.url, class, trade);
        }
    }
    gear.revision += 1;
}
fn item_row(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    item: &Item,
    d: &Definition,
    action: Action,
    extra: &str,
    selected: bool,
) {
    p.spawn((
        Button,
        action,
        skin::Control {
            primary: false,
            selected,
            enabled: true,
        },
        Node {
            width: Val::Percent(100.),
            min_height: Val::Px(47.),
            padding: UiRect::all(Val::Px(5.)),
            column_gap: Val::Px(8.),
            border: UiRect::all(Val::Px(1.)),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(skin::SURFACE),
        BorderColor::all(if selected { skin::GOLD } else { skin::EDGE }),
    ))
    .with_children(|p| {
        p.spawn((
            ItemIcon(d.icon.clone()),
            ImageNode::default(),
            Node {
                width: Val::Px(34.),
                height: Val::Px(34.),
                flex_shrink: 0.,
                ..default()
            },
        ));
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|p| {
            skin::label(
                p,
                font,
                format!("{} +{}", d.name, item.level),
                13.,
                skin::TEXT,
            );
            skin::label(p, font, extra, 11., skin::MUTED);
        });
    });
}
fn detail(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    class: u8,
    item: &Item,
    market_item: bool,
) {
    let Some(d) = gear.catalog.definition(&item.definition) else {
        return;
    };
    skin::label(p, f, format!("{} +{}", d.name, item.level), 22., skin::GOLD);
    skin::label(
        p,
        f,
        format!("Common · {}", label_slot(&d.category)),
        13.,
        skin::TEXT,
    );
    skin::label(
        p,
        f,
        gear.catalog
            .zones
            .get(&d.zone)
            .map_or(d.zone.as_str(), |z| z.name.as_str()),
        12.,
        skin::MUTED,
    );
    if !d.classes.contains(&class) {
        skin::label(
            p,
            f,
            "Your class cannot equip this item",
            13.,
            Color::srgb(1., 0.35, 0.25),
        );
    }
    skin::rule(p);
    skin::label(p, f, amounts(d.bonuses(item.level)), 15., skin::GREEN);
    let enabled = !gear.busy();
    if market_item {
        if let Some(sale) = gear
            .listing
            .and_then(|id| gear.listings.iter().find(|l| l.id == id))
        {
            skin::rule(p);
            skin::label(
                p,
                f,
                format!("{} gold\nSeller: {}", sale.price, sale.seller_name),
                16.,
                skin::GOLD,
            );
            row(p, |p| {
                button(
                    p,
                    f,
                    if sale.seller == "self" {
                        "Cancel auction"
                    } else {
                        "Buy item"
                    },
                    if sale.seller == "self" {
                        Action::Cancel
                    } else {
                        Action::Buy
                    },
                    enabled,
                )
            });
        }
    } else {
        skin::rule(p);
        if let Some(slot) = &item.slot {
            skin::label(
                p,
                f,
                format!("Equipped: {}", slot_title(slot)),
                12.,
                skin::GREEN,
            );
            button(p, f, "Unequip", Action::Unequip, enabled && !item.listed);
        } else if !item.listed {
            row(p, |p| {
                for slot in SLOTS.iter().filter(|slot| d.fits(class, slot)) {
                    button(
                        p,
                        f,
                        &format!("Equip: {}", slot_title(slot)),
                        Action::Equip((*slot).into()),
                        enabled,
                    );
                }
            });
        }
        row(p, |p| {
            button(
                p,
                f,
                if item.locked { "Unlock" } else { "Lock item" },
                Action::Lock,
                enabled && !item.listed,
            )
        });
        if let Some(upgrade) = d.upgrades.get(usize::from(item.level)) {
            skin::label(
                p,
                f,
                format!(
                    "Upgrade to +{}\nCosts: {}",
                    item.level + 1,
                    amounts(&upgrade.cost)
                ),
                12.,
                skin::TEXT,
            );
            button(
                p,
                f,
                "Upgrade",
                Action::Upgrade,
                enabled && !item.locked && !item.listed,
            );
        } else {
            skin::label(p, f, "Fully upgraded (+5)", 13., skin::GREEN);
        }
        skin::label(
            p,
            f,
            format!(
                "Scrap returns: {}",
                d.scrap
                    .iter()
                    .map(|(k, n)| format!("{} {k}", n + u32::from(item.level)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            12.,
            skin::MUTED,
        );
        button(
            p,
            f,
            "Scrap item",
            Action::Scrap,
            enabled && !item.locked && !item.listed && item.slot.is_none(),
        );
        if gear.market {
            skin::rule(p);
            skin::label(
                p,
                f,
                format!("Sell for {} gold · 48 hours", gear.price),
                15.,
                skin::GOLD,
            );
            row(p, |p| {
                for (name, delta) in [
                    ("−100", -100),
                    ("−10", -10),
                    ("−1", -1),
                    ("+1", 1),
                    ("+10", 10),
                    ("+100", 100),
                ] {
                    button(p, f, name, Action::Price(delta), true);
                }
            });
            button(
                p,
                f,
                "List item",
                Action::Sell,
                enabled && !item.locked && !item.listed && item.slot.is_none(),
            );
        }
    }
    if let Some(action) = &gear.confirm {
        skin::rule(p);
        let question = match action {
            Trade::Scrap { .. } => "Scrap this item permanently?".into(),
            Trade::Upgrade { .. } => "Spend the materials to upgrade?".into(),
            Trade::List { price, .. } => format!("List this item for {price} gold?"),
            Trade::Buy { .. } => "Buy this listing for the displayed price?".into(),
            Trade::Cancel { .. } => "Return this listing to your bags?".into(),
            _ => "Confirm?".into(),
        };
        skin::label(p, f, question, 14., skin::GOLD);
        row(p, |p| {
            button(p, f, "Confirm", Action::Confirm, enabled);
            button(p, f, "Keep item", Action::Keep, true);
        });
    }
}
fn draw(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    gear: Res<Equipment>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    roots: Query<Entity, With<Root>>,
    mut last: Local<Option<(u64, u32, bool)>>,
) {
    let show = gear.open && *phase.get() == Phase::Menu;
    let key = (gear.revision, selection.version, show);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    for e in &roots {
        commands.entity(e).despawn();
    }
    if !show {
        return;
    }
    let Some(font) = font else {
        *last = None;
        return;
    };
    let Some(class) = selection.class_def() else {
        return;
    };
    let f = &font.0;
    let hero = gear.hero(&coop.url, class.id);
    commands.spawn((Root, Node { position_type: PositionType::Absolute, width: Val::Percent(100.), height: Val::Percent(100.), align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() }, BackgroundColor(Color::srgba(0., 0., 0., 0.85)), GlobalZIndex(400)))
        .with_children(|root| {
            root.spawn(skin::panel(Node { width: Val::Percent(96.), max_width: Val::Px(1420.), height: Val::Percent(94.), padding: UiRect::all(Val::Px(18.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.), overflow: Overflow::clip(), ..default() })).with_children(|p| {
                row(p, |p| {
                    skin::label(p, f, format!("{} · {}", if gear.market { "Auction House" } else { "Inventory" }, class.name), 26., skin::GOLD);
                    button(p, f, "Inventory", Action::Tab(false), true);
                    button(p, f, "Auction House", Action::Tab(true), true);
                    button(p, f, "Refresh", Action::Refresh, !gear.busy());
                    button(p, f, "Close", Action::Close, true);
                });
                let gold = hero.map_or(0, |h| h.inventory.gold);
                let materials = hero.map_or(String::new(), |h| h.inventory.materials.iter().map(|(k, v)| format!("{v} {k}")).collect::<Vec<_>>().join("  ·  "));
                skin::label(p, f, format!("{gold} gold    {materials}"), 15., skin::GOLD);
                skin::label(p, f, &gear.status, 12., skin::MUTED);
                p.spawn(Node { flex_grow: 1., min_height: Val::Px(0.), column_gap: Val::Px(14.), ..default() }).with_children(|body| {
                    body.spawn((ScrollColumn, ScrollPosition::default(), bevy::ui::RelativeCursorPosition::default(), Node { width: Val::Percent(27.), flex_direction: FlexDirection::Column, row_gap: Val::Px(5.), overflow: Overflow::scroll_y(), ..default() })).with_children(|p| {
                        skin::label(p, f, "EQUIPMENT", 17., skin::GOLD);
                        let equipped = |slot: &str| hero.and_then(|h| h.inventory.items.iter().find(|i| i.slot.as_deref() == Some(slot))).and_then(|i| Some((i, gear.catalog.definition(&i.definition)?)));
                        let filtered = |slot: &str| gear.slot_filter.and_then(|i| SLOTS.get(i)).is_some_and(|s| *s == slot);
                        let doll_column = |p: &mut ChildSpawnerCommands, slots: &[&str], label_right: bool| {
                            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.), width: Val::Percent(50.), align_items: if label_right { AlignItems::FlexStart } else { AlignItems::FlexEnd }, ..default() }).with_children(|p| {
                                for slot in slots {
                                    let item = equipped(slot);
                                    doll_slot(p, f, slot, item, item.is_some_and(|(i, _)| gear.selected == Some(i.id)), filtered(slot), label_right);
                                }
                            });
                        };
                        p.spawn(Node { column_gap: Val::Px(6.), flex_shrink: 0., ..default() }).with_children(|p| {
                            doll_column(p, &DOLL_LEFT, true);
                            doll_column(p, &DOLL_RIGHT, false);
                        });
                        p.spawn(Node { column_gap: Val::Px(10.), justify_content: JustifyContent::Center, flex_shrink: 0., margin: UiRect::top(Val::Px(6.)), ..default() }).with_children(|p| {
                            for slot in DOLL_WEAPONS {
                                p.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(2.), ..default() }).with_children(|p| {
                                    let item = equipped(slot);
                                    doll_slot(p, f, slot, item, item.is_some_and(|(i, _)| gear.selected == Some(i.id)), filtered(slot), true);
                                });
                            }
                        });
                        skin::label(p, f, "Click an empty slot to show the items that fit it.", 11., skin::MUTED);
                        skin::rule(p);
                        if let Some(hero) = hero { skin::label(p, f, amounts(&hero.inventory.bonuses(&gear.catalog)), 12., skin::GREEN); }
                        skin::label(p, f, "Equipment bonuses apply when a run starts.\nKills earn items and materials. Unspent run gold is banked when you leave.", 12., skin::MUTED);
                    });
                    body.spawn((ScrollColumn, ScrollPosition::default(), bevy::ui::RelativeCursorPosition::default(), Node { width: Val::Percent(36.), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.), overflow: Overflow::scroll_y(), ..default() })).with_children(|p| {
                        if gear.market {
                            row(p, |p| { button(p, f, "Market", Action::Filter(0), true); button(p, f, "My auctions", Action::Filter(1), true); button(p, f, "Sell from bags", Action::Filter(2), true); });
                        } else {
                            row(p, |p| { for (i, name) in ["All", "Weapons", "Armor", "Accessories"].iter().enumerate() { button(p, f, name, Action::Filter(i), true); } });
                        }
                        slot_strip(p, f, gear.slot_filter);
                        let mut total = 0;
                        if gear.market && gear.filter < 2 {
                            let entries: Vec<_> = gear.listings.iter().filter(|l| gear.filter == 0 || l.seller == "self").filter(|l| gear.catalog.definition(&l.item.definition).is_some_and(|d| slot_matches(gear.slot_filter, &d.slot))).collect();
                            total = entries.len();
                            for sale in entries.into_iter().skip(gear.page * 8).take(8) { if let Some(d) = gear.catalog.definition(&sale.item.definition) {
                                item_row(p, f, &sale.item, d, Action::Listing(sale.id), &format!("{} · {} gold · {}", slot_info(&d.slot).0, sale.price, sale.seller_name), gear.listing == Some(sale.id));
                            } }
                        } else if let Some(hero) = hero {
                            let entries: Vec<_> = hero.inventory.items.iter().filter(|i| {
                                let Some(d) = gear.catalog.definition(&i.definition) else { return false };
                                slot_matches(gear.slot_filter, &d.slot) && (gear.market || match gear.filter { 1 => matches!(d.slot.as_str(), "main_hand" | "off_hand" | "ranged"), 2 => !matches!(d.slot.as_str(), "main_hand" | "off_hand" | "ranged" | "neck" | "finger_1" | "trinket_1"), 3 => matches!(d.slot.as_str(), "neck" | "finger_1" | "trinket_1"), _ => true })
                            }).collect();
                            total = entries.len();
                            for item in entries.into_iter().skip(gear.page * 8).take(8) { if let Some(d) = gear.catalog.definition(&item.definition) {
                                let state = if item.listed { "On auction" } else if item.slot.is_some() { "Equipped" } else if item.locked { "Locked" } else { "In bags" };
                                item_row(p, f, item, d, Action::Select(item.id), &format!("{} · {state}", item.slot.as_deref().map_or_else(|| slot_info(&d.slot).0.to_string(), slot_title)), gear.selected == Some(item.id));
                            } }
                        }
                        if total == 0 { skin::label(p, f, if gear.market && gear.filter < 2 { "No auctions listed yet." } else { "Your bags are empty. Defeat enemies to earn equipment." }, 14., skin::MUTED); }
                        row(p, |p| { button(p, f, "Previous", Action::Page(-1), gear.page > 0); skin::label(p, f, format!("{} / {}", gear.page + 1, total.div_ceil(8).max(1)), 12., skin::TEXT); button(p, f, "Next", Action::Page(1), (gear.page + 1) * 8 < total); });
                    });
                    body.spawn((ScrollColumn, ScrollPosition::default(), bevy::ui::RelativeCursorPosition::default(), Node { width: Val::Percent(34.), flex_direction: FlexDirection::Column, row_gap: Val::Px(7.), overflow: Overflow::scroll_y(), ..default() })).with_children(|p| {
                        if let Some(sale) = gear.listing.and_then(|id| gear.listings.iter().find(|l| l.id == id)) { detail(p, f, &gear, class.id, &sale.item, true); }
                        else if let Some(item) = gear.selected.and_then(|id| hero?.inventory.items.iter().find(|i| i.id == id)) { detail(p, f, &gear, class.id, item, false); }
                        else { skin::label(p, f, "Select an item to inspect it", 18., skin::GOLD); skin::label(p, f, "Equip, upgrade with materials, scrap duplicates, or list an item for other players.", 14., skin::TEXT); }
                    });
                });
            });
        });
}
fn icons(
    mut assets: ResMut<benilla_assets::WorldAssets>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<(&ItemIcon, &mut ImageNode), Added<ItemIcon>>,
) {
    for (path, mut node) in &mut icons {
        let path = if path.0.to_ascii_lowercase().starts_with("interface\\") {
            path.0.clone()
        } else {
            format!("Interface\\Icons\\{}", path.0)
        };
        if let Some(image) = assets.sprite_texture(&path, &mut images) {
            node.image = image;
        }
    }
}
fn scroll(
    gear: Res<Equipment>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut columns: Query<
        (
            &mut ScrollPosition,
            &ComputedNode,
            &bevy::ui::RelativeCursorPosition,
        ),
        With<ScrollColumn>,
    >,
) {
    if !gear.open {
        wheel.clear();
        return;
    }
    let delta: f32 = wheel
        .read()
        .map(|e| {
            e.y * if e.unit == bevy::input::mouse::MouseScrollUnit::Line {
                30.
            } else {
                1.
            }
        })
        .sum();
    for (mut scroll, node, cursor) in &mut columns {
        if cursor
            .normalized
            .is_some_and(|p| (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y))
        {
            let limit = (node.content_size.y - node.size.y).max(0.) * node.inverse_scale_factor();
            scroll.0.y = (scroll.0.y - delta).clamp(0., limit);
        }
    }
}
