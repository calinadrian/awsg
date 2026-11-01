//! Escape-menu settings, backed by the same saved CVars as the engine controls.
use super::{front_end as skin, hud::HudFont, Phase};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct Options {
    pub open: bool,
    pub settings: bool,
    audio: bool,
    names: bool,
}

impl Options {
    pub fn escape(&mut self, phase: Phase, next: &mut NextState<Phase>) {
        if self.settings {
            self.settings = false;
            if phase == Phase::Menu {
                self.open = false;
            }
        } else if self.open || phase == Phase::Paused {
            self.resume(phase, next);
        } else {
            self.open = true;
            self.settings = phase == Phase::Menu;
            if phase == Phase::Playing {
                next.set(Phase::Paused);
            }
        }
    }

    pub fn resume(&mut self, phase: Phase, next: &mut NextState<Phase>) {
        self.open = false;
        self.settings = false;
        if phase == Phase::Paused {
            next.set(Phase::Playing);
        }
    }
}

struct Setting {
    label: &'static str,
    key: &'static str,
    values: &'static [&'static str],
    labels: &'static [&'static str],
    audio: bool,
}

const SETTINGS: &[Setting] = &[
    Setting {
        label: "Display mode",
        key: "gxWindow",
        values: &["1", "0"],
        labels: &["Windowed", "Fullscreen"],
        audio: false,
    },
    Setting {
        label: "Vertical sync",
        key: "gxVSync",
        values: &["0", "1"],
        labels: &["Off", "On"],
        audio: false,
    },
    Setting {
        label: "Render scale",
        key: "renderScale",
        values: &["0.5", "0.75", "1", "1.25", "1.5", "2"],
        labels: &["50%", "75%", "100%", "125%", "150%", "200%"],
        audio: false,
    },
    Setting {
        label: "View distance",
        key: "farclip",
        values: &["177", "300", "500", "777"],
        labels: &["Low", "Medium", "High", "Ultra"],
        audio: false,
    },
    Setting {
        label: "Grass and flowers",
        key: "WorldDetail",
        values: &["0", "1", "2"],
        labels: &["Low", "Medium", "High"],
        audio: false,
    },
    Setting {
        label: "Bloom",
        key: "ffxGlow",
        values: &["0", "1"],
        labels: &["Off", "On"],
        audio: false,
    },
    Setting {
        label: "Sound",
        key: "MasterSoundEffects",
        values: &["0", "1"],
        labels: &["Muted", "On"],
        audio: true,
    },
    Setting {
        label: "Master volume",
        key: "MasterVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
        audio: true,
    },
    Setting {
        label: "Effects volume",
        key: "SoundVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
        audio: true,
    },
    Setting {
        label: "Music volume",
        key: "MusicVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
        audio: true,
    },
    Setting {
        label: "Ambience volume",
        key: "AmbienceVolume",
        values: &[
            "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1",
        ],
        labels: &[
            "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
        ],
        audio: true,
    },
];

fn nearest(setting: &Setting, value: &str) -> usize {
    let value = value.parse::<f32>().unwrap_or(0.0);
    setting
        .values
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let distance = |s: &str| (s.parse::<f32>().unwrap_or(0.0) - value).abs();
            distance(a).total_cmp(&distance(b))
        })
        .map_or(0, |(i, _)| i)
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct ValueLabel(usize);
#[derive(Component, Clone, Copy)]
enum Action {
    Adjust(usize, i32),
    Graphics,
    Audio,
    Names,
    Toggle(InterfaceSetting),
    Back,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct InterfaceRow(InterfaceSetting);
#[derive(Component)]
struct FlagLabel(InterfaceSetting);
#[derive(Component)]
struct InterfaceHint;

#[derive(Clone, Copy, PartialEq, Eq)]
enum InterfaceSetting {
    PlayerNames,
    NpcNames,
    OwnName,
    GuildNames,
    EnemyPlates,
    FriendlyPlates,
}

impl InterfaceSetting {
    const ALL: [Self; 6] = [
        Self::PlayerNames,
        Self::NpcNames,
        Self::OwnName,
        Self::GuildNames,
        Self::EnemyPlates,
        Self::FriendlyPlates,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::PlayerNames => "Player names",
            Self::NpcNames => "NPC names",
            Self::OwnName => "Your name",
            Self::GuildNames => "Guild names",
            Self::EnemyPlates => "Enemy nameplates",
            Self::FriendlyPlates => "Friendly nameplates",
        }
    }

