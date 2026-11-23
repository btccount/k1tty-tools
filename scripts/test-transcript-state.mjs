 
 
 
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
const source=readFileSync(new URL('../static/app.js',import.meta.url),'utf8');
const helper=source.slice(source.indexOf('let threadTransitionTimer = null;'),source.indexOf('\nasync function openThread('));
const paging=source.slice(source.indexOf('async function loadMore()'),source.indexOf('\nfunction updateScrollBtn()'));
const classes=(initial=[])=>{
 const set=new Set(initial);
 return {add:(...xs)=>xs.forEach(x=>set.add(x)),remove:(...xs)=>xs.forEach(x=>set.delete(x)),contains:x=>set.has(x),toggle(x,on){on ??= !set.has(x);on?set.add(x):set.delete(x);return on}};
};
const element=()=>({dataset:{},style:{},classList:classes(),attributes:{},setAttribute(k,v){this.attributes[k]=v},removeAttribute(k){delete this.attributes[k]}});
const elements=new Map();
const get=id=>elements.get(id);
const metrics={renders:0,geometryReads:0,yields:0};
let onYield=null;
let scrollValue=0;
const feed={...element(),children:[],get firstElementChild(){return this.children[0]||null},insertBefore(row,before){
 if(row.parent)row.remove();
 const index=before?this.children.indexOf(before):this.children.length;
 assert(index>=0,'DOM reference must exist');this.children.splice(index,0,row);row.parent=this;
}};
const scroll={...element(),clientHeight:400,get scrollHeight(){return 40+feed.children.reduce((sum,row)=>sum+row.height,0)},get scrollTop(){return Math.min(scrollValue,Math.max(0,this.scrollHeight-this.clientHeight))},set scrollTop(value){scrollValue=Math.max(0,Math.min(value,Math.max(0,this.scrollHeight-this.clientHeight)))},getBoundingClientRect:()=>({top:0,bottom:400})};
function rowFromItem(item){
 const body={classList:classes(item.text.length>200?['clamped']:[])};
 const details={open:false,setAttribute(name){if(name==='open')this.open=true}};
 const clamp={innerHTML:''};
 return {...element(),dataset:{key:String(item.key)},item,body,details,parent:null,
  get height(){return item.kind==='tool'?(details.open?160:24):(body.classList.contains('clamped')?100:180)},
  get nextElementSibling(){if(!this.parent)return null;return this.parent.children[this.parent.children.indexOf(this)+1]||null},
  matches(){return item.kind==='user'||item.kind==='assistant'},
  getBoundingClientRect(){metrics.geometryReads++;const previous=this.parent?this.parent.children.slice(0,this.parent.children.indexOf(this)):[];const top=40+previous.reduce((sum,row)=>sum+row.height,0)-scroll.scrollTop;return {top,bottom:top+this.height}},
  querySelector(selector){if(selector==='details[open]')return item.kind==='tool'&&details.open?details:null;if(selector==='details')return item.kind==='tool'?details:null;if(selector==='.msg-body')return item.kind==='tool'?null:body;if(selector==='.msg-body.clamped')return body.classList.contains('clamped')?body:null;if(selector==='.clamp-bar')return item.text.length>200?clamp:null;if(selector==='[data-act="clamp"]')return item.text.length>200?clamp:null;return null},
  remove(){if(this.parent){const index=this.parent.children.indexOf(this);if(index>=0)this.parent.children.splice(index,1);this.parent=null}}
 };
}
elements.set('#chatFeed',feed);elements.set('#feedScroll',scroll);
for(const id of ['#chatWrap','#threadTransitionOverlay','#toggleSysBtn','#threadMeta','#moreWrap','#moreBtn'])elements.set(id,element());
const state={active:{provider:'claude',id:'A',readonlyBackup:false},loadingThreadToken:1,showSys:false,loadedMsgs:0,cursor:null};
const backend={calls:[],delay:0,nextDelay:null,failFilter:false};
const makeItems=id=>Array.from({length:240},(_,i)=>[{key:'t-'+i,kind:'tool',text:`Tool ${id} ${i}`},{key:'m-'+i,kind:i%2?'assistant':'user',text:`Message ${id} ${i} `+'x'.repeat(i===119?300:60)}]).flat();
const dialogs={A:makeItems('A'),B:makeItems('B')};
const codeStore=[];
const errors=[];
const env={STATE:state,CODE_STORE:codeStore,CUR_LANG:'en',LOCK_PRESETS:[],Map,Set,Array,JSON,Number,String,Math,Promise,setTimeout,clearTimeout,console,
 scheduler:{yield:async()=>{metrics.yields++;await new Promise(resolve=>setTimeout(resolve,0));onYield?.()}},
 $:get,esc:x=>x,t:x=>x,ico:()=>'',toast:message=>errors.push(message),localizeTotalLabel:x=>x,
 updateThreadLockBtn(){},updateThreadNoticesBtn(){},refreshStatusBadge(){},showUnlockerCard(){},hideUnlockerCard(){},updateScrollBtn(){},
 renderItemHTML(item){metrics.renders++;codeStore.push('code populated while rendering');return JSON.stringify([item])},
 renderMoreBar(){},
 document:{createElement(kind){assert.equal(kind,'template');return{content:{children:[],get firstElementChild(){return this.children[0]||null}},set innerHTML(value){this.content.children=JSON.parse(value).map(rowFromItem)}}}},
 rpc:async(op,provider,data)=>{
  backend.calls.push({op,provider,data});const delay=backend.nextDelay??backend.delay;backend.nextDelay=null;
  const shouldFail=backend.failFilter&&data.include_tools;if(shouldFail)backend.failFilter=false;
  const items=dialogs[data.cid||'A'].filter(item=>data.include_tools||item.kind!=='tool');
  const end=data.before==null?items.length:Number(data.before);let start=end,count=0;
  const max=data.include_tools?100:2000;
  while(start>0&&count<data.limit&&end-start<max){start--;if(items[start].kind!=='tool')count++;}
  const response={turns:[{items:structuredClone(items.slice(start,end))}],page_msgs:count,cursor:start>0?String(start):null,has_more:start>0,total_label:`загружено: ${count}`,writable:true};
  await new Promise(resolve=>setTimeout(resolve,delay));if(shouldFail)throw Error('Read failed');return response;
 }
};
runInNewContext(helper+'\n'+paging+'\nglobalThis.api={applyTranscriptResponse,readTranscriptRange,refreshCurrentTranscript,refreshAfterTranscriptMutation,loadMore,captureTranscriptAnchor,restoreTranscriptAnchor,commitThreadTranscript};',env);
const api=env.api;
let response=await api.readTranscriptRange(state.active,false,80,1);await api.applyTranscriptResponse(response);
assert.equal(feed.children.length,80);
assert.equal(codeStore.length,80,'Code cache remains populated after commit');
await api.loadMore();assert.equal(state.loadedMsgs,160);
const anchorRow=feed.children.find(row=>row.dataset.key==='m-119');anchorRow.body.classList.remove('clamped');scroll.scrollTop+=anchorRow.getBoundingClientRect().top-32;
const offset=anchorRow.getBoundingClientRect().top;
state.showSys=true;await api.refreshCurrentTranscript({filterOnly:true});
assert.equal(state.loadedMsgs,160,'Filter retrieves all loaded normal messages despite tool page limits');
assert.equal(feed.children.filter(row=>row.item.kind!=='tool').length,160);
assert.equal(feed.children.find(row=>row.dataset.key==='m-119'),anchorRow,'Normal message DOM node retained');
assert.equal(anchorRow.getBoundingClientRect().top,offset,'Anchor restored after inserting service entries');
assert.equal(anchorRow.body.classList.contains('clamped'),false);
const count=backend.calls.length;state.showSys=false;await api.refreshCurrentTranscript({filterOnly:true});
assert.equal(backend.calls.length,count,'Cached return filter uses no RPC');
assert.equal(anchorRow.getBoundingClientRect().top,offset);
assert.equal(feed.children.length,160);
state.active.transcriptCache.delete(true);backend.nextDelay=30;state.showSys=true;const delayed=api.refreshCurrentTranscript({filterOnly:true});state.showSys=false;await api.refreshCurrentTranscript({filterOnly:true});await delayed;
assert.equal(state.showSys,false);assert.equal(feed.children.length,160);assert.equal(get('#chatWrap').attributes['aria-busy'],undefined);
state.active.transcriptCache.delete(true);backend.failFilter=true;state.showSys=true;await api.refreshCurrentTranscript({filterOnly:true});
assert.equal(state.showSys,false,'Failed filter rolls back its state');assert.equal(feed.children.length,160);assert.equal(anchorRow.getBoundingClientRect().top,offset);assert.deepEqual(errors,['Read failed']);
backend.nextDelay=30;const oldPage=api.loadMore();state.active={provider:'claude',id:'B',readonlyBackup:false};state.loadingThreadToken++;state.showSys=false;response=await api.readTranscriptRange(state.active,false,80,state.loadingThreadToken);await api.applyTranscriptResponse(response);await oldPage;
assert.equal(state.loadedMsgs,80);assert(feed.children.every(row=>row.item.text.includes('Message B')),'Old pagination cannot add rows to a new dialog');
const beforeWrongMutation=backend.calls.length;await api.refreshAfterTranscriptMutation({provider:'claude',cid:'A'});assert.equal(backend.calls.length,beforeWrongMutation,'Completed edit in previous chat does not refresh current chat');
await api.loadMore();const editedAnchor=feed.children.find(row=>row.dataset.key==='m-119');scroll.scrollTop+=editedAnchor.getBoundingClientRect().top-32;const beforeOffset=editedAnchor.getBoundingClientRect().top;
 
