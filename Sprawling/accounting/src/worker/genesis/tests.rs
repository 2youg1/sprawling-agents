// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_city_whose_norms_cannot_be_read_refuses_to_say_it_wrote_them_down() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let norms = dir.path().join(city::CITY_FILE);
    std::fs::remove_file(&norms).unwrap();
    std::fs::create_dir_all(&norms).unwrap();

    let err = worker
        .close_city(&by_hand())
        .expect_err("a close that cannot name the norms is not an orderly close");
    assert!(
        err.to_string().contains(city::CITY_FILE),
        "the refusal has to name the file a person must fix: {err}"
    );
}

#[test]
fn deleting_every_log_line_leaves_the_history_byte_identical() {
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let run_city = |log: runtime::diagnostics::Diagnostics| {
        let held = tempfile::tempdir().unwrap();

        let dir = held.path().join("kiln");
        std::fs::create_dir_all(&dir).unwrap();
        let dir = dir.as_path();
        let report = init_city(dir).unwrap();
        let base_url = base_url.clone();
        let mut worker = RunWorker::new(dir, log, hands()).unwrap();
        worker
            .handle(wire::Command::AttachEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url,
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
                admit: Vec::new(),
                named: Vec::new(),
                tuning: wire::EndpointTuning::default(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
            })
            .unwrap();
        worker
            .handle(wire::Command::SelectModel {
                endpoint: wire::ProviderName::parse("house").unwrap(),
                model: "m-local".to_owned(),
                tag: kernel::ModelTag::Main,
                context_tokens: kernel::Window::new(32_768),
                max_output_tokens: kernel::Ceiling::new(4_096),
                input: None,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
            })
            .unwrap();

        let _ = worker.handle(wire::Command::Cancel {
            run: RunId::CITY,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"cancel"),
        });
        let lines = runtime::replay::verify_ledger_dir(&report.ledger_dir)
            .unwrap()
            .raw_lines()
            .to_vec();

        lines
            .iter()
            .map(|line| {
                let mut record: serde_json::Value =
                    serde_json::from_slice(line).unwrap_or(serde_json::Value::Null);
                for volatile in ["t", "seq", "prev", "hash", "run", "id"] {
                    if let Some(map) = record.as_object_mut() {
                        map.remove(volatile);
                    }
                }
                record.to_string()
            })
            .collect::<Vec<String>>()
    };

    let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&written);
    let noisy = runtime::diagnostics::Diagnostics::new(
        runtime::diagnostics::Level::Wire,
        Box::new(move |entry: runtime::diagnostics::Entry<'_>| {
            sink.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(runtime::diagnostics::render(entry));
        }),
    );
    let with_logs = run_city(noisy);
    let without_logs = run_city(runtime::diagnostics::Diagnostics::off());
    assert_eq!(with_logs, without_logs);
    assert!(!with_logs.is_empty(), "the scenario has to do something");

    assert!(
        !written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty(),
        "the run with logging on wrote no lines"
    );
}

#[test]
fn a_registration_survives_the_process_that_made_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small", "m-large"], Vec::new());
    let worker = worker_with_provider(dir.path(), &base_url, "m-large").unwrap();

    let rebuilt = Standing::fold(&report.ledger_dir).unwrap().book;
    let live = worker
        .credentials
        .book
        .select(kernel::ModelTag::Main, &kernel::BuildingPolicy::default())
        .unwrap();
    let cold = rebuilt
        .select(kernel::ModelTag::Main, &kernel::BuildingPolicy::default())
        .unwrap();
    assert_eq!(live.entry, cold.entry);
    assert_eq!(live.endpoint.base_url, cold.endpoint.base_url);
    assert_eq!(
        cold.endpoint
            .models
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<&str>>(),
        vec!["m-large", "m-small"]
    );
    assert!(cold.endpoint.is_local(), "a loopback provider is local");
}

