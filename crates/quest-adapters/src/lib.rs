use std::{path::PathBuf, process::Command};

use quest_core::{AdapterType, Resource, RestoreOutcome};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum RestoreError {
    #[error("unsupported URL scheme: {0}")]
    UnsupportedScheme(String),
    #[error("browser URL has no host")]
    MissingHost,
    #[error("browser URL credentials are not allowed")]
    CredentialsNotAllowed,
    #[error("invalid browser URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("file does not exist: {0}")]
    MissingFile(String),
    #[error("path is not a file: {0}")]
    NotAFile(String),
    #[error("resource path cannot be empty")]
    EmptyPath,
    #[error("saved browser is unavailable: {0}")]
    MissingBrowser(String),
    #[error("failed to open resource: {0}")]
    Launch(#[from] std::io::Error),
}

pub type Result<T, E = RestoreError> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedResource {
    BrowserUrl(Url),
    File(PathBuf),
}

pub fn validate_browser_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw)?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(RestoreError::UnsupportedScheme(scheme.to_owned())),
    }
    if url.host_str().is_none() {
        return Err(RestoreError::MissingHost);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(RestoreError::CredentialsNotAllowed);
    }
    Ok(url)
}

pub fn validate_file_path(raw: &str) -> Result<PathBuf> {
    if raw.trim().is_empty() {
        return Err(RestoreError::EmptyPath);
    }
    let path = PathBuf::from(raw);
    let metadata = std::fs::metadata(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            RestoreError::MissingFile(raw.to_owned())
        } else {
            RestoreError::Launch(error)
        }
    })?;
    if !metadata.is_file() {
        return Err(RestoreError::NotAFile(raw.to_owned()));
    }
    Ok(path)
}

pub fn validate_resource(resource: &Resource) -> Result<ValidatedResource> {
    match resource.adapter_type {
        AdapterType::BrowserTab => {
            validate_browser_url(&resource.resource_uri).map(ValidatedResource::BrowserUrl)
        }
        AdapterType::File => {
            validate_file_path(&resource.resource_uri).map(ValidatedResource::File)
        }
    }
}

pub fn restore_resource(resource: &Resource) -> RestoreOutcome {
    let result =
        validate_resource(resource).and_then(|validated| open_resource(resource, validated));
    RestoreOutcome {
        resource_id: resource.id.clone(),
        adapter_type: resource.adapter_type,
        resource_uri: resource.resource_uri.clone(),
        ok: result.is_ok(),
        error: result.err().map(|error| error.to_string()),
    }
}

pub fn restore_resources(resources: &[&Resource]) -> Vec<RestoreOutcome> {
    let mut outcomes = Vec::with_capacity(resources.len());
    let mut groups = std::collections::BTreeMap::<&str, Vec<_>>::new();
    for (position, resource) in resources.iter().copied().enumerate() {
        if let Some(kind) = browser_group_key(resource) {
            match validate_browser_url(&resource.resource_uri) {
                Ok(url) => groups
                    .entry(kind)
                    .or_default()
                    .push((position, resource, url)),
                Err(error) => outcomes.push((
                    position,
                    RestoreOutcome {
                        resource_id: resource.id.clone(),
                        adapter_type: resource.adapter_type,
                        resource_uri: resource.resource_uri.clone(),
                        ok: false,
                        error: Some(error.to_string()),
                    },
                )),
            }
        } else {
            outcomes.push((position, restore_resource(resource)));
        }
    }

    for (browser_kind, group) in groups {
        if group.is_empty() {
            continue;
        }
        let error = open_browser_group(browser_kind, &group)
            .err()
            .map(|error| error.to_string());
        for (position, resource, _) in group {
            outcomes.push((
                position,
                RestoreOutcome {
                    resource_id: resource.id.clone(),
                    adapter_type: resource.adapter_type,
                    resource_uri: resource.resource_uri.clone(),
                    ok: error.is_none(),
                    error: error.clone(),
                },
            ));
        }
    }

    outcomes.sort_by_key(|(position, _)| *position);
    outcomes.into_iter().map(|(_, outcome)| outcome).collect()
}

