import { browser, $, expect } from '@wdio/globals';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
describe('Native specimen desktop',()=>{
 before(async()=>{ await mkdir(resolve('../../work'),{recursive:true}); await browser.saveScreenshot(resolve('../../work/native-first-screen.png')); console.log('Native page title', await browser.getTitle()); });
 it('runs the Rust engine and renders a scoped response',async()=>{
  await $('#prompt').scrollIntoView();
  await $('#prompt').waitForDisplayed();
  await browser.waitUntil(async()=>!(await $('body').getText()).includes('Opening workspace'),{timeout:30000});
  await $('#prompt').setValue('What is 2 + 2?');
  await $('button=Run cycle').click();
  await expect($('.answer-text')).toHaveText('4');
  await $('button=Contributors').click();
  await expect($('.provenance-detail')).toHaveText(expect.stringContaining('Calculate'));
 });
 it('shows installed model metadata and performs native image analysis',async()=>{
  await $('nav[aria-label="Main navigation"] a[href="?view=lab"]').click();
  await expect($('.native-lab')).toBeDisplayed();
  await expect($('.runtime-pills')).toHaveText(expect.stringContaining('Native Rust engine'));
  const bytes=Array.from(await readFile(resolve('work/acceptance/ocr-fixture.png')));
  const result=await browser.tauri.execute(async(tauri,bytes)=>{
   const channel="__CHANNEL__:0";
   return tauri.core.invoke('native_call',{request:{op:'analyze',jobId:crypto.randomUUID(),bytes,focus:{x:0,y:0,width:1,height:1}},progress:channel});
  },bytes);
  if(!result.analysis.patternId.startsWith('visual-v2:'))throw Error('Missing native visual descriptor');
  if(result.analysis.sdf.values.length!==4096)throw Error('Invalid signed distance field');
  await mkdir(resolve('../../work'),{recursive:true});
  await browser.saveScreenshot(resolve('../../work/native-desktop.png'));
 });
});
