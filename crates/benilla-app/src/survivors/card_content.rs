//! Card comparisons use the same parameters as combat and reward application.
use super::{
    abilities,
    card_view::{Content, StatRow, Tag},
    data::{AbilityDef, Kind},
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

/// What a buff raises, the same on the card that teaches it and the cards that upgrade it.
fn buff_label(def: &AbilityDef) -> &'static str {
    if def.self_effect == 9 {
        "RECOVERY"
    } else {
        "DAMAGE BONUS"
    }
}

fn spell_rows(a: &abilities::Params, b: &abilities::Params, def: &AbilityDef) -> Vec<StatRow> {
    let mut rows = Vec::new();
    if (a.dmg - b.dmg).abs() > 0.001 {
        let label = match def.kind {
            Kind::Shield => "ABSORB",
            Kind::Heal => "HEALING",
            _ => "DAMAGE",
        };
        if def.kind == Kind::Buff {
            rows.push(comparison(
                buff_label(def),
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
        kind: "SPELL".into(),
        name: card.name.clone(),
        tag: None,
        rows: Vec::new(),
        description: card.desc.clone(),
        footer: String::new(),
        pips: None,
    };
    match choice {
        Choice::NewAbility(def, rarity) => {
            let owned = Owned::new(def, rarity);
            let p = abilities::parameters(&owned, run);
            let (label, value) = if def.kind == Kind::Buff {
                (buff_label(def), format!("+{}%", number(p.dmg * 100.)))
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
            result.tag = Some(Tag::New("NEW".into()));
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
            result.footer = "Takes a spell slot".into();
            result.pips = Some((run.abilities.len(), 1, super::data::MAX_ABILITIES));
        }
        Choice::Upgrade(index, rarity, axes) => {
            let owned = &run.abilities[index];
            let mut next = owned.clone();
            next.upgrade(rarity, axes);
            result.tag = Some(Tag::Rank(format!("RANK {} → {}", owned.level, next.level)));
            result.rows = spell_rows(
                &abilities::parameters(owned, run),
                &abilities::parameters(&next, run),
                owned.def,
            );
            // The prose reads at the rank the card grants, as the rows end on.
            result.description = super::hud::ability_description(&next, run);
            result.footer = "Upgrades an existing spell".into();
        }
        Choice::Food(id) => {
            let old = run.food[id.kind as usize];
            result.kind = id.kind().one.into();
            for bonus in id.build().bonuses {
                let after = bonus.amount(id.rank);
                let before = old.map(|held| bonus.amount(held.rank));
                result.rows.push(StatRow {
                    label: bonus.label.into(),
                    // A bonus this rank leaves alone shows once, without a comparison.
                    before: before.filter(|before| *before != after),
                    after,
                });
            }
            match old {
                Some(held) => {
                    result.tag = Some(Tag::Rank(format!(
                        "RANK {} → {}",
                        held.rank + 1,
                        id.rank + 1
                    )));
                    result.footer = if id.is_last() {
                        format!("Final upgrade of {}", held.food().name)
                    } else {
                        format!("Upgrades {}", held.food().name)
                    };
                }
                None => {
                    result.tag = Some(Tag::New("PERMANENT".into()));
                    result.footer = format!("Fills your {} slot", id.kind().slot);
                    let held = run.food.iter().flatten().count();
                    result.pips = Some((held, 1, super::food::SLOTS));
                }
            }
        }
        Choice::Gold => {
            result.kind = "TREASURE".into();
            result.rows.push(StatRow {
                label: "GOLD".into(),
                before: None,
                after: "+25".into(),
            });
            result.description = "No eligible upgrades remain in this reward pool.".into();
            result.footer = "Added to your run gold".into();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::{food::FoodId, hud::describe_card};
    use super::*;

    /// Lays text out in the font the cards really letter in, giving its lines and widest line.
    struct Measure {
        fonts: cosmic_text::FontSystem,
        family: String,
    }

    impl Measure {
        fn text(&mut self, text: &str, size: f32, line_height: f32, width: Option<f32>) -> (usize, f32) {
            use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};
            let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, line_height));
            buffer.set_size(&mut self.fonts, width, None);
            buffer.set_wrap(&mut self.fonts, Wrap::Word);
            buffer.set_text(
                &mut self.fonts,
                text,
                &Attrs::new().family(Family::Name(&self.family)),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.fonts, false);
            let lines: Vec<f32> = buffer.layout_runs().map(|line| line.line_w).collect();
            assert!(!lines.is_empty(), "{text:?} did not lay out");
            (lines.len(), lines.into_iter().fold(0.0, f32::max))
        }

        /// One line's width, for parts of the card that must not wrap.
        fn width(&mut self, text: &str, size: f32) -> f32 {
            self.text(text, size, size * 1.2, None).1
        }

        /// Every way this card can fail to fit: the description is clipped by its section, the name
        /// or tags run out of the head, a row runs past its well, or the footer hits its pips.
        fn check(&mut self, content: &Content, problems: &mut Vec<String>) {
            use super::super::card_view::{
                ARROW, ARROW_GAP, BODY, BODY_LINE, CAPTION, DESC_TEXT_HEIGHT, DESC_WIDTH,
                FOOT_WIDTH, HEADING, LEAD, NAME_WIDTH, PIP, PIP_GAP, ROW_GAP, ROW_WIDTH, SMALL,
                TAG_GAP, TAG_PADDING, TITLE,
            };
            let title = &content.name;
            if self.width(title, HEADING) > NAME_WIDTH {
                problems.push(format!("{title}: name too wide"));
            }
            if !content.description.is_empty() {
                let (lines, _) =
                    self.text(&content.description, BODY, BODY_LINE, Some(DESC_WIDTH));
                if lines as f32 * BODY_LINE > DESC_TEXT_HEIGHT {
                    problems.push(format!("{title}: {lines} lines: {}", content.description));
                }
            }
            let mut tags = self.width(&content.rarity_name, CAPTION)
                + 2.0 * TAG_PADDING
                + 4.0
                + self.width(&content.kind, CAPTION);
            if let Some(tag) = &content.tag {
                let (text, arrow) = match tag.text().split_once('→') {
                    Some((before, after)) => {
                        (format!("{}{}", before.trim(), after.trim()), ARROW + 2.0 * ARROW_GAP)
                    }
                    None => (tag.text().to_owned(), 0.0),
                };
                tags += TAG_GAP + self.width(&text, CAPTION) + arrow + 2.0 * TAG_PADDING + 2.0;
            }
            if tags > NAME_WIDTH {
                problems.push(format!("{title}: tags need {tags} of {NAME_WIDTH}"));
            }
            let alone = content.rows.len() == 1;
            for row in &content.rows {
                let label = self.width(&row.label, CAPTION);
                let values = match &row.before {
                    Some(before) => {
                        self.width(before, SMALL)
                            + ARROW
                            + 2.0 * ARROW_GAP
                            + self.width(&row.after, LEAD)
                    }
                    None => self.width(&row.after, if alone { TITLE } else { LEAD }),
                };
                if label + ROW_GAP + values > ROW_WIDTH {
                    problems.push(format!(
                        "{title}: {} {:?} -> {} needs {} of {ROW_WIDTH}",
                        row.label,
                        row.before,
                        row.after,
                        label + ROW_GAP + values
                    ));
                }
            }
            if content.rows.len() > 3 {
                problems.push(format!("{title}: {} rows", content.rows.len()));
            }
            let mut foot = self.width(&content.footer, SMALL);
            if let Some((_, _, total)) = content.pips {
                foot += ROW_GAP + total as f32 * (PIP + PIP_GAP);
            }
            if foot > FOOT_WIDTH {
                problems.push(format!("{title}: footer needs {foot} of {FOOT_WIDTH}"));
            }
        }
    }

    #[test]
    fn every_card_fits_in_the_game_font() {
        use super::super::rewards::Rarity;
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the install's MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        // Only this face in the database, so a system-font fallback cannot mask overflow.
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(chain.read_file("Fonts\\FRIZQT__.ttf").expect("card font"));
        let family = db.faces().next().expect("card font face").families[0].0.clone();
        let fonts = cosmic_text::FontSystem::new_with_locale_and_db("en-US".into(), db);
        let mut measure = Measure { fonts, family };
        let mut problems = Vec::new();
        let run = Run::default();
        let mut spells_seen = 0;
        for def in super::super::data::every_ability() {
            spells_seen += 1;
            let name = spells.get(def.spell).map_or_else(
                || super::super::hud::spell_name(None, def.spell, def.key),
                |spell| spell.name.clone(),
            );
            let axes = super::super::reward_axes::axes(def.key);
            let mut picks: Vec<[Option<super::super::rewards::Upgrade>; 2]> = Vec::new();
            for &first in axes {
                picks.push([Some(first), None]);
                picks.extend(axes.iter().map(|&second| [Some(first), Some(second)]));
            }
            for learned in [Rarity::Common, Rarity::Legendary] {
                let choice = Choice::NewAbility(def, learned);
                let mut card = describe(choice, &run, &describe_card(choice, &run, None));
                card.name = name.clone();
                measure.check(&card, &mut problems);
                for upgrade in [Rarity::Common, Rarity::Legendary] {
                    for &pick in &picks {
                        let mut run = Run::default();
                        run.abilities.push(Owned::new(def, learned));
                        while run.abilities[0].level < super::super::data::MAX_ABILITY_LEVEL {
                            let choice = Choice::Upgrade(0, upgrade, pick);
                            let mut card = describe(choice, &run, &describe_card(choice, &run, None));
                            card.name = name.clone();
                            measure.check(&card, &mut problems);
                            run.abilities[0].upgrade(upgrade, pick);
                        }
                    }
                }
            }
        }
        assert!(spells_seen > 100, "only {spells_seen} spells checked");
        let mut foods_seen = 0;
        for kind in 0..super::super::food::SLOTS as u8 {
            for build in 0..3 {
                let mut run = Run::default();
                for rank in 0..super::super::food::RANKS as u8 {
                    let id = FoodId { kind, build, rank };
                    let choice = Choice::Food(id);
                    let card = describe(choice, &run, &describe_card(choice, &run, None));
                    measure.check(&card, &mut problems);
                    run.food[kind as usize] = Some(id);
                    foods_seen += 1;
                }
            }
        }
        assert_eq!(foods_seen, 75);
        let gold = describe(Choice::Gold, &run, &describe_card(Choice::Gold, &run, None));
        measure.check(&gold, &mut problems);
        problems.sort();
        problems.dedup();
        assert!(
            problems.is_empty(),
            "{} cards do not fit:\n{}",
            problems.len(),
            problems.join("\n")
        );
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
        assert!(matches!(&result.tag, Some(Tag::New(text)) if text == "NEW"));
        assert_eq!(result.pips, Some((0, 1, super::super::data::MAX_ABILITIES)));
    }

    #[test]
    fn empty_food_slot_is_permanent_and_shows_what_it_gives() {
        let run = Run::default();
        let choice = Choice::Food(FoodId { kind: 0, build: 0, rank: 0 });
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert!(matches!(&result.tag, Some(Tag::New(text)) if text == "PERMANENT"));
        assert_eq!(result.kind, super::super::food::KINDS[0].one);
        assert_eq!(result.pips, Some((0, 1, super::super::food::SLOTS)));
        assert_eq!(result.footer, format!("Fills your {} slot", super::super::food::KINDS[0].slot));
        assert!(result.rows.iter().all(|row| row.before.is_none()));
    }

    #[test]
    fn food_upgrade_compares_the_rank_it_replaces() {
        let held = FoodId { kind: 0, build: 0, rank: 0 };
        let next = FoodId { rank: 1, ..held };
        let mut run = Run::default();
        run.food[0] = Some(held);
        let choice = Choice::Food(next);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert!(matches!(&result.tag, Some(Tag::Rank(text)) if text == "RANK 1 → 2"));
        assert_eq!(result.footer, format!("Upgrades {}", held.food().name));
        assert_eq!(result.rows[0].before.as_deref(), Some("+1.0%"));
        assert_eq!(result.rows[0].after, "+2.5%");
        assert!(result.pips.is_none());

        let top = FoodId { rank: 4, ..held };
        run.food[0] = Some(FoodId { rank: 3, ..held });
        let choice = Choice::Food(top);
        let result = describe(choice, &run, &describe_card(choice, &run, None));
        assert!(result.footer.starts_with("Final upgrade of"));
    }

    #[test]
    fn a_banished_food_comes_back_after_its_chest() {
        let mut run = Run::default();
        let id = FoodId { kind: 0, build: 0, rank: 0 };
        run.hand.banned.insert(super::super::cards::RewardKey::Food(id));
        run.hand.banned.insert(super::super::cards::RewardKey::Spell("holy_shock"));
        run.hand.forget_banished_food();
        assert_eq!(run.hand.banned.len(), 1);
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
        assert!(matches!(&result.tag, Some(Tag::Rank(rank)) if rank.contains(" → ")));
        assert!(result.rows.iter().any(|row| row.label == "DAMAGE"));
        assert!(result.rows.iter().any(|row| row.label == "TARGETS"));
        assert!(result.rows.iter().all(|row| row.before.is_some()));
    }
}