fn browser_group_key(resource: &Resource) -> Option<&str> {
    if resource.adapter_type != AdapterType::BrowserTab {
        return None;
    }
    let kind = resource.state_json["browser_kind"].as_str()?;
    #[cfg(target_os = "linux")]
    if matches!(kind, "brave_flatpak" | "helium_app_image") {
        return Some(kind);
    }
    #[cfg(target_os = "windows")]
    if windows_browser_relative_path(kind).is_some() {
        return Some(kind);
    }
    let _ = kind;
    None
}

fn open_browser_group(browser_kind: &str, resources: &[(usize, &Resource, Url)]) -> Result<()> {
    let mut command = browser_group_command(browser_kind)?;
    command.arg("--new-window");
    for (_, _, url) in resources {
        command.arg(url.as_str());
    }
    command.spawn()?;
    Ok(())
}

fn browser_group_command(browser_kind: &str) -> Result<Command> {
    #[cfg(target_os = "linux")]
    match browser_kind {
        "brave_flatpak" => {
            let mut command = Command::new("flatpak");
            command.args(["run", "com.brave.Browser"]);
            return Ok(command);
        }
        "helium_app_image" => {
            return Ok(Command::new(
                helium_appimage().ok_or_else(|| RestoreError::MissingBrowser("Helium".into()))?,
            ))
        }
        _ => {}
    }
    #[cfg(target_os = "windows")]
    if windows_browser_relative_path(browser_kind).is_some() {
        return windows_browser_command(browser_kind);
    }
    Err(RestoreError::MissingBrowser(browser_kind.into()))
}

/// Only fixed installation-relative executable names are accepted. Saved metadata
/// cannot choose an executable or inject command-line flags.
#[cfg(any(windows, test))]
fn windows_browser_relative_path(kind: &str) -> Option<&'static str> {
    match kind {
        "chrome" => Some("Google/Chrome/Application/chrome.exe"),
        "edge" => Some("Microsoft/Edge/Application/msedge.exe"),
        "brave" => Some("BraveSoftware/Brave-Browser/Application/brave.exe"),
        "chromium" => Some("Chromium/Application/chrome.exe"),
        _ => None,
    }
}

#[cfg(any(windows, test))]
fn windows_browser_command_in(kind: &str, roots: &[PathBuf]) -> Result<Command> {
    let relative = windows_browser_relative_path(kind)
        .ok_or_else(|| RestoreError::MissingBrowser(kind.into()))?;
    let executable = roots
        .iter()
        .filter(|root| root.is_absolute())
        .map(|root| root.join(relative))
        .find(|path| path.is_file())
        .ok_or_else(|| RestoreError::MissingBrowser(kind.into()))?;
    Ok(Command::new(executable))
}

#[cfg(windows)]
fn windows_browser_command(kind: &str) -> Result<Command> {
    let roots = ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    windows_browser_command_in(kind, &roots)
}

fn open_resource(resource: &Resource, validated: ValidatedResource) -> Result<()> {
    let target = match validated {
        ValidatedResource::BrowserUrl(url) => std::ffi::OsString::from(url.as_str()),
        ValidatedResource::File(path) => path.into_os_string(),
    };
    platform_command(resource, target)?.spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn platform_command(resource: &Resource, target: std::ffi::OsString) -> Result<Command> {
    if resource.adapter_type == AdapterType::BrowserTab {
        match resource.state_json["browser_kind"].as_str() {
            Some("brave_flatpak") => {
                let mut command = Command::new("flatpak");
                command.args(["run", "com.brave.Browser"]).arg(target);
                return Ok(command);
            }
            Some("helium_app_image") => {
                if let Some(executable) = helium_appimage() {
                    let mut command = Command::new(executable);
                    command.arg(target);
                    return Ok(command);
                }
            }
            _ => {}
        }
    }
    let mut command = Command::new("xdg-open");
    command.arg(target);
    Ok(command)
}

#[cfg(target_os = "linux")]
fn helium_appimage() -> Option<PathBuf> {
    let applications = PathBuf::from(std::env::var_os("HOME")?).join("Applications");
    std::fs::read_dir(applications)
        .ok()?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            path.is_file() && name.starts_with("helium-") && name.ends_with(".appimage")
        })
        .max()
}

#[cfg(target_os = "macos")]
fn platform_command(_resource: &Resource, target: std::ffi::OsString) -> Result<Command> {
    let mut command = Command::new("open");
    command.arg(target);
    Ok(command)
}

