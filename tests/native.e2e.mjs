import { browser, $, expect } from '@wdio/globals';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
describe('Native specimen desktop',()=>{
 before(async()=>{ await mkdir(resolve('../../work'),{recursive:true}); await browser.saveScreenshot(resolve('../../work/native-first-screen.png')); console.log('Native page title', await browser.getTitle()); });
 it('runs the Rust engine and renders a scoped response',async()=>{
  await browser.waitUntil(async()=>!(await $('body').getText()).includes('Opening workspace'),{timeout:30000});
  await $('button=What is 2 + 2?').click();
  await expect($('.answer-text')).toHaveText('4');
  await $('button=Contributors').click();
  await expect($('.provenance-detail')).toHaveText(expect.stringContaining('Calculate'));
 });
 it('shows installed model metadata and performs native image analysis',async()=>{
  await $('nav[aria-label="Main navigation"] a[href="?view=lab"]').click();
  await expect($('.native-lab')).toBeDisplayed();
  await expect($('.runtime-pills')).toHaveText(expect.stringContaining('Native Rust engine'));
  console.log('Native model workspace ready');
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
});
