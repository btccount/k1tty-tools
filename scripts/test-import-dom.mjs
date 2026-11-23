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
const document = window.document;
const context = vm.createContext(window);
const run = code => vm.runInContext(code, context);
const tick = () => new Promise(resolve => setTimeout(resolve, 0));
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const pack = (id, title = `Диалог ${id}`) => ({
  token: `pack-${id}`,
  info: { version: 1, source_provider: 'claude', title, preset: 'ultra', include_tools: false,
    entry_count: 4, packed_bytes: 3072, raw_bytes: 425000, ratio_pct: 99.3 },
});
const inspections = new Map();
const inspected = [];
const imports = [];
const calls = [];
const errors = [];
let failRefresh = false;
const providers = Object.fromEntries(['antigravity', 'antigravity_cli', 'codex', 'claude', 'opencode']
  .map(id => [id, { available: true, writable: true, running: [], appdata: '/test/' + id }]));
const selectFile = (id, bufferPromise) => {
  window.testImportFile = { name: `Диалог ${id}.k1tty`, size: 3072,
    arrayBuffer: () => bufferPromise || Promise.resolve(new Uint8Array([id]).buffer) };
  return run('handleImportFile(window.testImportFile)');
};
const openImport = () => document.getElementById('ovImportBtn').click();
const applyImport = () => document.getElementById('importApplyBtn').onclick();
window.addEventListener('error', event => errors.push(event.message));
window.localStorage.setItem('k1tty_lang', 'ru');
document.write(read('static/index.html').replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '').replace(/<link\b[^>]*>/gi, ''));
const style = document.createElement('style');
style.textContent = read('static/style.css');
document.head.append(style);
window.K1ttyDesktop = {
  connect: async callback => { window.testEmit = callback; },
  setWindowLabels() {},
  writeText: async () => {},
  rpc: async (op, provider, data) => {
    calls.push({ op, provider, data });
    if (op === 'state' && failRefresh) throw new Error('Archive refresh failed after successful import');
    if (op === 'state') return { providers, settings: { auto_backup: false } };
    if (op === 'convs') return { convs: [] };
    if (op === 'backups_summary') return { total_count: 0, total_bytes: 0, providers: [] };
    return {};
  },
  inspectPack: bytes => {
    const id = new Uint8Array(bytes)[0];
    inspected.push(id);
    return inspections.get(id)?.promise || Promise.resolve(pack(id));
  },
  importPack: (...args) => {
    const request = deferred();
    imports.push({ args, ...request });
    return request.promise;
  },
};
window.K1ttyWinter = { isEnabled: () => false, setEnabled() {} };
window.K1ttyCat = { isEnabled: () => false, setEnabled() {} };