#[test]
fn a_building_created_from_the_control_surface_is_read_back_by_the_city() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let create = |name: &str| wire::Command::CreateBuilding {
        addr: Address::parse("vault").unwrap(),
        template: wire::TemplateName::parse(name).unwrap(),
        rules: None,
        source: None,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
    };

    worker.handle(create("confidential")).unwrap();

    let rules = city::load(dir.path(), &Address::parse("vault").unwrap()).unwrap();
    assert!(rules.policy().confidential);
    assert_eq!(rules.model_pool(), city::ModelPool::LocalOnly);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("building_created"));
    assert!(history.contains("\"template\":\"confidential\""));

    let err = worker.handle(create("minimal")).unwrap_err();
    assert!(err.recovery().contains("already has rules"));
    assert!(
        city::load(dir.path(), &Address::parse("vault").unwrap())
            .unwrap()
            .policy()
            .confidential
    );
}

fn created_line(ledger_dir: &std::path::Path, addr: &str) -> serde_json::Value {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .find(|line| line["kind"] == "building_created" && line["data"]["addr"] == addr)
        .unwrap_or_else(|| panic!("no building_created line for {addr}"))
}

#[test]
fn a_folder_raised_from_the_control_surface_is_adopted_and_its_files_kept() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("Roadmap.md"), "the owner's plan\n").unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();

    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("notes").unwrap(),
            template: wire::TemplateName::parse("confidential").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"adopt"),
        })
        .unwrap();

    let line = created_line(&report.ledger_dir, "notes");
    assert_eq!(
        (
            line["data"]["adopted"].clone(),
            line["data"]["template"].clone()
        ),
        (serde_json::json!(true), serde_json::json!("confidential"))
    );
    assert_eq!(
        std::fs::read_to_string(notes.join("Roadmap.md")).unwrap(),
        "the owner's plan\n"
    );
    assert!(
        city::load(dir.path(), &Address::parse("notes").unwrap())
            .unwrap()
            .policy()
            .confidential
    );
}

