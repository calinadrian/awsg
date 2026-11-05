import fs from 'node:fs';

const path = 'crates/asr-coop/assets/economy_catalog.json';
const data = fs.readFileSync(path, 'utf8');
const catalog = JSON.parse(data);
const baseline = JSON.parse(fs.readFileSync('scripts/early_zone_loot_baseline.json', 'utf8'));
const source = fs.readFileSync('crates/benilla-app/src/survivors/data.rs', 'utf8');
const elwynnSource = fs.readFileSync('crates/benilla-app/src/survivors/elwynn.rs', 'utf8');
const classes = [...source.matchAll(/ClassDef\s*\{([\s\S]*?)\n\s*\},/g)].map(([, body]) => ({
  id: Number(body.match(/id:\s*(\d+)/)?.[1]), spell: body.match(/start:\s*"([^"]+)"/)?.[1],
})).filter((x) => x.id && x.spell);
const byId = new Map(catalog.items.map((item) => [item.id, item]));
const dropDef = (zone, drop) => byId.get(drop.item);
const eligible = (item, classId) => item && item.classes.includes(classId);
const transition = { duskwood: 'uncommon', barrens: 'uncommon', deadmines: 'rare', wailing: 'rare', stranglethorn: 'rare' };
const qualityFloor = {
  westfall: 'uncommon', duskwood: 'uncommon', barrens: 'uncommon', stonetalon: 'uncommon',
  deadmines: 'rare', wailing: 'rare', stranglethorn: 'rare',
};
catalog.sets ??= {};
for (const [zoneId, equipment] of Object.entries(baseline)) {
  catalog.zones[zoneId].equipment = structuredClone(equipment);
}
// These three retail starting areas are present in Map.dbc but do not yet have native drop
// catalogs. Reuse the existing common starter pools until their own creature loot is imported.
for (const [id, name, templateId] of [
  ['teldrassil', 'Teldrassil', 'elwynn'],
  ['mulgore', 'Mulgore', 'durotar'],
  ['tirisfal', 'Tirisfal Glades', 'elwynn'],
  ['dun-morogh', 'Dun Morogh', 'elwynn'],
]) {
  const template = catalog.zones[templateId];
  catalog.zones[id] = { ...structuredClone(template), name, chapter: 1 };
}
for (const item of catalog.items) {
  if (item.quality !== 'common') item.quality = 'common';
  delete item.set_name;
}
for (const [zoneId, quality] of Object.entries(transition)) {
  const zone = catalog.zones[zoneId];
  if (!zone) continue;
  const existingClassItems = zone.equipment.map((drop) => ({ drop, item: dropDef(zone, drop) }))
    .filter(({ item }) => item?.spell_damage_bp && Object.keys(item.spell_damage_bp).length);
  const assigned = new Set();
  for (const cls of classes) {
    let selected = existingClassItems.find(({ item }) => item.classes.includes(cls.id));
    if (!selected) {
      const candidates = zone.equipment.map((drop) => ({ drop, item: dropDef(zone, drop) }))
        .filter(({ item }) => eligible(item, cls.id) && !assigned.has(item.id))
        .sort((a, b) => a.item.classes.length - b.item.classes.length || a.item.name.localeCompare(b.item.name));
      selected = candidates[0];
      if (!selected) continue;
      selected.item.classes = [cls.id];
      selected.item.spell_damage_bp = {};
    }
    assigned.add(selected.item.id);
    selected.item.quality = quality;
    selected.item.spell_damage_bp = { [cls.spell]: quality === 'uncommon' ? 500 : 1000 };
    selected.item.base_bonuses = { max_health: zoneId === 'stranglethorn' ? 25 : 20, damage_reduction_bp: zoneId === 'stranglethorn' ? 25 : 20 };
  }
  for (const drop of zone.equipment) {
    const item = dropDef(zone, drop);
    if (item?.spell_damage_bp && Object.keys(item.spell_damage_bp).length) continue;
    item.quality = 'common';
  }
}

// From Westfall onward, every entry meets that area's minimum quality.
const rank = { common: 0, uncommon: 1, rare: 2 };
for (const [zoneId, floor] of Object.entries(qualityFloor)) {
  const zone = catalog.zones[zoneId];
  if (!zone) continue;
  for (const drop of zone.equipment) {
    const item = byId.get(drop.item);
    if (item && rank[item.quality] < rank[floor]) item.quality = floor;
  }
}

