use quest_core::{BrowserTab, CreateQuestInput, QuestDesignation, QuestState};
use quest_storage::Storage;

#[test]
fn quest_files_and_latest_save_survive_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("journal.sqlite3");
    let attached = directory.path().join("reference.txt");
    std::fs::write(&attached, "reference").unwrap();

    let quest_id;
    let save_id;
    let created_at;
    {
        let storage = Storage::open(&database).unwrap();
        let quest = storage
            .create_quest(CreateQuestInput {
                icon: String::new(),
                parent_id: None,
                title: "Ship the journal".into(),
                territory: "Product".into(),
                objective: "Complete the local-first round trip".into(),
                designation: QuestDesignation::Main,
                state: QuestState::Active,
                current_checkpoint: "Connect storage".into(),
            })
            .unwrap();
        created_at = quest.created_at.clone();
        quest_id = quest.id;
        storage
            .add_quest_files(&quest_id, &[attached.to_string_lossy().into_owned()])
            .unwrap();
        let save = storage
            .create_save_point(
                &quest_id,
                "Native capture persisted",
                &[
                    BrowserTab {
                        id: 8,
                        url: "https://example.com/second".into(),
                        title: "Second".into(),
                        pinned: false,
                        active: true,
                        index: 4,
                        group_id: Some(19),
                        browser_kind: Some(quest_core::BrowserKind::HeliumAppImage),
                    },
                    BrowserTab {
                        id: 3,
                        url: "https://example.com/first".into(),
                        title: "First".into(),
                        pinned: true,
                        active: false,
                        index: 1,
                        group_id: Some(19),
                        browser_kind: Some(quest_core::BrowserKind::HeliumAppImage),
                    },
                ],
            )
            .unwrap();
        assert_eq!(save.resources.len(), 3);
        assert_eq!(save.resources[0].resource_uri, "https://example.com/first");
        assert_eq!(save.resources[2].resource_uri, attached.to_string_lossy());
        assert_eq!(save.resources[0].state_json["group_id"], 19);
        assert_eq!(
            save.resources[0].state_json["browser_kind"],
            "helium_app_image"
        );
        let capture = storage
            .create_save_point(
                &quest_id,
                "Second window persisted",
                &[
                    BrowserTab {
                        id: 11,
                        url: "https://example.com/third".into(),
                        title: "Third".into(),
                        pinned: false,
                        active: true,
                        index: 0,
                        group_id: None,
                        browser_kind: Some(quest_core::BrowserKind::HeliumAppImage),
                    },
                    BrowserTab {
                        id: 12,
                        url: "https://example.com/first".into(),
                        title: "First updated".into(),
                        pinned: false,
                        active: false,
                        index: 1,
                        group_id: None,
                        browser_kind: Some(quest_core::BrowserKind::HeliumAppImage),
                    },
                ],
            )
            .unwrap();
        save_id = capture.id;
        assert_eq!(capture.resources.len(), 3);
        assert_eq!(
            capture
                .resources
                .iter()
                .map(|resource| resource.resource_uri.as_str())
                .collect::<Vec<_>>(),
            [
                "https://example.com/third",
                "https://example.com/first",
                attached.to_string_lossy().as_ref(),
            ]
        );
        assert_eq!(capture.resources[1].state_json["title"], "First updated");
    }

    let reopened = Storage::open(&database).unwrap();
    let quest = reopened.get_quest(&quest_id).unwrap();
    assert_eq!(quest.created_at, created_at);
    assert_eq!(quest.files.len(), 1);
    assert_eq!(quest.materials.len(), 3);
    let latest = quest.latest_save.unwrap();
    assert_eq!(latest.id, save_id);
    assert_eq!(latest.resources.len(), 3);
    assert_eq!(
        latest
            .resources
            .iter()
            .map(|resource| resource.restore_order)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn update_preserves_created_at() {
    let storage = Storage::open(":memory:").unwrap();
    let created = storage
        .create_quest(CreateQuestInput {
                icon: String::new(),
                parent_id: None,
            title: "Original".into(),
            territory: String::new(),
            objective: String::new(),
            designation: QuestDesignation::Side,
            state: QuestState::Dormant,
            current_checkpoint: String::new(),
        })
        .unwrap();
    let updated = storage
        .update_quest(
            &created.id,
            quest_core::UpdateQuestInput {
                title: Some("Renamed".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(created.created_at, updated.created_at);
    assert_eq!(updated.title, "Renamed");
}

#[test]
fn hierarchy_survives_reopen_and_rejects_cycles() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hierarchy.db");
    let storage = Storage::open(&path).unwrap();
    let create = |title: &str, parent: Option<&str>| -> CreateQuestInput {
        serde_json::from_value(serde_json::json!({"title": title, "parent_id": parent})).unwrap()
    };
    let trading = storage.create_quest(create("Trading", None)).unwrap();
    let course = storage.create_quest(create("Course", Some(&trading.id))).unwrap();
    let concept = storage.create_quest(create("Concept", Some(&course.id))).unwrap();
    let save = storage.create_save_point(&concept.id, "Resume here", &[]).unwrap();
    assert!(storage.move_quest(&trading.id, Some(&concept.id)).is_err());
    assert!(storage.move_quest(&course.id, Some(&course.id)).is_err());
    assert!(storage.move_quest(&course.id, Some("missing")).is_err());
    assert!(storage.create_quest(create("Invalid", Some("missing"))).is_err());
    storage.move_quest(&course.id, None).unwrap();
    assert_eq!(storage.get_quest(&concept.id).unwrap().parent_id.as_deref(), Some(course.id.as_str()));
    drop(storage);
    let storage = Storage::open(&path).unwrap();
    assert_eq!(storage.get_quest(&course.id).unwrap().parent_id, None);
    assert_eq!(storage.get_quest(&concept.id).unwrap().latest_save.unwrap().id, save.id);
    storage.move_quest(&course.id, Some(&trading.id)).unwrap();
    assert_eq!(storage.get_quest(&course.id).unwrap().parent_id, Some(trading.id));
}

#[test]
fn legacy_database_migration_keeps_quests_and_is_repeatable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(include_str!("../../../migrations/0001_initial.sql")).unwrap();
    connection.execute("INSERT INTO quests VALUES ('old','Trading','Finance','Learn','main','paused','Continue','today','today',NULL)", []).unwrap();
    drop(connection);
    for _ in 0..2 {
        let storage = Storage::open(&path).unwrap();
        let quest = storage.get_quest("old").unwrap();
        assert_eq!(quest.title, "Trading");
        assert_eq!(quest.current_checkpoint, "Continue");
        assert_eq!(quest.parent_id, None);
    }
}

#[test]
fn material_moves_are_atomic_and_do_not_rewrite_history() {
    let storage = Storage::open(":memory:").unwrap();
    let create = |title: &str| serde_json::from_value::<CreateQuestInput>(serde_json::json!({"title":title})).unwrap();
    let source = storage.create_quest(create("Trading")).unwrap();
    let destination = storage.create_quest(create("Course")).unwrap();
    let tab = |id: i64, url: &str| BrowserTab { id, url: url.into(), title: url.into(), index: id as u32, pinned: false, active: false, group_id: None, browser_kind: None };
    let original = storage.create_save_point(&source.id, "Original", &[tab(1,"https://example.com/a"),tab(2,"https://example.com/b")]).unwrap();
    let materials = storage.get_quest(&source.id).unwrap().materials;
    storage.create_save_point(&destination.id, "Duplicate", &[tab(3,"https://example.com/b")]).unwrap();
    let ids = materials.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    assert!(storage.move_materials(&source.id, &destination.id, &ids).is_err());
    assert_eq!(storage.get_quest(&source.id).unwrap().materials.len(),2);
    storage.move_materials(&source.id, &destination.id, &ids[..1]).unwrap();
    assert_eq!(storage.get_quest(&source.id).unwrap().materials.len(),1);
    assert_eq!(storage.get_quest(&destination.id).unwrap().materials.len(),2);
    assert_eq!(storage.get_save_point(&original.id).unwrap(),original);
    let local = storage.create_local_save_point(&destination.id,"Current collection").unwrap();
    assert_eq!(local.resources.len(),2);
    let source_local = storage.create_local_save_point(&source.id,"Current collection").unwrap();
    assert_eq!(source_local.resources.len(),1);
    assert!(storage.move_materials(&source.id, &destination.id, &ids[..1]).is_err());
    assert!(storage.selected_materials(&source.id, &ids[..1]).is_err());
    let next = storage.create_save_point(&source.id, "New session", &[tab(4,"https://example.com/c")]).unwrap();
    assert_eq!(next.resources.len(),1);
    assert_eq!(storage.get_quest(&source.id).unwrap().materials.len(),2);
}

#[test]
fn material_migration_deduplicates_per_quest_and_does_not_repeat() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("materials.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(include_str!("../../../migrations/0001_initial.sql")).unwrap();
    connection.execute_batch("INSERT INTO quests VALUES ('q','Trading','','','side','paused','','1','1',NULL); INSERT INTO quests VALUES ('d','Course','','','side','paused','','1','1',NULL); INSERT INTO save_points VALUES ('s1','q','First','1'); INSERT INTO save_points VALUES ('s2','q','Second','2'); INSERT INTO resources VALUES ('r1','s1','browser_tab','https://example.com','{\"title\":\"Old\"}',0); INSERT INTO resources VALUES ('r2','s2','browser_tab','https://example.com','{\"title\":\"New\"}',0);").unwrap();
    drop(connection);
    let storage = Storage::open(&path).unwrap();
    let materials = storage.get_quest("q").unwrap().materials;
    assert_eq!(materials.len(),1);
    assert_eq!(materials[0].state_json["title"],"New");
    storage.move_materials("q","d",&[materials[0].id.clone()]).unwrap();
    drop(storage);
    let storage = Storage::open(&path).unwrap();
    assert!(storage.get_quest("q").unwrap().materials.is_empty());
    assert_eq!(storage.get_quest("d").unwrap().materials.len(),1);
    assert_eq!(storage.get_save_point("s1").unwrap().resources[0].state_json["title"],"Old");
}

#[test]
fn manual_quest_order_survives_activity_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("order.db");
    let storage = Storage::open(&path).unwrap();
    let create = |title: &str, parent: Option<&str>| serde_json::from_value::<CreateQuestInput>(serde_json::json!({"title":title,"parent_id":parent})).unwrap();
    let a = storage.create_quest(create("A",None)).unwrap();
    let b = storage.create_quest(create("B",None)).unwrap();
    let c = storage.create_quest(create("C",None)).unwrap();
    let child = storage.create_quest(create("Child",Some(&a.id))).unwrap();
    let order = vec![c.id.clone(),a.id.clone(),b.id.clone()];
    storage.reorder_quests(None,&order).unwrap();
    assert!(storage.reorder_quests(None,&[a.id.clone(),b.id.clone()]).is_err());
    assert!(storage.reorder_quests(None,&[a.id.clone(),b.id.clone(),child.id.clone()]).is_err());
    storage.touch_last_active(&b.id).unwrap();
    storage.create_local_save_point(&c.id,"checkpoint").unwrap();
    drop(storage);
    let storage = Storage::open(&path).unwrap();
    let roots = storage.list_quests().unwrap().into_iter().filter(|q|q.parent_id.is_none()).map(|q|q.id).collect::<Vec<_>>();
    assert_eq!(roots,order);
    storage.move_quest(&child.id,None).unwrap();
    let roots = storage.list_quests().unwrap().into_iter().filter(|q|q.parent_id.is_none()).map(|q|q.id).collect::<Vec<_>>();
    assert_eq!(roots.last(),Some(&child.id));
}

#[test]
fn new_capture_retains_previous_materials_and_saves() {
    let storage = Storage::open(":memory:").unwrap();
    let quest = storage.create_quest(serde_json::from_value(serde_json::json!({"title":"Trading"})).unwrap()).unwrap();
    let tab = |id: i64| BrowserTab { id, url: format!("https://example.com/{id}"), title: id.to_string(), index: id as u32, pinned: false, active: false, group_id: None, browser_kind: None };
    let first = storage.create_save_point(&quest.id,"A and B",&[tab(1),tab(2)]).unwrap();
    let second = storage.create_save_point(&quest.id,"C",&[tab(3)]).unwrap();
    let current = storage.get_quest(&quest.id).unwrap();
    assert_eq!(current.materials.iter().map(|m|m.resource_uri.as_str()).collect::<Vec<_>>(),vec!["https://example.com/1","https://example.com/2","https://example.com/3"]);
    assert_eq!(second.resources.len(),1);
    assert_eq!(storage.get_save_point(&first.id).unwrap(),first);
    assert_eq!(storage.create_local_save_point(&quest.id,"Everything").unwrap().resources.len(),3);
}

#[test]
fn trash_is_persistent_reversible_and_preserves_saved_sessions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("trash.db");
    let storage = Storage::open(&path).unwrap();
    let quest = storage.create_quest(serde_json::from_value(serde_json::json!({"title":"Trading"})).unwrap()).unwrap();
    let tab = |id: i64| BrowserTab { id, url: format!("https://example.com/{id}"), title: id.to_string(), index: id as u32, pinned: false, active: false, group_id: None, browser_kind: None };
    let original = storage.create_save_point(&quest.id,"Before trash",&[tab(1),tab(2)]).unwrap();
    let ids = storage.get_quest(&quest.id).unwrap().materials.iter().map(|m|m.id.clone()).collect::<Vec<_>>();
    assert!(storage.set_materials_trashed(&quest.id,&[ids[0].clone(),"missing".into()],true).is_err());
    assert_eq!(storage.get_quest(&quest.id).unwrap().materials.len(),2);
    storage.set_materials_trashed(&quest.id,&ids,true).unwrap();
    assert!(storage.selected_materials(&quest.id,&ids).is_err());
    assert_eq!(storage.create_local_save_point(&quest.id,"After trash").unwrap().resources.len(),0);
    assert_eq!(storage.get_save_point(&original.id).unwrap(),original);
    drop(storage);
    let storage = Storage::open(&path).unwrap();
    let current = storage.get_quest(&quest.id).unwrap();
    assert!(current.materials.is_empty());
    assert_eq!(current.trashed_materials.len(),2);
    storage.set_materials_trashed(&quest.id,&ids[..1],false).unwrap();
    assert_eq!(storage.get_quest(&quest.id).unwrap().materials.len(),1);
    storage.create_save_point(&quest.id,"Explicit recapture",&[tab(2)]).unwrap();
    let current = storage.get_quest(&quest.id).unwrap();
    assert_eq!(current.materials.len(),2);
    assert!(current.trashed_materials.is_empty());
    assert_eq!(current.materials.iter().map(|m|m.id.clone()).collect::<Vec<_>>(),ids);
}

