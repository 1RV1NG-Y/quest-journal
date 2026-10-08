import { createHash } from 'node:crypto';
import { copyFileSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Chromium can retain an unpacked extension's old worker across browser restarts.
// A different script URL forces a new service-worker registration on reload.
export function fingerprintExtension(directory) {
  const manifestPath = join(directory, 'manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const worker = manifest.background?.service_worker;
  if (!worker) return; // Firefox builds do not use an MV3 service worker.
  const source = join(directory, worker);
  const hash = createHash('sha256').update(readFileSync(source)).update(manifest.version).digest('hex').slice(0, 16);
  const filename = `background-${hash}.js`;
  if (worker !== filename) copyFileSync(source, join(directory, filename));
  manifest.background.service_worker = filename;
  writeFileSync(manifestPath, JSON.stringify(manifest));
  return filename;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const filename = fingerprintExtension(join(root, 'apps/extension/.output/chrome-mv3'));
  console.log(`Extension background: ${filename}`);
}
