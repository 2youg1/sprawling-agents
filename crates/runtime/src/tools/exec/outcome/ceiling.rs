// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `memory_ceiling` entry of an exec result: its keys, its states
//! and the sentence a reader is given (`crates/runtime/spec/Tools/Exec.lean`
//! D95). Part of the one file that owns the result's spellings.

use kernel::{AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::backlog::{Ceiling, Unapplied};

/// Adds how the person's memory ceiling fared to a settled result.
pub(in crate::tools::exec) fn with_ceiling(
    outcome: ToolOutcome,
    ceiling: Option<Ceiling>,
) -> Result<ToolOutcome, AxError> {
    if ceiling.is_none() {
        return Ok(outcome);
    }
    let mut result = outcome.result.as_map().clone();
    memory_ceiling(&mut result, ceiling);
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: outcome.attachments,
    })
}

/// Writes how the person's memory ceiling fared into a result, under
/// `memory_ceiling` (`crates/runtime/spec/Tools/Exec.lean` D95). Nothing
/// is written for a ceiling that was not asked for or held unreached.
pub(super) fn memory_ceiling(result: &mut Map<String, Value>, ceiling: Option<Ceiling>) {
    let Some(ceiling) = ceiling else {
        return;
    };
    let mut entry = Map::new();
    let (state, limit, detail) = match ceiling {
        Ceiling::Hit { limit } => (
            "hit",
            limit,
            format!(
                "an allocation of this run's processes was refused at the memory ceiling of {} \
                 bytes the person set under Settings > Performance, so this command may have \
                 failed for lack of memory; ask the person to raise or clear the ceiling, or \
                 make the command use less memory",
                limit.get()
            ),
        ),
        Ceiling::Unapplied { limit, why } => {
            entry.insert("why".to_owned(), Value::String(why.as_str().to_owned()));
            (
                "unapplied",
                limit,
                format!(
                    "the memory ceiling of {} bytes the person set did not apply to this \
                     command: {}",
                    limit.get(),
                    unapplied(why)
                ),
            )
        }
        Ceiling::Unread { limit } => (
            "unread",
            limit,
            format!(
                "whether this run reached the memory ceiling of {} bytes cannot be read, so a \
                 failure for lack of memory cannot be told apart from any other failure",
                limit.get()
            ),
        ),
    };
    entry.insert("state".to_owned(), Value::String(state.to_owned()));
    entry.insert("limit_bytes".to_owned(), Value::Number(limit.get().into()));
    entry.insert("detail".to_owned(), Value::String(detail));
    result.insert("memory_ceiling".to_owned(), Value::Object(entry));
}

/// Why an asked ceiling did not apply, as a reader is told it.
fn unapplied(why: Unapplied) -> &'static str {
    match why {
        Unapplied::Platform => "this platform has no memory ceiling for a run's processes",
        Unapplied::NotDelegated => {
            "this machine does not delegate its cgroup to the city, so no run can be limited"
        }
        Unapplied::Refused => {
            "the operating system refused the ceiling, or the means to tell when it is reached"
        }
        Unapplied::Unjoined => {
            "the command did not join its run's job or cgroup; a container command is held by \
             the building's container limits instead"
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::super::{settled, with_backlog};
    use super::*;
    use crate::backlog::{Exit, Finished};

    /// The client reads `memory_ceiling` under these spellings
    /// (`client/src/views/monitor/trace.ts`), from a settled result and
    /// from each background row alike.
    #[test]
    fn a_ceiling_is_written_under_one_spelling_in_both_places() {
        let limit = std::num::NonZeroU64::new(1 << 30).unwrap();
        let ceiling = Some(Ceiling::Unapplied {
            limit,
            why: Unapplied::NotDelegated,
        });
        let answer = with_ceiling(
            settled("", "", Exit::Ended { code: 1 }, "shell").unwrap(),
            ceiling,
        )
        .unwrap();
        let entry = |map: &Map<String, Value>| {
            let mut entry = map["memory_ceiling"].as_object().unwrap().clone();
            assert!(
                entry
                    .remove("detail")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("1073741824")
            );
            Value::Object(entry)
        };
        let expected = serde_json::json!({"state": "unapplied", "why": "not_delegated", "limit_bytes": 1_u64 << 30});
        assert_eq!(entry(answer.result.as_map()), expected);
        let background = with_backlog(
            settled("", "", Exit::Ended { code: 0 }, "shell").unwrap(),
            vec![Finished {
                id: crate::Backlog::default()
                    .enrol_run(
                        &kernel::Address::parse("vault").unwrap(),
                        "a run".to_owned(),
                    )
                    .unwrap(),
                what: "cargo build".to_owned(),
                exit: Exit::Ended { code: 101 },
                stdout: String::new(),
                stderr: String::new(),
                ceiling: Some(Ceiling::Hit { limit }),
            }],
        )
        .unwrap();
        let row = background.result.as_map()["background"][0]
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(
            entry(&row),
            serde_json::json!({"state": "hit", "limit_bytes": 1_u64 << 30})
        );
        let quiet = with_ceiling(
            settled("", "", Exit::Ended { code: 0 }, "shell").unwrap(),
            None,
        )
        .unwrap();
        assert!(!quiet.result.as_map().contains_key("memory_ceiling"));
    }
}
