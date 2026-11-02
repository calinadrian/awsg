(() => {
  const data = window.ZONE_LOOT_DATA;
  const byId = (id) => document.getElementById(id);
  const storageKey = 'asr-zone-loot-collected-v1';
  const slotNames = {
    back: 'Back', belt: 'Waist', chest: 'Chest', feet: 'Feet', finger_1: 'Finger',
    hands: 'Hands', head: 'Head', legs: 'Legs', main_hand: 'Main hand', neck: 'Neck',
    off_hand: 'Off hand', ranged: 'Ranged', shirt: 'Shirt', shoulders: 'Shoulders',
    trinket_1: 'Trinket', wrists: 'Wrists',
  };
  const statNames = {
    max_health: 'Health', damage_bp: 'Damage', damage_reduction_bp: 'Damage reduction',
    critical_chance_bp: 'Critical chance', cooldown_reduction_bp: 'Cooldown recovery',
  };
  let selectedZone = data.zones[0]?.id ?? '';
  let collected = {};
  try { collected = JSON.parse(localStorage.getItem(storageKey) || '{}') || {}; } catch { collected = {}; }

  const escape = (value) => String(value ?? '').replace(/[&<>"']/g, (char) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[char]);
  const slotLabel = (slot) => slotNames[slot] ?? slot.replaceAll('_', ' ');
  const spellLabel = (spell) => spell.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
  const chance = (bp) => `${((bp ?? 0) / 100).toFixed((bp ?? 0) % 100 ? 1 : 0)}%`;
  const bonusLabel = (key, value) => key.endsWith('_bp')
    ? `+${(value / 100).toFixed(2)}% ${statNames[key] ?? key.replace(/_bp$/, '').replaceAll('_', ' ')}`
    : `+${value} ${statNames[key] ?? key.replaceAll('_', ' ')}`;
  const bonusLines = (bonuses = {}) => Object.entries(bonuses).map(([key, value]) => bonusLabel(key, value));
  const materialLines = (materials = {}) => Object.entries(materials).map(([material, value]) => `${value} ${material}`);
  const itemStats = (item) => [
    ...bonusLines(item.baseBonuses),
    ...Object.entries(item.spellDamageBp ?? {}).map(([spell, value]) => `+${(value / 100).toFixed(0)}% ${spellLabel(spell)} damage`),
    ...(item.setName ? [`${item.setName}`, ...(Object.entries(item.setBonus?.spell_damage_bp ?? {}).map(([spell, value]) => `${item.setBonus.pieces}-piece bonus: +${(value / 100).toFixed(0)}% ${spellLabel(spell)} damage`))] : []),
  ];
  const itemTooltip = (item) => [
    item.name,
    `${item.quality || 'common'} · ${slotLabel(item.slot)}${item.armor ? ` · ${item.armor} armor` : ''}${item.twoHanded ? ' · two-handed' : ''}`,
    ...itemStats(item),
    ...(item.upgrades ?? []).map((upgrade) => `Level ${upgrade.level} (${materialLines(upgrade.cost).join(', ')}): ${bonusLines(upgrade.bonuses).join(', ')}`),
    `Scrap: ${materialLines(item.scrap).join(', ') || 'none'}`,
    `Usable by: ${item.classes.join(', ')}`,
  ].join('\n');
  const zone = () => data.zones.find((entry) => entry.id === selectedZone) ?? data.zones[0];
  const collectedKey = (item) => `${selectedZone}/${item.id}`;
  const save = () => {
    try { localStorage.setItem(storageKey, JSON.stringify(collected)); } catch { /* private mode */ }
  };

  for (const name of data.classes) {
    const option = document.createElement('option'); option.value = name; option.textContent = name;
    byId('class-filter').append(option);
  }
  byId('class-filter').insertAdjacentHTML('afterbegin', '<option value="">Any class</option>');
  const slots = [...new Set(data.zones.flatMap((entry) => entry.equipment.map((item) => item.slot)))].sort();
  for (const slot of slots) {
    const option = document.createElement('option'); option.value = slot; option.textContent = slotLabel(slot);
    byId('slot-filter').append(option);
  }
  const qualities = [...new Set(data.zones.flatMap((entry) => entry.equipment.map((item) => item.quality || 'common')))]
    .sort((a, b) => ['poor', 'common', 'uncommon', 'rare', 'epic', 'legendary'].indexOf(a)
      - ['poor', 'common', 'uncommon', 'rare', 'epic', 'legendary'].indexOf(b));
  for (const quality of qualities) {
    const option = document.createElement('option'); option.value = quality;
    option.textContent = quality.charAt(0).toUpperCase() + quality.slice(1);
    byId('quality-filter').append(option);
  }
  byId('zone-count').textContent = `${data.zones.filter((entry) => !entry.unconfigured).length} loot tables · ${data.zones.length} areas tracked`;

  function drawZones() {
    byId('zone-list').replaceChildren(...data.zones.map((entry) => {
      const button = document.createElement('button');
      button.className = 'zone-pick';
      button.type = 'button';
      button.setAttribute('aria-current', String(entry.id === selectedZone));
      const chapter = entry.unconfigured ? 'NO TABLE' : entry.chapter ? `CHAPTER ${entry.chapter}` : 'INSTANCE';
      const count = entry.unconfigured ? '—' : entry.equipment.length;
      button.innerHTML = `<span><span class="zone-name">${escape(entry.name)}</span><br><span class="zone-meta">${chapter}</span></span><span class="count">${count}</span>`;
      button.addEventListener('click', () => { selectedZone = entry.id; draw(); });
      return button;
    }));
  }

  function drawHeader(entry, eligiblePool) {
    const eligibleWeight = eligiblePool.reduce((sum, item) => sum + item.weight, 0);
    byId('chapter').textContent = entry.unconfigured ? 'NO LOOT TABLE CONFIGURED' : entry.chapter ? `CHAPTER ${entry.chapter}` : 'INSTANCE LOOT TABLE';
    byId('zone-title').textContent = entry.name;
    byId('zone-sub').textContent = entry.unconfigured
      ? 'This playable map has no zone equipment or material table in the current catalog.'
      : `${entry.equipment.length} equipment entries · ${entry.materialPool.length} shared materials`;
    byId('pool-note').textContent = entry.unconfigured
      ? 'No equipment pool is configured for this map.'
      : `Showing ${eligiblePool.length} usable item${eligiblePool.length === 1 ? '' : 's'} · weight total ${eligibleWeight}`;
    byId('metrics').innerHTML = [
      ['Equipment pool', eligiblePool.length],
      ['Normal equipment', entry.unconfigured ? '—' : chance(entry.equipmentChanceBp.normal)],
      ['Elite equipment', entry.unconfigured ? '—' : chance(entry.equipmentChanceBp.elite)],
      ['Boss equipment', entry.unconfigured ? '—' : chance(entry.equipmentChanceBp.boss)],
    ].map(([label, value]) => `<div class="metric"><small>${escape(label)}</small><strong>${escape(value)}</strong></div>`).join('');
    byId('materials').innerHTML = entry.unconfigured ? '' : [
      `<span class="material"><b>Material roll</b> · normal ${chance(entry.materialChanceBp.normal)} · elite ${chance(entry.materialChanceBp.elite)} · boss ${chance(entry.materialChanceBp.boss)}</span>`,
      ...entry.materialPool.map((material) => {
        const sourceAmounts = ['normal', 'elite', 'boss'].map((kind) => `${kind} +${entry.materials[kind] ?? 0}`).join(' · ');
        return `<span class="material"><b>${escape(material)}</b> · ${sourceAmounts}</span>`;
      }),
    ].join('');
    byId('boss-drops').innerHTML = (entry.bossDrops ?? []).map(({ boss, item }) =>
      `<span class="material"><b>${escape(boss)}</b> · <span class="quality quality-${escape(item.quality || 'common')}">${escape(item.quality || 'common')}</span> · <span title="${escape(itemTooltip(item))}">${escape(item.name)}</span></span>`
    ).join('') || '<span class="zone-meta">No boss loot entries configured.</span>';
  }

  function drawItems(entry, eligiblePool) {
    if (entry.unconfigured) {
      byId('result-count').textContent = 'No table';
      byId('items').innerHTML = `<tr><td colspan="7" class="empty">No equipment pool is configured for ${escape(entry.name)}.</td></tr>`;
      return;
    }
    const query = byId('search').value.trim().toLowerCase();
    const chosenSlot = byId('slot-filter').value;
    const chosenQuality = byId('quality-filter').value;
    const missingOnly = byId('missing-only').checked;
    const showSources = byId('show-sources').checked;
    const poolWeight = eligiblePool.reduce((sum, item) => sum + item.weight, 0);
    const filtered = eligiblePool.filter((item) => {
      const isCollected = Boolean(collected[collectedKey(item)]);
      return (!query || `${item.name} ${item.id} ${itemStats(item).join(' ')} ${item.classes.join(' ')}`.toLowerCase().includes(query))
        && (!chosenSlot || item.slot === chosenSlot)
        && (!chosenQuality || (item.quality || 'common') === chosenQuality)
        && (!missingOnly || !isCollected);
    });
    byId('result-count').textContent = `${filtered.length} / ${eligiblePool.length}`;
    if (!filtered.length) {
      byId('items').innerHTML = `<tr><td colspan="7" class="empty">No items match these filters.</td></tr>`;
      return;
    }
    byId('items').innerHTML = filtered.map((item) => {
      const key = collectedKey(item);
      const isCollected = Boolean(collected[key]);
      const quality = item.quality || 'common';
      const stats = itemStats(item);
      const sources = showSources && item.sourceCreatures.length
        ? `<details class="sources"><summary>${item.sourceCreatures.length} reference source${item.sourceCreatures.length === 1 ? '' : 's'}</summary><ul>${item.sourceCreatures.map((source) => `<li>${escape(source)}</li>`).join('')}</ul></details>`
        : '<span class="zone-meta">—</span>';
      return `<tr>
        <td><input class="track" type="checkbox" aria-label="Mark ${escape(item.name)} collected" data-item="${escape(item.id)}" ${isCollected ? 'checked' : ''}></td>
        <td><span class="item-name" title="${escape(itemTooltip(item))}">${escape(item.name)}</span><span class="item-id">${escape(item.id)}${item.armor ? ` · ${escape(item.armor)} armor` : ''}${item.twoHanded ? ' · two-handed' : ''}</span><span class="quality quality-${escape(quality)}">${escape(quality)}</span>${item.setName ? `<span class="quality quality-rare">Set · ${escape(item.setName)}</span>` : ''}</td>
        <td>${escape(slotLabel(item.slot))}</td>
        <td class="item-stats">${stats.length ? stats.map((line) => `<span class="${line.includes(' damage') ? 'spell-affix' : ''}">${escape(line)}</span>`).join('') : '<span class="zone-meta">—</span>'}</td>
        <td>${escape(item.classes.join(', '))}</td>
        <td class="chance">${poolWeight ? (item.weight * 100 / poolWeight).toFixed(2) : '0.00'}%</td><td>${sources}</td>
      </tr>`;
    }).join('');
    for (const checkbox of byId('items').querySelectorAll('[data-item]')) {
      checkbox.addEventListener('change', () => {
        const key = `${selectedZone}/${checkbox.dataset.item}`;
        if (checkbox.checked) collected[key] = true;
        else delete collected[key];
        save();
        drawItems(entry, eligiblePool);
      });
    }
  }

  function draw() {
    const entry = zone();
    if (!entry) return;
    drawZones();
    const className = byId('class-filter').value;
    const eligible = entry.equipment.filter((item) => !className || item.classes.includes(className));
    drawHeader(entry, eligible);
    drawItems(entry, eligible);
  }

  for (const id of ['search', 'class-filter', 'slot-filter', 'quality-filter', 'missing-only', 'show-sources']) {
    byId(id).addEventListener(id === 'search' ? 'input' : 'change', draw);
  }
  byId('clear-zone').addEventListener('click', () => {
    for (const key of Object.keys(collected)) if (key.startsWith(`${selectedZone}/`)) delete collected[key];
    save(); draw();
  });
  byId('export').addEventListener('click', () => {
    const rows = [['Zone', 'Item', 'Item ID', 'Quality', 'Stats', 'Collected']];
    for (const entry of data.zones) {
      for (const item of entry.equipment) {
        rows.push([
          entry.name,
          item.name,
          item.id,
          item.quality || 'common',
          itemStats(item).join('; '),
          collected[`${entry.id}/${item.id}`] ? 'yes' : 'no',
        ]);
      }
    }
    const csv = rows.map((row) => row.map((cell) => `"${String(cell).replaceAll('"', '""')}"`).join(',')).join('\r\n');
    const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' }));
    const link = document.createElement('a'); link.href = url; link.download = 'azeroth-survivors-loot-checklist.csv'; link.click();
    URL.revokeObjectURL(url);
  });
  draw();
})();
