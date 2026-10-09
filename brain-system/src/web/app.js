// Global Brain UI. No inline handlers anywhere (the page runs under a strict
// CSP); every value that came from disk or an agent goes through esc().
import { NeuralBrain, KINDS, KIND_ORDER } from '/brain.js';

const $ = (id) => document.getElementById(id);
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

async function api(path, opts = {}) {
  const init = { ...opts };
  if (opts.json !== undefined) {
    init.method = 'POST';
    init.headers = { 'Content-Type': 'application/json' };
    init.body = JSON.stringify(opts.json);
    delete init.json;
  }
  const res = await fetch(path, init);
  let data = null;
  try { data = await res.json(); } catch { /* non-JSON */ }
  if (!res.ok) throw new Error((data && data.error) || `${res.status} ${res.statusText}`);
  return data;
}

function toast(msg, kind = '') {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = msg;
  $('toasts').appendChild(el);
  setTimeout(() => el.remove(), kind === 'err' ? 7000 : 3800);
}

const fmtTime = (ms) => {
  if (!ms) return '';
  const d = new Date(typeof ms === 'string' ? ms : Number(ms));
  if (isNaN(d)) return '';
  const diff = (Date.now() - d) / 1000;
  if (diff < 60) return 'just now';
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
};

const kindColor = (k, x) => {
  if (k === 'note') return ({ entity: '#22d3c5', concept: '#ff5fa2', inbox: '#ff7a59' })[x] || '#8fb3d9';
  if (k === 'entity') return '#22d3c5';
  if (k === 'concept') return '#ff5fa2';
  if (k === 'inbox') return '#ff7a59';
  if (k === 'skillnote') return KINDS.skill.color;
  return (KINDS[k] || KINDS.note).color;
};

