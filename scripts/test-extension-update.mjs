import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import vm from 'node:vm';
import { build } from 'esbuild';
import { fingerprintExtension } from './fingerprint-extension.mjs';

const directory = mkdtempSync(join(tmpdir(), 'quest-extension-update-'));
try {
  const manifestPath = join(directory, 'manifest.json');
  writeFileSync(manifestPath, JSON.stringify({ version: '0.2.4', background: { service_worker: 'background.js' } }));
  writeFileSync(join(directory, 'background.js'), '/* first worker */');
  const first = fingerprintExtension(directory);
  assert.notEqual(first, 'background.js');
  assert.equal(fingerprintExtension(directory), first, 'Repeated installation must be stable');

  // A cached worker URL from an earlier build must not be reused after a change.
  writeFileSync(manifestPath, JSON.stringify({ version: '0.2.4', background: { service_worker: 'background.js' } }));
  writeFileSync(join(directory, 'background.js'), '/* updated worker */');
  const second = fingerprintExtension(directory);
  assert.notEqual(second, first);
  assert.equal(readFileSync(join(directory, second), 'utf8'), '/* updated worker */');
  assert.equal(JSON.parse(readFileSync(manifestPath, 'utf8')).background.service_worker, second);

  const compiled = await build({ entryPoints: ['apps/extension/utils/update.ts'], bundle: true, write: false, format: 'cjs' });
  const module = { exports: {} };
  vm.runInNewContext(compiled.outputFiles[0].text, { module, exports: module.exports });
  let preferences = { lastQuestId: 'trading' };
  let reloads = 0;
  const api = {
    storage: { local: { get: async () => preferences, set: async items => Object.assign(preferences, items) } },
    runtime: { reload: () => { assert.equal(preferences.backgroundReloadVersion, '0.2.4'); reloads++; } },
  };
  assert.equal(await module.exports.reloadOutdatedBackground(api), true);
  assert.equal(await module.exports.reloadOutdatedBackground(api), false);
  assert.equal(reloads, 1, 'A mismatched worker must not trigger an endless reload loop');
  assert.equal(preferences.lastQuestId, 'trading', 'Recovery must preserve user preferences');
  console.log('Update regression checks passed: new worker URL, idempotent install, one-time recovery, preference preservation.');
} finally {
  rmSync(directory, { recursive: true });
}

// Exercise the worker actually referenced by the production manifest, not just source code.
const output = 'apps/extension/.output/chrome-mv3';
const manifest = JSON.parse(readFileSync(join(output, 'manifest.json'), 'utf8'));
assert.match(manifest.background.service_worker, /^background-[a-f0-9]{16}\.js$/);
const workerPath = join(output, manifest.background.service_worker);
assert.ok(existsSync(workerPath));
let listener;
const tab = { id: 7, windowId: 3, url: 'https://example.com', title: 'Example', index: 0, active: true, highlighted: true };
const chrome = {
  runtime: { id: 'test-extension', onMessage: { addListener: callback => listener = callback } },
  windows: { getCurrent: async () => ({ id: 99 }), getAll: async () => [{ id: 3, tabs: [tab] }] },
};
vm.runInNewContext(readFileSync(workerPath, 'utf8'), { chrome, URL, console });
const response = await listener({ type: 'extension_preview', scope: 'current_window', window_id: 3 }, {});
assert.equal(response.ok, true);
assert.equal(response.data.protocol_version, 4);
assert.equal(response.data.suggested_ids[0], 7);
assert.equal(response.data.tabs[0].window_id, 3);
console.log('Production manifest/worker preview check passed.');
