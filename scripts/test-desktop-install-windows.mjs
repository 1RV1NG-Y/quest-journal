import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { appendFileSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installWindowsDesktop } from './install-windows-desktop.mjs';
import { readWindowsShortcut } from './windows-shortcut.mjs';

if (process.platform !== 'win32') throw new Error('Run this integration test on Windows.');
// CI's TEMP may use an 8.3 alias (RUNNER~1). Shell links expand that alias;
// start with the canonical path so assertions isolate Unicode preservation.
const directory = realpathSync.native(mkdtempSync(join(tmpdir(), 'quest-desktop-install-')));
try {
  const source = join(directory, 'build.exe');
  // CI can first exercise the installer with an existing GUI executable, then
  // repeat the same checks against the real release build. Neither is launched.
  copyFileSync(resolve(process.argv[2] ?? 'target/release/quest-desktop.exe'), source);
  const executable = readFileSync(source);
  const peOffset = executable.readUInt32LE(0x3c);
  assert.equal(executable.toString('ascii', peOffset, peOffset + 4), 'PE\0\0');
  assert.equal(executable.readUInt16LE(peOffset + 24 + 68), 2, 'Release desktop must use the Windows GUI subsystem');
  const options = { localAppData: join(directory, "local 日本語 ñ ' $"), appData: join(directory, "roaming 中文 Ж ' $") };
  assert.throws(() => installWindowsDesktop(source, { ...options, localAppData: 'relative' }), /absolute paths/);
  const result = installWindowsDesktop(source, options);
  const journal = join(options.appData, 'Quest Journal', 'Quest Journal', 'data', 'quests.sqlite3');
  mkdirSync(dirname(journal), { recursive: true });
  writeFileSync(journal, 'existing journal sentinel');
  const cli = fileURLToPath(new URL('./install-desktop.mjs', import.meta.url));
  const lockedUpdate = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    "$ErrorActionPreference = 'Stop'; $file = [System.IO.File]::Open($env:QUEST_TEST_BINARY, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::Read); try { $ErrorActionPreference = 'Continue'; & $env:QUEST_TEST_NODE $env:QUEST_TEST_INSTALLER '--binary' $env:QUEST_TEST_SOURCE 2>&1 | Out-String | Write-Output; $status = $LASTEXITCODE } finally { $file.Dispose() }; $ErrorActionPreference = 'Stop'; if ($status -eq 0) { throw 'Installation unexpectedly replaced a locked executable' }"],
  { encoding: 'utf8', windowsHide: true, env: { ...process.env, LOCALAPPDATA: options.localAppData, APPDATA: options.appData, QUEST_TEST_BINARY: result.binary, QUEST_TEST_NODE: process.execPath, QUEST_TEST_INSTALLER: cli, QUEST_TEST_SOURCE: source } });
  assert.match(lockedUpdate, /Close the application and rerun the installer/);
  assert.deepEqual(readFileSync(result.binary), executable, 'A blocked update must preserve the previous executable');
  assert.ok(readdirSync(dirname(result.binary)).every((name) => !name.includes('.new-')), 'A blocked update must clean up its staged executable');
  appendFileSync(source, '\nreinstall marker\n');
  installWindowsDesktop(source, options);
  assert.equal(readFileSync(journal, 'utf8'), 'existing journal sentinel');
  assert.deepEqual(readFileSync(source), readFileSync(result.binary));
  rmSync(source);
  const shortcut = readWindowsShortcut(result.shortcut);
  assert.ok(readFileSync(result.shortcut).includes(Buffer.from(result.binary, 'utf16le')), 'The saved shortcut must contain the exact Unicode target');
  assert.equal(shortcut.target, result.binary);
  assert.equal(shortcut.workingDirectory, dirname(result.binary));
  assert.equal(shortcut.icon, `${result.binary},0`);
  assert.equal(shortcut.arguments, '');
  console.log('Windows desktop install passed: GUI executable, permanent shortcut with Unicode paths, safe reinstall, blocked-update preservation, and existing journal data.');
} finally {
  rmSync(directory, { recursive: true, force: true });
}
