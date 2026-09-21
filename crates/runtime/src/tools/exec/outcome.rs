// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The shape of every answer the exec tool gives: the four result
//! payloads, and the two tails a caller reads after them.
//!
//! One file owns the key names — `arm`, `stdout`, `stderr`,
//! `exit_code`, `outcome`, `handle`, `what`, `detail`, `background`,
//! `env` — so an arm cannot spell a result differently from its
//! neighbour.

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
