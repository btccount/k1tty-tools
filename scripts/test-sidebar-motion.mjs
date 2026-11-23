import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../static/app.js', import.meta.url), 'utf8');
const start = source.indexOf('function isMobile()');
const end = source.indexOf('$("#brandHome").onclick', start);
assert(start >= 0 && end > start);
const drawerSource = source.slice(start, end);

function boot(width = 1280) {
  let now = 0, nextId = 0, reduced = false;
  const timers = new Map(), frames = new Map();
  class Events {
    listeners = new Map();
    addEventListener(type, callback) {
      const list = this.listeners.get(type) || [];
      list.push(callback);
      this.listeners.set(type, list);
    }
    fire(type, event = {}) {
      for (const callback of this.listeners.get(type) || []) callback(event);
    }
  }
  const document = new Events();
  const element = (parent = null) => {
    const classes = new Set(), attrs = new Map();
    let inert = false;
    const node = Object.assign(new Events(), {
      parent, isConnected: true,
      classList: {
        contains: value => classes.has(value),
        add: (...values) => values.forEach(value => classes.add(value)),
        remove: (...values) => values.forEach(value => classes.delete(value)),
        toggle(value, force) {
          const next = force ?? !classes.has(value);
          if (next) classes.add(value); else classes.delete(value);
          return next;
        },
      },
      contains(node) { for (; node; node = node.parent) if (node === this) return true; return false; },
      setAttribute: (key, value) => attrs.set(key, value),
      getAttribute: key => attrs.get(key),
      focus() {
        for (let node = this; node; node = node.parent) if (node.inert) return;
        document.activeElement = this;
      },
    });
    Object.defineProperty(node, 'inert', {
      get() { return inert; },
      set(value) {
        inert = value;
        if (value && this.contains(document.activeElement)) document.activeElement = document.body;
      },
    });
    return node;
  };
  document.body = element();
  document.activeElement = document.body;
  document.visibilityState = 'visible';
  document.documentElement = { dataset: { motion: 'full' } };
  const root = element(document.body), drawer = element(root), inner = element(drawer);
  const open = element(root), close = element(inner), search = element(inner), backdrop = element(root), content = element(root);
  const elements = new Map([['#appRoot', root], ['#drawer', drawer], ['.drawer-inner', inner], ['#drawerOpenBtn', open], ['#drawerCloseBtn', close], ['#drawerBackdrop', backdrop]]);
  const window = Object.assign(new Events(), { innerWidth: width, K1ttyAppearance: { isReducedMotion: () => reduced } });
  const context = vm.createContext({
    document, window, $: selector => elements.get(selector),
    setTimeout(callback, delay) { const id = ++nextId; timers.set(id, { callback, at: now + delay }); return id; },
    clearTimeout: id => timers.delete(id),
    requestAnimationFrame(callback) { const id = ++nextId; frames.set(id, callback); return id; },
  });
  vm.runInContext(drawerSource + '\nglobalThis.toggle = toggleDrawer;', context);
  return {
    root, drawer, inner, open, close, search, content, document, window, timers, frames, toggle: context.toggle,
    endTransition() { inner.fire('transitionend', { target: inner, propertyName: 'opacity' }); },
    advance(ms) {
      now += ms;
      for (const [id, timer] of [...timers]) if (timer.at <= now) { timers.delete(id); timer.callback(); }
    },
    resize(value) {
      window.innerWidth = value;
      window.fire('resize');
      for (const [id, callback] of [...frames]) { frames.delete(id); callback(); }
    },
    reduce(value) { reduced = value; window.fire('k1tty:appearancechange'); },
  };
}

const normal = boot();
assert.equal(normal.drawer.inert, false);
normal.close.focus();
normal.toggle(false);
assert(normal.root.classList.contains('drawer-closing'));
assert(!normal.root.classList.contains('drawer-collapsed'), 'The panel keeps its layout width during the exit');
assert(normal.drawer.inert, 'Exiting controls cannot receive input');
assert.equal(normal.open.getAttribute('aria-expanded'), 'false');
normal.advance(159);
assert(!normal.root.classList.contains('drawer-collapsed'));
normal.endTransition();
assert(normal.root.classList.contains('drawer-collapsed'));
assert(!normal.root.classList.contains('drawer-closing'));
assert.equal(normal.timers.size, 0);
assert.equal(normal.document.activeElement, normal.open, 'Focus returns to the reveal control after the exit');
normal.open.onclick();
assert(!normal.root.classList.contains('drawer-collapsed'));
assert(!normal.drawer.inert);
assert.equal(normal.open.getAttribute('aria-expanded'), 'true');
assert.equal(normal.document.activeElement, normal.close, 'Reopening moves focus away from the disappearing reveal control');

