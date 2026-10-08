import { EXTENSION_VERSION } from './version';

interface ExtensionUpdateApi {
  storage: { local: {
    get(key: string): Promise<Record<string, unknown>>;
    set(items: Record<string, unknown>): Promise<void>;
  } };
  runtime: { reload(): void };
}

// Persist before reloading: a failed update must not trap the user in a loop.
export async function reloadOutdatedBackground(api: ExtensionUpdateApi): Promise<boolean> {
  const key = 'backgroundReloadVersion';
  const previous = await api.storage.local.get(key);
  if (previous[key] === EXTENSION_VERSION) return false;
  await api.storage.local.set({ [key]: EXTENSION_VERSION });
  api.runtime.reload();
  return true;
}