    fn cvar(self) -> Option<&'static str> {
        match self {
            Self::PlayerNames => Some("UnitNamePlayer"),
            Self::NpcNames => Some("UnitNameNPC"),
            Self::OwnName => Some("UnitNameOwn"),
            Self::GuildNames => Some("UnitNamePlayerGuild"),
            Self::EnemyPlates | Self::FriendlyPlates => None,
        }
    }

    fn enabled(
        self,
        cvars: &crate::cvars::Cvars,
        plates: &super::plates::NameplateSettings,
    ) -> bool {
        match self {
            Self::EnemyPlates => plates.enemies,
            Self::FriendlyPlates => plates.friends,
            _ => self.cvar().and_then(|key| cvars.flag(key)).unwrap_or(false),
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Options>().add_systems(
        Update,
        (spawn, clicks, paint)
            .chain()
            .after(super::hud::buttons)
            .before(super::hud::keys),
    );
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    if let Ok(preview) = std::env::var("WOW_SURVIVORS_OPTIONS_PREVIEW") {
        app.insert_resource(Options {
            open: true,
            settings: preview != "menu",
            audio: preview == "audio",
            names: false,
        });
    }
}

fn text(font: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

fn control(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str, action: Action) {
    parent
        .spawn((
            Button,
            action,
            Node {
                min_width: Val::Px(38.),
                height: Val::Px(34.),
                padding: UiRect::horizontal(Val::Px(12.)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(4.)),
                ..default()
            },
            BackgroundColor(skin::SURFACE),
            BorderColor::all(skin::EDGE),
        ))
        .with_children(|b| {
            b.spawn(text(font, label, 16., skin::GOLD));
        });
}

fn flag_control(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, setting: InterfaceSetting) {
    parent
        .spawn((
            Button,
            Action::Toggle(setting),
            Node {
                min_width: Val::Px(66.),
                height: Val::Px(34.),
                padding: UiRect::horizontal(Val::Px(12.)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.)),
                border_radius: BorderRadius::all(Val::Px(4.)),
                ..default()
            },
            BackgroundColor(skin::SURFACE),
            BorderColor::all(skin::EDGE),
        ))
        .with_children(|b| {
            b.spawn((FlagLabel(setting), text(font, "Off", 16., skin::GOLD)));
        });
}

fn spawn(mut commands: Commands, font: Option<Res<HudFont>>, roots: Query<(), With<Root>>) {
    if !roots.is_empty() {
        return;
    }
    let Some(font) = font else {
        return;
    };
    let f = &font.0;
    commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
            GlobalZIndex(450),
        ))
        .with_children(|root| {
            root.spawn(skin::panel(Node {
                width: Val::Percent(90.),
                max_width: Val::Px(600.),
                max_height: Val::Percent(92.),
                padding: UiRect::all(Val::Px(26.)),
                row_gap: Val::Px(10.),
                flex_direction: FlexDirection::Column,
                ..default()
            }))
            .with_children(|panel| {
                panel.spawn(text(f, "SETTINGS", 28., skin::GOLD));
                panel
                    .spawn(Node {
                        column_gap: Val::Px(10.),
                        margin: UiRect::bottom(Val::Px(6.)),
                        ..default()
                    })
                    .with_children(|tabs| {
                        control(tabs, f, "Graphics", Action::Graphics);
                        control(tabs, f, "Sound", Action::Audio);
                        control(tabs, f, "Names", Action::Names);
                    });
                skin::rule(panel);
                for (i, setting) in SETTINGS.iter().enumerate() {
                    panel
                        .spawn((
                            Row(i),
                            Node {
                                width: Val::Percent(100.),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Node {
                                    flex_grow: 1.,
                                    ..default()
                                },
                                text(f, setting.label, 16., skin::TEXT),
                            ));
                            control(row, f, "−", Action::Adjust(i, -1));
                            row.spawn((
                                ValueLabel(i),
                                Node {
                                    width: Val::Px(108.),
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                text(f, "", 16., skin::GOLD),
                                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                            ));
                            control(row, f, "+", Action::Adjust(i, 1));
                        });
                }
                for setting in InterfaceSetting::ALL {
                    panel
                        .spawn((
                            InterfaceRow(setting),
                            Node {
                                width: Val::Percent(100.),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.),
                                display: Display::None,
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Node {
                                    flex_grow: 1.,
                                    ..default()
                                },
                                text(f, setting.label(), 16., skin::TEXT),
                            ));
                            flag_control(row, f, setting);
                        });
                }
                panel.spawn((
                    InterfaceHint,
                    Node {
                        display: Display::None,
                        ..default()
                    },
                    text(
                        f,
                        "V: nearby enemies · Shift+V: friends · Ctrl+V: both · boss plates stay visible",
                        12.,
                        skin::MUTED,
                    ),
                ));
                skin::rule(panel);
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(text(
                            f,
                            "Changes apply immediately and are saved automatically.",
                            12.,
                            skin::MUTED,
                        ));
                        control(row, f, "Back", Action::Back);
                    });
            });
        });
}