// ------------------------------------------------------------------ markdown
// Escape first, then add a small, safe subset of markdown.
function inlineMd(s) {
  let out = esc(s);
  out = out.replace(/`([^`]+)`/g, (_, c) => `<code>${c}</code>`);
  out = out.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  out = out.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>');
  out = out.replace(/!?\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|([^\]]+))?\]\]/g, (_, t, a) =>
    `<a class="wikilink" data-target="${t.trim()}">${a || t}</a>`);
  out = out.replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');
  return out;
}

function renderMd(src) {
  const lines = String(src || '').replace(/\r\n/g, '\n').split('\n');
  let html = '', i = 0;
  if (lines[0] && lines[0].trim() === '---') {
    const end = lines.indexOf('---', 1);
    if (end > 0) { html += `<div class="frontmatter">${esc(lines.slice(1, end).join('\n'))}</div>`; i = end + 1; }
  }
  let list = null, para = [];
  const flushPara = () => { if (para.length) { html += `<p>${inlineMd(para.join(' '))}</p>`; para = []; } };
  const closeList = () => { if (list) { html += `</${list}>`; list = null; } };
  for (; i < lines.length; i++) {
    const line = lines[i];
    const t = line.trim();
    const fence = t.match(/^(```|~~~)/);
    if (fence) {
      flushPara(); closeList();
      const buf = [];
      for (i++; i < lines.length && !lines[i].trim().startsWith(fence[1]); i++) buf.push(lines[i]);
      html += `<pre><code>${esc(buf.join('\n'))}</code></pre>`;
      continue;
    }
    if (!t) { flushPara(); closeList(); continue; }
    let m;
    if ((m = t.match(/^(#{1,6})\s+(.*)$/))) { flushPara(); closeList(); const lv = Math.min(m[1].length, 4); html += `<h${lv}>${inlineMd(m[2])}</h${lv}>`; continue; }
    if (/^(-{3,}|\*{3,})$/.test(t)) { flushPara(); closeList(); html += '<hr>'; continue; }
    if ((m = t.match(/^>\s?(.*)$/))) { flushPara(); closeList(); html += `<blockquote>${inlineMd(m[1])}</blockquote>`; continue; }
    if ((m = t.match(/^[-*+]\s+(?:\[( |x|X)\]\s+)?(.*)$/))) {
      flushPara();
      if (list !== 'ul') { closeList(); html += '<ul>'; list = 'ul'; }
      const box = m[1] !== undefined ? (m[1].trim() ? '☑ ' : '☐ ') : '';
      html += `<li>${box}${inlineMd(m[2])}</li>`; continue;
    }
    if ((m = t.match(/^\d+[.)]\s+(.*)$/))) {
      flushPara();
      if (list !== 'ol') { closeList(); html += '<ol>'; list = 'ol'; }
      html += `<li>${inlineMd(m[1])}</li>`; continue;
    }
    if (t.startsWith('|')) {
      flushPara(); closeList();
      const rows = [];
      for (; i < lines.length && lines[i].trim().startsWith('|'); i++) rows.push(lines[i].trim());
      i--;
      html += '<table>' + rows.filter((r) => !/^\|[\s:|-]+\|$/.test(r)).map((r, ri) => '<tr>' + r.replace(/^\||\|$/g, '').split('|').map((c) => ri === 0 ? `<th>${inlineMd(c.trim())}</th>` : `<td>${inlineMd(c.trim())}</td>`).join('') + '</tr>').join('') + '</table>';
      continue;
    }
    closeList();
    para.push(t);
  }
  flushPara(); closeList();
  return html;
}

// ------------------------------------------------------------------ state
const state = {
  tab: null,
  brain: null,
  graphVersion: -1,
  seq: 0,
  notes: [],
  notesByStem: new Map(),
  wikiFilter: '',
  wikiActivePath: null,
  wikiDoc: null,
  skills: [],
  selected: null,
  detail: null,
  editing: false,
};

// ------------------------------------------------------------------ routing
const ROUTES = { '/': 'brain', '/brain': 'brain', '/wiki': 'wiki', '/librarian': 'librarian', '/skills': 'skills' };
const PATHS = { brain: '/', wiki: '/wiki', librarian: '/librarian', skills: '/skills' };

function switchTab(tab, push = true) {
  if (!PATHS[tab]) tab = 'brain';
  state.tab = tab;
  document.querySelectorAll('.view-panel').forEach((p) => p.classList.toggle('active', p.id === `view-${tab}`));
  document.querySelectorAll('.tab-btn').forEach((b) => b.classList.toggle('active', b.dataset.tab === tab));
  if (push && location.pathname !== PATHS[tab]) history.pushState({ tab }, '', PATHS[tab]);
  if (tab === 'brain') ensureBrain();
  if (tab === 'wiki' && !state.notes.length) loadNotes();
  if (tab === 'librarian') loadProposals();
  if (tab === 'skills') loadSkills();
  if (state.brain) state.brain.resize();
}

$('tabs').addEventListener('click', (e) => {
  const a = e.target.closest('a.tab-btn');
  if (!a || e.ctrlKey || e.metaKey || e.shiftKey) return;
  e.preventDefault();
  switchTab(a.dataset.tab);
});
window.addEventListener('popstate', () => switchTab(ROUTES[location.pathname] || 'brain', false));

// ------------------------------------------------------------------ status
async function refreshStatus() {
  try {
    const s = await api('/api/status');
    $('stat-notes').textContent = `Notes ${s.notes}`;
    $('stat-skills').textContent = `Skills ${s.skills}`;
    $('stat-convs').textContent = `Conversations ${s.conversations}`;
    $('live-dot').className = `pulse-dot ${s.ready ? 'ok' : ''}`;
    const up = s.update || {};
    const targets = [up.core?.available && 'core', up.skills?.available && 'skills'].filter(Boolean);
    const badge = $('update-badge');
    badge.hidden = !targets.length;
    if (targets.length && !badge.classList.contains('armed')) {
      badge.dataset.targets = targets.join(',');
      badge.textContent = `Update available: ${targets.map((t) => t === 'core' ? `Brain.Skills ${up.core.latest}` : `skills ${up.skills.latest}`).join(' · ')}`;
    }
    $('live-dot').title = s.ready ? `Indexer live · graph v${s.graph_version}` : 'Indexing…';
  } catch {
    $('live-dot').className = 'pulse-dot err';
    $('live-dot').title = 'Server unreachable';
  }
}
api('/api/version').then((v) => { $('stat-version').textContent = `v${v.version}`; $('stat-version').title = `Built ${v.release_date}`; }).catch(() => {});

$('update-badge').addEventListener('click', async (e) => {
  const b = e.currentTarget;
  if (!b.classList.contains('armed')) {
    b.classList.add('armed');
    b.textContent = 'Click again to install the update';
    setTimeout(() => { b.classList.remove('armed'); refreshStatus(); }, 5000);
    return;
  }
  b.classList.remove('armed');
  const targets = (b.dataset.targets || '').split(',').filter(Boolean);
  // Skills first: the core update restarts this app.
  for (const t of targets.sort((x) => (x === 'skills' ? -1 : 1))) {
    try { const r = await api('/api/update', { json: { target: t } }); toast(r.message, 'ok'); }
    catch (err) { toast(err.message, 'err'); }
  }
});

// ------------------------------------------------------------------ brain
function ensureBrain() {
  if (state.brain) return;
  let brain;
  try {
    brain = new NeuralBrain($('brain-stage'));
  } catch (err) {
    $('brain-loading-text').textContent = `3D view unavailable: ${err.message}. Your browser may have WebGL disabled.`;
    return;
  }
  state.brain = brain;
  buildLegend();
  brain.on('select', (n) => (n ? openPanel(n.id) : closePanel()));
  brain.on('graph', (s) => {
    $('graph-stats').textContent = `${s.nodes.toLocaleString()} nodes · ${s.links.toLocaleString()} links`;
    updateLegendCounts(s.counts);
  });
  brain.on('fly', (on) => { $('fly-hint').hidden = !on; $('hint').hidden = on; });
  brain.on('flyspeed', (v) => { $('fly-speed').textContent = `(${Math.round(v)} u/s)`; });
  pollBrain(true);
}

async function pollBrain(force = false) {
  try {
    const act = await api(`/api/brain/activity?since=${state.seq}`);
    if (force || act.version !== state.graphVersion) {
      const g = await api('/api/brain/graph');
      if (g.nodes.length) {
        state.graphVersion = g.version;
        state.brain.setGraph(g);
        $('brain-loading').hidden = true;
      } else {
        $('brain-loading').hidden = false;
      }
      if (!g.ready) $('brain-loading-text').textContent = 'Indexing sessions, tools, skills and notes…';
    }
    if (state.seq && act.events.length) {
      for (const ev of act.events) {
        state.brain.firePath(ev.path, 1);
        feed(ev);
      }
      if (state.selected && act.events.some((ev) => ev.path.includes(state.selected)) && !state.editing) refreshPanel();
    }
    state.seq = act.seq;
    state.brain.setLive(act.live);
  } catch (e) {
    $('brain-loading-text').textContent = `Waiting for the brain server… (${e.message})`;
  }
}

function feed(ev) {
  const el = document.createElement('div');
  el.className = 'act';
  const color = { tool: KINDS.tools.color, skill: KINDS.skill.color, subagent: KINDS.subagent.color, prompt: KINDS.conversation.color, touch: KINDS.note.color }[ev.kind] || '#fff';
  el.style.borderLeftColor = color;
  const what = ev.kind === 'prompt' ? 'new prompt' : ev.kind === 'touch' ? `touched ${ev.name.split('/').pop()}` : `${ev.kind} · ${ev.name}`;
  const conv = state.brain.nodes[state.brain.index.get(ev.path.find((p) => p.startsWith('conv:')))];
  el.textContent = `${what}${conv ? '  —  ' + conv.l : ''}`;
  const box = $('activity-feed');
  box.prepend(el);
  while (box.children.length > 7) box.lastChild.remove();
  setTimeout(() => el.remove(), 9000);
}

function buildLegend() {
  const box = $('legend');
  box.innerHTML = KIND_ORDER.map((k) => `
    <div class="legend-item ${state.brain.hidden.has(k) ? 'off' : ''}" data-kind="${k}" title="Click to show/hide">
      <span class="dot" style="color:${KINDS[k].color}"></span>${esc(KINDS[k].label)}<span class="n" data-n="${k}"></span>
    </div>`).join('');
  box.addEventListener('click', (e) => {
    const it = e.target.closest('.legend-item');
    if (!it) return;
    const k = it.dataset.kind;
    const visible = state.brain.hidden.has(k);
    state.brain.setKindVisible(k, visible);
    it.classList.toggle('off', !visible);
  });
}

function updateLegendCounts(counts) {
  document.querySelectorAll('[data-n]').forEach((el) => {
    const k = el.dataset.n;
    el.textContent = counts[k] ? counts[k].toLocaleString() : '';
    el.parentElement.hidden = !counts[k];
  });
}

// search
let resultIdx = -1;
$('brain-search').addEventListener('input', () => renderResults());
$('brain-search').addEventListener('keydown', (e) => {
  const items = [...$('brain-results').querySelectorAll('.result')];
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault();
    resultIdx = Math.max(0, Math.min(items.length - 1, resultIdx + (e.key === 'ArrowDown' ? 1 : -1)));
    items.forEach((el, i) => el.classList.toggle('on', i === resultIdx));
  } else if (e.key === 'Enter') {
    const pick = items[Math.max(resultIdx, 0)];
    if (pick) chooseResult(pick.dataset.id);
  } else if (e.key === 'Escape') {
    $('brain-results').hidden = true; e.target.blur();
  }
});
$('brain-search').addEventListener('blur', () => setTimeout(() => { $('brain-results').hidden = true; }, 150));
$('brain-results').addEventListener('mousedown', (e) => {
  const r = e.target.closest('.result');
  if (r) { e.preventDefault(); chooseResult(r.dataset.id); }
});
function renderResults() {
  const q = $('brain-search').value;
  const res = state.brain ? state.brain.search(q) : [];
  resultIdx = -1;
  $('brain-results').hidden = !res.length;
  $('brain-results').innerHTML = res.map((n) => `
    <div class="result" data-id="${esc(n.id)}"><span class="dot" style="color:${esc(n.c || kindColor(n.k, n.x))}"></span>
    <span class="lbl">${esc(n.l)}</span><span class="muted" style="margin-left:auto;font-size:11px">${esc(n.k)}</span></div>`).join('');
}
function chooseResult(id) {
  $('brain-results').hidden = true;
  $('brain-search').value = '';
  const n = state.brain.nodes[state.brain.index.get(id)];
  if (n && state.brain.hidden.has(n.k)) {
    state.brain.setKindVisible(n.k, true);
    document.querySelector(`.legend-item[data-kind="${n.k}"]`)?.classList.remove('off');
  }
  state.brain.select(id, { focus: true });
}

