import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

 
 
function boot(file, saved = '0') {
  let now = 0, nextId = 1, reduced = false;
  const frames = new Map(), timers = new Map(), elements = [];
  const stats = { contexts: 0, transforms: 0, draws: 0, resizes: 0 };
  class Target {
    listeners = new Map();
    addEventListener(type, callback) {
      const list = this.listeners.get(type) || [];
      list.push(callback); this.listeners.set(type, list);
    }
    fire(type, event = {}) { for (const callback of this.listeners.get(type) || []) callback(event); }
  }
  function element(tag) {
    const classes = new Set();
    let width = 300, height = 150;
    const node = {
      tagName: tag, children: [],
      classList: {
        add: (...names) => names.forEach(name => classes.add(name)),
        remove: (...names) => names.forEach(name => classes.delete(name)),
        contains: name => classes.has(name),
        toggle(name, force) { const on = force ?? !classes.has(name); on ? classes.add(name) : classes.delete(name); return on; },
      },
      setAttribute() {}, appendChild(child) { this.children.push(child); },
      style: new Proxy({}, { set(object, key, value) { if (key === 'transform') stats.transforms++; object[key] = value; return true; } }),
      getContext() {
        stats.contexts++;
        return new Proxy({}, { get(_object, key) {
          if (key === 'createRadialGradient') return () => ({ addColorStop() {} });
          if (key === 'drawImage') return () => { stats.draws++; };
          return () => {};
        }, set() { return true; } });
      },
      get width() { return width; }, set width(value) { width = value; stats.resizes++; },
      get height() { return height; }, set height(value) { height = value; stats.resizes++; },
    };
    elements.push(node); return node;
  }
  const document = new Target();
  Object.assign(document, { readyState: 'complete', hidden: false, body: element('body'), createElement: element });
  const window = new Target();
  const media = new Target();
  Object.defineProperty(media, 'matches', { get: () => reduced });
  Object.assign(window, { innerWidth: 1280, innerHeight: 840, devicePixelRatio: 1.5,
    matchMedia: () => media, K1ttyAppearance: { isReducedMotion: () => reduced } });
  const store = new Map([['k1tty_cat', saved], ['k1tty_winter', saved]]);
  const context = vm.createContext({
    window, document, console,
    localStorage: { getItem: key => store.get(key) ?? null, setItem: (key, value) => store.set(key, value) },
    requestAnimationFrame(callback) { const id = nextId++; frames.set(id, callback); return id; },
    cancelAnimationFrame: id => frames.delete(id),
    setTimeout(callback, delay = 0) { const id = nextId++; timers.set(id, { callback, at: now + delay }); return id; },
    clearTimeout: id => timers.delete(id),
  });
  vm.runInContext(readFileSync(new URL(`../static/${file}`, import.meta.url), 'utf8'), context, { filename: file });
  function advance(ms) {
    now += ms;
    for (const [id, timer] of [...timers]) if (timer.at <= now) { timers.delete(id); timer.callback(); }
  }
  function frame(ms = 16) {
    advance(ms);
    const pending = [...frames.values()]; frames.clear();
    for (const callback of pending) callback(now);
  }
  return { window, document, frames, timers, stats, elements, frame, advance,
    reduce(value) { reduced = value; window.fire('k1tty:appearancechange'); },
    visibility(hidden) { document.hidden = hidden; document.fire('visibilitychange'); },
    canvas: () => elements.find(el => el.id === (file === 'winter.js' ? 'winterCanvas' : 'pixelCat')),
  };
}

for (const file of ['winter.js', 'pixel-cat.js']) {
  const env = boot(file);
  assert.equal(env.frames.size, 0, `${file}: disabled effects do not request frames`);
  assert.equal(env.timers.size, 0, `${file}: disabled effects do not start timers`);
  assert.equal(env.stats.contexts, 0, `${file}: disabled effects do not create canvas resources`);
  const initialTransforms = env.stats.transforms;
  for (let i = 0; i < 20; i++) {
    env.window.fire('pointermove', { clientX: i * 5, clientY: 30 });
    env.window.fire('resize');
  }
  assert.equal(env.stats.transforms, initialTransforms, `${file}: hidden effects ignore pointer/resize work`);
  assert.equal(env.frames.size, 0);
}

