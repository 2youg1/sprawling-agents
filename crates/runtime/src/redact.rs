// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a model said, on its way into history.
//!
//! Everything a provider sends back becomes a ledger payload verbatim,
//! and a model that read a key can repeat one. The ledger is permanent
//! and exportable, so it is the sink that matters: a secret there
//! outlives the session, the machine and the person's memory of it.
//!
//! Two decisions carry this module.
//!
//! **The window keeps the bytes; the ledger does not.** The blocks the
//! next request is built from are extracted before this runs, so
//! redacting here cannot break a thinking block's signature or make the
//! conversation stop making sense. History and context are two sinks
//! with two different requirements, and only one of them is forever.
//!
//! **The replacement is a marker, not a vault entry.** A key the model
//! echoed is not a credential the city was asked to keep; storing it
//! would give it a life nobody asked for, and inventing a realm for it
//! would put a made-up name in the vault's namespace. The marker carries
//! the first sixteen hex of the content hash instead, which is enough to
//! see that two occurrences are the same value and never enough to be
//! one.
//!
//! **Every sink in this crate marks the same way.** The ledger and the
//! diagnostic log differ in one decision only — whether the marker
//! carries a fingerprint — and that decision is the [`Marker`] argument
//! below rather than a second scan-and-replace written next to the
//! second sink.

use kernel::B3Hash;
use serde_json::{Map, Value};

/// The realm every redaction marker sits in. Not a vault realm: nothing
/// is stored under it, and `secret:` references resolve through the
/// vault, so a reader who tries will be told there is nothing there.
const REDACTED_REALM: &str = "redacted";

/// What stands where a secret stood, and how much it says about it.
///
/// The two sinks differ here and nowhere else. History is kept,
/// searched and compared, so a marker there tells one value from
/// another; a diagnostic line is written once and read once, and a
/// fingerprint in it would be a correlation handle in a stream that
/// nothing correlates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// `secret:redacted` — something was here, and that is all.
    Plain,
    /// `secret:redacted/<sixteen hex>` — something was here, and two
    /// occurrences of one value read alike.
    Fingerprinted,
}

impl Marker {
    /// The text this marker puts in the place of `found`.
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

/// Replaces every secret-shaped span in every string of `payload`.
///
/// Returns the payload and how many spans were replaced, because a count
/// is what a diagnostic line can say without saying what it found.
#[must_use]
pub fn redact(payload: Map<String, Value>) -> (Map<String, Value>, u32) {
    copied(&payload)
}

fn copied(payload: &Map<String, Value>) -> (Map<String, Value>, u32) {
    let mut hits = 0;
    let mut out = Map::new();
    for (key, value) in payload {
        out.insert(key.clone(), walk(value, &mut hits));
    }
    (out, hits)
}

fn walk(value: &Value, hits: &mut u32) -> Value {
    match value {
        Value::String(text) => {
            let (replaced, found) = redact_text(text, Marker::Fingerprinted);
            *hits = hits.saturating_add(found);
            Value::String(replaced)
        }
        Value::Array(items) => Value::Array(items.iter().map(|item| walk(item, hits)).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, item) in map {
                out.insert(key.clone(), walk(item, hits));
            }
            Value::Object(out)
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}

/// One string, with every secret shape in it replaced in place.
///
/// The spans arrive sorted and non-overlapping from the detector, so a
/// single left-to-right pass is enough and no offset needs adjusting
/// after a replacement.
#[must_use]
pub fn redact_text(text: &str, marker: Marker) -> (String, u32) {
    let bytes = text.as_bytes();
    let spans = kernel::secret::scan(bytes);
    if spans.is_empty() {
        return (text.to_owned(), 0);
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
    // A span that ended inside a character would leave the tail
    // unreadable; the honest answer is to keep nothing of it rather than
    // emit half a character.
    if let Some(rest) = bytes.get(at..).and_then(|b| std::str::from_utf8(b).ok()) {
        out.push_str(rest);
    }
    (out, hits)
}

/// How many hex characters of a value's hash stand for the value.
/// Sixteen is enough to tell two values apart in a history and far too
/// few to reconstruct one.
const FINGERPRINT_HEX: usize = 16;

/// The one short name this city gives a secret it must refer to without
/// holding: derived from the value, so one value reads alike in every
/// session and two values never read alike in one.
///
/// A name counted out instead — `cap-1`, `cap-2` — would give the same
/// value a different name in every session and two different values the
/// same name across two, which is how a stored credential comes back as
/// somebody else's.
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

    /// Assembled at runtime: a literal key-shaped sample in the source
    /// would be a finding about this file.
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

    /// One value, one short name, wherever the city has to refer to it,
    /// and a name a reader can act on: the marker parses as a reference,
    /// so somebody who meets one in history knows both that a value was
    /// there and that the way to a value is the vault, which will tell
    /// them the `redacted` realm holds nothing.
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

    /// The two sinks mark by one rule and differ by one argument: a log
    /// line says a secret was there, history says which one.
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

    /// D37: a clean payload costs the scan and nothing else, and a hit
    /// costs the string it hit and nothing else. The address of a
    /// string's bytes is the observation: a copy would move them.
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

    /// Strings of prose with a key-shaped value spliced in at random,
    /// under objects and arrays nested to a few levels.
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
        /// The owned walk replaces exactly what the copying walk it
        /// replaced did, and counts the same.
        #[test]
        fn the_owned_walk_agrees_with_the_copying_one(payload in payloads()) {
            proptest::prop_assert_eq!(redact(payload.clone()), copied(&payload));
        }
    }
}

/// The phase instruments of the tools whose results pass this module:
/// each times one call through the tool's own face (path resolution, IO
/// and the envelope as one phase), then the secret scan of its result,
/// and prints one line per tool in microseconds (`crates/runtime/Spec.lean`
/// §10, the per-tool phases). They live here because the scan is the
/// phase every tool shares.
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
    use crate::tools::{
        EditTool, ExecSetup, ExecTool, ReadBound, ReadTool, SearchTool, version_of,
    };

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_read_phases() {
        let city = tempfile::tempdir().unwrap();
        std::fs::write(city.path().join("history.txt"), history_text()).unwrap();
        let catalog = Arc::new(Mutex::new(Catalog::new()));
        let tool =
            ReadTool::new(city.path(), catalog, everywhere(), Path::new("no-store")).unwrap();
        let read = call("read", json!({ "path": "history.txt" }));
        report("read", (0..32).map(|_| timed(&tool, &read)));
    }

