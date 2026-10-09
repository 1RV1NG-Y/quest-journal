use std::io::{BufReader, Write};
use std::process::{Command, Stdio};

use quest_native_host::{read_frame, write_frame, NativeResponse};

#[test]
fn native_binary_keeps_stdout_framed_across_multiple_requests() {
    let directory = tempfile::tempdir().unwrap();
    let mut input = Vec::new();
    for request in [
        br#"{"type":"list_quests"}"#.as_slice(),
        b"not json".as_slice(),
    ] {
        write_frame(&mut input, request).unwrap();
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_quest-native-host"))
        .env(
            "QUEST_JOURNAL_DB",
            directory.path().join("isolated.sqlite3"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let mut reader = BufReader::new(output.stdout.as_slice());
    let first: NativeResponse =
        serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
    assert!(first.ok);
    assert_eq!(first.data.unwrap(), serde_json::json!([]));
    let second: NativeResponse =
        serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
    assert!(!second.ok);
    assert!(second.error.unwrap().starts_with("invalid request:"));
    assert!(read_frame(&mut reader).unwrap().is_none());
}

#[cfg(windows)]
#[test]
fn renamed_windows_hosts_capture_their_browser_in_the_configured_database() {
    use quest_core::BrowserKind;
    use quest_storage::Storage;
    use serde_json::json;

    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("shared.sqlite3");
    std::fs::write(
        directory.path().join("quest-native-host.json"),
        json!({"database":database}).to_string(),
    )
    .unwrap();
    let storage = Storage::open(&database).unwrap();
    for (name, kind) in [
        ("chrome", BrowserKind::Chrome),
        ("edge", BrowserKind::Edge),
        ("brave", BrowserKind::Brave),
        ("chromium", BrowserKind::Chromium),
    ] {
        let executable = directory
            .path()
            .join(format!("quest-native-host-{name}.exe"));
        std::fs::copy(env!("CARGO_BIN_EXE_quest-native-host"), &executable).unwrap();
        let mut child = Command::new(executable)
            // The installed config must take precedence over a different launch environment.
            .env("QUEST_JOURNAL_DB", directory.path().join("wrong.sqlite3"))
            .arg("chrome-extension://example/")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut writer = child.stdin.take().unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        write_frame(
            &mut writer,
            br#"{"type":"create_quest","title":"Host capture"}"#,
        )
        .unwrap();
        let response: NativeResponse =
            serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
        assert!(response.ok, "{:?}", response.error);
        let id = response.data.unwrap()["id"].as_str().unwrap().to_owned();
        let request = json!({"type":"pause_quest","quest_id":id,"checkpoint":"",
            "tabs":[{"id":1,"url":"https://example.com","title":"Example","pinned":false,
                "active":true,"index":0,"browser_kind":"brave_flatpak"}]});
        write_frame(&mut writer, request.to_string().as_bytes()).unwrap();
        let response: NativeResponse =
            serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
        assert!(response.ok, "{:?}", response.error);
        let save_id = response.data.unwrap()["save_point_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let save = storage.get_save_point(&save_id).unwrap();
        assert_eq!(save.resources[0].state_json["browser_kind"], json!(kind));
        drop(writer);
        assert!(child.wait_with_output().unwrap().status.success());
    }
    assert!(!directory.path().join("wrong.sqlite3").exists());
}
