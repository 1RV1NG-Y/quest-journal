#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import {
  cpSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { homedir, platform } from 'node:os';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { fingerprintExtension } from './fingerprint-extension.mjs';
import { installNativeBinary, shellQuote, writeInstalledJson } from './install-native-binary.mjs';
import { defaultDatabasePath, validateBrowser, windowsInstallPlan } from './native-install-plan.mjs';

const HOST_NAME = 'com.quest_journal.native_host';
const CHROME_EXTENSION_ID = 'jbaajfphjklfaifahgoeingbnejcehlp';
const FIREFOX_EXTENSION_ID = 'quest-journal@local';
const SCRIPT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(SCRIPT_DIRECTORY, '..');

function usage() {
  console.log(`Install the Quest Journal browser native-messaging host.

Usage:
  node scripts/install-native-host.mjs [--binary PATH] [--extension PATH] [--helium]
  node scripts/install-native-host.mjs [--binary PATH] [--extension PATH] [--brave-flatpak]
  node scripts/install-native-host.mjs [--binary PATH] --browser chrome|chromium|edge|brave|firefox|helium|brave-flatpak

With no browser flag, installed Helium and Brave Flatpak profiles are detected first.
The extension defaults to apps/extension/.output/chrome-mv3 when that build exists.
The binary defaults to QUEST_JOURNAL_NATIVE_HOST or target/release/quest-native-host.
It is copied to a permanent per-user location; installed browsers never depend on the build directory.
On Windows, Chrome, Chromium, Edge and Brave manifests are registered by default.
The extension is copied to LOCALAPPDATA/Quest Journal/browser-extension for Load unpacked.`);
  console.log('Windows supports Chrome, Chromium, Edge and Brave. Firefox is supported only on Linux/macOS.\nClose browser native-host connections before updating to a changed native-host build.');
}

let binaryArgument;
let extensionArgument;
const requestedBrowsers = new Set();
for (let index = 2; index < process.argv.length; index += 1) {
  const argument = process.argv[index];
  if (argument === '--help' || argument === '-h') {
    usage();
    process.exit(0);
  }
  if (argument === '--binary') {
    binaryArgument = process.argv[index + 1];
    if (!binaryArgument) throw new Error('--binary requires a path.');
    index += 1;
    continue;
  }
  if (argument.startsWith('--binary=')) {
    binaryArgument = argument.slice('--binary='.length);
    continue;
  }
  if (argument === '--extension') {
    extensionArgument = process.argv[index + 1];
    if (!extensionArgument) throw new Error('--extension requires a path.');
    index += 1;
    continue;
  }
  if (argument.startsWith('--extension=')) {
    extensionArgument = argument.slice('--extension='.length);
    continue;
  }
  if (argument === '--browser') {
    const browser = process.argv[index + 1];
    if (!browser) throw new Error('--browser requires a supported browser name.');
    requestedBrowsers.add(browser);
    index += 1;
    continue;
  }
  if (argument.startsWith('--browser=')) {
    requestedBrowsers.add(argument.slice('--browser='.length));
    continue;
  }
  if (
    argument === '--chrome' ||
    argument === '--chromium' ||
    argument === '--edge' ||
    argument === '--brave' ||
    argument === '--firefox' ||
    argument === '--helium' ||
    argument === '--brave-flatpak'
  ) {
    requestedBrowsers.add(argument.slice(2));
    continue;
  }
  throw new Error(`Unknown argument: ${argument}`);
}

if (requestedBrowsers.size === 0) {
  const home = homedir();
  if (platform() === 'linux' && existsSync(join(home, '.config', 'net.imput.helium'))) requestedBrowsers.add('helium');
  if (platform() === 'linux' && existsSync(join(home, '.var', 'app', 'com.brave.Browser'))) {
    requestedBrowsers.add('brave-flatpak');
  }
  if (requestedBrowsers.size === 0) {
    requestedBrowsers.add('chrome');
    requestedBrowsers.add('chromium');
    if (platform() === 'win32') {
      requestedBrowsers.add('edge');
      requestedBrowsers.add('brave');
    }
  }
}
for (const browser of requestedBrowsers) {
  validateBrowser(browser, platform());
}

const executableName = platform() === 'win32' ? 'quest-native-host.exe' : 'quest-native-host';
const configuredBinary =
  binaryArgument ??
  process.env.QUEST_JOURNAL_NATIVE_HOST ??
  join(REPOSITORY_ROOT, 'target', 'release', executableName);