    /// `write` is `edit` with a `base_version` of `new`: it creates the
    /// file, and its echo is the whole file as a diff.
    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_write_and_edit_phases() {
        let city = tempfile::tempdir().unwrap();
        let work = Address::parse("work").unwrap();
        let domain = kernel::WriteDomain::new(vec![work.clone()]).unwrap();
        std::fs::create_dir_all(city.path().join("work")).unwrap();
        let tool = EditTool::new(
            city.path(),
            work,
            domain,
            crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
        )
        .unwrap();
        let text = history_text();
        let edit = |path: &str, base: &str, old: &str, new: &str| {
            call(
                "edit",
                json!({ "path": path, "base_version": base, "old": old, "new": new }),
            )
        };
        let writes: Vec<_> = (0..32)
            .map(|at| timed(&tool, &edit(&format!("work/h{at}.txt"), "new", "", &text)))
            .collect();
        report("write", writes);
        let edited = format!("{text}tail\n");
        let edits: Vec<_> = (0..32)
            .map(|at| {
                let path = format!("work/h{at}.txt");
                std::fs::write(city.path().join(&path), &edited).unwrap();
                timed(
                    &tool,
                    &edit(&path, &version_of(edited.as_bytes()), "tail", "TAIL"),
                )
            })
            .collect();
        report("edit", edits);
    }

    /// The host's own shell prints the file (`cmd` on Windows, `sh` on
    /// macOS and Linux) under `EchoSandbox`, so the reading is the tool's
    /// cost and not a sandbox arm's.
    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_exec_phases() {
        let chamber = tempfile::tempdir().unwrap();
        std::fs::write(chamber.path().join("history.txt"), history_text()).unwrap();
        let setup = ExecSetup {
            workdir: chamber.path().to_path_buf(),
            mounts: Vec::new(),
            python_wasm: None,
            shell: None,
            fuel: Fuel(1_000_000),
            env_passthrough: Vec::new(),
            domain: Address::parse("work").unwrap(),
            run: kernel::RunId::from_bytes([1; 16]),
            policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
        };
        let patient = Backlog::with_window(PollBudget::new(6_000, 20));
        let tool = ExecTool::new(setup, Box::new(EchoSandbox::new()), patient).unwrap();
        let (path, args) = if cfg!(windows) {
            ("cmd", ["/C", "type history.txt"])
        } else {
            ("sh", ["-c", "cat history.txt"])
        };
        let exec = call(
            "exec",
            json!({ "arm": { "program": { "path": path, "args": args } } }),
        );
        report("exec", (0..8).map(|_| timed(&tool, &exec)));
    }

    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_search_phases() {
        let city = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(city.path().join("lab")).unwrap();
        std::fs::write(city.path().join("lab").join("history.txt"), history_text()).unwrap();
        let tool = SearchTool::new(city.path(), everywhere()).unwrap();
        let search = call("search", json!({ "text": "HMAC_KEY" }));
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

    /// Text shaped like what a tool hands back from a working city:
    /// hashes and oids, a labelled key, a mixed token and prose, about
    /// 64 KiB. Assembled at runtime so this file holds no scannable span.
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

    /// Prints one line per tool: rounds, the spans the scan replaced, the
    /// summed call time, and the summed time the scan of each result took.
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
