// Neural Brain — 3D graph renderer.
// Nodes are one THREE.Points draw call (glowing sprites with pulse rings),
// edges one THREE.LineSegments with a shader that flows light along active
// paths, plus a pool of spark particles riding active edges. Layout runs in a
// worker; positions persist per node so the brain keeps its shape.

import * as THREE from '/vendor/three.module.min.js';
import { OrbitControls } from '/vendor/jsm/controls/OrbitControls.js';
import { EffectComposer } from '/vendor/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from '/vendor/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from '/vendor/jsm/postprocessing/UnrealBloomPass.js';
import { OutputPass } from '/vendor/jsm/postprocessing/OutputPass.js';

export const KINDS = {
  brain:        { color: '#eafcff', label: 'Global brain' },
  project:      { color: '#ff9f43', label: 'Projects' },
  conversation: { color: '#3d8bff', label: 'Conversations' },
  session:      { color: '#ffffff', label: 'Sessions (vault logs)' },
  tools:        { color: '#2dff88', label: 'Tools (per project)' },
  skill:        { color: '#ffd23f', label: 'Skills' },
  subagent:     { color: '#b36bff', label: 'Sub-agents' },
  note:         { color: '#22d3c5', label: 'Notes' },
  ghost:        { color: '#5b6475', label: 'Unresolved links' },
  category:     { color: '#c99a1e', label: 'Skill categories' },
  folder:       { color: '#1f8f84', label: 'Vault folders' },
  hub:          { color: '#2fb8a8', label: 'Hubs' },
};
// Note sub-types get their own shade.
const NOTE_SHADES = { entity: '#22d3c5', concept: '#ff5fa2', inbox: '#ff7a59', note: '#8fb3d9' };
export const KIND_ORDER = ['brain', 'project', 'conversation', 'session', 'tools', 'skill', 'subagent', 'note', 'ghost', 'category', 'folder', 'hub'];

// Visual size per kind (world units at weight 1).
const KIND_SIZE = { brain: 26, project: 13, conversation: 9.5, session: 9, tools: 8, skill: 8, subagent: 7, note: 7, ghost: 4.5, category: 9, folder: 9.5, hub: 14 };

const POS_KEY = 'brain.positions.v1';
const SPARKS = 1600;

function hashSeed(str) {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619); }
  return ((h >>> 0) % 10000) / 10000;
}

export class NeuralBrain {
  constructor(container) {
    this.container = container;
    this.listeners = {};
    this.nodes = []; this.index = new Map(); this.adj = [];
    this.edgeIndex = new Map();
    this.hidden = new Set(['ghost']);
    this.selected = -1; this.hovered = -1;
    this.pulse = null; this.edgeActive = null;
    this.live = new Set();
    this.layoutAlpha = 0;
    this.showLabels = true;
    this.bloomOn = true;
    this.fly = { on: false, keys: new Set(), speed: 60, yaw: 0, pitch: 0 };
    this.tween = null;
    this.sparkQueue = [];
    this.clock = new THREE.Clock();
    this.lastInteract = performance.now();
    this.savedPos = this.loadPositions();
    this.initThree();
    this.initInput();
    this.worker = new Worker('/layout-worker.js');
    this.worker.onmessage = (e) => this.onLayout(e.data);
    this.animate = this.animate.bind(this);
    window.__brain = this; // handy from the devtools console
    requestAnimationFrame(this.animate);
    setInterval(() => this.savePositions(), 8000);
    window.addEventListener('beforeunload', () => this.savePositions());
  }

  on(ev, fn) { (this.listeners[ev] ||= []).push(fn); }
  emit(ev, ...a) { (this.listeners[ev] || []).forEach((f) => f(...a)); }

