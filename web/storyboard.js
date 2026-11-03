/* Local story planning. No game files, accounts, or network connections are used. */
(() => {
  'use strict';
  const $ = id => document.getElementById(id);
  const NS = 'http://www.w3.org/2000/svg';
  const STORAGE = 'asr-storyboard.workspace.v1';
  const GRID = 24;
  const COLORS = {
    sand: { tint: '#f7edda', accent: '#ac8b4b' },
    sage: { tint: '#e5eee2', accent: '#7f9a73' },
    blue: { tint: '#e3ebf7', accent: '#7696c1' },
    lavender: { tint: '#ebe5f3', accent: '#a08aba' },
    rose: { tint: '#f5e3df', accent: '#bf8e81' },
    yellow: { tint: '#f6e7a3', accent: '#ab9240' },
  };
  const KINDS = {
    zone: { label: 'Zone', color: 'sage', title: 'New zone', w: 264, h: 240 },
    quest: { label: 'Quest', color: 'blue', title: 'New quest', w: 264, h: 240 },
    boss: { label: 'Boss', color: 'rose', title: 'New encounter', w: 264, h: 240 },
    reward: { label: 'Reward', color: 'lavender', title: 'New reward', w: 264, h: 240 },
    note: { label: 'Sticky note', color: 'yellow', title: 'An idea…', w: 264, h: 240 },
    chapter: { label: 'Chapter', color: 'sand', title: 'New chapter', w: 700, h: 480 },
  };
  const SIDES = ['left', 'right', 'top', 'bottom'];
  const validSide = value => SIDES.includes(value) ? value : null;
  const FIELDS = ['title', 'body', 'faction', 'status', 'zone', 'level', 'bosses', 'armor', 'condition'];
  const viewport = $('viewport');
  const nodeElements = new Map();
  const selection = new Set();
  const history = new Map();
  let selectedEdge = null, tool = 'select', space = false, connectionFrom = null;
  let gesture = null, saveTimer = null, toastTimer = null, savingFailed = false;
  let touchPoints = new Map(), pinch = null;
  const uid = prefix => prefix + '_' + (globalThis.crypto?.randomUUID?.() || Date.now().toString(36) + Math.random().toString(36).slice(2));
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const plain = (v, max = 12000) => typeof v === 'string' ? v.slice(0, max) : '';
  const number = (v, fallback, lo = -1000000, hi = 1000000) => typeof v === 'number' && Number.isFinite(v) ? clamp(v, lo, hi) : fallback;
  const clone = value => JSON.parse(JSON.stringify(value));
  const icon = name => {
    const el = document.createElementNS(NS, 'svg');
    const use = document.createElementNS(NS, 'use');
    use.setAttribute('href', '#i-' + name);
    el.append(use);
    return el;
  };
  const el = (tag, className, text) => {
    const result = document.createElement(tag);
    if (className) result.className = className;
    if (text !== undefined) result.textContent = text;
    return result;
  };
  function newBoard(title = 'Untitled story') {
    return { id: uid('board'), title, nodes: [], edges: [], snapToGrid: true, view: { x: 100, y: 100, z: 1 }, updated: Date.now() };
  }
  function normalizeBoard(raw, imported = false) {
    if (!raw || !Array.isArray(raw.nodes) || !Array.isArray(raw.edges)) throw new Error('This file does not contain a story board.');
    if (raw.nodes.length > 2000 || raw.edges.length > 6000) throw new Error('This board is too large (maximum 2,000 cards and 6,000 connections).');
    const ids = new Set();
    const nodes = raw.nodes.map(n => {
      if (!n || !Object.hasOwn(KINDS, n.kind) || typeof n.id !== 'string' || !n.id || n.id.length > 100 || ids.has(n.id)) throw new Error('A card has an invalid type or duplicate ID.');
      ids.add(n.id);
      const def = KINDS[n.kind];
      const node = { id: n.id, kind: n.kind, color: Object.hasOwn(COLORS, n.color) ? n.color : def.color,
        x: number(n.x, 0), y: number(n.y, 0), w: number(n.w, def.w, 220, 2000), h: number(n.h, def.h, 150, 2000) };
      for (const key of FIELDS) node[key] = plain(n[key], key === 'title' ? 200 : 12000);
      node.faction = ['any', 'alliance', 'horde', 'neutral'].includes(n.faction) ? n.faction : 'any';
      node.status = ['idea', 'draft', 'decided'].includes(n.status) ? n.status : 'idea';
      return node;
    });
    const edgeIds = new Set();
    const edges = raw.edges.map(e => {
      if (!e || typeof e.id !== 'string' || !e.id || e.id.length > 100 || edgeIds.has(e.id) || !ids.has(e.from) || !ids.has(e.to) || e.from === e.to) throw new Error('A connection has an invalid ID or missing endpoint.');
      edgeIds.add(e.id);
      return { id: e.id, from: e.from, to: e.to, fromSide: validSide(e.fromSide), toSide: validSide(e.toSide),
        label: plain(e.label, 200), style: e.style === 'dashed' ? 'dashed' : 'solid' };
    });
    return { id: imported ? uid('board') : plain(raw.id, 100) || uid('board'), title: plain(raw.title, 100) || 'Untitled story', nodes, edges, snapToGrid: raw.snapToGrid !== false,
      view: { x: number(raw.view?.x, 100), y: number(raw.view?.y, 100), z: number(raw.view?.z, 1, .15, 2.5) }, updated: number(raw.updated, Date.now(), 0, 1e15) };
  }
  let workspace;
  let startupMessage = '';
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE) || 'null');
    if (raw && Array.isArray(raw.boards) && raw.boards.length) {
      const boards = raw.boards.slice(0, 100).map(b => normalizeBoard(b));
      if (new Set(boards.map(b => b.id)).size !== boards.length) throw new Error('Duplicate board IDs');
      workspace = { boards, active: boards.some(b => b.id === raw.active) ? raw.active : boards[0].id };
    }
  } catch {
    startupMessage = 'Browser save unavailable or unreadable. Export your board to keep a backup.';
    savingFailed = true;
  }
  if (!workspace) { const board = newBoard(); workspace = { boards: [board], active: board.id }; }
  const board = () => workspace.boards.find(b => b.id === workspace.active);
  const node = id => board().nodes.find(n => n.id === id);
  const edge = id => board().edges.find(e => e.id === id);
  const primary = () => node([...selection][0]);
  const snap = (value, free = false) => board().snapToGrid && !free ? Math.round(value / GRID) * GRID : value;
  function snapSize(value, min, max, free = false) {
    return board().snapToGrid && !free
      ? clamp(snap(value), Math.ceil(min / GRID) * GRID, Math.floor(max / GRID) * GRID)
      : clamp(value, min, max);
  }
  function toggleSnap() {
    if (gesture || pinch) return;
    checkpoint(); board().snapToGrid = !board().snapToGrid; renderView(); save();
    toast(board().snapToGrid ? 'Snap to grid on. Hold Alt while dragging for free placement.' : 'Snap to grid off.');
  }
  function toast(message) {
    $('toast').textContent = message;
    $('toast').hidden = false;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { $('toast').hidden = true; }, 4200);
  }
  function saveNow() {
    clearTimeout(saveTimer);
    try {
      localStorage.setItem(STORAGE, JSON.stringify(workspace));
      savingFailed = false;
      $('save-state').className = '';
      $('save-state').replaceChildren(el('i'), document.createTextNode('Saved on this device'));
    } catch {
      const firstFailure = !savingFailed;
      savingFailed = true;
      $('save-state').className = 'error';
      $('save-state').replaceChildren(el('i'), document.createTextNode('Export to save a backup'));
      if (firstFailure) toast('Browser storage is unavailable or full. Export a board file to keep your work.');
    }
  }
  function save() {
    board().updated = Date.now();
    $('save-state').className = 'unsaved';
    $('save-state').replaceChildren(el('i'), document.createTextNode('Saving…'));
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveNow, 300);
  }
  function stack() {
    if (!history.has(board().id)) history.set(board().id, { undo: [], redo: [] });
    return history.get(board().id);
  }
  function checkpoint() {
    const h = stack();
    h.undo.push(JSON.stringify(board()));
    if (h.undo.length > 80) h.undo.shift();
    h.redo = [];
    updateHistory();
  }
  function updateHistory() { $('undo').disabled = !stack().undo.length; $('redo').disabled = !stack().redo.length; }
  function undo(redo = false) {
    cancelGesture();
    const h = stack(), from = redo ? h.redo : h.undo, to = redo ? h.undo : h.redo;
    if (!from.length) return;
    to.push(JSON.stringify(board()));
    const restored = JSON.parse(from.pop());
    workspace.boards[workspace.boards.findIndex(b => b.id === workspace.active)] = restored;
    for (const id of selection) if (!node(id)) selection.delete(id);
    if (!edge(selectedEdge)) selectedEdge = null;
    renderAll(); save();
  }
  function screenPoint(event) { const rect = viewport.getBoundingClientRect(); return { x: event.clientX - rect.left, y: event.clientY - rect.top }; }
  function worldPoint(point) { const v = board().view; return { x: (point.x - v.x) / v.z, y: (point.y - v.y) / v.z }; }
  function center() { return worldPoint({ x: viewport.clientWidth / 2, y: viewport.clientHeight / 2 }); }
  function renderView() {
    const v = board().view;
    $('world').style.transform = `translate(${v.x}px,${v.y}px) scale(${v.z})`;
    const grid = GRID * v.z;
    viewport.style.backgroundSize = `${grid}px ${grid}px`;
    viewport.style.backgroundPosition = `${v.x - grid / 2}px ${v.y - grid / 2}px`;
    $('zoom-level').textContent = Math.round(v.z * 100) + '%';
    $('snap-grid').classList.toggle('active', board().snapToGrid);
    $('snap-grid').setAttribute('aria-pressed', String(board().snapToGrid));
  }
  function zoomAt(z, point = { x: viewport.clientWidth / 2, y: viewport.clientHeight / 2 }) {
    const before = worldPoint(point), v = board().view;
    v.z = clamp(z, .15, 2.5); v.x = point.x - before.x * v.z; v.y = point.y - before.y * v.z;
    renderView(); save();
  }
  function bounds(nodes = board().nodes) {
    if (!nodes.length) return null;
    return { left: Math.min(...nodes.map(n => n.x)), top: Math.min(...nodes.map(n => n.y)),
      right: Math.max(...nodes.map(n => n.x + n.w)), bottom: Math.max(...nodes.map(n => n.y + n.h)) };
  }
  function fit(nodes = board().nodes) {
    const b = bounds(nodes);
    if (!b) { board().view = { x: 100, y: 100, z: 1 }; renderView(); save(); return; }
    const width = viewport.clientWidth, height = viewport.clientHeight;
    const z = clamp(Math.min((width - 130) / (b.right - b.left), (height - 150) / (b.bottom - b.top)), .15, 1.3);
    board().view = { x: width / 2 - (b.left + b.right) / 2 * z, y: height / 2 - (b.top + b.bottom) / 2 * z, z };
    renderView(); save();
  }
  function setTool(value) {
    tool = value; connectionFrom = null; $('draft-edge').hidden = true;
    viewport.classList.toggle('hand', tool === 'hand'); viewport.classList.toggle('connect', tool === 'connect');
    document.querySelectorAll('[data-tool]').forEach(button => {
      button.classList.toggle('active', button.dataset.tool === tool);
      button.setAttribute('aria-pressed', String(button.dataset.tool === tool));
    });
    $('mode-hint').hidden = tool !== 'connect';
    $('mode-hint').textContent = 'Click a starting card, then a destination. Esc to cancel.';
  }
  function updateSelection() {
    for (const [id, item] of nodeElements) item.classList.toggle('selected', selection.has(id));
    renderEdges(); renderInspector(); renderOutline();
  }
  function chooseNode(id, toggle = false) {
    selectedEdge = null;
    if (!toggle) selection.clear();
    if (toggle && selection.has(id)) selection.delete(id); else selection.add(id);
    updateSelection();
  }
  function deselect() { selection.clear(); selectedEdge = null; updateSelection(); }
  function addNode(kind, point = center()) {
    if (!Object.hasOwn(KINDS, kind)) return;
    if (board().nodes.length >= 2000) { toast('This board has reached its 2,000-card limit. Start another board.'); return; }
    checkpoint();
    const def = KINDS[kind];
    const w = snap(def.w), h = snap(def.h), offset = board().snapToGrid ? GRID : 32;
    let x = snap(point.x - w / 2), y = snap(point.y - h / 2);
    for (let i = 0; i < 8 && board().nodes.some(n => Math.abs(n.x - x) < 15 && Math.abs(n.y - y) < 15); i++) { x += offset; y += offset; }
    const item = { id: uid('node'), kind, color: def.color, x, y, w, h,
      title: def.title, body: '', faction: 'any', status: 'idea', zone: '', level: '', bosses: '', armor: '', condition: '' };
    board().nodes.push(item);
    selection.clear(); selection.add(item.id); selectedEdge = null; setTool('select');
    renderAll(); save(); $('edit-title').focus(); $('edit-title').select();
  }
  function connect(from, to, fromSide = null, toSide = null) {
    if (!node(from) || !node(to) || from === to || node(from).kind === 'chapter' || node(to).kind === 'chapter') return;
    const existing = board().edges.find(e => e.from === from && e.to === to);
    if (existing) {
      // Redrawing an existing route from a handle changes its anchors and retains its label.
      if (fromSide || toSide) {
        checkpoint(); existing.fromSide = fromSide; existing.toSide = toSide;
        selection.clear(); selectedEdge = existing.id; updateSelection(); save();
      } else { toast('These cards are already connected. Click the path to edit its label.'); }
      return;
    }
    if (board().edges.length >= 6000) { toast('This board has reached its connection limit.'); return; }
    checkpoint();
    const item = { id: uid('edge'), from, to, fromSide, toSide, label: '', style: 'solid' };
    board().edges.push(item); selection.clear(); selectedEdge = item.id;
    updateSelection(); save();
  }
  function removeSelected() {
    if (!selection.size && !selectedEdge) return;
    checkpoint();
    const count = selection.size;
    board().nodes = board().nodes.filter(n => !selection.has(n.id));
    board().edges = board().edges.filter(e => e.id !== selectedEdge && !selection.has(e.from) && !selection.has(e.to));
    selection.clear(); selectedEdge = null;
    renderAll(); save(); toast(count ? `${count === 1 ? 'Card' : count + ' cards'} deleted. Ctrl+Z to undo.` : 'Connection deleted. Ctrl+Z to undo.');
  }
  function duplicate() {
    const originals = board().nodes.filter(n => selection.has(n.id));
    if (!originals.length) return;
    if (board().nodes.length + originals.length > 2000) { toast('This would exceed the board’s card limit.'); return; }
    checkpoint();
    const mapping = new Map(originals.map(n => [n.id, uid('node')]));
    const offset = board().snapToGrid ? GRID * 2 : 36;
    const dx = snap(originals[0].x + offset) - originals[0].x, dy = snap(originals[0].y + offset) - originals[0].y;
    const copies = originals.map(n => ({ ...clone(n), id: mapping.get(n.id), x: n.x + dx, y: n.y + dy }));
    const edges = board().edges.filter(e => mapping.has(e.from) && mapping.has(e.to)).map(e => ({ ...e, id: uid('edge'), from: mapping.get(e.from), to: mapping.get(e.to) }));
    board().nodes.push(...copies); board().edges.push(...edges);
    selection.clear(); copies.forEach(n => selection.add(n.id)); selectedEdge = null;
    renderAll(); save();
  }
  function colorize(item, color) { const colors = COLORS[color]; item.style.setProperty('--tint', colors.tint); item.style.setProperty('--accent', colors.accent); }
  function buildCard(n) {
    const card = el('article', 'card ' + n.kind + (selection.has(n.id) ? ' selected' : ''));
    card.dataset.node = n.id; card.tabIndex = 0; card.setAttribute('aria-label', `${KINDS[n.kind].label}: ${n.title}`);
    colorize(card, n.color);
    card.style.left = n.x + 'px'; card.style.top = n.y + 'px'; card.style.width = n.w + 'px'; card.style.height = n.h + 'px';
    card.append(el('div', 'card-color-bar'));
    const header = el('div', 'card-header'), kind = el('span', 'card-kind');
    kind.append(icon(n.kind), document.createTextNode(KINDS[n.kind].label));
    const status = el('span', 'card-status ' + n.status); status.title = n.status[0].toUpperCase() + n.status.slice(1);
    header.append(kind, status);
    card.append(header, el('h2', 'card-title', n.title || 'Untitled'), el('p', 'card-body', n.body || (n.kind === 'chapter' ? 'Drag cards into this chapter' : 'Add your story notes…')));
    const footer = el('div', 'card-footer');
    if (n.faction !== 'any') footer.append(el('span', 'badge ' + n.faction, n.faction[0].toUpperCase() + n.faction.slice(1)));
    if (n.level) footer.append(el('span', 'badge', n.level));
    if (n.armor) footer.append(el('span', 'badge', 'Armor choices'));
    if (!footer.childElementCount) footer.append(document.createTextNode(n.kind === 'zone' ? n.zone || 'A new place in your story' : n.condition ? 'Unlock rule added' : 'Click to edit details'));
    card.append(footer);
    if (n.kind !== 'chapter') for (const side of SIDES) {
      const port = el('button', 'port'); port.dataset.side = side; port.title = `Drag from ${side} to another card to connect`; port.setAttribute('aria-label', `Connect ${n.title || 'card'} from ${side}`); card.append(port);
    }
    const resize = el('div', 'resize-handle'); resize.title = 'Drag to resize'; card.append(resize);
    return card;
  }
  function renderCards() {
    nodeElements.clear(); $('cards').replaceChildren();
    for (const n of [...board().nodes].sort((a, b) => Number(a.kind !== 'chapter') - Number(b.kind !== 'chapter'))) {
      const item = buildCard(n); nodeElements.set(n.id, item); $('cards').append(item);
    }
    $('empty-state').hidden = board().nodes.length > 0;
    $('node-count').textContent = board().nodes.length;
  }
  function positions() {
    for (const n of board().nodes) {
      const item = nodeElements.get(n.id);
      if (!item) continue;
      item.style.left = n.x + 'px'; item.style.top = n.y + 'px'; item.style.width = n.w + 'px'; item.style.height = n.h + 'px';
    }
    renderEdges();
  }
  function anchor(n, side) {
    if (side === 'top') return { x: n.x + n.w / 2, y: n.y, dx: 0, dy: -1 };
    if (side === 'bottom') return { x: n.x + n.w / 2, y: n.y + n.h, dx: 0, dy: 1 };
    if (side === 'left') return { x: n.x, y: n.y + n.h / 2, dx: -1, dy: 0 };
    return { x: n.x + n.w, y: n.y + n.h / 2, dx: 1, dy: 0 };
  }
  function nearestSide(n, point) {
    return SIDES.reduce((best, side) => {
      const a = anchor(n, side), b = anchor(n, best);
      return Math.hypot(point.x - a.x, point.y - a.y) < Math.hypot(point.x - b.x, point.y - b.y) ? side : best;
    }, 'left');
  }
  function curve(a, b, fromSide = null, toSide = null) {
    const dx = b.x + b.w / 2 - a.x - a.w / 2;
    const dy = b.y + b.h / 2 - a.y - a.h / 2;
    const automatic = Math.abs(dy) > Math.abs(dx)
      ? (dy >= 0 ? ['bottom', 'top'] : ['top', 'bottom'])
      : (dx >= 0 ? ['right', 'left'] : ['left', 'right']);
    return curveBetween(anchor(a, fromSide || automatic[0]), anchor(b, toSide || automatic[1]));
  }
  function curveBetween(a, b) {
    const reach = clamp(Math.hypot(b.x - a.x, b.y - a.y) * .45, 45, 350);
    const c1 = { x: a.x + reach * a.dx, y: a.y + reach * a.dy };
    const c2 = { x: b.x + reach * b.dx, y: b.y + reach * b.dy };
    return { d: `M${a.x},${a.y} C${c1.x},${c1.y} ${c2.x},${c2.y} ${b.x},${b.y}`,
      x: (a.x + c1.x * 3 + c2.x * 3 + b.x) / 8, y: (a.y + c1.y * 3 + c2.y * 3 + b.y) / 8 };
  }
  function svgEl(tag, attrs = {}) { const item = document.createElementNS(NS, tag); for (const [key, value] of Object.entries(attrs)) item.setAttribute(key, value); return item; }
  function renderEdges() {
    $('edge-layer').replaceChildren();
    const nodes = new Map(board().nodes.map(n => [n.id, n]));
    for (const e of board().edges) {
      const a = nodes.get(e.from), b = nodes.get(e.to); if (!a || !b) continue;
      const path = curve(a, b, e.fromSide, e.toSide), group = svgEl('g', { class: 'edge-group' + (selectedEdge === e.id ? ' selected' : ''), 'data-edge': e.id });
      const visible = svgEl('path', { d: path.d, class: 'edge-visible', ...(e.style === 'dashed' ? { 'stroke-dasharray': '7 6' } : {}) });
      group.append(visible, svgEl('path', { d: path.d, class: 'edge-hit' }));
      if (e.label) { const label = svgEl('text', { x: path.x, y: path.y - 10, class: 'edge-label' }); label.textContent = e.label.length > 55 ? e.label.slice(0, 52) + '…' : e.label; group.append(label); }
      $('edge-layer').append(group);
    }
  }
  function renderOutline() {
    const query = $('search').value.toLowerCase(); $('outline').replaceChildren();
    const nodes = board().nodes.filter(n => (n.title + ' ' + n.zone + ' ' + KINDS[n.kind].label).toLowerCase().includes(query));
    if (!nodes.length) { $('outline').append(el('p', 'outline-empty', board().nodes.length ? 'No matching cards.' : 'Your cards will appear here.')); return; }
    for (const n of nodes) {
      const button = el('button', 'outline-row' + (selection.has(n.id) ? ' selected' : '')); colorize(button, n.color);
      const arrow = icon('chevron'); arrow.classList.add('outline-arrow');
      button.append(icon(n.kind), el('span', '', n.title || 'Untitled'), arrow);
      button.addEventListener('click', () => { chooseNode(n.id); requestAnimationFrame(() => fit([n])); });
      $('outline').append(button);
    }
  }
  function renderInspector() {
    const n = primary(), e = edge(selectedEdge);
    $('inspector').hidden = !n && !e;
    $('node-details').hidden = !n; $('edge-details').hidden = !e;
    if (n) {
      $('inspector-caption').textContent = selection.size > 1 ? selection.size + ' CARDS SELECTED' : 'CARD DETAILS';
      $('inspector-type').replaceChildren(icon(n.kind), document.createTextNode(KINDS[n.kind].label)); colorize($('inspector-type'), n.color);
      for (const key of FIELDS) $('edit-' + key).value = n[key] || (key === 'faction' ? 'any' : key === 'status' ? 'idea' : '');
      $('zone-fields').hidden = n.kind !== 'zone';
      $('color-picker').replaceChildren();
      for (const [name, color] of Object.entries(COLORS)) {
        const button = el('button', 'color-swatch' + (n.color === name ? ' active' : '')); button.style.background = color.tint;
        button.title = name; button.setAttribute('aria-label', name + ' card color'); button.setAttribute('aria-pressed', String(n.color === name));
        button.addEventListener('click', () => { checkpoint(); for (const id of selection) if (node(id)) node(id).color = name; renderCards(); renderInspector(); renderOutline(); save(); });
        $('color-picker').append(button);
      }
    } else if (e) {
      $('inspector-caption').textContent = 'CONNECTION DETAILS';
      $('edge-endpoints').textContent = `${node(e.from)?.title || 'Untitled'} → ${node(e.to)?.title || 'Untitled'}`;
      $('edit-edge-label').value = e.label; $('edit-edge-style').value = e.style;
    }
  }
  function renderBoards() {
    $('board-picker').replaceChildren();
    for (const b of workspace.boards) { const option = el('option', '', b.title); option.value = b.id; $('board-picker').append(option); }
    $('board-picker').value = workspace.active; $('board-title').value = board().title;
  }
  function renderAll() { renderBoards(); renderCards(); renderEdges(); renderInspector(); renderOutline(); renderView(); updateHistory(); }

  // One undo entry per field edit, with immediate canvas updates as the author types.
  for (const key of FIELDS) {
    const field = $('edit-' + key); let recorded = false;
    field.addEventListener('focus', () => { recorded = false; });
    field.addEventListener('input', () => {
      const n = primary(); if (!n) return;
      if (!recorded) { checkpoint(); recorded = true; }
      n[key] = field.value; renderCards(); renderEdges(); renderOutline(); save();
    });
    field.addEventListener('change', () => { recorded = false; });
  }
  for (const key of ['label', 'style']) {
    const field = $('edit-edge-' + key); let recorded = false;
    field.addEventListener('focus', () => { recorded = false; });
    field.addEventListener('input', () => { const e = edge(selectedEdge); if (!e) return; if (!recorded) { checkpoint(); recorded = true; } e[key] = field.value; renderEdges(); save(); });
    field.addEventListener('change', () => { recorded = false; });
  }
  let titleRecorded = false;
  $('board-title').addEventListener('focus', () => { titleRecorded = false; });
  $('board-title').addEventListener('input', () => { if (!titleRecorded) { checkpoint(); titleRecorded = true; } board().title = $('board-title').value || 'Untitled story'; const option = [...$('board-picker').options].find(o => o.value === workspace.active); if (option) option.textContent = board().title; save(); });
  $('board-title').addEventListener('blur', () => { $('board-title').value = board().title; });
  $('board-picker').addEventListener('change', () => { saveNow(); cancelGesture(); workspace.active = $('board-picker').value; selection.clear(); selectedEdge = null; $('search').value = ''; setTool('select'); renderAll(); save(); });
  $('new-board').addEventListener('click', () => {
    if (workspace.boards.length >= 100) { toast('Export your boards before creating more (100-board limit).'); return; }
    saveNow(); cancelGesture(); const b = newBoard('Untitled story ' + (workspace.boards.length + 1)); workspace.boards.push(b); workspace.active = b.id;
    selection.clear(); selectedEdge = null; $('search').value = ''; setTool('select'); renderAll(); save(); $('board-title').focus(); $('board-title').select();
  });
  $('search').addEventListener('input', renderOutline);
  $('close-inspector').addEventListener('click', deselect);
  $('duplicate').addEventListener('click', duplicate);
  $('delete-node').addEventListener('click', removeSelected);
  $('delete-edge').addEventListener('click', removeSelected);
  $('undo').addEventListener('click', () => undo()); $('redo').addEventListener('click', () => undo(true));
  $('zoom-in').addEventListener('click', () => zoomAt(board().view.z * 1.2)); $('zoom-out').addEventListener('click', () => zoomAt(board().view.z / 1.2));
  $('zoom-level').addEventListener('click', () => zoomAt(1)); $('fit').addEventListener('click', () => fit());
  $('snap-grid').addEventListener('click', toggleSnap);
  $('first-zone').addEventListener('click', () => addNode('zone')); $('quick-zone').addEventListener('click', () => addNode('zone')); $('quick-note').addEventListener('click', () => addNode('note'));
  $('help').addEventListener('click', () => $('help-dialog').showModal());
  document.querySelectorAll('[data-tool]').forEach(button => button.addEventListener('click', () => setTool(button.dataset.tool)));
  for (const [kind, def] of Object.entries(KINDS)) {
    const button = el('button', 'palette-card'); button.draggable = true; button.title = 'Add ' + def.label.toLowerCase();
    const holder = el('span', 'palette-icon'); holder.append(icon(kind)); colorize(holder, def.color);
    button.append(holder, el('span', 'palette-name', def.label));
    button.addEventListener('click', () => addNode(kind));
    button.addEventListener('dragstart', event => { event.dataTransfer.setData('application/x-asr-story-card', kind); event.dataTransfer.effectAllowed = 'copy'; });
    $('palette').append(button);
  }
  viewport.addEventListener('dragover', event => { if ([...event.dataTransfer.types].includes('application/x-asr-story-card')) { event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; } });
  viewport.addEventListener('drop', event => { const kind = event.dataTransfer.getData('application/x-asr-story-card'); if (Object.hasOwn(KINDS, kind)) { event.preventDefault(); addNode(kind, worldPoint(screenPoint(event))); } });
  viewport.addEventListener('wheel', event => { event.preventDefault(); if (gesture || pinch) return; zoomAt(board().view.z * Math.exp(-clamp(event.deltaY * (event.deltaMode === 1 ? 16 : 1), -120, 120) * .0025), screenPoint(event)); }, { passive: false });
  viewport.addEventListener('dblclick', event => {
    if (event.target.closest('.card')) { const n = node(event.target.closest('.card').dataset.node); if (n) { chooseNode(n.id); $('edit-title').focus(); $('edit-title').select(); } return; }
    if (event.target.closest('button, .edge-group, .canvas-tools, .zoom-controls')) return;
    addNode('note', worldPoint(screenPoint(event)));
  });

  function startConnection(id, side = 'right', point) {
    connectionFrom = id;
    gesture = { kind: 'connect', from: id, side, start: point };
    $('draft-edge').hidden = false;
    $('mode-hint').hidden = false; $('mode-hint').textContent = 'Drop on another card to connect. Esc to cancel.';
  }
  function cancelGesture() {
    gesture = null; pinch = null; touchPoints.clear(); connectionFrom = null;
    $('draft-edge').hidden = true; $('selection-box').hidden = true;
    viewport.classList.remove('panning');
    for (const item of nodeElements.values()) item.classList.remove('connect-target');
    $('mode-hint').hidden = tool !== 'connect';
    $('mode-hint').textContent = 'Click a starting card, then a destination. Esc to cancel.';
  }
  viewport.addEventListener('pointerdown', event => {
    if (event.button > 1 || event.target.closest('.canvas-tools, .zoom-controls, .empty-state, .mode-hint')) return;
    if (event.pointerType === 'touch') {
      touchPoints.set(event.pointerId, screenPoint(event));
      if (touchPoints.size === 2) {
        if (gesture?.recorded) save();
        gesture = null;
        const [a, b] = [...touchPoints.values()];
        const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
        pinch = { anchor: worldPoint(mid), distance: Math.max(1, Math.hypot(a.x - b.x, a.y - b.y)), z: board().view.z };
        viewport.setPointerCapture(event.pointerId); return;
      }
    }
    const point = screenPoint(event), world = worldPoint(point);
    const card = event.target.closest('.card'), port = event.target.closest('.port'), path = event.target.closest('.edge-group');
    if (event.button === 1 || tool === 'hand' || space) {
      gesture = { kind: 'pan', start: point, view: { ...board().view } }; viewport.classList.add('panning');
    } else if (port && card) {
      chooseNode(card.dataset.node); startConnection(card.dataset.node, port.dataset.side, world);
    } else if (path) {
      selection.clear(); selectedEdge = path.dataset.edge; updateSelection();
    } else if (card) {
      const n = node(card.dataset.node);
      if (tool === 'connect') {
        if (n.kind === 'chapter') { toast('Connect story cards inside the chapter.'); return; }
        if (connectionFrom && connectionFrom !== n.id) { connect(connectionFrom, n.id); connectionFrom = null; $('draft-edge').hidden = true; $('mode-hint').textContent = 'Connection added. Click a card to start another.'; }
        else { connectionFrom = n.id; chooseNode(n.id); $('mode-hint').textContent = 'Now click the next card in the story.'; }
        return;
      }
      if (event.target.closest('.resize-handle')) {
        if (!selection.has(n.id)) chooseNode(n.id);
        gesture = { kind: 'resize', id: n.id, start: world, w: n.w, h: n.h, recorded: false };
      } else {
        if (event.shiftKey) chooseNode(n.id, true); else if (!selection.has(n.id)) chooseNode(n.id);
        const moving = new Set(selection);
        for (const frame of board().nodes.filter(f => f.kind === 'chapter' && selection.has(f.id))) {
          for (const child of board().nodes) if (child.id !== frame.id && child.x >= frame.x && child.y >= frame.y && child.x + child.w <= frame.x + frame.w && child.y + child.h <= frame.y + frame.h) moving.add(child.id);
        }
        gesture = { kind: 'drag', start: world, anchorId: n.id, points: [...moving].map(id => ({ id, x: node(id).x, y: node(id).y })), recorded: false };
      }
    } else {
      selectedEdge = null;
      if (!event.shiftKey) selection.clear();
      updateSelection();
      if (tool === 'connect') { connectionFrom = null; $('draft-edge').hidden = true; return; }
      gesture = { kind: 'box', start: point, initial: new Set(selection) };
    }
    if (gesture) { event.preventDefault(); viewport.setPointerCapture(event.pointerId); }
  });
  viewport.addEventListener('pointermove', event => {
    const point = screenPoint(event);
    if (event.pointerType === 'touch' && touchPoints.has(event.pointerId)) touchPoints.set(event.pointerId, point);
    if (pinch && touchPoints.size >= 2) {
      const [a, b] = [...touchPoints.values()], v = board().view;
      v.z = clamp(pinch.z * Math.hypot(a.x - b.x, a.y - b.y) / pinch.distance, .15, 2.5);
      v.x = (a.x + b.x) / 2 - pinch.anchor.x * v.z; v.y = (a.y + b.y) / 2 - pinch.anchor.y * v.z;
      renderView(); return;
    }
    const world = worldPoint(point);
    if (!gesture) return;
    if (gesture.kind === 'pan') {
      board().view.x = gesture.view.x + point.x - gesture.start.x; board().view.y = gesture.view.y + point.y - gesture.start.y; renderView();
    } else if (gesture.kind === 'drag' || gesture.kind === 'resize') {
      const dx = world.x - gesture.start.x, dy = world.y - gesture.start.y;
      if (!gesture.recorded && Math.hypot(dx, dy) * board().view.z < 3) return;
      if (!gesture.recorded) { checkpoint(); gesture.recorded = true; }
      if (gesture.kind === 'drag') {
        const anchor = gesture.points.find(p => p.id === gesture.anchorId) || gesture.points[0];
        if (!anchor) return;
        // Snap one anchor, then translate the whole group without changing its layout.
        const moveX = snap(anchor.x + dx, event.altKey) - anchor.x;
        const moveY = snap(anchor.y + dy, event.altKey) - anchor.y;
        for (const start of gesture.points) { const n = node(start.id); if (n) { n.x = start.x + moveX; n.y = start.y + moveY; } }
      } else {
        const n = node(gesture.id);
        if (n) { n.w = snapSize(gesture.w + dx, 220, 2000, event.altKey); n.h = snapSize(gesture.h + dy, 150, 2000, event.altKey); }
      }
      positions();
    } else if (gesture.kind === 'box') {
      const left = Math.min(point.x, gesture.start.x), top = Math.min(point.y, gesture.start.y), width = Math.abs(point.x - gesture.start.x), height = Math.abs(point.y - gesture.start.y);
      const box = $('selection-box'); box.hidden = false; Object.assign(box.style, { left: left + 'px', top: top + 'px', width: width + 'px', height: height + 'px' });
      const a = worldPoint({ x: left, y: top }), b = worldPoint({ x: left + width, y: top + height });
      selection.clear(); for (const id of gesture.initial) selection.add(id);
      for (const n of board().nodes) if (n.x < b.x && n.x + n.w > a.x && n.y < b.y && n.y + n.h > a.y && n.kind !== 'chapter') selection.add(n.id);
      for (const [id, item] of nodeElements) item.classList.toggle('selected', selection.has(id));
    } else if (gesture.kind === 'connect') {
      const n = node(gesture.from); if (!n) return;
      const start = anchor(n, gesture.side);
      $('draft-edge').setAttribute('d', curveBetween(start, { ...world, dx: -start.dx, dy: -start.dy }).d);
      const hover = document.elementFromPoint(event.clientX, event.clientY)?.closest('.card');
      for (const [id, item] of nodeElements) item.classList.toggle('connect-target', hover === item && id !== gesture.from && node(id)?.kind !== 'chapter');
    }
  });
  function endPointer(event) {
    touchPoints.delete(event.pointerId);
    if (pinch) { if (touchPoints.size < 2) { pinch = null; touchPoints.clear(); save(); } return; }
    if (!gesture) return;
    const current = gesture;
    if (current.kind === 'connect' && event.type !== 'pointercancel') {
      const hit = document.elementFromPoint(event.clientX, event.clientY);
      const target = hit?.closest('.card');
      if (target && target.dataset.node !== current.from) {
        const targetNode = node(target.dataset.node);
        const targetSide = validSide(hit.closest('.port')?.dataset.side) || nearestSide(targetNode, worldPoint(screenPoint(event)));
        connect(current.from, targetNode.id, current.side, targetSide);
      }
      else toast('Drag the blue handle onto another story card.');
    }
    if (current.recorded || current.kind === 'pan') save();
    cancelGesture(); updateSelection();
    if (viewport.hasPointerCapture(event.pointerId)) viewport.releasePointerCapture(event.pointerId);
  }
  viewport.addEventListener('pointerup', endPointer); viewport.addEventListener('pointercancel', endPointer);
  viewport.addEventListener('keydown', event => {
    if ((event.key === 'Enter' || event.key === ' ') && event.target.closest('.card')) {
      event.preventDefault(); chooseNode(event.target.closest('.card').dataset.node); $('edit-title').focus();
    }
  });
  document.addEventListener('keydown', event => {
    if (event.target.closest('input,textarea,select,[contenteditable="true"]') || $('help-dialog').open) return;
    const mod = event.ctrlKey || event.metaKey, key = event.key.toLowerCase();
    if (mod && key === 'z') { event.preventDefault(); undo(event.shiftKey); }
    else if (mod && key === 'y') { event.preventDefault(); undo(true); }
    else if (mod && key === 'd') { event.preventDefault(); duplicate(); }
    else if (mod && key === 's') { event.preventDefault(); saveNow(); toast(savingFailed ? 'Export a board file to keep your work.' : 'Board saved in this browser.'); }
    else if (mod && key === 'a') { event.preventDefault(); selection.clear(); board().nodes.forEach(n => selection.add(n.id)); selectedEdge = null; updateSelection(); }
    else if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); removeSelected(); }
    else if (event.code === 'Space') { event.preventDefault(); space = true; viewport.classList.add('space'); }
    else if (event.key === 'Escape') { cancelGesture(); setTool('select'); deselect(); $('export-menu').hidden = true; $('export').setAttribute('aria-expanded', 'false'); }
    else if (!mod && !event.altKey) {
      if (key === 'v') setTool('select'); else if (key === 'h') setTool('hand'); else if (key === 'c') setTool('connect');
      else if (key === 'n') addNode('note'); else if (key === 'z') addNode('zone'); else if (key === 'f') fit();
      else if (key === 'g') { if (!event.repeat) toggleSnap(); }
      else if (event.key === '+' || event.key === '=') zoomAt(board().view.z * 1.2); else if (event.key === '-') zoomAt(board().view.z / 1.2);
    }
  });
  document.addEventListener('keyup', event => { if (event.code === 'Space') { space = false; viewport.classList.remove('space'); } });
  window.addEventListener('blur', () => { space = false; viewport.classList.remove('space'); if (gesture?.recorded) save(); cancelGesture(); });

  const safeName = () => board().title.replace(/[<>:"/\\|?*\x00-\x1f]/g, '-').slice(0, 90).trim() || 'ASR story';
  function download(text, mime, extension) {
    const url = URL.createObjectURL(new Blob([text], { type: mime }));
    const link = el('a'); link.href = url; link.download = safeName() + '.' + extension;
    document.body.append(link); link.click(); link.remove(); setTimeout(() => URL.revokeObjectURL(url), 30000);
  }
  function markdown() {
    const b = board(), lines = ['# ' + b.title, '', 'Story planning board. Armor lists are selections to review, not game imports.', ''];
    for (const n of b.nodes) {
      lines.push('## ' + (n.title || 'Untitled'), '', `Type: ${KINDS[n.kind].label} · Status: ${n.status} · Faction: ${n.faction}`, '');
      if (n.body) lines.push(n.body, '');
      for (const [key, label] of [['zone', 'Zone'], ['level', 'Level / chapter'], ['bosses', 'Bosses & encounters'], ['armor', 'Armor to import'], ['condition', 'Unlock / completion rule']]) if (n[key]) lines.push('### ' + label, '', n[key], '');
    }
    lines.push('## Story connections', '');
    for (const e of b.edges) lines.push(`- ${node(e.from)?.title || 'Untitled'} → ${node(e.to)?.title || 'Untitled'}${e.label ? ': ' + e.label : ''}${e.style === 'dashed' ? ' (optional / side path)' : ''}`);
    if (!b.edges.length) lines.push('No connections yet.');
    return lines.join('\n');
  }
  const xml = value => String(value).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[c]);
  function wrapped(value, max, limit) {
    const words = value.split(/\s+/), lines = []; let line = '';
    for (let word of words) {
      while (word.length > max) { if (line) { lines.push(line); line = ''; } lines.push(word.slice(0, max)); word = word.slice(max); }
      if (line.length + word.length + 1 > max) { lines.push(line); line = word; } else line += (line ? ' ' : '') + word;
    }
    if (line) lines.push(line);
    return lines.slice(0, limit).map((s, i) => i === limit - 1 && lines.length > limit ? s.slice(0, -1) + '…' : s);
  }
  function svgExport() {
    const b = board(), area = bounds() || { left: 0, top: 0, right: 700, bottom: 400 };
    const left = area.left - 70, top = area.top - 120, width = area.right - left + 70, height = area.bottom - top + 70;
    const pieces = [`<svg xmlns="http://www.w3.org/2000/svg" width="${Math.ceil(width)}" height="${Math.ceil(height)}" viewBox="${left} ${top} ${width} ${height}"><title>${xml(b.title)}</title><defs><pattern id="dots" width="${GRID}" height="${GRID}" patternUnits="userSpaceOnUse"><circle cx="0" cy="0" r=".8" fill="#cfcec8"/></pattern><marker id="tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0 10 5 0 10Z" fill="#9b9eaa"/></marker></defs><rect x="${left}" y="${top}" width="${width}" height="${height}" fill="#f4f3ef"/><rect x="${left}" y="${top}" width="${width}" height="${height}" fill="url(#dots)"/><g font-family="Segoe UI,Arial,sans-serif"><text x="${left + 35}" y="${top + 44}" font-size="22" font-weight="600" fill="#292b35">${xml(b.title)}</text><text x="${left + 35}" y="${top + 66}" font-size="10" fill="#93969e">ASR STORYBOARD · STORY PLAN</text>`];
    const drawNode = n => {
      const color = COLORS[n.color], frame = n.kind === 'chapter';
      pieces.push(`<g transform="translate(${n.x} ${n.y})"><rect width="${n.w}" height="${n.h}" rx="${n.kind === 'note' ? 3 : 11}" fill="${frame ? '#efeee980' : n.kind === 'note' ? color.tint : '#fffefc'}" stroke="${frame ? '#bdbab1' : '#e0dfd8'}" ${frame ? 'stroke-dasharray="7 5"' : ''}/><text x="17" y="28" font-size="9" letter-spacing="1.1" fill="${color.accent}">${xml(KINDS[n.kind].label.toUpperCase())}</text>`);
      const title = wrapped(n.title || 'Untitled', Math.max(15, Math.floor((n.w - 34) / 9)), 2);
      title.forEach((s, i) => pieces.push(`<text x="17" y="${55 + i * 23}" font-size="17" font-weight="600" fill="#292b35">${xml(s)}</text>`));
      const maxLines = Math.max(0, Math.min(5, Math.floor((n.h - 82 - title.length * 23) / 19)));
      wrapped(n.body, Math.max(15, Math.floor((n.w - 34) / 6)), maxLines).forEach((s, i) => pieces.push(`<text x="17" y="${73 + (title.length - 1) * 23 + i * 19}" font-size="11" fill="#898d98">${xml(s)}</text>`));
      if (!frame) {
        pieces.push(`<path d="M0 ${n.h - 35}H${n.w}" stroke="#e8e7e1"/><text x="17" y="${n.h - 14}" font-size="9" fill="#93969e">${xml([n.faction !== 'any' ? n.faction : '', n.level, n.armor ? 'Armor choices' : ''].filter(Boolean).join(' · ').slice(0, Math.floor(n.w / 5)))}</text>`);
      }
      pieces.push('</g>');
    };
    b.nodes.filter(n => n.kind === 'chapter').forEach(drawNode);
    for (const e of b.edges) {
      const a = node(e.from), target = node(e.to); if (!a || !target) continue;
      const path = curve(a, target, e.fromSide, e.toSide);
      pieces.push(`<path d="${path.d}" fill="none" stroke="#9b9eaa" stroke-width="2" marker-end="url(#tip)" ${e.style === 'dashed' ? 'stroke-dasharray="7 6"' : ''}/>`);
      if (e.label) pieces.push(`<text x="${path.x}" y="${path.y - 10}" font-size="11" text-anchor="middle" fill="#777c88" stroke="#f4f3ef" stroke-width="7" paint-order="stroke">${xml(e.label)}</text>`);
    }
    b.nodes.filter(n => n.kind !== 'chapter').forEach(drawNode);
    pieces.push('</g></svg>'); return pieces.join('');
  }
  $('export').addEventListener('click', () => { $('export-menu').hidden = !$('export-menu').hidden; $('export').setAttribute('aria-expanded', String(!$('export-menu').hidden)); });
  document.addEventListener('click', event => { if (!event.target.closest('.export-wrap')) { $('export-menu').hidden = true; $('export').setAttribute('aria-expanded', 'false'); } });
  document.querySelectorAll('[data-export]').forEach(button => button.addEventListener('click', () => {
    saveNow();
    if (button.dataset.export === 'json') download(JSON.stringify({ format: 'asr-storyboard', version: 1, board: board() }, null, 2), 'application/json', 'json');
    else if (button.dataset.export === 'svg') download(svgExport(), 'image/svg+xml', 'svg');
    else download(markdown(), 'text/markdown;charset=utf-8', 'md');
    $('export-menu').hidden = true; $('export').setAttribute('aria-expanded', 'false');
  }));
  $('import').addEventListener('click', () => $('import-file').click());
  $('import-file').addEventListener('change', async event => {
    const file = event.target.files[0]; if (!file) return;
    try {
      if (file.size > 10 * 1024 * 1024) throw new Error('Choose a board file smaller than 10 MB.');
      if (workspace.boards.length >= 100) throw new Error('This workspace has reached its 100-board limit.');
      const raw = JSON.parse(await file.text());
      if (raw.format !== 'asr-storyboard' || raw.version !== 1) throw new Error('Choose an ASR Storyboard JSON export (version 1).');
      const imported = normalizeBoard(raw.board, true);
      saveNow(); cancelGesture(); workspace.boards.push(imported); workspace.active = imported.id;
      selection.clear(); selectedEdge = null; $('search').value = ''; setTool('select'); renderAll(); requestAnimationFrame(() => fit()); save();
      toast('Board imported. Your other boards are still in the workspace.');
    } catch (error) { toast('Could not import: ' + error.message); }
    finally { event.target.value = ''; }
  });
  window.addEventListener('pagehide', saveNow);
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') saveNow(); });
  window.addEventListener('resize', renderView);
  renderAll();
  if (startupMessage) { $('save-state').className = 'error'; $('save-state').replaceChildren(el('i'), document.createTextNode('Export to save a backup')); toast(startupMessage); }
})();
