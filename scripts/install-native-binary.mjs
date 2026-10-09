import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

// Browser manifests must never depend on a Cargo build directory surviving.
export function installNativeBinary(source, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  // Windows locks running executables. An unchanged build needs no replacement.
  if (existsSync(destination) && readFileSync(source).equals(readFileSync(destination))) {
    if (process.platform !== 'win32') chmodSync(destination, 0o755);
    return destination;
  }
  const staged = `${destination}.new-${process.pid}`;
  try {
    copyFileSync(source, staged);
    chmodSync(staged, 0o755);
    renameSync(staged, destination);
  } catch (error) {
    if (process.platform === 'win32' && ['EACCES', 'EPERM', 'EBUSY'].includes(error.code)) {
      throw new Error(`Cannot update native host ${destination}. Close browser native-host connections and retry; the existing installed host has been preserved.`, { cause: error });
    }
    throw error;
  } finally {
    rmSync(staged, { force: true });
  }
  return destination;
}

export function writeInstalledJson(destination, value) {
  mkdirSync(dirname(destination), { recursive: true });
  const staged = `${destination}.new-${process.pid}`;
  try {
    writeFileSync(staged, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600 });
    renameSync(staged, destination);
  } finally {
    rmSync(staged, { force: true });
  }
}

export function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}
