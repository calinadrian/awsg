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
                ("COOLDOWN", format!("{}s", number(p.cooldown)))
            };
            result.description = super::hud::ability_description(&owned, run);
            result.rows.push(StatRow {
                label: label.into(),
                before: None,
                after: value,
            });
            if label != "COOLDOWN" {
                result.rows.push(StatRow {
                    label: "COOLDOWN".into(),
                    before: None,
                    after: format!("{}s", number(p.cooldown)),
                });
            }
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
            // The prose reads at the rank the card grants, as the rows end on.
            result.description = super::hud::ability_description(&next, run);
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
            result.description.clear();
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
                // A fixed target bonus; ranking does not multiply it, so it shows no comparison.
                result.rows.push(StatRow {
                    label: "TARGETS".into(),
                    before: None,
                    after: format!("+{}", def.targets),
                });
            }
            result.description.clear();
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
    fn longest_baseline_spell_fits_the_semibold_body_area() {
        use super::super::card_view::{BODY_HEIGHT, BODY_LINE_HEIGHT, BODY_SIZE};
        use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight, Wrap};
        // Isolate the bundled face so a system-font fallback cannot mask overflow.
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(include_bytes!("../../assets/cards/font-3.ttf").to_vec());
        let family = db.faces().next().expect("bundled semibold font").families[0]
            .0
            .clone();
        let mut fonts = FontSystem::new_with_locale_and_db("en-US".into(), db);
        let run = Run::default();
        let owned = Owned::new(
            super::super::data::ability("holy_shock"),
            super::super::rewards::Rarity::Legendary,
        );
        let description = super::super::hud::ability_description(&owned, &run);
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(BODY_SIZE, BODY_LINE_HEIGHT));
        buffer.set_size(&mut fonts, Some(356.0), None);
        buffer.set_wrap(&mut fonts, Wrap::WordOrGlyph);
        buffer.set_text(
            &mut fonts,
            &description,
            &Attrs::new()
                .family(Family::Name(&family))
                .weight(Weight::SEMIBOLD),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let lines: Vec<_> = buffer.layout_runs().collect();
        assert!(!lines.is_empty());
        assert!(
            lines.len() as f32 * BODY_LINE_HEIGHT <= BODY_HEIGHT,
            "{} lines: {}",
            lines.len(),
            description
        );
        assert!(lines.iter().all(|line| line.line_w <= 356.01));
    }

    #[test]
    fn new_spell_keeps_description_and_maps_cooldown_to_stats() {
        let run = Run::default();
        let def = super::super::data::ability("holy_shock");
        let rarity = super::super::rewards::Rarity::Legendary;
        let choice = Choice::NewAbility(def, rarity);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert_eq!(
            result.description,
            super::super::hud::ability_description(&Owned::new(def, rarity), &run)
        );
        assert!(!result.description.contains("Cooldown:"));
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[1].label, "COOLDOWN");
        assert!(result.rows.iter().all(|row| row.before.is_none()));
        assert!(!result.positive);
        assert_eq!(result.state, "NEW");
    }

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
