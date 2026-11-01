# Elwynn NPC visual upgrade plan

The behavior reference is the user's installed `D:/Personal/ASR/ASR.exe` and
its data. Retail supplies replacement visuals. This checkout's implementation
is not authoritative when it differs from that installation. This is a plan;
implementation and verification are tracked against the acceptance criteria below.

The agreed roster exception is to keep Hogger as a boss and move Fedfennel
into the fourth elite slot. Preserve the source's other behavior, particularly
the regular mobs' spawning and pursuit and the encounter scripts. The user
prefers how those work in the reference installation.

## Reference and evidence

The executable inspected for this plan has SHA-256
`2e68f9867c53937a08e42fec5c933104e6d70c10295f9ba7866c97310898cc50`.
Its active zone catalog identifies Elwynn and contains 19 zones. A read-only
snapshot of its Elwynn records, including raw record bytes, is staged in
`out/retail-elwynn-npcs/asr-behavior-reference.json`.

Table entry times below are verified static values. Their runtime interpretation,
spawn frequency, AI and phase contents still require auditing. Neither table
membership nor a phase count alone proves what happens during play.

## Regular mobs

Keep the source entry schedule and reproduce the source's spawn selection,
counts, placement, scaling and attacks. Do not introduce new wave weights,
species-specific reinforcements, density caps or movement mechanics as part of
the visual conversion.

| First entry | NPC | Extracted visual candidate |
|---|---|---|
| 0:00 | Kobold Vermin | Kobold2, normal skin candidate |
| 0:30 | Young Wolf | WolfDraenor, timber-like skin candidate |
| 1:12 | Murloc Streamrunner | Murloc2, matching skin to be verified |
| 2:00 | Kobold Geomancer | Kobold2, albino skin candidate |
| 3:00 | Defias Thug | Human male HD with original NPC clothing/appearance |
| 4:30 | Riverpaw Brute | Gnoll2, Riverpaw appearance candidate |
| 6:00 | Prowler | WolfDraenor, dark skin candidate |
| 7:30 | Defias Rogue Wizard | Human female HD with original NPC clothing/appearance |
| 9:00 | Defias Enforcer | Human male HD with original NPC clothing/appearance |

Preserve Fireball/Frostbolt identities and verify their actual source ranges,
cooldowns, projectile timing and movement behavior. Preserve every melee attack's
timing and reach. Separate visual animation timing from the authoritative damage
schedule so changing an animation does not change combat.

## Four elites and four bosses

The requested presentation is four elites and four bosses. The source's raw
tables contain three elite-flagged enemy records and five boss records:

| Source table | Entry | Entry time | Visual candidate |
|---|---|---|---|
| Elite enemy | Kobold Taskmaster | 3:30 | Kobold2, distinct original appearance |
| Elite enemy | Gruff Swiftbite | 8:00 | Requires identity verification; retain source species |
| Elite enemy | Bellygrub | 13:00 | Boar2, brown skin candidate |
| Boss | Hogger | 3:00 | Dedicated retail Hogger |
| Boss | Mother Fang | 6:00 | MineSpiderBoss or reviewed newer Spider alternative |
| Boss | Fedfennel | 9:00 | Gnoll2Caster |
| Boss | Princess | 12:00 | Boar2, original appearance distinct from Bellygrub |
| Boss, final record | Morgan the Collector | 16:00 | Human male HD, original NPC clothing/appearance |

This checkout uses Taskmaster, Gruff, Fedfennel and Bellygrub as four lair
guardians. Taskmaster, Gruff and Bellygrub's lairs first fill at 0:20,
independently of enemy-table unlock times. Fedfennel retains his one-time 9:00
source encounter in the fourth elite slot. Their repeating kits and normal-mode lair rules are read from the
installed reference; `guardian_behavior.rs` owns those rules.

The agreed target roster is:

| Role | NPC | Timing policy |
|---|---|---|
| Elite | Kobold Taskmaster | Preserve source availability; table entry 3:30 |
| Elite | Gruff Swiftbite | Preserve source availability; table entry 8:00 |
| Elite | Fedfennel | Reclassify as elite; one-time source entry at 9:00 |
| Elite | Bellygrub | Preserve source availability; table entry 13:00 |
| Boss | Hogger | Preserve 3:00 boss event |
| Boss | Mother Fang | Preserve 6:00 boss event |
| Boss | Princess | Preserve 12:00 boss event |
| Final boss | Morgan the Collector | Preserve 16:00 boss event and source victory rules |

