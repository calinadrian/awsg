//! A short player flow: host a party, or join friends with a six-character code.
use super::{coop::Coop, data, front_end as skin, hud::HudFont, Phase, Selection};
use crate::{
    glue::GlueClicks,
    textinput::{wayland_display, HostClipboard},
};
use asr_coop::Client;
use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct ScrollPanel;
#[derive(Component)]
pub(super) struct OpenCoop;
#[derive(Component)]
struct PartyText;
#[derive(Clone, Copy, Default, PartialEq, Debug)]
enum Page {
    #[default]
    Home,
    Join,
    Browse,
}
#[derive(Component, Clone)]
enum Action {
    Host,
    JoinCode,
    Join(String),
    Page(Page),
    Settings,
    Connect,
    Private,
    Ready,
    Start,
    Leave,
    Close,
    Field(u8),
    Copy,
    Refresh,
}
#[derive(Clone)]
enum Pending {
    Host,
    Join(String),
    Browse,
}
#[derive(Resource, Default)]
struct Editor {
    page: Page,
    settings: bool,
    field: Option<u8>,
    selected: bool,
    signature: String,
    pending: Option<Pending>,
    room_seen: String,
    copied_until: f32,
    dirty: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Editor>()
        .add_systems(Update, skin::hover.after(render))
        .add_systems(Startup, setup.after(super::hud::setup_hud))
        .add_systems(
            Update,
            (toggle, controls, render, scroll_dialog)
                .chain()
                .after(super::hud::show_panels),
        );
}
fn setup(mut commands: Commands, font: Res<HudFont>) {
    commands.spawn((
        PartyText,
        Visibility::Hidden,
        Text::new(""),
        TextFont {
            font: font.0.clone(),
            font_size: 12.,
            ..default()
        },
        TextColor(skin::TEXT),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(22.),
            top: Val::Px(110.),
            ..default()
        },
        GlobalZIndex(450),
    ));
}
fn toggle(
    inventory: Res<super::inventory::Equipment>,
    buttons: Query<Entity, With<OpenCoop>>,
    clicks: Res<GlueClicks>,
    mut coop: ResMut<Coop>,
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !inventory.open && *phase.get() == Phase::Menu
        && (buttons.iter().any(|e| clicks.hit(e)) || keys.just_pressed(KeyCode::F6))
    {
        coop.open = coop.in_room() || !coop.open;
    }
}
fn request(coop: &mut Coop, editor: &mut Editor, pending: Pending) {
    if coop.name.trim().is_empty() {
        coop.name = "Hero".into();
    }
    if let Some(home) = crate::local_state::home() {
        let _ = crate::local_state::write_atomic(
            &home.join("survivors-coop.txt"),
            &format!("{}\n{}", coop.url.trim(), coop.name.trim()),
        );
    }
    editor.pending = Some(pending);
    editor.field = None;
    if !coop.connected || editor.dirty {
        if !coop.connecting {
            coop.connect();
            editor.dirty = false;
        }
    }
}
fn controls(
    mut coop: ResMut<Coop>,
    mut editor: ResMut<Editor>,
    selection: Res<Selection>,
    buttons: Query<(Entity, &Action, &skin::Control)>,
    clicks: Res<GlueClicks>,
    mut keyboard: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<Phase>>,
    mut clipboard: NonSendMut<HostClipboard>,
    window: Query<&bevy::window::RawHandleWrapper, With<bevy::window::PrimaryWindow>>,
    real: Res<Time<Real>>,
) {
    // Deferred actions survive the asynchronous connection handshake.
    if coop.connected {
        if let Some(pending) = editor.pending.take() {
            if let Some(member) = coop.member(&selection) {
                match pending {
                    Pending::Host => {
                        let name = format!("{}'s party", coop.name);
                        let private = coop.private;
                        coop.send(Client::Create {
                            name,
                            private,
                            map: selection.map,
                            member,
                        });
                    }
                    Pending::Join(code) => coop.send(Client::Join { code, member }),
                    Pending::Browse => coop.send(Client::List),
                }
            } else {
                coop.status = "Choose your hero before joining a party.".into();
            }
        }
    } else if !coop.connecting {
        editor.pending = None;
    }
    if let Some(room) = &coop.room {
        if editor.room_seen != room.code {
            editor.room_seen = room.code.clone();
            if coop.peer == 0 {
                coop.send(Client::Ready { ready: true });
            }
        }
    } else {
        editor.room_seen.clear();
    }
    if !coop.open {
        keyboard.clear();
        return;
    }
    let wl = wayland_display(window.single().ok());
    for (entity, action, style) in &buttons {
        if !clicks.hit(entity) || !style.enabled {
            continue;
        }
        match action {
            Action::Field(field) => {
                editor.field = Some(*field);
                editor.selected = true;
            }
            Action::Page(page) => {
                editor.page = *page;
                editor.field = if *page == Page::Join { Some(2) } else { None };
                editor.selected = true;
                if *page == Page::Browse {
                    request(&mut coop, &mut editor, Pending::Browse);
                }
            }
            Action::Settings => {
                editor.settings = !editor.settings;
                editor.field = None;
            }
            Action::Connect => {
                coop.connect();
                editor.dirty = false;
            }
            Action::Host => request(&mut coop, &mut editor, Pending::Host),
            Action::JoinCode => {
                let code = coop.code.trim().to_uppercase();
                request(&mut coop, &mut editor, Pending::Join(code));
            }
            Action::Join(code) => request(&mut coop, &mut editor, Pending::Join(code.clone())),
            Action::Private => coop.private = !coop.private,
            Action::Refresh => request(&mut coop, &mut editor, Pending::Browse),
            Action::Ready => {
                let ready = coop
                    .room
                    .as_ref()
                    .and_then(|r| r.members.iter().find(|m| m.peer == coop.peer))
                    .is_none_or(|m| !m.ready);
                coop.send(Client::Ready { ready });
            }
            Action::Start => coop.send(Client::Start),
            Action::Copy => {
                if let Some(room) = &coop.room {
                    clipboard.write(wl, &room.code);
                    editor.copied_until = real.elapsed_secs() + 2.;
                }
            }
            Action::Leave => {
                let running = coop.running;
                coop.leave();
                editor.page = Page::Home;
                if running {
                    next.set(Phase::Menu);
                }
            }
            Action::Close => {
                coop.open = false;
                editor.pending = None;
            }
        }
    }
    if keys.just_pressed(KeyCode::Escape) && !coop.in_room() {
        if editor.settings {
            editor.settings = false;
        } else if editor.page != Page::Home && !coop.in_room() {
            editor.page = Page::Home;
        } else {
            coop.open = false;
        }
    }
    if editor.field == Some(2)
        && keys.just_pressed(KeyCode::Enter)
        && coop.code.len() == 6
        && editor.pending.is_none()
        && !coop.connecting
    {
        let code = coop.code.clone();
        request(&mut coop, &mut editor, Pending::Join(code));
    }
    for event in keyboard.read().filter(|e| e.state == ButtonState::Pressed) {
        if coop.in_room() || coop.connecting || editor.pending.is_some() {
            continue;
        }
        let Some(field) = editor.field else { continue };
        let control = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
        if control && event.key_code == KeyCode::KeyA {
            editor.selected = true;
            continue;
        }
        let pasted = if control && event.key_code == KeyCode::KeyV {
            clipboard.read(wl)
        } else {
            None
        };
        if control && pasted.is_none() {
            continue;
        }
        let value = match field {
            0 => &mut coop.url,
            1 => &mut coop.name,
            _ => &mut coop.code,
        };
        if event.key_code == KeyCode::Backspace {
            if editor.selected {
                value.clear();
                editor.selected = false;
            } else {
                value.pop();
            }
            if field == 0 {
                editor.dirty = true;
            }
        } else if let Some(text) = pasted.as_deref().or(event.text.as_deref()) {
            if editor.selected && text.chars().any(|c| !c.is_control()) {
                value.clear();
                editor.selected = false;
            }
            let limit = match field {
                0 => 256,
                1 => 24,
                _ => 6,
            };
            for mut c in text.chars().filter(|c| !c.is_control()) {
                if field == 2 {
                    if !c.is_ascii_alphanumeric() {
                        continue;
                    }
                    c = c.to_ascii_uppercase();
                }
                if value.chars().count() < limit {
                    value.push(c);
                }
            }
            if field == 0 {
                editor.dirty = true;
            }
        }
    }
}
fn field(
    p: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    title: &str,
    value: &str,
    id: u8,
    editor: &Editor,
) {
    skin::label(p, f, title, 10., skin::MUTED);
    let shown = if value.is_empty() {
        if id == 2 {
            "Enter 6-character code"
        } else {
            "Type here"
        }
        .to_string()
    } else {
        value.to_string()
    };
    skin::button(
        p,
        f,
        &format!(
            "{}{}",
            shown,
            if editor.field == Some(id) { " |" } else { "" }
        ),
        "",
        Action::Field(id),
        false,
        editor.field == Some(id),
        true,
    );
}
fn nav(p: &mut ChildSpawnerCommands, f: &Handle<Font>, editor: &Editor) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.),
        ..default()
    })
    .with_children(|p| {
        for (page, title) in [
            (Page::Home, "Play with friends"),
            (Page::Join, "Join by code"),
            (Page::Browse, "Public games"),
        ] {
            p.spawn(Node {
                flex_grow: 1.,
                flex_basis: Val::Px(0.),
                ..default()
            })
            .with_children(|p| {
                skin::button(
                    p,
                    f,
                    title,
                    "",
                    Action::Page(page),
                    false,
                    editor.page == page,
                    true,
                );
            });
        }
    });
}
fn render(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    coop: Res<Coop>,
    phase: Res<State<Phase>>,
    font: Res<HudFont>,
    selection: Res<Selection>,
    panels: Query<Entity, With<Panel>>,
    mut party: Query<&mut Text, With<PartyText>>,
    real: Res<Time<Real>>,
) {
    for mut text in &mut party {
        text.0 = if coop.running {
            let mut rows = vec![format!(
                "PARTY  {}",
                coop.room.as_ref().map_or("", |r| r.code.as_str())
            )];
            if let Some(room) = &coop.room {
                for member in &room.members {
                    if let Some(p) = coop.players.get(&member.peer) {
                        rows.push(format!(
                            "{}  {:.0}/{:.0}{}",
                            member.name,
                            p.hp,
                            p.maximum,
                            if !p.loaded {
                                "  Loading"
                            } else if p.paused {
                                "  Choosing / paused"
                            } else if p.hp <= 0. {
                                "  Fallen"
                            } else {
                                ""
                            }
                        ));
                    }
                }
            }
            rows.join("\n")
        } else {
            String::new()
        };
    }
    let copied = real.elapsed_secs() < editor.copied_until;
    let signature = format!(
        "{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}|{}|{}|{}|{:?}",
        coop.open,
        coop.url,
        coop.name,
        coop.code,
        coop.private,
        coop.status,
        coop.peer,
        coop.room,
        coop.rooms,
        editor.field,
        editor.page,
        editor.settings,
        coop.connecting,
        editor.pending.is_some(),
        copied,
        selection.version,
        phase.get()
    );
    if signature == editor.signature {
        return;
    }
    editor.signature = signature;
    for e in &panels {
        commands.entity(e).despawn();
    }
    if !coop.open || *phase.get() != Phase::Menu {
        return;
    }
    let f = &font.0;
    commands.spawn((Panel,Node {position_type:PositionType::Absolute,width:Val::Percent(100.),height:Val::Percent(100.),justify_content:JustifyContent::Center,align_items:AlignItems::Center,..default()},BackgroundColor(Color::srgba(0.012,0.015,0.012,0.86)),bevy::ui::FocusPolicy::Block,Interaction::default(),GlobalZIndex(1200)))
        .with_children(|root| {
            root.spawn((ScrollPanel,ScrollPosition::default(),skin::panel(Node {width:Val::Px(650.),max_width:Val::Percent(94.),max_height:Val::Percent(92.),overflow:Overflow::scroll_y(),padding:UiRect::all(Val::Px(26.)),flex_direction:FlexDirection::Column,row_gap:Val::Px(12.),..default()}))).with_children(|p| {
                p.spawn(Node {flex_direction:FlexDirection::Row,justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,..default()}).with_children(|p| {
                    p.spawn(Node {flex_direction:FlexDirection::Column,row_gap:Val::Px(5.),..default()}).with_children(|p| {
                        skin::label(p,f,"CO-OP",32.,skin::GOLD);
                        skin::label(p,f,if coop.in_room() {"Your next adventure starts here."} else {"Better together. Up to four heroes."},12.,skin::MUTED);
                    });
                    p.spawn(Node {width:Val::Px(90.),..default()}).with_children(|p| {skin::button(p,f,if coop.in_room() {"Leave"} else {"Close"},"",if coop.in_room() {Action::Leave} else {Action::Close},false,false,true);});
                });
                skin::rule(p);
                if let Some(room)=&coop.room {
                    skin::label(p,f,if coop.peer==0 {"YOUR PARTY"} else {"PARTY INVITATION"},11.,skin::MUTED);
                    p.spawn((Node {padding:UiRect::all(Val::Px(16.)),border:UiRect::all(Val::Px(1.)),border_radius:BorderRadius::all(Val::Px(5.)),flex_direction:FlexDirection::Row,justify_content:JustifyContent::SpaceBetween,align_items:AlignItems::Center,..default()},BackgroundColor(skin::SURFACE),BorderColor::all(skin::EDGE))).with_children(|p| {
                        p.spawn(Node {flex_direction:FlexDirection::Column,row_gap:Val::Px(6.),..default()}).with_children(|p| {
                            skin::label(p,f,"JOIN CODE",10.,skin::MUTED);
                            skin::label(p,f,room.code.chars().map(|c|c.to_string()).collect::<Vec<_>>().join("  "),31.,skin::GOLD);
                        });
                        p.spawn(Node {width:Val::Px(110.),..default()}).with_children(|p| {skin::button(p,f,if copied {"Copied!"} else {"Copy code"},"",Action::Copy,false,false,true);});
                    });
                    skin::label(p,f,format!("{}  ·  {} of 4 heroes  ·  {}",data::MAPS.get(room.map).map_or("Unknown arena", |m| m.name),room.members.len(),if room.private {"Invite only"} else {"Public party"}),12.,skin::TEXT);
                    p.spawn(Node {flex_direction:FlexDirection::Row,flex_wrap:FlexWrap::Wrap,column_gap:Val::Px(10.),row_gap:Val::Px(10.),..default()}).with_children(|grid| {
                        for slot in 0..4 {
                            grid.spawn((Node {width:Val::Percent(48.),min_height:Val::Px(74.),padding:UiRect::all(Val::Px(12.)),border:UiRect::all(Val::Px(1.)),border_radius:BorderRadius::all(Val::Px(5.)),flex_direction:FlexDirection::Column,row_gap:Val::Px(7.),..default()},BackgroundColor(skin::SURFACE),BorderColor::all(skin::EDGE))).with_children(|p| {
                                if let Some(member)=room.members.get(slot) {
                                    skin::label(p,f,format!("{}{}",member.name,if member.peer==coop.peer {" (you)"} else {""}),16.,skin::TEXT);
                                    let class=data::CLASSES.get(member.hero.class).map_or("Hero",|c|c.name);
                                    skin::label(p,f,format!("{}  ·  {}{}",class,if member.ready {"Ready"} else {"Not ready"},if member.peer==0 {"  ·  Host"} else {""}),11.,if member.ready {skin::GREEN} else {skin::MUTED});
                                } else {skin::label(p,f,"Waiting for a friend",15.,skin::MUTED);skin::label(p,f,"Share your join code",11.,skin::MUTED);}
                            });
                        }
                    });
                    let all_ready=room.members.iter().all(|m|m.ready);
                    if coop.peer==0 {skin::button(p,f,"Start adventure",if all_ready {"Everyone is ready"} else {"Waiting for everyone to be ready"},Action::Start,true,false,all_ready);}
                    else { let ready=room.members.iter().find(|m|m.peer==coop.peer).is_some_and(|m|m.ready);skin::button(p,f,if ready {"Ready — waiting for host"} else {"Ready to play"},"",Action::Ready,true,ready,true); }
                    skin::button(p,f,"Leave party","",Action::Leave,false,false,true);
                } else if editor.settings {
                    skin::label(p,f,"CONNECTION SETTINGS",16.,skin::GOLD);
                    skin::label(p,f,"Use the same co-op server as your friends. This address is remembered.",12.,skin::TEXT);
                    field(p,f,"SERVER ADDRESS",&coop.url,0,&editor);
                    skin::button(p,f,if coop.connecting {"Connecting…"} else {"Save & connect"},"",Action::Connect,true,false,!coop.connecting);
                    if coop.status.starts_with("Connection lost:") {skin::label(p,f,&coop.status,11.,skin::MUTED);}
                    skin::button(p,f,"Back to co-op","",Action::Settings,false,false,true);
                } else {
                    nav(p,f,&editor);
                    if let Some(class)=selection.class_def() {skin::label(p,f,format!("Playing as {}",class.name),12.,skin::GOLD);}
                    field(p,f,"YOUR NAME",&coop.name,1,&editor);
                    let busy=coop.connecting || editor.pending.is_some();
                    match editor.page {
                        // The party's privacy is a setting of the host action below it; joining
                        // by code has its own tab above.
                        Page::Home => {
                            skin::label(p,f,"PARTY PRIVACY",10.,skin::MUTED);
                            skin::button(p,f,if coop.private {"Invite only"} else {"Public party"},if coop.private {"Only friends with your code can join. Click to make it public."} else {"Anyone can find your party in Public games. Click to make it invite only."},Action::Private,false,coop.private,!busy);
                            skin::button(p,f,"Host game","Create a party and share its join code",Action::Host,true,false,!busy);
                        }
                        Page::Join => {
                            skin::label(p,f,"JOIN YOUR FRIENDS",16.,skin::GOLD);
                            skin::label(p,f,"Ask your host for their six-character join code.",12.,skin::TEXT);
                            field(p,f,"JOIN CODE",&coop.code,2,&editor);
                            skin::button(p,f,if busy {"Joining…"} else {"Join game"},"",Action::JoinCode,true,false,!busy && coop.code.len()==6);
                        }
                        Page::Browse => {
                            skin::label(p,f,"PUBLIC GAMES",16.,skin::GOLD);
                            if coop.rooms.is_empty() {skin::label(p,f,if busy {"Looking for parties…"} else {"No public games yet. Host one or join a friend by code."},13.,skin::MUTED);}
                            for room in &coop.rooms {skin::button(p,f,&room.name,&format!("{}  ·  {}/4 heroes  ·  Join",data::MAPS.get(room.map).map_or("Unknown arena", |m| m.name),room.members.len()),Action::Join(room.code.clone()),false,false,!busy);}
                            skin::button(p,f,"Refresh games","",Action::Refresh,false,false,!busy);
                        }
                    }
                    if busy {skin::label(p,f,"Connecting your party…",12.,skin::GOLD);}
                    else if !coop.connected && coop.status.starts_with("Connection lost:") {skin::label(p,f,"Couldn't reach the co-op server. Check Connection settings and try again.",12.,Color::srgb(0.91,0.56,0.38));}
                    else if coop.status!="Connect to browse rooms or host a game." && !coop.status.starts_with("Connected.") && coop.status!="Connecting..." {skin::label(p,f,&coop.status,12.,skin::MUTED);}
                    skin::rule(p);
                    p.spawn(Node {flex_direction:FlexDirection::Row,align_items:AlignItems::Center,justify_content:JustifyContent::SpaceBetween,..default()}).with_children(|p| {
                        skin::label(p,f,if coop.connected {"Connected"} else if busy {"Connecting"} else {"Ready when you are"},11.,if coop.connected {skin::GREEN} else {skin::MUTED});
                        p.spawn(Node {width:Val::Px(190.),..default()}).with_children(|p| {skin::button(p,f,"Connection settings","",Action::Settings,false,false,!busy);});
                    });
                }
            });
        });
}

fn scroll_dialog(
    coop: Res<Coop>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollPanel>>,
) {
    if !coop.open {
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
    for (mut scroll, node) in &mut panels {
        let limit = (node.content_size.y - node.size.y).max(0.) * node.inverse_scale_factor();
        scroll.0.y = (scroll.0.y - delta).clamp(0., limit);
    }
}
