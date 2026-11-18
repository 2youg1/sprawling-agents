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

use std::time::Duration;

use super::*;
use super::{install, lock};

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

fn observed_environment(root: &Path) -> Environment {
    let observed = observed(root);
    Environment::new(observed.python.unwrap(), observed.uv.unwrap())
}

fn check(
    root: PathBuf,
    probe: impl FnMut(&Paths) -> Result<Observed>,
    repair: impl FnMut(Repair, &Paths, Observed) -> Result<Environment>,
) -> Result<Environment> {
    ensure_with(Request::new(root, "current", NOW), probe, repair)
}

fn writable(environment: &Environment) {
    std::fs::write(&environment.python.executable, []).unwrap();
    std::fs::write(&environment.uv.executable, []).unwrap();
}

#[expect(clippy::disallowed_methods, reason = "compile a standalone uv fixture")]
fn fake_uv(directory: &Path) {
    std::fs::create_dir_all(directory).unwrap();
    let source = directory.join("fixture.rs");
    std::fs::write(&source, "fn main() { println!(\"uv 0.12.3\"); }").unwrap();
    let executable = directory.join(uv_program());
    let status = Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn a_current_complete_cache_never_probes_or_installs() {
    let directory = tempfile::tempdir().unwrap();
    let expected = environment(directory.path(), "3.12.13", "0.12.3");
    writable(&expected);
    write_environment(
        &Paths::new(directory.path().to_path_buf()),
        "current",
        &expected,
    )
    .unwrap();
    let actual = check(
        directory.path().to_path_buf(),
        |_| panic!("cache hit probed"),
        |_, _, _| panic!("cache hit installed"),
    )
    .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn a_cache_whose_program_disappeared_is_rebuilt() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let stale = environment(root, "3.11.9", "0.1.0");
    writable(&stale);
    write_environment(&Paths::new(root.to_path_buf()), "current", &stale).unwrap();
    std::fs::remove_file(&stale.uv.executable).unwrap();
    let actual = check(
        root.to_path_buf(),
        |_| Ok(observed(root)),
        |_, _, _| panic!("a working probe must not trigger installation"),
    )
    .unwrap();
    assert_eq!(actual, observed_environment(root));
}

#[test]
fn a_cache_naming_a_directory_is_rebuilt() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let stale = environment(root, "3.11.9", "0.1.0");
    writable(&stale);
    write_environment(&Paths::new(root.to_path_buf()), "current", &stale).unwrap();
    std::fs::remove_file(&stale.python.executable).unwrap();
    std::fs::create_dir(&stale.python.executable).unwrap();
    let actual = check(
        root.to_path_buf(),
        |_| Ok(observed(root)),
        |_, _, _| panic!("a working probe must not trigger installation"),
    )
    .unwrap();
    assert_eq!(actual, observed_environment(root));
}

#[test]
fn an_update_replaces_the_whole_environment_record() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    let stale = environment(directory.path(), "3.11.9", "0.1.0");
    writable(&stale);
    write_environment(&paths, "old", &stale).unwrap();
    let actual = check(
        directory.path().to_path_buf(),
        |_| Ok(observed(directory.path())),
        |_, _, _| panic!("supported programs must not be installed"),
    )
    .unwrap();
    let expected = observed_environment(directory.path());
    assert_eq!(actual, expected);
    assert_eq!(read_environment(&paths, "current").unwrap(), Some(actual));
}

#[test]
fn only_missing_uv_requests_the_uv_repair() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let existing = program(root, "python", "3.12.13");
    let result = check(
        root.to_path_buf(),
        |_| {
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
        |_| {
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
        |_| {
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
            )
            .value("fixture"))
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
        |_| Ok(observed(directory.path())),
        |_, _, _| panic!("no install needed"),
    )
    .unwrap();
    assert_eq!(result.python.version, "3.12.13");
}

