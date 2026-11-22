// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::ci::content::Key;
use crate::ci::graph::Action;

#[test]
fn external_store_does_not_allow_logs_to_follow_target_into_source() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("checkout");
    std::fs::create_dir_all(root.join("src/cache")).unwrap();
    std::fs::write(root.join("src/kept.txt"), "captured\n").unwrap();
    let target = root.join("target");
    std::os::unix::fs::symlink(root.join("src/cache"), &target).unwrap();
    let view = Snapshot::capture(&root).unwrap();
    let store = Store::for_root(&root, Some(outer.path().join("external-verdicts"))).unwrap();
    let environment = Environment {
        os: "fixture".to_owned(),
        arch: "fixture".to_owned(),
        verifier: None,
    };
    static ACTION: Action = Action {
        id: "logged-tool",
        operation: Operation::Process {
            program: "bun",
            args: &["-e", "console.log('verified')"],
        },
        inputs: &[],
        needs: &[],
        reuse: Reuse::Never,
        required: true,
    };
    let prepared = PreparedAction {
        action: &ACTION,
        key: Key::named("output-boundary".to_owned()),
        variables: BTreeMap::new(),
        files: None,
    };
    let execution = Execution {
        view: &view,
        work: &root,
        store: &store,
        environment: &environment,
    };
    assert_eq!(execute_one(&execution, &prepared), Outcome::Broken);
    assert!(!root.join("src/cache/ci").exists());
    std::fs::remove_file(&target).unwrap();
    let external = outer.path().join("build");
    std::fs::create_dir(&external).unwrap();
    std::os::unix::fs::symlink(&external, &target).unwrap();
    assert_eq!(execute_one(&execution, &prepared), Outcome::Green);
    let logs = std::fs::read_dir(external.join("ci/logs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(logs.len(), 1);
    assert!(
        std::fs::read_to_string(logs.first().unwrap())
            .unwrap()
            .contains("verified")
    );
    assert!(!root.join("src/cache/ci").exists());
    view.close().unwrap();
    outer.close().unwrap();
}
