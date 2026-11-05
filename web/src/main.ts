import wasmUrl from './wasm/dhool.wasm?url';
import { FixedStep, RateMeter } from './loop';
import { ERASER_KEY, MATERIAL_COUNT, Mat, PALETTE, materialByKey, materialName, type MatId } from './materials';
import { PRESETS, presetById } from './presets';
import { parseSceneHash, sceneHash } from './share';
import { spacingFor, strokePoints, type Point } from './stroke';
import { Sim } from './wasm';

const SPEEDS = [0.25, 0.5, 1, 2, 4] as const;
const MAX_BRUSH = 24;
const INTRO_KEY = 'dhool.intro.seen';

function $<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el as T;
}

function storageGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function storageSet(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private mode or storage disabled: the intro just shows again next time.
  }
}

function randomSeed(): number {
  const buf = new Uint32Array(1);
  crypto.getRandomValues(buf);
  return buf[0] ?? 1;
}

/** Grid size for the current viewport: landscape by default, taller on portrait phones. */
function gridSize(): [number, number] {
  const portrait = window.innerHeight > window.innerWidth && window.innerWidth < 760;
  return portrait ? [200, 240] : [320, 200];
}

const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const sim = await Sim.load(wasmUrl);
if (sim.materialCount !== MATERIAL_COUNT) {
  console.warn(`palette expects ${MATERIAL_COUNT} materials, core has ${sim.materialCount}`);
}

const canvas = $<HTMLCanvasElement>('world');
const stage = $<HTMLElement>('stage');
const ctx = canvas.getContext('2d', { alpha: false });
if (!ctx) throw new Error('2D canvas unavailable');

const state = {
  material: Mat.Sand as MatId,
  eraser: false,
  brush: 4,
  speed: 1,
  running: !reducedMotion,
};

// ------------------------------------------------------------------ canvas

function fitCanvas(): void {
  const w = sim.width;
  const h = sim.height;
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
  }
  const style = getComputedStyle(stage);
  const availW = stage.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
  const availH = stage.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
  const scale = Math.max(0.5, Math.min(availW / w, availH / h));
  // Snap to whole device pixels per cell when the scale is large enough to matter.
  const dpr = window.devicePixelRatio || 1;
  const snapped = scale * dpr >= 2 ? Math.floor(scale * dpr) / dpr : scale;
  canvas.style.width = `${Math.round(w * snapped)}px`;
  canvas.style.height = `${Math.round(h * snapped)}px`;
  $<HTMLElement>('grid').textContent = `${w}×${h}`;
}

new ResizeObserver(fitCanvas).observe(stage);

function draw(): void {
  ctx!.putImageData(sim.frame(), 0, 0);
}

// ----------------------------------------------------------------- scenes

function loadPreset(id: string): void {
  const preset = presetById(id) ?? PRESETS[0];
  if (!preset) return;
  const [w, h] = gridSize();
  sim.init(w, h, randomSeed());
  preset.build(sim);
  history.replaceState(null, '', location.pathname + location.search);
  fitCanvas();
  draw();
}

function loadFromHash(): boolean {
  const bytes = parseSceneHash(location.hash);
  if (!bytes) return false;
  if (!sim.load(bytes)) {
    toast('That share link is damaged');
    return false;
  }
  fitCanvas();
  draw();
  return true;
}

async function share(): Promise<void> {
  const hash = sceneHash(sim.save());
  history.replaceState(null, '', hash);
  try {
    await navigator.clipboard.writeText(location.href);
    toast('Link copied');
  } catch {
    toast('Link is in the address bar');
  }
}

window.addEventListener('hashchange', () => {
  loadFromHash();
});

// ------------------------------------------------------------------ toast

let toastTimer = 0;
function toast(message: string): void {
  const el = $<HTMLElement>('toast');
  el.textContent = message;
  el.classList.add('show');
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => el.classList.remove('show'), 1800);
}

// ---------------------------------------------------------------- controls

const playBtn = $<HTMLButtonElement>('play');
const pausedBadge = $<HTMLElement>('paused-badge');

function setRunning(running: boolean): void {
  state.running = running;
  playBtn.setAttribute('aria-pressed', String(!running));
  playBtn.querySelector('.label')!.textContent = running ? 'Pause' : 'Play';
  playBtn.title = running ? 'Pause (Space)' : 'Play (Space)';
  pausedBadge.hidden = running;
  stepper.reset();
}

function step(): void {
  if (state.running) setRunning(false);
  sim.tick(1);
  draw();
}

