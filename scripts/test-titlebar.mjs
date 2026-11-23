import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const html = readFileSync(new URL('../static/index.html', import.meta.url), 'utf8');
const css = readFileSync(new URL('../static/style.css', import.meta.url), 'utf8');
const conf = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
const acl = JSON.parse(readFileSync(new URL('../src-tauri/capabilities/main.json', import.meta.url), 'utf8'));

assert.match(html, /class="topbar" id="windowTitlebar" data-tauri-drag-region/);
assert.match(html, /class="drawer-head" data-tauri-drag-region/);
assert.match(html, /class="spacer" data-tauri-drag-region/);
assert.match(html, /class="window-chrome-controls"/);
assert.doesNotMatch(html, /class="window-titlebar"/);
assert.equal((html.match(/class="brand-title"/g) || []).length, 1, 'Only original sidebar brand remains');
assert.doesNotMatch(html, /id="langSwitch"/);
const settingsPosition = html.indexOf('id="appearanceBtn"');
assert(settingsPosition > html.indexOf('class="drawer-foot"') && settingsPosition < html.indexOf('</aside>'));
assert(html.indexOf('id="windowClose"') < html.indexOf('</header>', html.indexOf('id="windowTitlebar"')));

assert.match(html, /id="windowMinimize"/);
assert.match(html, /id="windowMaximize"/);
assert.match(html, /id="windowClose"/);
assert.match(css, /\.window-chrome-controls \{/);
assert.match(css, /\.topbar-brand-fallback \{/);
assert.doesNotMatch(css, /--window-titlebar-height/);
assert.doesNotMatch(css, /\.window-titlebar \{/);
assert.equal(conf.app.windows[0].decorations, false);
const rustMain = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
assert.match(cargo, /tauri-plugin-single-instance\s*=\s*"2"/);
const singleInstance = rustMain.indexOf('.plugin(tauri_plugin_single_instance::init(');
const dialogPlugin = rustMain.indexOf('.plugin(tauri_plugin_dialog::init(');
assert(singleInstance > 0 && singleInstance < dialogPlugin, 'Single-instance plugin must initialize before others');
for (const action of ['window.show()', 'window.unminimize()', 'window.set_focus()']) {
  assert(rustMain.includes(action), `Missing second-launch restore: ${action}`);
}
for (const p of ['core:window:allow-minimize', 'core:window:allow-close', 'core:window:allow-toggle-maximize', 'core:window:allow-is-maximized', 'core:window:allow-start-dragging', 'core:window:allow-start-resize-dragging']) {
  assert(acl.permissions.includes(p), `Missing Tauri permission: ${p}`);
}

const log = [];
let maximized = false;
let fullscreen = false;
let resizeCb;
const makeElement = (id) => ({
  id,
  dataset: { resizeDirection: id === 'corner' ? 'SouthEast' : undefined },
  handlers: {},
  classList: { toggle: (...args) => log.push(['class', id, ...args]) },
  attributes: {},
  addEventListener(name, callback) { this.handlers[name] = callback; },
  setAttribute(name, val) { this.attributes[name] = val; },
});
const elements = new Map(['windowTitlebar', 'windowMinimize', 'windowMaximize', 'windowClose', 'corner'].map((id) => [id, makeElement(id)]));
const handlers = {};
const nativeWindow = {
  async isMaximized() { return maximized; },
  async isFullscreen() { return fullscreen; },
  async minimize() { log.push('minimize'); },
  async close() { log.push('close'); },
  async toggleMaximize() { maximized = !maximized; log.push('maximize'); resizeCb?.(); },
  async setFullscreen(value) { fullscreen = value; log.push(`fullscreen:${value}`); resizeCb?.(); },
  async startResizeDragging(direction) { log.push(`resize:${direction}`); },
  async onResized(callback) { resizeCb = callback; return () => {}; },
  async onFocusChanged() { return () => {}; },
};
const fakeDocument = {
  body: { classList: { toggle: () => {} } },
  getElementById: (id) => elements.get(id),
  querySelectorAll: () => [elements.get('corner')],
  addEventListener(name, cb) { handlers[name] = cb; },
};
const win = {
  __TAURI__: { core: { invoke: async () => ({}) }, event: { listen: async () => () => {} }, window: { getCurrentWindow: () => nativeWindow } },
  dispatchEvent: () => {},
};
runInNewContext(readFileSync(new URL('../static/desktop.js', import.meta.url), 'utf8'), {
  window: win,
  document: fakeDocument,
  console,
  Uint8Array,
  CustomEvent: class CustomEvent { constructor(type, opts) { this.type = type; this.detail = opts.detail; } },
});

const runClick = async (id) => {
  await elements.get(id).handlers.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
};
await runClick('windowMinimize');
await runClick('windowMaximize');
assert.equal(elements.get('windowMaximize').dataset.maximized, 'true');
await runClick('windowMaximize');
assert.equal(elements.get('windowMaximize').dataset.maximized, 'false');
await runClick('windowClose');
win.K1ttyDesktop.setWindowLabels({ minimize: 'Свернуть', maximize: 'Развернуть', restore: 'Восстановить', close: 'Закрыть' });
assert.equal(elements.get('windowMinimize').title, 'Свернуть');
assert.equal(elements.get('windowClose').ariaLabel, 'Закрыть');
handlers.keydown({ key: 'F11', altKey: false, ctrlKey: false, metaKey: false, repeat: false, preventDefault: () => {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(fullscreen, true);
assert.equal(elements.get('windowMaximize').title, 'Восстановить');
handlers.keydown({ key: 'F11', altKey: false, ctrlKey: false, metaKey: false, repeat: false, preventDefault: () => {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(fullscreen, false);
elements.get('corner').handlers.pointerdown({ button: 0, preventDefault: () => {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(log.filter((x) => typeof x === 'string'), ['minimize', 'maximize', 'maximize', 'close', 'fullscreen:true', 'fullscreen:false', 'resize:SouthEast']);
console.log('PASS: Unified header, single-instance callback, K1tty window controls / locale / fullscreen / resize / Tauri ACL');
