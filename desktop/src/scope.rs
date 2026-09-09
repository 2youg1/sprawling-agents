// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this server is allowed to touch, read from `DESKTOP.toml`.
//!
//! Fail closed, three times over: no file closes the scope, a file this
//! version cannot read closes the scope, and a window the file does not
//! list is refused. Nothing here reaches the platform — the decision is
//! pure, so every clause of it is testable while not one Win32 call has
//! run.
//!
//! A whole-screen capture is refused because the file has no way to
//! permit it: the allowlist speaks of windows, and a full-screen image
//! shows everything the allowlist left out (desktop-SPEC.md section 8.5,
//! third pair).

mod pattern;

use crate::refusal::{Refusal, RefusalCode};
// Re-exported rather than kept private: `platform::windows::target`
// matches a caller's window name with the same rule this allowlist is
// matched with. Two rules over window titles would be two places to
// answer "what did I actually allow", and the one that is not the
// allowlist's is the one nobody would read.
pub(crate) use pattern::Pattern;
use serde::Deserialize;
use std::path::Path;

/// What one `tools/call` asks to touch. The three travel together
/// because no one of them decides anything on its own.
pub(crate) struct Reach<'a> {
    pub(crate) tool: &'a str,
    pub(crate) title: Option<&'a str>,
    pub(crate) process: Option<&'a str>,
}

/// The tools that touch one window, and therefore have to name one.
const WINDOW_FACING: [&str; 4] = [
    "desktop.snapshot",
    "desktop.act",
    "desktop.screenshot",
    "desktop.record",
];

/// The scope file, as `serde` reads it. Unknown fields are refused: a
/// misspelt key that silently permitted nothing would be read by its
/// author as a key that worked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeFile {
    #[serde(default)]
    windows: Vec<String>,
    #[serde(default)]
    processes: Vec<String>,
    #[serde(default)]
    record: bool,
    #[serde(default)]
    clipboard: bool,
}

/// What the scope file permits.
pub(crate) enum Scope {
    /// Nothing at all, the sentence that says why, and the code that
    /// tells the two whys apart: a file nobody wrote is a permission
    /// nobody gave, and a file that cannot be read is a defect in it.
    Closed {
        code: RefusalCode,
        because: String,
    },
    Open(Allowance),
}

/// The four things a scope file states.
pub(crate) struct Allowance {
    windows: Vec<Pattern>,
    processes: Vec<Pattern>,
    record: bool,
    clipboard: bool,
}

impl Scope {
    /// Reads the file, or closes without one.
    ///
    /// This cannot fail: absence and damage are both answers, and both
    /// are the same answer.
    pub(crate) fn read(path: Option<&Path>) -> Scope {
        let Some(path) = path else {
            return Scope::Closed {
                code: RefusalCode::GateDenied,
                because: "no scope file was named".to_owned(),
            };
        };
        match std::fs::read_to_string(path) {
            Ok(text) => Scope::parse(&text),
            Err(err) => Scope::Closed {
                code: RefusalCode::GateDenied,
                because: format!("{}: {err}", path.display()),
            },
        }
    }

    /// Reads the file's text.
    pub(crate) fn parse(text: &str) -> Scope {
        match toml::from_str::<ScopeFile>(text) {
            Ok(file) => Scope::Open(Allowance {
                windows: file.windows.iter().map(|line| Pattern::new(line)).collect(),
                processes: file
                    .processes
                    .iter()
                    .map(|line| Pattern::new(line))
                    .collect(),
                record: file.record,
                clipboard: file.clipboard,
            }),
            Err(err) => Scope::Closed {
                code: RefusalCode::ConfigInvalid,
                because: format!("the scope file cannot be read: {err}"),
            },
        }
    }

    /// Whether this call may proceed to the platform.
    ///
    /// # Errors
    /// Refuses everything when the scope is closed, a tool whose switch
    /// is off, a call that names no window where one is required, and a
    /// window this file does not list.
    pub(crate) fn admits(&self, reach: &Reach<'_>) -> Result<(), Refusal> {
        match self {
            Scope::Closed { code, because } => Err(Refusal::new(
                *code,
                "use the desktop",
                format!("this server has no scope: {because}"),
                "write a DESKTOP.toml listing the windows this server may touch, and give its \
                 path as this server's first argument",
            )),
            Scope::Open(allowance) => allowance.admits(reach),
        }
    }
}

impl Allowance {
    fn admits(&self, reach: &Reach<'_>) -> Result<(), Refusal> {
        if reach.tool == "desktop.record" && !self.record {
            return Err(switched_off("record"));
        }
        if reach.tool == "desktop.clipboard" {
            if self.clipboard {
                return Ok(());
            }
            return Err(switched_off("clipboard"));
        }
        if !WINDOW_FACING.contains(&reach.tool) {
            return Ok(());
        }
        match (reach.title, reach.process) {
            (None, None) => Err(Refusal::new(
                RefusalCode::GateDenied,
                "use the desktop",
                format!("`{}` was asked for without naming a window", reach.tool),
                "name the window by its `title` or its `process`; this scope lists windows, so \
                 the whole screen would show what it leaves out",
            )),
            (title, process) => {
                if let Some(title) = title {
                    admitted_by(&self.windows, title, "title")?;
                }
                if let Some(process) = process {
                    admitted_by(&self.processes, process, "process")?;
                }
                Ok(())
            }
        }
    }
}

fn switched_off(switch: &str) -> Refusal {
    Refusal::new(
        RefusalCode::GateDenied,
        "use the desktop",
        format!("this scope does not switch `{switch}` on"),
        // The two sentences name the same file the closed-scope refusal
        // names, so a reader learns one place to go.
        &format!("set `{switch} = true` in DESKTOP.toml if this machine's operator wants it"),
    )
}

/// # Errors
/// Refuses a name no pattern in `allowed` matches.
fn admitted_by(allowed: &[Pattern], named: &str, kind: &str) -> Result<(), Refusal> {
    if allowed.iter().any(|glob| glob.matches(named)) {
        return Ok(());
    }
    Err(Refusal::new(
        RefusalCode::GateDenied,
        "use the desktop",
        format!("no {kind} pattern in this scope matches `{named}`"),
        "add a pattern for it to DESKTOP.toml, or work on a window this scope already lists",
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
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
}
