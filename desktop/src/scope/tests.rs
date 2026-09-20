// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the allowlist is held to: the three ways it closes, the two
//! switches, and the windows it neither admits nor discloses.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;

fn reach<'a>(tool: &'a str, title: Option<&'a str>) -> Reach<'a> {
    Reach {
        tool,
        title,
        process: None,
    }
}

/// The rule the whole file exists for: with no scope, this server
/// does nothing at all.
#[test]
fn a_missing_scope_file_refuses_everything() {
    let nowhere = Path::new("no-such-directory-here/DESKTOP.toml");
    for scope in [Scope::read(None), Scope::read(Some(nowhere))] {
        for tool in [
            "desktop.windows",
            "desktop.snapshot",
            "desktop.act",
            "desktop.screenshot",
            "desktop.record",
            "desktop.clipboard",
        ] {
            let refusal = scope
                .admits(&reach(tool, Some("Notepad")))
                .expect_err("a closed scope admits nothing");
            let error = refusal.as_error();
            assert_eq!(error["data"]["code"], "E_GATE_DENIED", "{tool}");
            assert!(
                error["data"]["recovery"]
                    .as_str()
                    .unwrap()
                    .contains("DESKTOP.toml"),
                "{tool}"
            );
        }
    }
}

/// A file that cannot be read is not a file that permits everything.
#[test]
fn a_damaged_scope_file_closes_the_scope_rather_than_opening_it() {
    for text in ["windows = [", "windows = 3", "widnows = [\"*\"]"] {
        let scope = Scope::parse(text);
        assert!(
            scope
                .admits(&reach("desktop.act", Some("Notepad")))
                .is_err(),
            "{text}"
        );
    }
}

#[test]
fn a_listed_window_is_admitted_and_an_unlisted_one_is_not() {
    let scope = Scope::parse("windows = [\"*Notepad*\", \"Calculator\"]\n");
    assert!(
        scope
            .admits(&reach("desktop.act", Some("a.txt — Notepad")))
            .is_ok()
    );
    assert!(
        scope
            .admits(&reach("desktop.snapshot", Some("Calculator")))
            .is_ok()
    );
    let refused = scope
        .admits(&reach("desktop.screenshot", Some("Password Manager")))
        .expect_err("an unlisted window is refused");
    assert_eq!(refused.as_error()["data"]["code"], "E_GATE_DENIED");
    assert!(
        refused.as_error()["data"]["recovery"]
            .as_str()
            .unwrap()
            .contains("DESKTOP.toml")
    );
    // A process name is judged against the other list, and a call
    // naming both has to satisfy both.
    let by_process = Scope::parse("processes = [\"Notepad.exe\"]\n");
    let named = |process| Reach {
        tool: "desktop.snapshot",
        title: None,
        process: Some(process),
    };
    assert!(by_process.admits(&named("NOTEPAD.EXE")).is_ok());
    assert!(by_process.admits(&named("notepad2.exe")).is_err());
    // A pattern anchored at both ends does not match a longer title.
    assert!(
        scope
            .admits(&reach("desktop.act", Some("Calculator Plus")))
            .is_err()
    );
}

/// The scope file has no way to say "the whole screen", so the whole
/// screen is refused rather than assumed.
#[test]
fn a_capture_that_names_no_window_is_refused_with_a_next_step() {
    let scope = Scope::parse("windows = [\"*\"]\nrecord = true\n");
    for tool in [
        "desktop.snapshot",
        "desktop.act",
        "desktop.screenshot",
        "desktop.record",
    ] {
        let refusal = scope
            .admits(&reach(tool, None))
            .expect_err("naming no window is refused");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_GATE_DENIED", "{tool}");
        assert!(
            error["data"]["recovery"].as_str().unwrap().contains("name"),
            "{tool}"
        );
    }
    // Listing windows names none by nature, and is how a caller
    // learns what to name.
    assert!(scope.admits(&reach("desktop.windows", None)).is_ok());
}

