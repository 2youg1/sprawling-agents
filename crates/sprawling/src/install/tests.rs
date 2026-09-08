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