$('btn-fit').addEventListener('click', () => state.brain?.fitAll(true));
$('btn-fly').addEventListener('click', () => state.brain?.setFly(true));
$('btn-reheat').addEventListener('click', () => state.brain?.reheat());
$('tg-labels').addEventListener('change', (e) => { if (state.brain) state.brain.showLabels = e.target.checked; });
$('tg-bloom').addEventListener('change', (e) => state.brain?.setBloom(e.target.checked));
$('tg-orbit').addEventListener('change', (e) => state.brain?.setAutoRotate(e.target.checked));

document.addEventListener('keydown', (e) => {
  if (state.tab !== 'brain' || !state.brain) return;
  const typing = /INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName);
  if (typing) return;
  if ((e.key === 'f' || e.key === 'F') && !e.repeat) { e.preventDefault(); state.brain.setFly(!state.brain.fly.on); return; }
  if (state.brain.fly.on) return;
  if (e.key === 'Escape') { state.brain.select(null); return; }
  if (e.key === '/') { e.preventDefault(); $('brain-search').focus(); }
});

// ------------------------------------------------------------------ side panel
function closePanel() {
  state.selected = null; state.detail = null; state.editing = false;
  $('side-panel').hidden = true;
}

async function openPanel(id) {
  state.selected = id;
  state.editing = false;
  $('side-panel').hidden = false;
  const n = state.brain.nodes[state.brain.index.get(id)];
  paintHead(n, null);
  $('sp-body').innerHTML = '<div class="muted">Loading…</div>';
  await refreshPanel();
}

