// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::B3Hash;
use serde_json::{Map, Value};

const REDACTED_REALM: &str = "redacted";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {

    Plain,

    Fingerprinted,
}

impl Marker {

    #[must_use]
    pub fn spell(self, found: &[u8]) -> String {
        match self {
            Marker::Plain => format!("secret:{REDACTED_REALM}"),
            Marker::Fingerprinted => {
                format!("secret:{REDACTED_REALM}/{}", fingerprint(found))
            }
        }
    }
}

#[must_use]
pub fn redact(mut payload: Map<String, Value>) -> (Map<String, Value>, u32) {
    let mut hits = 0;
    for value in payload.values_mut() {
        walk(value, &mut hits);
    }
    (payload, hits)
}

fn walk(value: &mut Value, hits: &mut u32) {
    match value {
        Value::String(text) => {
            if let Some((replaced, found)) = replaced(text, Marker::Fingerprinted) {
                *hits = hits.saturating_add(found);
                *text = replaced;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| walk(item, hits)),
        Value::Object(map) => map.values_mut().for_each(|item| walk(item, hits)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
fn copied(payload: &Map<String, Value>) -> (Map<String, Value>, u32) {
    fn walk(value: &Value, hits: &mut u32) -> Value {
        match value {
            Value::String(text) => {
                let (replaced, found) = redact_text(text, Marker::Fingerprinted);
                *hits = hits.saturating_add(found);
                Value::String(replaced)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| walk(item, hits)).collect())
            }
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| (key.clone(), walk(item, hits)))
                    .collect(),
            ),
            Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
        }
    }
    let mut hits = 0;
    let out = payload
        .iter()
        .map(|(key, value)| (key.clone(), walk(value, &mut hits)))
        .collect();
    (out, hits)
}

#[must_use]
pub fn redact_text(text: &str, marker: Marker) -> (String, u32) {
    replaced(text, marker).unwrap_or_else(|| (text.to_owned(), 0))
}

fn replaced(text: &str, marker: Marker) -> Option<(String, u32)> {
    let bytes = text.as_bytes();
    let spans = kernel::secret::scan(bytes);
    if spans.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    let mut hits: u32 = 0;
    for span in spans {
        let end = span.start.saturating_add(span.len);
        if span.start < at || end > bytes.len() {
            continue;
        }
        let Some(before) = bytes
            .get(at..span.start)
            .and_then(|b| std::str::from_utf8(b).ok())
        else {
            continue;
        };
        let Some(found) = bytes.get(span.start..end) else {
            continue;
        };
        out.push_str(before);
        out.push_str(&marker.spell(found));
        at = end;
        hits = hits.saturating_add(1);
    }

    if let Some(rest) = bytes.get(at..).and_then(|b| std::str::from_utf8(b).ok()) {
        out.push_str(rest);
    }
    (hits > 0).then_some((out, hits))
}

const FINGERPRINT_HEX: usize = 16;

