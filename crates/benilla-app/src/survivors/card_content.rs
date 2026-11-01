//! Reward presentation takes its numbers from the same parameters as the cast and reward apply.
use super::{
    abilities,
    card_view::Content,
    data::Kind,
    hud::{CardText, Choice},
    rewards::Upgrade,
    Owned, Run,
};

fn number(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
fn gain(before: f32, after: f32, suffix: &str) -> String {
    if before.abs() < 0.001 {
        return format!("+{} {suffix}", number(after - before));
    }
    let delta = (after / before - 1.) * 100.;
    format!("{}{:.0}%", if delta >= 0. { "+" } else { "−" }, delta.abs())
}

pub(super) fn describe(choice: Choice, run: &Run, card: &CardText) -> Content {
    let mut result = Content {
        rarity_name: choice.rarity().label().to_uppercase(),
        name: card.name.clone(),
        state: "NEW".into(),
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
                    format!("+{}%", number(p.dmg * 100.0)),
                )
            } else if p.dmg > 0. {
                (
                    match def.kind {
                        Kind::Shield => "ABSORB",
                        Kind::Heal => "HEALING",
                        _ => "DAMAGE",
                    },
                    number(p.dmg),
                )
            } else if p.dot > 0. {
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
            result.state = format!("RANK {}", next.level);
            let a = abilities::parameters(owned, run);
            let b = abilities::parameters(&next, run);
            let (label, value) = match axes[0].or(axes[1]).unwrap_or(Upgrade::Power) {
                Upgrade::Power => (
                    match owned.def.kind {
                        Kind::Shield => "ABSORB",
                        Kind::Heal => "HEALING",
                        _ => "DAMAGE",
                    },
                    if a.dmg > 0. {
                        gain(a.dmg, b.dmg, "damage")
                    } else {
                        gain(a.dot, b.dot, "damage/sec")
                    },
                ),
                Upgrade::Cooldown => ("COOLDOWN", gain(a.cooldown, b.cooldown, "sec")),
                Upgrade::Targets => ("TARGETS", format!("+{}", b.count.saturating_sub(a.count))),
                Upgrade::Area => ("RADIUS", gain(a.radius, b.radius, "yd")),
                Upgrade::Duration => ("DURATION", gain(a.duration, b.duration, "sec")),
            };
            result.stat_label = label.into();
            result.stat_value = value;
            // Keep every changed axis visible, including incidental power gains.
            result.description = format!(
                "Improves a spell you know.\n\n{}",
                card.desc.replace(" > ", " → ")
            );
            result.footer = "Upgrades an existing spell".into();
        }
        Choice::Jewelry(index, _) => {
            result.state = if run.jewelry.iter().any(|j| j.index == index) {
                "UPGRADE"
            } else {
                "NEW"
            }
            .into();
            let delta = card
                .desc
                .strip_suffix("\n\nApplies for this run")
                .unwrap_or(&card.desc);
            if let Some((label, values)) = delta.split_once('\n') {
                result.stat_label = label.to_uppercase();
                result.stat_value = values.replace("  >  ", " → ").replace("\n\n", "\n");
            } else {
                result.stat_value = delta.into();
            }
            result.description = "Strengthens your equipment for this run.".into();
            result.footer = if super::jewelry::JEWELS[index].ring {
                "Ring · Run equipment"
            } else {
                "Trinket · Run equipment"
            }
            .into();
        }
        Choice::Amulet(index) => {
            let rank = run
                .amulet
                .as_ref()
                .filter(|a| a.index == index)
                .map_or(1, |a| a.rank + 1);
            result.state = if rank > 1 {
                format!("RANK {rank}")
            } else {
                "NEW".into()
            };
            result.description = card.tag.clone();
            result.stat_value = card.desc.replace("\n\n", "\n");
            result.footer = "Amulet · One equipped".into();
        }
        Choice::Gold => {
            result.stat_label = "GOLD".into();
            result.stat_value = "+25".into();
            result.description = "No eligible upgrades remain in this reward pool.".into();
            result.footer = "Added to your run gold".into();
        }
    }
    result
}
