// Gear power by zone tier: enemies grow ×1.4 health per tier, and a full set of a tier's gear is
// what lets a hero keep up. Every item's numbers come from its zone's tier, its quality and its
// slot alone, so this script can be rerun after configure_zone_quality_progression.mjs.
//
// Targets are for a full on-tier set at the tier's floor quality, at +0 and at +5:
//   - damage_bp multiplies the hero's damage. At +0 a tier-k set leaves the hero at about 95% of
//     the horde's health growth (with talents at ×1.08 a tier); at +5 it is ×1.21 that, ahead of
//     this tier and about 89% of the next.
//   - max_health is flat health, about 450 being a mid-run hero; damage_reduction_bp adds to
//     the hero's reduction.
import fs from 'node:fs';

const path = 'crates/asr-coop/assets/economy_catalog.json';
const catalog = JSON.parse(fs.readFileSync(path, 'utf8'));

const TIER = {
  elwynn: 1, durotar: 1, teldrassil: 1, mulgore: 1, tirisfal: 1, 'dun-morogh': 1,
  westfall: 2, barrens: 2, deadmines: 3, wailing: 3, duskwood: 4, stonetalon: 4, stranglethorn: 5,
};
const QUALITY = ['common', 'uncommon', 'rare', 'epic'];
const POWER = { common: 1.0, uncommon: 1.15, rare: 1.32, epic: 1.52 };
const FLOOR = [null, 'common', 'uncommon', 'rare', 'rare', 'rare'];
const TARGET = {
  damage_bp: [[1000, 2000], [2300, 4880], [6000, 9360], [10700, 15050], [16800, 22430]],
  critical_chance_bp: [[100, 200], [200, 300], [300, 450], [400, 600], [500, 750]],
  cooldown_reduction_bp: [[100, 200], [200, 300], [300, 450], [400, 600], [500, 750]],
  max_health: [[36, 135], [68, 171], [113, 225], [158, 284], [203, 338]],
  damage_reduction_bp: [[200, 400], [400, 700], [700, 1000], [1000, 1400], [1300, 1800]],
};
const OFFENCE = new Set(['damage_bp', 'critical_chance_bp', 'cooldown_reduction_bp']);
const CLASSES = [1, 2, 3, 4, 5, 7, 8, 9, 11];
const ARMOR = { 1: 'plate', 2: 'plate', 3: 'mail', 7: 'mail', 4: 'leather', 11: 'leather', 5: 'cloth', 8: 'cloth', 9: 'cloth' };
const ARMOR_SLOTS = ['head', 'shoulders', 'chest', 'wrists', 'hands', 'belt', 'legs', 'feet'];
const WEAPON_SLOTS = new Set(['main_hand', 'off_hand', 'ranged']);

const weapon = (item) => WEAPON_SLOTS.has(item.slot);
// How much of a set's offence or defence a slot carries.
function weight(item, stat) {
  if (OFFENCE.has(stat)) {
    if (item.slot === 'main_hand') return item.two_handed ? 0.9 : 0.65;
    if (item.slot === 'off_hand') return 0.35;
    if (item.slot === 'ranged') return 0.1;
    return 0;
  }
  return {
    head: 1.2, chest: 1.2, legs: 1.2, shoulders: 0.9, hands: 0.9, feet: 0.9, belt: 0.9,
    wrists: 0.7, back: 0.7, neck: 0.7, finger_1: 0.7, trinket_1: 0.7, shirt: 0.2,
    off_hand: 0.8, main_hand: 0.5, ranged: 0.5,
  }[item.slot] ?? 0;
}
// Every weapon adds to damage; each item keeps its other stats.
const stats = (item) => [...new Set([...Object.keys(item.base_bonuses), ...(weapon(item) ? ['damage_bp'] : [])])];

// The game's Definition::can_use.
function canUse(item, cls) {
  if (!item.classes.includes(cls)) return false;
  if ([1, 2, 7].includes(cls)) {
    if (item.slot === 'ranged') return false;
    if (item.slot === 'main_hand' && item.two_handed) return false;
    if (item.slot === 'off_hand' && !item.icon.toLowerCase().includes('shield')) return false;
  }
  if (!ARMOR_SLOTS.includes(item.slot)) return true;
  return !item.armor || item.armor === ARMOR[cls] || ([1, 2].includes(cls) && item.armor === 'mail');
}

// Hall of Thanes has no creature loot of its own yet: it shares Westfall's tier-2 pool, and each
// of its four bosses binds one of that pool's boss items.
if (!catalog.zones['hall-of-thanes']) {
  const westfall = catalog.zones.westfall;
  const bosses = ['Faldrim Anvilmar', 'Magmatus', 'Plunder', 'Durgen Dirgehammer'];
  catalog.zones['hall-of-thanes'] = {
    ...structuredClone(westfall),
    name: 'Hall of Thanes',
    boss_drops: bosses.map((boss, i) => ({ boss, item: westfall.boss_drops[i % westfall.boss_drops.length].item })),
  };
}
TIER['hall-of-thanes'] = 2;