#[cfg(target_os = "windows")]
fn platform_command(resource: &Resource, target: std::ffi::OsString) -> Result<Command> {
    if resource.adapter_type == AdapterType::BrowserTab {
        if let Some(kind) = resource.state_json["browser_kind"].as_str() {
            if windows_browser_relative_path(kind).is_some() {
                let mut command = windows_browser_command(kind)?;
                command.arg(target);
                return Ok(command);
            }
        }
    }
    let mut command = Command::new("explorer.exe");
    command.arg(target);
    Ok(command)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_command(_resource: &Resource, _target: std::ffi::OsString) -> Result<Command> {
    Ok(Command::new("false"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_browser_commands_use_allowlisted_executables_and_literal_urls() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("Program Files with spaces");
        for kind in ["chrome", "edge", "brave", "chromium"] {
            let executable = root.join(windows_browser_relative_path(kind).unwrap());
            std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
            std::fs::write(&executable, "test executable").unwrap();
            let mut command = windows_browser_command_in(kind, &[root.clone()]).unwrap();
            let url = validate_browser_url("https://example.com/?q=a&other=%22%20%26%20calc.exe")
                .unwrap();
            command.arg("--new-window").arg(url.as_str());
            assert_eq!(command.get_program(), executable);
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                ["--new-window", url.as_str()]
            );
        }
        assert!(windows_browser_command_in("cmd.exe", &[root]).is_err());
        assert!(windows_browser_command_in("chrome", &[directory.path().join("missing")]).is_err());
    }

    #[test]
    fn windows_browser_lookup_does_not_use_the_working_directory() {
        // Construct an existing relative candidate without changing the process's cwd.
        let directory = tempfile::tempdir_in(".").unwrap();
        let root = directory
            .path()
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .to_path_buf();
        assert!(!root.is_absolute());
        let executable = root.join(windows_browser_relative_path("chrome").unwrap());
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, "test executable").unwrap();
        assert!(executable.is_file());
        assert!(windows_browser_command_in("chrome", &[root, PathBuf::new()]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_tabs_are_grouped_by_saved_browser_and_files_are_not() {
        let mut resource = Resource {
            id: "r".into(),
            save_point_id: "s".into(),
            adapter_type: AdapterType::BrowserTab,
            resource_uri: "https://example.com".into(),
            state_json: serde_json::json!({"browser_kind":"edge"}),
            restore_order: 0,
        };
        assert_eq!(browser_group_key(&resource), Some("edge"));
        resource.adapter_type = AdapterType::File;
        assert_eq!(browser_group_key(&resource), None);
    }

    #[test]
    fn only_plain_http_urls_are_restorable() {
        assert!(validate_browser_url("https://example.com/path?q=1").is_ok());
        assert!(validate_browser_url("http://localhost:3000").is_ok());
        assert!(validate_browser_url("javascript:alert(1)").is_err());
        assert!(validate_browser_url("file:///etc/passwd").is_err());
        assert!(validate_browser_url("https://user:secret@example.com").is_err());
    }

    #[test]
    fn file_must_exist_and_be_a_regular_file() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("notes.txt");
        std::fs::write(&file, "notes").unwrap();
        assert_eq!(validate_file_path(file.to_str().unwrap()).unwrap(), file);
        assert!(matches!(
            validate_file_path(directory.path().to_str().unwrap()),
            Err(RestoreError::NotAFile(_))
        ));
        assert!(matches!(
            validate_file_path(directory.path().join("missing").to_str().unwrap()),
            Err(RestoreError::MissingFile(_))
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn brave_saves_restore_through_the_brave_flatpak() {
        let resource = Resource {
            id: "resource".into(),
            save_point_id: "save".into(),
            adapter_type: AdapterType::BrowserTab,
            resource_uri: "https://example.com".into(),
            state_json: serde_json::json!({ "browser_kind": "brave_flatpak" }),
            restore_order: 0,
        };
        let command = platform_command(&resource, resource.resource_uri.clone().into()).unwrap();
        assert_eq!(command.get_program(), "flatpak");
        assert_eq!(
            command
                .get_args()
                .map(|argument| argument.to_string_lossy())
                .collect::<Vec<_>>(),
            ["run", "com.brave.Browser", "https://example.com"]
        );
    }
}
