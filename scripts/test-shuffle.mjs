import assert from 'node:assert/strict';
import vm from 'node:vm';
import { build } from 'esbuild';
const bundled = await build({ entryPoints: ['apps/desktop/src/lib/shuffle.ts'], bundle: true, write: false, format: 'cjs' });
const module = { exports: {} };
vm.runInNewContext(bundled.outputFiles[0].text, { module, exports: module.exports });
const { buildShufflePool, pickShuffleItem } = module.exports;
const quest = (id, parent_id = null) => ({ id, parent_id, title: id, state: 'paused', materials: [], files: [], trashed_materials: [] });
const quests = [quest('Trading'), quest('Course', 'Trading'), quest('Lesson', 'Course'), quest('Career'), quest('Practice', 'Career')];
quests[1].materials = [{ id: 'tab1', resource_uri: 'https://example.com/lesson', state_json: { title: 'Lecture' } }];
quests[1].trashed_materials = [{ id: 'trash', resource_uri: 'https://example.com/deleted' }];
quests[1].latest_save = { resources: [{ id: 'historic', resource_uri: 'https://example.com/old' }] };
quests[2].files = [{ id: 'file1', path: '/notes/review.md', label: '' }];
quests[4].materials = [{ id: 'tab2', resource_uri: 'https://example.com/practice', state_json: null }];
const all = { topLevel: true, subquests: true, elements: true };
const keys = pool => Array.from(pool, item => item.key);
for (let mask = 0; mask < 8; mask++) {
  const filters = { topLevel: !!(mask & 1), subquests: !!(mask & 2), elements: !!(mask & 4) };
  const pool = buildShufflePool(quests, null, filters);
  assert.equal(pool.length, (filters.topLevel ? 2 : 0) + (filters.subquests ? 3 : 0) + (filters.elements ? 3 : 0));
  assert.equal(new Set(keys(pool)).size, pool.length);
  assert.ok(pool.every(item => !['trash', 'historic'].includes(item.id)));
}
assert.deepEqual(keys(buildShufflePool(quests, 'Trading', all)), ['quest:Trading', 'quest:Course', 'tab:tab1', 'quest:Lesson', 'file:file1']);
assert.deepEqual(keys(buildShufflePool(quests, 'Course', all)), ['quest:Course', 'tab:tab1', 'quest:Lesson', 'file:file1']);
assert.equal(buildShufflePool(quests, 'Course', { topLevel: true, subquests: false, elements: false }).length, 0);
assert.equal(buildShufflePool(quests, 'missing', all).length, 0);
assert.equal(buildShufflePool([], null, all).length, 0);
const pool = buildShufflePool(quests, null, all);
assert.equal(pool.find(item => item.id === 'Lesson').context, 'Trading → Course · Subquest');
assert.equal(pool.find(item => item.id === 'file1').context, 'Trading → Course → Lesson · File');
assert.equal(pool.find(item => item.id === 'file1').title, 'review.md');
assert.equal(pool.find(item => item.id === 'tab2').title, 'https://example.com/practice');
assert.equal(pickShuffleItem([], () => 0), null);
assert.equal(pickShuffleItem([pool[0]], () => 0.99), pool[0]);
// Each equal-sized interval selects exactly one eligible item, including endpoints.
for (let index = 0; index < pool.length; index++) {
  assert.equal(pickShuffleItem(pool, () => (index + 0.5) / pool.length), pool[index]);
}
assert.equal(pickShuffleItem(pool, () => 0), pool[0]);
assert.equal(pickShuffleItem(pool, () => 1 - Number.EPSILON), pool.at(-1));
// Defensive traversal must terminate even with a malformed hierarchy.
assert.equal(buildShufflePool([quest('a', 'b'), quest('b', 'a')], 'a', all).length, 2);
console.log('Shuffle checks passed: all filter combinations, recursive scope, contextual paths, current-only materials, empty/single pools, equal item odds, malformed hierarchy.');
