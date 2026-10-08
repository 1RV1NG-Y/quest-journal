export const NATIVE_HOST_NAME = 'com.quest_journal.native_host';

export type QuestDesignation = 'main' | 'side';
export type QuestState =
  | 'active'
  | 'paused'
  | 'dormant'
  | 'waiting'
  | 'completed'
  | 'abandoned';

export interface QuestFile {
  id: string;
  quest_id: string;
  path: string;
  label: string | null;
  added_at: string;
}

export interface Resource {
  id: string;
  save_point_id: string;
  adapter_type: 'browser_tab' | 'file';
  resource_uri: string;
  state_json: unknown;
  restore_order: number;
}

export interface SavePoint {
  id: string;
  quest_id: string;
  checkpoint: string;
  created_at: string;
  resources: Resource[];
}

export interface Quest {
  parent_id: string | null;
  id: string;
  title: string;
  territory: string;
  objective: string;
  designation: QuestDesignation;
  state: QuestState;
  current_checkpoint: string;
  created_at: string;
  updated_at: string;
  last_active_at: string;
  files: QuestFile[];
  latest_save: SavePoint | null;
}

export interface CapturedTab {
  window_id?: number;
  id: number;
  url: string;
  title: string;
  pinned: boolean;
  active: boolean;
  index: number;
  group_id: number | null;
}

export interface ListQuestsRequest {
  type: 'list_quests';
}

export interface CreateQuestRequest {
  type: 'create_quest';
  title: string;
  parent_id?: string | null;
}

export interface PauseQuestRequest {
  type: 'pause_quest';
  quest_id: string;
  checkpoint: string;
  tabs: CapturedTab[];
  close_after_save: boolean;
}
export type NativeRequest = ListQuestsRequest | CreateQuestRequest | PauseQuestRequest | { type: 'add_tabs'; quest_id: string; tabs: CapturedTab[] };
export type NativeResponse<T = unknown> =
  | { ok: true; data: T }
  | { ok: false; error: string };

export type CaptureScope = 'current_window' | 'all_windows';

export type PopupRequest =
  | { type: 'extension_list_quests' }
  | { type: 'extension_create_quest'; title: string; parent_id?: string | null }
  | { type: 'extension_preview'; scope: CaptureScope; window_id: number }
  | {
      type: 'extension_pause_quest';
      action: 'add' | 'save' | 'close';
      expected_urls: Record<string, string>;
      quest_id: string;
      checkpoint: string;
      scope: CaptureScope;
      window_id: number;
      selected_tab_ids: number[];
      close_after_save: boolean;
    };

export interface PreviewData {
  protocol_version: 4;
  windows: { id: number; label: string }[];
  suggested_ids: number[];
  tabs: CapturedTab[];
  filtered_count: number;
  context_label: string;
}

export interface PauseResultData {
  save_point: unknown;
  captured_count: number;
  closed_count: number;
  close_errors: string[];
}

export type PopupResponse<T = unknown> =
  | { ok: true; data: T }
  | {
      ok: false;
      error: string;
      code?: 'host_unavailable' | 'no_supported_tabs' | 'invalid_request';
    };
