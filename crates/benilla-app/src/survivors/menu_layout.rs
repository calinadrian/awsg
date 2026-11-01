//! Hero selection surrounds a live preview with two legible, bounded panels.
use super::{
    data, front_end as skin,
    hud::{HudFont, MenuRoot},
    menu::{MenuAction, MenuBuilt, PickDescription, PickName, RaceFace},
    menu_icons::icon_button,
    races::{race_tc, RACES},
};
use crate::glue::{
    art::{class_tc, tc_rect, GlueArt},
    widgets::{GlueDisabled, LockHighlight},
};
use bevy::prelude::*;

pub(super) fn build_menu(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, Without<MenuBuilt>)>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut art: ResMut<GlueArt>,
    mut images: ResMut<Assets<Image>>,
    mut add_mats: ResMut<Assets<crate::glue::add_material::AddUiMaterial>>,
    font: Res<HudFont>,
) {
    let Ok(root) = roots.single() else { return };
    let Some(mut assets) = assets else { return };
    art.ensure_loaded(&mut assets, &mut images, &mut add_mats);
    let f = &font.0;
    commands.entity(root).insert(MenuBuilt).with_children(|ui| {
        ui.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                width: Val::Percent(100.),
                height: Val::Px(72.),
                padding: UiRect::axes(Val::Px(26.), Val::Px(14.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.023, 0.02, 0.83)),
        ))
        .with_children(|p| {
            skin::label(p, f, "AZEROTH SURVIVORS", 25., skin::GOLD);
            skin::label(
                p,
                f,
                "Choose your hero. Begin your adventure.",
                12.,
                skin::MUTED,
            );
        });
        tower(ui, &art, f);
        adventure(ui, f);
        ui.spawn(skin::panel(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(306.),
            right: Val::Px(324.),
            bottom: Val::Px(34.),
            min_height: Val::Px(115.),
            padding: UiRect::all(Val::Px(16.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.),
            ..default()
        }))
        .with_children(|p| {
            let title = skin::label(p, f, "", 25., skin::GOLD);
            p.commands().entity(title).insert(PickName);
            let description = skin::label(p, f, "", 12., skin::TEXT);
            p.commands().entity(description).insert(PickDescription);
            skin::rule(p);
            let progress = skin::label(p, f, "", 11., skin::MUTED);
            p.commands()
                .entity(progress)
                .insert(super::talents::MenuProgress);
        });
        ui.spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.),
            width: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            skin::label(
                p,
                f,
                "ENTER  Play solo     F6  Play with friends     ESC  Settings",
                11.,
                skin::TEXT,
            );
        });
    });
}

fn tower(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(22.),
        top: Val::Px(91.),
        bottom: Val::Px(34.),
        width: Val::Px(264.),
        padding: UiRect::all(Val::Px(16.)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.),
        ..default()
    }))
    .with_children(|p| {
        skin::label(p, f, "YOUR HERO", 18., skin::GOLD);
        skin::label(p, f, "RACE", 10., skin::MUTED);
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(5.),
            row_gap: Val::Px(5.),
            ..default()
        })
        .with_children(|grid| {
            for def in RACES {
                let face = art.races.as_ref().and_then(|(sheet, size)| {
                    race_tc(def.id, 0).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                });
                icon_button(
                    grid,
                    f,
                    MenuAction::Race(def.id),
                    Some(RaceFace(def.id)),
                    face,
                    None::<()>,
                    def.name,
                    art,
                    7.0 / 12.0,
                );
            }
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.),
            ..default()
        })
        .with_children(|row| {
            for (sex, name) in [(0, "Male"), (1, "Female")] {
                row.spawn(Node {
                    width: Val::Percent(48.),
                    ..default()
                })
                .with_children(|p| {
                    let e = skin::button(p, f, name, "", MenuAction::Sex(sex), false, false, true);
                    p.commands().entity(e).insert(LockHighlight::default());
                });
            }
        });
        skin::rule(p);
        skin::label(p, f, "CLASS", 10., skin::MUTED);
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.),
            row_gap: Val::Px(6.),
            ..default()
        })
        .with_children(|grid| {
            for (i, class) in data::CLASSES.iter().enumerate() {
                grid.spawn(Node {
                    width: Val::Px(70.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.),
                    ..default()
                })
                .with_children(|p| {
                    let face = art.classes.as_ref().and_then(|(sheet, size)| {
                        class_tc(class.id).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                    });
                    icon_button(
                        p,
                        f,
                        MenuAction::Class(i),
                        None::<()>,
                        face,
                        None::<()>,
                        class.name,
                        art,
                        0.75,
                    );
                    skin::label(p, f, class.name, 10., skin::TEXT);
                });
            }
        });
        p.spawn(Node {
            flex_grow: 1.,
            min_height: Val::Px(2.),
            ..default()
        });
        skin::button(
            p,
            f,
            "Randomize hero",
            "",
            MenuAction::Randomize,
            false,
            false,
            true,
        );
        skin::button(p, f, "Talents", "", MenuAction::Talents, false, false, true);
    });
}

fn adventure(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn(skin::panel(Node { position_type: PositionType::Absolute, right: Val::Px(22.), top: Val::Px(91.), bottom: Val::Px(34.), width: Val::Px(282.), padding: UiRect::all(Val::Px(18.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(7.), ..default() }))
        .with_children(|p| {
            skin::label(p,f,"YOUR ADVENTURE",12.,skin::MUTED);
            for (i,map) in data::MAPS.iter().enumerate() {
                p.spawn((Button,MenuAction::Map(i),LockHighlight::default(),Node { width: Val::Percent(100.), min_height: Val::Px(110.), padding: UiRect::all(Val::Px(14.)), border: UiRect::all(Val::Px(1.)), border_radius: BorderRadius::all(Val::Px(5.)), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, row_gap: Val::Px(9.), ..default() },BackgroundColor(Color::srgb(0.10,0.125,0.085)),BorderColor::all(skin::EDGE)))
                    .with_children(|p| {
                        skin::label(p,f,map.zone,11.,skin::MUTED);
                        skin::label(p,f,map.name,25.,skin::GOLD);
                        skin::label(p,f,"NORMAL DIFFICULTY",10.,skin::TEXT);
                    });
                skin::label(p,f,"Hold your ground against the creatures of Elwynn. Defeat the final boss to claim victory.",12.,skin::TEXT);
                skin::rule(p);
                skin::label(p,f,"ENCOUNTERS",10.,skin::MUTED);
                for boss in &data::roster(i).bosses {
                    p.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, column_gap: Val::Px(8.), ..default() }).with_children(|p| {
                        skin::label(p,f,format!("{:02}:{:02}",(boss.from as u32)/60,(boss.from as u32)%60),11.,skin::GOLD);
                        skin::label(p,f,boss.name,12.,skin::TEXT);
                    });
                }
            }
            p.spawn(Node { flex_grow: 1., min_height: Val::Px(8.), ..default() });
            skin::label(p,f,"A new run. A stronger hero.",12.,skin::MUTED);
            let solo=skin::button(p,f,"Play solo","Enter Elwynn Forest",MenuAction::Enter,true,false,true);
            p.commands().entity(solo).insert(GlueDisabled(false));
            skin::button(p,f,"Co-op","Host a game or join friends",super::coop_ui::OpenCoop,false,false,true);
        });
}