// Keep one usable item per armor slot and class in the first two chapters, plus two weapons.
const armorSlots = ['head', 'shoulders', 'chest', 'wrists', 'hands', 'belt', 'legs', 'feet'];
const weaponSlots = new Set(['main_hand', 'off_hand', 'ranged']);
const armorFor = new Map([[1, 'plate'], [2, 'plate'], [3, 'mail'], [7, 'mail'], [4, 'leather'], [11, 'leather'], [5, 'cloth'], [8, 'cloth'], [9, 'cloth']]);
for (const zoneId of ['elwynn', 'durotar', 'westfall', 'barrens', 'teldrassil', 'mulgore', 'tirisfal', 'dun-morogh']) {
  const zone = catalog.zones[zoneId];
  if (!zone) continue;
  const keep = new Set();
  const canArmor = (item, classId) => {
    const allowedKind = item.armor === armorFor.get(classId)
      || ([1, 2].includes(classId) && item.armor === 'mail');
    const allowedClass = item.classes.includes(classId) || ([1, 2].includes(classId) && item.armor === 'mail');
    return armorSlots.includes(item.slot) && allowedKind && allowedClass;
  };
  for (const slot of armorSlots) {
    const pending = new Set(classes.map(({ id }) => id));
    while (pending.size) {
      const candidate = zone.equipment.map((drop) => ({ drop, item: byId.get(drop.item) }))
        .filter(({ item }) => item && item.slot === slot)
        .map((entry) => ({
          ...entry,
          covered: [...pending].filter((classId) => canArmor(entry.item, classId)),
        }))
        .filter(({ covered }) => covered.length)
        .sort((a, b) => b.covered.length - a.covered.length
          || Number(keep.has(b.drop.item)) - Number(keep.has(a.drop.item))
          || a.item.name.localeCompare(b.item.name))[0];
      if (!candidate) break;
      keep.add(candidate.drop.item);
      for (const classId of candidate.covered) {
        if ([1, 2].includes(classId) && candidate.item.armor === 'mail' && !candidate.item.classes.includes(classId)) {
          candidate.item.classes.push(classId);
        }
        pending.delete(classId);
      }
    }
  }
  const weaponRows = zone.equipment.map((drop) => ({ drop, item: byId.get(drop.item) }))
    .filter(({ item }) => item && weaponSlots.has(item.slot));
  const classWeapons = [
    weaponRows.filter(({ item }) => item.slot === 'main_hand' && !item.two_handed && [1, 2, 7].every((id) => item.classes.includes(id))),
    weaponRows.filter(({ item }) => item.slot === 'off_hand' && /shield/i.test(item.icon ?? '') && [1, 2, 7].every((id) => item.classes.includes(id))),
  ].map((rows) => rows.sort((a, b) => a.item.name.localeCompare(b.item.name))[0]).filter(Boolean);
  const weapons = classWeapons.length === 2 ? classWeapons : weaponRows
    .sort((a, b) => a.item.name.localeCompare(b.item.name)).slice(0, 2);
  for (const { drop } of weapons) keep.add(drop.item);
  zone.equipment = zone.equipment.filter((drop) => keep.has(drop.item));
}

