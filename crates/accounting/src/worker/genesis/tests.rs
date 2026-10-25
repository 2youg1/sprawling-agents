// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::worker::fixture::*;
use crate::worker::*;

/// The city's norms are what a closing city tells the next session
/// to read, so a close that cannot read them has nothing to say.
///
/// `unwrap_or_default` made the must-read locator the hash of zero
/// bytes: the handoff still claimed the next session must read the
/// city's norms, and pointed at nothing.
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
    // The one test that keeps a log a diagnostic rather than data.
    // Two cities, the same work, one with every level on and one
    // with logging off: if a log line could reach a decision, these
    // two ledgers would differ somewhere.
    // One provider for both cities: a second listener would take a
    // second ephemeral port, and the ledger records the URL.
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let run_city = |log: runtime::diagnostics::Diagnostics| {
        let held = tempfile::tempdir().unwrap();
        // Both cities carry the same name, because the genesis record
        // now states it: two differently-named cities would differ in
        // their first line for a reason that has nothing to do with
        // logging, which is what this test is about.
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
        // A command that fails, so the refuse level has something to
        // write in the noisy run and nothing to change in the quiet
        // one.
        let _ = worker.handle(wire::Command::Cancel {
            run: RunId::CITY,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"cancel"),
        });
        let lines = runtime::replay::verify_ledger_dir(&report.ledger_dir)
            .unwrap()
            .raw_lines()
            .to_vec();
        // The volatile fields are the ones a second run must be
        // allowed to differ in: identifiers derived from time, and
        // the times themselves. What must match is everything else.
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
    // And the noisy run really was noisy: an invariance that held
    // because nothing was ever written would prove nothing.
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

    // The book is a projection: throwing it away and rebuilding from
    // the ledger has to produce the same answer, or what the city
    // can call depends on a process that has already exited.
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
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
    };

    worker.handle(create("confidential")).unwrap();

    // The building the city reads is the building the command made.
    let rules = city::load(dir.path(), &Address::parse("vault").unwrap()).unwrap();
    assert!(rules.policy().confidential);
    assert_eq!(rules.model_pool(), city::ModelPool::LocalOnly);

    // And the history says it happened, in the address's own words.
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("building_created"));
    assert!(history.contains("\"template\":\"confidential\""));

    // A second creation does not quietly relax the rules of a
    // building that is already working under them.
    let err = worker.handle(create("minimal")).unwrap_err();
    assert!(err.recovery().contains("already has rules"));
    assert!(
        city::load(dir.path(), &Address::parse("vault").unwrap())
            .unwrap()
            .policy()
            .confidential
    );
}

/// The `building_created` line the history holds for `addr`.
fn created_line(ledger_dir: &std::path::Path, addr: &str) -> serde_json::Value {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .find(|line| line["kind"] == "building_created" && line["data"]["addr"] == addr)
        .unwrap_or_else(|| panic!("no building_created line for {addr}"))
}

/// A folder a person put under the city and then raised from a page is
/// found, not built: the history says it was adopted, its files are as
/// they were, and it has the base checkpoint `sprawling adopt` takes.
/// Before, the page laid a building over it and recorded that the city
/// had made it.
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

/// A folder whose base checkpoint is refused, because the staged scan
/// finds a credential in it, is left waiting to be adopted rather than
/// half adopted: it has no rules and the history no line, and once the
/// file is gone the same press adopts it. Before, the rules stayed, the
/// line never came, and the next press was refused as a building that
/// already had rules.
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

/// Rules a person wrote on the page are the building's rules, and rules
/// the city cannot read leave nothing behind: no directory, no file and
/// no line, so a refused raise can be sent again as it stands.
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
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, addr.as_bytes()),
    };
    // A confidential building's rules, as a person would paste them.
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("model").unwrap(),
            template: wire::TemplateName::parse("confidential").unwrap(),
            rules: None,
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

/// A person who already has a workspace could only be told to make a
/// new one: `init` formed a city and said nothing about what was
/// there, and `adopt` needed the folder to be inside a city already.
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

    // The work itself is untouched, byte for byte.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("parser").join("src").join("lib.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "# my work\n"
    );
    // And each folder is now a building with its own rules, in the
    // reserved subtree where its own runs cannot reach them.
    for name in ["parser", "notes"] {
        let addr = Address::parse(name).unwrap();
        assert!(
            city::rules_path(dir.path(), &addr).is_file(),
            "{name} has no rules of its own"
        );
        assert!(city::load(dir.path(), &addr).is_ok());
    }

    // Forming a city over a city is refused: history starts once.
    let err = crate::worker::genesis::form(dir.path(), Adopt::EveryFolder, hands()).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

