// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `--shells` lens of `sprawling view` (`crates/sprawling/spec/Main.lean`
//! §8-105): each shell interpreter's calls and failures by class.
//!
//! The fold and its rules are `runtime::ShellTally`'s
//! (`crates/runtime/Spec.lean` §8-13-2 D30); this file reads the ledger,
//! hands it every tool result, and writes what it counted.

use std::io::Write;
use std::path::Path;

use kernel::{EventKind, EventRecord};
use serde_json::{Map, Value, json};

use super::ViewError;

/// Writes one JSON line per interpreter, by name.
pub(super) fn write_shells(dir: &Path, out: &mut impl Write) -> Result<(), ViewError> {
    let index = storage::LedgerIndex::rebuild(dir)?;
    let mut reader = index.reader(dir);
    let mut tally = runtime::ShellTally::default();
    for seq in index.seqs() {
        let record = EventRecord::parse_line(&reader.line_at(seq)?)?;
        if record.kind() != EventKind::ToolResult {
            continue;
        }
        if let Some(result) = record
            .data()
            .as_map()
            .get("result")
            .and_then(Value::as_object)
        {
            tally.absorb(result);
        }
    }
    for (interpreter, count) in tally.interpreters() {
        let failures: Map<String, Value> = count
            .failures
            .iter()
            .map(|(class, times)| (class.as_str().to_owned(), Value::from(*times)))
            .collect();
        let line = json!({
            "interpreter": interpreter,
            "calls": count.calls,
            "failures": failures,
        });
        serde_json::to_writer(&mut *out, &line).map_err(std::io::Error::from)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
    use serde_json::{Value, json};

    use super::write_shells;

    /// Shell results from two interpreters and one older record without
    /// the field, among records the lens must pass over.
    #[test]
    fn each_interpreter_gets_one_line_with_its_calls_and_failures_by_class() {
        let dir = tempfile::tempdir().unwrap();
        let script = [
            (EventKind::ToolCalled, json!({"tool": "exec"})),
            (
                EventKind::ToolResult,
                json!({"result": {"arm": "shell", "interpreter": "pwsh", "exit_code": 1,
                                  "stdout": "", "stderr": "ParserError"}}),
            ),
            (
                EventKind::ToolResult,
                json!({"result": {"arm": "shell", "interpreter": "pwsh", "exit_code": 0,
                                  "stdout": "ok", "stderr": ""}}),
            ),
            (
                EventKind::ToolResult,
                json!({"result": {"arm": "shell", "exit_code": 9009, "stdout": "", "stderr": ""}}),
            ),
            (
                EventKind::ToolResult,
                json!({"result": {"arm": "program", "exit_code": 2, "stdout": "", "stderr": ""}}),
            ),
        ];
        let mut prev = kernel::GENESIS_PREV;
        let mut blob = Vec::new();
        for (seq, (kind, data)) in script.into_iter().enumerate() {
            let seq = u64::try_from(seq).unwrap();
            let draft = EventDraft {
                run: RunId::from_bytes([1; 16]),
                t: TimeMs::new(seq),
                who: "tester".to_owned(),
                addr: None,
                kind,
                data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
                ig: false,
            };
            let line = EventRecord::from_draft(draft, Seq::new(seq), prev)
                .canonical_line()
                .unwrap();
            prev = B3Hash::digest(&line);
            blob.extend_from_slice(&line);
            blob.push(b'\n');
        }
        std::fs::write(dir.path().join("ledger-00000000000000000000.jsonl"), &blob).unwrap();

        let mut out = Vec::new();
        write_shells(dir.path(), &mut out).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            lines,
            vec![
                json!({"interpreter": "pwsh", "calls": 2, "failures": {"syntax": 1}}),
                json!({"interpreter": "system", "calls": 1, "failures": {"command_not_found": 1}}),
            ]
        );
    }
}