#[must_use]
pub fn fingerprint(found: &[u8]) -> String {
    let digest = B3Hash::digest(found).to_string();
    digest.get(..FINGERPRINT_HEX).unwrap_or(&digest).to_owned()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn key_shaped() -> String {
        ["sk", "9fQ2xZ", "7Lm4Rt", "0Bv8Kd", "3Wp6"].join("")
    }

    #[test]
    fn a_key_shaped_run_of_bytes_does_not_reach_the_payload() {
        let key = key_shaped();
        let mut payload = Map::new();
        payload.insert(
            "message".to_owned(),
            Value::String(format!("the token is {key} and it works")),
        );
        let (redacted, hits) = redact(payload);
        assert_eq!(hits, 1);
        let text = redacted
            .get("message")
            .and_then(Value::as_str)
            .unwrap()
            .to_owned();
        assert!(!text.contains(&key), "the value itself is gone");
        assert!(text.contains("the token is "), "the sentence survives");
        assert!(text.contains("secret:redacted/"));
    }

    #[test]
    fn the_same_value_twice_marks_the_same_way_and_two_values_do_not() {
        let key = key_shaped();
        let other = ["sk", "2Tg7Yh", "4Nn1Qs", "8Cz5Ud", "6Ke0"].join("");
        let (first, _) = redact_text(&key, Marker::Fingerprinted);
        let (again, _) = redact_text(&key, Marker::Fingerprinted);
        let (different, _) = redact_text(&other, Marker::Fingerprinted);
        assert_eq!(first, again, "one value, one marker");
        assert_ne!(first, different, "two values are distinguishable");
    }

    #[test]
    fn nested_strings_are_reached_and_ordinary_prose_is_untouched() {
        let key = key_shaped();
        let mut inner = Map::new();
        inner.insert("text".to_owned(), Value::String(key.clone()));
        let mut payload = Map::new();
        payload.insert(
            "content".to_owned(),
            Value::Array(vec![Value::Object(inner)]),
        );
        payload.insert(
            "prose".to_owned(),
            Value::String("nothing here looks like a key at all".to_owned()),
        );
        payload.insert("count".to_owned(), Value::Number(3.into()));
        let (redacted, hits) = redact(payload);
        assert_eq!(hits, 1);
        assert_eq!(
            redacted.get("prose").and_then(Value::as_str),
            Some("nothing here looks like a key at all"),
            "prose is not a shape, and a scanner that trimmed it would be unusable"
        );
        assert_eq!(redacted.get("count").and_then(Value::as_u64), Some(3));
        let text = serde_json::to_string(&redacted).unwrap();
        assert!(!text.contains(&key));
    }

    #[test]
    fn a_marker_is_a_reference_a_reader_can_follow_to_an_honest_nothing() {
        let key = key_shaped();
        let short = fingerprint(key.as_bytes());
        assert_eq!(short.len(), 16);
        let (marked, hits) = redact_text(&key, Marker::Fingerprinted);
        assert_eq!(hits, 1);
        assert_eq!(marked, format!("secret:redacted/{short}"));
        let reference = kernel::SecretRef::parse(&marked).unwrap();
        assert_eq!(reference.realm(), REDACTED_REALM);
        assert_eq!(reference.name(), short);
    }

    #[test]
    fn a_plain_marker_says_that_something_was_here_and_nothing_more() {
        let key = key_shaped();
        let (line, hits) = redact_text(&format!("token {key} used"), Marker::Plain);
        assert_eq!(hits, 1);
        assert_eq!(line, "token secret:redacted used");
        assert!(!line.contains(&fingerprint(key.as_bytes())));
        let (kept, _) = redact_text(&format!("token {key} used"), Marker::Fingerprinted);
        assert!(kept.contains(&fingerprint(key.as_bytes())));
    }

    #[test]
    fn a_payload_with_nothing_to_hide_comes_back_unchanged() {
        let mut payload = Map::new();
        payload.insert(
            "message".to_owned(),
            Value::String("measured in metres".to_owned()),
        );
        let (redacted, hits) = redact(payload.clone());
        assert_eq!(hits, 0);
        assert_eq!(redacted, payload);
    }

    fn text_at<'v>(payload: &'v Map<String, Value>, key: &str) -> &'v str {
        payload.get(key).and_then(Value::as_str).unwrap()
    }

    #[test]
    fn a_clean_string_comes_back_in_the_allocation_it_went_in_with() {
        let key = key_shaped();
        let mut payload = Map::new();
        payload.insert(
            "prose".to_owned(),
            Value::String("measured in metres".to_owned()),
        );
        payload.insert("token".to_owned(), Value::String(format!("use {key}")));
        let before = text_at(&payload, "prose").as_ptr();
        let (redacted, hits) = redact(payload);
        assert_eq!(hits, 1);
        assert_eq!(
            text_at(&redacted, "prose").as_ptr(),
            before,
            "the clean string was copied"
        );
        assert!(!text_at(&redacted, "token").contains(&key));
    }

    fn payloads() -> impl proptest::strategy::Strategy<Value = Map<String, Value>> {
        use proptest::prelude::*;
        let text =
            ("[a-z =:]{0,12}", any::<bool>(), "[a-z ]{0,6}").prop_map(|(head, keyed, tail)| {
                let middle = if keyed { key_shaped() } else { String::new() };
                Value::String(format!("{head}{middle}{tail}"))
            });
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|n| Value::Number(n.into())),
            text,
        ];
        let tree = leaf.prop_recursive(3, 24, 4, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                proptest::collection::btree_map("[a-c]{1,2}", inner, 0..4)
                    .prop_map(|map| Value::Object(map.into_iter().collect())),
            ]
        });
        proptest::collection::btree_map("[a-d]{1,2}", tree, 0..5)
            .prop_map(|map| map.into_iter().collect())
    }

    proptest::proptest! {

        #[test]
        fn the_owned_walk_agrees_with_the_copying_one(payload in payloads()) {
            proptest::prop_assert_eq!(redact(payload.clone()), copied(&payload));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::disallowed_methods,
    reason = "test code: an instrument reads the wall clock"
)]
mod phases {
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use kernel::{Address, Payload, ReadVerdict, Tool, ToolCall, ToolName, ToolOutcome};
    use serde_json::{Map, Value, json};

