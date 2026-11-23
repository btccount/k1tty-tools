import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const domModule = process.env.K1TTY_QA_MODULES
  ? pathToFileURL(resolve(process.env.K1TTY_QA_MODULES, 'happy-dom/lib/index.js')).href
  : process.env.HAPPY_DOM_MODULE ? pathToFileURL(process.env.HAPPY_DOM_MODULE).href : 'happy-dom';
const { Window } = await import(domModule);
const read = path => readFileSync(resolve(root, path), 'utf8');
const window = new Window({ url: 'https://k1tty.test/', width: 1280, height: 900,
  settings: { disableCSSFileLoading: true, disableJavaScriptFileLoading: true } });
window.structuredClone = structuredClone;
window.history.replaceState = () => {};
const document = window.document;
const context = vm.createContext(window);
const run = code => vm.runInContext(code, context);
const tick = () => new Promise(resolve => setTimeout(resolve, 0));
const wait = async predicate => {
  for (let attempt = 0; attempt < 300; attempt++) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  throw new Error('Timed out waiting for Claude notice UI');
};
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const $ = selector => document.querySelector(selector);
const errors = [];
window.addEventListener('error', event => errors.push(event.message));
window.localStorage.setItem('k1tty_lang', 'ru');
document.write(read('static/index.html').replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '').replace(/<link\b[^>]*>/gi, ''));
const fake = read('scripts/test-transcript-async.mjs').match(/const fake = String\.raw`([\s\S]*?)`;\nconst app/)[1];
run(fake);
window.backend.delay = 0;
const baseRpc = window.K1ttyDesktop.rpc;
const catalog = ['info', 'notice', 'suggestion', 'warning', 'safety_classifier'].map(id => ({
  id, label: id, level: id === 'safety_classifier' ? 'warning' : id,
  subtype: 'informational',
  prefix: id === 'safety_classifier' ? 'Your response above was stopped by a safety classifier.' : '',
  default_text: '',
}));
const noticeRow = (key, notice_type, text) => ({
  key, uuid: key, kind: 'system', subtype: notice_type === 'safety_classifier' ? 'safeguard' : 'notice',
  notice_type, text, label: 'Уведомление Claude', actions: ['edit', 'del_single', 'cut_tail'],
});
window.backend.dialogs.A.push(noticeRow('notice-original-a', 'notice', 'Original notice'));
window.backend.dialogs.A.push(noticeRow('notice-classifier-a', 'safety_classifier', 'Your response above was stopped by a safety classifier.'));
window.backend.dialogs.B.push(noticeRow('notice-original-b', 'warning', 'Other conversation warning'));
const revisions = { A: 1, B: 1 };
const noticeCalls = [];
const infoQueue = [];
const writes = [];
let holdWrites = false;
let nextWriteError = null;
let nextInfoError = null;
const info = cid => {
  const rows = window.backend.dialogs[cid].filter(row => row.notice_type);
  return { provider: 'claude', cid, revision: `revision-${cid}-${revisions[cid]}`, writable: true, lock_reason: '',
    notice_count: rows.length,
    counts: Object.fromEntries(catalog.map(type => [type.id, rows.filter(row => row.notice_type === type.id).length])),
    types: structuredClone(catalog),
  };
};
const finishWrite = write => {
  if (write.finished) return;
  write.finished = true;
  if (write.error) {
    window.backend.emit({ type: 'job', job: { id: write.job_id, state: 'error', phase: 'error', error: write.error } });
    return;
  }
  const { op, data } = write;
  let result;
  if (op === 'add_system_notice') {
    const entry = noticeRow(`added-notice-${writes.indexOf(write)}`, data.notice_type, data.text);
    window.backend.dialogs[data.cid].push(entry);
    result = { added: 1, deleted_count: 0, changed: true, no_op: false, entry };
  } else {
    const deleted = window.backend.dialogs[data.cid].filter(row => row.notice_type);
    window.backend.dialogs[data.cid] = window.backend.dialogs[data.cid].filter(row => !row.notice_type);
    result = { added: 0, deleted_count: deleted.length, deleted_keys: deleted.map(row => row.key),
      deleted_uuids: deleted.map(row => row.uuid), changed: deleted.length > 0, no_op: deleted.length === 0 };
  }
  revisions[data.cid]++;
  result = { ...result, provider: 'claude', cid: data.cid, tid: data.cid,
    revision: `revision-${data.cid}-${revisions[data.cid]}`, backup: '/test-backup/claude-notices' };
  window.backend.emit({ type: 'job', job: { id: write.job_id, state: 'done', phase: 'done', result } });
};
window.K1ttyDesktop.rpc = async (op, provider, data = {}) => {
  if (!['system_notices_info', 'add_system_notice', 'delete_system_notices'].includes(op)) return baseRpc(op, provider, data);
  noticeCalls.push({ op, provider, data: structuredClone(data) });
  if (op === 'system_notices_info') {
    if (nextInfoError) {
      const error = nextInfoError;
      nextInfoError = null;
      throw new Error(error);
    }
    if (infoQueue.length) return infoQueue.shift().promise;
    return info(data.cid);
  }
  const write = { op, provider, data: structuredClone(data), job_id: `notice-job-${writes.length + 1}`, error: nextWriteError, finished: false };
  nextWriteError = null;
  writes.push(write);
  if (!holdWrites) setTimeout(() => finishWrite(write), 0);
  return { job_id: write.job_id };
};

