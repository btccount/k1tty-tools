 
 
 
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import vm from 'node:vm';
const root=resolve(fileURLToPath(new URL('..',import.meta.url)));
const {Window}=await import(process.env.HAPPY_DOM_MODULE ? pathToFileURL(process.env.HAPPY_DOM_MODULE).href : 'happy-dom');
const win=new Window({url:'https://k1tty.test',width:1280,height:900,settings:{disableCSSFileLoading:true,disableJavaScriptFileLoading:true}});
win.structuredClone=structuredClone;
 
 
win.history.replaceState=()=>{};
const browserHarness=readFileSync(resolve(root,'scripts/test-transcript-async.mjs'),'utf8');
const fake=browserHarness.match(/const fake = String\.raw`([\s\S]*?)`;\nconst app/)[1];
const html=readFileSync(resolve(root,'static/index.html'),'utf8').replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi,'').replace(/<link\b[^>]*>/gi,'');
const runtimeErrors=[];win.addEventListener('error',event=>runtimeErrors.push(event.message));
win.document.write(html);
const context=vm.createContext(win);
vm.runInContext(fake+'\n'+readFileSync(resolve(root,'static/app.js'),'utf8')+`\nwindow.testApi={state:STATE,openThread,openBackupResearch,loadMore,openEdit,applyEdit,executeDeleteSingle,executeDeleteConv,canMutateTranscript,transcriptMutationPending,refreshCurrentTranscript,setLang};`,context);
const api=win.testApi;
const $=selector=>win.document.querySelector(selector);
const count=selector=>win.document.querySelectorAll(selector).length;
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const wait=async condition=>{for(let i=0;i<300;i++){if(condition())return;await sleep(10)}throw Error('Timed out')};
try{
 await wait(()=>api.state.allConvs.length===2);
 await sleep(0);  
 const sidebarRows=[...win.document.querySelectorAll('#convList .conv-item')];
 assert.equal($('#threadMeta'),null,'Loaded count is removed from the header');
 assert.equal($('#langSwitch'),null,'Language control is no longer in the top bar');
 await api.openThread('claude','A',{preserveSys:false});await api.loadMore();
 assert(sidebarRows.every((row,index)=>row===win.document.querySelectorAll('#convList .conv-item')[index]),'Opening a conversation retains sidebar nodes');
 assert.equal(count('#chatFeed > .msg-block'),160,$('#chatFeed').textContent);
 const unchanged=$('#chatFeed > [data-key="m-120"]');unchanged.querySelector('.msg-body').classList.remove('clamped');
 await $('#toggleSysBtn').onclick();
 assert.equal(count('#chatFeed > .msg-block'),160);assert(count('#chatFeed > .tool-block')>0);
 assert.equal($('#chatFeed > [data-key="m-120"]'),unchanged);assert.equal(unchanged.querySelector('.msg-body').classList.contains('clamped'),false);
 const callCount=win.backend.calls.length;await $('#toggleSysBtn').onclick();assert.equal(win.backend.calls.length,callCount);
 assert.equal(count('#chatFeed > .tool-block'),0);
 unchanged.querySelector('[data-act="copy-code"]').click();await sleep(0);assert.equal(win.backend.copied,'const value = 42;');
  
  
 win.dispatchEvent(new win.CustomEvent('k1tty:languagechange',{detail:{language:'ru'}}));
 await api.state.active.languageRender;
 assert.equal(win.document.documentElement.lang,'ru');
 assert.equal(win.localStorage.getItem('k1tty_lang'),'ru');
 const translatedRow=$('#chatFeed > [data-key="m-120"]');
 const translatedSidebar=$('#convList .conv-item');
 const languageCalls=win.backend.calls.length;
 await api.setLang('ru');
 assert.equal($('#chatFeed > [data-key="m-120"]'),translatedRow);
 assert.equal($('#convList .conv-item'),translatedSidebar);
 assert.equal(win.backend.calls.length,languageCalls);
  
 const originalRpc=win.K1ttyDesktop.rpc;
 win.K1ttyDesktop.rpc=async(op,provider,data)=>{
  if(op==='edit_preview')await sleep(data.key==='m-130'?60:5);
  return originalRpc(op,provider,data);
 };
 const firstPreview=api.openEdit('m-130');
 const writesBefore=win.backend.calls.filter(call=>call.op==='apply_edit').length;
 await api.applyEdit();assert.equal(win.backend.calls.filter(call=>call.op==='apply_edit').length,writesBefore,'Ctrl+Enter before preview cannot submit');
 const secondPreview=api.openEdit('m-131');await Promise.all([firstPreview,secondPreview]);
 assert($('#editText').value.startsWith('Message A 131'),'Late preview must not replace newer modal content');
 $('#editText').value='Edited asynchronously';await api.applyEdit();
 assert.equal($('#chatFeed > [data-key="m-131"] .msg-body').textContent,'Edited asynchronously');assert.equal(count('#chatFeed > .msg-block'),160);
 const beforeClose=win.backend.calls.filter(call=>call.op==='entries').length;await $('#repCloseBtn').onclick();assert.equal(win.backend.calls.filter(call=>call.op==='entries').length,beforeClose);
  
 let releaseWrite;
 let interceptedDeletes=0;
 win.K1ttyDesktop.rpc=async(op,provider,data)=>{
  if(op==='delete_single'){interceptedDeletes++;await new Promise(resolve=>{releaseWrite=resolve});}
  return originalRpc(op,provider,data);
 };
 const deleting=api.executeDeleteSingle($('#chatFeed > [data-key="m-130"] [data-act="del-single"]'),'m-130');
 assert.equal(api.transcriptMutationPending(),true);assert.equal(api.canMutateTranscript(),false);
 await api.executeDeleteSingle($('#chatFeed > [data-key="m-132"] [data-act="del-single"]'),'m-132');
 assert.equal(interceptedDeletes,1,'Second mutation cannot use pre-write offsets');
  
 await api.openThread('claude','B',{preserveSys:false});
 win.backend.nextDelay=150;const reopening=api.openThread('claude','A',{preserveSys:false});
 releaseWrite();await deleting;await reopening;
 assert.equal(api.state.active.id,'A');assert.equal(count('#chatFeed > [data-key="m-130"]'),0,'Late pre-write initial read must be discarded after mutation');
 assert.equal(api.transcriptMutationPending(),false);assert.equal(api.canMutateTranscript(),true);
  
 win.K1ttyDesktop.rpc=originalRpc;
 win.backend.nextDelay=80;const paging=api.loadMore();await api.openThread('claude','B',{preserveSys:false});await paging;
 assert.equal(count('#chatFeed > .msg-block'),80);assert(!$('#chatFeed').textContent.includes('Message A'));
  
 const dup=structuredClone(win.backend.dialogs.B.find(row=>row.kind==='assistant'));
 win.backend.dialogs.B.push({...dup,key:'duplicate-one'},{...dup,key:'duplicate-two'});
 await api.refreshCurrentTranscript();const normalCount=count('#chatFeed > .msg-block');
 await $('#toggleSysBtn').onclick();assert.equal(count('#chatFeed > .msg-block'),normalCount);
 assert($('#chatFeed > [data-key="duplicate-one"]'));assert($('#chatFeed > [data-key="duplicate-two"]'));
 assert.notEqual($('#chatFeed > [data-key="duplicate-one"]'),$('#chatFeed > [data-key="duplicate-two"]'));
 await api.openBackupResearch('claude','backup-a','Copy A','Dialog A');await api.loadMore();await $('#toggleSysBtn').onclick();
 assert.equal(api.state.active.readonlyBackup,true);assert.equal(count('#chatFeed [data-act="edit"]'),0);
  
  
 win.K1ttyDesktop.rpc=async(op,provider,data)=>{
  if(op==='delete_conv'){
   const job_id='delete-conversation-test';
   setTimeout(()=>win.backend.emit({type:'job',job:{id:job_id,state:'done',phase:'done',result:{}}}),0);
   return {job_id};
  }
  return originalRpc(op,provider,data);
 };
 await api.openThread('claude','B',{preserveSys:false});
 await api.executeDeleteConv('claude','B',$('#convList [data-cid="B"]'));
 assert.equal(api.state.active,null);
 assert.equal($('#convList [data-cid="B"]'),null);
 assert.equal($('#cnt-all').textContent,'1');
 assert.equal(count('#chatFeed .skel-msg'),0);assert.deepEqual(runtimeErrors,[]);
 console.log('PASS: full app DOM/IPC — retained sidebar nodes, removed header counter, settings language event/no-op, async filters, retained expanded nodes, cached return, real code copy handler, stale edit previews, Ctrl+Enter guard, async edit/report close, overlapping writes, reopen during mutation, stale pagination, duplicate contents, backup read-only mode, active-conversation deletion updates sidebar. Browser layout not exercised.');
}finally{await win.happyDOM.abort();win.happyDOM.close();}