    use crate::backlog::{Backlog, PollBudget};
    use crate::catalog::Catalog;
    use crate::sandbox::{EchoSandbox, Fuel};
    use crate::tools::{EditTool, ExecSetup, ExecTool, ReadBound, ReadTool, SearchTool};

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_read_phases() {
        let city = tempfile::tempdir().unwrap();
        std::fs::write(city.path().join("history.txt"), history_text()).unwrap();
        let catalog = Arc::new(Mutex::new(Catalog::new()));
        let tool =
            ReadTool::new(city.path(), catalog, everywhere(), Path::new("no-store")).unwrap();
        let read = call("read", json!({ "file_path": "history.txt" }));
        report("read", (0..32).map(|_| timed(&tool, &read)));
    }

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_write_and_edit_phases() {
        let city = tempfile::tempdir().unwrap();
        let work = Address::parse("work").unwrap();
        let domain = kernel::WriteDomain::new(vec![work.clone()]).unwrap();
        std::fs::create_dir_all(city.path().join("work")).unwrap();
        let seen = crate::ReadStamps::default();
        let tool = EditTool::new(
            city.path(),
            work,
            domain,
            crate::tools::EditDesk {
                policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
                seen: seen.clone(),
                locks: Default::default(),
            },
        )
        .unwrap();
        let text = history_text();
        let edit = |path: &str, old: &str, new: &str| {
            call(
                "edit",
                json!({ "file_path": path, "old_string": old, "new_string": new }),
            )
        };
        let writes: Vec<_> = (0..32)
            .map(|at| timed(&tool, &edit(&format!("work/h{at}.txt"), "", &text)))
            .collect();
        report("write", writes);
        let edited = format!("{text}tail\n");
        let edits: Vec<_> = (0..32)
            .map(|at| {
                let path = format!("work/h{at}.txt");
                std::fs::write(city.path().join(&path), &edited).unwrap();
                let stamp = crate::tools::Stamp::of(&city.path().join(&path)).unwrap();
                seen.look(&Address::parse(&path).unwrap(), stamp.clone(), stamp);
                timed(&tool, &edit(&path, "tail", "TAIL"))
            })
            .collect();
        report("edit", edits);
    }

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_exec_phases() {
        let chamber = tempfile::tempdir().unwrap();
        std::fs::write(chamber.path().join("history.txt"), history_text()).unwrap();
        let setup = ExecSetup {
            workdir: chamber.path().to_path_buf(),
            mounts: Vec::new(),
            python_wasm: None,
            shell: crate::Shell::Absent,
            fuel: Fuel(1_000_000),
            env_passthrough: Vec::new(),
            domain: Address::parse("work").unwrap(),
            run: kernel::RunId::from_bytes([1; 16]),
            policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
        };
        let patient = Backlog::with_window(PollBudget::new(6_000, 20));
        let tool = ExecTool::new(setup, Box::new(EchoSandbox::new()), patient).unwrap();
        let line = if cfg!(windows) {
            r#"cmd /C "type history.txt""#
        } else {
            r#"sh -c "cat history.txt""#
        };
        let exec = call("exec", json!({ "command": line }));
        report("exec", (0..8).map(|_| timed(&tool, &exec)));
    }

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_search_phases() {
        let city = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(city.path().join("lab")).unwrap();
        std::fs::write(city.path().join("lab").join("history.txt"), history_text()).unwrap();
        let tool = SearchTool::new(city.path(), everywhere()).unwrap();
        let search = call(
            "search",
            json!({ "pattern": "HMAC_KEY", "output_mode": "content" }),
        );
        report("search", (0..32).map(|_| timed(&tool, &search)));
    }

    fn everywhere() -> ReadBound {
        Arc::new(|_: &Address| ReadVerdict::Open)
    }

    fn call(name: &str, args: Value) -> ToolCall {
        let Value::Object(args) = args else {
            return call(name, Value::Object(Map::new()));
        };
        ToolCall {
            id: "call-1".to_owned(),
            name: ToolName::parse(name).unwrap(),
            args: Payload::new(args).unwrap(),
        }
    }

    fn timed(tool: &dyn Tool, call: &ToolCall) -> (Duration, ToolOutcome) {
        let started = Instant::now();
        let outcome = tool.invoke(call).unwrap();
        (started.elapsed(), outcome)
    }

    fn history_text() -> String {
        let hex = "9f3c1a7e5b20d48c6f1ea7b3905d2ce81f64b07a".repeat(2);
        let token = ["kJ8vQ2xR9m", "W4nZ7pL3sT", "6yB1cD5fG0", "hN8aE2iU4o"].concat();
        let line = format!(
            "prev={hex} oid={} HMAC_KEY={} note: see {token} for the run, and nothing else\n",
            hex.get(..40).unwrap(),
            hex.get(3..43).unwrap()
        );
        line.repeat(400)
    }

    fn report(tool: &str, calls: impl IntoIterator<Item = (Duration, ToolOutcome)>) {
        let (mut rounds, mut found) = (0u32, 0u32);
        let (mut call, mut scan) = (Duration::ZERO, Duration::ZERO);
        for (took, outcome) in calls {
            let started = Instant::now();
            let (_redacted, hits) = super::redact(outcome.result.into_map());
            scan = scan.saturating_add(started.elapsed());
            call = call.saturating_add(took);
            rounds = rounds.saturating_add(1);
            found = found.saturating_add(hits);
        }
        println!(
            "instrument_tool_phases tool={tool} rounds={rounds} spans={found} call_us={} secret_scan_us={}",
            call.as_micros(),
            scan.as_micros()
        );
    }
}