fn clicks(
    mut options: ResMut<Options>,
    phase: Res<State<Phase>>,
    mut cvars: ResMut<crate::cvars::Cvars>,
    mut nameplates: ResMut<super::plates::NameplateSettings>,
    mut commands: Commands,
    mut buttons: Query<(&Interaction, &Action, &mut BackgroundColor), Changed<Interaction>>,
) {
    if !options.settings {
        return;
    }
    for (interaction, action, mut bg) in &mut buttons {
        bg.0 = if *interaction == Interaction::None {
            skin::SURFACE
        } else {
            Color::srgb(0.15, 0.12, 0.075)
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Action::Graphics => {
                options.audio = false;
                options.names = false;
            }
            Action::Audio => {
                options.audio = true;
                options.names = false;
            }
            Action::Names => {
                options.audio = false;
                options.names = true;
            }
            Action::Back => {
                options.settings = false;
                if *phase.get() == Phase::Menu {
                    options.open = false;
                }
            }
            Action::Adjust(i, delta) => {
                let s = &SETTINGS[i];
                if options.names || s.audio != options.audio {
                    continue;
                }
                let at = nearest(s, cvars.get(s.key).unwrap_or(s.values[0]));
                let index = (at as i32 + delta).clamp(0, s.values.len() as i32 - 1) as usize;
                cvars.set(s.key, s.values[index]);
                cvars.commit_latched();
                for event in cvars.take_events() {
                    commands.trigger(event);
                }
            }
            Action::Toggle(setting) => {
                if let Some(key) = setting.cvar() {
                    let value = if !cvars.flag(key).unwrap_or(false) {
                        "1"
                    } else {
                        "0"
                    };
                    cvars.set(key, value);
                    cvars.commit_latched();
                    for event in cvars.take_events() {
                        commands.trigger(event);
                    }
                } else {
                    match setting {
                        InterfaceSetting::EnemyPlates => nameplates.enemies = !nameplates.enemies,
                        InterfaceSetting::FriendlyPlates => {
                            nameplates.friends = !nameplates.friends
                        }
                        _ => unreachable!("CVar-backed interface setting"),
                    }
                    super::plates::persist_nameplate_settings(
                        &nameplates,
                        &mut cvars,
                        &mut commands,
                    );
                }
            }
        }
    }
}

