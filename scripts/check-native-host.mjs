import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function checkNativeHost(command, args = []) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let output = Buffer.alloc(0);
    let errors = '';
    let finished = false;
    const finish = (error, response) => {
      if (finished) return;
      finished = true;
      clearTimeout(timeout);
      child.kill(); // End only this diagnostic process after its first response.
      error ? reject(error) : resolve(response);
    };
    const timeout = setTimeout(() => finish(new Error(`Native host timed out. ${errors}`)), 15000);
    child.on('error', error => finish(error));
    child.stdin.on('error', error => finish(error));
    child.stderr.on('data', data => { errors += data.toString(); });
    child.on('close', code => { if (!finished) finish(new Error(`Native host exited before replying (${code}). ${errors}`)); });
    child.stdout.on('data', data => {
      output = Buffer.concat([output, data]);
      if (output.length < 4) return;
      const length = output.readUInt32LE(0);
      if (length > 1024 * 1024) return finish(new Error('Native host returned an oversized frame'));
      if (output.length < length + 4) return;
      try { finish(null, JSON.parse(output.subarray(4, length + 4).toString())); }
      catch (error) { finish(error); }
    });
    const payload = Buffer.from(JSON.stringify({ type: 'list_quests' }));
    const header = Buffer.alloc(4);
    header.writeUInt32LE(payload.length);
    child.stdin.end(Buffer.concat([header, payload]));
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) throw new Error('Provide the native host command and optional arguments.');
  const response = await checkNativeHost(process.argv[2], process.argv.slice(3));
  if (!response.ok || !Array.isArray(response.data)) throw new Error(response.error || 'Invalid quest-list response');
  console.log(`Native messaging verified: ${response.data.length} quests readable. No quest content printed.`);
}
