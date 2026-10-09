use std::{
    collections::HashSet,
    io::{self, Read, Write},
};

use quest_adapters::validate_browser_url;
use quest_core::{
    BrowserKind, BrowserTab, CreateQuestInput, QuestDesignation, QuestState, SavePoint,
};
use quest_storage::Storage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

pub const HOST_NAME: &str = "com.quest_journal.native_host";
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRequest {
    ListQuests,
    CreateQuest {
        title: String,
        #[serde(default)]
        parent_id: Option<String>,
    },
    AddTabs {
        quest_id: String,
        tabs: Vec<BrowserTab>,
    },
    PauseQuest {
        quest_id: String,
        checkpoint: String,
        tabs: Vec<BrowserTab>,
        #[serde(default)]
        close_after_save: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl NativeResponse {
    pub fn success<T: Serialize>(data: T) -> Self {
        match serde_json::to_value(data) {
            Ok(data) => Self {
                ok: true,
                data: Some(data),
                error: None,
            },
            Err(error) => Self::error(format!("failed to serialize response: {error}")),
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Error)]
pub enum FramingError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("native message is too large: {0} bytes")]
    MessageTooLarge(usize),
    #[error("response is too large: {0} bytes")]
    ResponseTooLarge(usize),
}

pub type Result<T, E = FramingError> = std::result::Result<T, E>;

pub fn read_frame(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    let mut prefix = [0u8; 4];
    match reader.read(&mut prefix[..1]) {
        Ok(0) => return Ok(None),
        Ok(1) => {}
        Ok(_) => unreachable!(),
        Err(error) => return Err(error.into()),
    }
    reader.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_MESSAGE_BYTES {
        return Err(FramingError::MessageTooLarge(length));
    }
    let mut message = vec![0; length];
    reader.read_exact(&mut message)?;
    Ok(Some(message))
}

pub fn write_frame(writer: &mut impl Write, message: &[u8]) -> Result<()> {
    let length =
        u32::try_from(message.len()).map_err(|_| FramingError::ResponseTooLarge(message.len()))?;
    if message.len() > MAX_MESSAGE_BYTES {
        return Err(FramingError::ResponseTooLarge(message.len()));
    }
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(message)?;
    writer.flush()?;
    Ok(())
}

pub fn handle_message(storage: &Storage, message: &[u8]) -> NativeResponse {
    handle_message_with_browser(
        storage,
        message,
        detect_browser_kind(
            std::env::var("FLATPAK_ID").ok().as_deref(),
            std::env::var("APPIMAGE").ok().as_deref(),
        ),
    )
}

/// Browser identity comes from the installed host, never from extension tab metadata.
pub fn handle_message_with_browser(
    storage: &Storage,
    message: &[u8],
    browser_kind: Option<BrowserKind>,
) -> NativeResponse {
    let request = match serde_json::from_slice::<NativeRequest>(message) {
        Ok(request) => request,
        Err(error) => return NativeResponse::error(format!("invalid request: {error}")),
    };
    match handle_request(storage, request, browser_kind) {
        Ok(data) => NativeResponse::success(data),
        Err(error) => NativeResponse::error(error),
    }
}

fn handle_request(
    storage: &Storage,
    request: NativeRequest,
    browser_kind: Option<BrowserKind>,
) -> std::result::Result<Value, String> {
    match request {
        NativeRequest::ListQuests => {
            let quests = storage.list_quests().map_err(|error| error.to_string())?;
            serde_json::to_value(quests).map_err(|error| error.to_string())
        }
        NativeRequest::CreateQuest { title, parent_id } => {
            let title = title.trim();
            if title.len() < 2 {
                return Err("quest title must contain at least two characters".into());
            }
            let quest = storage
                .create_quest(CreateQuestInput {
                    icon: String::new(),
                    parent_id,
                    title: title.to_owned(),
                    territory: "Browser".into(),
                    objective: String::new(),
                    designation: QuestDesignation::Side,
                    state: QuestState::Active,
                    current_checkpoint: String::new(),
                })
                .map_err(|error| error.to_string())?;
            serde_json::to_value(quest).map_err(|error| error.to_string())
        }
        NativeRequest::AddTabs { quest_id, mut tabs } => {
            if tabs.is_empty() {
                return Err("Select at least one tab".into());
            }
            let mut ids = HashSet::new();
            for tab in &mut tabs {
                validate_browser_url(&tab.url)
                    .map_err(|error| format!("unsafe tab URL: {error}"))?;
                if !ids.insert(tab.id) {
                    return Err("duplicate tab id".into());
                }
                tab.browser_kind = browser_kind;
            }
            storage
                .add_tabs(&quest_id, &tabs)
                .map_err(|error| error.to_string())?;
            Ok(json!({"added_count": tabs.len()}))
        }
        NativeRequest::PauseQuest {
            quest_id,
            checkpoint,
            mut tabs,
            close_after_save,
        } => {
            if quest_id.trim().is_empty() {
                return Err("quest_id cannot be empty".into());
            }
            let mut tab_ids = HashSet::with_capacity(tabs.len());
            for tab in &mut tabs {
                validate_browser_url(&tab.url)
                    .map_err(|error| format!("unsafe tab URL: {error}"))?;
                if !tab_ids.insert(tab.id) {
                    return Err(format!("duplicate tab id: {}", tab.id));
                }
                tab.browser_kind = browser_kind;
            }
            let save = storage
                .create_save_point(&quest_id, &checkpoint, &tabs)
                .map_err(|error| error.to_string())?;
            Ok(pause_data(save, close_after_save))
        }
    }
}

fn pause_data(save: SavePoint, close_after_save: bool) -> Value {
    json!({ "save_point_id": save.id, "close_after_save": close_after_save })
}

fn detect_browser_kind(flatpak_id: Option<&str>, appimage: Option<&str>) -> Option<BrowserKind> {
    if flatpak_id == Some("com.brave.Browser") {
        return Some(BrowserKind::BraveFlatpak);
    }
    appimage
        .filter(|path| path.to_ascii_lowercase().contains("helium"))
        .map(|_| BrowserKind::HeliumAppImage)
}

/// Read the installer's adjacent shared-database configuration. Missing config
/// keeps the existing default database workflow; malformed config fails closed.
pub fn configured_database(
    executable: &std::path::Path,
) -> std::result::Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct HostConfig {
        database: std::path::PathBuf,
    }
    let path = executable
        .parent()
        .ok_or("host has no parent directory")?
        .join("quest-native-host.json");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let config: HostConfig = serde_json::from_slice(&bytes)?;
    if !config.database.is_absolute() {
        return Err("native host database must be absolute".into());
    }
    Ok(Some(config.database))
}

