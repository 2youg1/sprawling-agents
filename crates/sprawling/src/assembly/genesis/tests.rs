// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

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
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let norms = dir.path().join(city::CITY_FILE);
    std::fs::remove_file(&norms).unwrap();
    std::fs::create_dir_all(&norms).unwrap();

    let err = worker
        .close_city()
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
        let mut worker = RunWorker::new(dir, gateway::Custodian::in_memory(), log).unwrap();
        worker
            .handle(channels::Command::AttachEndpoint {
                name: channels::ProviderName::parse("house").unwrap(),
                base_url,
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
                admit: Vec::new(),
                tuning: channels::EndpointTuning::default(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
            })
            .unwrap();
        worker
            .handle(channels::Command::SelectModel {
                endpoint: channels::ProviderName::parse("house").unwrap(),
                model: "m-local".to_owned(),
                tag: kernel::ModelTag::Main,
                context_tokens: 32_768,
                max_output_tokens: kernel::Ceiling::new(4_096),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
            })
            .unwrap();
        // A command that fails, so the refuse level has something to
        // write in the noisy run and nothing to change in the quiet
        // one.
        let _ = worker.handle(channels::Command::Cancel {
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
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small", "m-large"], Vec::new());
    let worker = worker_with_provider(dir.path(), &base_url, "m-large").unwrap();

    // The book is a projection: throwing it away and rebuilding from
    // the ledger has to produce the same answer, or what the city
    // can call depends on a process that has already exited.
    let rebuilt = Standing::fold(&ledger_dir(dir.path())).unwrap().book;
    let live = worker
        .book
        .select(kernel::ModelTag::Main, &kernel::BuildingPolicy::default())
        .unwrap();
    let cold = rebuilt
        .select(kernel::ModelTag::Main, &kernel::BuildingPolicy::default())
        .unwrap();
    assert_eq!(live.entry, cold.entry);
    assert_eq!(live.endpoint.base_url, cold.endpoint.base_url);
    assert_eq!(cold.endpoint.models, vec!["m-large", "m-small"]);
    assert!(cold.endpoint.is_local(), "a loopback provider is local");
}

#[test]
fn a_building_created_from_the_control_surface_is_read_back_by_the_city() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let create = |name: &str| channels::Command::CreateBuilding {
        addr: Address::parse("vault").unwrap(),
        template: channels::TemplateName::parse(name).unwrap(),
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

    let report = form_city(dir.path(), Adopt::EveryFolder).unwrap();
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
            city::building_path(dir.path(), &addr).is_file(),
            "{name} has no rules of its own"
        );
        assert!(city::load(dir.path(), &addr).is_ok());
    }

    // Forming a city over a city is refused: history starts once.
    let err = form_city(dir.path(), Adopt::EveryFolder).unwrap_err();
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
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker.close_city().unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let last = verified
        .raw_lines()
        .last()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .expect("the ledger has a last line");
    assert!(last.contains("handoff_written"), "{last}");
    assert!(
        last.contains("closed by the person"),
        "the record does not say the stop was chosen: {last}"
    );
    assert!(
        last.contains("cas:b3-"),
        "the next session is not told what to read first: {last}"
    );
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
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
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
    let verified = runtime::replay::verify_ledger_dir(&ledger_dir(dir.path())).unwrap();
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
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
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
