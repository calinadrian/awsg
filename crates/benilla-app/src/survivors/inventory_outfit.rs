//! Dress local and co-op heroes with owned item displays, preserving empty starter slots.
#[cfg(not(target_os = "macos"))]
use super::{
    coop::{Ally, Coop},
    data,
    hero::Hero,
    inventory::Equipment as Inventory,
    Phase, Run, Selection,
};
#[cfg(not(target_os = "macos"))]
use crate::{
    entities::{Creatures, Equipment},
    net::{NetEntity, ObjectStore},
};
use bevy::prelude::*;

const OUTFIT_MARKER: u16 = 2000;
pub(crate) fn has_outfit(store: &crate::net::ObjectStore) -> bool {
    store
        .0
        .raw_fields()
        .any(|(field, value)| field == OUTFIT_MARKER && value == 1)
}
/// The FEATURES dials `[skin, face, hair style, hair colour, facial hair]`, at raw fields
/// 2001..=2005 beside the outfit marker.
const FEATURE_FIELDS: u16 = OUTFIT_MARKER + 1;
pub(crate) fn features(store: &crate::net::ObjectStore) -> Option<[u8; 5]> {
    let mut f = [None; 5];
    for (field, value) in store.0.raw_fields() {
        if let Some(slot) = field.checked_sub(FEATURE_FIELDS).and_then(|i| f.get_mut(usize::from(i))) {
            *slot = Some(value as u8);
        }
    }
    Some(f.map(|v| v.unwrap_or_default())).filter(|_| f.iter().all(Option::is_some))
}
#[cfg(not(target_os = "macos"))]
#[derive(Component)]
struct Applied(Vec<asr_coop::economy::LookItem>, u32, Option<[u8; 5]>);

