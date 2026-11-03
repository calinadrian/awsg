#!/usr/bin/env node
// Writes crates/asr-coop/assets/quest_catalog.json: every zone's quests and the achievements.
// Boss line-ups come from the economy catalog (its boss_drops follow each zone's roster);
// creature targets are named here and checked against the rosters by
// `survivors::quests::tests::catalog_targets_are_roster_names` in benilla-app.
//
// Ids are permanent: heroes' logs and earned achievements refer to them. Change text freely;
// to change what a quest asks, add a new id and raise REVISION.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const economy = JSON.parse(readFileSync(join(root, 'crates/asr-coop/assets/economy_catalog.json'), 'utf8'));
const REVISION = 3;

// Story order within each faction's road; tier per difficulty.rs.
const ZONES = [
  ['elwynn', 1, 'Marshal Dughan', 'Goldshire', 'Murloc Streamrunner', 15,
    ['Hogger’s Raiders', 'Gnolls raid the farms south of Goldshire, and Hogger grows bolder every night. Bring the brute down and break the raiders who follow him.'],
    ['The Collector', 'Morgan the Collector runs the racket that bleeds Elwynn dry, and every thug on the road answers to him. End it.']],
  ['dun-morogh', 1, 'Senir Whitebeard', 'Kharanos', 'Wendigo', 15,
    ['The Howler', 'Edan the Howler hunts the passes above Kharanos, and the whole pack follows its call. Silence it and thin the pack.'],
    ['The White Bear', 'Bjarn, the great white bear, has come down from the peaks. Nothing in the valley is safe while it roams.']],
  ['teldrassil', 1, 'Conservator Ilthalaine', 'the Shadowglen', 'Nightsaber', 15,
    ['Shadows in the Glen', 'Corruption creeps through the Shadowglen and Threggil feeds on it. Root out the grell and whatever follows in its wake.'],
    ['The Augur’s Poison', 'The Gnarlpine Augur poisons the furbolg tribe from within. Silence the Augur and the tribe may yet heal.']],
  ['durotar', 1, 'Gornek', 'Razor Hill', 'Dire Mottled Boar', 15,
    ['The Geolord', 'Geolord Mottle’s quilboar dig in along the Valley of Trials. Break his warband before it reaches Razor Hill.'],
    ['Cult of the Burning Blade', 'The Burning Blade gathers in Skull Rock. Its cultist must fall before the demons answer the call.']],
  ['mulgore', 1, 'Grull Hawkwind', 'Bloodhoof Village', 'Venture Co. Hireling', 15,
    ['Stalkers of the Plain', 'Stalkers circle the plains below Red Cloud Mesa. Hunt the boldest of them and drive the rest away.'],
    ['The Pale Wolf', 'Ghost Howl walks Mulgore at night, and the herds will not graze while it does. Put the spirit to rest.']],
  ['tirisfal', 1, 'Executor Arren', 'Brill', 'Rot Hide Graverobber', 15,
    ['The Farmer’s Field', 'Farmer Solliden still guards his fields, long after the plague took him. The Forsaken need that land.'],
    ['Gorefang', 'Gorefang, the great worg, haunts the glades between Brill and the Scarlet border. Bring the beast down.']],
  ['westfall', 2, 'Gryan Stoutmantle', 'Sentinel Hill', 'Defias Trapper', 18,
    ['The Brotherhood’s Voice', 'The Defias Brotherhood holds Westfall in its fist. Start with the Brainwashed Noble who preaches their cause.'],
    ['Carrion Skies', 'Vultros picks the Brotherhood’s leavings clean and nests above the Dagger Hills. Clear the skies over Westfall.']],
  ['barrens', 2, 'Thork', 'the Crossroads', 'Kolkar Wrangler', 18,
    ['Echeyakee', 'Echeyakee prowls the tall grass outside the Crossroads. Hunt the great cat before it takes another caravan.'],
    ['Bloodmark', 'Hezrul Bloodmark leads the Kolkar against the Crossroads. Cut the head from the snake.']],
  ['deadmines', 3, 'Captain Danuvin', 'Moonbrook', 'Defias Watchman', 20,
    ['The Foreman', 'Rhahk’Zor drives the Brotherhood’s miners under Moonbrook. Smash through the foreman and the tunnels are open.'],
    ['VanCleef', 'Edwin VanCleef commands the Defias from a ship hidden deep in the mine. Finish the Brotherhood for good.']],
  ['wailing', 3, 'Nalpak', 'the Lushwater Oasis', 'Deviate Slayer', 20,
    ['The Fanglords', 'The Druids of the Fang twisted Naralex’s dream into a nightmare. Lady Anacondra leads them; begin with her.'],
    ['The Devourer', 'Mutanus the Devourer rises from the nightmare itself. Slay it, and Naralex may finally wake.']],
  ['duskwood', 4, 'Commander Althea Ebonlocke', 'Darkshire', 'Skeletal Mage', 22,
    ['The Spectral Wolf', 'Lupos stalks the road to Darkshire and no caravan has passed in a week. Hunt it down.'],
    ['Nightbane', 'Nefaru leads the Nightbane worgen against Darkshire’s walls. The Night Watch needs this ended.']],
  ['stonetalon', 4, 'Seereth Stonebreak', 'Sun Rock Retreat', 'Venture Co. Logger', 22,
    ['Iron in the Forest', 'The Venture Co. levels Stonetalon’s forests with machines like XT:4. Stop the machines and the men who run them.'],
    ['Windshear Crag', 'Gerenzo Wrenchwhistle runs the whole operation from Windshear Crag. Shut him down.']],
  ['hall-of-thanes', 2, 'Keeper Brannoc', 'the Thanes’ Rest', 'Dark Iron Intruder', 18,
    ['Robbers in the Tomb', 'Dark Iron raiders have broken into the burial halls of the old thanes. Faldrim Anvilmar leads the first band; start with him.'],
    ['The Dirge', 'Durgen Dirgehammer means to wake the thanes and bind them to the Dark Iron. Silence his hammer before the dead answer.']],
  ['stranglethorn', 5, 'Lieutenant Doren', 'the Rebel Camp', 'Lashtail Raptor', 25,
    ['The Uncatchable', 'Sin’Dall, the tiger no hunter has caught, prowls the northern jungle. Be the first.'],
    ['King of the Jungle', 'King Bangalash rules the panthers of Stranglethorn. Only the greatest hunters come back with his pelt.']],
];
const ROADS = [
  ['road.alliance', 'The Alliance Road', 'Clear the story road from Elwynn through Deadmines to Stranglethorn.', ['elwynn', 'westfall', 'deadmines', 'stranglethorn']],
  ['road.horde', 'The Horde Road', 'Clear the story road from Durotar through Wailing Caverns to Stranglethorn.', ['durotar', 'barrens', 'wailing', 'stranglethorn']],
  ['road.classic', 'A Classic Story', 'Clear the shared story road from Stranglethorn to Burning Steppes.', ['stranglethorn', 'scarlet-monastery', 'tanaris', 'ungoro', 'burning-steppes']],
  ['road.cradles', 'Cradles of Azeroth', 'Clear all six starting lands.', ['elwynn', 'dun-morogh', 'teldrassil', 'durotar', 'mulgore', 'tirisfal']],
];
ZONES.push(
  ['scarlet-monastery', 6, 'Crusade Scout', 'the cathedral', 'Scarlet Soldier', 30,
    ['Into the Cathedral', 'Break the Scarlet garrison guarding the road ahead.'],
    ['The High Inquisitor', 'Defeat High Inquisitor Whitemane and open the roads to Tanaris and the Swamp of Sorrows.']],
  ['tanaris', 7, 'Desert Scout', 'the desert road', 'Dune Basilisk', 35,
    ['Across the Sands', 'Drive back the desert raiders.'],
    ['The Desert Colossus', "Defeat the Desert Colossus to open Un'Goro and the dungeon branch to Zul'Farrak."]],
  ['ungoro', 8, 'Crater Scout', "Un'Goro", 'Ravasaur', 40,
    ['Into the Crater', 'Face the creatures guarding Fire Plume Ridge.'],
    ['King of the Crater', 'Defeat King Mosh and continue to Burning Steppes.']],
  ['burning-steppes', 9, 'Blackrock Scout', 'the Blackrock road', 'Blackrock Grunt', 45,
    ['The Blackrock Road', 'Break the Blackrock warband.'],
    ['The Firelord at the Gate', 'Defeat the Blackrock Firelord and prepare for the Core.']],
  ['swamp-of-sorrows', 7, 'Brood Tracker', 'the swamp trail', 'Swamp Crocolisk', 35,
    ["The Brood Mother's Shadow", 'Follow the trail through the swamp.'],
    ['The Trail Guardian', 'Defeat the Brood Trail Guardian and prepare to face Onyxia.']],
  ['zulfarrak', 8, 'Desert Scout', "Zul'Farrak", 'Sandfury Headhunter', 40,
    ['The Sandfury Stronghold', 'Fight through the Sandfury defenders.'],
    ['Chief of the Sandfury', 'Defeat Chief Ukorz Sandscalp.']],
  ['onyxia', 10, 'Brood Tracker', "Onyxia's Lair", 'Onyxian Whelp', 50,
    ["The Brood Mother's Lair", 'Hold against the brood until Onyxia answers.'],
    ['The Brood Mother', 'Defeat Onyxia.']],
  ['molten-core', 10, 'Blackrock Scout', 'the Core', 'Flame Imp', 50,
    ['Into the Core', 'Break the firelord guards.'],
    ['The Firelord', 'Banish Ragnaros.']],
);
const CLASSES = [[1, 'Warrior'], [2, 'Paladin'], [3, 'Hunter'], [4, 'Rogue'], [5, 'Priest'], [7, 'Shaman'], [8, 'Mage'], [9, 'Warlock'], [11, 'Druid']];