// Give every named boss a single zone item it can award. The server binds these names to
// boss deaths, so a boss no longer rolls an arbitrary item from the whole zone pool.
const bossLists = new Map();
const mapBlocks = [...source.matchAll(/MapDef\s*\{([\s\S]*?)\n\s*\},/g)].map(([, body]) => ({
  name: body.match(/name:\s*"([^"]+)"/)?.[1],
  bosses: [...(body.match(/bosses:\s*&\[([\s\S]*?)\n\s*\]/)?.[1] ?? '').matchAll(/(?:foe|giant)\("([^"]+)"/g)].map(([, name]) => name),
}));
const globalBosses = [...source.matchAll(/\.\.en\("([^"]+)",\s*\d+,\s*[\d.]+,\s*[\d.]+,\s*[\d.]+,\s*\d+,\s*\d+\)/g)].map(([, name]) => name).slice(0, 6);
const zoneIdByMap = new Map([
  ['Dun Morogh', 'dun-morogh'],
  ['Durotar', 'durotar'], ['Teldrassil', 'teldrassil'], ['Mulgore', 'mulgore'],
  ['Tirisfal Glades', 'tirisfal'], ['The Barrens', 'barrens'], ['Westfall', 'westfall'], ['Duskwood', 'duskwood'],
  ['Stonetalon Mountains', 'stonetalon'], ['Stranglethorn Vale', 'stranglethorn'],
  ['The Deadmines', 'deadmines'], ['Wailing Caverns', 'wailing'],
]);
for (const { name, bosses } of mapBlocks) {
  const zoneId = zoneIdByMap.get(name);
  if (zoneId && bosses.length) bossLists.set(zoneId, bosses);
}
bossLists.set('elwynn', [...elwynnSource.matchAll(/"(Hogger|Mother Fang|Princess|Morgan the Collector)"/g)].map(([, name]) => name)
  .filter((name, index, all) => all.indexOf(name) === index));
for (const zoneId of Object.keys(catalog.zones)) {
  const bosses = bossLists.get(zoneId) ?? globalBosses;
  const equipment = catalog.zones[zoneId].equipment;
  const candidates = equipment.map((drop) => ({ drop, item: byId.get(drop.item) }))
    .filter(({ item }) => item)
    .sort((a, b) => b.item.classes.length - a.item.classes.length || a.item.name.localeCompare(b.item.name));
  catalog.zones[zoneId].boss_drops = bosses.map((boss, index) => ({ boss, item: candidates[index % candidates.length].item.id }));
}

// Give each Stranglethorn class a second, compatible rare set piece with a two-piece bonus.
const stv = catalog.zones.stranglethorn;
for (const cls of classes) {
  const first = stv.equipment.map((drop) => ({ drop, item: byId.get(drop.item) }))
    .find(({ item }) => item?.quality === 'rare' && item.classes.includes(cls.id) && item.spell_damage_bp?.[cls.spell]);
  if (!first) continue;
  const second = stv.equipment.map((drop) => ({ drop, item: byId.get(drop.item) }))
    .filter(({ item }) => eligible(item, cls.id) && item.id !== first.item.id && item.slot !== first.item.slot
      && !['finger_1', 'finger_2', 'trinket_1', 'trinket_2'].includes(item.slot))
    .sort((a, b) => a.item.classes.length - b.item.classes.length || a.item.name.localeCompare(b.item.name))[0];
  if (!second) continue;
  const setName = `Jungle Stalker (${cls.id})`;
  second.item.quality = 'rare';
  second.item.classes = [cls.id];
  second.item.set_name = setName;
  second.item.spell_damage_bp = {};
  first.item.set_name = setName;
  catalog.sets[setName] = { pieces: 2, spell_damage_bp: { [cls.spell]: 500 } };
}

// Keep both pieces discoverable at a modest share of each class's pool.
for (const cls of classes) {
  const classDrops = stv.equipment.filter((drop) => byId.get(drop.item)?.classes.includes(cls.id));
  const rare = classDrops.filter((drop) => byId.get(drop.item)?.quality === 'rare');
  const commonWeight = classDrops.filter((drop) => byId.get(drop.item)?.quality === 'common')
    .reduce((sum, drop) => sum + drop.weight, 0);
  if (rare.length === 2 && commonWeight) {
    rare[0].weight = Math.max(1, Math.round(commonWeight / 18));
    rare[1].weight = Math.max(1, Math.round(commonWeight / 18));
  }
}

// Other tiered zones get one class signature item each; the base pool remains common.
for (const [zoneId, quality] of Object.entries(transition)) {
  if (zoneId === 'stranglethorn') continue;
  const zone = catalog.zones[zoneId];
  for (const cls of classes) {
    const item = zone.equipment.map((drop) => byId.get(drop.item)).find((entry) => entry?.classes.length === 1
      && entry.classes[0] === cls.id && entry.spell_damage_bp?.[cls.spell]);
    if (!item) continue;
    const drop = zone.equipment.find((entry) => entry.item === item.id);
    const classWeight = zone.equipment.filter((entry) => byId.get(entry.item)?.classes.includes(cls.id))
      .reduce((sum, entry) => sum + entry.weight, 0);
    drop.weight = Math.max(1, Math.round(classWeight / (quality === 'rare' ? 9 : 9)));
  }
}

fs.writeFileSync(path, `${JSON.stringify(catalog, null, 2)}\n`.replace(/\r?\n/g, '\r\n'));
console.log('Applied common, uncommon, rare, and Stranglethorn set loot progression.');