function paintHead(n, d) {
  const kind = d?.kind || n?.k || '';
  $('sp-dot').style.color = n ? (n.c || kindColor(n.k, n.x)) : '#fff';
  $('sp-kind').textContent = kind + (n?.x && !['external', 'vault'].includes(n.x) ? ` · ${n.x}` : n?.x === 'external' ? ' · plugin / built-in' : '');
  $('sp-title').textContent = d?.edit?.label || n?.l || d?.title || '';
  $('sp-wiki').hidden = !(d?.files?.length);
}

async function refreshPanel() {
  const id = state.selected;
  if (!id) return;
  let d;
  try { d = await api(`/api/brain/node?id=${encodeURIComponent(id)}`); }
  catch (e) { $('sp-body').innerHTML = `<div class="muted">${esc(e.message)}</div>`; return; }
  if (state.selected !== id) return;
  state.detail = d;
  const n = state.brain.nodes[state.brain.index.get(id)];
  paintHead(n, d);
  $('sp-body').innerHTML = state.editing ? editHtml(d) : viewHtml(d, n);
  bindPanel(d);
}

const chip = (id, label, k, x, extra = '') => `<button class="chip" data-go="${esc(id)}"><span class="dot" style="color:${esc(kindColor(k, x))}"></span><span class="t">${esc(label)}</span>${extra}</button>`;

