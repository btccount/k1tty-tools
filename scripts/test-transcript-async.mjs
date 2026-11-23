 
 
 
 
 
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
let browserTools;
try { browserTools = await import('playwright'); }
catch (error) {
  if (!process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES) throw error;
  browserTools = createRequire(`${process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES}/`)('playwright');
}
let html = readFileSync(resolve(root, 'static/index.html'), 'utf8');
html = html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '').replace(/<link\b[^>]*>/gi, '');
const css = ['style.css', 'design.css', 'appearance.css'].filter((name) => existsSync(resolve(root, 'static', name)))
  .map((name) => readFileSync(resolve(root, 'static', name), 'utf8')).join('\n');
html = html.replace('</head>', `<style>${css}</style></head>`);
const fake = String.raw`
window.backend = { calls: [], delay: 30, nextDelay: null, failFilter: false, copied: '', jobCounter: 0 };
const testItems = (id) => Array.from({length: 240}, (_, i) => [
 {key:'t-'+(i+1), kind:'tool', tool:'shell', text:'Service step '+id+' '+(i+1), actions:[]},
 {key:'m-'+(i+1), kind:i%2 ? 'assistant' : 'user', text:'Message '+id+' '+(i+1)+'\n'+('Transcript content. '.repeat(i===119 ? 160 : 9))+(i===119 ? '\n\x60\x60\x60js\nconst value = 42;\n\x60\x60\x60' : ''), actions:['edit','del_single','cut_head','cut_tail']}
]).flat();
window.backend.dialogs = {A:testItems('A'), B:testItems('B')};
window.K1ttyDesktop = {
 connect: async (callback) => { window.backend.emit = callback; },
 setWindowLabels(){}, writeText:async (text) => { window.backend.copied=text; },
 rpc:async (op, provider, data={}) => {
  const state=window.backend;
  state.calls.push({op,provider,data:{...data}});
  if(op==='state') return {providers:Object.fromEntries(['antigravity','antigravity_cli','codex','claude','opencode'].map(id=>[id,{id,name:id,available:id==='claude',exists:id==='claude',writable:true,running:[]}])),settings:{auto_backup:true}};
  if(op==='convs') return {convs:['A','B'].map(id=>({id,provider:'claude',title:'Dialog '+id,writable:true,mtime:'2026-11-20 12:00',size:12000}))};
  if(op==='backups_summary') return {providers:[],total_bytes:0,total_count:0};
  if(op==='set_view') return {};
  if(op==='entries'||op==='backup_entries') {
   const delay=state.nextDelay ?? state.delay; state.nextDelay=null;
   const fail = state.failFilter && data.include_tools; if(fail) state.failFilter=false;
   const rows=(state.dialogs[data.cid||'A']||[]).filter(item=>data.include_tools || item.kind!=='tool');
   const end=data.before == null ? rows.length : Number(data.before);
   const cap=data.include_tools ? 100 : 2000;
   let start=end, msgs=0;
   while(start>0 && msgs<data.limit && end-start<cap) {start--; if(rows[start].kind!=='tool')msgs++;}
   const response={turns:[{items:structuredClone(rows.slice(start,end))}],page_msgs:msgs,cursor:start>0?String(start):null,has_more:start>0,total_label:'загружено: '+msgs,writable:op==='entries'};
   await new Promise(resolve=>setTimeout(resolve,delay));
   if(fail)throw new Error('Test read failure');
   return response;
  }
  if(op==='edit_preview') return {text:state.dialogs[data.cid].find(row=>row.key===data.key).text,revision:'test',writable:true};
  if(op==='apply_edit'||op==='delete_single') {
   if(op==='apply_edit')state.dialogs[data.cid].find(row=>row.key===data.key).text=data.text;
   else state.dialogs[data.cid]=state.dialogs[data.cid].filter(row=>row.key!==data.key);
   const job_id='job-'+(++state.jobCounter);
   setTimeout(()=>state.emit({type:'job',job:{id:job_id,state:'done',phase:'done',result:{}}}),5);
   return {job_id};
  }
  return {};
 }
};
`;
const app = readFileSync(resolve(root, 'static/app.js'), 'utf8');
html += `<script>${fake}</script><script>${app.replaceAll('</script>', '<\\/script>')}</script>`;
const executablePath=process.env.CHROMIUM_PATH || (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined);
const browser=await browserTools.chromium.launch({headless:true,executablePath,args:['--no-sandbox']});
try {
 const page=await browser.newPage({viewport:{width:1280,height:900}});
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.setContent(html,{waitUntil:'domcontentloaded'});
 await page.waitForFunction(()=>window.__booted && STATE.allConvs.length===2);
 await page.evaluate(()=>openThread('claude','A',{preserveSys:false}));
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),80);
 await page.evaluate(()=>loadMore());
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),160);
 await page.evaluate(()=>{
  const row=document.querySelector('#chatFeed > [data-key="m-120"]');
  row.querySelector('.msg-body').classList.remove('clamped');
  const sc=document.querySelector('#feedScroll');
  sc.scrollTop+=row.getBoundingClientRect().top-sc.getBoundingClientRect().top-32;
  window.testRetained=row;
 });
 const anchor = () => page.evaluate(()=>{
  const row=document.querySelector('#chatFeed > [data-key="m-120"]');
  return {offset:row.getBoundingClientRect().top-document.querySelector('#feedScroll').getBoundingClientRect().top,top:document.querySelector('#feedScroll').scrollTop,same:row===window.testRetained,clamped:row.querySelector('.msg-body').classList.contains('clamped')};
 });
 const original=await anchor();
 await page.evaluate(()=>document.querySelector('#toggleSysBtn').onclick());
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),160,'All loaded messages remain when showing service steps');
 assert((await page.locator('#chatFeed > .tool-block').count())>0);
 const withTools=await anchor();
 assert(withTools.same && !withTools.clamped,'Unchanged expanded message node retained');
 assert(Math.abs(withTools.offset-original.offset)<2,'Service steps preserve visible message offset');
 const callsBefore=await page.evaluate(()=>backend.calls.filter(call=>call.op==='entries').length);
 await page.evaluate(()=>document.querySelector('#toggleSysBtn').onclick());
 assert.equal(await page.locator('#chatFeed > .tool-block').count(),0);
 assert.equal(await page.evaluate(()=>backend.calls.filter(call=>call.op==='entries').length),callsBefore,'Returning to cached filter does not refetch');
 assert(Math.abs((await anchor()).offset-original.offset)<2);
 await page.evaluate(()=>document.querySelector('#chatFeed > [data-key="m-120"] [data-act="copy-code"]').click());
 assert.equal(await page.evaluate(()=>backend.copied),'const value = 42;','Copy code survives a retained DOM node and filter render');
  
 await page.evaluate(()=>{STATE.active.transcriptCache.delete(true);backend.nextDelay=180;document.querySelector('#toggleSysBtn').click();document.querySelector('#toggleSysBtn').click();});
 await page.waitForTimeout(240);
 assert.equal(await page.locator('#chatFeed > .tool-block').count(),0,'Late filter response ignored');
 assert.equal(await page.evaluate(()=>STATE.showSys),false);
  
 await page.evaluate(()=>{STATE.active.transcriptCache.delete(true);backend.failFilter=true;return document.querySelector('#toggleSysBtn').onclick();});
 assert.equal(await page.evaluate(()=>STATE.showSys),false);
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),160);
 assert(Math.abs((await anchor()).offset-original.offset)<2);
  
 await page.evaluate(()=>{backend.nextDelay=180;loadMore();return openThread('claude','B',{preserveSys:false});});
 await page.waitForTimeout(240);
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),80);
 assert.equal(await page.locator('#chatFeed').evaluate(el=>el.textContent.includes('Message A')),false);
 assert.equal(await page.evaluate(()=>STATE.active.id),'B');
  
 await page.evaluate(async()=>{await openThread('claude','A',{preserveSys:false});await loadMore();});
 await page.evaluate(()=>{
  const row=document.querySelector('#chatFeed > [data-key="m-120"]'),sc=document.querySelector('#feedScroll');
  sc.scrollTop+=row.getBoundingClientRect().top-sc.getBoundingClientRect().top-32;
  window.testRetained=row;
 });
 const beforeEdit=await anchor();
 await page.evaluate(async()=>{await openEdit('m-130');document.querySelector('#editText').value='Edited message';await applyEdit();});
 assert.equal(await page.locator('#chatFeed > .msg-block').count(),160);
 assert.equal(await page.locator('#chatFeed > [data-key="m-130"] .msg-body').textContent(),'Edited message');
 assert(Math.abs((await anchor()).offset-beforeEdit.offset)<2);
 const refreshCount=await page.evaluate(()=>backend.calls.filter(call=>call.op==='entries').length);
 await page.evaluate(()=>document.querySelector('#repCloseBtn').onclick());
 assert.equal(await page.evaluate(()=>backend.calls.filter(call=>call.op==='entries').length),refreshCount,'Closing the report does not refetch the transcript');
 await page.evaluate(()=>executeDeleteSingle(document.querySelector('#chatFeed > [data-key="m-130"] [data-act="del-single"]'),'m-130'));
 assert.equal(await page.locator('#chatFeed > [data-key="m-130"]').count(),0);
 assert(Math.abs((await anchor()).offset-beforeEdit.offset)<2);
  
 await page.evaluate(()=>setLang('ru'));
 assert(Math.abs((await anchor()).offset-beforeEdit.offset)<2);
  
 await page.evaluate(async()=>{await openBackupResearch('claude','backup-a','Copy A','Dialog A');await loadMore();});
 await page.evaluate(()=>{
  const row=document.querySelector('#chatFeed > [data-key="m-120"]'),sc=document.querySelector('#feedScroll');
  sc.scrollTop+=row.getBoundingClientRect().top-sc.getBoundingClientRect().top-32;
  window.testRetained=row;
 });
 const backupAnchor=await anchor();
 await page.evaluate(()=>document.querySelector('#toggleSysBtn').onclick());
 assert.equal(await page.evaluate(()=>STATE.active.readonlyBackup),true);
 assert.equal(await page.locator('#chatFeed [data-act="edit"]').count(),0);
 assert(Math.abs((await anchor()).offset-backupAnchor.offset)<2);
 assert.equal(await page.locator('#chatFeed .skel-msg').count(),0);
 assert.equal(await page.evaluate(()=>document.querySelector('#feedScroll').inert),false);
 assert.deepEqual(errors,[],'No browser runtime errors');
 console.log('PASS: service filters retain messages, scroll and expanded DOM; cached toggle; stale filter/pagination guards; code copy; async edit/delete; report close; language; backup reader.');
} finally {await browser.close();}
