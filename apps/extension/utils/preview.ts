import type { PreviewData } from './protocol';

export const RELOAD_EXTENSION_MESSAGE = 'The popup and background are running different extension versions. Reload the extension to finish the update. Your saved quests and tabs are safe.';

export function validatePreview(value: unknown): PreviewData {
  if (!value || typeof value !== 'object') throw new Error('The browser returned an invalid tab preview. Refresh and try again.');
  const data = value as Partial<PreviewData>;
  if (data.protocol_version !== 4) throw new Error(RELOAD_EXTENSION_MESSAGE);
  if (!Array.isArray(data.tabs) || !Array.isArray(data.windows) || !Array.isArray(data.suggested_ids) ||
      !data.tabs.every(tab => tab && typeof tab.id === 'number' && typeof tab.window_id === 'number' && typeof tab.url === 'string' && typeof tab.title === 'string') ||
      !data.windows.every(window => window && typeof window.id === 'number' && typeof window.label === 'string') ||
      !data.suggested_ids.every(id => typeof id === 'number')) {
    throw new Error('The browser returned an incomplete tab preview. Refresh and try again.');
  }
  return data as PreviewData;
}