/// A stop somebody chose and a stop that was a crash left the same
/// silence in the record: `sprawling resume` recovered both, and
/// nothing said which had happened.
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

#[test]
fn init_writes_genesis_and_refuses_a_second_birth() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    assert_eq!(report.genesis.seq(), kernel::Seq::FIRST);
    assert_eq!(report.genesis.kind(), EventKind::CityInitialized);
    // The chain verifies offline (A2 face).
    runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    // Genesis happens once.
    let err = init_city(dir.path()).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

#[test]
fn the_startup_scan_closes_dangling_calls_once_and_reports_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let raised = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    // A process death mid-call: tool_called with no tool_result.
    let run = RunId::from_bytes([7u8; 16]);
    let mut data = serde_json::Map::new();
    data.insert(
        "id".to_owned(),
        serde_json::Value::String("tu_9".to_owned()),
    );
    data.insert(
        "name".to_owned(),
        serde_json::Value::String("edit".to_owned()),
    );
    data.insert(
        "args".to_owned(),
        serde_json::Value::Object(serde_json::Map::new()),
    );
    worker
        .record_for(
            run,
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::ToolCalled,
                data: Payload::new(data).unwrap(),
            },
        )
        .unwrap();
    let report = worker.startup_scan().unwrap();
    assert_eq!(report.closed_calls, 1, "the dangling call is closed");
    // The account now shows an outcome; a second scan repairs nothing.
    let again = worker.startup_scan().unwrap();
    assert_eq!(again.closed_calls, 0, "the repair is idempotent");
    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let closed = verified.lines().iter().any(|line| match line {
        runtime::replay::VerifiedLine::Known { record, .. } => {
            record.kind() == EventKind::ToolResult
                && serde_json::to_string(record.data())
                    .unwrap()
                    .contains("E_TOOL_OUTCOME_UNKNOWN")
        }
        _ => false,
    });
    assert!(closed, "the closing result states the unknown outcome");
}

/// `crates/runtime/spec/Tools.lean` D116: recovery never admits a call
/// it finds in the history again, so a call an older build recorded
/// under the argument names it took then derives no key and runs
/// nothing here; it is closed as an unknown outcome and the file it
/// named keeps its bytes.
#[test]
fn an_old_shaped_dangling_call_is_closed_and_never_run() {
    let dir = tempfile::tempdir().unwrap();
    let raised = init_city(dir.path()).unwrap();
    let notes = dir.path().join("lab/room1/notes.md");
    std::fs::create_dir_all(notes.parent().unwrap()).unwrap();
    std::fs::write(&notes, "draft\n").unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let older = serde_json::json!({
        "id": "tu_old",
        "name": "edit",
        "args": { "path": "lab/room1/notes.md", "old": "draft", "new": "final" },
        "subject": "lab/room1/notes.md",
    });
    worker
        .record_for(
            RunId::from_bytes([8u8; 16]),
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::ToolCalled,
                data: Payload::of(&older).unwrap(),
            },
        )
        .unwrap();
    let report = worker.startup_scan().unwrap();
    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let closing = verified.lines().iter().rev().find_map(|line| match line {
        runtime::replay::VerifiedLine::Known { record, .. }
            if record.kind() == EventKind::ToolResult =>
        {
            Some(serde_json::to_value(record.data()).unwrap())
        }
        _ => None,
    });
    assert_eq!(
        (
            report.closed_calls,
            closing.map(|data| (data["tool_use_id"].clone(), data["error"]["code"].clone())),
            std::fs::read_to_string(&notes).unwrap(),
        ),
        (
            1,
            Some((
                serde_json::json!("tu_old"),
                serde_json::json!("E_TOOL_OUTCOME_UNKNOWN")
            )),
            "draft\n".to_owned(),
        )
    );
}

/// A city is raised with its approvals delegated to the clerk, and the
/// delegation is a line of history rather than a default value: the
/// person can change who answers, and a change needs something to
/// change.
#[test]
fn a_new_city_delegates_its_approvals_to_the_clerk_on_the_record() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    assert_eq!(
        worker.governance.autonomy,
        kernel::Autonomy::Delegate(
            kernel::ResidentId::new(kernel::consts_policy::HALL_CLERK).unwrap()
        ),
        "the clerk answers from the moment the city exists, folded back from the ledger"
    );
}

/// A ledger path that exists but will not list is not "no city".
#[test]
fn a_ledger_that_cannot_be_listed_is_an_error_not_an_empty_city() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = kernel::layout::CityLayout::new(dir.path()).ledger();
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(&ledger, b"not a directory").unwrap();
    assert!(has_history(dir.path()).is_err());
}
