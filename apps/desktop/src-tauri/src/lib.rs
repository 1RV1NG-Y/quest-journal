use std::collections::HashSet;

use quest_core::{CreateQuestInput, Quest, RestoreOutcome, SavePoint, UpdateQuestInput};
use quest_storage::Storage;
use tauri::Manager;

struct AppState {
    storage: Storage,
}

type CommandResult<T, E = String> = std::result::Result<T, E>;

#[tauri::command(rename_all = "camelCase")]
fn list_quests(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Quest>> {
    state
        .storage
        .list_quests()
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn trash_quest(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.storage.trash_quest(&id).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn list_trashed_quests(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Quest>> {
    state.storage.list_trashed_quests().map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn restore_quest(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.storage.restore_quest(&id).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_quest(id: String, state: tauri::State<'_, AppState>) -> CommandResult<Quest> {
    state
        .storage
        .get_quest(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn create_quest(
    input: CreateQuestInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Quest> {
    state
        .storage
        .create_quest(input)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn update_quest(
    id: String,
    input: UpdateQuestInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Quest> {
    state
        .storage
        .update_quest(&id, input)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn reorder_quests(parent_id: Option<String>, ids: Vec<String>, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.storage.reorder_quests(parent_id.as_deref(), &ids).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn move_quest(id: String, parent_id: Option<String>, state: tauri::State<'_, AppState>) -> CommandResult<Quest> {
    state.storage.move_quest(&id, parent_id.as_deref()).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn move_materials(source: String, destination: String, ids: Vec<String>, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.storage.move_materials(&source, &destination, &ids).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn set_materials_trashed(quest_id: String, ids: Vec<String>, trashed: bool, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.storage.set_materials_trashed(&quest_id, &ids, trashed).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn open_materials(quest_id: String, ids: Vec<String>, state: tauri::State<'_, AppState>) -> CommandResult<Vec<RestoreOutcome>> {
    let materials = state.storage.selected_materials(&quest_id, &ids).map_err(|error| error.to_string())?;
    let resources = materials.into_iter().map(|material| quest_core::Resource { id: material.id, save_point_id: String::new(), adapter_type: quest_core::AdapterType::BrowserTab, resource_uri: material.resource_uri, state_json: material.state_json, restore_order: 0 }).collect::<Vec<_>>();
    Ok(quest_adapters::restore_resources(&resources.iter().collect::<Vec<_>>()))
}

// Resolve the reference from storage rather than accepting a path from the UI.
#[tauri::command(rename_all = "camelCase")]
fn open_quest_file(quest_id: String, id: String, state: tauri::State<'_, AppState>) -> CommandResult<RestoreOutcome> {
    let quest = state.storage.get_quest(&quest_id).map_err(|error| error.to_string())?;
    let file = quest.files.into_iter().find(|file| file.id == id)
        .ok_or_else(|| "This file reference is no longer in the quest. Reopen the picker to refresh.".to_owned())?;
    let resource = quest_core::Resource {
        id: file.id,
        save_point_id: String::new(),
        adapter_type: quest_core::AdapterType::File,
        resource_uri: file.path,
        state_json: Default::default(),
        restore_order: 0,
    };
    Ok(quest_adapters::restore_resource(&resource))
}

#[tauri::command(rename_all = "camelCase")]
fn add_quest_files(
    quest_id: String,
    paths: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Quest> {
    state
        .storage
        .add_quest_files(&quest_id, &paths)
        .map_err(|error| error.to_string())?;
    state
        .storage
        .get_quest(&quest_id)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn remove_quest_file(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state
        .storage
        .remove_quest_file(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn create_local_save_point(
    quest_id: String,
    checkpoint: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<SavePoint> {
    state
        .storage
        .create_local_save_point(&quest_id, &checkpoint)
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn preview_continue(
    quest_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<SavePoint> {
    state
        .storage
        .get_quest(&quest_id)
        .map_err(|error| error.to_string())?
        .latest_save
        .ok_or_else(|| "quest has no save point".to_owned())
}

#[tauri::command(rename_all = "camelCase")]
fn continue_quest(
    save_point_id: String,
    resource_ids: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RestoreOutcome>> {
    let save = state
        .storage
        .get_save_point(&save_point_id)
        .map_err(|error| error.to_string())?;
    state.storage.get_quest(&save.quest_id).map_err(|error| error.to_string())?;
    let selected = resource_ids.map(|ids| ids.into_iter().collect::<HashSet<_>>());
    if let Some(selected) = &selected {
        let available = save
            .resources
            .iter()
            .map(|resource| resource.id.as_str())
            .collect::<HashSet<_>>();
        if let Some(unknown) = selected.iter().find(|id| !available.contains(id.as_str())) {
            return Err(format!("resource does not belong to save point: {unknown}"));
        }
    }
    let resources = save
        .resources
        .iter()
        .filter(|resource| {
            selected
                .as_ref()
                .map_or(true, |ids| ids.contains(&resource.id))
        })
        .collect::<Vec<_>>();
    let outcomes = quest_adapters::restore_resources(&resources);
    state
        .storage
        .touch_last_active(&save.quest_id)
        .map_err(|error| error.to_string())?;
    Ok(outcomes)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(AppState {
                storage: Storage::open_default()?,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_quests,
            trash_quest,
            list_trashed_quests,
            restore_quest,
            get_quest,
            create_quest,
            update_quest,
            move_quest,
            reorder_quests,
            move_materials,
            open_materials,
            open_quest_file,
            set_materials_trashed,
            add_quest_files,
            remove_quest_file,
            create_local_save_point,
            preview_continue,
            continue_quest,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Quest Journal");
}