#[test]
fn recording_and_the_clipboard_are_each_off_until_their_own_switch_is_on() {
    let shut = Scope::parse("windows = [\"*\"]\n");
    let record = shut
        .admits(&reach("desktop.record", Some("Notepad")))
        .expect_err("recording is off by default");
    assert_eq!(record.as_error()["data"]["code"], "E_GATE_DENIED");
    assert!(
        record.as_error()["data"]["recovery"]
            .as_str()
            .unwrap()
            .contains("record")
    );
    let clipboard = shut
        .admits(&reach("desktop.clipboard", None))
        .expect_err("the clipboard is off by default");
    assert!(
        clipboard.as_error()["data"]["recovery"]
            .as_str()
            .unwrap()
            .contains("clipboard")
    );

    let open = Scope::parse("windows = [\"*\"]\nrecord = true\nclipboard = true\n");
    assert!(
        open.admits(&reach("desktop.record", Some("Notepad")))
            .is_ok()
    );
    assert!(open.admits(&reach("desktop.clipboard", None)).is_ok());
}

/// An empty allowlist is a file that lists no window, which is not
/// the same thing as a file that lists every window.
#[test]
fn an_empty_allowlist_admits_no_window() {
    let scope = Scope::parse("record = true\nclipboard = true\n");
    assert!(
        scope
            .admits(&reach("desktop.act", Some("Notepad")))
            .is_err()
    );
    assert!(scope.admits(&reach("desktop.clipboard", None)).is_ok());
}

#[test]
fn a_scope_file_on_disk_is_read_from_the_path_it_was_given() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("DESKTOP.toml");
    std::fs::write(&path, "windows = [\"*Notepad*\"]\nclipboard = true\n").unwrap();
    let scope = Scope::read(Some(&path));
    assert!(
        scope
            .admits(&reach("desktop.act", Some("x — Notepad")))
            .is_ok()
    );
    assert!(scope.admits(&reach("desktop.clipboard", None)).is_ok());
    assert!(
        scope
            .admits(&reach("desktop.record", Some("x — Notepad")))
            .is_err()
    );
}

/// S-10: a window list is as disclosing as a snapshot, so the tool that
/// reports titles is held to the allowlist that governs acting on them.
/// A scope naming Notepad does not disclose that a password manager is
/// open, nor what document is in the browser.
#[test]
fn a_window_this_scope_does_not_list_is_not_even_reported() {
    let scope = Scope::parse("windows = [\"*Notepad*\"]\n");
    let admitted = scope
        .admits(&reach("desktop.windows", None))
        .expect("listing windows names none by nature");
    assert!(admitted.visible("a.txt — Notepad", "notepad.exe"));
    assert!(!admitted.visible("Vault — 1Password", "1password.exe"));
    assert!(!admitted.visible("Board minutes.docx — Word", "winword.exe"));
}

/// Visibility reads the two lists `admits` reads, so a window reachable
/// by its process is reported with the title that reaches it — and a
/// window reachable by neither identifier is reported at all.
#[test]
fn a_process_this_scope_lists_makes_its_windows_visible_by_that_name() {
    let by_process = Scope::parse("processes = [\"notepad.exe\"]\n");
    let admitted = by_process
        .admits(&reach("desktop.windows", None))
        .expect("listing windows names none by nature");
    assert!(admitted.visible("a.txt — Notepad", "NOTEPAD.EXE"));
    assert!(!admitted.visible("a.txt — Notepad", "notepad2.exe"));
    // An empty allowlist shows nothing, which is the reading that makes
    // it admit nothing.
    let empty = Scope::parse("record = true\n");
    let nothing = empty
        .admits(&reach("desktop.windows", None))
        .expect("the tool itself is not switched off");
    assert!(!nothing.visible("a.txt — Notepad", "notepad.exe"));
}

/// A closed scope hands back no admission at all, so there is no value
/// a listing could be filtered through and the tool is refused whole.
#[test]
fn a_closed_scope_yields_no_admission_to_list_windows_with() {
    assert!(
        Scope::read(None)
            .admits(&reach("desktop.windows", None))
            .is_err()
    );
}