const paletteEl = $<HTMLElement>('palette');
const swatches = new Map<MatId, HTMLButtonElement>();
for (const m of PALETTE) {
  const b = document.createElement('button');
  b.type = 'button';
  b.className = 'swatch';
  b.setAttribute('role', 'radio');
  b.setAttribute('aria-checked', 'false');
  b.setAttribute('aria-keyshortcuts', m.key);
  b.title = `${m.name}: ${m.hint} (${m.key.toUpperCase()})`;
  b.style.setProperty('--chip', m.swatch);
  b.innerHTML = `<span class="chip" aria-hidden="true"></span><span>${m.name}</span><kbd aria-hidden="true">${m.key.toUpperCase()}</kbd>`;
  b.addEventListener('click', () => selectMaterial(m.id));
  paletteEl.append(b);
  swatches.set(m.id, b);
}

// Arrow keys move through the radio group, as ARIA expects.
paletteEl.addEventListener('keydown', (e) => {
  const order = PALETTE.map((m) => m.id);
  const i = order.indexOf(state.material);
  const delta = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
  if (delta === 0 || i < 0) return;
  e.preventDefault();
  const next = order[(i + delta + order.length) % order.length];
  if (next !== undefined) {
    selectMaterial(next);
    swatches.get(next)?.focus();
  }
});

const eraserBtn = $<HTMLButtonElement>('eraser');

function selectMaterial(id: MatId): void {
  state.material = id;
  setEraser(false);
  for (const [mid, b] of swatches) {
    const on = mid === id;
    b.setAttribute('aria-checked', String(on));
    b.tabIndex = on ? 0 : -1;
  }
}

function setEraser(on: boolean): void {
  state.eraser = on;
  eraserBtn.setAttribute('aria-pressed', String(on));
}

const brushInput = $<HTMLInputElement>('brush');
const brushOut = $<HTMLOutputElement>('brush-out');
brushInput.max = String(MAX_BRUSH);

function setBrush(r: number): void {
  state.brush = Math.max(0, Math.min(MAX_BRUSH, Math.round(r)));
  brushInput.value = String(state.brush);
  brushOut.value = String(state.brush);
}

brushInput.addEventListener('input', () => setBrush(Number(brushInput.value)));

const speedEl = $<HTMLElement>('speed');
const speedBtns: HTMLButtonElement[] = [];
for (const s of SPEEDS) {
  const b = document.createElement('button');
  b.type = 'button';
  b.setAttribute('role', 'radio');
  b.textContent = `${s}×`;
  b.setAttribute('aria-label', `${s} times speed`);
  b.addEventListener('click', () => setSpeed(s));
  speedEl.append(b);
  speedBtns.push(b);
}

function setSpeed(s: number): void {
  state.speed = s;
  SPEEDS.forEach((v, i) => {
    const b = speedBtns[i];
    if (!b) return;
    b.setAttribute('aria-checked', String(v === s));
    b.tabIndex = v === s ? 0 : -1;
  });
}

const presetsEl = $<HTMLElement>('presets');
for (const p of PRESETS) {
  const b = document.createElement('button');
  b.type = 'button';
  b.className = 'preset';
  b.dataset.preset = p.id;
  b.innerHTML = `<span>${p.name}</span><small>${p.blurb}</small>`;
  b.addEventListener('click', () => loadPreset(p.id));
  presetsEl.append(b);
}

playBtn.addEventListener('click', () => setRunning(!state.running));
$<HTMLButtonElement>('step').addEventListener('click', step);
$<HTMLButtonElement>('clear').addEventListener('click', () => {
  sim.clear();
  history.replaceState(null, '', location.pathname + location.search);
  draw();
});
$<HTMLButtonElement>('share').addEventListener('click', () => void share());
eraserBtn.addEventListener('click', () => setEraser(!state.eraser));

// ------------------------------------------------------------------ intro

const intro = $<HTMLElement>('intro');
function dismissIntro(): void {
  if (intro.hidden) return;
  intro.hidden = true;
  storageSet(INTRO_KEY, '1');
}
if (!storageGet(INTRO_KEY)) intro.hidden = false;
$<HTMLButtonElement>('intro-ok').addEventListener('click', () => {
  dismissIntro();
  swatches.get(state.material)?.focus();
});

// --------------------------------------------------------------- painting

interface Stroke {
  readonly id: number;
  readonly erase: boolean;
  last: Point;
}

let stroke: Stroke | null = null;

function toCell(e: PointerEvent): Point {
  const rect = canvas.getBoundingClientRect();
  const x = Math.floor(((e.clientX - rect.left) / rect.width) * sim.width);
  const y = Math.floor(((e.clientY - rect.top) / rect.height) * sim.height);
  return [x, y];
}

function dab([x, y]: Point, erase: boolean): void {
  sim.paint(x, y, state.brush, erase ? Mat.Empty : state.material);
}

canvas.addEventListener('pointerdown', (e) => {
  if (e.pointerType === 'mouse' && e.button !== 0 && e.button !== 2) return;
  e.preventDefault();
  canvas.setPointerCapture(e.pointerId);
  const p = toCell(e);
  stroke = { id: e.pointerId, erase: e.button === 2 || state.eraser, last: p };
  dab(p, stroke.erase);
  dismissIntro();
  draw();
});