normal.search.focus();
normal.toggle(false);
normal.toggle();
assert(!normal.root.classList.contains('drawer-closing'), 'A repeated toggle reverses the pending close');
assert(!normal.root.classList.contains('drawer-collapsed'));
assert(!normal.drawer.inert);
assert.equal(normal.document.activeElement, normal.search);
normal.endTransition();
normal.advance(500);
assert(!normal.root.classList.contains('drawer-collapsed'), 'Canceled completion cannot close a reopened panel');

normal.toggle(false);
normal.content.focus();
normal.advance(200);
assert(normal.root.classList.contains('drawer-collapsed'), 'A timer settles when transitionend is unavailable');
assert.equal(normal.document.activeElement, normal.content, 'Completion does not steal newly moved focus');
normal.toggle(true);
normal.toggle(false);
normal.reduce(true);
assert(normal.root.classList.contains('drawer-collapsed'));
assert.equal(normal.timers.size, 0, 'Reduced motion cancels the pending motion clock');
normal.toggle(true);
normal.toggle(false);
assert(normal.root.classList.contains('drawer-collapsed'), 'Reduced motion closes without a delayed phase');
assert.equal(normal.timers.size, 0);
normal.reduce(false);
normal.toggle(true);
normal.toggle(false);
normal.document.visibilityState = 'hidden';
normal.document.fire('visibilitychange');
assert(normal.root.classList.contains('drawer-collapsed'));
assert.equal(normal.timers.size, 0);

const responsive = boot();
responsive.toggle(false);
responsive.resize(800);
assert(!responsive.root.classList.contains('drawer-closing'));
assert(responsive.root.classList.contains('drawer-collapsed'));
assert(responsive.drawer.inert);
assert.equal(responsive.timers.size, 0);
responsive.toggle(true);
assert(responsive.root.classList.contains('drawer-mobile-open'));
assert(!responsive.drawer.inert);
responsive.resize(1280);
assert(!responsive.root.classList.contains('drawer-mobile-open'));
assert(responsive.root.classList.contains('drawer-collapsed'), 'Desktop state survives an overlay layout');
assert(responsive.drawer.inert);
responsive.toggle(true);
responsive.close.focus();
responsive.resize(800);
assert(responsive.drawer.inert);
responsive.resize(1280);
assert(!responsive.root.classList.contains('drawer-collapsed'));
assert(!responsive.drawer.inert);
assert.equal(responsive.document.activeElement, responsive.close, 'Returning to an expanded desktop panel keeps focus visible');

const mobile = boot(800);
assert(mobile.drawer.inert, 'Off-screen mobile controls are not keyboard reachable');
mobile.open.focus();
mobile.open.onclick();
assert.equal(mobile.document.activeElement, mobile.close, 'Opening the overlay moves focus inside it');
mobile.search.focus();
mobile.toggle(false);
assert(!mobile.root.classList.contains('drawer-mobile-open'));
assert(mobile.drawer.inert);
assert.equal(mobile.document.activeElement, mobile.open);
assert.equal(mobile.timers.size, 0);

const css = readFileSync(new URL('../static/style.css', import.meta.url), 'utf8');
assert.match(css, /\.app\.drawer-collapsed \.topbar\s*\{\s*padding-left:\s*8px;/);
assert.match(css, /\.app\.drawer-closing \.drawer-inner\s*\{[^}]*opacity:\s*0;/);
assert.match(css, /\.app\.drawer-collapsed #drawerOpenBtn\s*\{[^}]*width:\s*32px;/);
assert(!/transition:\s*(?:width|height|margin)/.test(css));
console.log('PASS: actual drawer close/reopen, cancellation, focus, reduced motion, background/viewport settlement and aligned 32px reveal control. No browser geometry claim.');