  // ---------------------------------------------------------------- setup
  initThree() {
    const w = this.container.clientWidth || 800, h = this.container.clientHeight || 600;
    this.renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.setSize(w, h);
    this.renderer.setClearColor(0x03050b, 1);
    this.container.appendChild(this.renderer.domElement);
    this.canvas = this.renderer.domElement;
    this.canvas.tabIndex = 0;

    this.scene = new THREE.Scene();
    // Background via the scene (colour-managed). A renderer clear colour gets
    // sRGB-encoded twice by the post-processing chain and turns grey.
    this.scene.background = new THREE.Color(0x03050b);
    this.camera = new THREE.PerspectiveCamera(58, w / h, 0.5, 400000);
    this.camera.position.set(0, 160, 620);

    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.zoomToCursor = true;
    this.controls.zoomSpeed = 1.6;
    this.controls.rotateSpeed = 0.55;
    this.controls.panSpeed = 0.9;
    this.controls.screenSpacePanning = true;
    this.controls.minDistance = 4;
    this.controls.maxDistance = 120000;
    this.controls.autoRotate = true;
    this.controls.autoRotateSpeed = 0.25;
    this.controls.addEventListener('start', () => { this.lastInteract = performance.now(); this.controls.autoRotate = false; this.tween = null; });

    this.composer = new EffectComposer(this.renderer);
    this.composer.addPass(new RenderPass(this.scene, this.camera));
    this.bloom = new UnrealBloomPass(new THREE.Vector2(w, h), 0.95, 0.42, 0.22);
    this.composer.addPass(this.bloom);
    this.composer.addPass(new OutputPass());

    this.uniforms = { uTime: { value: 0 }, uScale: { value: 1 }, uGlow: { value: 1 }, uEdgeGain: { value: 1 } };
    this.addStarfield();

    // Edges
    this.edgeMat = new THREE.ShaderMaterial({
      uniforms: { uTime: this.uniforms.uTime, uEdgeGain: this.uniforms.uEdgeGain },
      vertexShader: `
        attribute vec3 color; attribute float tpos; attribute float aFlow; attribute float hl;
        varying vec3 vColor; varying float vT; varying float vActive; varying float vHl;
        void main() {
          vColor = color; vT = tpos; vActive = aFlow; vHl = hl;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }`,
      fragmentShader: `
        uniform float uTime; uniform float uEdgeGain;
        varying vec3 vColor; varying float vT; varying float vActive; varying float vHl;
        void main() {
          if (vHl < 0.001) discard;
          float flow = 0.0;
          if (vActive > 0.01) {
            float x = fract(vT * 3.0 - uTime * 1.3);
            flow = (smoothstep(0.80, 0.98, x) + 0.25) * vActive;
          }
          float a = 0.42 * min(vHl, 1.0) * uEdgeGain + 0.2 * max(vHl - 1.0, 0.0) + flow * 0.95;
          gl_FragColor = vec4(vColor * (1.0 + flow * 2.4), a);
        }`,
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    });
    this.edgeGeo = new THREE.BufferGeometry();
    this.edges = new THREE.LineSegments(this.edgeGeo, this.edgeMat);
    this.edges.frustumCulled = false;
    this.scene.add(this.edges);

    // Nodes
    this.nodeMat = new THREE.ShaderMaterial({
      uniforms: this.uniforms,
      vertexShader: `
        attribute float size; attribute vec3 color; attribute float pulse; attribute float hl; attribute float seed;
        uniform float uTime; uniform float uScale;
        varying vec3 vColor; varying float vPulse; varying float vHl; varying float vSeed;
        void main() {
          vColor = color; vPulse = pulse; vHl = hl; vSeed = seed;
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          float breathe = 1.0 + 0.07 * sin(uTime * 1.7 + seed * 6.2831);
          float emph = hl > 1.5 ? 1.35 : (hl < 0.5 ? 0.75 : 1.0);
          float s = size * (1.0 + pulse * 0.9) * breathe * emph;
          gl_PointSize = hl < 0.001 ? 0.0 : clamp(s * uScale / -mv.z, 2.2, hl < 0.5 ? 34.0 : 120.0);
          gl_Position = projectionMatrix * mv;
        }`,
      fragmentShader: `
        uniform float uTime; uniform float uGlow;
        varying vec3 vColor; varying float vPulse; varying float vHl; varying float vSeed;
        void main() {
          vec2 c = gl_PointCoord * 2.0 - 1.0;
          float d = length(c);
          if (d > 1.0) discard;
          float core = smoothstep(0.50, 0.40, d);
          float rim  = smoothstep(0.53, 0.47, d) - core;
          float glow = pow(max(0.0, 1.0 - d), 2.4) * 0.6 * uGlow;
          float ring = 0.0;
          if (vPulse > 0.01) {
            float r = fract(uTime * 0.85 + vSeed);
            ring = smoothstep(0.06, 0.0, abs(d - (0.5 + r * 0.48))) * (1.0 - r) * vPulse * 1.6;
          }
          float dim = vHl < 0.5 ? 0.22 : 1.0;
          vec3 col = vColor * (core * 0.95 + glow * 0.8 + ring) + vec3(1.0) * (core * 0.10 + rim * 0.30);
          float a = clamp(core + rim + glow + ring, 0.0, 1.0) * dim;
          gl_FragColor = vec4(col * dim, a);
        }`,
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    });
    this.nodeGeo = new THREE.BufferGeometry();
    this.points = new THREE.Points(this.nodeGeo, this.nodeMat);
    this.points.frustumCulled = false;
    this.scene.add(this.points);

    // Sparks riding active edges
    this.sparkGeo = new THREE.BufferGeometry();
    this.sparkPos = new Float32Array(SPARKS * 3);
    this.sparkCol = new Float32Array(SPARKS * 3);
    this.sparkAlpha = new Float32Array(SPARKS);
    this.sparkGeo.setAttribute('position', new THREE.BufferAttribute(this.sparkPos, 3).setUsage(THREE.DynamicDrawUsage));
    this.sparkGeo.setAttribute('color', new THREE.BufferAttribute(this.sparkCol, 3).setUsage(THREE.DynamicDrawUsage));
    this.sparkGeo.setAttribute('alpha', new THREE.BufferAttribute(this.sparkAlpha, 1).setUsage(THREE.DynamicDrawUsage));
    this.sparkMat = new THREE.ShaderMaterial({
      uniforms: { uScale: this.uniforms.uScale },
      vertexShader: `
        attribute vec3 color; attribute float alpha; uniform float uScale;
        varying vec3 vColor; varying float vA;
        void main() {
          vColor = color; vA = alpha;
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          gl_PointSize = alpha < 0.01 ? 0.0 : clamp(2.2 * uScale / -mv.z, 2.0, 40.0);
          gl_Position = projectionMatrix * mv;
        }`,
      fragmentShader: `
        varying vec3 vColor; varying float vA;
        void main() {
          float d = length(gl_PointCoord * 2.0 - 1.0);
          if (d > 1.0) discard;
          float g = pow(1.0 - d, 2.0);
          gl_FragColor = vec4((vColor + 0.5) * g * vA * 1.6, g * vA);
        }`,
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    });
    this.sparkPoints = new THREE.Points(this.sparkGeo, this.sparkMat);
    this.sparkPoints.frustumCulled = false;
    this.scene.add(this.sparkPoints);
    this.sparks = Array.from({ length: SPARKS }, () => ({ e: -1, t: 0, v: 0, rev: false, life: 0 }));
    this.sparkCursor = 0;

    // Labels
    this.labelLayer = document.createElement('div');
    this.labelLayer.className = 'brain-labels';
    this.container.appendChild(this.labelLayer);
    this.labelPool = [];
    for (let i = 0; i < 90; i++) {
      const el = document.createElement('div');
      el.className = 'brain-label';
      el.style.display = 'none';
      this.labelLayer.appendChild(el);
      this.labelPool.push(el);
    }

    new ResizeObserver(() => this.resize()).observe(this.container);
    this.resize();
  }