const byId = new Map(catalog.items.map((item) => [item.id, item]));
const tierOf = (item) => TIER[item.zone] ?? 1;

// The first zones' pools keep two weapons, which left casters, druids and hunters none to find.
// Every class gets one weapon it can use from the zone's own items: a ranged one for hunters.
for (const zone of Object.values(catalog.zones)) {
  const pool = () => zone.equipment.map((drop) => byId.get(drop.item)).filter(Boolean);
  const home = pool()[0]?.zone;
  const weights = zone.equipment.map((drop) => drop.weight).sort((a, b) => a - b);
  const weight = weights[Math.floor(weights.length / 2)] ?? 1;
  for (const cls of CLASSES) {
    const slot = cls === 3 ? 'ranged' : 'main_hand';
    if (pool().some((item) => item.slot === slot && canUse(item, cls))) continue;
    const pick = catalog.items
      .filter((item) => item.zone === home && item.slot === slot && canUse(item, cls))
      .sort((a, b) => b.classes.length - a.classes.length || a.name.localeCompare(b.name))[0];
    if (pick) zone.equipment.push({ item: pick.id, weight });
  }
}
const rank = (quality) => Math.max(0, QUALITY.indexOf(quality));

// Duskwood and Stonetalon are tier 4: rare is their floor, like the dungeons before them.
for (const [zoneId, zone] of Object.entries(catalog.zones)) {
  zone.tier = TIER[zoneId] ?? 1;
  const floor = FLOOR[zone.tier];
  for (const drop of zone.equipment) {
    const item = byId.get(drop.item);
    if (item && rank(item.quality) < rank(floor)) item.quality = floor;
  }
  // Elites of the dungeons and beyond drop gear twice as often.
  if (zone.tier >= 3) zone.equipment_chance_bp.elite = 1000;
}
// A named boss's item is a step above its zone's floor.
for (const zone of Object.values(catalog.zones)) {
  const step = Math.min(rank(FLOOR[zone.tier]) + 1, QUALITY.length - 1);
  for (const drop of zone.boss_drops ?? []) {
    const item = byId.get(drop.item);
    if (item && rank(item.quality) < step) item.quality = QUALITY[step];
  }
}

// The best set weight a class can gather from one zone, for one stat.
function setWeight(zone, cls, stat) {
  const items = zone.equipment.map((drop) => byId.get(drop.item))
    .filter((item) => item && canUse(item, cls) && stats(item).includes(stat));
  const best = (filter) => Math.max(0, ...items.filter(filter).map((item) => weight(item, stat)));
  let total = 0;
  for (const slot of [...ARMOR_SLOTS, 'back', 'neck', 'shirt']) total += best((item) => item.slot === slot);
  total += 2 * best((item) => item.slot === 'finger_1');
  total += 2 * best((item) => item.slot === 'trinket_1');
  const oneHand = best((item) => item.slot === 'main_hand' && !item.two_handed)
    + best((item) => item.slot === 'off_hand');
  const twoHand = best((item) => item.slot === 'main_hand' && item.two_handed);
  return total + Math.max(oneHand, twoHand) + best((item) => item.slot === 'ranged');
}

// One unit of each stat per tier: the target spread over what a class can wear there.
const unit = {};
for (const tier of [1, 2, 3, 4, 5]) {
  const zones = Object.values(catalog.zones).filter((zone) => zone.tier === tier);
  unit[tier] = {};
  const average = (stat) => {
    const weights = zones.flatMap((zone) => CLASSES.map((cls) => setWeight(zone, cls, stat))).filter((w) => w > 0);
    return weights.length ? weights.reduce((a, b) => a + b, 0) / weights.length : 0;
  };
  for (const stat of Object.keys(TARGET)) {
    // A stat no droppable item carries here still prices the zone's other items by damage's slots.
    const spread = average(stat) || average(OFFENCE.has(stat) ? 'damage_bp' : 'max_health');
    unit[tier][stat] = TARGET[stat][tier - 1][0] / (spread * POWER[FLOOR[tier]]);
    // A tier whose zones fill more slots still makes each piece better than the tier below.
    if (tier > 1) unit[tier][stat] = Math.max(unit[tier][stat], unit[tier - 1][stat] * 1.1);
  }
}

for (const item of catalog.items) {
  const tier = tierOf(item);
  const at = (upgrade) => Object.fromEntries(stats(item).map((stat) => {
    const [base, top] = TARGET[stat][tier - 1];
    const value = unit[tier][stat] * weight(item, stat) * POWER[item.quality] * (1 + (top / base - 1) * upgrade / 5);
    return [stat, Math.max(1, Math.round(value))];
  }));
  item.base_bonuses = at(0);
  item.upgrades.forEach((upgrade, index) => { upgrade.bonuses = at(index + 1); });
}

fs.writeFileSync(path, `${JSON.stringify(catalog, null, 2)}\n`.replace(/\r?\n/g, '\r\n'));
for (const tier of [1, 2, 3, 4, 5]) {
  console.log(`tier ${tier}`, Object.fromEntries(Object.entries(unit[tier]).map(([k, v]) => [k, +v.toFixed(2)])));
}