#[test]
fn adding_tabs_preserves_checkpoint_state_and_latest_save() {
    let storage = Storage::open(":memory:").unwrap();
    let quest = storage.create_quest(serde_json::from_value(serde_json::json!({"title":"Trading"})).unwrap()).unwrap();
    let saved = storage.create_local_save_point(&quest.id,"Keep this checkpoint").unwrap();
    let tab = BrowserTab {id:1,url:"https://example.com".into(),title:"Course".into(),index:0,pinned:false,active:true,group_id:None,browser_kind:None};
    storage.add_tabs(&quest.id,&[tab.clone()]).unwrap();
    storage.add_tabs(&quest.id,&[tab]).unwrap();
    let after = storage.get_quest(&quest.id).unwrap();
    assert_eq!(after.materials.len(),1);
    assert_eq!(after.current_checkpoint,"Keep this checkpoint");
    assert_eq!(after.state,QuestState::Paused);
    assert_eq!(after.latest_save.unwrap(),saved);
}

#[test]
fn saves_accept_optional_notes_without_erasing_existing_context() {
    let storage = Storage::open(":memory:").unwrap();
    let quest = storage.create_quest(CreateQuestInput {
        icon: String::new(),
        parent_id: None, title: "Optional notes".into(), territory: String::new(),
        objective: String::new(), designation: QuestDesignation::Side,
        state: QuestState::Active, current_checkpoint: "Keep this note".into(),
    }).unwrap();
    let tab = BrowserTab { id: 1, url: "https://example.com".into(), title: "Example".into(),
        pinned: false, active: true, index: 0, group_id: None, browser_kind: None };
    let capture = storage.create_save_point(&quest.id, "  \n ", &[tab]).unwrap();
    assert_eq!(capture.checkpoint, "");
    assert_eq!(capture.resources.len(), 1);
    assert_eq!(storage.get_quest(&quest.id).unwrap().current_checkpoint, "Keep this note");
    let local = storage.create_local_save_point(&quest.id, "").unwrap();
    assert_eq!(local.checkpoint, "");
    assert_eq!(local.resources.len(), 1);
    assert_eq!(storage.get_quest(&quest.id).unwrap().materials.len(), 1);
    let noted = storage.create_local_save_point(&quest.id, "  A new note  ").unwrap();
    assert_eq!(noted.checkpoint, "A new note");
    assert_eq!(storage.get_quest(&quest.id).unwrap().current_checkpoint, "A new note");
    assert_eq!(storage.get_save_point(&capture.id).unwrap().resources.len(), 1);
    assert_eq!(storage.get_save_point(&capture.id).unwrap().checkpoint, "");
}