const binaryPath = isAbsolute(configuredBinary)
  ? configuredBinary
  : resolve(process.cwd(), configuredBinary);

if (!existsSync(binaryPath)) {
  throw new Error(
    `Native host binary does not exist: ${binaryPath}\nBuild quest-native-host or pass --binary with its absolute path.`,
  );
}
const binaryStat = statSync(binaryPath);
if (!binaryStat.isFile()) {
  throw new Error(`Native host path is not a file: ${binaryPath}`);
}
if (platform() !== 'win32' && (binaryStat.mode & 0o111) === 0) {
  throw new Error(`Native host binary is not executable: ${binaryPath}`);
}
const sharedDatabasePath =
  defaultDatabasePath(platform(), homedir(), process.env);
const installedBinaryPath = join(dirname(sharedDatabasePath), 'bin', executableName);

const defaultExtension = join(
  REPOSITORY_ROOT,
  'apps',
  'extension',
  '.output',
  'chrome-mv3',
);
const configuredExtension =
  extensionArgument ?? (existsSync(defaultExtension) ? defaultExtension : undefined);
const extensionSource = configuredExtension
  ? isAbsolute(configuredExtension)
    ? configuredExtension
    : resolve(process.cwd(), configuredExtension)
  : undefined;
if (
  extensionSource &&
  (!existsSync(join(extensionSource, 'manifest.json')) ||
    !statSync(extensionSource).isDirectory())
) {
  throw new Error(`Extension build is invalid: ${extensionSource}`);
}

function manifestPathFor(browser) {
  const currentPlatform = platform();
  const home = homedir();

  if (currentPlatform === 'linux') {
    if (browser === 'helium') {
      return join(home, '.config', 'net.imput.helium', 'NativeMessagingHosts', `${HOST_NAME}.json`);
    }
    if (browser === 'brave-flatpak') {
      return join(
        home,
        '.var',
        'app',
        'com.brave.Browser',
        'config',
        'BraveSoftware',
        'Brave-Browser',
        'NativeMessagingHosts',
        `${HOST_NAME}.json`,
      );
    }
    if (browser === 'chrome') {
      return join(home, '.config', 'google-chrome', 'NativeMessagingHosts', `${HOST_NAME}.json`);
    }
    if (browser === 'chromium') {
      return join(home, '.config', 'chromium', 'NativeMessagingHosts', `${HOST_NAME}.json`);
    }
    return join(home, '.mozilla', 'native-messaging-hosts', `${HOST_NAME}.json`);
  }

  if (currentPlatform === 'darwin') {
    if (browser === 'chrome') {
      return join(
        home,
        'Library',
        'Application Support',
        'Google',
        'Chrome',
        'NativeMessagingHosts',
        `${HOST_NAME}.json`,
      );
    }
    if (browser === 'chromium') {
      return join(
        home,
        'Library',
        'Application Support',
        'Chromium',
        'NativeMessagingHosts',
        `${HOST_NAME}.json`,
      );
    }
    return join(
      home,
      'Library',
      'Application Support',
      'Mozilla',
      'NativeMessagingHosts',
      `${HOST_NAME}.json`,
    );
  }

  if (currentPlatform === 'win32') {
    return windowsInstallPlan(browser, process.env, sharedDatabasePath).manifestPath;
  }

  throw new Error(`Unsupported operating system: ${currentPlatform}`);
}

function registerWindowsManifest(browser) {
  if (platform() !== 'win32') return;
  execFileSync('reg.exe', windowsInstallPlan(browser, process.env, sharedDatabasePath).registryArgs, { stdio: 'inherit' });
}

function grantBraveFlatpakAccess() {
  mkdirSync(dirname(sharedDatabasePath), { recursive: true });
  execFileSync(
    'flatpak',
    [
      'override',
      '--user',
      `--filesystem=${dirname(installedBinaryPath)}:ro`,
      `--filesystem=${dirname(sharedDatabasePath)}:rw`,
      'com.brave.Browser',
    ],
    { stdio: 'inherit' },
  );
}