try {
  run(read('static/app.js'));
  run('window.noticeTestApi = { state: STATE, openThread, openBackupResearch, openNoticeModal, applySystemNotice, deleteSystemNotices, loadMore, openCtxMenu, closeModal, transcriptMutationPending };');
  const api = window.noticeTestApi;
  await wait(() => api.state.allConvs.length === 2);
  await tick();
  await api.openThread('claude', 'A', { preserveSys: false });
  assert(!$('#threadNoticesBtn').classList.contains('hidden'), 'Claude exposes the notice action');
  assert.equal($('#threadNoticesBtn').disabled, false, 'The notice action becomes usable when transcript loading settles');
  $('#threadNoticesBtn').click();
  await wait(() => document.querySelectorAll('#noticeTypeOptions [data-notice-type]').length === catalog.length);
  assert($('#noticeModal').classList.contains('open'), 'The real thread action opens the notice editor');
  api.closeModal('noticeModal');
  api.openCtxMenu(20, 20, 'claude', 'A', $('#convList [data-cid="A"]'));
  assert(!$('#ctxNoticesBtn').classList.contains('hidden'));
  $('#ctxNoticesBtn').click();
  await wait(() => document.querySelectorAll('#noticeTypeOptions [data-notice-type]').length === catalog.length);
  assert.equal(document.querySelectorAll('#noticeTypeOptions [data-notice-type]').length, catalog.length);
  assert.equal($('#noticeModal').classList.contains('open'), true);
  assert($('#noticeSummary').textContent.includes('2'));
  api.closeModal('noticeModal');

  await api.openThread('codex', 'A', { preserveSys: false });
  assert($('#threadNoticesBtn').classList.contains('hidden'), 'Codex keeps its own actions');
  api.openCtxMenu(20, 20, 'codex', 'A', null);
  assert($('#ctxNoticesBtn').classList.contains('hidden'));
  await api.openBackupResearch('claude', 'backup-a', 'Saved copy', 'Dialog A');
  assert($('#threadNoticesBtn').classList.contains('hidden'), 'Backup browsing does not expose a live-write action');

  await api.openThread('claude', 'A', { preserveSys: false });
  await api.loadMore();
  const activeBeforeAdd = api.state.active;
  const retained = $('#chatFeed > [data-key="m-200"]');
  const retainedSidebar = $('#convList [data-cid="A"]');
  retained.querySelector('.msg-body').classList.remove('clamped');
  await api.openNoticeModal('claude', 'A');
  $('#noticeTypeOptions [data-notice-type="warning"]').click();
  const literal = '  Не запускать повторно: <img src=x onerror=alert(1)>\nC:\\Проект с пробелами\\файл  ';
  $('#noticeTextInput').value = literal;
  $('#noticeTextInput').dispatchEvent(new window.Event('input', { bubbles: true }));
  holdWrites = true;
  const adding = api.applySystemNotice();
  await wait(() => writes.length === 1);
  assert($('#noticeApplyBtn').disabled);
  assert($('#noticeDeleteBtn').disabled);
  assert(api.transcriptMutationPending());
  await api.applySystemNotice();
  assert.equal(writes.length, 1, 'Repeated submit cannot write two notices');
  assert.equal(writes[0].provider, 'claude');
  assert.equal(writes[0].data.cid, 'A');
  assert.equal(writes[0].data.notice_type, 'warning');
  assert.equal(writes[0].data.text, literal, 'Custom text is preserved without markup conversion or trimming');
  assert.equal(writes[0].data.revision, 'revision-A-1');
  assert.equal(writes[0].data.confirm, true);
  finishWrite(writes[0]);
  await adding;
  holdWrites = false;
  assert.equal(api.state.active, activeBeforeAdd, 'Notice changes refresh the active transcript without opening it again');
  assert.equal($('#chatFeed > [data-key="m-200"]'), retained, 'Unchanged transcript nodes survive an added notice');
  assert.equal(retained.querySelector('.msg-body').classList.contains('clamped'), false);
  assert.equal($('#convList [data-cid="A"]'), retainedSidebar);
  assert.equal($('#chatFeed .msg-body img'), null, 'Notice text is not interpreted as an HTML payload');
  assert.equal(api.transcriptMutationPending(), false);
  assert.equal($('#threadNoticesBtn').disabled, false, 'The notice action is restored after a mutation');

  await api.openNoticeModal('claude', 'A');
  const beforeDelete = writes.length;
  $('#noticeDeleteBtn').click();
  await tick();
  assert.equal(writes.length, beforeDelete, 'First delete click only asks for confirmation');
  $('#noticeDeleteBtn').click();
  await wait(() => writes.length === beforeDelete + 1 && !api.transcriptMutationPending() && !api.state.active.loading
    && document.querySelectorAll('#chatFeed > [data-key^="notice-"], #chatFeed > [data-key^="added-notice-"]').length === 0);
  const deleting = writes.at(-1);
  assert.equal(deleting.op, 'delete_system_notices');
  assert.equal(deleting.data.revision, 'revision-A-2');
  assert.equal(deleting.data.confirm, true);
  assert.equal(api.state.active, activeBeforeAdd);
  assert.equal($('#chatFeed > [data-key="m-200"]'), retained);
  assert.equal(document.querySelectorAll('#chatFeed > [data-key^="notice-"], #chatFeed > [data-key^="added-notice-"]').length, 0);
  assert.equal(window.backend.dialogs.B.filter(row => row.notice_type).length, 1, 'Mass removal is scoped to one conversation');

  const slow = deferred();
  infoQueue.push(slow);
  const pendingPreview = api.openNoticeModal('claude', 'A');
  const beforePreviewSubmit = writes.length;
  $('#noticeTextInput').value = 'Cannot submit this before the preview';
  await api.applySystemNotice();
  assert.equal(writes.length, beforePreviewSubmit);
  await api.openNoticeModal('claude', 'B');
  $('#noticeTextInput').value = 'Draft for B';
  slow.resolve({ ...info('A'), notice_count: 97 });
  await pendingPreview;
  assert.equal($('#noticeTextInput').value, 'Draft for B');
  assert(!$('#noticeSummary').textContent.includes('97'), 'Late preview cannot replace a newer dialog');

  await api.openNoticeModal('claude', 'A');
  const draft = 'Keep my draft after a revision error';
  $('#noticeTextInput').value = draft;
  $('#noticeTextInput').dispatchEvent(new window.Event('input', { bubbles: true }));
  revisions.A++;
  nextWriteError = 'Файл изменился. Обновите ревизию. <b>literal error</b>';
  const countBeforeError = noticeCalls.filter(call => call.op === 'system_notices_info').length;
  await api.applySystemNotice();
  assert($('#noticeModal').classList.contains('open'));
  assert.equal($('#noticeTextInput').value, draft);
  assert($('#noticeError').textContent.includes('Файл изменился'));
  assert.equal($('#noticeError b'), null);
  assert.equal(api.transcriptMutationPending(), false);
  assert(noticeCalls.filter(call => call.op === 'system_notices_info').length > countBeforeError, 'A failed write refreshes the revision before retry');
  const freshRevision = `revision-A-${revisions.A}`;
  await api.applySystemNotice();
  assert.equal(writes.at(-1).data.revision, freshRevision);
  assert.equal(writes.at(-1).data.text, draft);

  await api.openNoticeModal('claude', 'A');
  $('#noticeTextInput').value = 'Complete in A while viewing B';
  $('#noticeTextInput').dispatchEvent(new window.Event('input', { bubbles: true }));
  holdWrites = true;
  const backgroundAdd = api.applySystemNotice();
  await wait(() => writes.at(-1)?.data.text === 'Complete in A while viewing B');
  const backgroundWrite = writes.at(-1);
  await api.openThread('claude', 'B', { preserveSys: false });
  const activeB = api.state.active;
  const rowB = $('#chatFeed > [data-key="m-200"]');
  await api.openNoticeModal('claude', 'B');
  $('#noticeTextInput').value = 'Untouched newer draft';
  finishWrite(backgroundWrite);
  await backgroundAdd;
  holdWrites = false;
  assert.equal(api.state.active, activeB, 'Finishing an earlier mutation cannot steal the current conversation');
  assert.equal($('#chatFeed > [data-key="m-200"]'), rowB);
  assert($('#noticeModal').classList.contains('open'), 'An old mutation cannot close a newer modal');
  assert.equal($('#noticeTextInput').value, 'Untouched newer draft');

  api.closeModal('noticeModal');
  nextInfoError = 'Нельзя прочитать историю';
  await api.openNoticeModal('claude', 'A');
  assert($('#noticeApplyBtn').disabled);
  assert($('#noticeDeleteBtn').disabled);
  const beforeReadErrorWrite = writes.length;
  await api.applySystemNotice();
  assert.equal(writes.length, beforeReadErrorWrite, 'A failed inspection does not authorize a write');
  assert.deepEqual(errors, []);
  console.log('PASS: Claude notice catalog and scope, literal text, confirmation/revision, duplicate-write guard, retained transcript/sidebar, preview races, error recovery, and background mutation isolation. Node DOM only; no native filesystem or browser-layout claim.');
} finally {
  await window.happyDOM.abort();
  window.happyDOM.close();
}