fn paint(
    options: Res<Options>,
    cvars: Res<crate::cvars::Cvars>,
    nameplates: Res<super::plates::NameplateSettings>,
    mut roots: Query<&mut Node, (With<Root>, Without<Row>)>,
    mut rows: Query<(&Row, &mut Node), Without<Root>>,
    mut labels: Query<(&ValueLabel, &mut Text), Without<FlagLabel>>,
    mut interface_rows: Query<
        (&InterfaceRow, &mut Node),
        (Without<Row>, Without<Root>, Without<InterfaceHint>),
    >,
    mut flag_labels: Query<(&FlagLabel, &mut Text), Without<ValueLabel>>,
    mut hints: Query<&mut Node, (With<InterfaceHint>, Without<Root>, Without<Row>)>,
    mut tabs: Query<(&Action, &mut BorderColor)>,
) {
    for (action, mut border) in &mut tabs {
        let on = match action {
            Action::Graphics => !options.audio && !options.names,
            Action::Audio => options.audio,
            Action::Names => options.names,
            _ => false,
        };
        let want = BorderColor::all(if on { skin::GOLD } else { skin::EDGE });
        if *border != want {
            *border = want;
        }
    }
    for mut node in &mut roots {
        node.display = if options.settings {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (row, mut node) in &mut rows {
        node.display = if !options.names && SETTINGS[row.0].audio == options.audio {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (_, mut node) in &mut interface_rows {
        node.display = if options.names {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut hints {
        node.display = if options.names {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (label, mut text) in &mut labels {
        let s = &SETTINGS[label.0];
        let value = cvars.get(s.key).unwrap_or(s.values[0]);
        let want = s.labels[nearest(s, value)];
        if text.0 != want {
            text.0 = want.into();
        }
    }
    for (label, mut text) in &mut flag_labels {
        let want = if label.0.enabled(&cvars, &nameplates) {
            "On"
        } else {
            "Off"
        };
        if text.0 != want {
            text.0 = want.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_closes_settings_before_resuming_the_paused_run() {
        let mut o = Options::default();
        let mut next = NextState::default();
        o.escape(Phase::Playing, &mut next);
        assert!(o.open);
        assert!(matches!(next, NextState::Pending(Phase::Paused)));
        o.settings = true;
        next = NextState::default();
        o.escape(Phase::Paused, &mut next);
        assert!(o.open && !o.settings);
        assert!(matches!(next, NextState::Unchanged));
        o.escape(Phase::Paused, &mut next);
        assert!(!o.open);
        assert!(matches!(next, NextState::Pending(Phase::Playing)));
    }
    #[test]
    fn opening_menu_during_reward_selection_does_not_discard_the_reward() {
        let mut o = Options::default();
        let mut next = NextState::default();
        o.escape(Phase::LevelUp, &mut next);
        o.escape(Phase::LevelUp, &mut next);
        assert!(!o.open);
        assert!(matches!(next, NextState::Unchanged));
    }
    #[test]
    fn numeric_values_match_saved_cvars_and_controls_stay_in_range() {
        for setting in SETTINGS {
            assert_eq!(setting.values.len(), setting.labels.len());
            for (i, v) in setting.values.iter().enumerate() {
                assert_eq!(nearest(setting, v), i);
            }
        }
        assert_eq!(nearest(&SETTINGS[2], "1.00"), 2);
    }

    #[test]
    fn setting_buttons_update_the_live_audio_and_graphics_backends() {
        let mut app = App::new();
        app.init_resource::<crate::cvars::Cvars>()
            .init_resource::<crate::sound::SoundConfig>()
            .insert_resource(crate::world_backdrop::RenderScale(1.))
            .insert_resource(Options {
                open: true,
                settings: true,
                audio: true,
                names: false,
            })
            .init_resource::<super::plates::NameplateSettings>()
            .add_observer(crate::sound::on_cvar)
            .add_observer(crate::world_backdrop::on_cvar)
            .add_systems(Update, clicks);
        let volume = SETTINGS
            .iter()
            .position(|s| s.key == "MasterVolume")
            .unwrap();
        let button = app
            .world_mut()
            .spawn((
                Interaction::Pressed,
                Action::Adjust(volume, -1),
                BackgroundColor(Color::BLACK),
            ))
            .id();
        app.update();
        assert!((app.world().resource::<crate::sound::SoundConfig>().master - 0.9).abs() < 0.001);
        assert_eq!(
            app.world()
                .resource::<crate::cvars::Cvars>()
                .get("MasterVolume"),
            Some("0.9")
        );
        app.world_mut().despawn(button);
        app.world_mut().resource_mut::<Options>().audio = false;
        let render = SETTINGS
            .iter()
            .position(|s| s.key == "renderScale")
            .unwrap();
        app.world_mut().spawn((
            Interaction::Pressed,
            Action::Adjust(render, 1),
            BackgroundColor(Color::BLACK),
        ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::world_backdrop::RenderScale>()
                .0,
            1.25
        );
    }
}
