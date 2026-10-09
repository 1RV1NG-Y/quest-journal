import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, renameSync, rmSync, statSync } from 'node:fs';
import { dirname, isAbsolute, join } from 'node:path';

export function installWindowsDesktop(source, { localAppData = process.env.LOCALAPPDATA, appData = process.env.APPDATA } = {}) {
  if (process.platform !== 'win32') throw new Error('Windows desktop installation requires Windows.');
  if (!localAppData || !appData) throw new Error('LOCALAPPDATA and APPDATA are required.');
  if (!isAbsolute(localAppData) || !isAbsolute(appData)) throw new Error('LOCALAPPDATA and APPDATA must be absolute paths.');
  if (!statSync(source).isFile()) throw new Error(`Desktop binary is not a file: ${source}`);
  const expectedBinary = readFileSync(source);
  const binary = join(localAppData, 'Quest Journal', 'bin', 'quest-journal.exe');
  const shortcut = join(appData, 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Quest Journal.lnk');
  mkdirSync(dirname(binary), { recursive: true });
  mkdirSync(dirname(shortcut), { recursive: true });
  const staged = `${binary}.new-${process.pid}`;
  try {
    copyFileSync(source, staged);
    try {
      renameSync(staged, binary);
    } catch (error) {
      throw new Error('Could not replace Quest Journal. Close the application and rerun the installer. Journal data has been preserved.', { cause: error });
    }
  } finally {
    rmSync(staged, { force: true });
  }
  if (!expectedBinary.equals(readFileSync(binary))) throw new Error('Installed executable does not match the build.');
  // Pass paths through environment variables, never interpolate them as PowerShell code.
  execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    "$ErrorActionPreference = 'Stop'; $shell = New-Object -ComObject WScript.Shell; $link = $shell.CreateShortcut($env:QUEST_DESKTOP_SHORTCUT); $link.TargetPath = $env:QUEST_DESKTOP_BINARY; $link.WorkingDirectory = [System.IO.Path]::GetDirectoryName($env:QUEST_DESKTOP_BINARY); $link.Description = 'Quest Journal'; $link.IconLocation = $env:QUEST_DESKTOP_BINARY + ',0'; $link.Save()"],
  { stdio: 'pipe', windowsHide: true, env: { ...process.env, QUEST_DESKTOP_BINARY: binary, QUEST_DESKTOP_SHORTCUT: shortcut } });
  return { binary, shortcut };
}
