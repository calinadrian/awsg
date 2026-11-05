//! Three-way reconciliation of a player's last acknowledged inventory and offline changes.
use crate::economy::Inventory;
use std::collections::{BTreeMap, BTreeSet};

pub fn same<T: serde::Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_vec(a).ok() == serde_json::to_vec(b).ok()
}

pub fn same_inventory(a: &Inventory, b: &Inventory) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.class = 0;
    b.class = 0;
    a.name.clear();
    b.name.clear();
    same(&a, &b)
}

fn delta(server: u64, base: u64, local: u64) -> Result<u64, String> {
    let n = i128::from(server) + i128::from(local) - i128::from(base);
    u64::try_from(n).map_err(|_| {
        "Sync conflict: a balance changed on another device; offline progress was retained".into()
    })
}

/// Changes to the same item conflict; independent rewards and balance changes combine.
pub fn merge(
    server: &Inventory,
    base: &Inventory,
    local: &Inventory,
    catalog: &crate::economy::Catalog,
) -> Result<Inventory, String> {
    let mut out = server.clone();
    out.class = local.class;
    out.gold = delta(server.gold, base.gold, local.gold)?;
    if out.gold > crate::ledger::MAX_GOLD {
        return Err("Combined gold exceeds limit".into());
    }
    out.achievement_points = u32::try_from(delta(
        server.achievement_points.into(),
        base.achievement_points.into(),
        local.achievement_points.into(),
    )?)
    .map_err(|_| "Achievement balance exceeds limit")?;
    out.achievement_total = u32::try_from(delta(
        server.achievement_total.into(),
        base.achievement_total.into(),
        local.achievement_total.into(),
    )?)
    .map_err(|_| "Achievement total exceeds limit")?;
    for key in base
        .materials
        .keys()
        .chain(local.materials.keys())
        .collect::<BTreeSet<_>>()
    {
        let n = delta(
            server.materials.get(key).copied().unwrap_or(0).into(),
            base.materials.get(key).copied().unwrap_or(0).into(),
            local.materials.get(key).copied().unwrap_or(0).into(),
        )?;
        out.materials.insert(
            key.clone(),
            u32::try_from(n).map_err(|_| "Material balance exceeds limit")?,
        );
    }
    let old: BTreeMap<_, _> = base.items.iter().map(|i| (i.id, i)).collect();
    let current: BTreeMap<_, _> = local.items.iter().map(|i| (i.id, i)).collect();
    if current.len() != local.items.len() {
        return Err("Duplicate item IDs in offline inventory".into());
    }
    for (&id, before) in &old {
        let after = current.get(&id);
        if after.is_some_and(|item| same(*item, *before)) {
            continue;
        }
        let remote = out.items.iter().position(|i| i.id == id);
        if remote.is_none() && after.is_none() {
            continue;
        }
        let Some(index) = remote else {
            return Err("Sync conflict: an offline item was traded on the server".into());
        };
        if !same(&out.items[index], *before) {
            if after.is_some_and(|item| same(*item, &out.items[index])) {
                continue;
            }
            return Err("Sync conflict: the same item changed online and offline".into());
        }
        if out.items[index].listed {
            return Err("A listed item cannot be changed offline".into());
        }
        if let Some(item) = after {
            out.items[index] = (*item).clone();
        } else {
            out.items.remove(index);
        }
    }
    for (&id, item) in &current {
        if old.contains_key(&id) {
            continue;
        }
        if let Some(existing) = out.items.iter().find(|i| i.id == id) {
            if !same(existing, *item) {
                return Err("Sync conflict: item ID collision".into());
            }
        } else {
            let mut item = (*item).clone();
            item.listed = false;
            out.items.push(item);
        }
    }
    // One item per class/slot. Existing online gear wins a concurrent equipment choice.
    let mut equipped = BTreeSet::new();
    for item in &mut out.items {
        if let Some(slot) = &item.slot {
            let class = item.equipped_class.unwrap_or(out.class);
            if !equipped.insert((class, slot.clone())) {
                item.slot = None;
                item.equipped_class = None;
            }
        }
    }
    // A device and a server can each have given the same starter set; one copy is kept.
    let mut starters = BTreeSet::new();
    out.items.retain(|item| {
        !item.definition.starts_with(crate::starter::ZONE) || starters.insert(item.definition.clone())
    });
    out.starter_classes.extend(local.starter_classes.iter().copied());
    out.cleared_zones
        .extend(local.cleared_zones.iter().cloned());
    for (id, times) in &local.quests_done {
        let count = out.quests_done.entry(id.clone()).or_default();
        *count = (*count).max(*times);
    }
    out.quests.clear();
    let fresh: BTreeMap<_, _> = local
        .achievements
        .iter()
        .filter(|(id, _)| !base.achievements.contains_key(*id))
        .map(|(id, when)| (id.clone(), *when))
        .collect();
    let overlap = catalog.quests.overlap(&fresh, &server.achievements);
    out.achievement_points = out
        .achievement_points
        .checked_sub(overlap)
        .ok_or("Sync conflict: duplicate achievement points were already spent offline")?;
    out.achievement_total = out.achievement_total.saturating_sub(overlap);
    for (id, when) in &local.achievements {
        out.achievements.entry(id.clone()).or_insert(*when);
    }
    for (id, values) in &local.achievement_progress {
        let remote = out.achievement_progress.entry(id.clone()).or_default();
        remote.resize(remote.len().max(values.len()), 0);
        for (i, n) in values.iter().enumerate() {
            let before = base
                .achievement_progress
                .get(id)
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(0);
            let objective = catalog
                .quests
                .achievement(id)
                .and_then(|a| a.criteria.get(i));
            remote[i] = if objective.is_some_and(|o| o.kind.adds()) {
                delta(remote[i].into(), before.into(), (*n).into())?
                    .min(objective.map_or(u32::MAX, |o| o.count).into()) as u32
            } else {
                remote[i].max(*n)
            };
        }
    }
    for (key, value) in &local.offline_progress {
        let remote = server.offline_progress.get(key).copied().unwrap_or(0);
        let before = base.offline_progress.get(key).copied().unwrap_or(0);
        let n = if key.starts_with("spec:") || key.starts_with("active:") {
            if *value == before {
                remote
            } else if remote == before || remote == *value {
                *value
            } else {
                return Err(
                    "Sync conflict: the same talent build changed on another device".into(),
                );
            }
        } else if key.starts_with("best:") {
            remote.max(*value)
        } else {
            delta(remote, before, *value)?
        };
        out.offline_progress.insert(key.clone(), n);
    }
    out.quest_rev = server.quest_rev.max(local.quest_rev);
    out.achievement_progress
        .retain(|id, _| !out.achievements.contains_key(id));
    Ok(out)
}
