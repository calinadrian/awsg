//! Inventory, equipment and shared auctions in the character menu.
use super::{
    coop::Coop,
    front_end::{self as skin, gap, size},
    hud::HudFont,
    inventory::{Equipment, Pane},
    Phase, Selection,
};
use crate::glue::GlueClicks;
use asr_coop::economy::{Action as Trade, Definition, Item, SLOTS};
use bevy::input::{keyboard::KeyboardInput, ButtonState};
use bevy::prelude::*;

#[derive(Component, Clone)]
pub(super) enum Action {
    Open(Pane),
    Close,
    Tab(Pane),
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
    Redeem(bool),
    LootClass(Option<u8>),
    LootSlot,
    LootQuality,
    LootSource,
    Search,
    ClearSearch,
    /// Keep the bags in the solo economy (true) or on the online realm (false).
    Realm(bool),
    /// Open a quest in the quest log.
    QuestPick(String),
    /// Offer quests from one economy zone, or from all the hero's zones.
    QuestZone(Option<String>),
    Accept(String),
    Abandon(String),
    TurnIn(String),
    /// Show an achievement category; 0 is the summary.
    Category(usize),
}
#[derive(Component)]
struct Root;
#[derive(Component)]
pub(super) struct ItemIcon(pub String);
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
    positions: [Vec2; 8],
    #[allow(clippy::type_complexity)]
    query: Option<(Option<usize>, Pane, usize, Option<usize>, String, (Option<u8>, Option<usize>, usize, usize), (Option<String>, usize))>,
}
const BAG_MIN_SLOTS: usize = 72;
/// A bag slot's target size and the gap between slots, in canvas units.
const BAG_CELL: f32 = 46.;
const BAG_GAP: f32 = 5.;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ScrollMemory>();
    if let Some(wanted) = Preview::from_env() {
        app.insert_resource(wanted).add_systems(Update, preview.before(draw));
    }
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
            border_radius: BorderRadius::all(Val::Px(3.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
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
pub(super) fn choice(
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

fn spell_title(key: &str) -> String {
    key.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |head| {
                head.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    let view = gear.view(&gear.realm(&coop), class);
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
        Action::Open(pane) => {
            gear.open = true;
            gear.pane = pane;
            gear.confirm = None;
            gear.slot_filter = None;
            gear.filter = 0;
            gear.search.clear();
            if pane == Pane::Drops {
                gear.loot_class = Some(class);
            }
            open_journal(&mut gear, &selection, pane);
        }
        Action::Close => {
            gear.open = false;
            gear.confirm = None;
        }
        Action::Realm(solo) => gear.set_solo(solo),
        Action::Tab(pane) => {
            gear.pane = pane;
            gear.filter = 0;
            gear.confirm = None;
            gear.selected = None;
            gear.listing = None;
            gear.search.clear();
            if pane == Pane::Drops {
                gear.loot_class = Some(class);
            }
            open_journal(&mut gear, &selection, pane);
        }
        Action::QuestPick(id) => gear.quest = Some(id),
        Action::QuestZone(zone) => {
            gear.quest_zone = zone;
            gear.quest = None;
        }
        Action::Accept(quest) => {
            gear.quest = Some(quest.clone());
            trade = Some(Trade::AcceptQuest { quest });
        }
        Action::Abandon(quest) => trade = Some(Trade::AbandonQuest { quest }),
        Action::TurnIn(quest) => trade = Some(Trade::TurnInQuest { quest }),
        Action::Category(category) => gear.category = category,
        Action::Select(id) => {
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
        Action::Redeem(gear_reward) => trade = Some(Trade::RedeemAchievement { gear: gear_reward }),
        Action::LootClass(class) => gear.loot_class = class,
        Action::LootSlot => {
            gear.loot_slot = match gear.loot_slot {
                None => Some(0),
                Some(index) if index + 1 < SLOTS.len() => Some(index + 1),
                _ => None,
            };
        }
        Action::LootQuality => gear.loot_quality = (gear.loot_quality + 1) % 5,
        Action::LootSource => gear.loot_source = (gear.loot_source + 1) % 3,
        Action::Listing(id) => {
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
            let url = gear.realm(&coop);
            gear.action(&url, class, trade);
        }
    }
    gear.revision += 1;
}
/// `WOW_SURVIVORS_ACCOUNT_PREVIEW=quests,achievements` opens the account window on each named
/// tab in turn once the menu has settled, for layout checks, and closes it after the last; with
/// `WOW_SURVIVORS_SHOTS=<dir>` each tab is saved there. It takes a few of the zone's quests
/// first so the log has something in it: point `BENILLA_HOME` at a scratch folder.
#[derive(Resource)]
struct Preview {
    panes: Vec<(Pane, String)>,
    index: usize,
    t: f32,
    shots: Option<std::path::PathBuf>,
    opened: bool,
    accepted: bool,
}
impl Preview {
    fn from_env() -> Option<Self> {
        let want = crate::webenv::var("WOW_SURVIVORS_ACCOUNT_PREVIEW")?;
        let panes = want
            .split(',')
            .filter_map(|name| {
                let name = name.trim().to_ascii_lowercase();
                let pane = match name.as_str() {
                    "inventory" => Pane::Inventory,
                    "auction" => Pane::Auction,
                    "drops" => Pane::Drops,
                    "quests" => Pane::Quests,
                    "achievements" => Pane::Achievements,
                    _ => return None,
                };
                Some((pane, name))
            })
            .collect();
        Some(Self {
            panes,
            index: 0,
            t: -3.,
            shots: std::env::var_os("WOW_SURVIVORS_SHOTS").map(std::path::PathBuf::from),
            opened: false,
            accepted: false,
        })
    }
}
#[allow(clippy::too_many_arguments)]
fn preview(
    mut commands: Commands,
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    mut gear: ResMut<Equipment>,
    mut preview: ResMut<Preview>,
) {
    if *phase.get() != Phase::Menu || font.is_none() || preview.index > preview.panes.len() {
        return;
    }
    let Some(class) = selection.class_def().map(|c| c.id) else { return };
    preview.t += time.delta_secs();
    if preview.t < 0. {
        return;
    }
    let Some((pane, name)) = preview.panes.get(preview.index).cloned() else {
        gear.open = false;
        gear.revision += 1;
        preview.index += 1;
        return;
    };
    if !preview.opened {
        preview.opened = true;
        gear.open = true;
        gear.pane = pane;
        open_journal(&mut gear, &selection, pane);
        gear.revision += 1;
    }
    if !preview.accepted && gear.view(&gear.realm(&coop), class).is_some() {
        preview.accepted = true;
        let url = gear.realm(&coop);
        let zone = super::quests::zone_id(&gear.catalog, selection.stage_def().name).map(str::to_owned);
        let quests: Vec<_> = gear
            .catalog
            .quests
            .quests
            .iter()
            .filter(|q| Some(&q.zone) == zone.as_ref() && q.requires.is_empty())
            .take(3)
            .map(|q| q.id.clone())
            .collect();
        for quest in quests {
            gear.action(&url, class, Trade::AcceptQuest { quest });
        }
    }
    if preview.t >= 4. {
        if let Some(dir) = &preview.shots {
            let _ = std::fs::create_dir_all(dir);
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(dir.join(format!("account-{name}.png"))));
        }
        preview.index += 1;
        preview.t = 0.;
        preview.opened = false;
    }
}
/// Opening the quest log starts on the zone the hero is set to play; achievements on the
/// summary.
fn open_journal(gear: &mut Equipment, selection: &Selection, pane: Pane) {
    match pane {
        Pane::Quests => {
            gear.quest = None;
            gear.quest_zone = super::quests::zone_id(&gear.catalog, selection.map_def().name)
                .filter(|zone| journal_zones(gear, selection).any(|(id, _)| id == zone))
                .map(str::to_owned);
        }
        Pane::Achievements => gear.category = 0,
        _ => {}
    }
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
        width: Val::Percent(100.),
        aspect_ratio: Some(1.),
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
    for (spell, value) in &d.spell_damage_bp {
        skin::label(
            p,
            f,
            format!("+{:.0}% {} damage", *value as f32 / 100., spell_title(spell)),
            13.,
            Color::srgb(0.45, 0.67, 1.0),
        );
    }
    if let Some(set_name) = &d.set_name {
        if let Some(set) = gear.catalog.sets.get(set_name) {
            skin::label(p, f, format!("{set_name} · {}-piece bonus", set.pieces), 13., skin::GOLD);
            for (spell, value) in &set.spell_damage_bp {
                skin::label(p, f, format!("+{:.0}% {} damage", *value as f32 / 100., spell_title(spell)), 12., skin::MUTED);
            }
        }
    }
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
        if gear.pane == Pane::Auction {
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
/// The filters' column beside the drop list.
const DROPS_FILTERS: f32 = 300.;
fn zone_loot(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    zone_name: &str,
) {
    let catalog = &gear.catalog;
    let zone = catalog.zones.values().find(|zone| zone.name == zone_name);
    p.spawn(Node {
        flex_grow: 1.,
        min_height: Val::Px(0.),
        column_gap: Val::Px(PANE_GAP),
        ..default()
    })
    .with_children(|p| {
        // Left: what the zone drops at all, and the filters.
        p.spawn(Node {
            width: Val::Px(DROPS_FILTERS),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap::S),
            overflow: Overflow::clip(),
            ..default()
        })
        .with_children(|p| {
            skin::label(p, f, zone_name, size::HEADING, skin::GOLD);
            let Some(zone) = zone else {
                skin::label(p, f, "This zone has no equipment drop table yet.", size::SMALL, skin::MUTED);
                return;
            };
            p.spawn(Node {
                column_gap: Val::Px(6.),
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(gap::XS),
                ..default()
            })
            .with_children(|p| {
                let chance = |kind: &str| zone.equipment_chance_bp.get(kind).copied().unwrap_or(0) as f32 / 100.;
                skin::chip(p, f, format!("Elite {:.0}%", chance("elite")), skin::TEXT);
                skin::chip(p, f, format!("Boss {:.0}%", chance("boss")), skin::GOLD);
            });
            skin::label(
                p,
                f,
                format!(
                    "Up to {} elite drops a run. Bosses roll their own item separately.",
                    asr_coop::economy::MAX_EQUIPMENT_PER_RUN
                ),
                size::SMALL,
                skin::MUTED,
            );
            skin::rule(p);
            skin::label(p, f, "CLASS", size::CAPTION, skin::MUTED);
            p.spawn(Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.),
                column_gap: Val::Px(gap::XS),
                row_gap: Val::Px(gap::XS),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|p| {
                choice(p, f, "All classes", Action::LootClass(None), gear.loot_class.is_none());
                for class in super::data::CLASSES {
                    choice(p, f, class.name, Action::LootClass(Some(class.id)), gear.loot_class == Some(class.id));
                }
            });
            skin::label(p, f, "SHOW", size::CAPTION, skin::MUTED);
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gap::XS),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|p| {
                let slot = gear.loot_slot.and_then(|i| SLOTS.get(i)).map_or("All slots".into(), |slot| label_slot(slot));
                button(p, f, &format!("Slot: {slot}"), Action::LootSlot, true);
                button(p, f, ["Quality: All", "Quality: Common", "Quality: Uncommon", "Quality: Rare", "Quality: Epic"][gear.loot_quality], Action::LootQuality, true);
                button(p, f, ["Source: All", "Source: Elites", "Source: Bosses"][gear.loot_source], Action::LootSource, true);
            });
            if let Some(map) = super::data::MAPS.iter().position(|map| map.name == zone_name) {
                let names = super::data::roster(map).guardians.iter().map(|elite| elite.name).collect::<Vec<_>>();
                if !names.is_empty() {
                    skin::rule(p);
                    skin::label(p, f, "ELITES", size::CAPTION, skin::MUTED);
                    skin::label(p, f, names.join(", "), size::SMALL, skin::TEXT);
                }
            }
        });
        // Right: the matching items, two to a row.
        p.spawn(pane_node(1.)).with_children(|p| {
            let Some(zone) = zone else { return };
            let eligible = |item: &Definition| gear.loot_class.is_none_or(|class| item.can_use(class));
            let total: u64 = zone.equipment.iter().filter_map(|drop| {
                let item = catalog.definition(&drop.item)?;
                eligible(item).then_some(u64::from(drop.weight))
            }).sum();
            let mut items: Vec<_> = zone.equipment.iter().map(|drop| drop.item.as_str())
                .chain(zone.boss_drops.iter().map(|drop| drop.item.as_str()))
                .filter_map(|id| catalog.definition(id)).filter(|item| eligible(item)).collect();
            items.sort_by(|a, b| a.slot.cmp(&b.slot).then_with(|| a.name.cmp(&b.name)));
            items.dedup_by(|a, b| a.id == b.id);
            let quality = ["", "common", "uncommon", "rare", "epic"][gear.loot_quality];
            items.retain(|item| {
                let elite = zone.equipment.iter().any(|drop| drop.item == item.id && drop.weight > 0);
                let boss = zone.boss_drops.iter().any(|drop| drop.item == item.id);
                slot_matches(gear.loot_slot, &item.slot)
                    && (quality.is_empty() || item.quality == quality)
                    && match gear.loot_source { 1 => elite, 2 => boss, _ => true }
            });
            skin::label(
                p,
                f,
                format!("{}  ·  hover an item for its full tooltip", skin::plural(items.len() as u32, "item")),
                size::SMALL,
                skin::MUTED,
            );
            p.spawn(scroller(4)).with_children(|p| {
                if items.is_empty() {
                    skin::label(p, f, "No drops match these filters.", size::BODY, skin::TEXT);
                    return;
                }
                p.spawn(Node {
                    width: Val::Percent(100.),
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(2, 1.),
                    column_gap: Val::Px(gap::S),
                    row_gap: Val::Px(gap::S),
                    padding: UiRect::right(Val::Px(gap::S)),
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|p| {
                    for item in items {
                        let weight: u64 = zone.equipment.iter().filter(|drop| drop.item == item.id)
                            .map(|drop| u64::from(drop.weight)).sum();
                        let bosses: Vec<_> = zone.boss_drops.iter().filter(|drop| drop.item == item.id)
                            .map(|drop| drop.boss.as_str()).collect();
                        let color = item_color(item);
                        p.spawn((
                            Interaction::None,
                            ItemTip(Some(Item { definition: item.id.clone(), ..Item::default() })),
                            Node {
                                min_width: Val::Px(0.),
                                padding: UiRect::all(Val::Px(gap::S + 1.)),
                                border: UiRect::left(Val::Px(3.)),
                                border_radius: BorderRadius::all(Val::Px(4.)),
                                column_gap: Val::Px(10.),
                                align_items: AlignItems::Start,
                                ..default()
                            },
                            BackgroundColor(skin::SURFACE),
                            BorderColor::all(color),
                        )).with_children(|row| {
                            row.spawn((
                                ItemIcon(item.icon.clone()),
                                ImageNode::default(),
                                Node { width: Val::Px(36.), height: Val::Px(36.), flex_shrink: 0., ..default() },
                            ));
                            row.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.,
                                min_width: Val::Px(0.),
                                row_gap: Val::Px(3.),
                                ..default()
                            }).with_children(|text| {
                                skin::label(text, f, &item.name, size::BODY, color);
                                skin::label(text, f, format!("{}  ·  {}", label_slot(&item.slot), item.quality), size::CAPTION, skin::MUTED);
                                skin::label(text, f, amounts(&item.base_bonuses).replace('\n', "  ·  "), size::SMALL, skin::GREEN);
                                for (spell, value) in &item.spell_damage_bp {
                                    skin::label(text, f, format!("+{:.0}% {} damage", *value as f32 / 100., spell_title(spell)), size::SMALL, color);
                                }
                                if let Some(set) = &item.set_name {
                                    skin::label(text, f, format!("Set: {set}"), size::SMALL, skin::GOLD);
                                }
                                if weight > 0 {
                                    skin::label(text, f, format!("Any elite  ·  {:.1}% of elite gear rolls", weight as f64 * 100. / total.max(1) as f64), size::CAPTION, skin::MUTED);
                                }
                                if !bosses.is_empty() {
                                    skin::label(text, f, format!("Boss: {}", bosses.join(", ")), size::SMALL, skin::GOLD);
                                }
                            });
                        });
                    }
                });
            });
        });
    });
}

