// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::doctor::host::{PythonGuestSource, judge_python_guest};
use crate::doctor::probe::{ask_version, component_at, on_search_path};
use crate::doctor::{Absence, Fault, Presence, Version};

const VARIABLE: &str = "SPRAWLING_TEST_COMPONENT";

#[test]
fn a_guest_with_the_wrong_digest_is_broken_not_absent() {
    scan_python_in_child(false);
}

#[test]
fn an_override_scan_is_reported_unpinned() {
    scan_python_in_child(true);
}

fn scan_python_in_child(overridden: bool) {
    let dir = tempfile::tempdir().unwrap();
    let at = accounting::home::Home::at(dir.path())
        .components()
        .join("python-wasi/python.wasm");
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, b"wrong guest").unwrap();
    let mut child = child::command(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "doctor::tests::faults::isolated_python_scan",
            "--nocapture",
        ])
        .env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .env(
            "SPRAWLING_TEST_PYTHON_SCAN",
            if overridden { "override" } else { "pinned" },
        )
        .env_remove(crate::doctor::table::PYTHON_WASM_VARIABLE);
    if overridden {
        child.env(crate::doctor::table::PYTHON_WASM_VARIABLE, &at);
    }
    let output = child.output().unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("running 1 test"));
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn isolated_python_scan() {
    let Ok(mode) = std::env::var("SPRAWLING_TEST_PYTHON_SCAN") else {
        return;
    };
    let machine = crate::doctor::ThisMachine::new(
        crate::doctor::Platform::current(),
        crate::doctor::PATIENCE,
    );
    let findings = crate::doctor::examine(&machine);
    let found = &findings
        .iter()
        .find(|finding| finding.requirement.name == "python-wasi")
        .unwrap()
        .presence;
    let rendered = found.describe();
    if mode == "override" {
        assert!(found.usable(), "{rendered}");
        assert!(crate::doctor::usable_python_wasm().is_some());
        assert!(rendered.contains("unpinned override"), "{rendered}");
    } else {
        assert!(matches!(found, Presence::Broken { .. }), "{rendered}");
        assert!(crate::doctor::usable_python_wasm().is_none());
        assert!(
            rendered.contains(crate::doctor::table::PYTHON_GUEST_B3),
            "{rendered}"
        );
        assert!(
            rendered.contains(&kernel::B3Hash::digest(b"wrong guest").to_string()),
            "{rendered}"
        );
    }
}

#[test]
fn a_pinned_guest_requires_its_stdlib_even_after_a_cached_digest_match() {
    let dir = tempfile::tempdir().unwrap();
    let at = dir.path().join("python.wasm");
    std::fs::write(&at, b"registered guest").unwrap();
    let expected = kernel::B3Hash::digest(b"registered guest").to_string();
    let checked = std::sync::OnceLock::new();
    let found = Presence::Present {
        at: at.clone(),
        version: Version::Silent,
    };
    assert!(matches!(
        judge_python_guest(
            found.clone(),
            PythonGuestSource::Pinned,
            &expected,
            &checked
        ),
        Presence::Broken { .. }
    ));
    let lib = dir.path().join("lib/python3.14");
    for name in [
        "encodings/__init__.py",
        "collections/__init__.py",
        "difflib.py",
        "re/__init__.py",
        "os.py",
    ] {
        let path = lib.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"installed").unwrap();
    }
    assert_eq!(
        judge_python_guest(
            found.clone(),
            PythonGuestSource::Pinned,
            &expected,
            &checked
        ),
        found
    );
    std::fs::remove_file(lib.join("encodings/__init__.py")).unwrap();
    assert!(matches!(
        judge_python_guest(found, PythonGuestSource::Pinned, &expected, &checked),
        Presence::Broken { .. }
    ));
}

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

#[test]
fn a_present_tool_that_says_nothing_is_still_present() {

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
    assert!(answered.describe().contains("no version"));
}

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

#[test]
fn a_printed_recipe_and_a_manual_one_refuse_with_their_own_reason() {
    let printed = crate::doctor::Recipe::Print("curl -fsSL https://bun.sh/install")
        .command("bun")
        .err()
        .expect("a printed recipe is not run from here");
    assert_eq!(printed.code(), &kernel::AxCode::ToolUnavailable);
    assert!(printed.recovery().contains("code nobody read"));
    let by_hand = crate::doctor::Recipe::Manual("it ships with the system")
        .command("shell")
        .err()
        .expect("a manual recipe is not run from here");
    assert!(by_hand.recovery().contains("nothing here can install"));
    assert_ne!(printed.recovery(), by_hand.recovery());
}
