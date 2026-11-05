#!/usr/bin/env node
// Writes the achievement definitions to crates/asr-coop/assets/quest_catalog.json.
// Boss line-ups come from the economy catalog (its boss_drops follow each zone's roster);
// creature targets are named here and checked against the rosters by
// `survivors::quests::tests::catalog_targets_are_roster_names` in benilla-app.
//
// Keep achievement ids stable so existing saves can resolve them.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const economy = JSON.parse(readFileSync(join(root, 'crates/asr-coop/assets/economy_catalog.json'), 'utf8'));
const REVISION = 4;

const ZONES = [
  'elwynn',
  'dun-morogh',
  'teldrassil',
  'durotar',
  'mulgore',
  'tirisfal',
  'westfall',
  'barrens',
  'deadmines',
  'wailing',
  'duskwood',
  'stonetalon',
  'hall-of-thanes',
  'stranglethorn',
  'scarlet-monastery',
  'tanaris',
  'ungoro',
  'burning-steppes',
  'swamp-of-sorrows',
  'zulfarrak',
  'onyxia',
  'molten-core',
];
const ROADS = [
  ['road.alliance', 'The Alliance Road', 'Clear the story road from Elwynn through Deadmines to Stranglethorn.', ['elwynn', 'westfall', 'deadmines', 'stranglethorn']],
  ['road.horde', 'The Horde Road', 'Clear the story road from Durotar through Wailing Caverns to Stranglethorn.', ['durotar', 'barrens', 'wailing', 'stranglethorn']],
  ['road.classic', 'A Classic Story', 'Clear the shared story road from Stranglethorn to Burning Steppes.', ['stranglethorn', 'scarlet-monastery', 'tanaris', 'ungoro', 'burning-steppes']],
  ['road.cradles', 'Cradles of Azeroth', 'Clear all six starting lands.', ['elwynn', 'dun-morogh', 'teldrassil', 'durotar', 'mulgore', 'tirisfal']],
];
const CLASSES = [[1, 'Warrior'], [2, 'Paladin'], [3, 'Hunter'], [4, 'Rogue'], [5, 'Priest'], [7, 'Shaman'], [8, 'Mage'], [9, 'Warlock'], [11, 'Druid']];

const objective = (kind, count, text, extra = {}) => ({ kind, target: '', zone: '', class: 0, count, text, ...extra });

const achievements = [];
for (const id of ZONES) {
  const zone = economy.zones[id];
  if (!zone) throw new Error(`no economy zone ${id}`);
  const bosses = zone.bosses ?? zone.boss_drops.map((b) => b.boss);
  const last = bosses[bosses.length - 1];
  achievements.push({
    id: `conquer.${id}`, category: 'Zones', title: `Conqueror of ${zone.name}`,
    text: `Defeat ${last} and clear ${zone.name}.`, points: 10,
    criteria: [objective('clear', 1, `Clear ${zone.name}`, { zone: id })], requires: [],
  });
  achievements.push({
    id: `wanted.${id}`, category: 'Bosses', title: `Most Wanted: ${zone.name}`,
    text: `Defeat every boss in ${zone.name}.`, points: 10,
    criteria: bosses.map((boss) => objective('boss', 1, boss, { target: boss, zone: id })), requires: [],
  });
}
for (const [id, title, text, zones] of ROADS) {
  achievements.push({ id, category: 'Zones', title, text, points: 25, criteria: [], requires: zones.map((z) => `conquer.${z}`) });
}
achievements.push({
  id: 'road.world', category: 'Zones', title: 'Azeroth Survivor', text: 'Clear every zone.', points: 50,
  criteria: [], requires: ZONES.map((id) => `conquer.${id}`),
});
const counter = (category, id, kind, steps) => steps.forEach(([count, title, text, points]) =>
  achievements.push({ id: `${id}.${count}`, category, title, text, points, criteria: [objective(kind, count, text)], requires: [] }));
counter('Bosses', 'bosses', 'boss', [
  [10, 'Boss Hunter', 'Defeat 10 bosses.', 5],
  [50, 'Boss Slayer', 'Defeat 50 bosses.', 10],
  [200, 'Kingslayer', 'Defeat 200 bosses.', 20],
]);
counter('Slayer', 'kills', 'kill', [
  [1000, 'Blooded', 'Slay 1,000 enemies.', 5],
  [10000, 'Veteran', 'Slay 10,000 enemies.', 10],
  [50000, 'Legion of One', 'Slay 50,000 enemies.', 25],
]);
counter('Slayer', 'elites', 'elite', [
  [25, 'Champion Hunter', 'Slay 25 elites.', 5],
  [100, 'Lairbreaker', 'Slay 100 elites.', 10],
  [500, 'Bane of Champions', 'Slay 500 elites.', 20],
]);
counter('Survival', 'survive', 'survive', [
  [600, 'Holdout', 'Survive 10:00 in a single run.', 5],
  [840, 'Unbroken', 'Survive 14:00 in a single run.', 10],
]);
counter('Survival', 'clears', 'clear', [
  [10, 'Seasoned', 'Clear 10 runs.', 10],
  [50, 'Relentless', 'Clear 50 runs.', 25],
]);
counter('Survival', 'level', 'level', [
  [30, 'Ascendant', 'Reach level 30 in a single run.', 10],
]);
counter('Co-op', 'party', 'party', [
  [2, 'Brothers in Arms', 'Clear a zone with at least one ally.', 10],
  [3, 'Warband', 'Clear a zone in a party of three or more.', 10],
  [4, 'Full Party', 'Clear a zone in a party of four.', 15],
]);
for (const [id, name] of CLASSES) {
  achievements.push({
    id: `class.${id}`, category: 'Mastery', title: `${name}’s Road`, text: `Clear 5 runs as a ${name}.`, points: 10,
    criteria: [objective('clear', 5, `Clear 5 runs as a ${name}`, { class: id })], requires: [],
  });
}
achievements.push({
  id: 'master.all', category: 'Mastery', title: 'Master of Arms', text: 'Earn every class’s Road achievement.', points: 50,
  criteria: [], requires: CLASSES.map(([id]) => `class.${id}`),
});

const out = join(root, 'crates/asr-coop/assets/quest_catalog.json');
writeFileSync(out, `${JSON.stringify({ revision: REVISION, achievements }, null, 1)}\n`);
console.log(`${achievements.length} achievements -> ${out}`);
