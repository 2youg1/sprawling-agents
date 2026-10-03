// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The shape of every answer the exec tool gives: the four result
//! payloads, and the two tails a caller reads after them.
//!
//! One file owns the key names — `arm`, `stdout`, `stderr`,
//! `exit_code`, `outcome`, `handle`, `what`, `detail`, `background`,
//! `env`, `interpreter` — so an arm cannot spell a result differently
//! from its neighbour, and the tally that reads shell results back out
//! of the ledger reads them under the same spellings
//! (`crates/runtime/Spec.lean` §8-13-2 D30).

use std::collections::BTreeMap;

use kernel::{AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::backlog::{BacklogId, Exit, Finished};

/// Writes how a child stopped into a result, in the one spelling this
/// file owns.
///
/// A code is written only when there is a code: a command a signal
/// stopped and a command this city never managed to wait on both used
/// to be reported as `exit_code: -1`, which a model reads as a program
/// that ran and failed.
fn ending(result: &mut Map<String, Value>, exit: Exit) {
    match exit {
        Exit::Ended { code } => {
            result.insert(
                "exit_code".to_owned(),
                Value::Number(i64::from(code).into()),
            );
        }
        Exit::Signalled => {
            result.insert(
                "outcome".to_owned(),
                Value::String(exit.as_str().to_owned()),
            );
            result.insert(
                "detail".to_owned(),
                Value::String(
                    "a signal stopped this command, so it returned no code; `halt` is \
                     what usually sends one"
                        .to_owned(),
                ),
            );
        }
        Exit::Unknown { why } => {
            result.insert(
                "outcome".to_owned(),
                Value::String(exit.as_str().to_owned()),
            );
            result.insert(
                "detail".to_owned(),
                Value::String(why.sentence().to_owned()),
            );
        }
    }
}

/// What a caller is told about a command that outlived its window.
///
/// The handle and the sentence travel together: an agent that is given
/// an identifier and no instruction waits for it anyway, which is the
/// behaviour this whole table exists to stop.
pub(super) fn backgrounded(id: &BacklogId, what: &str, arm: &str) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String(arm.to_owned()));
    result.insert(
        "outcome".to_owned(),
        Value::String("backgrounded".to_owned()),
    );
    result.insert("handle".to_owned(), Value::String(id.to_string()));
    result.insert("what".to_owned(), Value::String(what.to_owned()));
    result.insert(
        "detail".to_owned(),
        Value::String(
            "still running; do not wait for it - carry on, and its result arrives at the end \
             of a later tool result"
                .to_owned(),
        ),
    );
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Adds every background member that has stopped since the last call to
/// the tail of this result. A command that settles inside its window
/// carries no handle: the caller asked whether it finished, the table
/// answered, and the entry is already gone.
///
/// It is the tail rather than the head because the answer the caller
/// asked for is the one it is reading for; what arrived while it was
/// working comes after.
pub(super) fn with_backlog(
    outcome: ToolOutcome,
    done: Vec<Finished>,
) -> Result<ToolOutcome, AxError> {
    if done.is_empty() {
        return Ok(outcome);
    }
    let mut result = outcome.result.as_map().clone();
    let rows = done
        .into_iter()
        .map(|member| {
            let mut row = Map::new();
            row.insert("handle".to_owned(), Value::String(member.id.to_string()));
            row.insert("what".to_owned(), Value::String(member.what));
            ending(&mut row, member.exit);
            row.insert("stdout".to_owned(), Value::String(member.stdout));
            row.insert("stderr".to_owned(), Value::String(member.stderr));
            Value::Object(row)
        })
        .collect();
    result.insert("background".to_owned(), Value::Array(rows));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

pub(super) fn settled(
    stdout: &str,
    stderr: &str,
    exit: Exit,
    arm: &str,
) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String(arm.to_owned()));
    result.insert("stdout".to_owned(), Value::String(stdout.to_owned()));
    result.insert("stderr".to_owned(), Value::String(stderr.to_owned()));
    ending(&mut result, exit);
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Adds the names this call's child inherited to its result.
///
/// Names only, never values: which names a run inherited is a fact the
/// ledger keeps, and what those names held is a fact it must not.
pub(super) fn with_environment(
    outcome: ToolOutcome,
    inherited: &BTreeMap<String, String>,
) -> Result<ToolOutcome, AxError> {
    let mut result = outcome.result.as_map().clone();
    result.insert(
        "env".to_owned(),
        Value::Array(
            inherited
                .keys()
                .map(|name| Value::String(name.clone()))
                .collect(),
        ),
    );
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Adds which interpreter ran a shell line to its result.
pub(super) fn with_interpreter(
    outcome: ToolOutcome,
    interpreter: &str,
) -> Result<ToolOutcome, AxError> {
    let mut result = outcome.result.as_map().clone();
    result.insert(
        INTERPRETER.to_owned(),
        Value::String(interpreter.to_owned()),
    );
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

const INTERPRETER: &str = "interpreter";

/// What a record written before results named their interpreter is
/// counted under: then the shell arm only ever ran the platform's own.
const BEFORE_INTERPRETERS: &str = "system";

/// One class of shell failure a reader can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureClass {
    /// The interpreter found no command by the name the line gave.
    CommandNotFound,
    /// The interpreter could not parse the line.
    Syntax,
    /// The output held bytes that were not UTF-8, written down as U+FFFD.
    Encoding,
}

/// The interpreter families whose failures read alike.
enum Family {
    Cmd,
    Pwsh,
    Posix,
    /// A record from before results named their interpreter.
    Unnamed,
}

impl Family {
    fn of(interpreter: &str) -> Family {
        match interpreter {
            "cmd" => Family::Cmd,
            "pwsh" | "powershell" => Family::Pwsh,
            BEFORE_INTERPRETERS => Family::Unnamed,
            _ => Family::Posix,
        }
    }
}

impl FailureClass {
    /// Which class a shell result's failure falls in, or `None` for a
    /// success and for a failure no rule recognises.
    ///
    /// The exit code is read first, then the error ids that do not change
    /// with the display language, then the replacement character, and
    /// English text last: cmd prints its syntax errors in the language
    /// Windows is set to, so on another language they stay unclassified
    /// rather than this file carrying a second copy of Microsoft's text.
    #[must_use]
    pub fn of(interpreter: &str, exit_code: i64, stdout: &str, stderr: &str) -> Option<Self> {
        if exit_code == 0 {
            return None;
        }
        let family = Family::of(interpreter);
        let not_found = match family {
            Family::Cmd => exit_code == 9009,
            Family::Posix => exit_code == 127,
            Family::Pwsh => stderr.contains("CommandNotFoundException"),
            Family::Unnamed => exit_code == 9009 || exit_code == 127,
        };
        let parser = matches!(family, Family::Pwsh) && stderr.contains("ParserError");
        let cmd_text = stderr.contains("was unexpected at this time.")
            || stderr.contains("The syntax of the command is incorrect.");
        let posix_text = exit_code == 2 && stderr.contains("syntax error");
        let syntax_text = match family {
            Family::Cmd => cmd_text,
            Family::Posix => posix_text,
            Family::Pwsh => false,
            Family::Unnamed => cmd_text || posix_text,
        };
        if not_found {
            Some(FailureClass::CommandNotFound)
        } else if parser {
            Some(FailureClass::Syntax)
        } else if stdout.contains('\u{FFFD}') || stderr.contains('\u{FFFD}') {
            Some(FailureClass::Encoding)
        } else if syntax_text {
            Some(FailureClass::Syntax)
        } else {
            None
        }
    }
}

/// How one interpreter's shell lines ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellCount {
    /// Lines that ended with a code.
    pub calls: u64,
    /// Of those, how many failed in each class.
    pub failures: BTreeMap<FailureClass, u64>,
}

