// 3D force-directed layout (Barnes–Hut octree), off the main thread.
// Messages in : {type:'init', n, links:Uint32Array(2m), linkKinds:Uint8Array(m),
//                weights:Float32Array(n), pos:Float32Array(3n), pinned:Int32Array, alpha}
//               {type:'reheat', alpha} | {type:'pause'} | {type:'resume'}
// Messages out: {type:'tick', pos:Float32Array(3n), alpha}
'use strict';

let n = 0, m = 0;
let pos, vel, links, linkKinds, weights, charge, bias, dist, strength, pinned;
let alpha = 1, alphaMin = 0.004, alphaTarget = 0;
const alphaDecay = 1 - Math.pow(0.001, 1 / 320);
const velocityDecay = 0.62;
const THETA2 = 0.81; // theta = 0.9
let running = false, paused = false, timer = null;

// Rest length per link kind: contains, wiki, uses, touch, spawn, tools
const KIND_DIST = [20, 28, 42, 50, 18, 16];
const KIND_STRENGTH = [0.7, 0.5, 0.18, 0.08, 0.8, 1.0];

self.onmessage = (e) => {
  const d = e.data;
  if (d.type === 'init') {
    n = d.n; m = d.links.length / 2;
    pos = d.pos; links = d.links; linkKinds = d.linkKinds; weights = d.weights;
    pinned = new Uint8Array(n);
    for (const i of d.pinned) pinned[i] = 1;
    vel = new Float32Array(n * 3);
    const deg = new Float32Array(n);
    for (let l = 0; l < m; l++) { deg[links[2 * l]]++; deg[links[2 * l + 1]]++; }
    charge = new Float32Array(n);
    for (let i = 0; i < n; i++) charge[i] = -(9 + weights[i] * 5);
    bias = new Float32Array(m); dist = new Float32Array(m); strength = new Float32Array(m);
    for (let l = 0; l < m; l++) {
      const s = links[2 * l], t = links[2 * l + 1];
      const k = linkKinds[l];
      bias[l] = deg[s] / (deg[s] + deg[t]);
      dist[l] = (KIND_DIST[k] || 28) + (weights[s] + weights[t]) * 1.6;
      strength[l] = (KIND_STRENGTH[k] || 0.5) / Math.min(deg[s], deg[t]);
    }
    alpha = d.alpha;
    start();
  } else if (d.type === 'reheat') {
    alpha = Math.max(alpha, d.alpha); start();
  } else if (d.type === 'pause') {
    paused = true;
  } else if (d.type === 'resume') {
    paused = false; start();
  }
};

function start() {
  if (running || paused || !n) return;
  running = true;
  loop();
}

function loop() {
  if (paused) { running = false; return; }
  const t0 = performance.now();
  // Several ticks per message on small graphs, one on huge ones.
  do { tick(); } while (performance.now() - t0 < 12 && alpha > alphaMin);
  const out = new Float32Array(pos);
  self.postMessage({ type: 'tick', pos: out, alpha }, [out.buffer]);
  if (alpha <= alphaMin) { running = false; return; }
  timer = setTimeout(loop, 16);
}

function tick() {
  alpha += (alphaTarget - alpha) * alphaDecay;
  // Links
  for (let l = 0; l < m; l++) {
    const s = links[2 * l] * 3, t = links[2 * l + 1] * 3;
    let x = pos[t] + vel[t] - pos[s] - vel[s] || 1e-6;
    let y = pos[t + 1] + vel[t + 1] - pos[s + 1] - vel[s + 1] || 1e-6;
    let z = pos[t + 2] + vel[t + 2] - pos[s + 2] - vel[s + 2] || 1e-6;
    let len = Math.sqrt(x * x + y * y + z * z);
    len = (len - dist[l]) / len * alpha * strength[l];
    x *= len; y *= len; z *= len;
    const b = bias[l];
    vel[t] -= x * b; vel[t + 1] -= y * b; vel[t + 2] -= z * b;
    vel[s] += x * (1 - b); vel[s + 1] += y * (1 - b); vel[s + 2] += z * (1 - b);
  }
  // Many-body
  const tree = buildTree();
  for (let i = 0; i < n; i++) applyCharge(tree, i);
  // Gentle gravity keeps islands in the same universe.
  const g = 0.028 * alpha;
  for (let i = 0; i < n; i++) {
    const k = i * 3;
    vel[k] -= pos[k] * g; vel[k + 1] -= pos[k + 1] * g; vel[k + 2] -= pos[k + 2] * g;
  }
  for (let i = 0; i < n; i++) {
    const k = i * 3;
    if (pinned[i]) { vel[k] = vel[k + 1] = vel[k + 2] = 0; continue; }
    vel[k] *= velocityDecay; vel[k + 1] *= velocityDecay; vel[k + 2] *= velocityDecay;
    pos[k] += vel[k]; pos[k + 1] += vel[k + 1]; pos[k + 2] += vel[k + 2];
  }
}

