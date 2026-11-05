import fs from 'node:fs';

const catalog = JSON.parse(fs.readFileSync('crates/asr-coop/assets/economy_catalog.json', 'utf8'));
const source = fs.readFileSync('crates/benilla-app/src/survivors/data.rs', 'utf8');
const classes = [...source.matchAll(/ClassDef\s*\{([\s\S]*?)\n\s*\},/g)]
  .map(([, body]) => ({
    id: Number(body.match(/id:\s*(\d+)/)?.[1]),
    name: body.match(/name:\s*"([^"]+)"/)?.[1],
  }))
  .filter(({ id, name }) => Number.isFinite(id) && name);
const className = new Map(classes.map(({ id, name }) => [id, name]));
const items = new Map(catalog.items.map((item) => [item.id, item]));
const armorFor = new Map([[1, 'plate'], [2, 'plate'], [3, 'mail'], [7, 'mail'], [4, 'leather'], [11, 'leather'], [5, 'cloth'], [8, 'cloth'], [9, 'cloth']]);
const armorSlots = new Set(['head', 'shoulders', 'chest', 'wrists', 'hands', 'belt', 'legs', 'feet']);
const canUse = (item, classId) => item.classes.includes(classId)
  && (!(classId === 1 || classId === 2 || classId === 7)
    || (item.slot !== 'ranged'
      && (item.slot !== 'main_hand' || !item.two_handed)
      && (item.slot !== 'off_hand' || /shield/i.test(item.icon ?? ''))))
  && (!armorSlots.has(item.slot) || !item.armor || item.armor === armorFor.get(classId)
    || ((classId === 1 || classId === 2) && item.armor === 'mail'));

const zones = Object.entries(catalog.zones)
  .map(([id, zone]) => ({
    id,
    name: zone.name,
    chapter: zone.chapter,
    equipmentChanceBp: zone.equipment_chance_bp,
    materialChanceBp: zone.material_chance_bp,
    materials: zone.materials,
    materialPool: zone.material_pool,
    bossDrops: (zone.boss_drops ?? []).map((entry) => {
      const item = items.get(entry.item);
      return item ? { boss: entry.boss, item: {
        id: item.id, name: item.name, quality: item.quality, slot: item.slot,
        armor: item.armor, twoHanded: item.two_handed, baseBonuses: item.base_bonuses,
        spellDamageBp: item.spell_damage_bp ?? {}, setName: item.set_name ?? null,
        setBonus: item.set_name ? catalog.sets?.[item.set_name] ?? null : null,
        upgrades: (item.upgrades ?? []).map((upgrade) => ({ level: upgrade.level, cost: upgrade.cost, bonuses: upgrade.bonuses })),
        scrap: item.scrap, classes: item.classes.map((id) => className.get(id)).filter(Boolean),
      } } : null;
    }).filter(Boolean),
    equipment: zone.equipment.flatMap(({ item, weight }) => {
      const def = items.get(item);
      if (!def || weight <= 0) return [];
      const sources = [...new Set((def.provenance ?? [])
        .filter((entry) => entry.type === 'creature_drop' && entry.creature)
        .map((entry) => entry.creature))];
      return [{
        id: def.id,
        name: def.name,
        weight,
        slot: def.slot,
        quality: def.quality,
        armor: def.armor,
        twoHanded: def.two_handed,
        baseBonuses: def.base_bonuses,
        spellDamageBp: def.spell_damage_bp ?? {},
        setName: def.set_name ?? null,
        setBonus: def.set_name ? catalog.sets?.[def.set_name] ?? null : null,
        upgrades: (def.upgrades ?? []).map((upgrade) => ({
          level: upgrade.level,
          cost: upgrade.cost,
          bonuses: upgrade.bonuses,
        })),
        scrap: def.scrap,
        classes: def.classes.filter((classId) => canUse(def, classId)).map((id) => className.get(id)).filter(Boolean),
        sourceCreatures: sources,
      }];
    }),
  }))
  .sort((a, b) => a.chapter - b.chapter || a.name.localeCompare(b.name));

const playableSource = source.match(/pub\(crate\) static MAPS:[\s\S]*?LazyLock::new\(\|\|\s*\{\s*\[([\s\S]*?)\]\s*\.into_iter\(\)/)?.[1] ?? '';
const playableNames = [...playableSource.matchAll(/"([^"]+)"/g)].map(([, name]) => name);
for (const name of playableNames) {
  if (zones.some((zone) => zone.name === name)) continue;
  zones.push({
    id: name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''),
    name,
    chapter: null,
    unconfigured: true,
    equipmentChanceBp: {},
    materialChanceBp: {},
    materials: {},
    materialPool: [],
    equipment: [],
  });
}

const output = { classes: classes.map(({ name }) => name), zones };
fs.writeFileSync(
  'web/zones-data.js',
  `// Generated from economy_catalog.json and survivors/data.rs.\nwindow.ZONE_LOOT_DATA = ${JSON.stringify(output, null, 2)};\n`,
);
process.stdout.write(`Wrote loot tables for ${zones.length} zones and instances.\n`);
