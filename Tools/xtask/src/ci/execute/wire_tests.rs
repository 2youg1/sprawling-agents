// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::ci::content::{self, Identity};
use crate::ci::suite;

#[test]
fn generated_wire_cache_refuses_legacy_extra_deleted_and_changed_chunks() {
    let action = suite::ACTIONS
        .iter()
        .find(|action| action.id == "wire-ts")
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    for input in suite::BASE_INPUTS.iter().chain(action.inputs) {
        let mut file = root.path().join(input.root);
        if [
            "Tools/xtask",
            ".github/ci-cache",
            crate::walk::CLIENT_SRC,
            "Sprawling/wire",
            "Sprawling/kernel",
            "Sprawling/documents",
        ]
        .contains(&input.root)
        {
            file = file.join("fixture.txt");
        }
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "fixture\n").unwrap();
    }
    let environment = Environment {
        os: "fixture".to_owned(),
        arch: "fixture".to_owned(),
        verifier: Some("fixture-verifier".to_owned()),
    };
    let store = Store::local(root.path().join("target/verdicts"));
    let dependencies = BTreeMap::new();
    let variables = BTreeMap::new();
    let identity = Identity {
        base: suite::BASE_INPUTS,
        dependencies: &dependencies,
        environment: &environment,
        variables: &variables,
        files: None,
    };
    let chunk = root
        .path()
        .join(crate::walk::CLIENT_SRC)
        .join("wire/c00.ts");
    for mutation in ["wire.ts", "wire/rogue.ts", "delete", "change"] {
        crate::wire_ts::write(root.path()).unwrap();
        let good = Snapshot::capture(root.path()).unwrap();
        let good_key = content::key_in_snapshot(&good, action, &identity).unwrap();
        assert_eq!(
            execute_one(
                &Execution {
                    view: &good,
                    work: root.path(),
                    store: &store,
                    environment: &environment,
                },
                &PreparedAction {
                    action,
                    key: good_key.clone(),
                    variables: BTreeMap::new(),
                    files: None,
                }
            ),
            Outcome::Green
        );
        assert!(
            store
                .restore("wire-ts", &good_key, &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_some()
        );
        match mutation {
            "delete" => std::fs::remove_file(&chunk).unwrap(),
            "change" => std::fs::write(&chunk, "changed\n").unwrap(),
            path => std::fs::write(
                root.path().join(crate::walk::CLIENT_SRC).join(path),
                "unexpected\n",
            )
            .unwrap(),
        }
        let changed = match mutation {
            "delete" | "change" => "wire/c00.ts",
            path => path,
        };
        assert!(
            crate::ci::graph::affected_with_base(
                suite::ACTIONS,
                suite::BASE_INPUTS,
                &BTreeSet::from([format!("{}/{changed}", crate::walk::CLIENT_SRC)]),
            )
            .contains("wire-ts")
        );
        let bad = Snapshot::capture(root.path()).unwrap();
        let bad_key = content::key_in_snapshot(&bad, action, &identity).unwrap();
        assert_ne!(
            good_key, bad_key,
            "{mutation} must invalidate the production wire-ts key"
        );
        assert!(
            store
                .restore("wire-ts", &bad_key, &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            execute_one(
                &Execution {
                    view: &bad,
                    work: root.path(),
                    store: &store,
                    environment: &environment,
                },
                &PreparedAction {
                    action,
                    key: bad_key.clone(),
                    variables: BTreeMap::new(),
                    files: None,
                }
            ),
            Outcome::Refused,
            "{mutation} must run the real wire-ts gate"
        );
        assert!(
            store
                .restore("wire-ts", &bad_key, &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_none()
        );
        good.close().unwrap();
        bad.close().unwrap();
    }
    root.close().unwrap();
}
