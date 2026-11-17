// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use super::super::graph::Reuse;
use super::*;

const ACTION_INPUTS: &[Input] = &[Input::only("src", &["rs"])];

fn fixture(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("xtask-ci-content-{label}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn action() -> Action {
    Action {
        id: "unit",
        operation: Operation::Gate("header"),
        inputs: ACTION_INPUTS,
        needs: &[],
        reuse: Reuse::SuccessOnly,
        required: true,
    }
}

fn environment() -> Environment {
    Environment {
        os: "test-os".to_owned(),
        arch: "test-arch".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    }
}

fn identity<'a>(
    dependencies: &'a BTreeMap<String, Key>,
    environment: &'a Environment,
) -> Identity<'a> {
    Identity {
        base: &[],
        dependencies,
        environment,
        variables: empty_variables(),
        files: None,
    }
}

fn empty_variables() -> &'static BTreeMap<String, String> {
    static VARIABLES: BTreeMap<String, String> = BTreeMap::new();
    &VARIABLES
}

#[test]
fn equal_bytes_have_the_same_key_and_changed_bytes_move_it() {
    let root = fixture("bytes");
    std::fs::write(root.join("src/lib.rs"), "fn first() {}\n").unwrap();
    let before = key(
        &root,
        &action(),
        &identity(&BTreeMap::new(), &environment()),
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "fn second() {}\n").unwrap();
    let after = key(
        &root,
        &action(),
        &identity(&BTreeMap::new(), &environment()),
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert_ne!(before, after);
}

#[test]
fn paths_dependencies_and_environment_are_part_of_the_key() {
    let root = fixture("identity");
    std::fs::write(root.join("src/lib.rs"), "fn same() {}\n").unwrap();
    let base = key(
        &root,
        &action(),
        &identity(&BTreeMap::new(), &environment()),
    )
    .unwrap();
    let other_root = fixture("identity-other");
    std::fs::write(other_root.join("src/other.rs"), "fn same() {}\n").unwrap();
    let other_path = key(
        &other_root,
        &action(),
        &identity(&BTreeMap::new(), &environment()),
    )
    .unwrap();
    let with_dependency = key(
        &root,
        &action(),
        &identity(
            &BTreeMap::from([("base".to_owned(), Key::named("digest".to_owned()))]),
            &environment(),
        ),
    )
    .unwrap();
    let other_environment = key(
        &root,
        &action(),
        &identity(
            &BTreeMap::new(),
            &Environment {
                os: "other".to_owned(),
                arch: "test-arch".to_owned(),
                verifier: Some("test-verifier".to_owned()),
            },
        ),
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(other_root).unwrap();
    assert_ne!(base, other_path);
    assert_ne!(base, with_dependency);
    assert_ne!(base, other_environment);
}

#[test]
fn a_missing_declared_input_is_refused() {
    let root = fixture("missing");
    let verdict = key(
        &root,
        &action(),
        &identity(&BTreeMap::new(), &environment()),
    );
    std::fs::remove_dir_all(root).unwrap();
    assert!(verdict.is_err());
}

#[test]
fn a_captured_bad_header_cannot_borrow_a_later_good_header() {
    let root = fixture("header-view");
    let file = root.join("src/lib.rs");
    let bad = "fn captured() {}\n";
    std::fs::write(&file, bad).unwrap();
    let captured = Snapshot::capture(&root).unwrap();
    let good = crate::header::notice(crate::header::Leader::Rust)
        .into_iter()
        .chain(["".to_owned(), "fn captured() {}".to_owned()])
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file, good).unwrap();
    let findings = crate::header::check(captured.root()).unwrap();
    std::fs::write(&file, bad).unwrap();
    captured.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        findings.len(),
        1,
        "the real header gate must read captured A, even after the original became B then A"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_replaced_executable_path_does_not_replace_the_loaded_identity() {
    const CHILD: &str = "XTASK_CI_IMAGE_REPLACEMENT_TEST";
    if std::env::var_os(CHILD).is_some() {
        let before = Environment::current();
        assert!(before.verifier.is_some());
        let executable = std::env::current_exe().unwrap();
        std::fs::rename(&executable, executable.with_extension("running")).unwrap();
        std::fs::write(&executable, b"replacement path bytes").unwrap();
        std::fs::remove_file(executable.with_extension("running")).unwrap();
        assert_eq!(Environment::current(), before);
        assert_ne!(
            before.verifier.unwrap(),
            blake3::hash(&std::fs::read(executable).unwrap())
                .to_hex()
                .to_string()
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join("verifier");
    std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    let output = std::process::Command::new(executable)
        .args([
            "--exact",
            "ci::content::tests::a_replaced_executable_path_does_not_replace_the_loaded_identity",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    dir.close().unwrap();
}

#[test]
fn unavailable_loaded_identity_disables_reuse() {
    let mut environment = environment();
    environment.verifier = None;
    assert_eq!(environment.reuse(Reuse::SuccessOnly), Reuse::Never);
    assert_eq!(environment.reuse(Reuse::Never), Reuse::Never);
}

#[test]
fn file_scope_key_distinguishes_inventory_empty_and_normalized_paths() {
    let root = fixture("scope-key");
    std::fs::write(root.join("src/lib.rs"), "fn first() {}\n").unwrap();
    std::fs::write(root.join("src/other.rs"), "fn other() {}\n").unwrap();
    let snapshot = Snapshot::capture(&root).unwrap();
    let environment = environment();
    let dependencies = BTreeMap::new();
    let variables = BTreeMap::new();
    let mut action = action();
    action.operation = Operation::ScopedGate("comments");
    action.reuse = Reuse::Never;
    let derive = |files| {
        key_in_snapshot(
            &snapshot,
            &action,
            &Identity {
                base: &[],
                dependencies: &dependencies,
                environment: &environment,
                variables: &variables,
                files,
            },
        )
        .unwrap()
    };
    let empty = crate::source::Scope::new(&[]).unwrap();
    let first = crate::source::Scope::new(&["src/lib.rs".to_owned()]).unwrap();
    let equivalent = crate::source::Scope::new(&[
        "./src/lib.rs".to_owned(),
        "src\\lib.rs".to_owned(),
        "src/lib.rs".to_owned(),
    ])
    .unwrap();
    let other = crate::source::Scope::new(&["src/other.rs".to_owned()]).unwrap();
    assert_ne!(derive(None), derive(Some(&empty)));
    assert_ne!(derive(Some(&empty)), derive(Some(&first)));
    assert_ne!(derive(Some(&first)), derive(Some(&other)));
    assert_eq!(derive(Some(&first)), derive(Some(&equivalent)));
    snapshot.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn loaded_verifier_and_explicit_process_inputs_change_the_key() {
    let root = fixture("explicit-identity");
    std::fs::write(root.join("src/lib.rs"), "fn same() {}\n").unwrap();
    let environment = environment();
    let dependencies = BTreeMap::new();
    let variables = BTreeMap::from([("SPRAWLING_CI_SOURCE_SHA".to_owned(), "first".to_owned())]);
    let first = key(
        &root,
        &action(),
        &Identity {
            base: &[],
            dependencies: &dependencies,
            environment: &environment,
            variables: &variables,
            files: None,
        },
    )
    .unwrap();
    let mut other = environment.clone();
    other.verifier = Some("other-loaded-verifier".to_owned());
    let changed_verifier = key(
        &root,
        &action(),
        &Identity {
            base: &[],
            dependencies: &dependencies,
            environment: &other,
            variables: &variables,
            files: None,
        },
    )
    .unwrap();
    let variables = BTreeMap::from([("SPRAWLING_CI_SOURCE_SHA".to_owned(), "second".to_owned())]);
    let changed_head = key(
        &root,
        &action(),
        &Identity {
            base: &[],
            dependencies: &dependencies,
            environment: &environment,
            variables: &variables,
            files: None,
        },
    )
    .unwrap();
    assert_ne!(first, changed_verifier);
    assert_ne!(first, changed_head);
    std::fs::remove_dir_all(root).unwrap();
}
