use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestDesignation {
    Main,
    Side,
}

impl Default for QuestDesignation {
    fn default() -> Self {
        Self::Side
    }
}

impl QuestDesignation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Side => "side",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestState {
    Active,
    Paused,
    Dormant,
    Waiting,
    Completed,
    Abandoned,
}

impl Default for QuestState {
    fn default() -> Self {
        Self::Active
    }
}

impl QuestState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Dormant => "dormant",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    BrowserTab,
    File,
}

impl AdapterType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BrowserTab => "browser_tab",
            Self::File => "file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    BraveFlatpak,
    HeliumAppImage,
    Chrome,
    Edge,
    Brave,
    Chromium,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub save_point_id: String,
    pub adapter_type: AdapterType,
    pub resource_uri: String,
    pub state_json: Value,
    pub restore_order: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavePoint {
    pub id: String,
    pub quest_id: String,
    pub checkpoint: String,
    pub created_at: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestFile {
    pub id: String,
    pub quest_id: String,
    pub path: String,
    pub label: String,
    pub added_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub quest_id: String,
    pub resource_uri: String,
    pub state_json: Value,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    pub icon: String,
    pub materials: Vec<Material>,
    pub trashed_materials: Vec<Material>,
    pub parent_id: Option<String>,
    pub id: String,
    pub title: String,
    pub territory: String,
    pub objective: String,
    pub designation: QuestDesignation,
    pub state: QuestState,
    pub current_checkpoint: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_active_at: Option<String>,
    pub files: Vec<QuestFile>,
    pub latest_save: Option<SavePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateQuestInput {
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub territory: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub designation: QuestDesignation,
    #[serde(default)]
    pub state: QuestState,
    #[serde(default)]
    pub current_checkpoint: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateQuestInput {
    pub icon: Option<String>,
    pub title: Option<String>,
    pub territory: Option<String>,
    pub objective: Option<String>,
    pub designation: Option<QuestDesignation>,
    pub state: Option<QuestState>,
    pub current_checkpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub pinned: bool,
    pub active: bool,
    pub index: u32,
    #[serde(default)]
    pub group_id: Option<i64>,
    #[serde(default)]
    pub browser_kind: Option<BrowserKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOutcome {
    pub resource_id: String,
    pub adapter_type: AdapterType,
    pub resource_uri: String,
    pub ok: bool,
    pub error: Option<String>,
}
