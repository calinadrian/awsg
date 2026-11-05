//! Dress local and co-op heroes with exactly the item displays they have equipped.
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
            Option<&super::item_preview::ViewerModel>,
            Option<&Applied>,
        ),
        Or<(With<Hero>, With<Ally>, With<super::item_preview::ViewerModel>)>,
    >,
) {
    let Some(creatures) = creatures else { return };
    for (entity, net, mut store, ally, viewer, applied) in &mut heroes {
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
            // An ally that has sent no outfit yet still wears its class's basic gear.
            let look = if pose.outfit.is_empty() {
                asr_coop::starter::look(None, class.id, &inventory.catalog)
            } else {
                pose.outfit.clone()
            };
            (look, class)
        } else if viewer.is_some() {
            let Some(class) = data::CLASSES.iter().find(|class| class.id == inventory.viewer_class) else { continue };
            features = Some(super::appearance::base(base));
            let look = inventory.viewer_outfit.iter().filter_map(|(slot, id)| {
                let def = inventory.catalog.definition(id)?;
                def.fits(class.id, slot).then(|| asr_coop::economy::LookItem {
                    slot: slot.clone(), display: def.display_id, category: def.category.clone(),
                })
            }).collect();
            (look, class)
        } else if *phase.get() == Phase::Menu {
            let Some(class) = selection.class_def() else {
                continue;
            };
            let hero = inventory.view(&inventory.realm(&coop), class.id);
            let look = asr_coop::starter::look(hero.as_ref().map(|hero| &hero.inventory), class.id, &inventory.catalog);
            (look, class)
        } else if run.gear_look.is_empty() {
            // A run entered past the menu (a restart, auto-play) has no snapshot of the hero's gear.
            let hero = inventory.view(&inventory.realm(&coop), run.class.id);
            let look = asr_coop::starter::look(hero.as_ref().map(|hero| &hero.inventory), run.class.id, &inventory.catalog);
            (look, run.class)
        } else {
            (run.gear_look.clone(), run.class)
        };
        if applied.is_some_and(|a| a.0 == look && a.1 == display && a.2 == features) {
            continue;
        }
        // A dial change alone leaves the worn set as it was, so the re-dress is forced below.
        let refeature = applied.is_some_and(|a| a.2 != features);
        // The body starts bare: a trainer display's own armor and weapons would show through or
        // clash with what is worn, so only equipped items reach the character.
        let mut eq = Equipment {
            survivors_shoulders: Some(0),
            settled: true,
            ..default()
        };
        let mut pairs = vec![(OUTFIT_MARKER, 1)];
        let mut weapons = [(0u32, 0u32, 0u32); 3];
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
