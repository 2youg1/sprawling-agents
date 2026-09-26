// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::ledger::chain_hash;
use kernel::{EventDraft, EventKind, GENESIS_PREV, Payload, RunId, TimeMs};

fn genesis_line() -> Vec<u8> {
    let draft = EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(0),
        who: "city".to_string(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: Payload::empty(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::FIRST, GENESIS_PREV)
        .canonical_line()
        .unwrap()
}

fn future_line(prev: &[u8], seq: u64, ig: bool) -> Vec<u8> {
    let ig_part = if ig { ",\"ig\":true" } else { "" };
    format!(
        "{{\"v\":1,\"run\":\"00000000-0000-0000-0000-000000000000\",\"seq\":{seq},\
         \"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\
         \"data\":{{}}{ig_part}}}",
        chain_hash(prev)
    )
    .into_bytes()
}

#[test]
fn ig_true_unknown_kind_is_skipped_but_chained() {
    let first = genesis_line();
    let second = future_line(&first, 1, true);
    let verified = verify_lines(vec![first, second]).unwrap();
    assert_eq!(verified.lines().len(), 2);
    assert!(matches!(
        verified.lines()[1],
        VerifiedLine::IgnoredUnknown { seq } if seq == Seq::new(1)
    ));
    assert_eq!(verified.tail_seq(), Some(Seq::new(1)));
}

#[test]
fn unknown_kind_without_ig_speaks_direction() {
    let first = genesis_line();
    let second = future_line(&first, 1, false);
    let err = verify_lines(vec![first, second]).unwrap_err();
    assert_eq!(err.code(), &AxCode::LogVersionUnsupported);
}

fn next_line(prev: &[u8], seq: u64) -> Vec<u8> {
    let draft = EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(0),
        who: "city".to_string(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: Payload::empty(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), chain_hash(prev))
        .canonical_line()
        .unwrap()
}

#[test]
fn the_index_covers_exactly_the_history_the_fold_saw() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger-00000000000000000000.jsonl");
    let first = genesis_line();
    let second = next_line(&first, 1);
    let late = [
        next_line(&second, 2),
        b"
"
        .to_vec(),
    ]
    .concat();
    std::fs::write(
        &path,
        [
            first,
            b"
"
            .to_vec(),
            second,
            b"
"
            .to_vec(),
        ]
        .concat(),
    )
    .unwrap();
    let mut folded = Vec::new();
    let index = fold_ledger_dir(dir.path(), |record| {
        folded.push(record.seq());
        if record.seq() == Seq::new(1) {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            file.write_all(&late).unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        (folded, index.tail_seq(), index.len()),
        (vec![Seq::FIRST, Seq::new(1)], Some(Seq::new(1)), 2)
    );
}

#[test]
fn a_fold_reads_the_history_line_by_line() {
    let dir = tempfile::tempdir().unwrap();
    let first = genesis_line();
    let second = future_line(&first, 1, false);
    let segment = [
        first,
        b"
"
        .to_vec(),
        second,
        b"
"
        .to_vec(),
    ]
    .concat();
    std::fs::write(
        dir.path().join("ledger-00000000000000000000.jsonl"),
        segment,
    )
    .unwrap();
    let mut folded = Vec::new();
    let refused = fold_ledger_dir(dir.path(), |record| {
        folded.push(record.seq());
        Ok(())
    })
    .err()
    .unwrap();
    assert_eq!(
        (folded, refused.code()),
        (vec![Seq::FIRST], &AxCode::LogVersionUnsupported)
    );
}

#[test]
fn higher_v_is_refused_before_anything_else() {
    let line = br#"{"v":2,"seq":0,"prev":"0000000000000000000000000000000000000000000000000000000000000000","kind":"x"}"#;
    let err = verify_lines(vec![line.to_vec()]).unwrap_err();
    assert_eq!(err.code(), &AxCode::LogVersionUnsupported);
}

#[test]
fn a15_rebuild_matches_the_recorded_segment_hashes() {
    use crate::prefix::{PrefixPlan, SegmentCaps, SourceDoc, build_prefix};
    use std::collections::BTreeMap;
    let long_job = "x".repeat(3000);
    let docs: BTreeMap<&str, String> = [
        ("city.md", "city rules, long enough to matter".to_owned()),
        ("b/building.md", "building rules".to_owned()),
        ("b/urbanite.md", "who i am".to_owned()),
        ("b/room/job.md", long_job),
    ]
    .into_iter()
    .collect();
    let source = |addr: &str| SourceDoc {
        addr: Address::parse(addr).unwrap(),
        bytes: docs.get(addr).map(|s| s.as_bytes().to_vec()),
    };
    let plan = PrefixPlan {
        city: vec![source("city.md")],
        building: vec![source("b/building.md")],
        resident: vec![source("b/urbanite.md")],
        run: vec![source("b/room/job.md")], // truncates under the cap
        caps: SegmentCaps::startup_default(),
    };
    let prefix = build_prefix(plan).unwrap();
    let payload = serde_json::to_value(
        prefix
            .prompt_payload(&crate::prefix::BreakpointPlan::for_conversation(&[]))
            .unwrap(),
    )
    .unwrap();
    let resolver = |addr: &Address| docs.get(addr.as_str()).map(|s| s.as_bytes().to_vec());
    let rebuilt = rebuild_prefix(&payload, &resolver).unwrap();
    assert_eq!(
        rebuilt,
        prefix.segment_hashes(),
        "A15: offline rebuild agrees"
    );
    // A drifted source document is caught, not silently accepted.
    let drifted = |addr: &Address| {
        if addr.as_str() == "city.md" {
            // Same length, different bytes: only the hash can tell.
            Some(b"city rules, long enough to differ".to_vec())
        } else {
            docs.get(addr.as_str()).map(|s| s.as_bytes().to_vec())
        }
    };
    let err = rebuild_prefix(&payload, &drifted).unwrap_err();
    assert_eq!(err.code(), &AxCode::CasCorrupt);
    // A shorter document is caught too, on the span check.
    let shorter = |addr: &Address| {
        if addr.as_str() == "city.md" {
            Some(b"tiny".to_vec())
        } else {
            docs.get(addr.as_str()).map(|s| s.as_bytes().to_vec())
        }
    };
    assert!(rebuild_prefix(&payload, &shorter).is_err());
}

#[test]
fn dangling_tool_calls_are_detected_and_repairable() {
    use kernel::Ledger as _;
    struct Mem {
        lines: Vec<Vec<u8>>,
        next: Seq,
        prev: B3Hash,
    }
    impl kernel::Ledger for Mem {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            let record = EventRecord::from_draft(draft, self.next, self.prev);
            let line = record.canonical_line()?;
            self.prev = chain_hash(&line);
            self.next = self.next.next()?;
            let echo = record.to_ref();
            self.lines.push(line);
            Ok(echo)
        }
    }
    let mut mem = Mem {
        lines: Vec::new(),
        next: Seq::FIRST,
        prev: GENESIS_PREV,
    };
    let run = RunId::parse("0198f6a2-7c4a-7bbb-9d1e-00000000000a").unwrap();
    let draft = |kind: EventKind, data: Payload| EventDraft {
        run,
        t: TimeMs::new(1),
        who: "r".to_owned(),
        addr: None,
        kind,
        data,
        ig: false,
    };
    let mut call_data = serde_json::Map::new();
    call_data.insert(
        "id".to_owned(),
        serde_json::Value::String("call-7".to_owned()),
    );
    call_data.insert(
        "name".to_owned(),
        serde_json::Value::String("exec".to_owned()),
    );
    // Every `tool_called` the city writes carries its arguments, and
    // the repair now reads the line through the struct that writes it.
    call_data.insert(
        "args".to_owned(),
        serde_json::Value::Object(serde_json::Map::new()),
    );
    mem.append(draft(
        EventKind::ToolCalled,
        Payload::new(call_data.clone()).unwrap(),
    ))
    .unwrap();
    // Crash here: no tool_result follows.
    let verified = verify_lines(mem.lines.clone()).unwrap();
    let dangling = dangling_tool_calls(&verified);
    assert_eq!(dangling.len(), 1);
    let (_, seq) = dangling[0];
    let record = match &verified.lines()[0] {
        VerifiedLine::Known { record, .. } => record.clone(),
        other => panic!("expected a known record, got {other:?}"),
    };
    assert_eq!(record.seq(), seq);
    let repair = outcome_unknown_draft(&record, TimeMs::new(2)).unwrap();
    mem.append(repair).unwrap();
    let verified = verify_lines(mem.lines.clone()).unwrap();
    assert!(
        dangling_tool_calls(&verified).is_empty(),
        "repair closes the account"
    );
    let repaired: serde_json::Value = serde_json::from_slice(&mem.lines[1]).unwrap();
    assert_eq!(repaired["data"]["error"]["code"], "E_TOOL_OUTCOME_UNKNOWN");
    assert_eq!(repaired["data"]["tool_use_id"], "call-7");
}

#[test]
fn a_payload_without_segments_names_the_event_that_carries_them() {
    let err = rebuild_prefix(&serde_json::json!({}), &|_| None).unwrap_err();
    let expected = AxError::failure(
        AxCode::InvalidArgs,
        "rebuild prefix",
        "payload has no segments",
    )
    .with_recovery(format!(
        "replay a run whose `{}` line carries `segments`; a hand-written line cannot be rebuilt",
        serde_json::to_value(EventKind::PromptAssembled)
            .unwrap()
            .as_str()
            .unwrap()
    ));
    assert_eq!(err, expected);
}