const plural = (name) => name.endsWith('man') ? `${name.slice(0, -3)}men` : name.endsWith('s') ? name : `${name}s`;
const objective = (kind, count, text, extra = {}) => ({ kind, target: '', zone: '', class: 0, count, text, ...extra });

const quests = [];
const achievements = [];
for (const [id, tier, giver, place, slayer, level, [storyTitle, storyText], [finaleTitle, finaleText]] of ZONES) {
  const zone = economy.zones[id];
  if (!zone) throw new Error(`no economy zone ${id}`);
  const bosses = zone.bosses ?? zone.boss_drops.map((b) => b.boss);
  const [first, last] = [bosses[0], bosses[bosses.length - 1]];
  const q = (suffix, title, text, objectives, reward, more = {}) => quests.push({
    id: `${id}.${suffix}`, zone: id, title, giver, text, requires: [], repeatable: false,
    objectives: objectives.map((o) => ({ ...o, zone: id })),
    reward: { gold: 0, materials: 0, points: 0, ...reward, item: !!reward.item && zone.equipment.length > 0 }, ...more,
  });
  q('story', storyTitle, storyText,
    [objective('boss', 1, `Defeat ${first}`, { target: first }), objective('kill', 150, 'Slay 150 enemies')],
    { gold: 40 * tier, materials: 2 + tier });
  q('bounty', `Bounty: ${plural(slayer)}`,
    `${plural(slayer)} keep coming at ${place}. ${giver} pays for every thirty you put down, as often as you bring proof.`,
    [objective('kill', 30, `Slay 30 ${plural(slayer)}`, { target: slayer })],
    { gold: 25 * tier, materials: 1 + tier }, { repeatable: true });
  q('lairs', 'Break the Lairs',
    `Four lairs ring the battlefield, each held by a champion. Break three of them and ${place} sleeps easier.`,
    [objective('elite', 3, 'Slay 3 elites')],
    { gold: 30 * tier, materials: 2, item: true });
  q('hold', 'Hold the Line',
    `${giver} needs the field held while ${place} regroups. Survive ten minutes in ${zone.name}, whatever comes.`,
    [objective('survive', 600, 'Survive 10:00')],
    { gold: 50 * tier, points: 5 });
  q('finale', finaleTitle, finaleText,
    [objective('boss', 1, `Defeat ${last}`, { target: last })],
    { gold: 120 * tier, materials: 3 + tier, points: 10, item: true }, { requires: [`${id}.story`] });
  q('proven', `Proven in ${zone.name}`,
    `${giver} trusts only those who have grown strong in the field. Reach level ${level} in a single run in ${zone.name}.`,
    [objective('level', level, `Reach level ${level}`)],
    { gold: 60 * tier, points: 5 });

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
for (const [id, zone, title, giver, text] of [
  ['story.onyxia_attunement', 'swamp-of-sorrows', "The Brood Mother's Trail", 'Brood Tracker', "Clear Swamp of Sorrows, then return to the quest log and turn this quest in. Onyxia's Lair requires this attunement and permanent class level 50."],
  ['story.molten_attunement', 'burning-steppes', 'Attunement to the Core', 'Blackrock Scout', 'Clear Burning Steppes, then return to the quest log and turn this quest in. Molten Core requires this attunement and permanent class level 50.'],
]) {
  quests.push({ id, zone, title, giver, text, requires: [], repeatable: false,
    objectives: [objective('clear', 1, `Clear ${economy.zones[zone].name}`, { zone })],
    reward: { gold: 100, materials: 0, points: 10, item: false } });
}
for (const [id, title, text, zones] of ROADS) {
  achievements.push({ id, category: 'Zones', title, text, points: 25, criteria: [], requires: zones.map((z) => `conquer.${z}`) });
}
achievements.push({
  id: 'road.world', category: 'Zones', title: 'Azeroth Survivor', text: 'Clear every zone.', points: 50,
  criteria: [], requires: ZONES.map(([id]) => `conquer.${id}`),
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
writeFileSync(out, `${JSON.stringify({ revision: REVISION, quests, achievements }, null, 1)}\n`);
console.log(`${quests.length} quests, ${achievements.length} achievements -> ${out}`);
