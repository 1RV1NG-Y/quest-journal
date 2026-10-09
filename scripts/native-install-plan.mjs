import { posix, win32 } from 'node:path';

export const HOST_NAME = 'com.quest_journal.native_host';
const WINDOWS_VENDORS = {
  chrome: 'Google\\Chrome',
  chromium: 'Chromium',
  edge: 'Microsoft\\Edge',
  brave: 'BraveSoftware\\Brave-Browser',
};

export function validateBrowser(browser, platform) {
  const supported = platform === 'win32'
    ? Object.keys(WINDOWS_VENDORS)
    : platform === 'linux'
      ? ['chrome', 'chromium', 'firefox', 'helium', 'brave-flatpak']
      : platform === 'darwin' ? ['chrome', 'chromium', 'firefox'] : [];
  if (!supported.includes(browser)) throw new Error(`Unsupported browser “${browser}” on ${platform}.`);
}

export function defaultDatabasePath(platform, home, env) {
  if (env.QUEST_JOURNAL_DB !== undefined) {
    const paths = platform === 'win32' ? win32 : posix;
    if (!env.QUEST_JOURNAL_DB || !paths.isAbsolute(env.QUEST_JOURNAL_DB)) throw new Error('QUEST_JOURNAL_DB must be a nonempty absolute path.');
    if (platform === 'win32') validateWindowsPath(env.QUEST_JOURNAL_DB, 'QUEST_JOURNAL_DB');
    return env.QUEST_JOURNAL_DB;
  }
  if (platform === 'win32') {
    if (!env.APPDATA) throw new Error('APPDATA is required on Windows.');
    validateWindowsPath(env.APPDATA, 'APPDATA');
    // directories::ProjectDirs::from("com", "Quest Journal", "Quest Journal").data_dir()
    return win32.join(env.APPDATA, 'Quest Journal', 'Quest Journal', 'data', 'quests.sqlite3');
  }
  if (platform === 'darwin') return posix.join(home, 'Library', 'Application Support', 'com.Quest-Journal.Quest-Journal', 'quests.sqlite3');
  const dataHome = env.XDG_DATA_HOME && posix.isAbsolute(env.XDG_DATA_HOME)
    ? env.XDG_DATA_HOME
    : posix.join(home, '.local', 'share');
  return posix.join(dataHome, 'questjournal', 'quests.sqlite3');
}

export function windowsInstallPlan(browser, env, database) {
  validateBrowser(browser, 'win32');
  if (!env.LOCALAPPDATA) throw new Error('LOCALAPPDATA is required on Windows.');
  validateWindowsPath(env.LOCALAPPDATA, 'LOCALAPPDATA');
  validateWindowsPath(database, 'Native host database');
  const directory = win32.join(env.LOCALAPPDATA, 'Quest Journal', 'NativeMessagingHosts', browser);
  const manifestPath = win32.join(directory, `${HOST_NAME}.json`);
  return {
    directory,
    manifestPath,
    executablePath: win32.join(directory, `quest-native-host-${browser}.exe`),
    configPath: win32.join(directory, 'quest-native-host.json'),
    config: { database },
    extensionPath: win32.join(env.LOCALAPPDATA, 'Quest Journal', 'browser-extension'),
    registryArgs: ['ADD', `HKCU\\Software\\${WINDOWS_VENDORS[browser]}\\NativeMessagingHosts\\${HOST_NAME}`, '/ve', '/t', 'REG_SZ', '/d', manifestPath, '/f'],
  };
}

function validateWindowsPath(path, label) {
  // A rooted path like \\data\\quests.sqlite3 still depends on the current drive.
  // The browser can launch the host on a different drive than this installer.
  if (typeof path !== 'string' || !win32.isAbsolute(path) || !/^(?:[a-z]:[\\/]|[\\/]{2}[^\\/]+[\\/][^\\/]+(?:[\\/]|$))/i.test(path)) {
    throw new Error(`${label} must be an absolute Windows path with a drive or UNC share.`);
  }
}