function viewHtml(d, n) {
  let h = '';
  if (d.edit?.note) h += `<div class="sec"><h3>Your note</h3><div class="entry" style="border-color:var(--cyan)">${esc(d.edit.note)}</div></div>`;
  if (d.description) h += `<div class="sec"><h3>Description</h3><div>${esc(d.description)}</div></div>`;
  if (d.prompts_timeline?.length) {
    const rows = d.prompts_timeline.map((p, i) => (i === 4 && d.prompts_skipped ? `<div class="muted" style="font-size:12px">… ${d.prompts_skipped} more prompts …</div>` : '') +
      `<div class="entry" style="border-color:var(--blue)"><span class="when">${esc(fmtTime(p.ts))}</span>${esc(p.text)}</div>`).join('');
    h += `<div class="sec"><h3>What was asked</h3><div class="entries">${rows}</div></div>`;
  } else if (d.first_prompt) h += `<div class="sec"><h3>First prompt</h3><div class="entry" style="border-color:var(--blue)">${esc(d.first_prompt)}</div></div>`;
  if (d.last_reply) h += `<div class="sec"><h3>Last reply</h3><div class="entry">${esc(d.last_reply)}</div></div>`;
  const meta = Object.entries(d.meta || {}).filter(([, v]) => v !== '' && v !== null);
  if (meta.length) h += `<div class="sec"><h3>Details</h3><dl class="kv">${meta.map(([k, v]) => `<dt>${esc(k)}</dt><dd>${esc(v)}</dd>`).join('')}</dl></div>`;

  if (d.tool_counts?.length) {
    const max = d.tool_counts[0][1];
    h += `<div class="sec"><h3>Tool calls</h3><div class="bars">${d.tool_counts.slice(0, 18).map(([t, c]) => `
      <div class="bar"><span class="name" title="${esc(t)}">${esc(t)}</span><span class="track"><span class="fill" style="width:${(c / max) * 100}%"></span></span><span class="v">${c}</span></div>`).join('')}</div></div>`;
  }
  if (d.skills?.length) h += `<div class="sec"><h3>Skills used</h3><div class="chips">${d.skills.map(([s, c]) => chip(skillNodeId(s), `${s} ×${c}`, 'skill')).join('')}</div></div>`;
  if (d.subagents?.length) h += `<div class="sec"><h3>Sub-agents</h3><div class="chips">${d.subagents.map((s) => chip(s.id, `${s.description || s.type} · ${s.calls} calls`, 'subagent')).join('')}</div></div>`;
  if (d.usage?.length) h += `<div class="sec"><h3>Used in ${d.usage.length} conversation(s)</h3><div class="chips">${d.usage.map((u) => chip(u.id, `${u.title} ×${u.uses}`, 'conversation')).join('')}</div></div>`;
  if (d.conversations?.length) h += `<div class="sec"><h3>Sessions (heaviest first)</h3><div class="chips">${d.conversations.map((c) => chip(c.id, c.weight != null ? `${c.title} · w ${c.weight}` : c.title, 'conversation')).join('')}</div></div>`;
  if (d.files_changed?.length) h += `<div class="sec"><h3>Files changed</h3><div class="bars">${d.files_changed.slice(0, 20).map(([f, c]) => `
      <div class="bar"><span class="name" title="${esc(f)}">${esc(f.split('/').slice(-2).join('/'))}</span><span class="track"><span class="fill" style="width:${(c / d.files_changed[0][1]) * 100}%"></span></span><span class="v">${c}</span></div>`).join('')}</div></div>`;
  if (d.touched?.length) h += `<div class="sec"><h3>Brain files touched</h3><div class="chips">${d.touched.map((t) => { const nn = state.brain.nodes[state.brain.index.get(t.id)]; return chip(t.id, `${nn ? nn.l : t.path.split('/').pop()} ×${t.count}`, nn?.k || 'note', nn?.x); }).join('')}</div></div>`;
  if (d.linked_from?.length) h += `<div class="sec"><h3>Linked from</h3><div class="chips">${d.linked_from.filter((l) => l.id).map((l) => chip(l.id, l.title, 'note')).join('')}</div></div>`;

  // Neighbours from the graph, grouped by kind
  const nb = state.brain.neighbors(d.id);
  if (nb.length) {
    const groups = {};
    nb.forEach((x) => (groups[x.k] ||= []).push(x));
    h += `<div class="sec"><h3>Connections (${nb.length})</h3>${KIND_ORDER.filter((k) => groups[k]).map((k) => `
      <div class="chips" style="margin-bottom:6px">${groups[k].slice(0, 40).map((x) => chip(x.id, x.l, x.k, x.x)).join('')}${groups[k].length > 40 ? `<span class="muted">+${groups[k].length - 40} more</span>` : ''}</div>`).join('')}</div>`;
  }

  if (d.calls?.length) {
    h += `<div class="sec"><h3>Recent calls</h3><div class="calls">${d.calls.slice(0, 25).map((c) => `
      <div class="call"><div class="top"><span class="tool">${esc(c.tool)}</span><span class="when">${esc(fmtTime(c.ts))}${c.conversation ? ' · ' + esc(c.conversation) : ''}</span></div>
      ${c.args ? `<div class="args">${esc(c.args)}</div>` : ''}${c.result ? `<div class="res">→ ${esc(c.result)}</div>` : ''}</div>`).join('')}</div></div>`;
  }
  if (d.files?.length) {
    h += `<div class="sec"><h3>Content</h3>${d.files.length > 1 ? `<div class="file-tabs">${d.files.map((f, i) => `<button class="btn" data-file="${i}">${esc(f.label)}</button>`).join('')}</div>` : ''}
      <div class="preview md" id="sp-preview">${renderMd(d.files[0].content)}</div></div>`;
  }
  h += `<div class="sec"><h3>Entries log${d.entries.length ? ` (${d.entries.length})` : ''}</h3>
    <div class="entries">${d.entries.slice().reverse().map((e) => `<div class="entry"><span class="when">${esc(new Date(e.ts).toLocaleString())}${e.source ? ' · ' + esc(e.source) : ''}</span>${esc(e.text)}</div>`).join('') || '<div class="muted" style="font-size:12px">No entries yet. Sessions revisiting this node can append here.</div>'}</div>
    <div class="form" style="margin-top:8px"><textarea class="input" id="sp-entry" rows="2" placeholder="Append an entry to this node…"></textarea>
    <div class="row"><button class="btn" id="sp-entry-add">Add entry</button></div></div></div>`;
  return h;
}

function editHtml(d) {
  const e = d.edit || {};
  const color = e.color || state.brain.nodes[state.brain.index.get(d.id)]?.c || '';
  let h = `<div class="sec"><h3>Node appearance</h3><div class="form">
    <label>Label<input class="input" id="ed-label" value="${esc(e.label || '')}" placeholder="${esc(d.title)}"></label>
    <label>Colour<div class="row"><input type="color" id="ed-color" value="${esc(/^#[0-9a-f]{6}$/i.test(color) ? color : '#5ff3ff')}">
      <label class="row" style="display:flex"><input type="checkbox" id="ed-color-on" ${e.color ? 'checked' : ''}> custom colour</label></div></label>
    <label>Note<textarea class="input" id="ed-note" rows="4" placeholder="Anything you want to remember about this node">${esc(e.note || '')}</textarea></label>
    <div class="row"><button class="btn primary" id="ed-save">Save node</button><button class="btn" id="ed-reset">Reset</button></div></div></div>`;
  (d.files || []).forEach((f, i) => {
    h += `<div class="sec"><h3>Edit file · ${esc(f.label)}</h3><div class="form">
      <div class="muted mono" style="font-size:11px;word-break:break-all">${esc(f.path)}</div>
      <textarea class="input" id="ed-file-${i}" rows="16">${esc(f.content)}</textarea>
      <div class="row"><button class="btn primary" data-save-file="${i}">Save file</button></div></div></div>`;
  });
  h += `<div class="row" style="margin-top:14px"><button class="btn" id="ed-done">Done editing</button></div>`;
  return h;
}

