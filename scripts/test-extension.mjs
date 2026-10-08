import { build } from 'esbuild';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const bundled = await build({entryPoints:['apps/extension/entrypoints/background.ts'],bundle:true,write:false,format:'iife',plugins:[{name:'mock-browser',setup(build){build.onResolve({filter:/^wxt\/browser$/},()=>({path:'browser',namespace:'mock'}));build.onLoad({filter:/.*/,namespace:'mock'},()=>({contents:'export const browser = globalThis.mockBrowser;'}));}}]});
let listener; let nativeRequests=[]; let removed=[]; let nativeOK=true; let navigateAfterSave=false; let moveAfterSave=false;
const tabs=[{id:1,windowId:10,url:'https://example.com/a',title:'A',active:true,highlighted:true,index:0},{id:2,windowId:10,url:'chrome://settings',title:'Settings',index:1},{id:3,windowId:20,url:'https://example.com/b',title:'B',active:true,index:0}];
const mockBrowser={windows:{getCurrent:async()=>{throw new Error('The background must not resolve the popup window');},getAll:async()=>[10,20].map(id=>({id,tabs:tabs.filter(tab=>tab.windowId===id)}))},tabs:{get:async id=>({...tabs.find(tab=>tab.id===id),...(moveAfterSave?{windowId:30}:{}),...(navigateAfterSave?{url:'https://example.com/changed'}:{})}),remove:async id=>removed.push(id)},runtime:{onMessage:{addListener:fn=>listener=fn},sendNativeMessage:async(_,request)=>{nativeRequests.push(request);return nativeOK?{ok:true,data:{}}:{ok:false,error:'Storage failed'};}}};
vm.runInNewContext(bundled.outputFiles[0].text,{mockBrowser,defineBackground:fn=>fn(),URL,Set,Map});
const request=(action='add')=>({type:'extension_pause_quest',action,quest_id:'course',checkpoint:action==='add'?'':'Continue',scope:'all_windows',window_id:10,selected_tab_ids:[1,3],expected_urls:{1:tabs[0].url,3:tabs[2].url},close_after_save:action==='close'});
let result=await listener({type:'extension_preview',scope:'all_windows',window_id:10},{});
assert.equal(result.data.tabs.length,2);assert.equal(result.data.filtered_count,1);assert.equal(result.data.windows.length,2);assert.equal(result.data.suggested_ids.join(','),'1');
result=await listener(request(),{});assert.equal(result.ok,true);assert.equal(nativeRequests.at(-1).type,'add_tabs');assert.equal(nativeRequests.at(-1).tabs.map(tab=>tab.id).join(','),'1,3');assert.equal(removed.length,0);
result=await listener({...request(),expected_urls:{1:'https://old.invalid',3:tabs[2].url}},{});assert.equal(result.ok,false);assert.equal(nativeRequests.length,1);
nativeOK=false;result=await listener(request('close'),{});assert.equal(result.ok,false);assert.equal(removed.length,0);
nativeOK=true;navigateAfterSave=true;result=await listener(request('close'),{});assert.equal(result.ok,true);assert.equal(result.data.close_errors.length,2);assert.equal(removed.length,0);
navigateAfterSave=false;result=await listener(request('close'),{});assert.equal(result.data.closed_count,2);assert.equal(removed.join(','),'1,3');
result=await listener({type:'extension_create_quest',title:'New course',parent_id:'trading'},{});assert.equal(result.ok,true);assert.equal(nativeRequests.at(-1).parent_id,'trading');
console.log('Extension checks passed: multi-window capture, filtering, defaults, add mode, changed preview, failed persistence, post-save navigation, save-before-close, nested creation.');
const validationBundle = await build({entryPoints:['apps/extension/utils/preview.ts'],bundle:true,write:false,format:'cjs'});
const validationModule = {exports:{}};
vm.runInNewContext(validationBundle.outputFiles[0].text,{module:validationModule,exports:validationModule.exports});
const {validatePreview}=validationModule.exports;
assert.throws(()=>validatePreview({tabs:[],filtered_count:0,context_label:'This window'}),/different extension versions/);
assert.throws(()=>validatePreview({protocol_version:4,tabs:[],windows:[]}),/incomplete tab preview/);
const good=await listener({type:'extension_preview',scope:'all_windows',window_id:10},{});
assert.equal(validatePreview(good.data).tabs.length,2);
console.log('Preview compatibility checks passed: legacy background, incomplete response, current protocol.');

// The invoking window is explicit: never infer it from background focus.
for (const [windowId, expectedTab] of [[10, 1], [20, 3]]) {
  const preview = await listener({type:'extension_preview',scope:'current_window',window_id:windowId}, {});
  assert.equal(preview.ok, true);
  assert.equal(preview.data.tabs.map(tab => tab.id).join(','), String(expectedTab));
  assert.equal(preview.data.suggested_ids.join(','), String(expectedTab));
  assert.equal(preview.data.windows[0].id, windowId);
  assert.match(preview.data.windows[0].label, /^This window/);
}
const otherOrigin = await listener({type:'extension_preview',scope:'all_windows',window_id:20}, {});
assert.equal(otherOrigin.data.windows[0].id, 20);
assert.equal(otherOrigin.data.suggested_ids.join(','), '3');
assert.match(otherOrigin.data.windows[0].label, /^This window/);
assert.doesNotMatch(otherOrigin.data.windows[1].label, /^This window/);
const writesBefore = nativeRequests.length;
const closesBefore = removed.length;
for (const windowId of [undefined, -1, 1.5, '10', 99]) {
  const preview = await listener({type:'extension_preview',scope:'current_window',window_id:windowId}, {});
  assert.equal(preview.ok, false, 'Missing/closed origin must not fall back to another window');
  const capture = await listener({...request('close'),window_id:windowId}, {});
  assert.equal(capture.ok, false);
}
const localCapture = {...request('close'), scope:'current_window', selected_tab_ids:[1]};
// A tab moved after preview must not be saved or closed under This window.
tabs[0].windowId = 20;
assert.equal((await listener(localCapture, {})).ok, false);
tabs[0].windowId = 10;
assert.equal((await listener({...request('close'),scope:'current_window'}, {})).ok, false);
assert.equal(nativeRequests.length, writesBefore);
assert.equal(removed.length, closesBefore);
// The origin remains usable even if the worker's idea of focus is different.
assert.equal((await listener({...localCapture,action:'add',checkpoint:'',close_after_save:false}, {})).ok, true);
assert.equal(nativeRequests.at(-1).tabs.map(tab => tab.id).join(','), '1');
moveAfterSave = true;
const moved = await listener(localCapture, {});
assert.equal(moved.ok, true);
assert.equal(moved.data.closed_count, 0);
assert.match(moved.data.close_errors[0], /moved to another window/);
assert.equal(removed.length, closesBefore);
assert.throws(()=>validatePreview({protocol_version:2,tabs:[],windows:[],suggested_ids:[]}),/different extension versions/);
console.log('Window regression checks passed: explicit popup origin, both windows, all-window labels/defaults, missing/closed origins, moved tabs, scope enforcement, post-save moves.');

moveAfterSave = false;
for (const action of ['save', 'close']) {
  const before = removed.length;
  const saved = await listener({...request(action),checkpoint:'   '}, {});
  assert.equal(saved.ok, true);
  assert.equal(nativeRequests.at(-1).checkpoint, '');
  assert.equal(nativeRequests.at(-1).type, 'pause_quest');
  assert.equal(removed.length - before, action === 'close' ? 2 : 0);
}
console.log('Optional checkpoint checks passed: save and save-and-close accept blank notes.');