#[test]
fn an_absolute_host_folder_is_copied_into_a_building_without_moving_the_source() {
    let city_dir = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let report = init_city(city_dir.path()).unwrap();
    std::fs::create_dir_all(source.path().join("src")).unwrap();
    std::fs::write(source.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut worker = RunWorker::new(
        city_dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();

    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("imported").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: Some(source.path().to_string_lossy().into_owned()),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"import"),
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(city_dir.path().join("imported/src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(source.path().join("src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
    let line = created_line(&report.ledger_dir, "imported");
    assert_eq!(line["data"]["adopted"], serde_json::json!(true));
    assert!(line["data"].get("source").is_none());
}

#[test]
fn a_workspace_import_refuses_a_relative_source_or_an_occupied_target() {
    let city_dir = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    init_city(city_dir.path()).unwrap();
    std::fs::write(source.path().join("KEEP.md"), "source\n").unwrap();
    let mut worker = RunWorker::new(
        city_dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let import = |addr: &str, source_path: String, tag: &[u8]| wire::Command::CreateBuilding {
        addr: Address::parse(addr).unwrap(),
        template: wire::TemplateName::parse("minimal").unwrap(),
        rules: None,
        source: Some(source_path),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, tag),
    };

    let relative = worker
        .handle(import("relative", "elsewhere".to_owned(), b"relative"))
        .unwrap_err();
    assert_eq!(relative.code(), &kernel::AxCode::InvalidArgs);
    assert!(!city_dir.path().join("relative").exists());

    std::fs::create_dir(city_dir.path().join("occupied")).unwrap();
    std::fs::write(city_dir.path().join("occupied/KEEP.md"), "target\n").unwrap();
    let occupied = worker
        .handle(import(
            "occupied",
            source.path().to_string_lossy().into_owned(),
            b"occupied",
        ))
        .unwrap_err();
    assert_eq!(occupied.code(), &kernel::AxCode::InvalidArgs);
    assert_eq!(
        std::fs::read_to_string(city_dir.path().join("occupied/KEEP.md")).unwrap(),
        "target\n"
    );
}

#[test]
fn a_refused_adoption_leaves_the_folder_waiting_to_be_adopted_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
    std::fs::write(notes.join("key.env"), format!("KEY={token}")).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let addr = Address::parse("notes").unwrap();
    let raise = |press: &[u8]| wire::Command::CreateBuilding {
        addr: addr.clone(),
        template: wire::TemplateName::parse("minimal").unwrap(),
        rules: None,
        source: None,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, press),
    };

    let refused = worker.handle(raise(b"first")).unwrap_err();
    let waiting = city::has_rules(dir.path(), &city::Building::of(&addr).unwrap()).unwrap();
    std::fs::remove_file(notes.join("key.env")).unwrap();
    worker.handle(raise(b"second")).unwrap();

    assert_eq!(
        (
            refused.code(),
            waiting,
            created_line(&report.ledger_dir, "notes")["data"]["adopted"].clone()
        ),
        (
            &kernel::AxCode::SecretEgress,
            false,
            serde_json::json!(true)
        ),
        "{refused}"
    );
}

#[test]
fn written_rules_govern_the_new_building_and_refused_ones_lay_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let raise = |addr: &str, rules: Option<String>| wire::Command::CreateBuilding {
        addr: Address::parse(addr).unwrap(),
        template: wire::TemplateName::parse("minimal").unwrap(),
        rules,
        source: None,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, addr.as_bytes()),
    };

    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("model").unwrap(),
            template: wire::TemplateName::parse("confidential").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"model"),
        })
        .unwrap();
    let written = std::fs::read_to_string(city::rules_path(
        dir.path(),
        &Address::parse("model").unwrap(),
    ))
    .unwrap();

    worker.handle(raise("vault", Some(written))).unwrap();
    assert!(
        city::load(dir.path(), &Address::parse("vault").unwrap())
            .unwrap()
            .policy()
            .confidential,
        "the written rules, not the minimal template's, govern the building"
    );

    let refused = worker
        .handle(raise("broken", Some("confidential = [not toml".to_owned())))
        .unwrap_err();
    assert_eq!(refused.code(), &kernel::AxCode::ConfigInvalid, "{refused}");
    assert!(!dir.path().join("broken").exists());
    let history = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    assert!(
        !history
            .raw_lines()
            .iter()
            .any(|line| String::from_utf8_lossy(line).contains("\"broken\"")),
        "a refused raise writes no line"
    );
}

#[test]
fn a_folder_somebody_already_works_in_becomes_a_city_around_that_work() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("parser").join("src")).unwrap();
    std::fs::write(
        dir.path().join("parser").join("src").join("lib.rs"),
        "fn main() {}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    std::fs::write(dir.path().join("README.md"), "# my work\n").unwrap();

    let report = crate::worker::genesis::form(dir.path(), Adopt::EveryFolder, hands()).unwrap();
    let city::Standing::Work { adoptable, loose } = &report.standing else {
        panic!("a folder with work in it is not an empty one");
    };
    assert_eq!(adoptable.len(), 2);
    assert_eq!(*loose, 1, "the README is counted and left alone");
    assert_eq!(report.adopted.len(), 2);

    assert_eq!(
        std::fs::read_to_string(dir.path().join("parser").join("src").join("lib.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "# my work\n"
    );

    for name in ["parser", "notes"] {
        let addr = Address::parse(name).unwrap();
        assert!(
            city::rules_path(dir.path(), &addr).is_file(),
            "{name} has no rules of its own"
        );
        assert!(city::load(dir.path(), &addr).is_ok());
    }

    let err = crate::worker::genesis::form(dir.path(), Adopt::EveryFolder, hands()).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

#[test]
fn a_city_that_is_closed_says_so_before_it_stops() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    worker.close_city(&by_hand()).unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let last = verified
        .raw_lines()
        .last()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .expect("the ledger has a last line");
    for said in [
        "handoff_written",
        "the city was closed with /quit in its terminal",
        "every run under way landed before this line",
        "cas:b3-",
    ] {
        assert!(last.contains(said), "no {said:?} in the close: {last}");
    }
}