try {
  run(read('static/appearance.js'));
  run(read('static/desktop.js'));
  run(read('static/app.js'));
  document.dispatchEvent(new window.Event('DOMContentLoaded'));
  await tick();
  await tick();

  const modal = document.getElementById('importModal');
  const apply = document.getElementById('importApplyBtn');
  const error = document.getElementById('importError');
  const detail = document.getElementById('importErrorDetails');
  assert(error && detail, 'Import errors have a persistent place inside the modal');
  openImport();
  await selectFile(1);
  assert(modal.classList.contains('open'));
  assert.equal(apply.disabled, false);
  assert.equal(document.getElementById('importTitleInput').value, 'Диалог 1');
  const failed = applyImport();
  assert.equal(imports.length, 1);
  assert.equal(apply.disabled, true);
  for (const input of document.querySelectorAll('#importTargetGrid button, #importDropZone, #importFileInput, #importTitleInput, #importIncludeTools')) {
    assert.equal(input.disabled, true, 'The in-flight import keeps its file, target and options fixed');
  }
  await applyImport();
  assert.equal(imports.length, 1, 'Repeated submit cannot start a second native write');
  document.querySelector('#importTargetGrid [data-pick-prov="codex"]').click();
  await selectFile(2);
  assert.deepEqual(inspected, [1], 'Changing the file while writing does not create a new upload');
  assert.deepEqual(imports[0].args, ['pack-1', 'claude', 'Диалог 1', false]);
  const pathError = 'Не удалось сохранить файл «C:\\Users\\Алексей Иванов\\.claude\\projects\\C--Users-Алексей\\диалог.jsonl»: Системе не удается найти указанный путь. (os error 3)';
  imports[0].reject(new Error(pathError));
  await failed;
  assert(modal.classList.contains('open'), 'The failed import remains open for recovery');
  assert(!error.classList.contains('hidden') && !error.hidden);
  assert.equal(detail.textContent, pathError, 'Windows separators, spaces and Cyrillic remain readable');
  assert.equal(window.getComputedStyle(detail).userSelect, 'text', 'The path can be selected and copied');
  assert.equal(apply.disabled, false, 'The same inspected pack can be retried');
  assert.equal(document.getElementById('importTitleInput').disabled, false);
  assert.equal(document.querySelector('#importTargetGrid .active').dataset.pickProv, 'claude');
  const retry = applyImport();
  assert.equal(imports.length, 2);
  assert.deepEqual(imports[1].args, imports[0].args, 'Retry preserves the valid upload token and target');
  assert(error.classList.contains('hidden') || error.hidden, 'Retry clears the previous failure');
  const malicious = 'Не удалось записать <img src=x onerror="window.injected=true"> в C:\\Архив чатов\\history.jsonl';
  imports[1].reject(new Error(malicious));
  await retry;
  assert.equal(detail.textContent, malicious);
  assert.equal(detail.querySelector('*'), null, 'Native errors are text, never executable HTML');
  assert.equal(window.injected, undefined);
  const success = applyImport();
  const beforeRefresh = calls.length;
  imports[2].resolve({ ok: true, result: { provider: 'claude' } });
  await success;
  assert(!modal.classList.contains('open'));
  assert(calls.length > beforeRefresh, 'Successful import refreshes the local archive');

  openImport();
  assert(error.classList.contains('hidden') || error.hidden, 'A new dialog does not inherit the previous error');
  const oldPreview = deferred();
  const newPreview = deferred();
  inspections.set(3, oldPreview);
  inspections.set(4, newPreview);
  const oldPending = selectFile(3);
  await tick();
  const newPending = selectFile(4);
  await tick();
  newPreview.resolve(pack(4));
  await newPending;
  oldPreview.resolve(pack(3));
  await oldPending;
  assert.equal(document.getElementById('importDropTitle').textContent, 'Диалог 4.k1tty');
  assert.equal(document.getElementById('importTitleInput').value, 'Диалог 4', 'A late preview cannot replace the selected file');
  const latestImport = applyImport();
  assert.equal(imports.at(-1).args[0], 'pack-4');
  imports.at(-1).resolve({ ok: true, result: { provider: 'claude' } });
  await latestImport;

  openImport();
  const lateError = deferred();
  inspections.set(5, lateError);
  const staleFailure = selectFile(5);
  await tick();
  await selectFile(6);
  lateError.reject(new Error('Stale preview failure'));
  await staleFailure;
  assert.equal(document.getElementById('importTitleInput').value, 'Диалог 6');
  assert.equal(apply.disabled, false, 'An old failure cannot invalidate the current pack');
  assert(error.classList.contains('hidden') || error.hidden);

  const slowBuffer = deferred();
  const staleBuffer = selectFile(7, slowBuffer.promise);
  await selectFile(8);
  slowBuffer.resolve(new Uint8Array([7]).buffer);
  await staleBuffer;
  assert.equal(document.getElementById('importTitleInput').value, 'Диалог 8');
  assert(!inspected.includes(7), 'Superseded file reading never reaches native inspection');

  const previousWindow = deferred();
  inspections.set(9, previousWindow);
  const previousPending = selectFile(9);
  await tick();
  document.querySelector('[data-close="importModal"]').click();
  openImport();
  await selectFile(10);
  previousWindow.resolve(pack(9));
  await previousPending;
  assert.equal(document.getElementById('importTitleInput').value, 'Диалог 10', 'A previous modal cannot write into a newly opened import');

  const abandoned = applyImport();
  const abandonedRequest = imports.at(-1);
  document.querySelector('[data-close="importModal"]').click();
  openImport();
  await selectFile(11);
  const currentTitle = document.getElementById('importTitleInput').value;
  abandonedRequest.reject(new Error('Old import failure'));
  await abandoned;
  assert(modal.classList.contains('open'));
  assert.equal(document.getElementById('importTitleInput').value, currentTitle);
  assert.equal(apply.disabled, false);
  assert(error.classList.contains('hidden') || error.hidden, 'An old write failure cannot overwrite the new import state');

  const written = applyImport();
  const writesBeforeRefreshFailure = imports.length;
  failRefresh = true;
  imports.at(-1).resolve({ ok: true, result: { provider: 'claude' } });
  await written;
  assert(!modal.classList.contains('open'), 'A refresh failure does not turn a successful write into a failed import');
  assert(error.classList.contains('hidden') || error.hidden);
  assert.equal(apply.disabled, true, 'A successful import consumes its upload token');
  await applyImport();
  assert.equal(imports.length, writesBeforeRefreshFailure, 'Refreshing a completed import cannot create a duplicate chat');
  failRefresh = false;
  assert(!document.getElementById('topLoader').classList.contains('active'), 'All completed requests release their loading state');
  assert.deepEqual(errors, []);
  console.log('PASS: persistent import paths, text-only errors, retry without re-upload, duplicate-write guard, pending controls, preview and modal races, no repeat write after refresh failure. Node DOM only; no native filesystem claim.');
} finally {
  await window.happyDOM.close();
}