const skillNodeId = (name) => {
  const id = `skill:${String(name).toLowerCase()}`;
  return state.brain?.index.has(id) ? id : id;
};

function bindPanel(d) {
  const body = $('sp-body');
  body.onclick = (e) => {
    const go = e.target.closest('[data-go]');
    if (go) { state.brain.select(go.dataset.go, { focus: true }); return; }
    const ft = e.target.closest('[data-file]');
    if (ft) { $('sp-preview').innerHTML = renderMd(d.files[+ft.dataset.file].content); return; }
    const wl = e.target.closest('a.wikilink');
    if (wl) { e.preventDefault(); openWikiTarget(wl.dataset.target); return; }
    const sf = e.target.closest('[data-save-file]');
    if (sf) saveFile(d, +sf.dataset.saveFile, sf);
  };
  $('sp-entry-add')?.addEventListener('click', async () => {
    const text = $('sp-entry').value.trim();
    if (!text) return;
    try { await api('/api/brain/entry', { json: { id: d.id, text } }); toast('Entry added', 'ok'); refreshPanel(); }
    catch (err) { toast(err.message, 'err'); }
  });
  $('ed-save')?.addEventListener('click', async () => {
    try {
      await api('/api/brain/overlay', { json: { id: d.id, label: $('ed-label').value.trim(), color: $('ed-color-on').checked ? $('ed-color').value : '', note: $('ed-note').value } });
      toast('Node saved', 'ok');
      await pollBrain(true);
      refreshPanel();
    } catch (err) { toast(err.message, 'err'); }
  });
  $('ed-reset')?.addEventListener('click', async () => {
    try { await api('/api/brain/overlay', { json: { id: d.id, label: '', color: '', note: '' } }); toast('Node reset', 'ok'); await pollBrain(true); refreshPanel(); }
    catch (err) { toast(err.message, 'err'); }
  });
  $('ed-done')?.addEventListener('click', () => { state.editing = false; refreshPanel(); });
}

async function saveFile(d, i, btn) {
  const f = d.files[i];
  btn.disabled = true;
  try {
    const r = await api('/api/brain/file', { json: { path: f.path, content: $(`ed-file-${i}`).value, hash: f.hash } });
    f.hash = r.hash; f.content = $(`ed-file-${i}`).value;
    toast(`Saved ${f.label}`, 'ok');
    pollBrain(true);
  } catch (err) { toast(err.message, 'err'); }
  btn.disabled = false;
}

$('sp-close').addEventListener('click', () => state.brain.select(null));
$('sp-focus').addEventListener('click', () => state.selected && state.brain.focus(state.selected));
$('sp-edit').addEventListener('click', () => { state.editing = !state.editing; refreshPanel(); });
$('sp-wiki').addEventListener('click', () => {
  const f = state.detail?.files?.[0];
  if (f) { switchTab('wiki'); openWikiPath(f.path); }
});

// ------------------------------------------------------------------ wiki
async function loadNotes() {
  try {
    state.notes = await api('/api/notes');
    state.notesByStem = new Map();
    state.notes.forEach((n) => {
      const stem = n.rel.split('/').pop().replace(/\.md$/i, '').toLowerCase();
      if (!state.notesByStem.has(stem)) state.notesByStem.set(stem, n);
      if (!state.notesByStem.has(n.title.toLowerCase())) state.notesByStem.set(n.title.toLowerCase(), n);
    });
    renderNoteList();
  } catch (e) { $('note-list').innerHTML = `<div class="muted" style="padding:12px">${esc(e.message)}</div>`; }
}

function renderNoteList() {
  const q = state.wikiFilter.toLowerCase();
  const list = state.notes.filter((n) => !q || n.title.toLowerCase().includes(q) || n.rel.toLowerCase().includes(q) || (n.summary || '').toLowerCase().includes(q));
  $('note-list').innerHTML = list.map((n) => `
    <div class="note-item ${n.path === state.wikiActivePath ? 'active' : ''}" data-path="${esc(n.path)}">
      <div class="note-item-title"><span class="dot" style="color:${kindColor(n.type)}"></span>${esc(n.title)}</div>
      <div class="note-item-meta">${esc(n.type)} · ${esc(fmtTime(n.updated))} · ${esc(n.rel)}</div>
    </div>`).join('') || '<div class="muted" style="padding:12px">No notes match.</div>';
}

$('wiki-search').addEventListener('input', (e) => { state.wikiFilter = e.target.value; renderNoteList(); });
$('note-list').addEventListener('click', (e) => {
  const it = e.target.closest('.note-item');
  if (it) openWikiPath(it.dataset.path);
});
$('wiki-body').addEventListener('click', (e) => {
  const wl = e.target.closest('a.wikilink');
  if (wl) { e.preventDefault(); openWikiTarget(wl.dataset.target); }
});

