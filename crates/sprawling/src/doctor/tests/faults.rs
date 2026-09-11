// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ugly paths (sprawling-SPEC.md section 8-47): a binary that will
//! not start, a variable naming nothing, a half-written component, a
//! tool that says nothing. Each one is a typed answer with its cause,
//! never the word "absent" on its own.
//!
//! Every probe here is handed a temporary directory as its search path
//! or its component directory, so nothing touches this process's own
//! environment.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::doctor::probe::{ask_version, component_at, on_search_path};
use crate::doctor::{Absence, Fault, Presence, Version};

const VARIABLE: &str = "SPRAWLING_TEST_COMPONENT";

/// A file on the search path that is not a program is found there and
/// reported as broken with the operating system's own words, which is
/// what a person needs to hear when the answer is not "install it".
#[test]
fn a_program_that_will_not_start_is_broken_not_absent() {
    let dir = tempfile::tempdir().unwrap();
    let name = if cfg!(windows) {
        "broken.exe"
    } else {
        "broken"
    };
    std::fs::write(dir.path().join(name), b"this is not a program\n").unwrap();
    let search_path = OsString::from(dir.path().display().to_string());
    let found = on_search_path(&search_path, "broken").expect("the file is on the search path");
    match ask_version(&found, "--version", Duration::from_secs(5)) {
        Presence::Broken {
            at,
            fault: Fault::WillNotStart(said),
        } => {
            assert_eq!(at, found);
            assert!(!said.is_empty(), "the fault carries the OS's words");
        }
        other => panic!("a file that is not a program is broken, not {other:?}"),
    }
    assert_eq!(
        on_search_path(&search_path, "no-such-program"),
        None,
        "a name nothing on the path carries is absent"
    );
}

/// A set variable that names nothing is said so, and never falls
/// through to the component directory: a wrong pointer that silently
/// lost would stay wrong forever.
#[test]
fn a_variable_that_names_nothing_is_said_so() {
    let dir = tempfile::tempdir().unwrap();
    let component = dir.path().join("python-wasi");
    std::fs::create_dir_all(&component).unwrap();
    std::fs::write(component.join("python.wasm"), b"\0asm").unwrap();
    let nowhere = dir.path().join("nowhere.wasm");
    let answered = component_at(
        VARIABLE,
        Some(OsString::from(nowhere.display().to_string())),
        Some(component.clone()),
        "python.wasm",
    );
    assert_eq!(
        answered,
        Presence::Absent(Absence::VariableNamesNothing {
            variable: VARIABLE,
            path: nowhere,
        })
    );
    assert!(
        answered.describe().contains(VARIABLE),
        "the line names the variable a person set: {}",
        answered.describe()
    );

    let unset = component_at(VARIABLE, None, Some(component.clone()), "python.wasm");
    assert!(unset.usable(), "{unset:?}");
    assert_eq!(unset.at(), Some(component.join("python.wasm").as_path()));

    let empty = component_at(
        VARIABLE,
        Some(OsString::new()),
        Some(component.clone()),
        "python.wasm",
    );
    assert_eq!(empty, unset, "an empty variable is an unset one");
}

/// A component directory that exists without its file is an install
/// that stopped half way, and the line says what to do about it.
#[test]
fn a_half_written_component_directory_is_broken() {
    let dir = tempfile::tempdir().unwrap();
    let component = dir.path().join("python-wasi");
    std::fs::create_dir_all(&component).unwrap();
    let answered = component_at(VARIABLE, None, Some(component.clone()), "python.wasm");
    assert_eq!(
        answered,
        Presence::Broken {
            at: component.clone(),
            fault: Fault::HalfWritten,
        }
    );
    assert!(!answered.usable());
    assert!(answered.describe().contains("install again"));

    let never = component_at(
        VARIABLE,
        None,
        Some(dir.path().join("never-made")),
        "python.wasm",
    );
    assert!(
        matches!(never, Presence::Absent(Absence::NoComponent { .. })),
        "{never:?}"
    );
    assert_eq!(
        component_at(VARIABLE, None, None, "python.wasm"),
        Presence::Absent(Absence::NoHome)
    );
}

/// A program that starts and says nothing is present all the same: the
/// question presence answers is whether it is here, and a silent tool
/// is here.
#[test]
fn a_present_tool_that_says_nothing_is_still_present() {
    // `findstr /V x` and `/bin/sh -c` over an empty stdin: both start,
    // write nothing on stdout, and exit.
    let (program, arg) = if cfg!(windows) {
        (PathBuf::from("findstr.exe"), "/V")
    } else {
        (PathBuf::from("/bin/sh"), "-c")
    };
    let answered = ask_version(&program, arg, Duration::from_secs(5));
    assert_eq!(
        answered,
        Presence::Present {
            at: program,
            version: Version::Silent,
        }
    );
    assert!(answered.usable());
    assert!(answered.describe().contains("said nothing"));
}

/// A program is looked for under the names this platform gives it.
///
/// On Windows the extensionless file comes last: a directory holding
/// both `bun` and `bun.cmd` holds one file this operating system can
/// start and one it cannot, and taking them in the wrong order reports
/// an installed tool as a silent one.
#[test]
fn a_program_is_looked_for_under_every_name_this_platform_gives_it() {
    let names = crate::doctor::probe::names_of("bun");
    assert_eq!(names.last().map(String::as_str), Some("bun"));
    if cfg!(target_os = "windows") {
        assert_eq!(names.first().map(String::as_str), Some("bun.exe"));
        assert!(names.iter().any(|name| name == "bun.cmd"));
    } else {
        assert_eq!(names.len(), 1);
    }
}
