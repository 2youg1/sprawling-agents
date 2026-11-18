// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

const NOW: u64 = 1_780_000_000;

fn program(root: &Path, name: &str, version: &str) -> Program {
    Program::new(version, root.join(name))
}

fn environment(root: &Path, python: &str, uv: &str) -> Environment {
    Environment::new(program(root, "python", python), program(root, "uv", uv))
}

fn observed(root: &Path) -> Observed {
    let environment = environment(root, "3.12.13", "0.12.3");
    Observed {
        python: Some(environment.python),
        uv: Some(environment.uv),
    }
}

fn check(
    root: PathBuf,
    probe: impl FnMut() -> Result<Observed>,
    repair: impl FnMut(Repair, &Paths, Observed) -> Result<Environment>,
) -> Result<Environment> {
    ensure_with(Request::new(root, "current", NOW), probe, repair)
}

#[test]
fn a_current_complete_cache_never_probes_or_installs() {
    let directory = tempfile::tempdir().unwrap();
    let expected = environment(directory.path(), "3.12.13", "0.12.3");
    write_environment(
        &Paths::new(directory.path().to_path_buf()),
        "current",
        &expected,
    )
    .unwrap();
    let actual = check(
        directory.path().to_path_buf(),
        || panic!("cache hit probed"),
        |_, _, _| panic!("cache hit installed"),
    )
    .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn an_update_replaces_the_whole_environment_record() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    write_environment(
        &paths,
        "old",
        &environment(directory.path(), "3.11.9", "0.1.0"),
    )
    .unwrap();
    let actual = check(
        directory.path().to_path_buf(),
        || Ok(observed(directory.path())),
        |_, _, _| panic!("supported programs must not be installed"),
    )
    .unwrap();
    let expected = observed(directory.path());
    assert_eq!(actual.python, expected.python.unwrap());
    assert_eq!(actual.uv, expected.uv.unwrap());
    assert_eq!(read_environment(&paths, "current").unwrap(), Some(actual));
}

#[test]
fn only_missing_uv_requests_the_uv_repair() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let existing = program(root, "python", "3.12.13");
    let result = check(
        root.to_path_buf(),
        || {
            Ok(Observed {
                python: Some(program(root, "python", "3.12.13")),
                uv: None,
            })
        },
        |kind, _, observed| {
            assert_eq!(kind, Repair::Uv);
            Ok(Environment::new(
                observed.python.unwrap(),
                program(root, "uv-private", "0.12.3"),
            ))
        },
    )
    .unwrap();
    assert_eq!(result.python, existing);
    assert_eq!(result.uv.executable, root.join("uv-private"));
}

#[test]
fn expired_python_requires_the_combined_python_repair() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let result = check(
        root.to_path_buf(),
        || {
            Ok(Observed {
                python: Some(program(root, "python-old", "3.9.24")),
                uv: Some(program(root, "uv-old", "0.1.0")),
            })
        },
        |kind, _, _| {
            assert_eq!(kind, Repair::Python);
            Ok(environment(root, "3.15.0", "0.13.0"))
        },
    )
    .unwrap();
    assert_eq!(result.python.version, "3.15.0");
    assert_eq!(result.uv.version, "0.13.0");
}

#[test]
fn failed_repair_does_not_publish_a_successful_cache() {
    let directory = tempfile::tempdir().unwrap();
    let result = check(
        directory.path().to_path_buf(),
        || {
            Ok(Observed {
                python: None,
                uv: None,
            })
        },
        |_, _, _| {
            Err(failure(
                "fixture installation",
                "intentional failure",
                "inspect the test",
                None,
            ))
        },
    );
    assert!(result.is_err());
    assert_eq!(
        read_environment(&Paths::new(directory.path().to_path_buf()), "current").unwrap(),
        None
    );
}

#[test]
fn a_damaged_cache_is_rebuilt() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    std::fs::write(&paths.cache, "not json").unwrap();
    let result = check(
        directory.path().to_path_buf(),
        || Ok(observed(directory.path())),
        |_, _, _| panic!("no install needed"),
    )
    .unwrap();
    assert_eq!(result.python.version, "3.12.13");
}

#[test]
fn concurrent_first_starts_share_one_probe_and_cache() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let probes = std::sync::atomic::AtomicUsize::new(0);
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let probes = &probes;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                check(
                    root.to_path_buf(),
                    || {
                        probes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        Ok(observed(root))
                    },
                    |_, _, _| panic!("no install needed"),
                )
                .unwrap();
            });
        }
    });
    assert_eq!(probes.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn python_support_uses_the_official_eol_boundaries() {
    assert!(python_supported("3.10.20", 1_790_812_799));
    assert!(!python_supported("3.10.20", 1_790_812_800));
    assert!(python_supported("3.8.20", 1_728_345_599));
    assert!(!python_supported("3.8.20", 1_728_345_600));
    assert!(python_supported("3.11.14", NOW));
    assert!(python_supported("3.16.0", NOW));
    assert!(!python_supported("2.7.18", NOW));
    assert!(!python_supported("not-a-version", NOW));
}

#[cfg(unix)]
#[test]
fn cache_paths_preserve_non_unicode_unix_bytes() {
    use std::os::unix::ffi::OsStringExt as _;
    let path = PathBuf::from(OsString::from_vec(vec![b'p', 0xff]));
    assert_eq!(cache::decode_path(&cache::encode_path(&path)), Some(path));
}

#[cfg(windows)]
#[test]
fn cache_paths_preserve_windows_unicode() {
    let path = PathBuf::from("C:\\工作区\\python.exe");
    assert_eq!(cache::decode_path(&cache::encode_path(&path)), Some(path));
}
