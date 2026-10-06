// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::program_dir;
use std::path::{Path, PathBuf};

/// The judgements compile on Windows alone, and the tests that
/// exercise them compile there too: a test of a function that does
/// not exist on this platform proves nothing about this platform.
/// Windows is where the push gate runs, so these run on every push.
#[cfg(target_os = "windows")]
mod plan {
    use crate::install::{PathEdit, PathRemoval, SEPARATOR, plan_append, plan_remove};

    /// Stand-ins, not paths on any machine: the release gate refuses
    /// a published file that names somebody's home directory, and a
    /// test fixture is published like everything else.
    fn dir() -> &'static str {
        r"X:\elsewhere\Local\Programs\sprawling"
    }

    fn other() -> String {
        r"X:\unpacked\system32".to_owned()
    }

    #[test]
    fn an_empty_search_path_becomes_the_one_directory() {
        assert_eq!(plan_append("", dir()), PathEdit::Append(dir().to_owned()));
    }

    #[test]
    fn a_directory_already_there_is_not_added_twice() {
        let current = format!("{}{SEPARATOR}{}", other(), dir());
        assert_eq!(plan_append(&current, dir()), PathEdit::AlreadyPresent);
    }

    #[test]
    fn a_trailing_separator_does_not_make_a_second_entry() {
        let current = format!("{}{SEPARATOR}{}{SEPARATOR}", other(), dir());
        assert_eq!(plan_append(&current, dir()), PathEdit::AlreadyPresent);
    }

    /// Windows resolves paths case-insensitively, so a differently-cased
    /// entry is the same entry and adding another is a duplicate.
    #[test]
    fn a_differently_cased_entry_is_the_same_entry() {
        let current = format!("{}{SEPARATOR}{}", other(), dir().to_uppercase());
        assert_eq!(plan_append(&current, dir()), PathEdit::AlreadyPresent);
    }

    #[test]
    fn a_directory_that_is_not_there_is_appended_after_what_was() {
        let current = other();
        let expected = format!("{}{SEPARATOR}{}", other(), dir());
        assert_eq!(plan_append(&current, dir()), PathEdit::Append(expected));
    }

    #[test]
    fn removing_a_directory_that_was_never_added_changes_nothing() {
        assert_eq!(plan_remove(&other(), dir()), PathRemoval::Absent);
    }

    #[test]
    fn removing_keeps_every_other_entry_including_an_empty_one() {
        let current = format!("{}{SEPARATOR}{}{SEPARATOR}", other(), dir());
        assert_eq!(
            plan_remove(&current, dir()),
            PathRemoval::Rewrite(format!("{}{SEPARATOR}", other()))
        );
    }

    /// The pair of properties the whole card rests on: installing twice
    /// is installing once, and uninstalling returns the exact string.
    #[test]
    fn append_then_remove_returns_the_original_search_path() {
        for current in ["", &other(), &format!("{}{SEPARATOR}", other())] {
            let PathEdit::Append(extended) = plan_append(current, dir()) else {
                panic!("{current:?} does not contain the directory yet");
            };
            assert_eq!(plan_append(&extended, dir()), PathEdit::AlreadyPresent);
            let PathRemoval::Rewrite(back) = plan_remove(&extended, dir()) else {
                panic!("the directory was just added");
            };
            assert_eq!(
                back.trim_end_matches(SEPARATOR),
                current.trim_end_matches(SEPARATOR)
            );
        }
    }
}

