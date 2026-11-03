//! The unit frame's one design: the boss bars' frame (the grey edge, a dark inset, black-lined
//! wells) round a health well with the level, name and health in it, and a slim power well under
//! it in the power's colour. The buffs and the debuffs each sit in a row on the edge their setting
//! names, from the outer end, their edges on the frame's.
use super::super::right_column as col;
use super::*;

/// The frame's width; the target's mirrors the player's from the right.
pub(super) const WIDTH: f32 = 170.;
/// The frame's height with its power well, which a unit with no power keeps empty.
pub(super) const HEIGHT: f32 = 25.;
/// The health well's height when its setting is unset.
pub(super) const HEALTH_H: f32 = 13.;
const POWER_H: f32 = 4.;
/// The most icons a row, the gap between them, the gap to the frame and between two rows.
const AURAS: usize = 8;
const AURA_GAP: f32 = 2.;
const ROW_GAP: f32 = 4.;
const ROWS_GAP: f32 = 2.;

pub(super) fn spawn(
    frame: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    target: bool,
    design: u8,
    scale: f32,
    health_h: f32,
    auras: AuraLayout,
) {
    // Laid out at its size rather than scaled, so text is measured as it is drawn; whole pixels
    // keep the edges flush.
    let px = |v: f32| (v * scale).round();
    let (fill, fill_hi, well) = if target {
        (col::WINE, col::WINE_HI, col::WELL)
    } else {
        (
            Color::srgb(0.18, 0.55, 0.2),
            Color::srgb(0.36, 0.74, 0.36),
            Color::srgb(0.027, 0.07, 0.04),
        )
    };
    let span = || Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.),
        top: Val::Px(0.),
        width: Val::Percent(100.),
        height: Val::Percent(100.),
        ..default()
    };
    let lined = || Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK);

    // The aura rows ride the plate. Their insets are from the plate's padding box, one pixel in
    // from its edge: -1 puts an icon's edge on the frame's, and the extra pixel on the gap puts the
    // rows `ROW_GAP` off its edge. On a shared edge the debuffs sit nearest, the buffs beyond.
    let mut commands = frame.commands();
    let mut rows = Vec::new();
    let mut row = |buff: bool| {
        let (at, size) = if buff {
            (auras.buffs, px(auras.buff_size))
        } else {
            (auras.debuffs, px(auras.debuff_size))
        };
        let above = match at {
            AuraPos::Above => true,
            AuraPos::Below => false,
            AuraPos::Hidden => return Vec::new(),
        };
        let beyond = if buff && auras.debuffs == at {
            px(auras.debuff_size) + px(ROWS_GAP)
        } else {
            0.
        };
        // As many as fit along the frame, up to a row's worth.
        let gap = px(AURA_GAP);
        let fit = ((px(WIDTH) + gap) / (size + gap)).floor() as usize;
        let slots: Vec<AuraSlot> = (0..fit.clamp(1, AURAS))
            .map(|_| spawn_aura_slot(&mut commands, font, size))
            .collect();
        for slot in &slots {
            // The bar's own edge: grey inside (crimson for a debuff), black outside.
            commands.entity(slot.frame).insert((
                BorderColor::all(if buff { col::STEEL_LO } else { col::WINE_HI }),
                Outline::new(Val::Px(1.), Val::ZERO, Color::BLACK),
            ));
        }
        let frames: Vec<Entity> = slots.iter().map(|s| s.frame).collect();
        let off = Val::Px(px(ROW_GAP) + 1. + beyond);
        let (top, bottom, margin) = if above {
            (Val::Auto, Val::Percent(100.), UiRect::bottom(off))
        } else {
            (Val::Percent(100.), Val::Auto, UiRect::top(off))
        };
        let (left, right) = if target {
            (Val::Auto, Val::Px(-1.))
        } else {
            (Val::Px(-1.), Val::Auto)
        };
        let row = commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left,
                right,
                top,
                bottom,
                margin,
                // The target's first aura at its outer end, the right.
                flex_direction: if target {
                    FlexDirection::RowReverse
                } else {
                    FlexDirection::Row
                },
                column_gap: Val::Px(px(AURA_GAP)),
                ..default()
            })
            .add_children(&frames)
            .id();
        rows.push(row);
        slots
    };
    let buffs = row(true);
    let debuffs = row(false);

    frame
        .spawn((
            col::plate(Node {
                position_type: PositionType::Absolute,
                left: if target { Val::Auto } else { Val::Px(0.) },
                right: if target { Val::Px(0.) } else { Val::Auto },
                top: Val::Px(0.),
                width: Val::Px(px(WIDTH)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(px(2.)),
                padding: UiRect::all(Val::Px(px(2.))),
                ..default()
            }),
            FrameAuras {
                target,
                buffs,
                debuffs,
            },
        ))
        .add_children(&rows)
        .with_children(|plate| {
            plate
                .spawn((
                    Node {
                        height: Val::Px(px(health_h)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(well),
                    lined(),
                ))
                .with_children(|w| {
                    w.spawn((FrameFill(target), span(), BackgroundColor(fill)))
                        .with_children(|f| {
                            f.spawn((
                                Node {
                                    width: Val::Percent(100.),
                                    height: Val::Px(1.),
                                    ..default()
                                },
                                BackgroundColor(fill_hi),
                            ));
                        });
                    w.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(px(5.))),
                        column_gap: Val::Px(px(6.)),
                        ..span()
                    })
                    .with_children(|t| {
                        t.spawn(Node {
                            align_items: AlignItems::Center,
                            // A long name gives way, clipped, so the health text stays whole.
                            flex_shrink: 1.,
                            min_width: Val::Px(0.),
                            overflow: Overflow::clip(),
                            column_gap: Val::Px(px(4.)),
                            ..default()
                        })
                        .with_children(|who| {
                            who.spawn((FrameLevel(target), label(font, 8. * scale, col::STEEL)));
                            who.spawn((
                                FrameName(target),
                                label(
                                    font,
                                    10. * scale,
                                    if design == 1 {
                                        col::STEEL_HI
                                    } else {
                                        col::TEXT_HI
                                    },
                                ),
                            ));
                        });
                        t.spawn((
                            FrameHealth(target),
                            Node {
                                flex_shrink: 0.,
                                ..default()
                            },
                            label(font, 9. * scale, col::TEXT_HI),
                        ));
                    });
                });
            plate
                .spawn((
                    PowerNode(target, 1),
                    Node {
                        height: Val::Px(px(POWER_H)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.02, 0.024, 0.043)),
                    lined(),
                ))
                .with_children(|p| {
                    p.spawn((PowerNode(target, 2), span(), BackgroundColor(Color::NONE)));
                });
        });
}
