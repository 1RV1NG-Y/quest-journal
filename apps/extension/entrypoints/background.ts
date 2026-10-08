import { browser } from 'wxt/browser';
import {
  NATIVE_HOST_NAME,
  type CapturedTab,
  type CaptureScope,
  type NativeRequest,
  type NativeResponse,
  type PauseResultData,
  type PopupRequest,
  type PopupResponse,
  type PreviewData,
} from '../utils/protocol';


function isPopupRequest(value: unknown): value is PopupRequest {
  if (typeof value !== 'object' || value === null) return false;
  const request = value as Record<string, unknown>;
  if (typeof request.type !== 'string') return false;

  if (request.type === 'extension_list_quests') return true;
  if (request.type === 'extension_create_quest') {
    return typeof request.title === 'string' && request.title.trim().length >= 2 && (request.parent_id == null || typeof request.parent_id === 'string');
  }
  if ((request.type === 'extension_preview' || request.type === 'extension_pause_quest') &&
      (typeof request.window_id !== 'number' || !Number.isInteger(request.window_id) || request.window_id < 0)) return false;
  if (request.type === 'extension_preview') {
    return (
      request.scope === 'all_windows' ||
      request.scope === 'current_window'
    );
  }
  if (request.type === 'extension_pause_quest') {
    return (
      typeof request.quest_id === 'string' &&
      request.quest_id.trim().length > 0 &&
      typeof request.checkpoint === 'string' &&
      (request.scope === 'all_windows' ||
        request.scope === 'current_window') &&
      Array.isArray(request.selected_tab_ids) &&
      request.selected_tab_ids.every((id) => Number.isInteger(id) && id >= 0) &&
      typeof request.close_after_save === 'boolean' &&
      ['add','save','close'].includes(String(request.action)) &&
      request.expected_urls !== null && typeof request.expected_urls === 'object'
    );
  }
  return false;
}

function isUnsupportedUrl(url: string): boolean {
  try {
    const protocol = new URL(url.trim()).protocol;
    return protocol !== 'http:' && protocol !== 'https:';
  } catch {
    return true;
  }
}

async function captureTabs(
  scope: CaptureScope,
  windowId: number,
  popupTabId?: number,
): Promise<PreviewData> {
  const available = await browser.windows.getAll({ populate: true, windowTypes: ['normal'] });
  if (!available.some(window => window.id === windowId)) {
    throw new Error('The window that opened this popup is no longer available. Reopen the extension in the window you want to capture.');
  }
  const windows = available.filter(window => scope === 'all_windows' || window.id === windowId);
  windows.sort((a,b) => Number(b.id === windowId) - Number(a.id === windowId) || (a.id ?? 0) - (b.id ?? 0));
  const queried = windows.flatMap(window => window.tabs ?? []);
  const currentTabs = queried.filter(tab => tab.windowId === windowId);
  const active = currentTabs.find(tab => tab.active);
  const activeGroup = (active as typeof active & { groupId?: number })?.groupId;
  const highlighted = currentTabs.filter(tab => tab.highlighted);
  const suggested = activeGroup != null && activeGroup >= 0 ? currentTabs.filter(tab => (tab as typeof tab & {groupId?: number}).groupId === activeGroup) : highlighted.length > 1 ? highlighted : active ? [active] : [];
  const contextLabel = scope === 'all_windows' ? 'All browser windows' : 'This window';

  const tabs: CapturedTab[] = [];
  let filteredCount = 0;

  for (const tab of queried) {
    const extendedTab = tab as typeof tab & { pendingUrl?: string; groupId?: number };
    const url = tab.url ?? extendedTab.pendingUrl ?? '';
    if (
      tab.id === undefined ||
      tab.id < 0 ||
      tab.id === popupTabId ||
      isUnsupportedUrl(url)
    ) {
      filteredCount += 1;
      continue;
    }

    tabs.push({
      id: tab.id,
      window_id: tab.windowId,
      url,
      title: tab.title?.trim() || url,
      pinned: Boolean(tab.pinned),
      active: Boolean(tab.active),
      index: tab.index,
      group_id:
        extendedTab.groupId !== undefined && extendedTab.groupId >= 0
          ? extendedTab.groupId
          : null,
    });
  }

  return { protocol_version: 4, tabs, filtered_count: filteredCount, context_label: contextLabel, suggested_ids: suggested.flatMap(tab => tab.id == null ? [] : [tab.id]), windows: windows.map((window,index) => ({ id: window.id!, label: `${window.id === windowId ? 'This window' : `Window ${index + 1}`} · ${(window.tabs ?? []).find(tab => tab.active)?.title ?? 'Browser'}` })) };
}