const snow = boot('winter.js');
snow.window.K1ttyWinter.setEnabled(true);
assert.ok(snow.stats.contexts > 0);
snow.frame();
assert.equal(snow.frames.size, 1, 'Snow has one active render loop');
const beforeResize = snow.stats.resizes;
snow.window.innerWidth = 1400;
for (let i = 0; i < 40; i++) snow.window.fire('resize');
assert.equal(snow.stats.resizes, beforeResize, 'Resize events are batched until the next display frame');
snow.frame();
assert.equal(snow.stats.resizes, beforeResize + 2, 'The backing canvas resizes once per frame');
const snowQueue = [...snow.frames.keys()];
snow.window.fire('k1tty:appearancechange');
assert.deepEqual([...snow.frames.keys()], snowQueue, 'Color-only changes leave the snow loop alone');
snow.visibility(true);
assert.equal(snow.frames.size, 0, 'Background window stops snow');
snow.visibility(false); snow.frame();
assert.equal(snow.frames.size, 1);
snow.window.K1ttyWinter.setEnabled(false);
assert.equal(snow.frames.size, 0, 'Turning snow off cancels animation immediately');
snow.advance(180);
assert.equal(snow.canvas().width, 1, 'Disabled snow releases its full-window backing store');
snow.window.K1ttyWinter.setEnabled(true); snow.frame();
assert.equal(snow.canvas().width, 2100, 'Re-enabling snow restores the correctly scaled canvas');
snow.reduce(true);
assert.equal(snow.frames.size, 0, 'Reduced motion suspends snow without a polling loop');
snow.window.K1ttyWinter.setEnabled(false); snow.advance(180);
snow.visibility(true); snow.visibility(false);
assert.equal(snow.frames.size, 0, 'Visibility changes do not restart a disabled effect');
const fastSnow = boot('winter.js');
fastSnow.window.K1ttyWinter.setEnabled(true);
fastSnow.reduce(true);
assert.equal(fastSnow.frames.size, 0, 'Reduced motion also cancels a not-yet-painted snow fade');
fastSnow.reduce(false);
fastSnow.window.K1ttyWinter.setEnabled(false);
fastSnow.frame();
assert.equal(fastSnow.canvas().classList.contains('visible'), false, 'Rapid off cancels delayed fade-in');

const cat = boot('pixel-cat.js');
cat.window.K1ttyCat.setEnabled(true);
assert.ok(cat.stats.contexts > 0);
assert.equal(cat.frames.size, 0, 'A cat without a pointer stays still without a display loop');
cat.window.fire('pointermove', { clientX: 300, clientY: 300 });
cat.frame();
assert.equal(cat.frames.size, 0, 'A resting cat yields the display loop');
assert.equal(cat.timers.size, 1, 'A low-frequency timer advances only idle sprite state');
const idleTransforms = cat.stats.transforms;
for (let i = 0; i < 10; i++) cat.frame(100);
assert.equal(cat.stats.transforms, idleTransforms, 'An idle cat never rewrites the same transform');
cat.window.fire('pointermove', { clientX: 900, clientY: 300 });
assert.equal(cat.frames.size, 1, 'Pointer movement immediately wakes the cat');
assert.equal(cat.timers.size, 0, 'Waking movement cancels the idle timer');
cat.frame(); cat.frame();
assert.ok(cat.stats.transforms > idleTransforms);
const catQueue = [...cat.frames.keys()];
cat.window.fire('k1tty:appearancechange');
assert.deepEqual([...cat.frames.keys()], catQueue, 'Color-only changes do not restart a cat animation');
cat.visibility(true);
assert.equal(cat.frames.size + cat.timers.size, 0, 'A hidden window stops all cat clocks');
cat.visibility(false); cat.frame();
cat.reduce(true);
assert.equal(cat.frames.size + cat.timers.size, 0, 'Reduced motion stops cat clocks');
cat.reduce(false); cat.frame();
cat.window.K1ttyCat.setEnabled(false);
assert.equal(cat.frames.size + cat.timers.size, 0, 'Disabled cat has no idle callbacks');

const css = readFileSync(new URL('../static/style.css', import.meta.url), 'utf8');
const transitionValues = [...css.matchAll(/(?:^|[;{}\n])\s*transition\s*:\s*([^;]+);/g)].map(match => match[1]);
assert.ok(transitionValues.every(value => !/(?:^|,)\s*(?:all|width|height|top|left|margin(?:-[a-z]+)?|filter)\b/.test(value)),
  'UI transitions never animate layout dimensions, broad all, or blur filters');
assert.ok(!css.includes('backdrop-filter: blur(4px)'), 'Modal entry does not blur the full transcript');
console.log('PASS: effects allocate lazily, idle/hidden clocks stop, resize batches, motion overrides and layout-safe transitions');
