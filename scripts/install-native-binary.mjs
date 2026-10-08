import { chmodSync, copyFileSync, mkdirSync, renameSync, rmSync } from 'node:fs';
import { dirname } from 'node:path';

// Browser manifests must never depend on a Cargo build directory surviving.
export function installNativeBinary(source, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  const staged = `${destination}.new-${process.pid}`;
  try {
    copyFileSync(source, staged);
    chmodSync(staged, 0o755);
    renameSync(staged, destination);
  } finally {
    rmSync(staged, { force: true });
  }
  return destination;
}

export function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}
