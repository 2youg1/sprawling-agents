// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::*;

fn key() -> Key {
    Key::named("same-content".to_owned())
}

#[test]
fn custom_store_paths_cannot_become_captured_source() {
    let root = tempfile::tempdir().unwrap();
    assert!(output_path(root.path(), Path::new("cache with spaces")).is_err());
    assert!(output_path(root.path(), Path::new("target/../src/cache")).is_err());
    assert!(output_path(root.path(), Path::new("target/ci/cache with spaces")).is_ok());
    assert!(!root.path().join("cache with spaces").exists());
    root.close().unwrap();
}

#[cfg(unix)]
#[test]
fn an_outside_store_alias_cannot_write_into_source() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("checkout");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let alias = outer.path().join("alias");
    std::os::unix::fs::symlink(root.join("src"), &alias).unwrap();
    assert!(output_path(&root, &alias.join("new-cache")).is_err());
    outer.close().unwrap();
}

#[cfg(unix)]
#[test]
fn default_target_alias_obeys_the_same_source_boundary() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("checkout");
    std::fs::create_dir_all(root.join("src/cache")).unwrap();
    let target = root.join("target");
    std::os::unix::fs::symlink(root.join("src/cache"), &target).unwrap();
    assert!(Store::for_root(&root, None).is_err());
    std::fs::remove_file(&target).unwrap();
    let external = outer.path().join("build");
    std::fs::create_dir(&external).unwrap();
    std::os::unix::fs::symlink(&external, &target).unwrap();
    assert!(Store::for_root(&root, None).is_ok());
    outer.close().unwrap();
}

#[test]
fn no_loaded_identity_never_reads_or_publishes_verdicts() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::local(root.path().to_owned());
    let environment = Environment {
        os: "unsupported".to_owned(),
        arch: "test".to_owned(),
        verifier: None,
    };
    assert!(store.save_green("gate", &key(), &environment).is_err());
    assert!(
        store
            .restore("gate", &key(), &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    root.close().unwrap();
}

#[test]
fn only_the_same_green_content_is_reused() {
    let green = Cached {
        key: "same-content".to_owned(),
        verdict: Verdict::Green,
    };
    assert!(may_reuse(Reuse::SuccessOnly, &green, &key()));
    assert!(!may_reuse(Reuse::Never, &green, &key()));
    assert!(!may_reuse(
        Reuse::SuccessOnly,
        &green,
        &Key::named("changed".to_owned())
    ));
    for verdict in [Verdict::Refused, Verdict::Broken] {
        assert!(!may_reuse(
            Reuse::SuccessOnly,
            &Cached {
                key: green.key.clone(),
                verdict
            },
            &key()
        ));
    }
}

#[test]
fn local_round_trip_and_corruption_fail_closed() {
    let root = std::env::temp_dir().join(format!("xtask-ci-store-{}", std::process::id()));
    let store = Store::local(root.clone());
    let environment = Environment {
        os: "test".to_owned(),
        arch: "test".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    store.save_green("gate", &key(), &environment).unwrap();
    assert!(
        store
            .restore("gate", &key(), &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .restore(
                "gate",
                &Key::named("other".to_owned()),
                &environment,
                Reuse::SuccessOnly
            )
            .unwrap()
            .is_none()
    );
    let record = root.join(local_entry("gate", &key()));
    std::fs::write(&record, "not json").unwrap();
    assert!(
        store
            .restore("gate", &key(), &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_none()
    );
    store.save_green("gate", &key(), &environment).unwrap();
    assert!(
        store
            .restore("gate", &key(), &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_some(),
        "publishing over a corrupted entry must heal it"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_publication_leaves_one_valid_green_result() {
    let root = std::env::temp_dir().join(format!("xtask-ci-store-race-{}", std::process::id()));
    let store = Store::local(root.clone());
    let environment = Environment {
        os: "test".to_owned(),
        arch: "test".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| store.save_green("gate", &key(), &environment).unwrap());
        }
    });
    assert!(
        store
            .restore("gate", &key(), &environment, Reuse::SuccessOnly)
            .unwrap()
            .is_some()
    );
    let files = std::fs::read_dir(root.join("gate")).unwrap().count();
    assert_eq!(files, 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn remote_cache_keys_include_environment() {
    let environment = Environment {
        os: "windows".to_owned(),
        arch: "x86_64".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    let other = Environment {
        os: "linux".to_owned(),
        arch: "x86_64".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    let keys = BTreeSet::from([key().cache_key(&environment), key().cache_key(&other)]);
    assert_eq!(keys.len(), 2);
    assert_ne!(
        remote_key("one", &key(), &environment),
        remote_key("two", &key(), &environment)
    );
}

#[test]
fn the_remote_adapter_round_trip_preserves_each_content_generation() {
    let root = std::env::temp_dir().join(format!("xtask-ci-remote-store-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let adapter = root.join("adapter.ts");
    let store = serde_json::to_string(&root.join("objects")).unwrap();
    std::fs::write(
        &adapter,
        format!(
            r#"import {{ join }} from "node:path";
const [operation, key, directory] = process.argv.slice(2);
const object = join({store}, `${{key}}.json`);
if (operation === "save") {{
  await Bun.write(object, await Bun.file(join(directory, "verdict.json")).text());
}} else if (await Bun.file(object).exists()) {{
  await Bun.write(join(directory, "verdict.json"), await Bun.file(object).text());
  console.log("hit");
}} else {{
  console.log("miss");
}}
"#
        ),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("objects")).unwrap();
    let store = Store::Remote(RemoteStore {
        adapter,
        work: root.join("work"),
    });
    let environment = Environment {
        os: "test".to_owned(),
        arch: "test".to_owned(),
        verifier: Some("test-verifier".to_owned()),
    };
    for key in [key(), Key::named("next-generation".to_owned())] {
        store.save_green("gate", &key, &environment).unwrap();
        assert!(
            store
                .restore("gate", &key, &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_some()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