#[cfg(target_os = "windows")]
#[test]
fn windows_installs_under_the_per_user_program_directory() {
    let dir = program_dir(
        Some(Path::new(r"X:\appdata")),
        Some(Path::new(r"X:\elsewhere")),
    );
    assert_eq!(dir, Some(PathBuf::from(r"X:\appdata\Programs\sprawling")));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn other_platforms_install_under_the_xdg_user_binary_directory() {
    let dir = program_dir(None, Some(Path::new("/elsewhere")));
    assert_eq!(dir, Some(PathBuf::from("/elsewhere/.local/bin")));
}

#[test]
fn with_nowhere_to_install_the_answer_is_nowhere() {
    assert_eq!(program_dir(None, None), None);
}

#[test]
fn displacing_an_external_copy_never_removes_the_running_binary() {
    let root = tempfile::tempdir().unwrap();
    let running = std::env::current_exe().unwrap();
    let target = root.path().join(super::installed_name());
    std::fs::write(&target, b"installed copy").unwrap();
    assert!(super::displace(root.path()).unwrap());
    assert!(!target.exists());
    assert!(running.exists());
    assert!(!super::displace(root.path()).unwrap());
}

#[cfg(target_os = "windows")]
const FIXTURE: &str = "SPRAWLING_INSTALL_REMOVAL_FIXTURE";

#[cfg(target_os = "windows")]
#[test]
fn windows_uninstall_removes_its_running_executable() {
    if let Some(dir) = std::env::var_os(FIXTURE) {
        let dir = Path::new(&dir);
        assert_eq!(
            std::env::current_exe().unwrap().canonicalize().unwrap(),
            dir.join(super::installed_name()).canonicalize().unwrap()
        );
        assert!(super::displace(dir).unwrap());
        return;
    }
    let root = tempfile::Builder::new()
        .prefix("uninstall fixture ")
        .tempdir()
        .unwrap();
    let dir = root.path().join("installed");
    let scratch = root.path().join("scratch");
    std::fs::create_dir(&dir).unwrap();
    std::fs::create_dir(&scratch).unwrap();
    let target = dir.join(super::installed_name());
    std::fs::copy(std::env::current_exe().unwrap(), &target).unwrap();
    run_removal_fixture(&target, &dir, &scratch);
    // The helper and the `cmd.exe` it starts inherit the fixture's output,
    // so here the helper has deleted the moved image and closed its
    // handles. Whether Windows deletes the helper's own copy depends on
    // the helper's image being unmapped before its last handle closes,
    // an order no event marks (Install.lean D53), so only that copy may
    // remain.
    let names = |dir: &Path| {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>()
    };
    let mut left = names(&scratch);
    left.retain(|name| !name.ends_with(".__selfdelete__.exe"));
    assert_eq!((names(&dir), left), (Vec::new(), Vec::new()));
}

#[cfg(target_os = "windows")]
#[test]
fn windows_uninstall_removes_only_the_link_to_an_external_running_executable() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive.exe");
    let dir = root.path().join("installed");
    let scratch = root.path().join("scratch");
    std::fs::create_dir(&dir).unwrap();
    std::fs::create_dir(&scratch).unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), &archive).unwrap();
    let original = std::fs::read(&archive).unwrap();
    let target = dir.join(super::installed_name());
    std::os::windows::fs::symlink_file(&archive, &target).unwrap();
    assert!(std::fs::symlink_metadata(&target).unwrap().is_symlink());
    run_removal_fixture(&archive, &dir, &scratch);
    assert!(
        archive.is_file(),
        "the external running executable was removed"
    );
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    assert_eq!(
        std::fs::symlink_metadata(&target).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
    assert!(!super::displace(&dir).unwrap());
}

#[cfg(target_os = "windows")]
#[test]
fn windows_uninstall_removes_a_dangling_file_link() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing.exe");
    let dir = root.path().join("installed");
    std::fs::create_dir(&dir).unwrap();
    let target = dir.join(super::installed_name());
    std::os::windows::fs::symlink_file(&missing, &target).unwrap();
    assert!(std::fs::symlink_metadata(&target).unwrap().is_symlink());
    assert!(super::displace(&dir).unwrap());
    assert_eq!(
        std::fs::symlink_metadata(&target).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(!missing.exists());
    assert!(!super::displace(&dir).unwrap());
}

#[cfg(target_os = "windows")]
fn run_removal_fixture(exe: &Path, dir: &Path, scratch: &Path) {
    let done = std::process::Command::new(exe)
        .args([
            "--exact",
            "install::tests::windows_uninstall_removes_its_running_executable",
            "--nocapture",
        ])
        .env(FIXTURE, dir)
        .env("TEMP", scratch)
        .env("TMP", scratch)
        .output()
        .unwrap();
    assert!(
        done.status.success(),
        "stdout: {}
stderr: {}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
}