#[test]
fn quest_trash_restores_subtrees_and_preserves_independently_deleted_branches() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("trash.sqlite3");
    let storage = Storage::open(&path).unwrap();
    let create = |title: &str, parent: Option<String>| storage.create_quest(CreateQuestInput {
        icon: String::new(),
        title: title.into(), parent_id: parent, territory: String::new(), objective: String::new(),
        designation: QuestDesignation::Side, state: QuestState::Active, current_checkpoint: String::new(),
    }).unwrap();
    let root = create("Root", None);
    let child = create("Child", Some(root.id.clone()));
    let nested = create("Nested", Some(child.id.clone()));
    let separate = create("Already deleted", Some(root.id.clone()));
    let sibling = create("Sibling", None);
    let file = directory.path().join("notes.txt");
    std::fs::write(&file, "keep me").unwrap();
    storage.add_quest_files(&nested.id, &[file.to_string_lossy().into_owned()]).unwrap();
    let tab = BrowserTab { id: 1, url: "https://example.com".into(), title: "Keep tab".into(),
        pinned: false, active: true, index: 0, group_id: None, browser_kind: None };
    let save = storage.create_save_point(&nested.id, "Keep history", &[tab]).unwrap();
    storage.trash_quest(&separate.id).unwrap();
    storage.trash_quest(&root.id).unwrap();
    assert_eq!(storage.list_quests().unwrap().len(), 1);
    assert_eq!(storage.list_trashed_quests().unwrap().len(), 2);
    assert!(storage.get_quest(&nested.id).is_err());
    assert!(storage.add_tabs(&nested.id, &[]).is_err());
    assert!(storage.create_local_save_point(&nested.id, "").is_err());
    assert!(storage.move_quest(&sibling.id, Some(&root.id)).is_err());
    assert!(storage.create_quest(CreateQuestInput { parent_id: Some(root.id.clone()), ..serde_json::from_value(serde_json::json!({"title":"Invalid child"})).unwrap() }).is_err());
    storage.reorder_quests(None, &[sibling.id.clone()]).unwrap();
    assert!(storage.restore_quest(&child.id).is_err());
    drop(storage);
    let storage = Storage::open(&path).unwrap();
    assert_eq!(storage.list_trashed_quests().unwrap().len(), 2);
    storage.restore_quest(&root.id).unwrap();
    assert_eq!(storage.list_quests().unwrap().len(), 4);
    assert!(storage.get_quest(&separate.id).is_err());
    let restored = storage.get_quest(&nested.id).unwrap();
    assert_eq!(restored.parent_id, Some(child.id.clone()));
    assert_eq!(restored.materials.len(), 1);
    assert_eq!(restored.files.len(), 1);
    assert_eq!(restored.latest_save.unwrap().id, save.id);
    assert_eq!(storage.get_save_point(&save.id).unwrap().resources.len(), 2);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep me");
    storage.restore_quest(&separate.id).unwrap();
    assert_eq!(storage.get_quest(&separate.id).unwrap().parent_id, Some(root.id.clone()));
    storage.trash_quest(&child.id).unwrap();
    storage.trash_quest(&root.id).unwrap();
    storage.restore_quest(&child.id).unwrap();
    assert_eq!(storage.get_quest(&child.id).unwrap().parent_id, None);
    assert_eq!(storage.get_quest(&nested.id).unwrap().parent_id, Some(child.id));
    assert!(storage.trash_quest("missing").is_err());
    assert!(storage.restore_quest("missing").is_err());
}