// --- octree -----------------------------------------------------------------
// Flat arrays for speed: each cell has bounds centre/size, children, mass, centroid.
let cap = 0, cx, cy, cz, cs, child, cmass, mx, my, mz, leafIdx, used;

function ensure(sz) {
  if (sz <= cap) return;
  cap = Math.max(sz, cap * 2, 1024);
  cx = new Float64Array(cap); cy = new Float64Array(cap); cz = new Float64Array(cap); cs = new Float64Array(cap);
  child = new Int32Array(cap * 8); cmass = new Float64Array(cap);
  mx = new Float64Array(cap); my = new Float64Array(cap); mz = new Float64Array(cap);
  leafIdx = new Int32Array(cap);
}

function newCell(x, y, z, s) {
  const c = used++;
  cx[c] = x; cy[c] = y; cz[c] = z; cs[c] = s;
  child.fill(-1, c * 8, c * 8 + 8);
  cmass[c] = 0; mx[c] = my[c] = mz[c] = 0; leafIdx[c] = -1;
  return c;
}

function buildTree() {
  ensure(n * 4 + 16);
  used = 0;
  let x0 = Infinity, y0 = Infinity, z0 = Infinity, x1 = -Infinity, y1 = -Infinity, z1 = -Infinity;
  for (let i = 0; i < n; i++) {
    const k = i * 3;
    if (pos[k] < x0) x0 = pos[k]; if (pos[k] > x1) x1 = pos[k];
    if (pos[k + 1] < y0) y0 = pos[k + 1]; if (pos[k + 1] > y1) y1 = pos[k + 1];
    if (pos[k + 2] < z0) z0 = pos[k + 2]; if (pos[k + 2] > z1) z1 = pos[k + 2];
  }
  const size = Math.max(x1 - x0, y1 - y0, z1 - z0, 1) * 1.01;
  const root = newCell((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2, size);
  for (let i = 0; i < n; i++) insert(root, i, 0);
  return root;
}

function insert(c, i, depth) {
  const k = i * 3, q = charge[i];
  // accumulate centroid (weighted by |charge|)
  const w = -q;
  mx[c] = (mx[c] * cmass[c] + pos[k] * w) / (cmass[c] + w);
  my[c] = (my[c] * cmass[c] + pos[k + 1] * w) / (cmass[c] + w);
  mz[c] = (mz[c] * cmass[c] + pos[k + 2] * w) / (cmass[c] + w);
  cmass[c] += w;
  const isLeaf = child[c * 8] === -1 && child[c * 8 + 1] === -1 && child[c * 8 + 2] === -1 && child[c * 8 + 3] === -1 &&
    child[c * 8 + 4] === -1 && child[c * 8 + 5] === -1 && child[c * 8 + 6] === -1 && child[c * 8 + 7] === -1;
  if (isLeaf && leafIdx[c] === -1 && cmass[c] === w) { leafIdx[c] = i; return; }
  if (depth > 40) return; // coincident points: just merge mass
  if (isLeaf && leafIdx[c] !== -1) {
    const j = leafIdx[c]; leafIdx[c] = -1;
    pushDown(c, j, depth);
  }
  pushDown(c, i, depth);
}

function pushDown(c, i, depth) {
  const k = i * 3;
  const o = (pos[k] > cx[c] ? 1 : 0) | (pos[k + 1] > cy[c] ? 2 : 0) | (pos[k + 2] > cz[c] ? 4 : 0);
  let ch = child[c * 8 + o];
  if (ch === -1) {
    const h = cs[c] / 4;
    ch = newCell(cx[c] + (o & 1 ? h : -h), cy[c] + (o & 2 ? h : -h), cz[c] + (o & 4 ? h : -h), cs[c] / 2);
    child[c * 8 + o] = ch;
  }
  insert(ch, i, depth + 1);
}

function applyCharge(c, i) {
  const k = i * 3;
  let dx = mx[c] - pos[k], dy = my[c] - pos[k + 1], dz = mz[c] - pos[k + 2];
  let d2 = dx * dx + dy * dy + dz * dz;
  const leaf = leafIdx[c];
  if (leaf === i) return;
  if (leaf !== -1 || (cs[c] * cs[c]) / (d2 || 1e-9) < THETA2) {
    if (d2 < 1) { dx = (Math.random() - 0.5) * 1e-3 || 1e-3; d2 = 1; }
    if (d2 > 1e8) return;
    const f = -cmass[c] * alpha / d2; // cmass holds |charge| sum, so this repels
    vel[k] += dx * f; vel[k + 1] += dy * f; vel[k + 2] += dz * f;
    return;
  }
  for (let o = 0; o < 8; o++) {
    const ch = child[c * 8 + o];
    if (ch !== -1) applyCharge(ch, i);
  }
}
