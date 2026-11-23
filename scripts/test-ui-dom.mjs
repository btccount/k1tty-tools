 
 
 
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const domModule = process.env.K1TTY_QA_MODULES
  ? pathToFileURL(resolve(process.env.K1TTY_QA_MODULES, 'happy-dom/lib/index.js')).href
  : 'happy-dom';
const { Window } = await import(domModule);
const read = path => readFileSync(resolve(root, path), 'utf8');
const window = new Window({ url: 'https://k1tty.test/', width: 1280, height: 840,
  settings: { disableCSSFileLoading: true, disableJavaScriptFileLoading: true } });
const context = vm.createContext(window);
const run = code => vm.runInContext(code, context);
const document = window.document;
const errors = [];
window.addEventListener('error', event => errors.push(event.message));
window.localStorage.setItem('k1tty_lang', 'ru');
let html = read('static/index.html').replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '').replace(/<link\b[^>]*>/gi, '');
document.write(html);
const calls = [];
let failSettings = false;
const providers = Object.fromEntries(['antigravity', 'antigravity_cli', 'codex', 'claude', 'opencode'].map(id => [id, { available: true, writable: true, running: [], appdata: '/test/' + id }]));
window.K1ttyDesktop = {
  connect: async callback => { window.testEmit = callback; },
  setWindowLabels() {},
  writeText: async () => {},
  rpc: async (op, provider, data) => {
    calls.push({ op, provider, data });
    if (op === 'state') return { providers, settings: { auto_backup: true } };
    if (op === 'convs') return { convs: [{ id: 'a', provider: 'claude', title: 'Тестовый диалог', writable: true, size: 12345, mtime: '2026-11-20 12:00' }] };
    if (op === 'backups_summary') return { total_count: 0, total_bytes: 0, providers: [] };
    if (op === 'set_settings') {
      await new Promise(resolve => setTimeout(resolve, 10));
      if (failSettings) throw new Error('Test settings write failed');
      return { settings: { auto_backup: data.auto_backup } };
    }
    return {};
  },
};
let snow = false, cat = false;
window.K1ttyWinter = { isEnabled: () => snow, setEnabled: value => { snow = value; } };
window.K1ttyCat = { isEnabled: () => cat, setEnabled: value => { cat = value; } };
try {
  run(read('static/appearance.js'));
  run(read('static/desktop.js'));
  run(read('static/app.js'));
  document.dispatchEvent(new window.Event('DOMContentLoaded'));
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.equal(document.querySelectorAll('#ovStats .ov-stat-card').length, 4);
  assert.equal(document.querySelector('#ovBackupsPanel').parentElement.className, 'ov-columns');
  assert.equal(document.querySelectorAll('.switch-state, .ov-stat-tag').length, 0);
  assert.equal(document.querySelectorAll('#ovBkGrid .ov-bk-card').length, 0, 'Do not render five empty backup cards');
  for (const lang of ['ru', 'uk', 'en']) {
    run(`setLang('${lang}')`);
    assert(document.getElementById('welcomeSign').textContent.includes('quesada & k1ttyf <3'));
    assert(!document.getElementById('welcomeSign').textContent.includes('котик'));
  }
  run("setLang('ru')");

   
  const stat = document.querySelector('#ovStats .ov-stat-card');
  const providerRow = document.querySelector('#provGrid .prov-row');
  const pending = run('setAutoBackup(false)');
  assert(document.getElementById('ovAutoBackupToggle').disabled);
  assert.equal(document.getElementById('ovAutoBackupToggle').checked, false);
  assert.equal(document.getElementById('bkAutoBackupToggle').checked, false);
  await pending;
  assert.equal(document.querySelector('#ovStats .ov-stat-card'), stat);
  assert.equal(document.querySelector('#provGrid .prov-row'), providerRow);
  assert.equal(document.getElementById('ovAutoBackupToggle').disabled, false);
  failSettings = true;
  await run('setAutoBackup(true)');
  assert.equal(document.getElementById('ovAutoBackupToggle').checked, false, 'Failed save rolls back the switch');
  assert.equal(document.getElementById('bkAutoBackupToggle').checked, false);
  failSettings = false;
  window.testEmit({ type: 'settings', settings: { auto_backup: true } });
  assert(document.getElementById('bkAutoBackupToggle').checked);
  assert.equal(document.querySelector('#ovStats .ov-stat-card'), stat);

   
  const appearance = document.getElementById('appearanceBtn');
  assert(appearance.closest('.drawer-foot'), 'Settings live in the sidebar footer');
  assert.equal(document.querySelector('.top-controls #appearanceBtn'), null);
  assert.equal(document.getElementById('langSwitch'), null, 'Language is not duplicated in the titlebar');
  assert.equal(document.getElementById('catToggleBtn'), null, 'Cat is an option inside settings');
  assert.equal(document.getElementById('threadMeta'), null, 'The loaded counter is removed from the titlebar');
  appearance.focus(); appearance.click();
  const dialog = document.getElementById('appearanceDialog');
  assert(dialog.open);
  assert.equal(document.getElementById('appearanceTitle').textContent, 'Настройки');
  assert.equal(dialog.querySelector('input[data-pref="theme"][value="system"]'), null);
  assert(dialog.querySelector('input[data-pref="theme"][value="crimson"]'));
  assert.equal(dialog.querySelectorAll('[data-appearance-language]').length, 3);
  const callsBeforeAppearance = calls.length;
  const light = dialog.querySelector('input[data-pref="theme"][value="light"]');
  light.checked = true; light.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(document.documentElement.dataset.appearance, 'light');
  const size = document.getElementById('appearanceTextSize');
  size.value = '18'; size.dispatchEvent(new window.Event('input', { bubbles: true }));
  await new Promise(resolve => window.requestAnimationFrame(resolve));
  size.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(window.K1ttyAppearance.getSettings().textSize, 18);
  assert.equal(JSON.parse(window.localStorage.getItem('k1tty_appearance_v1')).textSize, 18);
  const crimson = dialog.querySelector('input[data-pref="theme"][value="crimson"]');
  crimson.checked = true; crimson.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(document.documentElement.dataset.appearance, 'crimson');
  const redCanvas = document.documentElement.style.getPropertyValue('--bg-canvas');
  assert.notEqual(redCanvas, '#141415', 'Red theme changes surfaces as well as its accent');
  assert.equal(dialog.querySelectorAll('input[type="color"]').length, 0);
  const canvasColor = dialog.querySelector('.appearance-hex[data-color-key="canvas"]');
  assert(canvasColor, 'Canvas has an editable color control');
  canvasColor.value = '#260b10';
  canvasColor.dispatchEvent(new window.Event('input', { bubbles: true }));
  await new Promise(resolve => window.requestAnimationFrame(resolve));
  canvasColor.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(document.documentElement.style.getPropertyValue('--bg-canvas'), '#260b10');
  assert.equal(JSON.parse(window.localStorage.getItem('k1tty_appearance_v1')).customColors.canvas, '#260b10');
   
  const surfaceColor = dialog.querySelector('.appearance-hex[data-color-key="surface"]');
  canvasColor.value = '#280d14';
  canvasColor.dispatchEvent(new window.Event('input', { bubbles: true }));
  surfaceColor.value = '#391922';
  surfaceColor.dispatchEvent(new window.Event('input', { bubbles: true }));
  await new Promise(resolve => window.requestAnimationFrame(resolve));
  surfaceColor.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(window.K1ttyAppearance.getSettings().customColors.canvas, '#280d14');
  assert.equal(window.K1ttyAppearance.getSettings().customColors.surface, '#391922');
  canvasColor.value = '#2b1018';
  canvasColor.dispatchEvent(new window.Event('input', { bubbles: true }));
  surfaceColor.value = '#3e1c26';
  surfaceColor.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(window.K1ttyAppearance.getSettings().customColors.canvas, '#2b1018', 'Settling another control preserves a pending color');
  assert.equal(window.K1ttyAppearance.getSettings().customColors.surface, '#3e1c26');
  const swatch = dialog.querySelector('[data-color-open="canvas"]');
  swatch.focus(); swatch.click();
  const picker = document.getElementById('appearanceColorPicker');
  assert(picker && !picker.hidden, 'Color selection opens inside settings');
  const pickerHex = document.getElementById('appearancePickerHex');
  pickerHex.value = '#330c18';
  pickerHex.dispatchEvent(new window.Event('input', { bubbles: true }));
  await new Promise(resolve => window.requestAnimationFrame(resolve));
  pickerHex.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(document.documentElement.style.getPropertyValue('--bg-canvas'), '#330c18');
  pickerHex.dispatchEvent(new window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  assert(picker.hidden && dialog.open, 'Escape closes the picker before settings');
  assert.equal(document.activeElement, swatch);
  const radius = document.getElementById('appearanceControlRadius');
  radius.value = '12'; radius.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert.equal(document.documentElement.style.getPropertyValue('--control-radius'), '12px');
  assert.equal(calls.length, callsBeforeAppearance, 'Appearance changes do not reload conversations');
  assert.equal(document.querySelector('#ovStats .ov-stat-card'), stat);
   
  dialog.querySelector('[data-appearance-language="en"]').click();
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(document.documentElement.lang, 'en');
  assert.equal(window.localStorage.getItem('k1tty_lang'), 'en');
  assert.equal(document.getElementById('appearanceTitle').textContent, 'Settings');
  assert.equal(calls.length, callsBeforeAppearance, 'Changing UI language does not fetch conversations');
  window.K1ttyAppearance.update({ motion: 'full', cat: false });
  const catToggle = dialog.querySelector('input[data-pref="cat"]');
  catToggle.checked = true;
  catToggle.dispatchEvent(new window.Event('change', { bubbles: true }));
  assert(window.K1ttyAppearance.getSettings().cat && cat, 'Cat option is controlled from settings');
  dialog.querySelector('[data-appearance-close]').click();
  assert(!dialog.open);
  assert.equal(document.activeElement, appearance, 'Dialog close restores the previous focus');
  window.K1ttyAppearance.update({ motion: 'reduced', snow: true });
  assert(!snow && !cat, 'Reduced motion pauses optional effects');

   
  const menu = new window.MouseEvent('contextmenu', { bubbles: true, cancelable: true });
  document.body.dispatchEvent(menu);
  assert(menu.defaultPrevented);
  const reload = new window.KeyboardEvent('keydown', { key: 'к', code: 'KeyR', ctrlKey: true, bubbles: true, cancelable: true });
  document.body.dispatchEvent(reload);
  assert(reload.defaultPrevented);
  const copy = new window.KeyboardEvent('keydown', { key: 'c', code: 'KeyC', ctrlKey: true, bubbles: true, cancelable: true });
  document.getElementById('editText').dispatchEvent(copy);
  assert(!copy.defaultPrevented, 'Native text copying remains available');
  assert.deepEqual(errors, []);
  console.log('PASS: overview, credits, backup rollback, sidebar settings, language routing, removed titlebar counter, red/custom surfaces, shape controls, persistent effects and browser-command guards. Node DOM only; no browser layout claim.');
} finally { await window.happyDOM.close(); }