canvas.addEventListener('pointermove', (e) => {
  const here = toCell(e);
  showHover(here);
  if (!stroke || e.pointerId !== stroke.id) return;
  const events = typeof e.getCoalescedEvents === 'function' ? e.getCoalescedEvents() : [];
  for (const ev of events.length > 0 ? events : [e]) {
    const p = toCell(ev);
    for (const q of strokePoints(stroke.last, p, spacingFor(state.brush))) dab(q, stroke.erase);
    stroke.last = p;
  }
});

function endStroke(e: PointerEvent): void {
  if (stroke && e.pointerId === stroke.id) stroke = null;
}
canvas.addEventListener('pointerup', endStroke);
canvas.addEventListener('pointercancel', endStroke);
canvas.addEventListener('pointerleave', () => {
  hoverEl.innerHTML = '&nbsp;';
});
canvas.addEventListener('contextmenu', (e) => e.preventDefault());

const hoverEl = $<HTMLElement>('hover');
function showHover([x, y]: Point): void {
  const id = sim.get(x, y);
  hoverEl.textContent = id < 0 ? ' ' : `(${x}, ${y}) ${materialName(id)}`;
}

// --------------------------------------------------------------- keyboard

window.addEventListener('keydown', (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const t = e.target as HTMLElement | null;
  if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT')) return;
  if (e.key === ' ') {
    // Space is always play/pause, even when a toolbar button has focus;
    // otherwise it would re-press whatever was clicked last (say, Clear).
    // Buttons stay reachable with Enter.
    e.preventDefault();
    if (!e.repeat) setRunning(!state.running);
    return;
  }
  if (e.key === '.') return step();
  if (e.key === '[') return setBrush(state.brush - 1);
  if (e.key === ']') return setBrush(state.brush + 1);
  if (e.key.toLowerCase() === ERASER_KEY) return setEraser(!state.eraser);
  if (e.key === 'Escape') return dismissIntro();
  const m = materialByKey(e.key);
  if (m) selectMaterial(m.id);
});

// Swallow the keyup too, which is when browsers activate a focused button.
window.addEventListener('keyup', (e) => {
  const t = e.target as HTMLElement | null;
  if (e.key === ' ' && t?.tagName === 'BUTTON') e.preventDefault();
});

// ------------------------------------------------------------------ stats

const countsEl = $<HTMLUListElement>('counts');
const countRows = PALETTE.map((m) => {
  const li = document.createElement('li');
  li.dataset.mat = m.name.toLowerCase();
  li.style.setProperty('--chip', m.swatch);
  li.innerHTML = `<span class="dot" aria-hidden="true"></span><span>${m.name}</span><span class="share-bar" aria-hidden="true"></span><span class="num">0</span>`;
  li.hidden = true;
  countsEl.append(li);
  return { id: m.id, li, bar: li.querySelector<HTMLElement>('.share-bar')!, num: li.querySelector<HTMLElement>('.num')! };
});

const fpsEl = $<HTMLElement>('fps');
const tpsEl = $<HTMLElement>('tps');

function updateStats(): void {
  fpsEl.textContent = fps.rate.toFixed(0);
  tpsEl.textContent = (state.running ? tps.rate : 0).toFixed(0);
  const total = sim.width * sim.height;
  for (const row of countRows) {
    const n = sim.count(row.id);
    row.li.hidden = n === 0;
    row.li.dataset.count = String(n);
    row.num.textContent = n.toLocaleString('en');
    row.bar.style.transform = `scaleX(${Math.min(1, (n / total) * 3).toFixed(3)})`;
  }
}

// ------------------------------------------------------------------- loop

const stepper = new FixedStep(1000 / 60);
const fps = new RateMeter();
const tps = new RateMeter();
let rafId = 0;
let lastTime = 0;
let lastStats = 0;

function frame(now: number): void {
  rafId = requestAnimationFrame(frame);
  const dt = lastTime === 0 ? 0 : now - lastTime;
  lastTime = now;
  // Holding still keeps pouring.
  if (stroke) dab(stroke.last, stroke.erase);
  if (state.running) {
    const n = stepper.advance(dt, state.speed);
    sim.tick(n);
    tps.add(n, now);
  }
  draw();
  fps.add(1, now);
  if (now - lastStats > 250) {
    lastStats = now;
    updateStats();
  }
}

function start(): void {
  if (rafId) return;
  lastTime = 0;
  stepper.reset();
  rafId = requestAnimationFrame(frame);
}

function stop(): void {
  cancelAnimationFrame(rafId);
  rafId = 0;
}

document.addEventListener('visibilitychange', () => (document.hidden ? stop() : start()));

// ------------------------------------------------------------------- boot

selectMaterial(Mat.Sand);
setBrush(state.brush);
setSpeed(1);
setRunning(state.running);
if (!loadFromHash()) loadPreset('hourglass');
updateStats();
if (!document.hidden) start();
