import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import vm from 'node:vm';

const rootPath = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const source = await readFile(resolve(rootPath, 'static/appearance.js'), 'utf8');
const css = await readFile(resolve(rootPath, 'static/appearance.css'), 'utf8');
const preferenceKey = 'k1tty_appearance_v1';
function boot({ saved, blocked = false, reduced = false } = {}) {
  const values = new Map(saved === undefined ? [] : [[preferenceKey, saved]]);
  const styles = new Map(), listeners = new Map(), media = new Map(), timers = new Map();
  const counts = { styles: 0, storage: 0, preferences: 0, events: 0 };
  let sequence = 0;
  const element = { dataset: {}, style: { setProperty(name, value) { counts.styles++; styles.set(name, value); } } };
  const document = { documentElement: element, readyState: 'loading', visibilityState: 'visible', getElementById: () => null, addEventListener() {} };
  const window = {
    matchMedia(query) { const result = { matches: reduced, addEventListener(type, handler) { this.change = handler; } }; media.set(query, result); return result; },
    addEventListener(type, handler) { listeners.set(type, handler); },
    dispatchEvent() { counts.events++; },
    setTimeout(callback) { const id = ++sequence; timers.set(id, callback); return id; },
    clearTimeout(id) { timers.delete(id); },
    requestAnimationFrame(callback) { const id = ++sequence; timers.set(id, callback); return id; },
    cancelAnimationFrame(id) { timers.delete(id); },
  };
  const localStorage = { getItem(key) { if (blocked) throw new Error('Storage blocked'); return values.get(key) ?? null; }, setItem(key, value) { if (blocked) throw new Error('Storage blocked'); counts.storage++; if (key === preferenceKey) counts.preferences++; values.set(key, String(value)); } };
  const context = vm.createContext({ window, document, localStorage, CustomEvent: class { constructor(type, init) { this.type = type; this.detail = init?.detail; } } });
  vm.runInContext(source, context, { filename: 'appearance.js' });
  function flushTimers() { for (const [id, callback] of [...timers]) { timers.delete(id); callback(); } }
  return { api: window.K1ttyAppearance, element, styles, values, media, listeners, window, counts, timers, flushTimers };
}
function luminance(color) {
  return [1, 3, 5].map(i => parseInt(color.slice(i, i + 2), 16) / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, v, i) => sum + v * [.2126, .7152, .0722][i], 0);
}
function contrast(a, b) { const x = luminance(a), y = luminance(b); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05); }

 
const first = boot();
assert.equal(first.element.dataset.appearance, 'graphite');
assert.equal(first.styles.get('--bg-canvas'), '#141415');
assert.equal(first.values.get('k1tty_winter'), '0');
assert.equal(first.values.get('k1tty_cat'), '0');
assert.equal(first.api.getSettings().textSize, 14);
assert.equal(first.api.getSettings().controlRadius, 8);
first.api.update({ theme: 'light', accent: 'green', textSize: 18, density: 'compact', readingWidth: 'wide' });
assert.equal(first.element.dataset.appearance, 'light');
assert.equal(first.element.dataset.colorScheme, 'light');
assert.equal(first.styles.get('--txt-1'), '#202024');
assert.equal(first.styles.get('--accent'), '#187044');
assert.equal(first.styles.get('--chat-font-size'), '18px');
assert.equal(first.styles.get('--reading-width'), '1040px');
const restart = boot({ saved: first.values.get(preferenceKey) });
assert.equal(restart.api.getSettings().theme, 'light');
assert.equal(restart.api.getSettings().textSize, 18);
assert.equal(restart.element.dataset.density, 'compact');

 
first.api.update({ customColors: { canvas: '#ffffff', surface: '#eeeeee' }, accent: 'green' });
first.api.update({ theme: 'crimson' });
assert.equal(first.api.getSettings().theme, 'crimson');
assert.equal(first.api.getSettings().accent, 'red');
assert.equal(first.api.getSettings().primaryStyle, 'accent');
assert.equal(first.styles.get('--primary-bg'), '#ed747e');
assert.equal(Object.keys(first.api.getSettings().customColors).length, 0);
assert.equal(first.styles.get('--bg-canvas'), '#200e12');
assert.equal(first.styles.get('--bg-sidebar'), '#180b0e');
assert.equal(first.styles.get('--accent'), '#ed747e');
for (const theme of ['graphite', 'charcoal', 'light', 'crimson']) {
  first.api.update({ theme });
  for (const surface of ['--bg-canvas', '--bg-sidebar', '--bg-surface', '--bg-raised']) {
    assert.ok(contrast(first.styles.get('--txt-1'), first.styles.get(surface)) >= 4.5, `${theme} readable primary text on ${surface}`);
    assert.ok(contrast(first.styles.get('--txt-3'), first.styles.get(surface)) >= 4.5, `${theme} readable muted text on ${surface}`);
  }
  for (const accent of ['blue', 'green', 'amber', 'violet', 'red', 'orange', 'teal', 'rose']) {
    first.api.update({ accent });
    assert.ok(contrast(first.styles.get('--accent'), first.styles.get('--accent-ink')) >= 4.5, `${theme}/${accent} readable switch ink`);
    assert.ok(contrast(first.styles.get('--primary-bg'), first.styles.get('--on-primary')) >= 4.5, `${theme}/${accent} readable primary button`);
    assert.ok(contrast(first.styles.get('--primary-hover'), first.styles.get('--primary-hover-ink')) >= 4.5, `${theme}/${accent} readable primary hover`);
  }
}

 
first.api.update({ theme: 'graphite', accent: 'custom', customAccent: '#F00', customColors: { canvas: '#2b0000', sidebar: '#180000', surface: '#390000', text: '#fff', border: '#660000' }, controlRadius: 0, panelRadius: 20, lineHeight: 1.9, readingWidth: 'full' });
assert.equal(first.styles.get('--accent'), '#ff0000');
assert.equal(first.styles.get('--bg-canvas'), '#2b0000');
assert.equal(first.styles.get('--bg-sidebar'), '#180000');
assert.equal(first.styles.get('--border'), '#660000');
assert.equal(first.styles.get('--txt-1'), '#ffffff');
assert.equal(first.styles.get('--control-radius'), '0px');
assert.equal(first.styles.get('--panel-radius'), '20px');
assert.equal(first.styles.get('--chat-line-height'), '1.9');
assert.equal(first.styles.get('--reading-width'), '100%');
assert.ok(contrast(first.styles.get('--accent'), first.styles.get('--accent-ink')) >= 4.5);
const customRestart = boot({ saved: first.values.get(preferenceKey) });
assert.equal(customRestart.styles.get('--bg-canvas'), '#2b0000');
assert.equal(customRestart.api.getSettings().panelRadius, 20);
const copy = first.api.getSettings(); copy.customColors.canvas = '#abcdef';
assert.equal(first.api.getSettings().customColors.canvas, '#2b0000');
first.api.update({ customColors: { canvas: '#ffffff', sidebar: '#eeeeee', surface: '#ffffff', text: '#202024' } });
assert.equal(first.element.dataset.colorScheme, 'light', 'custom bright palettes also use dark provider icons');
assert.ok(css.includes('html[data-color-scheme="light"] img[src$="/codex.svg"]'));

 
const migrated = boot({ saved: JSON.stringify({ theme: 'system', density: 'compact', textSize: 17, snow: true, cat: true }) });
assert.equal(migrated.api.getSettings().theme, 'graphite');
assert.equal(migrated.api.getSettings().density, 'compact');
assert.equal(JSON.parse(migrated.values.get(preferenceKey)).theme, 'graphite');
assert.equal(migrated.media.has('(prefers-color-scheme: dark)'), false);
assert.equal(boot({ saved: 'not json' }).api.getSettings().theme, 'graphite');
const invalid = boot({ saved: JSON.stringify({ theme: '__proto__', accent: '__proto__', customAccent: 'url(x)', customColors: { surface: 'red;display:none', canvas: '#12345', text: '#123456' }, textSize: -200, panelRadius: 1000, lineHeight: null, snow: 'true' }) });
assert.equal(invalid.api.getSettings().snow, false);
assert.equal(invalid.api.getSettings().theme, 'graphite');
assert.equal(invalid.api.getSettings().panelRadius, 12);
assert.equal(invalid.api.getSettings().textSize, 14);
assert.equal(invalid.api.getSettings().lineHeight, 1.6);
assert.equal(Object.keys(invalid.api.getSettings().customColors).join(','), 'text');
const blocked = boot({ blocked: true });
assert.doesNotThrow(() => blocked.api.update({ theme: 'light', textSize: 17 }));
assert.equal(blocked.styles.get('--chat-font-size'), '17px');

 
const efficient = boot();
const before = { ...efficient.counts };
efficient.api.update({ theme: 'graphite', textSize: 14 });
assert.deepEqual(efficient.counts, before);
efficient.api.update({ textSize: 15 });
assert.equal(efficient.counts.styles - before.styles, 2, 'only chat/code sizes are changed');
assert.equal(efficient.counts.storage - before.storage, 1, 'effect keys are not rewritten for typography');
const countBeforePreview = efficient.counts.preferences;
efficient.api.update({ textSize: 16 }, { preview: true });
efficient.api.update({ textSize: 17 }, { preview: true });
assert.equal(efficient.counts.preferences, countBeforePreview, 'slider previews wait to persist');
efficient.flushTimers();
assert.equal(efficient.counts.preferences, countBeforePreview + 1, 'a preview burst produces one settings write');
assert.equal(JSON.parse(efficient.values.get(preferenceKey)).textSize, 17);

 
efficient.api.update({ theme: 'crimson' });
assert.equal(efficient.element.dataset.themeTransition, 'on');
efficient.api.update({ customAccent: '#aa3344', accent: 'custom' }, { preview: true });
assert.equal(efficient.element.dataset.themeTransition, undefined, 'live picker input never chases a color tween');
efficient.api.update({ theme: 'light' });
assert.equal(efficient.element.dataset.themeTransition, 'on');
efficient.api.update({ motion: 'reduced' });
assert.equal(efficient.element.dataset.themeTransition, undefined);

 
const motion = migrated.media.get('(prefers-reduced-motion: reduce)');
motion.matches = true; motion.change();
assert.equal(migrated.element.dataset.motion, 'reduced');
assert.equal(migrated.values.get('k1tty_winter'), '0');
assert.equal(migrated.values.get('k1tty_cat'), '0');
assert.equal(migrated.api.getSettings().snow, true);
migrated.api.update({ motion: 'full' });
assert.equal(migrated.element.dataset.motion, 'full');
assert.equal(migrated.values.get('k1tty_winter'), '1');
assert.equal(migrated.values.get('k1tty_cat'), '1');
console.log('Appearance tests passed: preset/migration, custom colors and contrast, typography/radius, persistence, no-op/preview efficiency, reduced motion.');