// The zones the shown race may enter, with their economy id and table.
pub(super) fn journal_zones<'a>(
    gear: &'a Equipment,
    selection: &Selection,
) -> impl Iterator<Item = (&'a String, &'a asr_coop::economy::Zone)> {
    let race = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(1, |look| look.1);
    super::data::MAPS
        .iter()
        .filter(move |map| super::races::can_enter(race, map.name))
        .filter_map(move |map| gear.catalog.zones.iter().find(|(_, zone)| zone.name == map.name))
}
pub(super) fn pane_node(weight: f32) -> Node {
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
pub(super) fn scroller(key: usize) -> impl Bundle {
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
/// How many bag columns fit `width` canvas units at about [`BAG_CELL`] each.
fn bag_columns(width: f32) -> usize {
    ((width + BAG_GAP) / (BAG_CELL + BAG_GAP)).floor().max(4.) as usize
}
fn bag_grid(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    hero: Option<&super::inventory::HeroSave>,
    market: bool,
    columns: usize,
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
    // Whole rows only: the empty slots round the bag up to a full last row.
    let slots = items.len().max(BAG_MIN_SLOTS).div_ceil(columns) * columns;
    p.spawn(scroller(1)).with_children(|p| {
        p.spawn(Node {
            width: Val::Percent(100.),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(columns as u16, 1.),
            flex_shrink: 0.,
            column_gap: Val::Px(BAG_GAP),
            row_gap: Val::Px(BAG_GAP),
            padding: UiRect::right(Val::Px(8.)),
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
            for _ in items.len()..slots {
                item_slot(grid, f, None, None, "", false);
            }
        });
    });
    let label = gear.slot_filter.and_then(|i| SLOTS.get(i)).map_or_else(
        || format!("{} items in your bags", items.len()),
        |slot| format!("{} items fit {}", items.len(), slot_title(slot)),
    );
    row(p, |p| {
        skin::label(p, f, label, size::SMALL, skin::MUTED);
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
                "Search items by name..."
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
/// The account window's tabs, in bar order.
const PANES: [(Pane, &str); 5] = [
    (Pane::Inventory, "Inventory"),
    (Pane::Auction, "Auction House"),
    (Pane::Drops, "Zone Drops"),
    (Pane::Quests, "Quests"),
    (Pane::Achievements, "Achievements"),
];
/// The window's frame inside the canvas: 94% wide up to this, padded by [`WINDOW_PAD`].
const WINDOW_MAX: f32 = 1420.;
const WINDOW_PAD: f32 = 18.;
const PANE_GAP: f32 = 18.;
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    gear: Res<Equipment>,
    selection: Res<Selection>,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    roots: Query<Entity, With<Root>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut last: Local<Option<(u64, u32, bool, u32)>>,
) {
    let show = gear.open && *phase.get() == Phase::Menu;
    let canvas = windows
        .single()
        .map_or(1365., |w| w.width() / ui_scale.0.max(0.01));
    let key = (gear.revision, selection.version, show, canvas as u32);
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
    let realm = gear.realm(&coop);
    let solo = realm == super::inventory::SOLO;
    let shared_view = gear.view(&realm, class.id);
    let hero = shared_view.as_ref();
    // The panes share the window's inner width by weight, so the bags know their columns.
    let inner = (canvas * 0.94).min(WINDOW_MAX) - 2. * WINDOW_PAD - 2.;
    let share = |weight: f32, total: f32, panes: f32| (inner - PANE_GAP * (panes - 1.)) * weight / total;
    let pane = gear.pane;
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
            root.spawn(skin::panel(Node {
                width: Val::Percent(94.),
                height: Val::Percent(90.),
                max_width: Val::Px(WINDOW_MAX),
                padding: UiRect::all(Val::Px(WINDOW_PAD)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gap::M),
                ..default()
            }))
            .with_children(|p| {
                // The tab bar never moves: no per-tab title in front of it.
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(gap::S),
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|p| {
                    for (tab, title) in PANES {
                        skin::tab_state(p, f, title, Action::Tab(tab), pane == tab);
                    }
                    p.spawn(Node {
                        flex_grow: 1.,
                        ..default()
                    });
                    // Which economy these bags are: the player's own, or the shared realm.
                    if cfg!(target_arch = "wasm32") {
                        skin::chip(p, f, "Solo", skin::GREEN);
                    } else {
                        let solo = gear.solo();
                        skin::tab_state(p, f, "Solo", Action::Realm(true), solo);
                        skin::tab_state(p, f, "Online", Action::Realm(false), !solo);
                    }
                    p.spawn(Node {
                        width: Val::Px(gap::M),
                        ..default()
                    });
                    skin::label(
                        p,
                        f,
                        format!(
                            "Gear score {}  ·  {} gold  ·  {} AP",
                            hero.map_or(0, |h| h.inventory.gear_score(&gear.catalog, class.id)),
                            hero.map_or(0, |h| h.inventory.gold),
                            hero.map_or(0, |h| h.inventory.achievement_points)
                        ),
                        size::LEAD,
                        skin::GOLD,
                    );
                    skin::tab(p, f, "Close", Action::Close);
                });
                skin::rule(p);
                p.spawn(Node {
                    flex_grow: 1.,
                    min_height: Val::Px(0.),
                    column_gap: Val::Px(PANE_GAP),
                    ..default()
                })
                .with_children(|body| match pane {
                    Pane::Auction if solo => {
                        body.spawn(Node {
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..pane_node(1.)
                        })
                        .with_children(|p| {
                            p.spawn(skin::panel(Node {
                                max_width: Val::Px(520.),
                                padding: UiRect::all(Val::Px(gap::XL)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(gap::M),
                                ..default()
                            }))
                            .with_children(|p| {
                                skin::label(p, f, "The Auction House is for online realms", size::HEADING, skin::GOLD);
                                let note = skin::label(
                                    p,
                                    f,
                                    "Solo heroes keep everything they find; trading needs other players. Your solo bags, drops, quests and achievements are saved on this device.",
                                    size::BODY,
                                    skin::TEXT,
                                );
                                p.commands().entity(note).insert(TextLayout::new_with_justify(Justify::Center));
                                if !cfg!(target_arch = "wasm32") {
                                    skin::tab(p, f, "Play online instead", Action::Realm(false));
                                }
                            });
                        });
                    }
                    Pane::Drops => {
                        body.spawn(pane_node(1.)).with_children(|p| {
                            zone_loot(p, f, &gear, selection.map_def().name);
                        });
                    }
                    Pane::Quests => {
                        super::journal_ui::quests(body, f, &gear, &selection, hero.map(|hero| &hero.inventory));
                    }
                    Pane::Achievements => {
                        super::journal_ui::achievements(body, f, &gear, hero.map(|hero| &hero.inventory), inner);
                    }
                    Pane::Inventory | Pane::Auction => {
                        let market = pane == Pane::Auction;
                        if !market {
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
                        let bags = if market { share(32., 95., 3.) } else { share(45., 100., 3.) };
                        body.spawn(pane_node(if market { 32. } else { 45. }))
                            .with_children(|p| {
                                if market {
                                    skin::label(
                                        p,
                                        f,
                                        "YOUR BAGS  ·  select an item to sell",
                                        size::SMALL,
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
                                bag_grid(p, f, &gear, hero, market, bag_columns(bags - 8.));
                            });
                        if market {
                            body.spawn(pane_node(35.)).with_children(|p| {
                                search_box(p, f, &gear);
                                auction_list(p, f, &gear);
                            });
                        }
                        body.spawn((
                            Node {
                                padding: UiRect::all(Val::Px(gap::M)),
                                border: UiRect::all(Val::Px(1.)),
                                border_radius: BorderRadius::all(Val::Px(4.)),
                                ..pane_node(28.)
                            },
                            BackgroundColor(skin::WELL),
                            BorderColor::all(skin::EDGE),
                        ))
                        .with_children(|p| {
                            p.spawn(scroller(3)).with_children(|p| {
                                if let Some(sale) = gear
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
                                        size::BODY,
                                        skin::MUTED,
                                    );
                                }
                            });
                        });
                    }
                });
                // Materials and the server's state belong to the bags; the other tabs go
                // without the footer.
                if matches!(pane, Pane::Inventory | Pane::Auction) {
                    skin::rule(p);
                    p.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(gap::L),
                        flex_shrink: 0.,
                        ..default()
                    })
                    .with_children(|p| {
                        if let Some(hero) = hero {
                            for (name, count) in &hero.inventory.materials {
                                p.spawn(Node {
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(6.),
                                    ..default()
                                })
                                .with_children(|p| {
                                    p.spawn((
                                        ItemIcon(format!("Interface\\Icons\\{}", material_icon(name))),
                                        ImageNode::default(),
                                        Node {
                                            width: Val::Px(20.),
                                            height: Val::Px(20.),
                                            ..default()
                                        },
                                    ));
                                    skin::label(p, f, format!("{count} {name}"), size::SMALL, skin::TEXT);
                                });
                            }
                        }
                        p.spawn(Node {
                            flex_grow: 1.,
                            ..default()
                        });
                        skin::label(p, f, &gear.status, size::CAPTION, skin::MUTED);
                        button(p, f, "Refresh", Action::Refresh, !gear.busy());
                    });
                }
            });
        });
}
pub(super) fn material_icon(name: &str) -> &'static str {
    match name {
        "cloth" => "INV_Fabric_Linen_01",
        "leather" => "INV_Misc_LeatherScrap_03",
        "iron" => "INV_Ingot_Iron",
        "wood" => "INV_Misc_Branch_01",
        _ => "INV_Misc_QuestionMark",
    }
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
        gear.pane,
        gear.filter,
        gear.slot_filter,
        gear.search.clone(),
        (gear.loot_class, gear.loot_slot, gear.loot_quality, gear.loot_source),
        (gear.quest_zone.clone(), gear.category),
    );
    let reset = memory.query.as_ref() != Some(&query);
    if reset {
        memory.positions = [Vec2::ZERO; 8];
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

/// A bag item held against what the hero wears in its slot: the weaker of a pair of rings or
/// trinkets, since that is the one it would replace.
fn compare(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    gear: &Equipment,
    coop: &Coop,
    class: u8,
    item: &Item,
    d: &Definition,
) {
    let Some(hero) = gear.view(&gear.realm(coop), class) else {
        return;
    };
    let worn = hero
        .inventory
        .items
        .iter()
        .filter(|i| i.equipped_for(class))
        .filter_map(|i| gear.catalog.definition(&i.definition).map(|w| (i, w)))
        .filter(|(_, w)| w.slot == d.slot)
        .min_by_key(|(i, w)| w.score(&gear.catalog, i.level));
    let (old, old_score) = worn.map_or((Default::default(), 0), |(i, w)| {
        (w.bonuses(i.level).clone(), w.score(&gear.catalog, i.level))
    });
    let new = d.bonuses(item.level);
    tip_label(
        p,
        f,
        worn.map_or("Fills an empty slot".into(), |(i, w)| {
            format!(
                "Instead of {}{}",
                w.name,
                if i.level > 0 { format!(" +{}", i.level) } else { String::new() }
            )
        }),
        11.,
        skin::MUTED,
    );
    let score = i64::from(d.score(&gear.catalog, item.level)) - i64::from(old_score);
    let mut lines = vec![("gear_score".to_string(), score)];
    for stat in new.keys().chain(old.keys()).collect::<std::collections::BTreeSet<_>>() {
        let delta = i64::from(new.get(stat).copied().unwrap_or(0))
            - i64::from(old.get(stat).copied().unwrap_or(0));
        lines.push((stat.clone(), delta));
    }
    for (stat, delta) in lines.into_iter().filter(|(_, delta)| *delta != 0) {
        let text = if stat == "gear_score" {
            format!("{delta:+} Gear score")
        } else {
            let mut one = std::collections::BTreeMap::new();
            one.insert(stat, delta.unsigned_abs() as u32);
            let text = amounts(&one);
            if delta < 0 { text.replacen('+', "-", 1) } else { text }
        };
        let color = if delta > 0 { Color::srgb(0.12, 1., 0.) } else { Color::srgb(1., 0.2, 0.2) };
        tip_label(p, f, text, 11., color);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn item_tooltip(
    mut commands: Commands,
    gear: Res<Equipment>,
    coop: Res<Coop>,
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
                    tip_label(
                        p,
                        &font.0,
                        if item.level > 0 {
                            format!(
                                "Item Level {}  ·  Upgrade +{} of 5",
                                d.item_level(&gear.catalog, item.level),
                                item.level
                            )
                        } else {
                            format!("Item Level {}", d.item_level(&gear.catalog, 0))
                        },
                        11.,
                        Color::srgb(1., 0.82, 0.),
                    );
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
                    if item.slot.is_none() && d.can_use(class) {
                        compare(p, &font.0, &gear, &coop, class, item, d);
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
