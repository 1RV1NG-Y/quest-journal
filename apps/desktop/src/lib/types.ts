export type QuestDesignation = 'main' | 'side';
export type QuestState = 'active' | 'paused' | 'dormant' | 'waiting' | 'completed' | 'abandoned';
export type AdapterType = 'browser_tab' | 'file';

export interface QuestFile {
  id: string;
  quest_id: string;
  path: string;
  label: string;
  added_at: string;
}

export interface Resource {
  id: string;
  save_point_id: string;
  adapter_type: AdapterType;
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

export interface Material {
  id: string;
  quest_id: string;
  resource_uri: string;
  state_json: unknown;
  position: number;
}

export interface Quest {
  icon: string;
  materials: Material[];
  trashed_materials: Material[];
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
  last_active_at: string | null;
  files: QuestFile[];
  latest_save: SavePoint | null;
}

export interface QuestInput {
  icon?: string;
  parent_id?: string | null;
  title: string;
  territory: string;
  objective: string;
  designation: QuestDesignation;
  state: QuestState;
  current_checkpoint: string;
}

export interface RestoreOutcome {
  resource_id: string;
  adapter_type: AdapterType;
  resource_uri: string;
  ok: boolean;
  error: string | null;
}

export type ContinueResult = RestoreOutcome[];