Fedfennel's reclassification is the explicit exception to strict behavior
parity. Do not automatically give him this checkout's lair respawns, extra
abilities, reinforcement script or global guardian scaling. Determine what
the role change needs in presentation, reward handling and progression after
auditing the source; it does not authorize retuning his stats or attacks.

Gruff currently names a wolf display (246), whereas the initial extraction
selected a modern gnoll from the NPC name. Resolve the installed display through
the reference's actual MPQ chain before choosing its replacement. A name-based
species correction is outside this visual-only plan.

## Boss mechanics

Copy the source's phase definitions and runtime rules exactly, including
thresholds, delays, cooldowns, summons, dialogue, rewards and the win condition.
Do not redesign their kits based on the retail model.

- Hogger's source record has one phase entry. Decode it and its runtime role.
- Mother Fang and Fedfennel each have zero table phase entries. This checkout
  adds poison/charge and torch/stomp/charge scripts respectively. Audit generic
  source AI before deciding which behavior to restore; zero phase entries does
  not prove zero abilities.
- Princess and Morgan each have two source phase entries. Compare their exact
  contents with the current implementation before changing anything, including
  dialogue that appears mismatched to the model.
- Confirm boss overlap rules and the final-boss victory path. Do not introduce
  queues, pauses, despawns or automatic victory on a generic timer.

## Spawn, pursuit and script fidelity

Audit these before visual integration. The goal is the reference's observable
behavior, including the aspects the user finds better; do not substitute a new
AI design or describe current approximations as verified source behavior.

| Area | Compare against the installed reference |
|---|---|
| Mob composition | Unlock times, pool rotation, selection probabilities, packs and reinforcement composition |
| Spawn pressure | Spawn cadence, live counts, refill bursts, ring events and scaling through the run |
| Spawn placement | Distance and direction from the player, visibility, clear ground, terrain and obstacle checks |
| Pursuit | Reaction to player movement, approach direction, turns, speed, spacing and surrounding behavior |
| Terrain traversal | Trees, fences, buildings, water, slopes and how stuck mobs recover |
| Far-away mobs | Whether and when they relocate, despawn or continue pursuing; placement after recovery |
| Melee | Attack initiation distance, stop distance, attack interval, damage timing and crowd contact |
| Casters | Pursuit versus stopping to cast, range checks, cooldowns, cast delays and projectile hits |
| Control effects | Slow, root, stun, fear and their effect on pursuit, casting and scripted moves |
| Elite behavior | Availability, aggro, leash, return/healing, summons, death rewards and any respawn |
| Encounter scripts | Phase thresholds, action order, first-use delays, repeats, summons, dialogue and cleanup |

Current code inspection identifies specific comparisons, not proven defects:
`enemies.rs` spawns the regular horde on a 34–42 yard ring, refills at 0.12-second intervals,
and relocates non-lair enemies beyond 75 yards to roughly 38 yards from the
player. Its chase uses local separation. These rules still need comparison with the
reference instead of assuming they explain the user's observation.

Source-derived selection and approach rules are implemented in
`elwynn_behavior.rs`: all unlocked regular mobs retain weighted eligibility,
with weight `min(1 + age_minutes * 0.5, 4) / budget_cost` (RVA 0x5c8a2e0).
Melee pursuit now uses individual positions around the player, 14-yard maximum
move segments, native 0.15-radian angle updates and 0.25–0.7-second refresh
intervals (RVAs 0x5c0bb91–0x5c0c33b). WoW/Bevy axis handedness is accounted for.
The slot radius is clamped inside this engine's existing melee contact radius
so small mobs can still attack; this is an explicit adaptation, not exact
collision parity. Casters retain the source table's full 18-yard range.

The regular Elwynn horde samples the reference's base scaling at spawn
(RVAs 0x5c00340 and 0x5c6a330), with `t` in elapsed minutes:
`HP = base_hp * 4.95 * 0.82 * (1 + 0.10t + 0.036t² + 0.00704t³)` and
`damage = base_damage * 1.25 * sqrt(0.82) * (1 + 0.07t + 0.015t² + 0.0002t³)`.
Movement speed is `base_speed * 1.05`, and model size stays at its table value.
Regular enemies already alive are not rescaled. These formulas apply to the
nine regular entries, including summoned copies; bosses, lair guardians and
other maps retain their separate rules. Difficulty, co-op and endless modifiers
are excluded from this base-mode port.

The agreed four-boss/four-elite roster is implemented, with Hogger's source
3:00 entry and 3,600 base HP. The three source lair guardians first become available at 0:20,
independently of the enemy table's entry times: the installed reference initializes
each lair's fill time to 20 seconds at RVA 0x5bf95dd, and the lair update compares
run time against that field at RVA 0x5c663b2. Each lair waits for its own ground
to stream in. Fedfennel enters once at 9:00. Normal-mode respawns for the other
three wait 180 seconds after a guardian dies
(RVA 0x5c03783).

