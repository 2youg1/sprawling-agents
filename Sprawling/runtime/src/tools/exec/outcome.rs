// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::backlog::{BacklogId, Exit, Finished};

mod ceiling;
use ceiling::memory_ceiling;
pub(super) use ceiling::{with_ceiling, with_wait};

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
            memory_ceiling(&mut row, member.ceiling);
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

const BEFORE_INTERPRETERS: &str = "system";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureClass {
    CommandNotFound,

    Syntax,

    Encoding,
}

enum Family {
    Cmd,
    Pwsh,
    Posix,

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
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            FailureClass::CommandNotFound => "command_not_found",
            FailureClass::Syntax => "syntax",
            FailureClass::Encoding => "encoding",
        }
    }

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellCount {
    pub calls: u64,

    pub failures: BTreeMap<FailureClass, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellTally {
    by_interpreter: BTreeMap<String, ShellCount>,
}

impl ShellTally {
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

    pub fn interpreters(&self) -> impl Iterator<Item = (&str, &ShellCount)> {
        self.by_interpreter
            .iter()
            .map(|(name, count)| (name.as_str(), count))
    }
}

