PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS quests (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  territory TEXT NOT NULL,
  objective TEXT NOT NULL,
  designation TEXT NOT NULL CHECK (designation IN ('main', 'side')),
  state TEXT NOT NULL CHECK (state IN ('active', 'paused', 'dormant', 'waiting', 'completed', 'abandoned')),
  current_checkpoint TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_active_at TEXT
);

CREATE TABLE IF NOT EXISTS quest_files (
  id TEXT PRIMARY KEY NOT NULL,
  quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  label TEXT NOT NULL,
  added_at TEXT NOT NULL,
  UNIQUE (quest_id, path)
);

CREATE TABLE IF NOT EXISTS save_points (
  id TEXT PRIMARY KEY NOT NULL,
  quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
  checkpoint TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resources (
  id TEXT PRIMARY KEY NOT NULL,
  save_point_id TEXT NOT NULL REFERENCES save_points(id) ON DELETE CASCADE,
  adapter_type TEXT NOT NULL CHECK (adapter_type IN ('browser_tab', 'file')),
  resource_uri TEXT NOT NULL,
  state_json TEXT NOT NULL,
  restore_order INTEGER NOT NULL CHECK (restore_order >= 0),
  UNIQUE (save_point_id, restore_order)
);

CREATE INDEX IF NOT EXISTS idx_quests_updated_at ON quests(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_quest_files_quest ON quest_files(quest_id, added_at, id);
CREATE INDEX IF NOT EXISTS idx_save_points_quest ON save_points(quest_id, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_resources_save ON resources(save_point_id, restore_order);
