import { browser, $, $$, expect } from '@wdio/globals';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
describe('Native specimen desktop',()=>{
 before(async()=>{ await mkdir(resolve('../../work'),{recursive:true}); await browser.saveScreenshot(resolve('../../work/native-first-screen.png')); console.log('Native page title', await browser.getTitle()); });
 it('runs the Rust engine and renders a scoped response',async()=>{
  await browser.waitUntil(async()=>!(await $('body').getText()).includes('Opening workspace'),{timeout:30000});
  await $('nav[aria-label="Main navigation"] a[href="?view=studio"]').waitForDisplayed();
  await $('nav[aria-label="Main navigation"] a[href="?view=studio"]').click();
  await $('button=What is 2 + 2?').waitForDisplayed();
  await $('button=What is 2 + 2?').click();
  await expect($('.answer-text')).toHaveText('4');
  await $('button=Contributors').click();
  await expect($('.provenance-detail')).toHaveText(expect.stringContaining('Calculate'));
  await $('input#prompt').setValue("don't calculate 2 + 2; say hello");
  await $('button=Run cycle').click();
  await browser.waitUntil(async()=>/\b(Hello|Hi|Hey|Greetings)\b/.test((await $$('.answer-text').map(element=>element.getText())).join(' ')),{timeout:30000});
  if((await $$('.answer-text').map(element=>element.getText())).some(text=>text.trim()==='4'))throw Error('Negated arithmetic escaped its clause');
 });
 it('shows installed model metadata and performs native image analysis',async()=>{
  await $('nav[aria-label="Main navigation"] a[href="?view=lab"]').click();
  await expect($('.native-lab')).toBeDisplayed();
  await expect($('.runtime-pills')).toHaveText(expect.stringContaining('Native Rust engine'));
  const missing=await browser.tauri.execute(async()=>{
   try { await window.__wdbxTestCall({op:'generate',jobId:crypto.randomUUID(),modelId:'qwen3-4b',prompt:'hello',seed:1}); return 'unexpected success'; }
   catch(e){return e.code;}
  });
  if(missing!=='ModelMissing')throw Error('Missing model must produce a structured error: '+missing);
  console.log('Native model workspace ready; missing model handled locally');
  const bytes=Array.from(await readFile(resolve('work/acceptance/ocr-fixture.png')));
  const result=await browser.tauri.execute(async(tauri,bytes)=>{
   return window.__wdbxTestCall({op:'analyze',jobId:crypto.randomUUID(),bytes,focus:{x:0,y:0,width:1,height:1}});
  },bytes);
  if(!result.analysis.patternId.startsWith('visual-v2:'))throw Error('Missing native visual descriptor');
  if(result.analysis.sdf.values.length!==4096)throw Error('Invalid signed distance field');
  console.log('Native analysis ready');
  const visualCycle=await browser.tauri.execute(async(tauri,result)=>{
   const call=request=>window.__wdbxTestCall(request);
   const current=await call({op:'snapshot'});
   const learned=await call({op:'learnVisual',revision:current.revision,asset:result.asset,analysis:result.analysis,name:'Native visual fixture',ocrCorrection:'HELLO WORLD'});
   return call({op:'runVisual',revision:learned.revision,jobId:crypto.randomUUID(),asset:result.asset,focus:result.analysis.focus});
  },result);
  if(visualCycle.cycle.votes.length!==1)throw Error('Expected one learned visual vote');
  if(visualCycle.cycle.votes[0].evidence.similarity!==100)throw Error('Visual score not preserved');
  if(visualCycle.cycle.imageEvidence.asset!==result.asset)throw Error('Missing image provenance');

  await mkdir(resolve('../../work'),{recursive:true});
  await browser.saveScreenshot(resolve('../../work/native-desktop.png'));
 });
 it('searches a virtualized 100,000-record history',async()=>{
  const count=await browser.tauri.execute(async()=>{
   const call=request=>window.__wdbxTestCall(request);
   const snapshot=await call({op:'snapshot'});
   const specimen=snapshot.specimen;
   specimen.settings.historyLimit=100000;
   specimen.history=Array.from({length:100000},(_,i)=>({id:'scale-'+i,input:'Scale record '+i,createdAt:'2026-09-04T12:00:00Z',segments:[{id:'segment-'+i,text:'Evidence '+i,contributors:[]}],votes:[],trace:[],feedback:[],status:'matched',pinned:false}));
   await call({op:'edit',revision:snapshot.revision,specimen});
   return specimen.history.length;
  });
  if(count!==100000)throw Error('Scale fixture incomplete');
  await browser.refresh();
  await browser.waitUntil(async()=>!(await $('body').getText()).includes('Opening workspace'),{timeout:30000});
  await $('nav[aria-label="Main navigation"] a[href="?view=memory"]').click();
  await $('button=Conversation').click();
  await $('.history-record').waitForDisplayed({timeout:30000});
  const rendered=await $$('.history-record');
  if(rendered.length<1||rendered.length>30)throw Error('History virtualization failed: '+rendered.length);
  const start=Date.now();
  await $('input[placeholder="Search your memory…"]').setValue('Scale record 43210');
  await expect($('.history-heading')).toHaveText(expect.stringContaining('Scale record 43210'));
  const elapsed=Date.now()-start;
  console.log('100,000-record history search rendered in '+elapsed+' ms; initial DOM rows '+rendered.length);
  if(elapsed>5000)throw Error('History search exceeded five-second acceptance budget');
 });
});
