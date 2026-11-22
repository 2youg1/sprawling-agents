// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Mutex;

use super::*;
use crate::ci::content::Key;
use crate::ci::content::{self, Identity};
use crate::ci::graph::Action;
use crate::ci::graph::Input;

#[test]
fn scoped_gates_execute_owned_changed_files_without_unrelated_debt() {
    static LENGTH: Action = Action {
        id: "length",
        operation: Operation::ScopedGate("length"),
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    static COMMENTS: Action = Action {
        id: "comments",
        operation: Operation::ScopedGate("comments"),
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    for action in [&LENGTH, &COMMENTS] {
        let root = crate::root::fixture::relocated(&format!("ci-scoped-{}", action.id));
        crate::root::fixture::write(
            &root,
            crate::budget::REGISTER,
            "[function_length]\nbudget_lines = 200\n[argument_count]\nbudget_arguments = 4\n",
        );
        crate::root::fixture::write(
            &root,
            "tools/k/src/legacy.rs",
            &"// existing debt\n".repeat(501),
        );
        crate::root::fixture::write(&root, "tools/k/src/changed.rs", "pub fn clear() {}\n");
        let view = Snapshot::capture(&root).unwrap();
        crate::root::fixture::write(
            &root,
            "tools/k/src/changed.rs",
            &"// later bad input\n".repeat(501),
        );
        let mut prepared = PreparedAction {
            action,
            key: Key::named("scoped-fixture".to_owned()),
            variables: BTreeMap::new(),
            files: Some(crate::source::Scope::new(&["tools/k/src/changed.rs".to_owned()]).unwrap()),
        };
        let log = root.join("target/scoped.log");
        assert_eq!(
            isolated_process(&view, &prepared, &log).unwrap(),
            Outcome::Green,
            "the real scoped gate must check captured clean input, excluding unrelated legacy debt"
        );
        prepared.files = Some(crate::source::Scope::new(&[]).unwrap());
        assert_eq!(
            isolated_process(&view, &prepared, &log).unwrap(),
            Outcome::Green
        );
        prepared.files =
            Some(crate::source::Scope::new(&["tools/k/src/legacy.rs".to_owned()]).unwrap());
        assert_eq!(
            isolated_process(&view, &prepared, &log).unwrap(),
            Outcome::Refused
        );
        prepared.files = None;
        assert_eq!(
            isolated_process(&view, &prepared, &log).unwrap(),
            Outcome::Refused
        );
        let later = Snapshot::capture(&root).unwrap();
        prepared.files =
            Some(crate::source::Scope::new(&["tools/k/src/changed.rs".to_owned()]).unwrap());
        assert_eq!(
            isolated_process(&later, &prepared, &log).unwrap(),
            Outcome::Refused
        );
        later.close().unwrap();
        view.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn the_production_gate_door_refuses_scope_on_an_unsupported_gate() {
    static HEADER: Action = Action {
        id: "header",
        operation: Operation::ScopedGate("header"),
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    let root = tempfile::tempdir().unwrap();
    let prepared = PreparedAction {
        action: &HEADER,
        key: Key::named("unsupported-scope".to_owned()),
        variables: BTreeMap::new(),
        files: Some(crate::source::Scope::new(&[]).unwrap()),
    };
    assert_eq!(
        process(root.path(), &prepared, &root.path().join("unused.log")).unwrap(),
        Outcome::Broken
    );
    root.close().unwrap();
}

#[cfg(unix)]
#[test]
fn captured_file_scope_keeps_unix_punctuation_through_real_file_gate() {
    static LENGTH: Action = Action {
        id: "length",
        operation: Operation::ScopedGate("length"),
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    let root = crate::root::fixture::relocated("ci-scoped-punctuation");
    crate::root::fixture::write(
        &root,
        crate::budget::REGISTER,
        "[function_length]\nbudget_lines = 200\n[argument_count]\nbudget_arguments = 4\n",
    );
    let path = r"tools/k/src/changed:and\local.rs";
    crate::root::fixture::write(&root, path, &"// fixture\n".repeat(501));
    let view = Snapshot::capture(&root).unwrap();
    assert!(view.files().contains_key(path));
    let prepared = PreparedAction {
        action: &LENGTH,
        key: Key::named("punctuation".to_owned()),
        variables: BTreeMap::new(),
        files: Some(crate::source::Scope::from_capture(&BTreeSet::from([
            path.to_owned()
        ]))),
    };
    assert_eq!(
        isolated_process(&view, &prepared, &root.join("target")).unwrap(),
        Outcome::Refused
    );
    view.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_later_good_tree_cannot_publish_green_for_a_captured_bad_tree() {
    let original = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(original.path().join("src")).unwrap();
    let file = original.path().join("src/lib.rs");
    let bad = "fn captured() {}\n";
    std::fs::write(&file, bad).unwrap();
    let view = Snapshot::capture(original.path()).unwrap();
    static HEADER: Action = Action {
        id: "header",
        operation: Operation::Gate("header"),
        inputs: &[Input::only("", &["rs"])],
        needs: &[],
        reuse: Reuse::SuccessOnly,
        required: true,
    };
    let environment = Environment {
        os: "test".to_owned(),
        arch: "test".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    let key = content::key_in_snapshot(
        &view,
        &HEADER,
        &Identity {
            base: &[],
            dependencies: &BTreeMap::new(),
            environment: &environment,
            variables: &BTreeMap::new(),
            files: None,
        },
    )
    .unwrap();
    let good = crate::header::notice(crate::header::Leader::Rust)
        .into_iter()
        .chain(["".to_owned(), "fn captured() {}".to_owned()])
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file, good).unwrap();
    let good_view = Snapshot::capture(original.path()).unwrap();
    let good_key = content::key_in_snapshot(
        &good_view,
        &HEADER,
        &Identity {
            base: &[],
            dependencies: &BTreeMap::new(),
            environment: &environment,
            variables: &BTreeMap::new(),
            files: None,
        },
    )
    .unwrap();
    let store = Store::local(original.path().join("target/verdicts"));
    assert_ne!(
        key, good_key,
        "A and B must have distinct content identities"
    );
    let outcome = execute_one(
        &Execution {
            view: &view,
            work: original.path(),
            store: &store,
            environment: &environment,
        },
        &PreparedAction {
            action: &HEADER,
            key: key.clone(),
            variables: BTreeMap::new(),
            files: None,
        },
    );
    std::fs::write(&file, bad).unwrap();
    let cached = store
        .restore("header", &key, &environment, Reuse::SuccessOnly)
        .unwrap();
    let good_outcome = execute_one(
        &Execution {
            view: &good_view,
            work: original.path(),
            store: &store,
            environment: &environment,
        },
        &PreparedAction {
            action: &HEADER,
            key: good_key.clone(),
            variables: BTreeMap::new(),
            files: None,
        },
    );
    let good_cached = store
        .restore("header", &good_key, &environment, Reuse::SuccessOnly)
        .unwrap();
    let restored_view = Snapshot::capture(original.path()).unwrap();
    let restored_key = content::key_in_snapshot(
        &restored_view,
        &HEADER,
        &Identity {
            base: &[],
            dependencies: &BTreeMap::new(),
            environment: &environment,
            variables: &BTreeMap::new(),
            files: None,
        },
    )
    .unwrap();
    assert_eq!(
        restored_key, key,
        "a fresh capture of restored A must recover A's identity"
    );
    let restored_outcome = execute_one(
        &Execution {
            view: &restored_view,
            work: original.path(),
            store: &store,
            environment: &environment,
        },
        &PreparedAction {
            action: &HEADER,
            key: restored_key,
            variables: BTreeMap::new(),
            files: None,
        },
    );
    assert_eq!(restored_outcome, Outcome::Refused);
    restored_view.close().unwrap();
    good_view.close().unwrap();
    view.close().unwrap();
    assert_eq!(
        (outcome, cached.is_some()),
        (Outcome::Refused, false),
        "the executor must check captured A and must not save B's green under A's key"
    );
    assert_eq!(good_outcome, Outcome::Green);
    assert!(
        good_cached.is_some(),
        "the controlled fixture identity permits real green reuse"
    );
    assert!(
        store
            .restore("header", &key, &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_none(),
        "saving B must not populate A's key"
    );
}

#[test]
fn failed_loaded_image_probe_still_executes_the_real_gate() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("bad.rs"), "fn missing_header() {}\n").unwrap();
    let view = Snapshot::capture(root.path()).unwrap();
    let environment = Environment::from_probe(Err(XtaskError::Io {
        path: "fixture loaded-image probe".to_owned(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "probe unavailable"),
    }));
    assert_eq!(environment.reuse(Reuse::SuccessOnly), Reuse::Never);
    static HEADER: Action = Action {
        id: "header",
        operation: Operation::Gate("header"),
        inputs: &[],
        needs: &[],
        reuse: Reuse::SuccessOnly,
        required: true,
    };
    let store = Store::local(root.path().join("target/verdicts"));
    let outcome = execute_one(
        &Execution {
            view: &view,
            work: root.path(),
            store: &store,
            environment: &environment,
        },
        &PreparedAction {
            action: &HEADER,
            key: Key::named("fixture".to_owned()),
            variables: BTreeMap::new(),
            files: None,
        },
    );
    assert_eq!(
        outcome,
        Outcome::Refused,
        "probe failure must not prevent the real bad-header verdict"
    );
    assert!(!root.path().join("target/verdicts").exists());
    view.close().unwrap();
    root.close().unwrap();
}

#[test]
fn a_mutating_non_reusable_process_cannot_rewrite_pure_inputs() {
    let original = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(original.path().join("src")).unwrap();
    std::fs::write(original.path().join("src/lib.rs"), "captured A").unwrap();
    let view = Snapshot::capture(original.path()).unwrap();
    static MUTATION: Action = Action {
        id: "mutating-tool",
        operation: Operation::Process {
            program: "bun",
            args: &["-e", "await Bun.write('src/lib.rs', 'tool B')"],
        },
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    let outcome = isolated_process(
        &view,
        &PreparedAction {
            action: &MUTATION,
            key: Key::named("mutation-fixture".to_owned()),
            variables: BTreeMap::new(),
            files: None,
        },
        &original.path().join("target"),
    )
    .unwrap();
    assert_eq!(outcome, Outcome::Green);
    assert_eq!(
        std::fs::read_to_string(view.root().join("src/lib.rs")).unwrap(),
        "captured A"
    );
    assert_eq!(crate::header::check(view.root()).unwrap().len(), 1);
    view.close().unwrap();
}

fn prepared(id: &'static str, needs: &'static [&'static str]) -> PreparedAction {
    PreparedAction {
        action: Box::leak(Box::new(Action {
            id,
            operation: Operation::Just(id),
            inputs: &[],
            needs,
            reuse: Reuse::Never,
            required: true,
        })),
        key: Key::named(id.to_owned()),
        variables: BTreeMap::new(),
        files: None,
    }
}

#[test]
fn a_failure_blocks_dependents_but_not_independent_work() {
    let graph = vec![
        prepared("base", &[]),
        prepared("dependent", &["base"]),
        prepared("independent", &[]),
    ];
    let ran = Mutex::new(Vec::new());
    let outcomes = schedule(
        &graph,
        &BTreeSet::from([
            "base".to_owned(),
            "dependent".to_owned(),
            "independent".to_owned(),
        ]),
        |action| {
            ran.lock().unwrap().push(action.action.id);
            if action.action.id == "base" {
                Outcome::Refused
            } else {
                Outcome::Green
            }
        },
    );
    assert_eq!(outcomes.get("base"), Some(&Outcome::Refused));
    assert_eq!(outcomes.get("dependent"), Some(&Outcome::Blocked));
    assert_eq!(outcomes.get("independent"), Some(&Outcome::Green));
    assert!(!ran.lock().unwrap().contains(&"dependent"));
    assert!(ran.lock().unwrap().contains(&"independent"));
}

#[test]
fn an_empty_affected_scope_is_green_without_work() {
    let outcomes = schedule(&[], &BTreeSet::new(), |_| Outcome::Broken);
    assert_eq!(report(&outcomes), ExitCode::SUCCESS);
}

#[test]
fn refused_prerequisites_block_reverse_order_chains() {
    let graph = vec![
        prepared("leaf", &["middle"]),
        prepared("middle", &["base"]),
        prepared("base", &[]),
    ];
    let selected = BTreeSet::from(["leaf", "middle", "base"].map(str::to_owned));
    let outcomes = schedule(&graph, &selected, |action| {
        assert_eq!(action.action.id, "base");
        Outcome::Refused
    });
    assert_eq!(outcomes.get("leaf"), Some(&Outcome::Blocked));
    assert_eq!(report(&outcomes), ExitCode::FAILURE);
}

#[test]
fn workflow_bash_never_resolves_to_the_wsl_name_on_windows() {
    let resolved = executable("bash").unwrap();
    #[cfg(windows)]
    {
        assert!(resolved.is_absolute() && resolved.is_file());
        assert_eq!(
            resolved.file_name().and_then(|name| name.to_str()),
            Some("bash.exe")
        );
    }
    #[cfg(not(windows))]
    assert_eq!(resolved, PathBuf::from("bash"));
}
