use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::{SecondsFormat, Utc};
use directories::ProjectDirs;
use quest_core::{
    AdapterType, BrowserTab, CreateQuestInput, Quest, QuestDesignation, QuestFile, QuestState,
    Resource, SavePoint, UpdateQuestInput,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

const MIGRATION: &str = include_str!("../../../migrations/0001_initial.sql");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("database location is unavailable")]
    DatabaseLocationUnavailable,
}

pub type Result<T, E = StorageError> = std::result::Result<T, E>;

pub fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("QUEST_JOURNAL_DB") {
        if path.is_empty() {
            return Err(StorageError::InvalidInput(
                "QUEST_JOURNAL_DB cannot be empty".into(),
            ));
        }
        return Ok(PathBuf::from(path));
    }
    let dirs = ProjectDirs::from("com", "Quest Journal", "Quest Journal")
        .ok_or(StorageError::DatabaseLocationUnavailable)?;
    Ok(dirs.data_dir().join("quests.sqlite3"))
}

pub struct Storage {
    connection: Mutex<Connection>,
}

impl Storage {
    pub fn open_default() -> Result<Self> {
        Self::open(database_path()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let is_memory = path == Path::new(":memory:");
        if !is_memory {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        if !is_memory {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "NORMAL")?;
        }
        connection.execute_batch(MIGRATION)?;
        // Serialize schema inspection and migration across desktop and native-host processes.
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let has_parent: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('quests') WHERE name='parent_id')", [], |row| row.get(0))?;
        if !has_parent {
            tx.execute_batch("ALTER TABLE quests ADD COLUMN parent_id TEXT REFERENCES quests(id); CREATE INDEX idx_quests_parent ON quests(parent_id);")?;
        }
        let has_position: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('quests') WHERE name='position')", [], |row| row.get(0))?;
        if !has_position {
            tx.execute_batch("ALTER TABLE quests ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
                WITH ranked AS (SELECT id, ROW_NUMBER() OVER (ORDER BY last_active_at IS NULL,last_active_at DESC,updated_at DESC,title COLLATE NOCASE,id) AS rank FROM quests)
                UPDATE quests SET position=(SELECT rank FROM ranked WHERE ranked.id=quests.id);")?;
        }
        let has_materials: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='materials')", [], |row| row.get(0))?;
        if !has_materials {
            tx.execute_batch("CREATE TABLE materials (id TEXT PRIMARY KEY NOT NULL, quest_id TEXT NOT NULL REFERENCES quests(id), resource_uri TEXT NOT NULL, state_json TEXT NOT NULL, position INTEGER NOT NULL, UNIQUE(quest_id,resource_uri)); CREATE INDEX idx_materials_quest ON materials(quest_id,position);")?;
            let ids = { let mut statement = tx.prepare("SELECT id FROM quests")?; let rows = statement.query_map([], |row| row.get::<_, String>(0))?.collect::<std::result::Result<Vec<_>, _>>()?; rows };
            for id in ids {
                for (position, (uri, state)) in browser_context_conn(&tx, &id)?.into_iter().enumerate() {
                    tx.execute("INSERT INTO materials VALUES (?1,?2,?3,?4,?5)", params![new_id(), id, uri, serde_json::to_string(&state)?, position as i64])?;
                }
            }
        }
        let has_trash: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('materials') WHERE name='trashed')", [], |row| row.get(0))?;
        if !has_trash { tx.execute_batch("ALTER TABLE materials ADD COLUMN trashed INTEGER NOT NULL DEFAULT 0 CHECK(trashed IN (0,1));")?; }
        let has_quest_trash: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('quests') WHERE name='trash_root')", [], |row| row.get(0))?;
        if !has_quest_trash {
            tx.execute_batch("ALTER TABLE quests ADD COLUMN trash_root TEXT REFERENCES quests(id); CREATE INDEX idx_quests_trash ON quests(trash_root);")?;
        }
        let has_icon: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('quests') WHERE name='icon')", [], |row| row.get(0))?;
        if !has_icon { tx.execute_batch("ALTER TABLE quests ADD COLUMN icon TEXT NOT NULL DEFAULT '';")?; }
        tx.commit()?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn create_quest(&self, input: CreateQuestInput) -> Result<Quest> {
        let title = required("title", &input.title)?;
        let icon = validate_icon(&input.icon)?;
        let id = new_id();
        let now = now();
        let last_active_at = (input.state == QuestState::Active).then(|| now.clone());
        let connection = self.lock()?;
        if let Some(parent) = &input.parent_id { ensure_quest(&connection, parent)?; }
        connection.execute(
            "INSERT INTO quests (id,title,territory,objective,designation,state,current_checkpoint,created_at,updated_at,last_active_at,parent_id,position,icon)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?9,?10,(SELECT COALESCE(MAX(position),-1)+1 FROM quests WHERE parent_id IS ?10),?11)",
            params![id, title, input.territory.trim(), input.objective.trim(), input.designation.as_str(),
                input.state.as_str(), input.current_checkpoint.trim(), now, last_active_at, input.parent_id, icon],
        )?;
        get_quest_conn(&connection, &id)
    }

    pub fn move_quest(&self, id: &str, parent_id: Option<&str>) -> Result<Quest> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, id)?;
        if let Some(parent) = parent_id {
            ensure_quest(&tx, parent)?;
            let mut cursor = Some(parent.to_owned());
            let mut visited = std::collections::HashSet::new();
            while let Some(ancestor) = cursor {
                if ancestor == id || !visited.insert(ancestor.clone()) {
                    return Err(StorageError::InvalidInput("A quest cannot be moved inside itself or its descendants".into()));
                }
                cursor = tx.query_row("SELECT parent_id FROM quests WHERE id=?1", [&ancestor], |row| row.get(0))?;
            }
        }
        tx.execute("UPDATE quests SET parent_id=?2,updated_at=?3,position=CASE WHEN parent_id IS ?2 THEN position ELSE (SELECT COALESCE(MAX(position),-1)+1 FROM quests WHERE parent_id IS ?2) END WHERE id=?1", params![id, parent_id, now()])?;
        tx.commit()?;
        get_quest_conn(&connection, id)
    }

    pub fn reorder_quests(&self, parent_id: Option<&str>, ids: &[String]) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing = {
            let mut statement = tx.prepare("SELECT id FROM quests WHERE parent_id IS ?1 AND trash_root IS NULL")?;
            let rows = statement.query_map([parent_id], |row| row.get::<_, String>(0))?.collect::<std::result::Result<std::collections::HashSet<_>, _>>()?;
            rows
        };
        let requested: std::collections::HashSet<_> = ids.iter().cloned().collect();
        if requested != existing || requested.len() != ids.len() {
            return Err(StorageError::InvalidInput("The quest list changed. Refresh and try reordering again.".into()));
        }
        for (position, id) in ids.iter().enumerate() {
            tx.execute("UPDATE quests SET position=?2 WHERE id=?1", params![id,position as i64])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_quests(&self) -> Result<Vec<Quest>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id FROM quests WHERE trash_root IS NULL ORDER BY position,id",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| get_quest_conn(&connection, id))
            .collect()
    }

    pub fn trash_quest(&self, id: &str) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, id)?;
        // Each deletion is one restorable group. Previously deleted branches stay separate.
        tx.execute("WITH RECURSIVE subtree(id) AS (
            SELECT id FROM quests WHERE id=?1 AND trash_root IS NULL
            UNION SELECT q.id FROM quests q JOIN subtree s ON q.parent_id=s.id WHERE q.trash_root IS NULL
        ) UPDATE quests SET trash_root=?1,updated_at=?2 WHERE id IN (SELECT id FROM subtree)", params![id, now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_trashed_quests(&self) -> Result<Vec<Quest>> {
        let connection = self.lock()?;
        let ids = {
            let mut statement = connection.prepare("SELECT id FROM quests WHERE trash_root=id ORDER BY updated_at DESC,id")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?.collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        ids.iter().map(|id| get_quest_conn(&connection, id)).collect()
    }

    pub fn restore_quest(&self, id: &str) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let parent: Option<String> = tx.query_row("SELECT parent_id FROM quests WHERE id=?1 AND trash_root=id", [id], |row| row.get(0))
            .optional()?.ok_or_else(|| not_found("deleted quest", id))?;
        let destination = if let Some(parent) = parent {
            let available: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM quests WHERE id=?1 AND trash_root IS NULL)", [&parent], |row| row.get(0))?;
            available.then_some(parent)
        } else { None };
        tx.execute("UPDATE quests SET parent_id=?2,position=(SELECT COALESCE(MAX(position),-1)+1 FROM quests WHERE parent_id IS ?2 AND trash_root IS NULL) WHERE id=?1", params![id,destination])?;
        tx.execute("UPDATE quests SET trash_root=NULL,updated_at=?2 WHERE trash_root=?1", params![id,now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_quest(&self, id: &str) -> Result<Quest> {
        let connection = self.lock()?;
        ensure_quest(&connection, id)?;
        get_quest_conn(&connection, id)
    }

    pub fn update_quest(&self, id: &str, input: UpdateQuestInput) -> Result<Quest> {
        let connection = self.lock()?;
        ensure_quest(&connection, id)?;
        let existing = quest_record(&connection, id)?;
        let icon = input.icon.as_deref().map(validate_icon).transpose()?;
        let title = match input.title {
            Some(value) => required("title", &value)?.to_owned(),
            None => existing.title,
        };
        let territory = input
            .territory
            .map(|v| v.trim().to_owned())
            .unwrap_or(existing.territory);
        let objective = input
            .objective
            .map(|v| v.trim().to_owned())
            .unwrap_or(existing.objective);
        let designation = input.designation.unwrap_or(existing.designation);
        let state = input.state.unwrap_or(existing.state);
        let checkpoint = input
            .current_checkpoint
            .map(|v| v.trim().to_owned())
            .unwrap_or(existing.current_checkpoint);
        let timestamp = now();
        let last_active_at = if state == QuestState::Active && existing.state != QuestState::Active
        {
            Some(timestamp.clone())
        } else {
            existing.last_active_at
        };
        connection.execute(
            "UPDATE quests SET title=?2,territory=?3,objective=?4,designation=?5,state=?6,current_checkpoint=?7,updated_at=?8,last_active_at=?9,icon=COALESCE(?10,icon) WHERE id=?1",
            params![id, title, territory, objective, designation.as_str(), state.as_str(), checkpoint, timestamp, last_active_at, icon],
        )?;
        get_quest_conn(&connection, id)
    }

    pub fn add_quest_files(&self, quest_id: &str, paths: &[String]) -> Result<Vec<QuestFile>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.lock()?;
        let tx = connection.transaction()?;
        ensure_quest(&tx, quest_id)?;
        let mut added = Vec::with_capacity(paths.len());
        for raw_path in paths {
            let path = required("file path", raw_path)?;
            let label = Path::new(path)
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or(path);
            let id = new_id();
            let added_at = now();
            let changed = tx.execute(
                "INSERT OR IGNORE INTO quest_files (id,quest_id,path,label,added_at) VALUES (?1,?2,?3,?4,?5)",
                params![id, quest_id, path, label, added_at],
            )?;
            if changed == 1 {
                added.push(QuestFile {
                    id,
                    quest_id: quest_id.to_owned(),
                    path: path.to_owned(),
                    label: label.to_owned(),
                    added_at,
                });
            }
        }
        tx.execute(
            "UPDATE quests SET updated_at=?2 WHERE id=?1",
            params![quest_id, now()],
        )?;
        tx.commit()?;
        Ok(added)
    }

    pub fn remove_quest_file(&self, id: &str) -> Result<()> {
        let connection = self.lock()?;
        let quest_id = connection
            .query_row(
                "SELECT quest_id FROM quest_files WHERE id=?1",
                [id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| not_found("quest file", id))?;
        connection.execute("DELETE FROM quest_files WHERE id=?1", [id])?;
        connection.execute(
            "UPDATE quests SET updated_at=?2 WHERE id=?1",
            params![quest_id, now()],
        )?;
        Ok(())
    }

    pub fn add_tabs(&self, quest_id: &str, tabs: &[BrowserTab]) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, quest_id)?;
        for tab in tabs {
            tx.execute("INSERT INTO materials (id,quest_id,resource_uri,state_json,position) VALUES (?1,?2,?3,?4,(SELECT COALESCE(MAX(position),-1)+1 FROM materials WHERE quest_id=?2)) ON CONFLICT(quest_id,resource_uri) DO UPDATE SET state_json=excluded.state_json,trashed=0", params![new_id(),quest_id,tab.url,serde_json::to_string(tab)?])?;
        }
        tx.execute("UPDATE quests SET updated_at=?2 WHERE id=?1",params![quest_id,now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn create_save_point(
        &self,
        quest_id: &str,
        checkpoint: &str,
        tabs: &[BrowserTab],
    ) -> Result<SavePoint> {
        self.save_point(quest_id, checkpoint, Some(tabs))
    }

    pub fn create_local_save_point(&self, quest_id: &str, checkpoint: &str) -> Result<SavePoint> {
        self.save_point(quest_id, checkpoint, None)
    }

    fn save_point(&self, quest_id: &str, checkpoint: &str, tabs: Option<&[BrowserTab]>) -> Result<SavePoint> {
        let checkpoint = checkpoint.trim();
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, quest_id)?;
        let save_id = new_id();
        let created_at = now();
        tx.execute(
            "INSERT INTO save_points (id,quest_id,checkpoint,created_at) VALUES (?1,?2,?3,?4)",
            params![save_id, quest_id, checkpoint, created_at],
        )?;

        let mut ordered_tabs = tabs.unwrap_or(&[]).to_vec();
        ordered_tabs.sort_by_key(|tab| tab.index);
        let mut restore_order = 0u32;
        for tab in ordered_tabs {
            let state = serde_json::to_value(&tab)?;
            tx.execute("INSERT INTO materials (id,quest_id,resource_uri,state_json,position) VALUES (?1,?2,?3,?4,(SELECT COALESCE(MAX(position),-1)+1 FROM materials WHERE quest_id=?2)) ON CONFLICT(quest_id,resource_uri) DO UPDATE SET state_json=excluded.state_json,trashed=0", params![new_id(), quest_id, tab.url, serde_json::to_string(&state)?])?;
            insert_resource(&tx, &save_id, AdapterType::BrowserTab, &tab.url, state, restore_order)?;
            restore_order += 1;
        }

        if tabs.is_none() {
            for material in materials_conn(&tx, quest_id)? {
                insert_resource(&tx, &save_id, AdapterType::BrowserTab, &material.resource_uri, material.state_json, restore_order)?;
                restore_order += 1;
            }
        }

        let files = quest_files_conn(&tx, quest_id)?;
        for file in files {
            insert_resource(
                &tx,
                &save_id,
                AdapterType::File,
                &file.path,
                json!({
                    "quest_file_id": file.id, "label": file.label
                }),
                restore_order,
            )?;
            restore_order += 1;
        }

        let updated_at = now();
        tx.execute(
            "UPDATE quests SET current_checkpoint=CASE WHEN ?2='' THEN current_checkpoint ELSE ?2 END,state='paused',updated_at=?3 WHERE id=?1",
            params![quest_id, checkpoint, updated_at],
        )?;
        let save = save_point_conn(&tx, &save_id)?;
        tx.commit()?;
        Ok(save)
    }

    pub fn move_materials(&self, source: &str, destination: &str, ids: &[String]) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, source)?;
        ensure_quest(&tx, destination)?;
        if source == destination || ids.is_empty() { return Err(StorageError::InvalidInput("Choose tabs and a different destination".into())); }
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        if unique.len() != ids.len() { return Err(StorageError::InvalidInput("Duplicate selection".into())); }
        for id in ids {
            let uri: String = tx.query_row("SELECT resource_uri FROM materials WHERE id=?1 AND quest_id=?2 AND trashed=0", params![id, source], |row| row.get(0)).optional()?.ok_or_else(|| StorageError::InvalidInput("A selected tab has moved. Refresh and try again.".into()))?;
            let duplicate: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM materials WHERE quest_id=?1 AND resource_uri=?2)", params![destination, uri], |row| row.get(0))?;
            if duplicate { return Err(StorageError::InvalidInput(format!("The destination already contains {uri}. No tabs were moved."))); }
            tx.execute("UPDATE materials SET quest_id=?2,position=(SELECT COALESCE(MAX(position),-1)+1 FROM materials WHERE quest_id=?2) WHERE id=?1", params![id,destination])?;
        }
        tx.execute("UPDATE quests SET updated_at=?3 WHERE id IN (?1,?2)", params![source,destination,now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_materials_trashed(&self, quest_id: &str, ids: &[String], trashed: bool) -> Result<()> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure_quest(&tx, quest_id)?;
        if ids.is_empty() { return Err(StorageError::InvalidInput("Select at least one tab".into())); }
        for id in ids {
            let changed = tx.execute("UPDATE materials SET trashed=?3 WHERE id=?1 AND quest_id=?2", params![id,quest_id,trashed])?;
            if changed != 1 { return Err(StorageError::InvalidInput("A selected tab no longer belongs to this quest. Refresh and try again.".into())); }
        }
        tx.execute("UPDATE quests SET updated_at=?2 WHERE id=?1", params![quest_id,now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn selected_materials(&self, quest_id: &str, ids: &[String]) -> Result<Vec<quest_core::Material>> {
        let connection = self.lock()?;
        ensure_quest(&connection, quest_id)?;
        let all = materials_conn(&connection, quest_id)?;
        let mut selected = Vec::new();
        for id in ids {
            selected.push(all.iter().find(|material| &material.id == id).cloned().ok_or_else(|| StorageError::InvalidInput("A selected tab no longer belongs to this quest".into()))?);
        }
        Ok(selected)
    }

    pub fn get_save_point(&self, id: &str) -> Result<SavePoint> {
        let connection = self.lock()?;
        save_point_conn(&connection, id)
    }

    pub fn touch_last_active(&self, quest_id: &str) -> Result<()> {
        let connection = self.lock()?;
        let timestamp = now();
        let changed = connection.execute(
            "UPDATE quests SET state='active',last_active_at=?2,updated_at=?2 WHERE id=?1",
            params![quest_id, timestamp],
        )?;
        if changed == 0 {
            return Err(not_found("quest", quest_id));
        }
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| StorageError::InvalidInput("database lock is poisoned".into()))
    }
}

#[derive(Debug)]
struct QuestRecord {
    id: String,
    title: String,
    territory: String,
    objective: String,
    designation: QuestDesignation,
    state: QuestState,
    current_checkpoint: String,
    created_at: String,
    updated_at: String,
    last_active_at: Option<String>,
}

fn get_quest_conn(connection: &Connection, id: &str) -> Result<Quest> {
    let record = quest_record(connection, id)?;
    let files = quest_files_conn(connection, id)?;
    let latest_save_id = connection.query_row(
        "SELECT id FROM save_points WHERE quest_id=?1 ORDER BY created_at DESC,rowid DESC LIMIT 1",
        [id], |row| row.get::<_, String>(0),
    ).optional()?;
    let latest_save = latest_save_id
        .map(|save_id| save_point_conn(connection, &save_id))
        .transpose()?;
    Ok(Quest {
        icon: connection.query_row("SELECT icon FROM quests WHERE id=?1", [id], |row| row.get(0))?,
        materials: materials_conn(connection, id)?,
        trashed_materials: materials_with_trash_conn(connection, id, true)?,
        parent_id: connection.query_row("SELECT parent_id FROM quests WHERE id=?1", [id], |row| row.get(0))?,
        id: record.id,
        title: record.title,
        territory: record.territory,
        objective: record.objective,
        designation: record.designation,
        state: record.state,
        current_checkpoint: record.current_checkpoint,
        created_at: record.created_at,
        updated_at: record.updated_at,
        last_active_at: record.last_active_at,
        files,
        latest_save,
    })
}

fn quest_record(connection: &Connection, id: &str) -> Result<QuestRecord> {
    connection.query_row(
        "SELECT id,title,territory,objective,designation,state,current_checkpoint,created_at,updated_at,last_active_at FROM quests WHERE id=?1",
        [id], |row| {
            let designation: String = row.get(4)?;
            let state: String = row.get(5)?;
            Ok(QuestRecord {
                id: row.get(0)?, title: row.get(1)?, territory: row.get(2)?, objective: row.get(3)?,
                designation: parse_designation(&designation)?, state: parse_state(&state)?, current_checkpoint: row.get(6)?,
                created_at: row.get(7)?, updated_at: row.get(8)?, last_active_at: row.get(9)?,
            })
        },
    ).optional()?.ok_or_else(|| not_found("quest", id))
}

fn quest_files_conn(connection: &Connection, quest_id: &str) -> Result<Vec<QuestFile>> {
    let mut statement = connection.prepare(
        "SELECT id,quest_id,path,label,added_at FROM quest_files WHERE quest_id=?1 ORDER BY added_at,id",
    )?;
    let files = statement
        .query_map([quest_id], |row| {
            Ok(QuestFile {
                id: row.get(0)?,
                quest_id: row.get(1)?,
                path: row.get(2)?,
                label: row.get(3)?,
                added_at: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(files)
}

fn materials_conn(connection: &Connection, quest_id: &str) -> Result<Vec<quest_core::Material>> {
    materials_with_trash_conn(connection, quest_id, false)
}

fn materials_with_trash_conn(connection: &Connection, quest_id: &str, trashed: bool) -> Result<Vec<quest_core::Material>> {
    let mut statement = connection.prepare("SELECT id,quest_id,resource_uri,state_json,position FROM materials WHERE quest_id=?1 AND trashed=?2 ORDER BY position,id")?;
    let rows = statement.query_map(params![quest_id,trashed], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,i64>(4)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
    rows.into_iter().map(|(id,quest_id,resource_uri,state,position)| Ok(quest_core::Material {id,quest_id,resource_uri,state_json:serde_json::from_str(&state)?,position})).collect()
}

fn browser_context_conn(connection: &Connection, quest_id: &str) -> Result<Vec<(String, Value)>> {
    let mut statement = connection.prepare(
        "SELECT r.resource_uri,r.state_json
         FROM resources r
         JOIN save_points s ON s.id=r.save_point_id
         WHERE s.quest_id=?1 AND r.adapter_type='browser_tab'
         ORDER BY s.created_at,r.restore_order",
    )?;
    let raw = statement
        .query_map([quest_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut merged: Vec<(String, Value)> = Vec::new();
    let mut positions: HashMap<String, usize> = HashMap::new();
    for (uri, state) in raw {
        let state = serde_json::from_str(&state)?;
        if let Some(index) = positions.get(&uri).copied() {
            merged[index].1 = state;
        } else {
            positions.insert(uri.clone(), merged.len());
            merged.push((uri, state));
        }
    }
    Ok(merged)
}

fn save_point_conn(connection: &Connection, id: &str) -> Result<SavePoint> {
    let (quest_id, checkpoint, created_at) = connection
        .query_row(
            "SELECT quest_id,checkpoint,created_at FROM save_points WHERE id=?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| not_found("save point", id))?;
    let mut statement = connection.prepare(
        "SELECT id,adapter_type,resource_uri,state_json,restore_order FROM resources WHERE save_point_id=?1 ORDER BY restore_order",
    )?;
    let raw = statement
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u32>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let resources = raw
        .into_iter()
        .map(|(resource_id, adapter, uri, state_json, restore_order)| {
            Ok(Resource {
                id: resource_id,
                save_point_id: id.to_owned(),
                adapter_type: parse_adapter(&adapter)?,
                resource_uri: uri,
                state_json: serde_json::from_str(&state_json)?,
                restore_order,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SavePoint {
        id: id.to_owned(),
        quest_id,
        checkpoint,
        created_at,
        resources,
    })
}

fn insert_resource(
    tx: &Transaction<'_>,
    save_id: &str,
    adapter: AdapterType,
    uri: &str,
    state: serde_json::Value,
    order: u32,
) -> Result<()> {
    tx.execute(
        "INSERT INTO resources (id,save_point_id,adapter_type,resource_uri,state_json,restore_order) VALUES (?1,?2,?3,?4,?5,?6)",
        params![new_id(), save_id, adapter.as_str(), uri, serde_json::to_string(&state)?, order],
    )?;
    Ok(())
}

fn ensure_quest(connection: &Connection, id: &str) -> Result<()> {
    if connection
        .query_row("SELECT 1 FROM quests WHERE id=?1 AND trash_root IS NULL", [id], |_| Ok(()))
        .optional()?
        .is_none()
    {
        return Err(not_found("quest", id));
    }
    Ok(())
}

fn required<'a>(name: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        Err(StorageError::InvalidInput(format!(
            "{name} cannot be empty"
        )))
    } else {
        Ok(value)
    }
}

fn parse_designation(value: &str) -> rusqlite::Result<QuestDesignation> {
    match value {
        "main" => Ok(QuestDesignation::Main),
        "side" => Ok(QuestDesignation::Side),
        _ => Err(invalid_column(value)),
    }
}
fn parse_state(value: &str) -> rusqlite::Result<QuestState> {
    match value {
        "active" => Ok(QuestState::Active),
        "paused" => Ok(QuestState::Paused),
        "dormant" => Ok(QuestState::Dormant),
        "waiting" => Ok(QuestState::Waiting),
        "completed" => Ok(QuestState::Completed),
        "abandoned" => Ok(QuestState::Abandoned),
        _ => Err(invalid_column(value)),
    }
}
fn parse_adapter(value: &str) -> Result<AdapterType> {
    match value {
        "browser_tab" => Ok(AdapterType::BrowserTab),
        "file" => Ok(AdapterType::File),
        _ => Err(StorageError::InvalidInput(format!(
            "unknown adapter type in database: {value}"
        ))),
    }
}
fn invalid_column(value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid enum value: {value}"),
        )),
    )
}
fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn not_found(entity: &'static str, id: &str) -> StorageError {
    StorageError::NotFound {
        entity,
        id: id.to_owned(),
    }
}

fn validate_icon(icon: &str) -> Result<&str> {
    let icon = icon.trim();
    if icon.len() > 80 || !icon.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b':' || byte == b'-') {
        return Err(StorageError::InvalidInput("Invalid quest icon identifier".into()));
    }
    Ok(icon)
}