pub(super) fn plugin(app: &mut App) {
    #[cfg(not(target_os = "macos"))]
    app.add_systems(Update, dress.before(crate::entities::EntityVisualsSet));
    #[cfg(target_os = "macos")]
    let _ = app;
}
#[cfg(not(target_os = "macos"))]
#[allow(clippy::type_complexity)]
fn dress(
    mut commands: Commands,
    inventory: Res<Inventory>,
    coop: Res<Coop>,
    run: Res<Run>,
    selection: Res<Selection>,
    phase: Res<State<Phase>>,
    creatures: Option<Res<Creatures>>,
    mut heroes: Query<
        (
            Entity,
            &NetEntity,
            &mut ObjectStore,
            Option<&Ally>,
            Option<&Applied>,
        ),
        Or<(With<Hero>, With<Ally>)>,
    >,
) {
    let Some(creatures) = creatures else { return };
    for (entity, net, mut store, ally, applied) in &mut heroes {
        let Some(display) = net.display_id else {
            continue;
        };
        let Some(base) = creatures.survivors_appearance(display) else {
            continue;
        };
        // The ally's dials come with its hero pick; the local hero's are the menu's.
        let mut features = selection.current_features();
        let (look, class) = if let Some(ally) = ally {
            let Some(pose) = coop.players.get(&ally.0) else {
                continue;
            };
            let Some(member) = coop
                .room
                .as_ref()
                .and_then(|r| r.members.iter().find(|m| m.peer == ally.0))
            else {
                continue;
            };
            let Some(class) = data::CLASSES.get(member.hero.class) else {
                continue;
            };
            features = member.hero.features;
            (pose.outfit.clone(), class)
        } else if *phase.get() == Phase::Menu {
            let Some(class) = selection.class_def() else {
                continue;
            };
            let look = inventory
                .hero(&inventory.realm(&coop), class.id)
                .map_or_else(Vec::new, |hero| {
                    hero.inventory
                        .items
                        .iter()
                        .filter_map(|item| {
                            let slot = item.slot.as_ref()?;
                            let d = inventory.catalog.definition(&item.definition)?;
                            Some(asr_coop::economy::LookItem {
                                slot: slot.clone(),
                                display: d.display_id,
                                category: d.category.clone(),
                            })
                        })
                        .collect()
                });
            (look, class)
        } else {
            (run.gear_look.clone(), run.class)
        };
        if applied.is_some_and(|a| a.0 == look && a.1 == display && a.2 == features)
            || (look.is_empty() && features.is_none() && applied.is_none())
        {
            continue;
        }
        // A dial change alone leaves the worn set as it was, so the re-dress is forced below.
        let refeature = applied.is_some_and(|a| a.2 != features);
        let mut eq = Equipment {
            bodyslots: std::array::from_fn(|i| base.equipment[i + 2]),
            helm: base.equipment[0],
            survivors_shoulders: Some(base.equipment[1]),
            settled: true,
            ..default()
        };
        let mut pairs = vec![(OUTFIT_MARKER, 1)];
        let mut weapons = [(0u32, 0u32, 0u32); 3];
        for &(slot, display, class, sub, material, inv, sheath) in class.weapons {
            if let Some(weapon) = weapons.get_mut(usize::from(slot)) {
                *weapon = (
                    display,
                    u32::from(class)
                        | u32::from(sub) << 8
                        | u32::from(material) << 16
                        | u32::from(inv) << 24,
                    u32::from(sheath),
                );
            }
        }
        for item in &look {
            match item.slot.as_str() {
                "head" => eq.helm = item.display,
                "shoulders" => eq.survivors_shoulders = Some(item.display),
                "back" => eq.cloak = item.display,
                "shirt" => eq.bodyslots[0] = item.display,
                "chest" => eq.bodyslots[1] = item.display,
                "belt" => eq.bodyslots[2] = item.display,
                "legs" => eq.bodyslots[3] = item.display,
                "feet" => eq.bodyslots[4] = item.display,
                "wrists" => eq.bodyslots[5] = item.display,
                "hands" => eq.bodyslots[6] = item.display,
                "tabard" => eq.bodyslots[7] = item.display,
                "main_hand" | "off_hand" | "ranged" => {
                    let slot = match item.slot.as_str() {
                        "off_hand" => 1,
                        "ranged" => 2,
                        _ => 0,
                    };
                    let (item_class, sub, inv, sheath) = weapon_kind(&item.category);
                    weapons[slot] = (item.display, item_class | sub << 8 | inv << 24, sheath);
                    if slot == 0 && inv == 17 {
                        weapons[1] = (0, 0, 0);
                    }
                }
                _ => {}
            }
        }
        if look.iter().any(|i| i.slot == "off_hand") && weapons[0].1 >> 24 == 17 {
            weapons[0] = (0, 0, 0);
        }
        for (slot, (display, info, sheath)) in weapons.into_iter().enumerate() {
            let slot = slot as u16;
            pairs.extend([
                (37 + slot, display),
                (40 + 2 * slot, info),
                (41 + 2 * slot, sheath),
            ]);
        }
        let dials = features.unwrap_or_else(|| super::appearance::base(base));
        pairs.extend((0..5u16).map(|i| (FEATURE_FIELDS + i, u32::from(dials[usize::from(i)]))));
        store
            .0
            .merge(benilla_protocol::messages::ObjectFields::from_pairs(&pairs));
        if applied.is_none() || refeature {
            crate::entities::survivors_outfit::seed(&mut commands, entity);
        }
        commands.entity(entity).insert((eq, Applied(look, display, features)));
    }
}
#[cfg(not(target_os = "macos"))]
fn weapon_kind(category: &str) -> (u32, u32, u32, u32) {
    match category {
        "shield" => (4, 6, 14, 4),
        "held_off_hand" => (4, 0, 23, 0),
        "dagger" => (2, 15, 13, 3),
        "staff" => (2, 10, 17, 2),
        "wand" => (2, 19, 26, 0),
        "bow" => (2, 2, 15, 1),
        "gun" => (2, 3, 26, 1),
        "crossbow" => (2, 18, 26, 1),
        "two_hand_axe" => (2, 1, 17, 1),
        "two_hand_mace" => (2, 5, 17, 1),
        "two_hand_sword" => (2, 8, 17, 1),
        "one_hand_axe" => (2, 0, 13, 3),
        "one_hand_mace" => (2, 4, 13, 3),
        "polearm" => (2, 6, 17, 2),
        _ => (2, 7, 13, 3),
    }
}