for(const item of dialogs.B)item.key='new-'+item.key;
await api.refreshAfterTranscriptMutation({provider:'claude',cid:'B'});
const afterAnchor=feed.children.find(row=>row.dataset.key==='new-m-119');assert.equal(afterAnchor.getBoundingClientRect().top,beforeOffset,'Content identity restores anchor even when keys change');assert.equal(state.active.needsRefresh,false);

 
 
const large=[{items:Array.from({length:2000},(_,index)=>({key:'large-'+index,kind:index%2?'assistant':'user',text:'Large response '+index+(index===0?'x'.repeat(300):'')}))}];
const retainedBefore=[...feed.children];
const renderedBefore=metrics.renders;
const canceled=api.commitThreadTranscript(state.loadingThreadToken,large,false);
assert.deepEqual(feed.children,retainedBefore,'Off-DOM preparation does not blank the reader');
await new Promise(resolve=>setTimeout(resolve,0));
state.loadingThreadToken++;
assert.equal(await canceled,false);
assert(metrics.renders-renderedBefore<large[0].items.length,'Canceled preparation stops before rendering the full response');
assert.deepEqual(feed.children,retainedBefore,'Canceled rows are never attached');
const yieldsBefore=metrics.yields;
let heartbeat=false;setTimeout(()=>{heartbeat=true},0);
await api.commitThreadTranscript(state.loadingThreadToken,large,false);
assert(heartbeat,'The event loop runs before a large transcript finishes');
assert(metrics.yields-yieldsBefore>=80,'Work is split into bounded item batches');
assert.equal(feed.children.length,2000);
const readsBefore=metrics.geometryReads;
api.captureTranscriptAnchor();
assert(metrics.geometryReads-readsBefore<40,'Anchor lookup measures a neighborhood, not all 2000 rows');
const oldLargeRows=[...feed.children], rendersBeforeReuse=metrics.renders;
await api.commitThreadTranscript(state.loadingThreadToken,large,false,{preserve:true});
assert.equal(metrics.renders,rendersBeforeReuse,'Unchanged rows skip Markdown and HTML allocation');
assert(feed.children.every((row,index)=>row===oldLargeRows[index]));
const modified=[{items:large[0].items.map(item=>({...item,text:item.text+' edited'}))}];
let expandedDuringPreparation=false;
onYield=()=>{
 if(!expandedDuringPreparation&&metrics.renders>rendersBeforeReuse){
  expandedDuringPreparation=true;
  oldLargeRows[0].body.classList.remove('clamped');
 }
};
await api.commitThreadTranscript(state.loadingThreadToken,modified,false,{preserve:true});
onYield=null;
assert(expandedDuringPreparation);
assert.equal(feed.children[0].body.classList.contains('clamped'),false,'An expand click during detached preparation survives the final replacement');
console.log('PASS: actual transcript functions — complete filter range, retained nodes/expanded state, scroll-anchor algebra, code cache ordering, cached filter, stale requests, error rollback, pagination isolation, mutation routing and changed byte-offset keys.');
console.log('PASS: 2000-row synthetic workload — cooperative task yields, canceled detached work, unchanged-row reuse and fewer than 40 anchor geometry reads. No browser/FPS claim.');