function openWikiTarget(target) {
  const key = target.split('/').pop().replace(/\.md$/i, '').toLowerCase();
  const n = state.notesByStem.get(key);
  if (n) { switchTab('wiki'); openWikiPath(n.path); return; }
  const skill = state.skills.find((s) => s.name.toLowerCase() === key);
  if (skill) { switchTab('wiki'); openWikiPath(skill.path); return; }
  toast(`No note named "${target}" yet — it's an unresolved link.`);
}

async function openWikiPath(path) {
  if (!state.notes.length) await loadNotes();
  state.wikiActivePath = path;
  state.wikiEditing = false;
  renderNoteList();
  try {
    const doc = await api(`/api/note?path=${encodeURIComponent(path)}`);
    state.wikiDoc = doc;
    paintWiki();
  } catch (e) {
    $('wiki-title').textContent = 'Could not open note';
    $('wiki-body').innerHTML = `<p class="muted">${esc(e.message)}</p>`;
    $('wiki-actions').hidden = true;
  }
}

function paintWiki() {
  const doc = state.wikiDoc;
  $('wiki-title').textContent = doc.title;
  $('wiki-meta').textContent = `${doc.type} · updated ${fmtTime(doc.updated)} · ${doc.path}`;
  $('wiki-actions').hidden = false;
  $('wiki-edit').textContent = state.wikiEditing ? 'Cancel' : 'Edit';
  if (state.wikiEditing) {
    $('wiki-body').innerHTML = `<div class="form"><textarea class="input" id="wiki-editor" rows="28">${esc(doc.content)}</textarea>
      <div class="row"><button class="btn primary" id="wiki-save">Save</button></div></div>`;
    $('wiki-save').addEventListener('click', async () => {
      try {
        const r = await api('/api/brain/file', { json: { path: doc.path, content: $('wiki-editor').value, hash: doc.hash } });
        doc.content = $('wiki-editor').value; doc.hash = r.hash;
        state.wikiEditing = false; paintWiki(); toast('Saved', 'ok'); loadNotes();
      } catch (err) { toast(err.message, 'err'); }
    });
  } else {
    $('wiki-body').innerHTML = renderMd(doc.content);
    $('wiki-body').querySelectorAll('a.wikilink').forEach((a) => {
      const key = a.dataset.target.split('/').pop().toLowerCase();
      if (!state.notesByStem.has(key) && !state.skills.some((s) => s.name.toLowerCase() === key)) a.classList.add('missing');
    });
  }
}

$('wiki-edit').addEventListener('click', () => { state.wikiEditing = !state.wikiEditing; paintWiki(); });
$('wiki-brain').addEventListener('click', () => {
  const doc = state.wikiDoc;
  if (!doc) return;
  const skill = state.skills.find((s) => s.path === doc.path);
  const id = doc.id || (skill ? skill.node_id : null);
  switchTab('brain');
  const go = () => {
    if (!state.brain || !state.brain.index.size) return setTimeout(go, 300);
    const candidates = [id, skill?.node_id, doc.id && `skill:${doc.title.toLowerCase()}`].filter(Boolean);
    const hit = candidates.find((c) => state.brain.index.has(c));
    if (hit) state.brain.select(hit, { focus: true }); else toast('This note is not in the graph.');
  };
  go();
});

// ------------------------------------------------------------------ librarian
async function loadProposals() {
  try {
    const r = await api('/api/proposals');
    renderProposals(r.pending, r.last_sweep);
  } catch (e) { $('proposal-list').innerHTML = `<div class="empty">${esc(e.message)}</div>`; }
}

function renderProposals(list, last) {
  $('proposal-count').hidden = !list.length;
  $('proposal-count').textContent = list.length;
  $('sweep-info').textContent = last
    ? `Last sweep ${fmtTime(last.ran_at)} · ${last.checked} Gold notes checked${last.baselined ? ` · ${last.baselined} newly baselined` : ''} · ${last.new_drift} drift and ${last.new_links} link proposals created.`
    : 'No sweep has run since the brain started. The first automatic sweep runs ~20 s after launch, then every 10 minutes.';
  if (!list.length) {
    $('proposal-list').innerHTML = `<div class="empty">No pending proposals.${last ? ` Every Gold note matched its verified version at the last sweep.` : ''}</div>`;
    return;
  }
  $('proposal-list').innerHTML = list.map((p) => {
    const drift = p.kind === 'drift';
    const link = p.kind === 'link';
    return `<div class="proposal" data-id="${esc(p.proposal_id)}">
      <div class="ph"><span class="tag ${drift ? 'drift' : 'link'}">${esc(p.kind || 'edit')}</span><h3>${esc(p.title)}</h3><span class="muted" style="margin-left:auto;font-size:11px">${esc(fmtTime(p.created_at))}</span></div>
      <div class="path">${esc(p.entry_id)}</div>
      <div>${esc(p.reason)}</div>
      <div class="muted" style="font-size:12px;margin-top:2px">${esc(p.evidence_summary)}</div>
      <div class="diff">
        <div><div class="cap">${drift ? 'Removed since verified' : 'Current'}</div><pre class="old">${esc(p.current_text || '—')}</pre></div>
        <div><div class="cap">${drift ? 'Added since verified' : 'Proposed'}</div><pre class="new">${esc(p.suggested_edit || '—')}</pre></div>
      </div>
      <div class="acts">
        <button class="btn good" data-act="approve">${drift ? 'Accept new version' : link ? 'Apply link fix' : 'Approve & write'}</button>
        <button class="btn bad" data-act="reject">${drift ? 'Reject & restore verified' : 'Dismiss'}</button>
        <button class="btn" data-open="${esc(p.entry_id)}">Open note</button>
      </div></div>`;
  }).join('');
}