#[test]
fn concurrent_first_starts_share_one_probe_and_cache() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    writable(&observed_environment(root));
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
                    |_| {
                        probes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(100));
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
fn a_private_uv_is_reused_when_the_path_has_none() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    std::fs::create_dir_all(&paths.bin).unwrap();
    let python = probe_python(OsStr::new(python_program())).unwrap().unwrap();
    fake_uv(&paths.bin);
    let uv = probe_private_uv(&paths).unwrap().unwrap();
    assert_eq!(uv.version, "0.12.3");
    let observed = Observed {
        python: Some(python),
        uv: Some(uv),
    };
    assert_eq!(plan(&observed, NOW), None);
}

#[test]
fn a_waiter_takes_the_lock_once_its_holder_finishes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let paths = Paths::new(root.to_path_buf());
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let held = lock::Lock::acquire(&paths, Duration::from_secs(5)).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            drop(held);
        });
        std::thread::sleep(Duration::from_millis(50));
        let taken = lock::Lock::acquire(&paths, Duration::from_secs(5)).unwrap();
        drop(taken);
    });
}

#[test]
fn a_waiter_reports_a_holder_that_outlasts_its_budget() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let paths = Paths::new(root.to_path_buf());
    let held = lock::Lock::acquire(&paths, Duration::from_secs(5)).unwrap();
    let denied = lock::Lock::acquire(&paths, Duration::from_millis(150));
    assert!(denied.is_err());
    drop(held);
}

#[test]
fn the_waiter_outlasts_every_installer() {
    assert!(lock::LOCK_WAIT > install::BUDGET);
}

#[test]
fn a_leftover_lock_file_does_not_block_a_new_process() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    std::fs::write(&paths.lock, []).unwrap();
    drop(lock::Lock::acquire(&paths, Duration::from_secs(1)).unwrap());
}

#[cfg(unix)]
#[test]
fn a_python_probe_permission_error_is_not_missing_python() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("python");
    std::fs::write(&program, []).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(probe_python(program.as_os_str()).is_err());
}

#[test]
fn python_support_uses_the_official_eol_boundaries() {
    for minor in 0..=16 {
        let end = support::end_of_life(minor).unwrap();
        let exact = format!("3.{minor}.0");
        assert!(
            python_supported(&exact, end - 1),
            "3.{minor} one second early"
        );
        assert!(!python_supported(&exact, end), "3.{minor} at its end");
    }
    assert!(python_supported("3.17.0", NOW));
    assert!(python_supported("3.11.14", NOW));
    assert!(!python_supported("2.7.18", NOW));
    assert!(!python_supported("3", NOW));
    assert!(!python_supported("not-a-version", NOW));
}

#[test]
fn the_lean_environment_vectors_replay() {
    for vector in include_str!("doctor_vectors.txt").lines() {
        let vector = vector.trim();
        if vector.is_empty() || vector.starts_with('#') {
            continue;
        }
        let mut fields = vector.split_whitespace();
        let python = match fields.next() {
            Some("none") => {
                fields.next();
                fields.next();
                None
            }
            Some("python") => {
                let major = fields.next().unwrap();
                let minor = fields.next().unwrap();
                Some(program(
                    Path::new("."),
                    "python3",
                    &format!("{major}.{minor}.0"),
                ))
            }
            other => panic!("unexpected python field {other:?}: {vector}"),
        };
        let now = fields.next().unwrap().parse::<u64>().unwrap();
        let uv = match fields.next() {
            Some("none") => None,
            Some("uv") => Some(program(Path::new("."), "uv", "0.0.0")),
            other => panic!("unexpected uv field {other:?}: {vector}"),
        };
        let expected = match fields.next() {
            Some("none") => None,
            Some("uv") => Some(Repair::Uv),
            Some("python") => Some(Repair::Python),
            other => panic!("unexpected repair {other:?}: {vector}"),
        };
        assert_eq!(fields.next(), None, "trailing fields: {vector}");
        let actual = plan(&Observed { python, uv }, now);
        assert_eq!(actual, expected, "{vector}");
    }
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

#[path = "doctor_tests/tests.rs"]
mod review;
