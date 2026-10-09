#!/usr/bin/env node

import { installWindowsDesktop } from './install-windows-desktop.mjs';
import { execFileSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { homedir, platform } from 'node:os';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (!['linux', 'win32'].includes(platform())) throw new Error('The desktop installer supports Linux and Windows.');

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, '..');
let binaryArgument;
let iconArgument;

for (let index = 2; index < process.argv.length; index += 1) {
  const argument = process.argv[index];
  if (argument === '--binary' || argument === '--icon') {
    const value = process.argv[index + 1];
    if (!value) throw new Error(`${argument} requires a path.`);
    if (argument === '--binary') binaryArgument = value;
    else iconArgument = value;
    index += 1;
    continue;
  }
  if (argument.startsWith('--binary=')) {
    binaryArgument = argument.slice('--binary='.length);
    continue;
  }
  if (argument.startsWith('--icon=')) {
    iconArgument = argument.slice('--icon='.length);
    continue;
  }
  throw new Error(`Unknown argument: ${argument}`);
}

const configuredBinary =
  binaryArgument ?? join(repositoryRoot, 'target', 'release', platform() === 'win32' ? 'quest-desktop.exe' : 'quest-desktop');
const sourceBinary = isAbsolute(configuredBinary)
  ? configuredBinary
  : resolve(process.cwd(), configuredBinary);
const configuredIcon =
  iconArgument ?? join(repositoryRoot, 'apps', 'desktop', 'src-tauri', 'icons', 'icon.png');
const sourceIcon = isAbsolute(configuredIcon)
  ? configuredIcon
  : resolve(process.cwd(), configuredIcon);
validateFile(sourceBinary, 'desktop binary');
if (platform() === 'win32') {
  const result = installWindowsDesktop(sourceBinary);
  console.log(`Installed executable: ${result.binary}`);
  console.log(`Installed Start Menu shortcut: ${result.shortcut}`);
  console.log('Close and reopen Quest Journal to use this build. Existing journal data is preserved.');
  process.exit(0);
}
validateFile(sourceIcon, 'application icon');

const home = homedir();
const installedBinary = join(home, '.local', 'bin', 'quest-journal');
const installedIcon = join(
  home,
  '.local',
  'share',
  'icons',
  'hicolor',
  '512x512',
  'apps',
  'quest-journal.png',
);
const desktopEntry = join(home, '.local', 'share', 'applications', 'quest-journal.desktop');

mkdirSync(dirname(installedBinary), { recursive: true });
mkdirSync(dirname(installedIcon), { recursive: true });
mkdirSync(dirname(desktopEntry), { recursive: true });
// Replace atomically so an already-running app can finish on its old executable.
const stagedBinary = `${installedBinary}.new-${process.pid}`;
copyFileSync(sourceBinary, stagedBinary);
chmodSync(stagedBinary, 0o755);
renameSync(stagedBinary, installedBinary);
copyFileSync(sourceIcon, installedIcon);
chmodSync(installedIcon, 0o644);
writeFileSync(
  desktopEntry,
  `[Desktop Entry]\nVersion=1.0\nType=Application\nName=Quest Journal\nGenericName=Browser Context Journal\nComment=Pause browser contexts and continue them later\nExec=${installedBinary}\nIcon=quest-journal\nTerminal=false\nCategories=Utility;\nKeywords=quests;tabs;browser;session;workspace;journal;\nStartupNotify=true\nStartupWMClass=Quest-desktop\n`,
  { mode: 0o644 },
);
chmodSync(desktopEntry, 0o644);

refreshCache('update-desktop-database', [dirname(desktopEntry)]);
refreshCache('gtk-update-icon-cache', ['-f', '-t', join(home, '.local', 'share', 'icons', 'hicolor')]);

console.log(`Installed executable: ${installedBinary}`);
console.log(`Installed icon: ${installedIcon}`);
console.log(`Installed GNOME launcher: ${desktopEntry}`);


function validateFile(path, label) {
  if (!existsSync(path) || !statSync(path).isFile()) {
    throw new Error(`${label} does not exist: ${path}`);
  }
}

function refreshCache(command, arguments_) {
  try {
    execFileSync(command, arguments_, { stdio: 'ignore' });
  } catch {
    console.warn(`Could not run ${command}; GNOME will discover the launcher after its next refresh.`);
  }
}
