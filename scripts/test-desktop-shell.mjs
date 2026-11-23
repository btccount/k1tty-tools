import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const listeners = new Map();
const dispatched = [];
let modalOpen = false;
const document = {
  documentElement: { dataset: {} },
  addEventListener(type, callback) {
    if (!listeners.has(type)) listeners.set(type, []);
    listeners.get(type).push(callback);
  },
  querySelector: () => modalOpen ? {} : null,
  getElementById: () => null,
};
class MockCustomEvent {
  constructor(type, init = {}) {
    this.type = type;
    this.detail = init.detail;
  }
}
const win = {
  dispatchEvent(ev) {
    dispatched.push(ev.type);
  },
};
runInNewContext(readFileSync(new URL('../static/desktop.js', import.meta.url), 'utf8'), {
  document, window: win, CustomEvent: MockCustomEvent, navigator: { userAgent: 'Windows NT 10.0' }, console,
});

function fire(type, overrides = {}) {
  const event = {
    key: '', code: '', button: 0, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false,
    target: { closest: () => null }, defaultPrevented: false, propagationStopped: false,
    preventDefault() { this.defaultPrevented = true; },
    stopPropagation() { this.propagationStopped = true; },
    ...overrides,
  };
  for (const callback of listeners.get(type) || []) callback(event);
  return event;
}
for (const key of ['F5', 'F12', 'BrowserBack', 'BrowserForward']) {
  assert.equal(fire('keydown', { key }).defaultPrevented, true, `${key} should not invoke the browser`);
}
for (const code of ['KeyR', 'KeyS', 'KeyP', 'KeyU', 'KeyL', 'Equal', 'Minus', 'Digit0']) {
  for (const modifier of ['ctrlKey', 'metaKey']) {
    assert.equal(fire('keydown', { code, [modifier]: true }).defaultPrevented, true, `${modifier}+${code}`);
  }
}
assert.equal(fire('keydown', { key: 'к', code: 'KeyR', ctrlKey: true }).defaultPrevented, true, 'Physical shortcuts work with Cyrillic layout');
assert.equal(fire('keydown', { key: 'c', code: 'KeyC', ctrlKey: true, shiftKey: true }).defaultPrevented, true);
const input = { closest: (selector) => selector.includes('textarea') ? {} : null };
for (const code of ['KeyC', 'KeyV', 'KeyX', 'KeyA', 'KeyZ']) {
  assert.equal(fire('keydown', { code, ctrlKey: true, target: input }).defaultPrevented, false, `Keep editing: ${code}`);
}
for (const key of ['F11', 'PageUp', 'PageDown', 'Home', 'End', 'Escape']) {
  assert.equal(fire('keydown', { key }).defaultPrevented, false, `Keep app/navigation key ${key}`);
}
assert.equal(fire('keydown', { key: 'Backspace', target: input }).defaultPrevented, false);
assert.equal(fire('keydown', { key: 'Backspace' }).defaultPrevented, true);
assert.equal(fire('contextmenu').defaultPrevented, true);
assert.equal(fire('contextmenu').propagationStopped, false, 'Conversation menu still receives right click');
assert.equal(fire('wheel').defaultPrevented, false);
assert.equal(fire('wheel', { ctrlKey: true }).defaultPrevented, true);
assert.equal(fire('drop', { dataTransfer: { files: [{}] } }).defaultPrevented, true);
assert.equal(fire('drop', { dataTransfer: { files: [{}] } }).propagationStopped, false, 'Import drop zone still receives files');
assert.equal(fire('keydown', { code: 'KeyF', ctrlKey: true }).defaultPrevented, true);
assert.equal(fire('keydown', { code: 'KeyF', ctrlKey: true, shiftKey: true }).defaultPrevented, true);
assert.equal(fire('keydown', { code: 'KeyK', ctrlKey: true }).defaultPrevented, true);
assert.equal(fire('keydown', { code: 'KeyA', ctrlKey: true }).defaultPrevented, true);
assert.deepEqual(dispatched, ['k1tty:find', 'k1tty:globalfind', 'k1tty:palette', 'k1tty:selectall']);
modalOpen = true;
fire('keydown', { code: 'KeyF', metaKey: true });
assert.equal(dispatched.length, 4, 'Search must not steal focus from a modal');

for (const file of ['tauri.conf.json', 'tauri.linux.conf.json', 'tauri.macos.conf.json']) {
  const config = JSON.parse(readFileSync(new URL(`../src-tauri/${file}`, import.meta.url), 'utf8'));
  assert.equal(config.app.windows[0].devtools, false);
  assert.equal(config.app.windows[0].zoomHotkeysEnabled, false);
  assert.equal(config.app.windows[0].allowLinkPreview, false);
  assert.equal(config.app.windows[0].browserAcceleratorKeys, undefined, 'Do not use a config key absent from pinned Tauri 2.12.7');
}
console.log('PASS: Desktop browser defaults suppressed; app menus, import, text editing, search and window shortcuts preserved');
