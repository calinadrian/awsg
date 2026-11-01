//! Card comparisons use the same parameters as combat and reward application.
use super::{
    abilities,
    card_view::{Content, StatRow},
    data::Kind,
    hud::{CardText, Choice},
    Owned, Run,
};

fn number(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn comparison(label: &str, before: String, after: String) -> StatRow {
    StatRow {
        label: label.into(),
        before: Some(before),
        after,
    }
}

fn spell_rows(a: &abilities::Params, b: &abilities::Params, kind: Kind) -> Vec<StatRow> {
    let mut rows = Vec::new();
    if (a.dmg - b.dmg).abs() > 0.001 {
        let label = match kind {
            Kind::Shield => "ABSORB",
            Kind::Heal => "HEALING",
            _ => "DAMAGE",
        };
        if kind == Kind::Buff {
            rows.push(comparison(
                "DAMAGE BONUS",
                format!("+{}%", number(a.dmg * 100.)),
                format!("+{}%", number(b.dmg * 100.)),
            ));
        } else {
            rows.push(comparison(label, number(a.dmg), number(b.dmg)));
        }
    }
    if (a.dot - b.dot).abs() > 0.001 {
        rows.push(comparison("DAMAGE / SEC", number(a.dot), number(b.dot)));
    }
    if (a.cooldown - b.cooldown).abs() > 0.001 {
        rows.push(comparison(
            "COOLDOWN",
            format!("{}s", number(a.cooldown)),
            format!("{}s", number(b.cooldown)),
        ));
    }
    if a.count != b.count {
        rows.push(comparison(
            "TARGETS",
            a.count.to_string(),
            b.count.to_string(),
        ));
    }
    if (a.radius - b.radius).abs() > 0.001 {
        rows.push(comparison(
            "RADIUS",
            format!("{} yd", number(a.radius)),
            format!("{} yd", number(b.radius)),
        ));
    }
    if (a.duration - b.duration).abs() > 0.001 {
        rows.push(comparison(
            "DURATION",
            format!("{}s", number(a.duration)),
            format!("{}s", number(b.duration)),
        ));
    }
    rows
}

pub(super) fn describe(choice: Choice, run: &Run, card: &CardText) -> Content {
    let mut result = Content {
        rarity_name: choice.rarity().label().to_uppercase(),
        name: card.name.clone(),
        state: "NEW".into(),
        rank: None,
        rows: Vec::new(),
        description: card.desc.clone(),
        stat_label: "BONUS".into(),
        stat_value: String::new(),
        positive: true,
        footer: String::new(),
    };
    match choice {
        Choice::NewAbility(def, rarity) => {
            let owned = Owned::new(def, rarity);
            let p = abilities::parameters(&owned, run);
            let (label, value) = if def.kind == Kind::Buff {
                (
                    if def.self_effect == 9 {
                        "COOLDOWN RECOVERY"
                    } else {
                        "DAMAGE BONUS"
                    },
                    format!("+{}%", number(p.dmg * 100.)),
                )
            } else if p.dmg > 0.0 {
                (
                    match def.kind {
                        Kind::Shield => "ABSORB",
                        Kind::Heal => "HEALING",
                        _ => "DAMAGE",
                    },
                    number(p.dmg),
                )
            } else if p.dot > 0.0 {
                ("DAMAGE / SEC", number(p.dot))
            } else {
                ("COOLDOWN", format!("{} sec", number(p.cooldown)))
            };
            result.stat_label = label.into();
            result.stat_value = value;
            result.positive = false;
            result.footer = format!(
                "Takes a spell slot · {}/{}",
                run.abilities.len() + 1,
                super::data::MAX_ABILITIES
            );
        }
        Choice::Upgrade(index, rarity, axes) => {
            let owned = &run.abilities[index];
            let mut next = owned.clone();
            next.upgrade(rarity, axes);
            result.state.clear();
            result.rank = Some(format!("RANK {} → {}", owned.level, next.level));
            result.rows = spell_rows(
                &abilities::parameters(owned, run),
                &abilities::parameters(&next, run),
                owned.def.kind,
            );
            result.description = super::hud::ability_description(owned, run);
            result.footer = "Upgrades an existing spell".into();
        }
        Choice::Jewelry(index, rarity) => {
            let def = &super::jewelry::JEWELS[index];
            let old = run.jewelry.iter().find(|j| j.index == index);
            let current = old.map_or(0.0, |j| j.strength);
            let gain = old.map_or_else(
                || super::rewards::jewel_gain(def.stat, def.base, rarity, None),
                |j| j.gain(rarity),
            );
            if let Some(owned) = old {
                result.state.clear();
                result.rank = Some(format!("RANK {} → {}", owned.picks, owned.picks + 1));
            }
            for (stat, value) in super::jewelry::effects(index, current + gain) {
                if stat == 255 {
                    continue;
                }
                result.rows.push(StatRow {
                    label: super::jewelry::label(stat).to_uppercase(),
                    before: old.map(|_| {
                        super::jewelry::amount(stat, super::jewelry::stat(index, current, stat))
                    }),
                    after: super::jewelry::amount(stat, value),
                });
            }
            result.description = "Strengthens your equipment for this run.".into();
            result.footer = (if def.ring {
                "Ring · Run equipment"
            } else {
                "Trinket · Run equipment"
            })
            .into();
        }
        Choice::Amulet(index) => {
            let def = &super::amulets::AMULETS[index];
            let old = run.amulet.as_ref().filter(|a| a.index == index);
            let next = super::amulets::Amulet {
                index,
                rank: old.map_or(1, |a| a.rank + 1),
            };
            if run.amulet.is_some() {
                result.state.clear();
            }
            if let Some(owned) = old {
                result.rank = Some(format!("RANK {} → {}", owned.rank, next.rank));
            }
            for &(stat, value) in def.stats {
                result.rows.push(StatRow {
                    label: super::jewelry::label(stat).to_uppercase(),
                    before: old
                        .map(|a| format!("+{}", super::jewelry::amount(stat, value * a.scale()))),
                    after: format!("+{}", super::jewelry::amount(stat, value * next.scale())),
                });
            }
            if def.targets > 0 {
                // This definition grants a fixed target bonus; ranking it does not multiply it.
                result.rows.push(StatRow {
                    label: "TARGETS".into(),
                    before: old.map(|_| format!("+{}", def.targets)),
                    after: format!("+{}", def.targets),
                });
            }
            result.description = "Grants bonuses to your hero while equipped.".into();
            result.footer = match run.amulet.as_ref() {
                None => "Fills your amulet slot".into(),
                Some(owned) if owned.index == index => "Upgrades your equipped amulet".into(),
                Some(owned) => format!("Replaces {}", super::amulets::AMULETS[owned.index].name),
            };
        }
        Choice::Gold => {
            result.state.clear();
            result.stat_label = "GOLD".into();
            result.stat_value = "+25".into();
            result.description = "No eligible upgrades remain in this reward pool.".into();
            result.footer = "Added to your run gold".into();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::{amulets::Amulet, hud::describe_card};
    use super::*;

    #[test]
    fn empty_amulet_slot_is_new_and_shows_acquisition_values() {
        let run = Run::default();
        let choice = Choice::Amulet(0);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert_eq!(result.state, "NEW");
        assert_eq!(result.footer, "Fills your amulet slot");
        assert!(result.rank.is_none());
        assert!(result.rows.iter().all(|row| row.before.is_none()));
    }

    #[test]
    fn amulet_upgrade_compares_real_ranks_and_every_bonus() {
        let run = Run {
            amulet: Some(Amulet { index: 2, rank: 2 }),
            ..Run::default()
        };
        let choice = Choice::Amulet(2);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert!(result.state.is_empty());
        assert_eq!(result.rank.as_deref(), Some("RANK 2 → 3"));
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|row| row
            .before
            .as_ref()
            .is_some_and(|before| before != &row.after)));
        assert_eq!(result.footer, "Upgrades your equipped amulet");
    }

    #[test]
    fn replacement_names_the_equipped_amulet() {
        let run = Run {
            amulet: Some(Amulet { index: 0, rank: 2 }),
            ..Run::default()
        };
        let choice = Choice::Amulet(1);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert_eq!(result.footer, "Replaces Mark of Fordring");
        assert!(result.state.is_empty());
        assert!(result.rank.is_none());
    }
    #[test]
    fn spell_upgrade_keeps_both_power_and_targets() {
        let mut run = Run::default();
        let def = super::super::data::ability("chain_lightning");
        run.abilities
            .push(Owned::new(def, super::super::rewards::Rarity::Common));
        let choice = Choice::Upgrade(
            0,
            super::super::rewards::Rarity::Rare,
            [
                Some(super::super::rewards::Upgrade::Power),
                Some(super::super::rewards::Upgrade::Targets),
            ],
        );
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert!(result.state.is_empty());
        assert!(result
            .rank
            .as_ref()
            .is_some_and(|rank| rank.contains(" → ")));
        assert!(result.rows.iter().any(|row| row.label == "DAMAGE"));
        assert!(result.rows.iter().any(|row| row.label == "TARGETS"));
        assert!(result.rows.iter().all(|row| row.before.is_some()));
    }
}