Guardian behavior comes from the installed reference's per-creature lookup at
RVA 0x5ab4a20 and action interpreter at 0x5c26e80. Taskmaster charges every
9 seconds, bombards with three Fireballs every 10, and summons four minions
every 15. Gruff chains two charges 1.6 seconds apart every 11 seconds, cleaves
in a 90-degree cone every 7, and summons three minions every 16. Bellygrub
stomps every 9 seconds, leaves a six-second ground hazard every 12, and charges
every 9. Fedfennel retains his source boss kit: an 80-degree cone every 7
seconds, stomp every 10, charge every 11, and eight minions every 14.
Cooldowns start at half their period and reset on evade; below 30% health,
the next period is 70% of normal.

The native lair state machine (RVA 0x5b17bb0) wakes a guardian when damaged,
or when it is within 3 yards of home and the player comes within 20 yards.
It disengages beyond 97.5 yards from home, immediately heals and clears its
combat state, and returns home. Guardian health and damage use the normal-mode
source curves (RVAs 0x5c67730 and 0x5c67c20), Elwynn's 0.82 multiplier,
and additive per-kill growth of 25% health and 5% damage. These curves are
evaluated at spawn and on the wake event (RVA 0x5c0d688), not every frame.
Before 3:00, the opening boss's health floor ramps from 60% to 100%
(RVA 0x5c67ad7). Health percentage is preserved when waking increases the
maximum; an engaged guardian does not continuously gain HP. Fedfennel's roster
exception keeps his original boss base stats of 17,000 HP and 38 damage without
applying the guardian stat curve. A guardian's hoard offers jewelry.

Engine adaptations remain: fixed four-lair placement and roster (including the
agreed Fedfennel exception), Bevy terrain/collision and spell visuals, and the
existing jewelry rarity policy. Pending attacks are cleared on evade/death;
ground hazards tick once per second. Difficulty/affix variants and live behavior
parity have not been established.

The source script audit confirmed separate 56-byte repeating ability records
and 72-byte phase records. Both lists, including nested sequences, are captured
in the ignored reference JSON. Princess's phase actions replace ability lists;
they are not simply the current repeated stomp/whirlwind approximation.
Hogger's base kit includes a charge/cleave sequence and summons. Mother Fang's
and Fedfennel's zero phase counts do not imply empty base kits. Full script
porting and runtime comparison remain outstanding; existing scripts must not
be described as faithful to the source yet.

Use repeatable player routes for a straight run, a turn back through the horde,
circling a pack and traversing an obstacle-rich section. Compare time-stamped
positions, live counts, attack events and script events at matched progression
and stats. A screenshot alone cannot establish chase or script fidelity.
Any live combat comparison must follow repository unattended-probe restrictions;
use user-driven reference play for combat observations. Do not launch scripted
combat probes or write test state into the reference installation.

## Implementation sequence

1. Finish the reference audit: spawn pool and density, elite activation and
   respawn rules, movement and attack AI, boss phases, loot and victory.
   Record each finding as verified data/code or still needing runtime evidence.
2. Build a parity matrix against this checkout. Correct behavior differences
   in a separate, explicit pass before attaching new visuals. Preserve the
   reference installation, executable and saves throughout.
3. Convert and inspect creature families first, then the human NPCs and their
   composed clothing. Choose skins from the staged assets after inspecting them;
   exact retail display matches remain unverified because the local display DB2
   is encrypted.
4. Assign private visual display/model IDs. Preserve gameplay identity, stats,
   collision, hit reach, spawn location and rewards. Match world-space body size
   against the source; do not copy the old model's numeric scale onto a new mesh.
   Keep elite/boss recognition through the original silhouette and appropriate
   visual scale rather than new combat mechanics.
5. Validate Stand/Walk/Run/Attack/Cast/Death, textures, geosets, attachments,
   animation transitions and feet placement. Verify normal, elite and boss
   versions separately even when they share a model.
6. Compare deterministic spawn and combat traces with the reference, and check
   representative visual scenes at the existing camera. Validate performance
   under the preserved spawn load. Optimize visual LOD/materials if needed.

Acceptance requires the same observable encounter behavior with upgraded models.
Static extraction is evidence for the listed table values only; runtime parity
is not yet verified. The Hogger/Fedfennel role decision is agreed; broader
balance, encounter redesign and publication are outside this plan. This document
does not claim runtime behavior parity or completed retail visual integration.