  addStarfield() {
    const count = 4000, r = 150000;
    const p = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      const u = Math.random() * 2 - 1, th = Math.random() * Math.PI * 2, rr = r * (0.6 + Math.random() * 0.4);
      const s = Math.sqrt(1 - u * u);
      p[i * 3] = rr * s * Math.cos(th); p[i * 3 + 1] = rr * u; p[i * 3 + 2] = rr * s * Math.sin(th);
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(p, 3));
    this.stars = new THREE.Points(g, new THREE.PointsMaterial({ color: 0x56688f, size: 1.2, sizeAttenuation: false, transparent: true, opacity: 0.32, depthWrite: false }));
    this.scene.add(this.stars);
  }

  resize() {
    const w = this.container.clientWidth, h = this.container.clientHeight;
    if (!w || !h) return;
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(w, h);
    this.composer.setSize(w, h);
    this.bloom.setSize(w, h);
    const pr = this.renderer.getPixelRatio();
    this.uniforms.uScale.value = (h * pr) / (2 * Math.tan(THREE.MathUtils.degToRad(this.camera.fov) / 2));
  }

  // ---------------------------------------------------------------- data
  colorFor(n) {
    if (n.c) return n.c;
    if (n.k === 'note') return NOTE_SHADES[n.x] || NOTE_SHADES.note;
    if (n.k === 'skill' && n.x === 'external') return '#d9b84a';
    return (KINDS[n.k] || KINDS.note).color;
  }

  /** Replace or merge the graph. Existing nodes keep their position. */
  setGraph(graph) {
    const old = new Map();
    if (this.posArr) this.nodes.forEach((nd, i) => old.set(nd.id, [this.posArr[i * 3], this.posArr[i * 3 + 1], this.posArr[i * 3 + 2]]));
    const selectedId = this.selected >= 0 ? this.nodes[this.selected].id : null;
    const prevPulse = new Map();
    if (this.pulse) this.nodes.forEach((nd, i) => { if (this.pulse[i] > 0.01) prevPulse.set(nd.id, this.pulse[i]); });

    this.nodes = graph.nodes;
    this.links = graph.links;
    const n = this.nodes.length, m = this.links.length;
    this.index = new Map(this.nodes.map((nd, i) => [nd.id, i]));
    this.adj = Array.from({ length: n }, () => []);
    this.edgeIndex = new Map();
    this.links.forEach(([a, b], e) => {
      this.adj[a].push(b); this.adj[b].push(a);
      this.edgeIndex.set(a < b ? a * 1e7 + b : b * 1e7 + a, e);
    });

    // Positions: previous run > saved > near a placed neighbour > random shell.
    const pos = new Float32Array(n * 3);
    const placed = new Uint8Array(n);
    let known = 0;
    for (let i = 0; i < n; i++) {
      const id = this.nodes[i].id;
      const p = old.get(id) || this.savedPos[id];
      if (p) { pos.set(p, i * 3); placed[i] = 1; known++; }
    }
    const brain = this.index.get('brain');
    if (brain !== undefined) { pos.set([0, 0, 0], brain * 3); placed[brain] = 1; }
    const queue = []; for (let i = 0; i < n; i++) if (placed[i]) queue.push(i);
    const spread = 18;
    while (queue.length) {
      const i = queue.shift();
      for (const j of this.adj[i]) {
        if (placed[j]) continue;
        placed[j] = 1;
        const s = hashSeed(this.nodes[j].id) * 6.283, t = hashSeed(this.nodes[j].id + '#') * 2 - 1, r = spread * (1 + Math.random());
        const q = Math.sqrt(1 - t * t);
        pos[j * 3] = pos[i * 3] + r * q * Math.cos(s);
        pos[j * 3 + 1] = pos[i * 3 + 1] + r * t;
        pos[j * 3 + 2] = pos[i * 3 + 2] + r * q * Math.sin(s);
        queue.push(j);
      }
    }
    const R = Math.cbrt(n) * 40;
    for (let i = 0; i < n; i++) if (!placed[i]) {
      pos[i * 3] = (Math.random() - 0.5) * R; pos[i * 3 + 1] = (Math.random() - 0.5) * R; pos[i * 3 + 2] = (Math.random() - 0.5) * R;
    }
    this.posArr = pos;

    // Node attributes
    this.colors = new Float32Array(n * 3);
    this.sizes = new Float32Array(n);
    this.pulse = new Float32Array(n);
    this.hl = new Float32Array(n);
    const seeds = new Float32Array(n);
    const col = new THREE.Color();
    this.nodes.forEach((nd, i) => {
      col.set(this.colorFor(nd));
      // Recency (r: 1 = active now) fades old sessions toward the background.
      if (nd.r != null && !nd.c) col.multiplyScalar(0.26 + 0.74 * nd.r);
      this.colors.set([col.r, col.g, col.b], i * 3);
      this.sizes[i] = (KIND_SIZE[nd.k] || 2.5) * (0.65 + Math.sqrt(nd.w || 1) * 0.42);
      seeds[i] = hashSeed(nd.id);
      if (prevPulse.has(nd.id)) this.pulse[i] = prevPulse.get(nd.id);
    });
    const g = new THREE.BufferGeometry();
    this.posAttr = new THREE.BufferAttribute(this.posArr, 3).setUsage(THREE.DynamicDrawUsage);
    g.setAttribute('position', this.posAttr);
    g.setAttribute('color', new THREE.BufferAttribute(this.colors, 3));
    g.setAttribute('size', new THREE.BufferAttribute(this.sizes, 1));
    this.pulseAttr = new THREE.BufferAttribute(this.pulse, 1).setUsage(THREE.DynamicDrawUsage);
    g.setAttribute('pulse', this.pulseAttr);
    this.hlAttr = new THREE.BufferAttribute(this.hl, 1).setUsage(THREE.DynamicDrawUsage);
    g.setAttribute('hl', this.hlAttr);
    g.setAttribute('seed', new THREE.BufferAttribute(seeds, 1));
    this.points.geometry.dispose();
    this.points.geometry = g;
    this.nodeGeo = g;

    // Edge attributes (2 vertices per edge)
    this.edgePos = new Float32Array(m * 6);
    const ecol = new Float32Array(m * 6), tpos = new Float32Array(m * 2);
    this.edgeActive = new Float32Array(m * 2);
    this.edgeHl = new Float32Array(m * 2);
    const ca = new THREE.Color(), cb = new THREE.Color();
    // Structural "contains" links are dimmer so dense hubs don't flare white.
    const KIND_GAIN = [0.42, 1.0, 0.9, 0.7, 1.0, 1.0];
    this.links.forEach(([a, b, k], e) => {
      const g = KIND_GAIN[k] ?? 1;
      ca.fromArray(this.colors, a * 3).multiplyScalar(g); cb.fromArray(this.colors, b * 3).multiplyScalar(g);
      ecol.set([ca.r, ca.g, ca.b, cb.r, cb.g, cb.b], e * 6);
      tpos[e * 2] = 0; tpos[e * 2 + 1] = 1;
    });
    const eg = new THREE.BufferGeometry();
    this.edgePosAttr = new THREE.BufferAttribute(this.edgePos, 3).setUsage(THREE.DynamicDrawUsage);
    eg.setAttribute('position', this.edgePosAttr);
    eg.setAttribute('color', new THREE.BufferAttribute(ecol, 3));
    eg.setAttribute('tpos', new THREE.BufferAttribute(tpos, 1));
    this.edgeActiveAttr = new THREE.BufferAttribute(this.edgeActive, 1).setUsage(THREE.DynamicDrawUsage);
    eg.setAttribute('aFlow', this.edgeActiveAttr);
    this.edgeHlAttr = new THREE.BufferAttribute(this.edgeHl, 1).setUsage(THREE.DynamicDrawUsage);
    eg.setAttribute('hl', this.edgeHlAttr);
    this.edges.geometry.dispose();
    this.edges.geometry = eg;
    this.edgeGeo = eg;
    this.sparks.forEach((s) => { s.e = -1; s.life = 0; });

    // Additive light saturates on dense graphs: scale it with size.
    this.uniforms.uGlow.value = THREE.MathUtils.clamp(Math.sqrt(900 / Math.max(n, 1)), 0.2, 1);
    this.uniforms.uEdgeGain.value = THREE.MathUtils.clamp(Math.pow(1200 / Math.max(m, 1), 0.75), 0.06, 1);
    this.bloom.strength = 0.95 * THREE.MathUtils.clamp(Math.sqrt(1500 / Math.max(n, 1)), 0.4, 1);
    if (this.lastCount && (n > this.lastCount * 1.6 || n < this.lastCount / 1.6)) this.fitted = false;
    this.lastCount = n;

    this.selected = selectedId != null && this.index.has(selectedId) ? this.index.get(selectedId) : -1;
    this.hovered = -1;
    this.applyHighlight();
    this.syncEdgePositions();

    // Layout: a gentle settle when most positions are already known.
    const linkArr = new Uint32Array(m * 2), kinds = new Uint8Array(m), weights = new Float32Array(n);
    this.links.forEach(([a, b, k], e) => { linkArr[e * 2] = a; linkArr[e * 2 + 1] = b; kinds[e] = k; });
    this.nodes.forEach((nd, i) => { weights[i] = nd.w || 1; });
    const alpha = known / Math.max(n, 1) > 0.85 ? 0.12 : 1;
    this.worker.postMessage({ type: 'init', n, links: linkArr, linkKinds: kinds, weights, pos: new Float32Array(pos), pinned: brain !== undefined ? [brain] : [], alpha });
    this.layoutAlpha = alpha;
    if (!this.fitted && known / Math.max(n, 1) > 0.85) { this.fitted = true; setTimeout(() => this.fitAll(false), 50); }
    this.emit('graph', this.stats());
  }

  onLayout(d) {
    if (d.type !== 'tick' || !this.posArr || d.pos.length !== this.posArr.length) return;
    this.posArr.set(d.pos);
    this.posAttr.needsUpdate = true;
    this.layoutAlpha = d.alpha;
    this.syncEdgePositions();
    if (!this.fitted && d.alpha < 0.3) { this.fitted = true; this.fitAll(true); }
  }

  syncEdgePositions() {
    const p = this.posArr, ep = this.edgePos;
    for (let e = 0; e < this.links.length; e++) {
      const a = this.links[e][0] * 3, b = this.links[e][1] * 3, o = e * 6;
      ep[o] = p[a]; ep[o + 1] = p[a + 1]; ep[o + 2] = p[a + 2];
      ep[o + 3] = p[b]; ep[o + 4] = p[b + 1]; ep[o + 5] = p[b + 2];
    }
    this.edgePosAttr.needsUpdate = true;
  }

  stats() {
    const counts = {};
    this.nodes.forEach((n) => { counts[n.k] = (counts[n.k] || 0) + 1; });
    return { nodes: this.nodes.length, links: this.links.length, counts };
  }

  // ---------------------------------------------------------------- highlight
  setKindVisible(kind, visible) {
    if (visible) this.hidden.delete(kind); else this.hidden.add(kind);
    this.applyHighlight();
  }

  isHidden(i) { return this.hidden.has(this.nodes[i].k) && i !== this.selected; }

  applyHighlight() {
    const n = this.nodes.length;
    const sel = this.selected;
    const neigh = new Set(sel >= 0 ? this.adj[sel] : []);
    for (let i = 0; i < n; i++) {
      let h = 1;
      if (sel >= 0) h = i === sel ? 2.5 : neigh.has(i) ? 2 : 0.3;
      if (this.isHidden(i) && !(sel >= 0 && neigh.has(i))) h = 0;
      this.hl[i] = h;
    }
    this.links.forEach(([a, b], e) => {
      let h = this.hl[a] === 0 || this.hl[b] === 0 ? 0 : 1;
      if (sel >= 0 && h) h = a === sel || b === sel ? 4 : 0.12;
      this.edgeHl[e * 2] = this.edgeHl[e * 2 + 1] = h;
    });
    if (this.hlAttr) { this.hlAttr.needsUpdate = true; this.edgeHlAttr.needsUpdate = true; }
  }

  select(id, { focus = false, silent = false } = {}) {
    const i = id == null ? -1 : this.index.get(id) ?? -1;
    this.selected = i;
    this.applyHighlight();
    if (i >= 0) {
      for (const j of this.adj[i]) { const e = this.edge(i, j); if (e >= 0) this.spark(e, i, 2); }
      if (focus) this.focus(id);
    }
    if (!silent) this.emit('select', i >= 0 ? this.nodes[i] : null);
  }

  edge(a, b) { const k = a < b ? a * 1e7 + b : b * 1e7 + a; return this.edgeIndex.get(k) ?? -1; }

  neighbors(id) {
    const i = this.index.get(id);
    if (i === undefined) return [];
    return this.adj[i].map((j) => this.nodes[j]);
  }

  search(q, limit = 14) {
    q = q.trim().toLowerCase();
    if (!q) return [];
    const out = [];
    for (const n of this.nodes) {
      const l = n.l.toLowerCase();
      const idx = l.indexOf(q);
      if (idx >= 0) out.push({ n, score: (idx === 0 ? 0 : 1) + l.length / 200 - (n.w || 1) / 50 });
    }
    out.sort((a, b) => a.score - b.score);
    return out.slice(0, limit).map((o) => o.n);
  }

  // ---------------------------------------------------------------- activity
  /** Pulse a chain of nodes and send light along the edges between them. */
  firePath(ids, strength = 1) {
    const idx = ids.map((id) => this.index.get(id)).filter((i) => i !== undefined);
    idx.forEach((i, k) => {
      this.pulse[i] = Math.max(this.pulse[i], strength);
      if (k > 0) {
        const e = this.edge(idx[k - 1], i);
        if (e >= 0) {
          this.edgeActive[e * 2] = this.edgeActive[e * 2 + 1] = Math.max(this.edgeActive[e * 2], strength);
          this.sparkQueue.push({ at: performance.now() + k * 260, e, from: idx[k - 1], count: 4 });
        }
      }
    });
    this.pulseAttr.needsUpdate = true;
    this.edgeActiveAttr.needsUpdate = true;
  }

  setLive(ids) {
    this.live = new Set(ids.map((id) => this.index.get(id)).filter((i) => i !== undefined));
  }

  spark(e, fromNode, count = 3) {
    const [a] = this.links[e];
    for (let c = 0; c < count; c++) {
      const s = this.sparks[this.sparkCursor];
      this.sparkCursor = (this.sparkCursor + 1) % SPARKS;
      s.e = e; s.rev = fromNode !== a; s.t = -c * 0.18; s.v = 0.55 + Math.random() * 0.35; s.life = 1;
    }
  }

  // ---------------------------------------------------------------- camera
  focus(id, dist) {
    const i = this.index.get(id);
    if (i === undefined) return;
    const target = new THREE.Vector3().fromArray(this.posArr, i * 3);
    let reach = 0;
    for (const j of this.adj[i].slice(0, 60)) {
      reach = Math.max(reach, target.distanceTo(new THREE.Vector3().fromArray(this.posArr, j * 3)));
    }
    const d = dist || THREE.MathUtils.clamp(reach * 1.7, Math.max(70, this.sizes[i] * 9), 3000);
    const dir = this.camera.position.clone().sub(this.controls.target).normalize();
    this.flyTo(target, target.clone().add(dir.multiplyScalar(d)));
  }

  fitAll(animate = true) {
    const n = this.nodes.length;
    if (!n) return;
    const c = new THREE.Vector3();
    let count = 0;
    for (let i = 0; i < n; i++) if (this.hl[i] > 0) { c.x += this.posArr[i * 3]; c.y += this.posArr[i * 3 + 1]; c.z += this.posArr[i * 3 + 2]; count++; }
    if (!count) return;
    c.divideScalar(count);
    const d2 = [];
    for (let i = 0; i < n; i++) if (this.hl[i] > 0) {
      const dx = this.posArr[i * 3] - c.x, dy = this.posArr[i * 3 + 1] - c.y, dz = this.posArr[i * 3 + 2] - c.z;
      d2.push(dx * dx + dy * dy + dz * dz);
    }
    d2.sort((a, b) => a - b);
    const r = Math.sqrt(d2[Math.floor(d2.length * 0.97)] || 100) + 20;
    const dist = r / Math.sin(THREE.MathUtils.degToRad(this.camera.fov) / 2) * 0.85;
    const dir = this.camera.position.clone().sub(this.controls.target).normalize();
    if (dir.lengthSq() < 0.5) dir.set(0, 0.25, 1).normalize();
    const pos = c.clone().add(dir.multiplyScalar(dist));
    if (animate) this.flyTo(c, pos); else { this.controls.target.copy(c); this.camera.position.copy(pos); }
  }

  flyTo(target, position, ms = 1100) {
    this.tween = {
      t0: performance.now(), ms,
      fromT: this.controls.target.clone(), toT: target,
      fromP: this.camera.position.clone(), toP: position,
    };
    this.controls.autoRotate = false;
    this.lastInteract = performance.now();
  }

  /** Free flight. Uses pointer lock for mouse-look when the browser grants
   *  it; otherwise falls back to drag-to-look so flight always works. */
  setFly(on) {
    if (on === this.fly.on) return;
    const f = this.fly;
    if (on) {
      const dir = new THREE.Vector3();
      this.camera.getWorldDirection(dir);
      f.yaw = Math.atan2(-dir.x, -dir.z);
      f.pitch = Math.asin(THREE.MathUtils.clamp(dir.y, -1, 1));
      f.speed = THREE.MathUtils.clamp(this.camera.position.distanceTo(this.controls.target) * 0.6, 20, 6000);
      f.on = true;
      this.tween = null;
      this.controls.enabled = false;
      this.controls.autoRotate = false;
      try { const p = this.canvas.requestPointerLock?.(); if (p && p.catch) p.catch(() => {}); } catch { /* drag-to-look fallback */ }
    } else {
      f.on = false;
      if (document.pointerLockElement === this.canvas) document.exitPointerLock();
      const dir = new THREE.Vector3();
      this.camera.getWorldDirection(dir);
      this.controls.target.copy(this.camera.position).add(dir.multiplyScalar(Math.max(60, f.speed * 0.8)));
      this.controls.enabled = true;
    }
    f.keys.clear();
    this.lastInteract = performance.now();
    this.emit('fly', f.on);
  }

  setBloom(on) { this.bloomOn = on; }
  reheat() { this.worker.postMessage({ type: 'reheat', alpha: 0.6 }); }
  setAutoRotate(on) { this.controls.autoRotate = on; this.autoRotatePref = on; }

  // ---------------------------------------------------------------- input
  initInput() {
    const c = this.canvas;
    this.pointer = { x: -1, y: -1, down: null, moved: false };
    c.addEventListener('pointermove', (e) => {
      const r = c.getBoundingClientRect();
      this.pointer.x = e.clientX - r.left; this.pointer.y = e.clientY - r.top;
      if (this.pointer.down && Math.hypot(e.clientX - this.pointer.down[0], e.clientY - this.pointer.down[1]) > 4) this.pointer.moved = true;
      this.pointerDirty = true;
    });
    c.addEventListener('pointerleave', () => { this.pointer.x = -1; this.hovered = -1; c.style.cursor = ''; });
    c.addEventListener('pointerdown', (e) => { this.pointer.down = [e.clientX, e.clientY]; this.pointer.moved = false; c.focus(); });
    const local = (e) => { const r = c.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
    c.addEventListener('pointerup', (e) => {
      const wasClick = this.pointer.down && !this.pointer.moved && e.button === 0;
      this.pointer.down = null;
      if (!wasClick || this.fly.on) return;
      const i = this.pick(...local(e));
      if (i >= 0) this.select(this.nodes[i].id);
      else if (this.selected >= 0) this.select(null);
    });
    c.addEventListener('dblclick', (e) => {
      const i = this.pick(...local(e));
      if (i >= 0) { this.select(this.nodes[i].id); this.focus(this.nodes[i].id); }
    });

    // Fly mode: pointer lock + WASD
    // Losing pointer lock (Esc) ends flight only if we had it.
    let hadLock = false;
    document.addEventListener('pointerlockchange', () => {
      const locked = document.pointerLockElement === c;
      if (locked) hadLock = true;
      else if (hadLock) { hadLock = false; if (this.fly.on) this.setFly(false); }
    });
    document.addEventListener('mousemove', (e) => {
      if (!this.fly.on) return;
      const locked = document.pointerLockElement === c;
      if (!locked && !(e.buttons & 1)) return; // fallback: drag to look
      this.fly.yaw -= e.movementX * 0.0022;
      this.fly.pitch = THREE.MathUtils.clamp(this.fly.pitch - e.movementY * 0.0022, -1.55, 1.55);
    });
    c.addEventListener('wheel', (e) => {
      if (!this.fly.on) return;
      e.preventDefault();
      this.fly.speed = THREE.MathUtils.clamp(this.fly.speed * (e.deltaY > 0 ? 0.85 : 1.18), 2, 40000);
      this.emit('flyspeed', this.fly.speed);
    }, { passive: false });
    window.addEventListener('keydown', (e) => {
      if (!this.fly.on) return;
      if (e.code === 'Escape') { e.preventDefault(); this.setFly(false); return; }
      if (e.code === 'KeyF') return; // toggled by the app shell
      this.fly.keys.add(e.code);
      if (e.code === 'Space' || e.code.startsWith('Arrow')) e.preventDefault();
    });
    window.addEventListener('keyup', (e) => this.fly.keys.delete(e.code));
  }

  /** Screen-space pick: closest node whose sprite covers the pointer. */
  pick(px, py) {
    if (px < 0 || !this.posArr) return -1;
    const w = this.canvas.clientWidth, h = this.canvas.clientHeight;
    const m = new THREE.Matrix4().multiplyMatrices(this.camera.projectionMatrix, this.camera.matrixWorldInverse).elements;
    const scale = this.uniforms.uScale.value / this.renderer.getPixelRatio();
    let best = -1, bestD = Infinity;
    const p = this.posArr;
    for (let i = 0, n = this.nodes.length; i < n; i++) {
      if (this.hl[i] === 0) continue;
      const x = p[i * 3], y = p[i * 3 + 1], z = p[i * 3 + 2];
      const cw = m[3] * x + m[7] * y + m[11] * z + m[15];
      if (cw <= 0.1) continue;
      const sx = ((m[0] * x + m[4] * y + m[8] * z + m[12]) / cw * 0.5 + 0.5) * w;
      const sy = (1 - ((m[1] * x + m[5] * y + m[9] * z + m[13]) / cw * 0.5 + 0.5)) * h;
      const rad = Math.max(6, this.sizes[i] * scale / cw * 0.5);
      const d = Math.hypot(sx - px, sy - py);
      if (d <= rad && cw < bestD) { best = i; bestD = cw; }
    }
    return best;
  }

  // ---------------------------------------------------------------- frame
  animate() {
    requestAnimationFrame(this.animate);
    // Another tab is showing (or the window is hidden): do no GPU work.
    if (document.hidden || !this.container.offsetParent) { this.clock.getDelta(); return; }
    const dt = Math.min(this.clock.getDelta(), 0.1);
    const now = performance.now();
    this.uniforms.uTime.value += dt;

    if (this.tween) {
      const k = Math.min(1, (now - this.tween.t0) / this.tween.ms);
      const ease = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2;
      this.controls.target.lerpVectors(this.tween.fromT, this.tween.toT, ease);
      this.camera.position.lerpVectors(this.tween.fromP, this.tween.toP, ease);
      if (k >= 1) this.tween = null;
    }
    if (this.fly.on) this.stepFly(dt);
    else {
      if (this.autoRotatePref !== false && !this.controls.autoRotate && now - this.lastInteract > 45000) this.controls.autoRotate = true;
      this.controls.update();
    }
    if (this.stars) this.stars.rotation.y += dt * 0.002;

    if (this.pulse) this.stepActivity(dt, now);
    if (this.pointerDirty && !this.fly.on) {
      this.pointerDirty = false;
      const i = this.pick(this.pointer.x, this.pointer.y);
      if (i !== this.hovered) { this.hovered = i; this.canvas.style.cursor = i >= 0 ? 'pointer' : ''; this.emit('hover', i >= 0 ? this.nodes[i] : null); }
    }
    if (this.showLabels || this.hovered >= 0 || this.selected >= 0) this.updateLabels();
    else this.labelPool.forEach((el) => { el.style.display = 'none'; });

    if (this.bloomOn) this.composer.render(); else this.renderer.render(this.scene, this.camera);
  }

  stepFly(dt) {
    const f = this.fly, k = f.keys;
    const dir = new THREE.Vector3(-Math.sin(f.yaw) * Math.cos(f.pitch), Math.sin(f.pitch), -Math.cos(f.yaw) * Math.cos(f.pitch));
    this.camera.lookAt(this.camera.position.clone().add(dir));
    const right = new THREE.Vector3().crossVectors(dir, new THREE.Vector3(0, 1, 0)).normalize();
    const mv = new THREE.Vector3();
    if (k.has('KeyW') || k.has('ArrowUp')) mv.add(dir);
    if (k.has('KeyS') || k.has('ArrowDown')) mv.sub(dir);
    if (k.has('KeyD') || k.has('ArrowRight')) mv.add(right);
    if (k.has('KeyA') || k.has('ArrowLeft')) mv.sub(right);
    if (k.has('Space') || k.has('KeyE')) mv.y += 1;
    if (k.has('KeyC') || k.has('KeyQ') || k.has('ControlLeft')) mv.y -= 1;
    if (mv.lengthSq() > 0) {
      const boost = k.has('ShiftLeft') || k.has('ShiftRight') ? 4 : 1;
      this.camera.position.add(mv.normalize().multiplyScalar(f.speed * boost * dt));
    }
  }

  stepActivity(dt, now) {
    // Pulses decay; live conversations keep breathing; selection keeps a soft flow.
    const n = this.nodes.length;
    const decay = Math.exp(-dt / 2.2);
    let changed = false;
    for (let i = 0; i < n; i++) {
      const floor = this.live.has(i) ? 0.45 : 0;
      const v = Math.max(floor, this.pulse[i] * decay);
      if (Math.abs(v - this.pulse[i]) > 1e-4) { this.pulse[i] = v < 0.01 ? 0 : v; changed = true; }
    }
    if (changed) this.pulseAttr.needsUpdate = true;
    const edec = Math.exp(-dt / 1.8);
    let echanged = false;
    for (let e = 0; e < this.links.length; e++) {
      const sel = this.selected >= 0 && (this.links[e][0] === this.selected || this.links[e][1] === this.selected);
      const floor = sel ? 0.35 : 0;
      const v = Math.max(floor, this.edgeActive[e * 2] * edec);
      if (Math.abs(v - this.edgeActive[e * 2]) > 1e-4) { this.edgeActive[e * 2] = this.edgeActive[e * 2 + 1] = v < 0.01 ? 0 : v; echanged = true; }
    }
    if (echanged) this.edgeActiveAttr.needsUpdate = true;

    // Live conversations: an occasional heartbeat spark from their project.
    if (this.live.size && Math.random() < dt * 0.8) {
      const arr = [...this.live]; const i = arr[Math.floor(Math.random() * arr.length)];
      for (const j of this.adj[i]) { const e = this.edge(i, j); if (e >= 0) this.spark(e, j, 2); }
    }
    // Delayed sparks so paths light up in order (brain → project → conversation → tools → skill).
    if (this.sparkQueue.length) {
      this.sparkQueue = this.sparkQueue.filter((q) => { if (q.at <= now) { this.spark(q.e, q.from, q.count); return false; } return true; });
    }
    // Move sparks
    const p = this.posArr;
    for (let s = 0; s < SPARKS; s++) {
      const sp = this.sparks[s];
      const o = s * 3;
      if (sp.e < 0 || sp.life <= 0 || sp.e >= this.links.length) { this.sparkAlpha[s] = 0; continue; }
      sp.t += dt * sp.v;
      let [a, b] = this.links[sp.e];
      if (sp.rev) [a, b] = [b, a];
      if (sp.t > 1) { // arrival: the target node flickers
        this.pulse[b] = Math.max(this.pulse[b], 0.55);
        sp.life = 0; this.sparkAlpha[s] = 0;
        continue;
      }
      const t = Math.max(0, sp.t);
      this.sparkPos[o] = p[a * 3] + (p[b * 3] - p[a * 3]) * t;
      this.sparkPos[o + 1] = p[a * 3 + 1] + (p[b * 3 + 1] - p[a * 3 + 1]) * t;
      this.sparkPos[o + 2] = p[a * 3 + 2] + (p[b * 3 + 2] - p[a * 3 + 2]) * t;
      this.sparkCol[o] = this.colors[b * 3]; this.sparkCol[o + 1] = this.colors[b * 3 + 1]; this.sparkCol[o + 2] = this.colors[b * 3 + 2];
      this.sparkAlpha[s] = sp.t < 0 ? 0 : Math.sin(Math.PI * t) * 0.9 + 0.1;
    }
    this.sparkGeo.attributes.position.needsUpdate = true;
    this.sparkGeo.attributes.color.needsUpdate = true;
    this.sparkGeo.attributes.alpha.needsUpdate = true;
  }

  updateLabels() {
    this.frameNo = (this.frameNo || 0) + 1;
    if (this.frameNo % 2) return;
    const w = this.canvas.clientWidth, h = this.canvas.clientHeight;
    const m = new THREE.Matrix4().multiplyMatrices(this.camera.projectionMatrix, this.camera.matrixWorldInverse).elements;
    const scale = this.uniforms.uScale.value / this.renderer.getPixelRatio();
    const p = this.posArr;
    const cand = [];
    const pinned = new Set();
    if (this.selected >= 0) { pinned.add(this.selected); this.adj[this.selected].forEach((j) => pinned.add(j)); }
    if (this.hovered >= 0) pinned.add(this.hovered);
    for (let i = 0, n = this.nodes.length; i < n; i++) {
      if (this.hl[i] === 0) continue;
      const isPinned = pinned.has(i);
      if (!this.showLabels && !isPinned) continue;
      const x = p[i * 3], y = p[i * 3 + 1], z = p[i * 3 + 2];
      const cw = m[3] * x + m[7] * y + m[11] * z + m[15];
      if (cw <= 0.5) continue;
      const nx = (m[0] * x + m[4] * y + m[8] * z + m[12]) / cw, ny = (m[1] * x + m[5] * y + m[9] * z + m[13]) / cw;
      if (nx < -1.05 || nx > 1.05 || ny < -1.05 || ny > 1.05) continue;
      const rad = this.sizes[i] * scale / cw * 0.5;
      if (!isPinned && rad < 7) continue;
      if (this.selected >= 0 && !isPinned && this.hl[i] < 1) continue;
      cand.push({ i, sx: (nx * 0.5 + 0.5) * w, sy: (1 - (ny * 0.5 + 0.5)) * h, rad, pr: (isPinned ? 1e6 : 0) + (i === this.hovered ? 1e7 : 0) + rad });
    }
    cand.sort((a, b) => b.pr - a.pr);
    const cells = new Set();
    let used = 0;
    for (const c of cand) {
      if (used >= this.labelPool.length) break;
      const cx = Math.floor(c.sx / 110), cy = Math.floor((c.sy + c.rad + 8) / 18);
      const key = cx + ',' + cy, key2 = (cx + 1) + ',' + cy;
      if (c.pr < 1e7 * 0.9 && c.i !== this.selected && (cells.has(key) || cells.has(key2))) continue;
      cells.add(key); cells.add(key2);
      const el = this.labelPool[used++];
      const nd = this.nodes[c.i];
      if (el._id !== nd.id) { el.textContent = nd.l; el._id = nd.id; el.dataset.kind = nd.k; }
      el.classList.toggle('sel', c.i === this.selected);
      el.classList.toggle('hov', c.i === this.hovered);
      el.style.display = '';
      el.style.transform = `translate(${c.sx.toFixed(1)}px, ${(c.sy + c.rad + 3).toFixed(1)}px) translateX(-50%)`;
    }
    for (let k = used; k < this.labelPool.length; k++) this.labelPool[k].style.display = 'none';
  }

  // ---------------------------------------------------------------- persistence
  loadPositions() {
    try { return JSON.parse(localStorage.getItem(POS_KEY) || '{}') || {}; } catch { return {}; }
  }
  savePositions() {
    if (!this.posArr || this.layoutAlpha > 0.5) return;
    try {
      const out = {};
      this.nodes.forEach((n, i) => {
        out[n.id] = [Math.round(this.posArr[i * 3] * 10) / 10, Math.round(this.posArr[i * 3 + 1] * 10) / 10, Math.round(this.posArr[i * 3 + 2] * 10) / 10];
      });
      localStorage.setItem(POS_KEY, JSON.stringify(out));
      this.savedPos = out;
    } catch { /* storage full or blocked: layout just won't persist */ }
  }
}
