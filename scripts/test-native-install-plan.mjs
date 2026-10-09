import assert from 'node:assert/strict';
import { defaultDatabasePath, validateBrowser, windowsInstallPlan } from './native-install-plan.mjs';

const env = { APPDATA: "C:\\Users\\O'Brien $name\\AppData\\Roaming", LOCALAPPDATA: "C:\\Users\\O'Brien $name\\AppData\\Local" };
const database = defaultDatabasePath('win32', 'ignored', env);
assert.equal(database, `${env.APPDATA}\\Quest Journal\\Quest Journal\\data\\quests.sqlite3`);
const vendors = { chrome: 'Google\\Chrome', chromium: 'Chromium', edge: 'Microsoft\\Edge', brave: 'BraveSoftware\\Brave-Browser' };
for (const [browser, vendor] of Object.entries(vendors)) {
  const plan = windowsInstallPlan(browser, env, database);
  assert.equal(plan.registryArgs[1], `HKCU\\Software\\${vendor}\\NativeMessagingHosts\\com.quest_journal.native_host`);
  assert.deepEqual(plan.registryArgs.slice(2), ['/ve', '/t', 'REG_SZ', '/d', plan.manifestPath, '/f']);
  assert.equal(plan.executablePath, `${env.LOCALAPPDATA}\\Quest Journal\\NativeMessagingHosts\\${browser}\\quest-native-host-${browser}.exe`);
  assert.equal(plan.configPath, `${plan.directory}\\quest-native-host.json`);
  assert.deepEqual(plan.config, { database });
  assert.equal(plan.extensionPath, `${env.LOCALAPPDATA}\\Quest Journal\\browser-extension`);
  assert.ok(!plan.executablePath.includes('target\\'));
  assert.ok(!plan.extensionPath.includes('.output'));
}
assert.equal(defaultDatabasePath('linux', '/home/test', {}), '/home/test/.local/share/questjournal/quests.sqlite3');
assert.equal(defaultDatabasePath('linux', '/home/test', { XDG_DATA_HOME: '/data' }), '/data/questjournal/quests.sqlite3');
assert.equal(defaultDatabasePath('linux', '/home/test', { XDG_DATA_HOME: 'relative' }), '/home/test/.local/share/questjournal/quests.sqlite3');
assert.equal(defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: 'D:\\Custom data\\quests.sqlite3' }), 'D:\\Custom data\\quests.sqlite3');
assert.equal(defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: 'D:/Custom data/quests.sqlite3' }), 'D:/Custom data/quests.sqlite3');
assert.equal(defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: '\\\\server\\share\\quests.sqlite3' }), '\\\\server\\share\\quests.sqlite3');
assert.equal(defaultDatabasePath('darwin', '/Users/test', {}), '/Users/test/Library/Application Support/com.Quest-Journal.Quest-Journal/quests.sqlite3');
assert.throws(() => defaultDatabasePath('win32', '', {}), /APPDATA/);
assert.throws(() => defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: 'relative.sqlite3' }), /absolute/);
assert.throws(() => defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: '' }), /absolute/);
assert.throws(() => defaultDatabasePath('win32', '', { QUEST_JOURNAL_DB: '\\relative-to-drive.sqlite3' }), /drive or UNC/);
assert.throws(() => defaultDatabasePath('win32', '', { APPDATA: 'relative' }), /absolute/);
assert.throws(() => windowsInstallPlan('chrome', {}, database), /LOCALAPPDATA/);
assert.throws(() => windowsInstallPlan('chrome', { LOCALAPPDATA: 'relative' }, database), /absolute/);
assert.throws(() => windowsInstallPlan('chrome', env, 'relative'), /absolute/);
assert.throws(() => validateBrowser('firefox', 'win32'), /Unsupported/);
assert.throws(() => validateBrowser('helium', 'win32'), /Unsupported/);
assert.throws(() => validateBrowser('brave-flatpak', 'darwin'), /Unsupported/);
assert.throws(() => validateBrowser('unknown', 'linux'), /Unsupported/);
console.log('Native installer plan checks passed: Windows registry, stable executable/extension, sidecar, platform validation, shared database paths.');