function nativeHostPathFor(browser) {
  if (platform() === 'win32') {
    const plan = windowsInstallPlan(browser, process.env, sharedDatabasePath);
    installNativeBinary(binaryPath, plan.executablePath);
    writeInstalledJson(plan.configPath, plan.config);
    return plan.executablePath;
  }
  if (!['helium', 'brave-flatpak'].includes(browser)) return installedBinaryPath;
  const wrapperPath =
    browser === 'helium'
      ? join(homedir(), '.local', 'share', 'questjournal', 'native-host-helium')
      : join(
          homedir(),
          '.var',
          'app',
          'com.brave.Browser',
          'data',
          'quest-journal',
          'native-host-brave',
        );
  mkdirSync(dirname(wrapperPath), { recursive: true });
  writeFileSync(
    wrapperPath,
    `#!/bin/sh\nexport QUEST_JOURNAL_DB=${shellQuote(sharedDatabasePath)}\nexec ${shellQuote(installedBinaryPath)}\n`,
    { mode: 0o700 },
  );
  return wrapperPath;
}

function addExtensionFlag(desktopEntry, extensionPath) {
  const flag = `--load-extension=${extensionPath}`;
  return desktopEntry
    .split('\n')
    .map((line) => {
      if (!line.startsWith('Exec=') || line.includes('--load-extension=')) return line;
      if (line.includes(' @@u ')) return line.replace(' @@u ', ` ${flag} @@u `);
      if (line.includes(' %U')) return line.replace(' %U', ` ${flag} %U`);
      return `${line} ${flag}`;
    })
    .join('\n');
}

function installBrowserExtension(browser) {
  if (!extensionSource || (platform() !== 'win32' && !['helium', 'brave-flatpak'].includes(browser))) return;
  const home = homedir();
  const destination =
    platform() === 'win32'
      ? windowsInstallPlan(browser, process.env, sharedDatabasePath).extensionPath
      : browser === 'helium'
      ? join(home, '.local', 'share', 'questjournal', 'browser-extension')
      : join(
          home,
          '.var',
          'app',
          'com.brave.Browser',
          'data',
          'quest-journal',
          'browser-extension',
        );
  mkdirSync(destination, { recursive: true });
  fingerprintExtension(extensionSource);
  // Install assets before switching the manifest to their new worker URL.
  cpSync(extensionSource, destination, {
    recursive: true,
    force: true,
    filter: (source) => source !== join(extensionSource, 'manifest.json'),
  });
  const stagedManifest = join(destination, `manifest.new-${process.pid}.json`);
  try {
    copyFileSync(join(extensionSource, 'manifest.json'), stagedManifest);
    renameSync(stagedManifest, join(destination, 'manifest.json'));
  } finally {
    rmSync(stagedManifest, { force: true });
  }

  if (platform() === 'win32') {
    console.log(`Load unpacked in ${browser}'s extensions developer page: ${destination}`);
  } else if (browser === 'helium') {
    const launcher = join(home, '.local', 'share', 'applications', 'helium.desktop');
    if (!existsSync(launcher)) throw new Error(`Helium launcher does not exist: ${launcher}`);
    const contents = addExtensionFlag(readFileSync(launcher, 'utf8'), destination);
    writeFileSync(launcher, contents);
  } else {
    const systemLauncher = '/var/lib/flatpak/exports/share/applications/com.brave.Browser.desktop';
    const userLauncher = join(
      home,
      '.local',
      'share',
      'applications',
      'com.brave.Browser.desktop',
    );
    if (!existsSync(systemLauncher)) {
      throw new Error(`Brave Flatpak launcher does not exist: ${systemLauncher}`);
    }
    mkdirSync(dirname(userLauncher), { recursive: true });
    const contents = addExtensionFlag(readFileSync(systemLauncher, 'utf8'), destination);
    writeFileSync(userLauncher, contents);
  }
  console.log(`Installed ${browser} extension build: ${destination}`);
}

if (platform() !== 'win32') {
  installNativeBinary(binaryPath, installedBinaryPath);
  console.log(`Installed native host executable: ${installedBinaryPath}`);
}

for (const browser of requestedBrowsers) {
  const manifest = {
    name: HOST_NAME,
    description: 'Quest Journal local browser checkpoint bridge',
    path: nativeHostPathFor(browser),
    type: 'stdio',
    ...(browser === 'firefox'
      ? { allowed_extensions: [FIREFOX_EXTENSION_ID] }
      : { allowed_origins: [`chrome-extension://${CHROME_EXTENSION_ID}/`] }),
  };
  const manifestPath = manifestPathFor(browser);
  mkdirSync(dirname(manifestPath), { recursive: true });
  writeInstalledJson(manifestPath, manifest);
  registerWindowsManifest(browser);
  if (browser === 'brave-flatpak') grantBraveFlatpakAccess();
  installBrowserExtension(browser);
  console.log(`Installed ${browser} native-host manifest: ${manifestPath}`);
}
