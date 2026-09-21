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
//!
//! **Admission is a value, and it is the only way to reach the desk.**
//! [`Scope::admits`] hands back an [`Admitted`] carrying the allowlist
//! that admitted the call, so `desktop.windows` filters what it reports
//! through the very patterns that admitted it. A window list is as
//! disclosing as a snapshot — a title carries a document name, a URL
//! and often a person's name — so the tool that reports titles is held
//! to the same allowlist as the tools that act on them.

mod pattern;

use crate::refusal::{Refusal, RefusalCode};
use crate::tools::ToolName;
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
    pub(crate) tool: ToolName,
    pub(crate) title: Option<&'a str>,
    pub(crate) process: Option<&'a str>,
}

/// Whether this tool touches one window, and therefore has to name one.
///
/// Exhaustive on purpose: a seventh tool is a compile error here, which
/// is the only moment anybody is certain to ask whether it discloses a
/// window.
const fn names_a_window(tool: ToolName) -> bool {
    match tool {
        ToolName::Snapshot | ToolName::Act | ToolName::Screenshot | ToolName::Record => true,
        ToolName::Windows | ToolName::Clipboard => false,
    }
}

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
#[derive(Debug)]
pub(crate) struct Allowance {
    windows: Vec<Pattern>,
    processes: Vec<Pattern>,
    record: bool,
    clipboard: bool,
}

/// That one call cleared this scope, carrying the allowlist that
/// cleared it.
///
/// `platform::Desk::perform` takes one, so no tool can be carried out
/// without an admission having been decided first, and the one tool
/// whose *answer* is scoped rather than merely its permission —
/// `desktop.windows` — reads the allowlist through this value instead
/// of through a second copy of it.
#[derive(Debug)]
pub(crate) struct Admitted<'a> {
    allowance: &'a Allowance,
}

impl Admitted<'_> {
    /// Whether this scope's caller may be told that one window exists.
    ///
    /// A window is visible exactly when the caller could name it: its
    /// title matches the window list, or its process matches the
    /// process list. That is the dual of [`Scope::admits`], which
    /// judges the identifiers a call *gave*; here the identifiers are
    /// the window's own. An empty allowlist therefore shows nothing,
    /// which is the same reading that makes it admit nothing.
    pub(crate) fn visible(&self, title: &str, process: &str) -> bool {
        self.allowance.visible(title, process)
    }
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
    pub(crate) fn admits(&self, reach: &Reach<'_>) -> Result<Admitted<'_>, Refusal> {
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
    fn admits(&self, reach: &Reach<'_>) -> Result<Admitted<'_>, Refusal> {
        let admitted = Admitted { allowance: self };
        if reach.tool == ToolName::Record && !self.record {
            return Err(switched_off("record"));
        }
        if reach.tool == ToolName::Clipboard {
            if self.clipboard {
                return Ok(admitted);
            }
            return Err(switched_off("clipboard"));
        }
        if !names_a_window(reach.tool) {
            return Ok(admitted);
        }
        match (reach.title, reach.process) {
            (None, None) => Err(Refusal::new(
                RefusalCode::GateDenied,
                "use the desktop",
                format!(
                    "`{}` was asked for without naming a window",
                    reach.tool.as_str()
                ),
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
                Ok(admitted)
            }
        }
    }

    /// Whether one window on this desktop is one this scope's caller
    /// could name, which is what makes it one this caller may be told
    /// about.
    fn visible(&self, title: &str, process: &str) -> bool {
        self.windows.iter().any(|glob| glob.matches(title))
            || self.processes.iter().any(|glob| glob.matches(process))
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
mod tests;