#[test]
fn quest_icons_persist_and_survive_unrelated_edits_and_trash() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("icons.sqlite3");
    let storage = Storage::open(&database).unwrap();
    let create = |title: &str, icon: &str, parent: Option<&str>| -> CreateQuestInput {
        serde_json::from_value(serde_json::json!({"title":title,"icon":icon,"parent_id":parent})).unwrap()
    };
    let root = storage.create_quest(create(".NET", "devicon:dotnetcore", None)).unwrap();
    let child = storage.create_quest(create("Java", "devicon:java", Some(&root.id))).unwrap();
    assert_eq!(root.icon, "devicon:dotnetcore");
    let legacy: CreateQuestInput = serde_json::from_value(serde_json::json!({"title":"Legacy client"})).unwrap();
    assert_eq!(storage.create_quest(legacy).unwrap().icon, "");
    let updated = storage.update_quest(&root.id, quest_core::UpdateQuestInput { title: Some("Dotnet".into()), ..Default::default() }).unwrap();
    assert_eq!(updated.icon, "devicon:dotnetcore");
    storage.create_local_save_point(&root.id, "").unwrap();
    storage.trash_quest(&root.id).unwrap();
    assert_eq!(storage.list_trashed_quests().unwrap()[0].icon, "devicon:dotnetcore");
    drop(storage);
    let storage = Storage::open(&database).unwrap();
    storage.restore_quest(&root.id).unwrap();
    assert_eq!(storage.get_quest(&root.id).unwrap().icon, "devicon:dotnetcore");
    assert_eq!(storage.get_quest(&child.id).unwrap().icon, "devicon:java");
    assert_eq!(storage.update_quest(&child.id, quest_core::UpdateQuestInput { icon: Some("lucide:book".into()), ..Default::default() }).unwrap().icon, "lucide:book");
    assert_eq!(storage.update_quest(&child.id, quest_core::UpdateQuestInput { icon: Some(String::new()), ..Default::default() }).unwrap().icon, "");
    assert!(storage.update_quest(&root.id, quest_core::UpdateQuestInput { icon: Some("https://remote/icon.svg".into()), ..Default::default() }).is_err());
    assert!(storage.create_quest(create("Invalid icon", "<svg onload='bad'>", None)).is_err());
}