function normalizeNativeResponse(value: unknown): NativeResponse {
  if (typeof value !== 'object' || value === null) {
    return { ok: false, error: 'The native host returned an invalid response.' };
  }
  const response = value as { ok?: unknown; data?: unknown; error?: unknown };
  if (response.ok === true && 'data' in response) {
    return { ok: true, data: response.data };
  }
  if (response.ok === false && typeof response.error === 'string') {
    return { ok: false, error: response.error };
  }
  return { ok: false, error: 'The native host returned an invalid response.' };
}

async function sendNative(request: NativeRequest): Promise<PopupResponse> {
  try {
    const raw = await browser.runtime.sendNativeMessage(NATIVE_HOST_NAME, request);
    return normalizeNativeResponse(raw);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    const unavailable =
      /native messaging|native host|native application|specified native|host.*(not found|exited)|disconnected port/i.test(
        message,
      );
    return {
      ok: false,
      error: unavailable
        ? 'Quest Journal could not connect to its native host.'
        : message || 'The native host request failed.',
      code: unavailable ? 'host_unavailable' : undefined,
    };
  }
}

async function closeCapturedTabs(
  tabs: CapturedTab[],
  popupTabId?: number,
): Promise<{ closed_count: number; close_errors: string[] }> {
  let closedCount = 0;
  const closeErrors: string[] = [];

  for (const captured of tabs) {
    if (captured.id === popupTabId) continue;

    try {
      const current = await browser.tabs.get(captured.id);
      const pendingUrl = (current as typeof current & { pendingUrl?: string }).pendingUrl;
      const currentUrl = current.url ?? pendingUrl ?? '';
      if (isUnsupportedUrl(currentUrl)) {
        closeErrors.push(`“${captured.title}” stayed open because it is now a protected browser page.`);
        continue;
      }
      if (current.windowId !== captured.window_id) {
        closeErrors.push(`“${captured.title}” stayed open because it moved to another window after saving.`);
        continue;
      }
      if (currentUrl !== captured.url) {
        closeErrors.push(`“${captured.title}” stayed open because it navigated after the checkpoint was saved.`);
        continue;
      }
      await browser.tabs.remove(captured.id);
      closedCount += 1;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      closeErrors.push(`Could not close “${captured.title}”: ${message}`);
    }
  }

  return { closed_count: closedCount, close_errors: closeErrors };
}

async function handlePopupRequest(
  request: PopupRequest,
  popupTabId?: number,
): Promise<PopupResponse> {
  if (request.type === 'extension_list_quests') {
    return sendNative({ type: 'list_quests' });
  }

  if (request.type === 'extension_create_quest') {
    return sendNative({ type: 'create_quest', title: request.title.trim(), parent_id: request.parent_id ?? null });
  }

  if (request.type === 'extension_preview') {
    try {
      return { ok: true, data: await captureTabs(request.scope, request.window_id, popupTabId) };
    } catch (error) {
      return {
        ok: false,
        error: error instanceof Error ? error.message : 'Could not inspect browser tabs.',
      };
    }
  }

  const checkpoint = request.checkpoint.trim();

  let preview: PreviewData;
  try {
    preview = await captureTabs(request.scope, request.window_id, popupTabId);
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : 'Could not inspect browser tabs.',
    };
  }

  const selectedIds = new Set(request.selected_tab_ids);
  const selectedTabs = preview.tabs.filter((tab) => selectedIds.has(tab.id));
  if (selectedTabs.length !== selectedIds.size || selectedTabs.some(tab => request.expected_urls[String(tab.id)] !== tab.url)) {
    return { ok: false, error: 'Some selected tabs changed, moved to another window, or closed. Refresh the preview before capturing.', code: 'invalid_request' };
  }
  if (selectedTabs.length === 0) {
    return {
      ok: false,
      error: 'None of the selected tabs are still available to checkpoint.',
      code: 'no_supported_tabs',
    };
  }

  const orderedTabs = selectedTabs.map((tab,index) => ({...tab, index}));
  const nativeResponse = await sendNative(request.action === 'add' ? {
    type: 'add_tabs', quest_id: request.quest_id, tabs: orderedTabs,
  } : {
    type: 'pause_quest', quest_id: request.quest_id, checkpoint, tabs: orderedTabs, close_after_save: request.action === 'close',
  });
  if (!nativeResponse.ok) return nativeResponse;

  const closeResult = request.action === 'close'
    ? await closeCapturedTabs(selectedTabs, popupTabId)
    : { closed_count: 0, close_errors: [] };

  const data: PauseResultData = {
    save_point: nativeResponse.data,
    captured_count: selectedTabs.length,
    ...closeResult,
  };
  return { ok: true, data };
}

export default defineBackground(() => {
  browser.runtime.onMessage.addListener((message, sender) => {
    if (!isPopupRequest(message)) {
      return Promise.resolve<PopupResponse>({
        ok: false,
        error: 'Unsupported extension request.',
        code: 'invalid_request',
      });
    }
    return handlePopupRequest(message, sender.tab?.id);
  });
});