/// Shell results folded per interpreter: the reading that decides
/// whether PowerShell 7 should become the default (D30).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellTally {
    by_interpreter: BTreeMap<String, ShellCount>,
}

impl ShellTally {
    /// Counts one exec result, the `result` object of a tool result
    /// record. Any other arm, and a shell line that ended without a code
    /// (handed to the background, or stopped by a signal), is not
    /// counted: there is nothing yet to classify.
    pub fn absorb(&mut self, result: &Map<String, Value>) {
        if result.get("arm").and_then(Value::as_str) != Some("shell") {
            return;
        }
        let Some(code) = result.get("exit_code").and_then(Value::as_i64) else {
            return;
        };
        let text = |key: &str| result.get(key).and_then(Value::as_str).unwrap_or("");
        let interpreter = result
            .get(INTERPRETER)
            .and_then(Value::as_str)
            .unwrap_or(BEFORE_INTERPRETERS);
        let count = self
            .by_interpreter
            .entry(interpreter.to_owned())
            .or_default();
        count.calls = count.calls.saturating_add(1);
        if let Some(class) = FailureClass::of(interpreter, code, text("stdout"), text("stderr")) {
            let failed = count.failures.entry(class).or_default();
            *failed = failed.saturating_add(1);
        }
    }

    /// Every interpreter seen, by name, with its count.
    pub fn interpreters(&self) -> impl Iterator<Item = (&str, &ShellCount)> {
        self.by_interpreter
            .iter()
            .map(|(name, count)| (name.as_str(), count))
    }
}

pub(super) fn exceptional(
    stdout: &[u8],
    stderr: &[u8],
    kind: &str,
    detail: Option<String>,
) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String("python".to_owned()));
    result.insert(
        "stdout".to_owned(),
        Value::String(String::from_utf8_lossy(stdout).into_owned()),
    );
    result.insert(
        "stderr".to_owned(),
        Value::String(String::from_utf8_lossy(stderr).into_owned()),
    );
    result.insert("outcome".to_owned(), Value::String(kind.to_owned()));
    if let Some(detail) = detail {
        result.insert("detail".to_owned(), Value::String(detail));
    }
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}