$('proposal-list').addEventListener('click', async (e) => {
  const open = e.target.closest('[data-open]');
  if (open) {
    if (!state.notes.length) await loadNotes();
    const n = state.notes.find((x) => x.rel === open.dataset.open);
    if (n) { switchTab('wiki'); openWikiPath(n.path); } else toast('Note not found.', 'err');
    return;
  }
  const btn = e.target.closest('[data-act]');
  if (!btn) return;
  const card = btn.closest('.proposal');
  const action = btn.dataset.act;
  // Restoring a drifted note rewrites it: require a second click.
  if (action === 'reject' && card.querySelector('.tag.drift') && !btn.classList.contains('armed')) {
    btn.classList.add('armed');
    btn.textContent = 'Click again to restore';
    setTimeout(() => { btn.classList.remove('armed'); btn.textContent = 'Reject & restore verified'; }, 4000);
    return;
  }
  card.querySelectorAll('button').forEach((b) => { b.disabled = true; });
  try {
    const r = await api('/api/proposals/resolve', { json: { proposal_id: card.dataset.id, action } });
    toast(r.message, 'ok');
  } catch (err) { toast(err.message, 'err'); }
  loadProposals();
});

$('btn-sweep').addEventListener('click', async (ev) => {
  const btn = ev.currentTarget;
  btn.disabled = true; btn.textContent = 'Sweeping…';
  try {
    const r = await api('/api/verify', { json: {} });
    const rep = r.report;
    toast(`Checked ${rep.checked} Gold notes · ${rep.new_drift + rep.new_links} new proposal(s)`, 'ok');
    renderProposals(rep.pending, rep);
  } catch (err) { toast(err.message, 'err'); }
  btn.disabled = false; btn.textContent = 'Run verification sweep';
});

// ------------------------------------------------------------------ skills
async function loadSkills() {
  try {
    state.skills = await api('/api/skills');
    $('skills-sub').textContent = `${state.skills.length} skills in the library · ${state.skills.filter((s) => s.uses).length} used by agents so far.`;
    renderSkills();
  } catch (e) { $('skill-grid').innerHTML = `<div class="empty">${esc(e.message)}</div>`; }
}

function renderSkills() {
  const q = $('skills-search').value.trim().toLowerCase();
  const list = state.skills
    .filter((s) => !q || s.name.toLowerCase().includes(q) || (s.description || '').toLowerCase().includes(q) || (s.category || '').toLowerCase().includes(q))
    .sort((a, b) => b.uses - a.uses || a.name.localeCompare(b.name));
  $('skill-grid').innerHTML = list.map((s) => `
    <div class="skill-card" data-path="${esc(s.path)}" data-node="${esc(s.node_id)}">
      <h3>${esc(s.name)}${s.uses ? `<span class="uses">used ×${s.uses}</span>` : ''}</h3>
      <div class="cat">${esc(s.rel)}</div>
      <p>${esc(s.description || 'No description.')}</p>
      <div class="acts"><button class="btn" data-open>Open SKILL.md</button><button class="btn" data-brain>Show in brain</button></div>
    </div>`).join('') || '<div class="empty">No skills match.</div>';
}

$('skills-search').addEventListener('input', renderSkills);
$('skill-grid').addEventListener('click', (e) => {
  const card = e.target.closest('.skill-card');
  if (!card) return;
  if (e.target.closest('[data-brain]')) {
    switchTab('brain');
    const go = () => {
      if (!state.brain || !state.brain.index.size) return setTimeout(go, 300);
      if (state.brain.index.has(card.dataset.node)) state.brain.select(card.dataset.node, { focus: true });
      else toast('Skill not in the graph yet.');
    };
    go();
  } else {
    switchTab('wiki');
    openWikiPath(card.dataset.path);
  }
});

// ------------------------------------------------------------------ boot
switchTab(ROUTES[location.pathname] || 'brain', false);
refreshStatus();
setInterval(refreshStatus, 10000);
setInterval(() => { if (state.brain && !document.hidden) pollBrain(); }, 1500);
setInterval(() => { if (state.tab === 'librarian' && !document.hidden) loadProposals(); }, 30000);
api('/api/proposals').then((r) => { $('proposal-count').hidden = !r.pending.length; $('proposal-count').textContent = r.pending.length; }).catch(() => {});
api('/api/skills').then((s) => { state.skills = s; }).catch(() => {});
