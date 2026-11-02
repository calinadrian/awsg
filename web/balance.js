(() => {
  const data = window.BALANCE_DATA;
  const byId = (id) => document.getElementById(id);
  const rarityNames = ['Common', 'Uncommon', 'Rare', 'Epic', 'Legendary'];
  const powerStep = [0.18, 0.25, 0.34, 0.46, 0.65];
  const cooldownStep = [0.07, 0.10, 0.13, 0.17, 0.23];
  const areaStep = [0.12, 0.18, 0.25, 0.35, 0.50];
  const durationStep = [0.15, 0.22, 0.30, 0.40, 0.60];
  const passiveRates = { hunters_mark: 1.5, call_of_flame: 0.7, soul_fire: 0.4, seal_fate: 1.5, faerie_fire: 5, redemption: 2, polymorph: 4, bloodrage: 0.5 };
  const areas = new Set(['Nova', 'Ground', 'GroundSelf', 'Cone', 'Aura']);
  const passiveAreas = new Set(['hunters_mark', 'call_of_flame', 'redemption', 'bloodrage']);
  const noDamage = new Set(['Buff', 'Heal', 'Shield', 'Immune']);
  let classData = data.classes[0];

  for (const c of data.classes) {
    const opt = document.createElement('option'); opt.value = c.name; opt.textContent = c.name;
    byId('class').append(opt);
  }

  function setClass(name) {
    classData = data.classes.find((c) => c.name === name) ?? data.classes[0];
    const start = classData.start;
    const keys = [start, ...classData.spells.filter((key) => key !== start)
      .map((key) => ({ key, score: estimate(key, makeSlot(key)).dps }))
      .sort((a, b) => b.score - a.score)
      .slice(0, 5).map((entry) => entry.key)];
    byId('spell-slots').replaceChildren(...Array.from({ length: 6 }, (_, i) => makeSlot(keys[i] ?? '')));
    update();
  }

  function makeSlot(selected) {
    const row = document.createElement('div'); row.className = 'spell-row';
    const spellField = document.createElement('div');
    spellField.innerHTML = '<label>Spell</label>';
    const spellSelect = document.createElement('select'); spellSelect.className = 'pick';
    const none = document.createElement('option'); none.value = ''; none.textContent = 'Empty slot'; spellSelect.append(none);
    for (const key of classData.spells) {
      const opt = document.createElement('option'); opt.value = key; opt.textContent = key.replaceAll('_', ' '); spellSelect.append(opt);
    }
    spellSelect.value = selected;

    const levelField = document.createElement('div'); levelField.innerHTML = '<label>Level</label>';
    const level = document.createElement('select'); level.className = 'level';
    for (let n = 1; n <= 8; n++) { const opt = document.createElement('option'); opt.value = n; opt.textContent = n; level.append(opt); }
    level.value = 1;

    const axisField = document.createElement('div'); axisField.innerHTML = '<label>Upgrade focus</label>';
    const axis = document.createElement('select'); axis.className = 'axis';
    const rateField = document.createElement('div'); rateField.innerHTML = '<label>Activations / min</label>';
    const rate = document.createElement('input'); rate.className = 'rate'; rate.type = 'number'; rate.min = '0'; rate.max = '120'; rate.step = '0.5';
    rate.value = 0;
    const multiplierField = document.createElement('div'); multiplierField.innerHTML = '<label>Damage %</label>';
    const multiplier = document.createElement('input'); multiplier.className = 'multiplier'; multiplier.type = 'number'; multiplier.min = '0'; multiplier.max = '500'; multiplier.step = '5'; multiplier.value = '100';

    const updateAxes = () => {
      const spell = data.spells[spellSelect.value];
      axis.replaceChildren();
      for (const key of (spell?.axes ?? ['power', 'cooldown'])) {
        const opt = document.createElement('option'); opt.value = key; opt.textContent = key;
        axis.append(opt);
      }
      axis.value = spell?.axes.includes('power') ? 'power' : (spell?.axes[0] ?? 'power');
      const initialRate = passiveRates[spellSelect.value] ?? (spell && !noDamage.has(spell.kind) ? 60 / spell.cooldown : 0);
      rate.value = initialRate.toFixed(1);
      rateField.style.opacity = spell ? '1' : '.45';
    };
    spellSelect.addEventListener('change', () => { updateAxes(); update(); });
    level.addEventListener('change', update); axis.addEventListener('change', update); rate.addEventListener('input', update); multiplier.addEventListener('input', update);
    updateAxes();
    spellField.append(spellSelect); levelField.append(level); axisField.append(axis); rateField.append(rate); multiplierField.append(multiplier);
    row.append(spellField, levelField, axisField, rateField, multiplierField);
    return row;
  }

  function progressFor(spell, level, rarity, focus) {
    let power = 1, cooldown = 1, area = 1, duration = 1, targets = 0;
    const ranks = { power: 0, cooldown: 0, targets: 0, area: 0, duration: 0 };
    if (rarity > 0) {
      if (spell.axes.includes('power')) power *= 1 + powerStep[rarity - 1];
      else cooldown *= 1 - cooldownStep[rarity - 1];
    }
    for (let pick = 0; pick < level - 1; pick++) {
      const r = rarity;
      if (focus === 'power') power *= 1 + powerStep[r] / (1 + .5 * ranks.power);
      if (focus === 'cooldown') cooldown = Math.max(.2, cooldown - cooldownStep[r] * Math.max(0, (cooldown - .2) / .8));
      if (focus === 'targets') targets = Math.min(12, targets + (r >= 3 ? 2 : 1));
      if (focus === 'area') area = Math.min(5, area + areaStep[r] * Math.max(0, (5 - area) / 4));
      if (focus === 'duration') duration = Math.min(6, duration + durationStep[r] * Math.max(0, (6 - duration) / 5));
      ranks[focus]++;
      const rank = 2 + Object.values(ranks).reduce((a, b) => a + b, 0);
      power *= 1 + .04 / (1 + .125 * Math.max(0, rank - 2));
    }
    return { power, cooldown, area, duration, targets };
  }

  function estimate(key, row) {
    const spell = data.spells[key];
    const level = Number(row.querySelector('.level').value);
    const focus = row.querySelector('.axis').value;
    const rarity = Number(byId('rarity').value);
    const p = progressFor(spell, level, rarity, focus);
    const nativeScale = spell.native === 255 || spell.native === 9 ? 1 : ([2, 3, 8].includes(spell.native) ? .7 : 1.4);
    const nativeCooldown = spell.native === 255 ? 1 : 1.4;
    const cooldown = Math.max(.25, spell.cooldown * nativeCooldown * p.cooldown);
    const rate = Math.max(0, Math.min(120, Number(row.querySelector('.rate').value) || 0));
    const activations = Math.min(rate, 60 / cooldown);
    const enemies = Number(byId('enemies').value);
    const density = Number(byId('density').value) / 100;
    const might = Number(byId('might').value) / 100;
    const bonus = 1 + Number(byId('bonus').value) / 100;
    const coverageHits = Math.max(1, Math.ceil(enemies * density));
    const targetCount = Math.min(enemies, Math.max(1, spell.count + (focus === 'targets' ? p.targets : 0)));
    const areaHit = areas.has(spell.kind) || passiveAreas.has(key);
    const damageFactor = Math.max(0, Math.min(500, Number(row.querySelector('.multiplier').value) || 0)) / 100;
    const baseScale = nativeScale * p.power * might * bonus * damageFactor * classData.damageScale;
    let perCast = spell.damage * baseScale * (areaHit ? coverageHits : targetCount);
    if (spell.kind === 'Dot') {
      const activeSeconds = Math.max(0, spell.duration * p.duration);
      const ticksTotal = spell.dot * activeSeconds * baseScale;
      const interval = Math.max(cooldown, activeSeconds || cooldown);
      perCast = ticksTotal;
      // `rate` is casts per minute; divide by 60 to convert to damage per second.
      const dotActivations = Math.min(rate, 60 / interval);
      return { perCast, dps: ticksTotal * dotActivations / 60 * Math.min(enemies, Math.max(1, spell.count + (focus === 'targets' ? p.targets : 0))), level, rate: dotActivations };
    }
    if (spell.kind === 'Ground' || spell.kind === 'GroundSelf' || spell.kind === 'Totem') {
      perCast = spell.damage * baseScale * coverageHits;
      if (spell.kind === 'Totem') {
        const pulseHits = key === 'magma_totem' ? coverageHits : targetCount;
        perCast = spell.damage * baseScale * Math.max(1, spell.ticks) * pulseHits;
      }
    }
    if (noDamage.has(spell.kind)) return { perCast: 0, dps: 0, level, rate: 0, utility: true };
    return { perCast, dps: perCast * activations / 60, level, rate: activations };
  }

  const fmt = (n) => n >= 10000 ? `${(n / 1000).toFixed(1)}k` : n >= 100 ? Math.round(n).toLocaleString() : n.toFixed(1);
  function update() {
    byId('enemies-out').value = byId('enemies').value;
    byId('density-out').value = `${byId('density').value}%`;
    byId('might-out').value = `${(Number(byId('might').value) / 100).toFixed(2)}×`;
    byId('bonus-out').value = `${byId('bonus').value}%`;
    const rows = [...byId('spell-slots').querySelectorAll('.spell-row')];
    const entries = rows.map((row) => ({ key: row.querySelector('.pick').value, row })).filter((x) => x.key).map(({ key, row }) => ({ key, ...estimate(key, row) }));
    entries.sort((a, b) => b.dps - a.dps);
    const total = entries.reduce((sum, x) => sum + x.dps, 0);
    byId('total-dps').textContent = fmt(total);
    byId('dpm').textContent = fmt(total * 60);
    byId('active-count').textContent = `${entries.length} / 6`;
    byId('subtitle').textContent = `${classData.name} · class tuning ${classData.damageScale.toFixed(2)}× · ${rarityNames[Number(byId('rarity').value)]} · ${byId('enemies').value} enemies`;
    const tbody = byId('results'); tbody.replaceChildren();
    for (const x of entries) {
      const tr = document.createElement('tr');
      const cells = [x.key.replaceAll('_', ' '), x.utility ? 'utility' : fmt(x.perCast), x.utility ? '—' : fmt(x.dps)];
      for (const value of cells) { const td = document.createElement('td'); td.textContent = value; tr.append(td); }
      const barCell = document.createElement('td'); barCell.className = 'barcell';
      const bar = document.createElement('div'); bar.className = 'bar'; bar.style.width = `${total ? Math.max(1, x.dps / total * 100) : 0}%`; barCell.append(bar); tr.append(barCell); tbody.append(tr);
    }
    if (!entries.length) { const tr = document.createElement('tr'); const td = document.createElement('td'); td.colSpan = 4; td.textContent = 'Add a spell to see its estimated damage.'; tr.append(td); tbody.append(tr); }
    drawChart(entries, total);
  }

  function drawChart(entries, total) {
    const canvas = byId('chart');
    const rect = canvas.getBoundingClientRect();
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.round(rect.width * ratio); canvas.height = Math.round(rect.height * ratio);
    const ctx = canvas.getContext('2d'); ctx.scale(ratio, ratio);
    const w = rect.width, h = rect.height, pad = { l: 46, r: 14, t: 15, b: 26 };
    ctx.clearRect(0, 0, w, h); ctx.font = '10px DM Mono, monospace'; ctx.fillStyle = '#98a2b3'; ctx.strokeStyle = '#2b3341'; ctx.lineWidth = 1;
    const maxT = 300, maxD = Math.max(1, total * maxT);
    for (let i = 0; i <= 4; i++) {
      const y = pad.t + (h - pad.t - pad.b) * i / 4;
      ctx.beginPath(); ctx.moveTo(pad.l, y); ctx.lineTo(w - pad.r, y); ctx.stroke();
      ctx.fillText(fmt(maxD * (4 - i) / 4), 5, y + 3);
    }
    for (let i = 0; i <= 5; i++) { const x = pad.l + (w - pad.l - pad.r) * i / 5; ctx.fillText(`${i}m`, x - 6, h - 7); }
    const palette = ['#7daafc','#ef9b6d','#8bcf9e','#c59bf2','#e9cf73','#75c9c5'];
    let base = 0;
    entries.forEach((entry, index) => {
      const end = base + entry.dps;
      ctx.beginPath(); ctx.strokeStyle = palette[index % palette.length]; ctx.lineWidth = 2;
      ctx.moveTo(pad.l, h - pad.b);
      for (let i = 0; i <= 60; i++) {
        const x = pad.l + (w - pad.l - pad.r) * i / 60;
        const yv = (base + (end - base) * i / 60) * maxT;
        const y = h - pad.b - (h - pad.t - pad.b) * yv / maxD;
        ctx.lineTo(x, y);
      }
      ctx.stroke(); base = end;
    });
  }

  byId('class').addEventListener('change', (e) => setClass(e.target.value));
  for (const id of ['rarity', 'enemies', 'density', 'might', 'bonus']) byId(id).addEventListener('input', update);
  byId('reset').addEventListener('click', () => {
    byId('class').value = data.classes[0].name; byId('rarity').value = '0'; byId('enemies').value = '10'; byId('density').value = '60'; byId('might').value = '100'; byId('bonus').value = '0'; setClass(data.classes[0].name);
  });
  window.addEventListener('resize', update);
  setClass(classData.name);
})();
