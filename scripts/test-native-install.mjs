import assert from 'node:assert/strict';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { installNativeBinary, shellQuote } from './install-native-binary.mjs';
import { checkNativeHost } from './check-native-host.mjs';

const directory = mkdtempSync(join(tmpdir(), 'quest-native-install-'));
try {
  const source = join(directory, 'temporary-build');
  copyFileSync(resolve('target/release/quest-native-host'), source);
  const installed = join(directory, "installed $literal ' host", 'quest-native-host');
  installNativeBinary(source, installed);
  installNativeBinary(source, installed); // Atomic reinstall remains usable.
  assert.deepEqual(readFileSync(source), readFileSync(installed));
  assert.ok(statSync(installed).mode & 0o111);
  rmSync(source);

  const database = join(directory, "database $literal ' name.sqlite3");
  const wrapper = join(directory, 'native-host-wrapper');
  writeFileSync(wrapper, `#!/bin/sh\nexport QUEST_JOURNAL_DB=${shellQuote(database)}\nexec ${shellQuote(installed)}\n`, { mode: 0o700 });
  const response = await checkNativeHost(wrapper);
  assert.equal(response.ok, true);
  assert.deepEqual(response.data, []);
  console.log('Native installer regression passed: permanent executable works after build removal, reinstall is safe, quoted wrapper paths work, framed response is valid.');
} finally {
  rmSync(directory, { recursive: true });
}