/// Windows installers make one permanent executable copy per browser. Chromium passes
/// the extension origin as argv[1], so browser identity cannot use positional arguments.
pub fn browser_from_host_name(name: &str) -> Option<BrowserKind> {
    match name.to_ascii_lowercase().as_str() {
        "quest-native-host-chrome.exe" => Some(BrowserKind::Chrome),
        "quest-native-host-edge.exe" => Some(BrowserKind::Edge),
        "quest-native-host-brave.exe" => Some(BrowserKind::Brave),
        "quest-native-host-chromium.exe" => Some(BrowserKind::Chromium),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quest_core::{CreateQuestInput, QuestDesignation, QuestState};
    use std::io::Cursor;

    #[test]
    fn installed_host_database_configuration_is_absolute_and_strict() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("quest-native-host-chrome.exe");
        assert!(configured_database(&executable).unwrap().is_none());
        let config = directory.path().join("quest-native-host.json");
        let database = directory.path().join("shared.sqlite");
        std::fs::write(&config, json!({"database":database}).to_string()).unwrap();
        assert_eq!(configured_database(&executable).unwrap(), Some(database));
        for invalid in [
            json!({"database":"relative.sqlite"}),
            json!({"database":"x","browser":"cmd.exe"}),
        ] {
            std::fs::write(&config, invalid.to_string()).unwrap();
            assert!(configured_database(&executable).is_err());
        }
    }

    #[test]
    fn installed_host_identity_is_allowlisted() {
        for (name, kind) in [
            ("chrome", BrowserKind::Chrome),
            ("edge", BrowserKind::Edge),
            ("brave", BrowserKind::Brave),
            ("chromium", BrowserKind::Chromium),
        ] {
            assert_eq!(
                browser_from_host_name(&format!("quest-native-host-{name}.exe")),
                Some(kind)
            );
        }
        assert_eq!(browser_from_host_name("evil.exe"), None);
        assert_eq!(browser_from_host_name("quest-native-host.exe"), None);
    }

    #[test]
    fn capture_overrides_extension_browser_identity() {
        let storage = Storage::open(":memory:").unwrap();
        let response = handle_message(
            &storage,
            br#"{"type":"create_quest","title":"Windows tabs"}"#,
        );
        let id = response.data.unwrap()["id"].as_str().unwrap().to_owned();
        for kind in [
            BrowserKind::Chrome,
            BrowserKind::Edge,
            BrowserKind::Brave,
            BrowserKind::Chromium,
        ] {
            let request = json!({"type":"pause_quest","quest_id":id,"checkpoint":"",
                "tabs":[{"id":1,"url":"https://example.com","title":"Example","pinned":false,
                    "active":true,"index":0,"browser_kind":"brave_flatpak"}]});
            let response =
                handle_message_with_browser(&storage, request.to_string().as_bytes(), Some(kind));
            assert!(response.ok, "{:?}", response.error);
            let data = response.data.unwrap();
            let save = storage
                .get_save_point(data["save_point_id"].as_str().unwrap())
                .unwrap();
            assert_eq!(
                save.resources[0].state_json["browser_kind"],
                serde_json::to_value(kind).unwrap()
            );
            let request = json!({"type":"add_tabs","quest_id":id,
                "tabs":[{"id":1,"url":"https://example.com","title":"Example","pinned":false,
                    "active":true,"index":0,"browser_kind":"brave_flatpak"}]});
            let response =
                handle_message_with_browser(&storage, request.to_string().as_bytes(), Some(kind));
            assert!(response.ok, "{:?}", response.error);
            let quest = storage.get_quest(&id).unwrap();
            assert_eq!(quest.materials[0].state_json["browser_kind"], json!(kind));
        }
    }

    #[test]
    fn native_frame_is_little_endian_and_round_trips() {
        let payload = br#"{"type":"list_quests"}"#;
        let mut bytes = Vec::new();
        write_frame(&mut bytes, payload).unwrap();
        assert_eq!(&bytes[..4], &(payload.len() as u32).to_le_bytes());
        assert_eq!(
            read_frame(&mut Cursor::new(bytes)).unwrap().unwrap(),
            payload
        );
    }

    #[test]
    fn clean_eof_is_not_a_malformed_frame() {
        assert!(read_frame(&mut Cursor::new(Vec::<u8>::new()))
            .unwrap()
            .is_none());
    }

    #[test]
    fn creates_a_lightweight_browser_quest() {
        let storage = Storage::open(":memory:").unwrap();
        let response = handle_message(
            &storage,
            br#"{"type":"create_quest","title":"Compare browser models"}"#,
        );
        assert!(response.ok);
        let quest_id = response.data.unwrap()["id"].as_str().unwrap().to_owned();
        let quest = storage.get_quest(&quest_id).unwrap();
        assert_eq!(quest.title, "Compare browser models");
        assert_eq!(quest.territory, "Browser");
        assert_eq!(quest.designation, QuestDesignation::Side);
    }

    #[test]
    fn malformed_and_unsafe_requests_are_rejected() {
        let storage = Storage::open(":memory:").unwrap();
        assert!(!handle_message(&storage, b"not json").ok);
        let request = br#"{"type":"pause_quest","quest_id":"missing","checkpoint":"x","tabs":[{"id":1,"url":"javascript:alert(1)","title":"bad","pinned":false,"active":true,"index":0}],"close_after_save":true}"#;
        let response = handle_message(&storage, request);
        assert!(!response.ok);
        assert!(response.error.unwrap().contains("unsafe tab URL"));
    }

    #[test]
    fn pause_only_succeeds_after_save_is_queryable() {
        let storage = Storage::open(":memory:").unwrap();
        let quest = storage
            .create_quest(CreateQuestInput {
                icon: String::new(),
                parent_id: None,
                title: "Quest".into(),
                territory: String::new(),
                objective: String::new(),
                designation: QuestDesignation::Main,
                state: QuestState::Active,
                current_checkpoint: String::new(),
            })
            .unwrap();
        let request = json!({
            "type": "pause_quest", "quest_id": quest.id, "checkpoint": "Saved",
            "tabs": [{"id": 1, "url": "https://example.com", "title": "Example", "pinned": false, "active": true, "index": 0}],
            "close_after_save": true
        });
        let response = handle_message(&storage, request.to_string().as_bytes());
        assert!(response.ok);
        let save_id = response.data.unwrap()["save_point_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            storage.get_save_point(&save_id).unwrap().checkpoint,
            "Saved"
        );
    }

    #[test]
    fn detects_supported_browser_launch_environments() {
        assert_eq!(
            detect_browser_kind(Some("com.brave.Browser"), None),
            Some(BrowserKind::BraveFlatpak)
        );
        assert_eq!(
            detect_browser_kind(None, Some("/home/user/Applications/helium.AppImage")),
            Some(BrowserKind::HeliumAppImage)
        );
        assert_eq!(detect_browser_kind(None, None), None);
    }
    #[test]
    fn native_saves_accept_blank_checkpoint_notes() {
        let storage = Storage::open(":memory:").unwrap();
        let response = handle_message(
            &storage,
            br#"{"type":"create_quest","title":"No note needed"}"#,
        );
        let id = response.data.unwrap()["id"].as_str().unwrap().to_owned();
        for checkpoint in ["", "   "] {
            let request = json!({
                "type":"pause_quest", "quest_id":id, "checkpoint":checkpoint,
                "tabs":[{"id":1,"url":"https://example.com","title":"Example","pinned":false,"active":true,"index":0}],
                "close_after_save":true
            });
            let response = handle_message(&storage, request.to_string().as_bytes());
            assert!(response.ok, "{:?}", response.error);
            let save_id = response.data.unwrap()["save_point_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let save = storage.get_save_point(&save_id).unwrap();
            assert_eq!(save.checkpoint, "");
            assert_eq!(save.resources.len(), 1);
        }
    }
}
