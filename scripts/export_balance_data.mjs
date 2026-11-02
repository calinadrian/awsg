import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const spellSource = read('crates/benilla-app/src/survivors/spell_data.rs');
const axisSource = read('crates/benilla-app/src/survivors/reward_axes.rs');
const classSource = read('crates/benilla-app/src/survivors/data.rs');

const spells = {};
const spellRx = /ability!\(\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*(\w+)\s*\{([\s\S]*?)\}\s*\)/g;
for (const match of spellSource.matchAll(spellRx)) {
  const [, key, spell, kind, body] = match;
  const number = (name, fallback = 0) => {
    const value = body.match(new RegExp(`\\b${name}:\\s*(-?[\\d.]+)`));
    return value ? Number(value[1]) : fallback;
  };
  spells[key] = {
    id: Number(spell), kind,
    damage: number('damage'), dot: number('dot'), cooldown: number('cooldown', 2),
    radius: number('radius'), range: number('range', 20), count: number('count', 1),
    duration: number('duration'), ticks: number('ticks'), native: number('native_kind', 255),
    school: number('school'),
  };
}

const axes = {};
for (const match of axisSource.matchAll(/"([^"]+)"\s*=>\s*&\[([^\]]*)\]/g)) {
  axes[match[1]] = [...match[2].matchAll(/\b(Power|Cooldown|Targets|Area|Duration)\b/g)].map((m) => m[1].toLowerCase());
}

const classes = [];
const classRx = /ClassDef\s*\{([\s\S]*?)\n\s*\},/g;
for (const match of classSource.matchAll(classRx)) {
  const body = match[1];
  const name = body.match(/name:\s*"([^"]+)"/)?.[1];
  const start = body.match(/start:\s*"([^"]+)"/)?.[1];
  const pool = body.match(/pool:\s*&\[([\s\S]*?)\]/)?.[1];
  const damageScale = Number(body.match(/damage_scale:\s*([\d.]+)/)?.[1] ?? 1);
  if (name && pool) classes.push({ name, start, damageScale, spells: [...pool.matchAll(/"([^"]+)"/g)].map((m) => m[1]) });
}

const output = {
  classes,
  spells: Object.fromEntries(Object.entries(spells).map(([key, def]) => [key, { ...def, axes: axes[key] ?? ['power', 'cooldown'] }])),
};
fs.writeFileSync('web/balance-data.js', `// Generated from survivors/spell_data.rs, data.rs, and reward_axes.rs.\nwindow.BALANCE_DATA = ${JSON.stringify(output, null, 2)};\n`);
process.stdout.write(`Wrote ${Object.keys(spells).length} spells across ${classes.length} classes.\n`);
