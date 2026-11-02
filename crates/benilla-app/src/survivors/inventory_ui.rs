//! Inventory, equipment and shared auctions in the character menu.
use super::{coop::Coop, front_end as skin, hud::HudFont, inventory::Equipment, Phase, Selection};
use crate::glue::GlueClicks;
use asr_coop::economy::{Action as Trade, Definition, Item, SLOTS};
use bevy::input::{keyboard::KeyboardInput, ButtonState};
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
    LootTable,
    Search,
    ClearSearch,
}
#[derive(Component)]
struct Root;
#[derive(Component)]
struct ItemIcon(String);
#[derive(Component)]
struct ScrollColumn(usize);
#[derive(Component)]
struct ScrollThumb(usize);
#[derive(Component)]
struct SlotFrame;
#[derive(Component)]
struct ItemTip(Option<Item>);
#[derive(Component)]
struct ItemTipRoot;
#[derive(Resource, Default)]
struct ScrollMemory {
    positions: [Vec2; 4],
    query: Option<(Option<usize>, bool, usize, Option<usize>, String)>,
}
const BAG_MIN_SLOTS: usize = 72;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ScrollMemory>();
    app.add_systems(
        Update,
        (
            input,
            search_input,
            draw,
            icons,
            scroll,
            scrollbars,
            item_tooltip,
        )
            .chain()
            .after(super::hud::show_panels),
    );
    app.add_systems(Update, slot_colors.after(draw).after(skin::hover));
}
fn button(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    action: Action,
    enabled: bool,
) -> Entity {
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
    })
    .id()
}
fn choice(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    action: Action,
    selected: bool,
) {
    let entity = button(p, f, title, action, true);
    p.commands().entity(entity).insert((
        skin::Control {
            primary: false,
            selected,
            enabled: true,
        },
        BorderColor::all(if selected { skin::GOLD } else { skin::EDGE }),
        BackgroundColor(if selected {
            Color::srgb(0.19, 0.045, 0.02)
        } else {
            skin::SURFACE
        }),
    ));
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
fn class_name(class: u8) -> &'static str {
    super::data::CLASSES
        .iter()
        .find(|c| c.id == class)
        .map_or("another class", |c| c.name)
}
fn item_color(d: &Definition) -> Color {
    let hex = match d.quality.to_ascii_lowercase().as_str() {
        "poor" | "0" => 0x9d9d9d,
        "uncommon" | "2" => 0x1eff00,
        "rare" | "3" => 0x0070dd,
        "epic" | "4" => 0xa335ee,
        "legendary" | "5" => 0xff8000,
        "artifact" | "heirloom" | "6" | "7" => 0xe6cc80,
        _ => 0xffffff,
    };
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
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
fn slot_matches(filter: Option<usize>, slot: &str) -> bool {
    filter.is_none_or(|i| {
        SLOTS
            .get(i)
            .is_some_and(|s| slot_kind(s) == slot_kind(slot))
    })
}
#[allow(clippy::too_many_arguments)]
fn doll_slot(
    p: &mut ChildSpawnerCommands,
    _f: &Handle<Font>,
    slot: &str,
    item: Option<(&Item, &Definition)>,
    selected: bool,
    filtered: bool,
    _label_right: bool,
) {
    let action = item.map_or_else(
        || Action::SlotFilter(SLOTS.iter().position(|s| *s == slot)),
        |(i, _)| Action::Select(i.id),
    );
    p.spawn((
        SlotFrame,
        ItemTip(item.map(|(item, _)| item.clone())),
        Button,
        action,
        skin::Control {
            primary: false,
            selected: selected || filtered,
            enabled: true,
        },
        Node {
            width: Val::Px(42.),
            height: Val::Px(42.),
            padding: UiRect::all(Val::Px(2.)),
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(4.)),
            flex_shrink: 0.,
            ..default()
        },
        BackgroundColor(Color::srgb(0.035, 0.032, 0.025)),
        BorderColor::all(if selected || filtered {
            skin::GOLD
        } else {
            Color::srgb(0.25, 0.24, 0.23)
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
}
fn amounts(values: &std::collections::BTreeMap<String, u32>) -> String {
    values
        .iter()
        .map(|(k, v)| {
            if let Some(name) = k.strip_suffix("_bp") {
                let name = match name {
                    "critical_chance" => "Critical chance".into(),
                    "damage_reduction" => "Damage reduction".into(),
                    "cooldown_reduction" => "Cooldown recovery".into(),
                    "damage" => "Damage".into(),
                    other => other.replace('_', " "),
                };
                format!("+{:.2}% {}", *v as f32 / 100., name)
            } else {
                format!(
                    "+{v} {}",
                    if k == "max_health" {
                        "Max health".into()
                    } else {
                        k.replace('_', " ")
                    }
                )
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
    time: Res<Time<Real>>,
    mut last_pick: Local<Option<(u64, f64)>>,
    fields: super::text_edit::FieldQuery,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
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
        if gear.search_active {
            gear.search_active = false;
            gear.revision += 1;
            return;
        }
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
    let view = gear.view(&coop.url, class);
    let item = gear.selected.and_then(|id| {
        view.as_ref()?
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
    gear.search_active = matches!(action, Action::Search);
    match action {
        Action::Open(market) => {
            gear.open = true;
            gear.market = market;
            gear.confirm = None;
            gear.slot_filter = None;
            gear.filter = 0;
            gear.search.clear();
            gear.show_loot = false;
        }
        Action::Close => {
            gear.open = false;
            gear.confirm = None;
        }
        Action::Tab(market) => {
            gear.market = market;
            gear.filter = 0;
            gear.confirm = None;
            gear.selected = None;
            gear.listing = None;
            gear.show_loot = false;
            gear.search.clear();
        }
        Action::Select(id) => {
            gear.show_loot = false;
            gear.selected = Some(id);
            gear.listing = None;
            gear.confirm = None;
            let now = time.elapsed_secs_f64();
            if last_pick.is_some_and(|(previous, at)| previous == id && now - at < 0.35) {
                if let Some(hero) = view.as_ref() {
                    if let Some(item) = hero
                        .inventory
                        .items
                        .iter()
                        .find(|item| item.id == id && !item.listed)
                    {
                        if item.slot.is_some() {
                            if item.equipped_for(class) {
                                trade = Some(Trade::Equip {
                                    item: id,
                                    slot: None,
                                });
                            }
                        } else if let Some(d) = gear.catalog.definition(&item.definition) {
                            let slots: Vec<_> = SLOTS
                                .iter()
                                .filter(|slot| d.fits(class, slot))
                                .copied()
                                .collect();
                            let target = gear
                                .slot_filter
                                .and_then(|i| SLOTS.get(i))
                                .copied()
                                .filter(|slot| d.fits(class, slot))
                                .or_else(|| {
                                    slots.iter().copied().find(|slot| {
                                        !hero
                                            .inventory
                                            .items
                                            .iter()
                                            .any(|item| item.slot.as_deref() == Some(slot))
                                    })
                                })
                                .or_else(|| slots.first().copied());
                            if let Some(slot) = target {
                                trade = Some(Trade::Equip {
                                    item: id,
                                    slot: Some(slot.into()),
                                });
                            }
                        }
                    }
                }
                *last_pick = None;
            } else {
                *last_pick = Some((id, now));
            }
        }
        Action::LootTable => {
            gear.show_loot = true;
            gear.selected = None;
            gear.listing = None;
            gear.confirm = None;
        }
        Action::Listing(id) => {
            gear.show_loot = false;
            gear.listing = Some(id);
            gear.selected = None;
            gear.confirm = None;
        }
        Action::Filter(filter) => {
            gear.filter = filter;
        }
        Action::SlotFilter(slot) => {
            gear.slot_filter = slot;
        }
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
        Action::Search => {
            let point = windows
                .single()
                .ok()
                .and_then(|w| w.cursor_position().map(|p| p * w.scale_factor()));
            let value = gear.search.clone();
            let at = super::text_edit::clicked(10, &value, point, &fields);
            gear.search_cursor.place(
                &value,
                at,
                keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            );
        }
        Action::ClearSearch => {
            gear.search.clear();
        }
    }
    if let Some(trade) = trade {
        let item_id = match &trade {
            Trade::Equip { item, .. }
            | Trade::Lock { item, .. }
            | Trade::Upgrade { item }
            | Trade::Scrap { item }
            | Trade::List { item, .. } => Some(*item),
            _ => None,
        };
        if item_id.is_some_and(|item| gear.item_busy(item)) {
            gear.status = "This item already has an action waiting for confirmation.".into();
            gear.revision += 1;
            return;
        }
        if !gear.busy() {
            gear.action(&coop.url, class, trade);
        }
    }
    gear.revision += 1;
}
fn search_input(
    mut keyboard: MessageReader<KeyboardInput>,
    phase: Res<State<Phase>>,
    mut gear: ResMut<Equipment>,
    keys: Res<ButtonInput<KeyCode>>,
    mut clipboard: NonSendMut<crate::textinput::HostClipboard>,
    windows: Query<&bevy::window::RawHandleWrapper, With<bevy::window::PrimaryWindow>>,
) {
    let events: Vec<_> = keyboard.read().cloned().collect();
    if !gear.open || !gear.search_active || *phase.get() != Phase::Menu {
        return;
    }
    let control = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let wl = crate::textinput::wayland_display(windows.single().ok());
    let gear = &mut *gear;
    for event in events
        .into_iter()
        .filter(|e| e.state == ButtonState::Pressed)
    {
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => gear.search_active = false,
            KeyCode::Escape => continue,
            KeyCode::KeyC | KeyCode::KeyX if control => {
                if !gear.search_cursor.selected(&gear.search).is_empty() {
                    clipboard.write(wl, gear.search_cursor.selected(&gear.search));
                    if event.key_code == KeyCode::KeyX {
                        gear.search_cursor
                            .key(&mut gear.search, KeyCode::Backspace, false, false);
                    }
                }
            }
            KeyCode::KeyV if control => {
                if let Some(value) = clipboard.read(wl) {
                    gear.search_cursor.insert(&mut gear.search, &value, 64);
                }
            }
            key => {
                if !gear
                    .search_cursor
                    .key(&mut gear.search, key, shift, control)
                    && !control
                {
                    if let Some(text) = event.text {
                        gear.search_cursor.insert(&mut gear.search, &text, 64);
                    }
                }
            }
        }
        gear.revision += 1;
    }
}
fn item_slot(
    p: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    item: Option<(&Item, &Definition)>,
    action: Option<Action>,
    caption: &str,
    selected: bool,
) {
    let icon = item.map_or_else(
        || "Interface\\PaperDoll\\UI-Backpack-EmptySlot".to_owned(),
        |(_, d)| d.icon.clone(),
    );
    let node = Node {
        width: Val::Px(46.),
        height: Val::Px(46.),
        padding: UiRect::all(Val::Px(2.)),
        row_gap: Val::Px(1.),
        align_items: AlignItems::Center,
        flex_direction: FlexDirection::Column,
        border: UiRect::all(Val::Px(1.)),
        border_radius: BorderRadius::all(Val::Px(4.)),
        flex_shrink: 0.,
        ..default()
    };
    let contents = |p: &mut ChildSpawnerCommands| {
        if item.is_none() {
            return;
        }
        p.spawn((
            ItemIcon(icon),
            ImageNode::default(),
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ));
        if let Some((item, _)) = item {
            let badge = if !caption.is_empty() {
                caption.to_owned()
            } else if item.listed {
                "Sale".into()
            } else if item.locked {
                "Locked".into()
            } else if item.slot.is_some() {
                "Worn".into()
            } else if item.level > 0 {
                format!("+{}", item.level)
            } else {
                String::new()
            };
            if !badge.is_empty() {
                let label = skin::label(p, font, badge, 10., skin::GOLD);
                p.commands().entity(label).insert((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(1.),
                        right: Val::Px(2.),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
                ));
            }
        }
    };
    let border = if selected { skin::GOLD } else { skin::EDGE };
    if let Some(action) = action {
        p.spawn((
            SlotFrame,
            ItemTip(item.map(|(item, _)| item.clone())),
            Button,
            action,
            skin::Control {
                primary: false,
                selected,
                enabled: true,
            },
            node,
            BackgroundColor(skin::SURFACE),
            BorderColor::all(border),
        ))
        .with_children(contents);
    } else {
        p.spawn((
            node,
            BackgroundColor(skin::SURFACE),
            BorderColor::all(border),
        ))
        .with_children(contents);
    }
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
    p.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.),
        ..default()
    })
    .with_children(|p| {
        p.spawn((
            ItemIcon(d.icon.clone()),
            ImageNode::default(),
            Node {
                width: Val::Px(42.),
                height: Val::Px(42.),
                flex_shrink: 0.,
                ..default()
            },
        ));
        skin::label(
            p,
            f,
            format!(
                "{}{}",
                d.name,
                if item.level > 0 {
                    format!(" +{}", item.level)
                } else {
                    String::new()
                }
            ),
            18.,
            item_color(d),
        );
    });
    skin::label(
        p,
        f,
        format!(
            "{}{}",
            d.armor
                .as_deref()
                .map_or(String::new(), |a| format!("{a} · ")),
            label_slot(&d.category)
        ),
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
    if !d.can_use(class) {
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
    let enabled = !gear.item_busy(item.id);
    if !enabled {
        skin::label(
            p,
            f,
            "Waiting for this item's action to be confirmed. See the connection status below.",
            11.,
            skin::MUTED,
        );
    } else if !gear.online() {
        skin::label(
            p,
            f,
            "The inventory server is offline. Item actions will be queued until it reconnects.",
            11.,
            skin::MUTED,
        );
    }
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
                );
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
            if let Some(owner) = item.equipped_class.filter(|&owner| owner != class) {
                skin::label(
                    p,
                    f,
                    format!(
                        "Equipped by {}. Switch to that class to unequip.",
                        class_name(owner)
                    ),
                    12.,
                    skin::MUTED,
                );
            } else {
                button(p, f, "Unequip", Action::Unequip, enabled && !item.listed);
            }
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
            );
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
fn zone_loot(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    catalog: &asr_coop::economy::Catalog,
    zone_name: &str,
    class: u8,
) {
    skin::label(p, f, format!("{zone_name} drops"), 18., skin::GOLD);
    let Some((zone_id, zone)) = catalog
        .zones
        .iter()
        .find(|(_, zone)| zone.name == zone_name)
    else {
        skin::label(
            p,
            f,
            "This zone has no equipment drop table yet.",
            12.,
            skin::MUTED,
        );
        return;
    };
    skin::label(p, f, format!(
        "Elites: {:.0}% · Bosses: {:.0}%\nAt most {} equipment drops per run. Normal enemies drop materials only.",
        zone.equipment_chance_bp.get("elite").copied().unwrap_or(0) as f32 / 100.,
        zone.equipment_chance_bp.get("boss").copied().unwrap_or(0) as f32 / 100.,
        asr_coop::economy::MAX_EQUIPMENT_PER_RUN,
    ), 12., skin::MUTED);
    skin::label(
        p,
        f,
        "Your class's loot pool · chances within a successful equipment drop",
        12.,
        skin::TEXT,
    );
    let pool: Vec<_> = zone
        .equipment
        .iter()
        .filter_map(|drop| {
            let item = catalog.definition(&drop.item)?;
            (item.zone == *zone_id && item.can_use(class) && drop.weight > 0)
                .then_some((item, drop.weight))
        })
        .collect();
    let total: u64 = pool.iter().map(|(_, weight)| u64::from(*weight)).sum();
    for (item, weight) in pool {
        p.spawn((
            Interaction::None,
            ItemTip(Some(Item {
                definition: item.id.clone(),
                ..Item::default()
            })),
            Node {
                column_gap: Val::Px(6.),
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                ItemIcon(item.icon.clone()),
                ImageNode::default(),
                Node {
                    width: Val::Px(28.),
                    height: Val::Px(28.),
                    flex_shrink: 0.,
                    ..default()
                },
            ));
            skin::label(
                row,
                f,
                format!(
                    "{} · {:.1}%",
                    item.name,
                    weight as f64 * 100. / total.max(1) as f64
                ),
                12.,
                skin::TEXT,
            );
        });
    }
}

fn pane_node(weight: f32) -> Node {
    Node {
        flex_grow: weight,
        flex_basis: Val::Px(0.),
        min_width: Val::Px(0.),
        min_height: Val::Px(0.),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.),
        ..default()
    }
}
fn scroller(key: usize) -> impl Bundle {
    (
        ScrollColumn(key),
        ScrollPosition::default(),
        bevy::ui::RelativeCursorPosition::default(),
        Node {
            flex_grow: 1.,
            min_height: Val::Px(0.),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            row_gap: Val::Px(6.),
            ..default()
        },
    )
}
fn bag_grid(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    hero: Option<&super::inventory::HeroSave>,
    market: bool,
) {
    let query = gear.search.to_lowercase();
    let items: Vec<_> = hero
        .into_iter()
        .flat_map(|h| &h.inventory.items)
        .filter_map(|i| {
            let d = gear.catalog.definition(&i.definition)?;
            let category = match gear.filter {
                1 => d.armor.as_deref() == Some("cloth"),
                2 => d.armor.as_deref() == Some("leather"),
                3 => d.armor.as_deref() == Some("mail"),
                4 => d.armor.as_deref() == Some("plate"),
                _ => true,
            };
            (!i.equipped_for(hero.map_or(0, |h| h.inventory.class))
                && (market
                    || (category
                        && slot_matches(gear.slot_filter, &d.slot)
                        && d.name.to_lowercase().contains(&query))))
            .then_some((i, d))
        })
        .collect();
    p.spawn(scroller(1)).with_children(|p| {
        p.spawn(Node {
            width: Val::Percent(100.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            column_gap: Val::Px(5.),
            row_gap: Val::Px(5.),
            ..default()
        })
        .with_children(|grid| {
            for &(item, d) in &items {
                item_slot(
                    grid,
                    f,
                    Some((item, d)),
                    Some(Action::Select(item.id)),
                    "",
                    gear.selected == Some(item.id),
                );
            }
            for _ in items.len()..BAG_MIN_SLOTS {
                item_slot(grid, f, None, None, "", false);
            }
        });
    });
    let label = gear.slot_filter.and_then(|i| SLOTS.get(i)).map_or_else(
        || format!("{} items in your bags", items.len()),
        |slot| format!("{} items fit {}", items.len(), slot_title(slot)),
    );
    row(p, |p| {
        skin::label(p, f, label, 11., skin::MUTED);
        if gear.slot_filter.is_some() {
            button(p, f, "All slots", Action::SlotFilter(None), true);
        }
    });
}
fn paper_doll(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    hero: Option<&super::inventory::HeroSave>,
    class: &super::data::ClassDef,
) {
    skin::label(p, f, "EQUIPMENT", 12., skin::MUTED);
    let equipped = |slot: &str| {
        hero.and_then(|h| {
            h.inventory
                .items
                .iter()
                .find(|i| i.slot.as_deref() == Some(slot) && i.equipped_for(class.id))
        })
        .and_then(|i| Some((i, gear.catalog.definition(&i.definition)?)))
    };
    let candidate = gear
        .selected
        .and_then(|id| hero?.inventory.items.iter().find(|i| i.id == id))
        .and_then(|i| gear.catalog.definition(&i.definition));
    p.spawn(Node {
        column_gap: Val::Px(8.),
        flex_shrink: 0.,
        margin: UiRect::top(Val::Px(12.)),
        ..default()
    })
    .with_children(|body| {
        let column = |p: &mut ChildSpawnerCommands, slots: &[&str]| {
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|p| {
                for slot in slots {
                    let item = equipped(slot);
                    doll_slot(
                        p,
                        f,
                        slot,
                        item,
                        item.is_some_and(|(i, _)| gear.selected == Some(i.id)),
                        gear.slot_filter
                            .and_then(|i| SLOTS.get(i))
                            .is_some_and(|s| s == slot)
                            || candidate.is_some_and(|d| d.fits(class.id, slot)),
                        true,
                    );
                }
            });
        };
        column(body, &DOLL_LEFT);
        body.spawn(Node {
            flex_grow: 1.,
            flex_basis: Val::Px(0.),
            min_width: Val::Px(0.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(10.),
            ..default()
        })
        .with_children(|center| {
            skin::label(center, f, class.name, 20., skin::GOLD);
            let count = hero.map_or(0, |h| {
                h.inventory
                    .items
                    .iter()
                    .filter(|i| i.equipped_for(class.id))
                    .count()
            });
            skin::label(
                center,
                f,
                format!("{count} of 19 slots equipped"),
                12.,
                skin::TEXT,
            );
            skin::rule(center);
            skin::label(center, f, "BONUSES", 12., skin::MUTED);
            if let Some(hero) = hero {
                skin::label(
                    center,
                    f,
                    amounts(&hero.inventory.bonuses(&gear.catalog)),
                    14.,
                    Color::srgb(0.52, 0.84, 0.38),
                );
            }
            center.spawn(Node {
                flex_grow: 1.,
                ..default()
            });
            center
                .spawn(Node {
                    column_gap: Val::Px(5.),
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .with_children(|p| {
                    for slot in DOLL_WEAPONS {
                        let item = equipped(slot);
                        doll_slot(
                            p,
                            f,
                            slot,
                            item,
                            item.is_some_and(|(i, _)| gear.selected == Some(i.id)),
                            candidate.is_some_and(|d| d.fits(class.id, slot)),
                            true,
                        );
                    }
                });
        });
        column(body, &DOLL_RIGHT);
    });
}
fn auction_list(p: &mut ChildSpawnerCommands, f: &Handle<Font>, gear: &Equipment) {
    let query = gear.search.to_lowercase();
    p.spawn(Node {
        column_gap: Val::Px(6.),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|p| {
        choice(p, f, "Browse", Action::Filter(0), gear.filter != 1);
        choice(p, f, "My Auctions", Action::Filter(1), gear.filter == 1);
    });
    p.spawn(Node {
        column_gap: Val::Px(7.),
        ..default()
    })
    .with_children(|p| {
        p.spawn(Node {
            flex_grow: 1.,
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "ITEM", 10., skin::MUTED);
        });
        p.spawn(Node {
            width: Val::Px(66.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "SELLER", 10., skin::MUTED);
        });
        p.spawn(Node {
            width: Val::Px(45.),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, "PRICE", 10., skin::MUTED);
        });
    });
    p.spawn(scroller(2)).with_children(|p| {
        for sale in gear
            .listings
            .iter()
            .filter(|sale| gear.filter != 1 || sale.seller == "self")
        {
            let Some(d) = gear.catalog.definition(&sale.item.definition) else {
                continue;
            };
            if !d.name.to_lowercase().contains(&query) {
                continue;
            }
            let selected = gear.listing == Some(sale.id);
            p.spawn((
                Button,
                Action::Listing(sale.id),
                ItemTip(Some(sale.item.clone())),
                skin::Control {
                    primary: false,
                    selected,
                    enabled: true,
                },
                Node {
                    width: Val::Percent(100.),
                    padding: UiRect::all(Val::Px(5.)),
                    column_gap: Val::Px(7.),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.,
                    border: UiRect::all(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(if selected {
                    Color::srgb(0.18, 0.055, 0.025)
                } else {
                    Color::NONE
                }),
                BorderColor::all(if selected { skin::GOLD } else { Color::NONE }),
            ))
            .with_children(|p| {
                p.spawn((
                    ItemIcon(d.icon.clone()),
                    ImageNode::default(),
                    Node {
                        width: Val::Px(30.),
                        height: Val::Px(30.),
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
                p.spawn(Node {
                    flex_grow: 1.,
                    flex_basis: Val::Px(0.),
                    min_width: Val::Px(0.),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|p| {
                    skin::label(
                        p,
                        f,
                        format!(
                            "{}{}",
                            d.name,
                            if sale.item.level > 0 {
                                format!(" +{}", sale.item.level)
                            } else {
                                String::new()
                            }
                        ),
                        12.,
                        skin::TEXT,
                    );
                    skin::label(p, f, label_slot(&d.category), 10., skin::MUTED);
                });
                p.spawn(Node {
                    width: Val::Px(66.),
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                })
                .with_children(|p| {
                    skin::label(p, f, &sale.seller_name, 11., skin::TEXT);
                });
                p.spawn(Node {
                    width: Val::Px(45.),
                    flex_shrink: 0.,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|p| {
                    skin::label(p, f, format!("{}g", sale.price), 12., skin::GOLD);
                });
            });
        }
        if gear.listings.is_empty() {
            skin::label(p, f, "No listings found.", 13., skin::MUTED);
        }
    });
}
fn search_box(p: &mut ChildSpawnerCommands, f: &Handle<Font>, gear: &Equipment) {
    p.spawn(Node {
        width: Val::Percent(100.),
        column_gap: Val::Px(5.),
        ..default()
    })
    .with_children(|p| {
        p.spawn((
            Button,
            Action::Search,
            skin::Control {
                primary: false,
                selected: gear.search_active,
                enabled: true,
            },
            Node {
                flex_grow: 1.,
                flex_basis: Val::Px(0.),
                min_width: Val::Px(0.),
                min_height: Val::Px(29.),
                padding: UiRect::axes(Val::Px(9.), Val::Px(6.)),
                overflow: Overflow::clip(),
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(3.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.031, 0.025)),
            BorderColor::all(if gear.search_active {
                skin::GOLD
            } else {
                skin::EDGE
            }),
        ))
        .with_children(|p| {
            let label = if gear.search.is_empty() {
                "Search"
            } else {
                &gear.search
            };
            let text = skin::label(
                p,
                f,
                label,
                13.,
                if gear.search.is_empty() {
                    skin::MUTED
                } else {
                    skin::TEXT
                },
            );
            super::text_edit::mark(p, text, 10, &gear.search_cursor, gear.search_active);
        });
        if !gear.search.is_empty() {
            button(p, f, "Clear", Action::ClearSearch, true);
        }
    });
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
    let shared_view = gear.view(&coop.url, class.id);
    let hero = shared_view.as_ref();
    commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.80)),
            GlobalZIndex(400),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(94.),
                    height: Val::Percent(90.),
                    max_width: Val::Px(1420.),
                    padding: UiRect::all(Val::Px(18.)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.),
                    border: UiRect::all(Val::Px(1.)),
                    border_radius: BorderRadius::all(Val::Px(7.)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0745, 0.0667, 0.051)),
                BorderColor::all(Color::srgb(0.43, 0.33, 0.145)),
            ))
            .with_children(|p| {
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.),
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|p| {
                    skin::label(
                        p,
                        f,
                        if gear.market {
                            "AUCTION HOUSE"
                        } else {
                            "INVENTORY"
                        },
                        24.,
                        skin::GOLD,
                    );
                    choice(p, f, "Inventory", Action::Tab(false), !gear.market);
                    choice(p, f, "Auction House", Action::Tab(true), gear.market);
                    button(p, f, "Zone drops", Action::LootTable, true);
                    p.spawn(Node {
                        flex_grow: 1.,
                        ..default()
                    });
                    skin::label(
                        p,
                        f,
                        format!("{} gold", hero.map_or(0, |h| h.inventory.gold)),
                        17.,
                        skin::GOLD,
                    );
                    button(p, f, "Close", Action::Close, true);
                });
                p.spawn(Node {
                    flex_grow: 1.,
                    min_height: Val::Px(0.),
                    column_gap: Val::Px(18.),
                    ..default()
                })
                .with_children(|body| {
                    if !gear.market {
                        body.spawn((
                            Node {
                                overflow: Overflow::scroll_y(),
                                ..pane_node(27.)
                            },
                            ScrollColumn(0),
                            ScrollPosition::default(),
                            bevy::ui::RelativeCursorPosition::default(),
                        ))
                        .with_children(|p| {
                            paper_doll(p, f, &gear, hero, class);
                        });
                    }
                    body.spawn(pane_node(if gear.market { 32. } else { 45. }))
                        .with_children(|p| {
                            if gear.market {
                                skin::label(
                                    p,
                                    f,
                                    "YOUR BAGS · select an item to sell",
                                    12.,
                                    skin::MUTED,
                                );
                            } else {
                                row(p, |p| {
                                    for (i, name) in ["All", "Cloth", "Leather", "Mail", "Plate"]
                                        .iter()
                                        .enumerate()
                                    {
                                        choice(p, f, name, Action::Filter(i), gear.filter == i);
                                    }
                                });
                                search_box(p, f, &gear);
                            }
                            bag_grid(p, f, &gear, hero, gear.market);
                        });
                    if gear.market {
                        body.spawn(pane_node(35.)).with_children(|p| {
                            search_box(p, f, &gear);
                            auction_list(p, f, &gear);
                        });
                    }
                    body.spawn((
                        pane_node(28.),
                        BackgroundColor(Color::srgb(0.035, 0.031, 0.022)),
                        BorderColor::all(Color::srgb(0.24, 0.20, 0.13)),
                    ))
                    .insert(Node {
                        padding: UiRect::all(Val::Px(12.)),
                        border: UiRect::all(Val::Px(1.)),
                        border_radius: BorderRadius::all(Val::Px(4.)),
                        ..pane_node(28.)
                    })
                    .with_children(|p| {
                        p.spawn(scroller(3)).with_children(|p| {
                            if gear.show_loot {
                                zone_loot(p, f, &gear.catalog, selection.map_def().name, class.id);
                            } else if let Some(sale) = gear
                                .listing
                                .and_then(|id| gear.listings.iter().find(|l| l.id == id))
                            {
                                detail(p, f, &gear, class.id, &sale.item, true);
                            } else if let Some(item) = gear
                                .selected
                                .and_then(|id| hero?.inventory.items.iter().find(|i| i.id == id))
                            {
                                detail(p, f, &gear, class.id, item, false);
                            } else {
                                skin::label(
                                    p,
                                    f,
                                    "Select an item to see its details.",
                                    15.,
                                    skin::GOLD,
                                );
                            }
                        });
                    });
                });
                row(p, |p| {
                    if let Some(hero) = hero {
                        for (name, count) in &hero.inventory.materials {
                            let glyph = match name.as_str() {
                                "cloth" => "INV_Fabric_Linen_01",
                                "leather" => "INV_Misc_LeatherScrap_03",
                                "iron" => "INV_Ingot_Iron",
                                "wood" => "INV_Misc_Branch_01",
                                _ => "INV_Misc_QuestionMark",
                            };
                            p.spawn((
                                ItemIcon(format!("Interface\\Icons\\{glyph}")),
                                ImageNode::default(),
                                Node {
                                    width: Val::Px(20.),
                                    height: Val::Px(20.),
                                    ..default()
                                },
                            ));
                            skin::label(p, f, format!("{count} {name}  "), 12., skin::TEXT);
                        }
                    }
                    button(p, f, "Refresh", Action::Refresh, !gear.busy());
                });
                skin::label(p, f, &gear.status, 10., skin::MUTED);
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
    selection: Res<Selection>,
    mut memory: ResMut<ScrollMemory>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut columns: Query<
        (
            Ref<ScrollColumn>,
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
    let query = (
        selection.class,
        gear.market,
        gear.filter,
        gear.slot_filter,
        gear.search.clone(),
    );
    let reset = memory.query.as_ref() != Some(&query);
    if reset {
        memory.positions = [Vec2::ZERO; 4];
        memory.query = Some(query);
    }
    for (column, mut scroll, node, cursor) in &mut columns {
        if column.is_added() || reset {
            scroll.0 = memory.positions[column.0];
        }
        // New nodes have no layout until the next frame; do not clamp a restored offset yet.
        if node.size.y <= 0. {
            continue;
        }
        if cursor
            .normalized
            .is_some_and(|p| (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y))
        {
            let limit = (node.content_size.y - node.size.y).max(0.) * node.inverse_scale_factor();
            scroll.0.y = (scroll.0.y - delta).clamp(0., limit);
        }
        memory.positions[column.0] = scroll.0;
    }
}

fn slot_colors(
    mut slots: Query<
        (
            &skin::Control,
            &Interaction,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        With<SlotFrame>,
    >,
) {
    for (style, interaction, mut border, mut background) in &mut slots {
        let hot = *interaction != Interaction::None;
        *border = BorderColor::all(if style.selected {
            skin::GOLD
        } else if hot {
            Color::srgb(0.69, 0.55, 0.24)
        } else {
            Color::srgb(0.28, 0.27, 0.28)
        });
        background.0 = Color::srgb(0.035, 0.032, 0.025);
    }
}
fn scrollbars(
    mut commands: Commands,
    added: Query<(Entity, &ScrollColumn), Added<ScrollColumn>>,
    columns: Query<(&ScrollColumn, &ScrollPosition, &ComputedNode)>,
    mut thumbs: Query<(&ScrollThumb, &mut Node)>,
) {
    for (entity, column) in &added {
        commands.entity(entity).with_children(|p| {
            p.spawn((
                ScrollThumb(column.0),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.),
                    width: Val::Px(3.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.56, 0.43, 0.18)),
                bevy::ui::FocusPolicy::Pass,
            ));
        });
    }
    for (thumb, mut node) in &mut thumbs {
        let Some((_, scroll, computed)) = columns.iter().find(|(column, ..)| column.0 == thumb.0)
        else {
            continue;
        };
        let scale = computed.inverse_scale_factor();
        let visible = computed.size.y * scale;
        let content = computed.content_size.y * scale;
        if visible <= 0. || content <= visible + 1. {
            node.display = Display::None;
            continue;
        }
        node.display = Display::Flex;
        let height = (visible * visible / content).max(16.).min(visible);
        let offset =
            scroll.0.y.clamp(0., content - visible) / (content - visible) * (visible - height);
        node.height = Val::Px(height);
        node.top = Val::Px(scroll.0.y + offset);
    }
}

fn tip_label(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    text: impl Into<String>,
    size: f32,
    color: Color,
) {
    let entity = skin::label(p, f, text, size, color);
    p.commands().entity(entity).insert(Node {
        width: Val::Percent(100.),
        min_width: Val::Px(0.),
        flex_shrink: 0.,
        ..default()
    });
}

fn tooltip_panel() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.),
            padding: UiRect::all(Val::Px(7.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.),
            flex_shrink: 0.,
            border: UiRect::all(Val::Px(1.)),
            border_radius: BorderRadius::all(Val::Px(3.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.012, 0.012, 0.97)),
        BorderColor::all(Color::srgb(0.46, 0.46, 0.46)),
        bevy::ui::FocusPolicy::Pass,
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn item_tooltip(
    mut commands: Commands,
    gear: Res<Equipment>,
    selection: Res<Selection>,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    slots: Query<(
        &ItemTip,
        &Interaction,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut tips: Query<(Entity, &ComputedNode, &mut Node, &mut Visibility), With<ItemTipRoot>>,
    mut shown: Local<Option<(u64, String, u8, u64, u8)>>,
) {
    let class = selection.class_def().map_or(0, |class| class.id);
    let hovered = slots
        .iter()
        .find(|(tip, interaction, ..)| tip.0.is_some() && **interaction != Interaction::None)
        .filter(|_| gear.open && *phase.get() == Phase::Menu);
    let Some((tip, _, size, at)) = hovered else {
        for (entity, ..) in &tips {
            commands.entity(entity).despawn();
        }
        *shown = None;
        return;
    };
    let Some(item) = &tip.0 else {
        return;
    };
    let Some(d) = gear.catalog.definition(&item.definition) else {
        return;
    };
    let Some(font) = font else {
        return;
    };
    let key = (
        item.id,
        item.definition.clone(),
        item.level,
        gear.revision,
        class,
    );
    if shown.as_ref() != Some(&key) || tips.is_empty() {
        for (entity, ..) in &tips {
            commands.entity(entity).despawn();
        }
        *shown = Some(key);
        commands
            .spawn((
                ItemTipRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(290.),
                    flex_shrink: 0.,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.),
                    ..default()
                },
                GlobalZIndex(1100),
                Visibility::Hidden,
                bevy::ui::FocusPolicy::Pass,
            ))
            .with_children(|root| {
                root.spawn(tooltip_panel()).with_children(|p| {
                    tip_label(
                        p,
                        &font.0,
                        format!(
                            "{}{}",
                            d.name,
                            if item.level > 0 {
                                format!(" +{}", item.level)
                            } else {
                                String::new()
                            }
                        ),
                        14.,
                        item_color(d),
                    );
                    if item.level > 0 {
                        tip_label(
                            p,
                            &font.0,
                            format!("Upgrade +{} of 5", item.level),
                            11.,
                            Color::srgb(1., 0.82, 0.),
                        );
                    }
                    p.spawn(Node {
                        width: Val::Percent(100.),
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(8.),
                        flex_shrink: 0.,
                        ..default()
                    })
                    .with_children(|p| {
                        skin::label(p, &font.0, slot_title(&d.slot), 12., Color::WHITE);
                        let kind = d.armor.as_deref().unwrap_or(&d.category);
                        let kind = kind
                            .split('_')
                            .map(|part| {
                                let mut chars = part.chars();
                                chars.next().map_or(String::new(), |first| {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                })
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        skin::label(p, &font.0, kind, 12., Color::WHITE);
                    });
                    for (stat, value) in d.bonuses(item.level) {
                        let mut one = std::collections::BTreeMap::new();
                        one.insert(stat.clone(), *value);
                        tip_label(
                            p,
                            &font.0,
                            amounts(&one),
                            12.,
                            if stat == "max_health" {
                                Color::WHITE
                            } else {
                                Color::srgb(0.12, 1., 0.)
                            },
                        );
                    }
                    let classes = d
                        .classes
                        .iter()
                        .copied()
                        .filter(|&class| d.can_use(class))
                        .map(class_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    tip_label(
                        p,
                        &font.0,
                        format!("Classes: {classes}"),
                        11.,
                        if d.can_use(class) {
                            Color::WHITE
                        } else {
                            Color::srgb(1., 0.2, 0.2)
                        },
                    );
                    if item.slot.is_some() {
                        tip_label(
                            p,
                            &font.0,
                            format!(
                                "Equipped by {}",
                                class_name(item.equipped_class.unwrap_or(class))
                            ),
                            11.,
                            skin::GOLD,
                        );
                    }
                    if item.locked {
                        tip_label(p, &font.0, "Locked", 11., skin::GOLD);
                    }
                    if item.listed {
                        tip_label(p, &font.0, "On the Auction House", 11., skin::GOLD);
                    }
                });
                let zone = gear
                    .catalog
                    .zones
                    .get(&d.zone)
                    .map_or(d.zone.as_str(), |zone| zone.name.as_str());
                root.spawn(tooltip_panel()).with_children(|p| {
                    p.spawn((
                        Text::new("- Drops from elites and bosses in "),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 11.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.65, 0.65, 0.65)),
                        Node {
                            width: Val::Percent(100.),
                            min_width: Val::Px(0.),
                            flex_shrink: 0.,
                            ..default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            TextSpan::new(zone),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 11.,
                                ..default()
                            },
                            TextColor(Color::srgb(0., 0.65, 1.)),
                        ));
                    });
                });
            });
        return;
    }
    let inv = size.inverse_scale_factor();
    for (_, computed, mut node, mut visibility) in &mut tips {
        if computed.size.y <= 0. {
            continue;
        }
        let tip_size = computed.size() * inv;
        let mut x = (at.translation.x + size.size.x / 2.) * inv + 6.;
        let mut y = (at.translation.y - size.size.y / 2.) * inv;
        if let Ok(window) = windows.single() {
            let viewport = Vec2::new(
                window.physical_width() as f32,
                window.physical_height() as f32,
            ) * inv;
            if x + tip_size.x > viewport.x - 4. {
                x = (at.translation.x - size.size.x / 2.) * inv - tip_size.x - 6.;
            }
            x = x.clamp(4., (viewport.x - tip_size.x - 4.).max(4.));
            y = y.clamp(4., (viewport.y - tip_size.y - 4.).max(4.));
        }
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        *visibility = Visibility::Inherited;
    }
}
