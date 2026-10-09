import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { copyFileSync, existsSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { installNativeBinary, shellQuote, writeInstalledJson } from './install-native-binary.mjs';
import { defaultDatabasePath, windowsInstallPlan } from './native-install-plan.mjs';
import { checkNativeHost } from './check-native-host.mjs';

const windows = process.platform === 'win32';
const directory = mkdtempSync(join(tmpdir(), 'quest-native-install-'));
const origin = 'chrome-extension://jbaajfphjklfaifahgoeingbnejcehlp/';
const windowsBrowsers = ['chrome', 'chromium', 'edge', 'brave'];

// The diagnostic waits for process closure before releasing Windows executable
// and SQLite locks. Include the browser's actual origin/window arguments.
async function requestHost(command, request, env = process.env) {
  const response = await checkNativeHost(command, windows ? [origin, '--parent-window=0'] : [], { request, env });
  assert.equal(response.ok, true, response.error);
  return response.data;
}

try {
  const source = join(directory, windows ? 'temporary-build.exe' : 'temporary-build');
  copyFileSync(resolve('target/release', windows ? 'quest-native-host.exe' : 'quest-native-host'), source);
  const database = join(directory, "database $literal ' & ü name.sqlite3");
  let hosts;
  let hostEnvironment;
  if (windows) {
    const env = {
      APPDATA: join(directory, "roaming $literal ' & ü"),
      LOCALAPPDATA: join(directory, "local $literal ' & ü"),
      QUEST_JOURNAL_DB: database,
    };
    const sharedDatabase = defaultDatabasePath('win32', '', env);
    hosts = windowsBrowsers.map(browser => {
      const plan = windowsInstallPlan(browser, env, sharedDatabase);
      installNativeBinary(source, plan.executablePath);
      writeInstalledJson(plan.configPath, plan.config);
      return plan.executablePath;
    });
    // Browser sessions must use the installed sidecar even when the browser's
    // inherited environment differs from the one that ran this installer.
    hostEnvironment = { ...process.env, QUEST_JOURNAL_DB: join(directory, 'wrong-database.sqlite3') };
  } else {
    const installed = join(directory, "installed $literal ' host", 'quest-native-host');
    installNativeBinary(source, installed);
    assert.ok(statSync(installed).mode & 0o111);
    const wrapper = join(directory, 'native-host-wrapper');
    writeFileSync(wrapper, `#!/bin/sh\nexport QUEST_JOURNAL_DB=${shellQuote(database)}\nexec ${shellQuote(installed)}\n`, { mode: 0o700 });
    hosts = [wrapper];
  }

  assert.deepEqual(await requestHost(hosts[0], { type: 'list_quests' }, hostEnvironment), []);
  const quest = await requestHost(hosts[0], { type: 'create_quest', title: 'Installer persistence check' }, hostEnvironment);
  assert.ok(quest.id);
  for (const host of hosts) {
    const executable = windows ? host : join(directory, "installed $literal ' host", 'quest-native-host');
    installNativeBinary(source, executable);
    assert.deepEqual(readFileSync(source), readFileSync(executable));
    // A failed source copy must leave the installed host usable.
    assert.throws(() => installNativeBinary(join(directory, 'missing-build'), executable));
    assert.deepEqual(readFileSync(source), readFileSync(executable));
    assert.ok(!existsSync(`${executable}.new-${process.pid}`));
  }

  if (windows) {
    const child = spawn(hosts[0], [origin, '--parent-window=0'], {
      env: hostEnvironment, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
    });
    try {
      await once(child, 'spawn');
      installNativeBinary(source, hosts[0]); // Same build is safe even while running.
      // A PE overlay changes the file bytes without changing executable behavior.
      writeFileSync(source, Buffer.concat([readFileSync(source), Buffer.from('\ninstaller-update-test\n')]));
      const original = readFileSync(hosts[0]);
      assert.throws(() => installNativeBinary(source, hosts[0]), /Close browser native-host connections/);
      assert.deepEqual(readFileSync(hosts[0]), original, 'A locked-host update must preserve the installed build');
      assert.ok(!existsSync(`${hosts[0]}.new-${process.pid}`));
    } finally {
      const closed = once(child, 'close');
      child.stdin.end();
      const timer = setTimeout(() => child.kill(), 10000);
      await closed;
      clearTimeout(timer);
    }
    for (const executable of hosts) {
      installNativeBinary(source, executable);
      assert.deepEqual(readFileSync(source), readFileSync(executable));
    }
  }

  rmSync(source);
  for (const host of hosts) {
    const quests = await requestHost(host, { type: 'list_quests' }, hostEnvironment);
    assert.equal(quests.length, 1);
    assert.equal(quests[0].id, quest.id, 'Reinstallation must preserve the shared journal');
  }
  assert.ok(existsSync(database), 'The configured shared database must be used');
  if (windows) {
    assert.ok(!existsSync(hostEnvironment.QUEST_JOURNAL_DB), 'Inherited browser environment must not replace the sidecar database');
    for (const [index, host] of hosts.entries()) {
      await requestHost(host, {
        type: 'add_tabs', quest_id: quest.id,
        tabs: [{ id: index + 1, url: `https://example.com/${windowsBrowsers[index]}`, title: 'Browser identity check',
          index: 0, pinned: false, active: true, browser_kind: 'brave_flatpak' }],
      }, hostEnvironment);
    }
    const { DatabaseSync } = await import('node:sqlite');
    const stored = new DatabaseSync(database, { readOnly: true });
    try {
      const rows = stored.prepare('SELECT resource_uri,state_json FROM materials WHERE quest_id=?').all(quest.id);
      assert.equal(rows.length, windowsBrowsers.length);
      for (const row of rows) {
        const browser = new URL(row.resource_uri).pathname.slice(1);
        assert.equal(JSON.parse(row.state_json).browser_kind, browser, 'Installed executable must override extension browser identity');
      }
    } finally {
      stored.close();
    }
  }
  console.log(windows
    ? 'Windows native install passed: permanent per-browser executables and capture identity, sidecar database, framed responses with browser arguments, locked update preservation, reinstall and shared-journal persistence after build removal. Registry and interactive browser checks remain separate.'
    : 'Native installer regression passed: permanent executable after build removal, safe reinstall and failed-copy preservation, quoted wrapper paths, framed responses and journal persistence.');
} finally {
  rmSync(directory, { recursive: true, force: true });
}
