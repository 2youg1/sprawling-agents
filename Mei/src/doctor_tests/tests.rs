// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    reason = "test code"
)]

use super::*;

#[test]
fn a_waiter_retries_after_the_last_poll_sleep() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let held = lock::Lock::acquire(&paths, Duration::from_secs(1)).unwrap();
            std::thread::sleep(Duration::from_millis(150));
            drop(held);
        });
        std::thread::sleep(Duration::from_millis(40));
        let taken = lock::Lock::acquire(&paths, Duration::from_millis(200)).unwrap();
        drop(taken);
    });
}

#[test]
fn an_unsupported_repair_is_not_published() {
    let directory = tempfile::tempdir().unwrap();
    let result = check(
        directory.path().to_path_buf(),
        |_| {
            Ok(Observed {
                python: None,
                uv: None,
            })
        },
        |_, _, _| Ok(environment(directory.path(), "3.9.24", "0.12.3")),
    );
    assert!(result.is_err());
    assert_eq!(
        read_environment(&Paths::new(directory.path().to_path_buf()), "current").unwrap(),
        None
    );
}

#[test]
fn cache_candidates_are_unique_within_one_process() {
    let paths = Paths::new(PathBuf::from("doctor"));
    assert_ne!(
        cache::temporary_cache_path(&paths),
        cache::temporary_cache_path(&paths)
    );
}

#[cfg(unix)]
#[test]
fn path_probes_skip_nonexecutables_and_record_the_absolute_candidate() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let bad = directory.path().join("bad");
    let good = directory.path().join("good");
    std::fs::create_dir(&bad).unwrap();
    std::fs::write(bad.join("uv"), []).unwrap();
    std::fs::set_permissions(bad.join("uv"), std::fs::Permissions::from_mode(0o644)).unwrap();
    fake_uv(&good);
    let path = std::env::join_paths([bad, good]).unwrap();
    let uv = probe_uv_search(Some(path)).unwrap().unwrap();
    assert_eq!(uv.version, "0.12.3");
    assert!(uv.executable.is_absolute());
}

#[cfg(unix)]
#[test]
fn an_unexecutable_private_uv_is_not_a_probe_error() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    std::fs::create_dir_all(&paths.bin).unwrap();
    let candidate = paths.bin.join(uv_program());
    std::fs::write(&candidate, []).unwrap();
    std::fs::set_permissions(candidate, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(probe_private_uv(&paths).unwrap(), None);
}

#[test]
fn a_relative_doctor_override_is_made_absolute() {
    assert!(
        cache::absolute_override(PathBuf::from("relative-doctor"))
            .unwrap()
            .is_absolute()
    );
}

#[cfg(not(windows))]
#[test]
fn empty_and_relative_xdg_paths_fall_back_to_home() {
    let home = PathBuf::from("/home/person");
    assert_eq!(
        cache::cache_base(Some(PathBuf::new()), Some(home.clone())),
        Some(home.join(".cache"))
    );
    assert_eq!(
        cache::cache_base(Some(PathBuf::from("relative")), Some(home.clone())),
        Some(home.join(".cache"))
    );
}

#[test]
fn uv_commands_ignore_overrides_projects_and_the_windows_registry() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path().to_path_buf());
    let uv = program(directory.path(), "uv", "0.12.3");
    let install = install::python_install_command(&paths, &uv, true);
    let args = install
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>();
    assert!(args.iter().any(|arg| arg == "--reinstall"));
    if cfg!(windows) {
        assert!(args.iter().any(|arg| arg == "--no-registry"));
    }
    let override_value = install
        .get_envs()
        .find(|(name, _)| *name == "UV_PYTHON")
        .map(|(_, value)| value);
    assert_eq!(override_value, Some(None));
    let find = install::python_find_command(&paths, &uv);
    let args = find
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>();
    assert!(args.iter().any(|arg| arg == "--no-project"));
    assert!(args.iter().any(|arg| arg == "--managed-python"));
}

#[test]
fn an_installer_is_stopped_at_the_shared_deadline() {
    let python = probe_python(OsStr::new(python_program())).unwrap().unwrap();
    let deadline = install::Deadline::after(Duration::from_millis(60)).unwrap();
    let started = std::time::Instant::now();
    let result = install::run(
        &deadline,
        Command::new(&python.executable).args(["-I", "-c", "import time; time.sleep(30)"]),
        "test the installer deadline",
        &python.executable,
    );
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
}
