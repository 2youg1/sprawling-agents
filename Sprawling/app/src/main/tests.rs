// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::data::verified_chain;
use super::{CLIENT_BUNDLE_DIR, CLIENT_COMPLETE, CLIENT_FILES};

#[test]
fn a_place_holding_no_ledger_is_refused_rather_than_verified() {
    let dir = tempfile::tempdir().unwrap();
    let err = verified_chain(dir.path()).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::PathNotFound);
    assert!(
        err.recovery().contains("ledger directory itself"),
        "the recovery names the mistake that was actually made: {}",
        err.recovery()
    );
}

#[test]
fn replay_names_the_lines_it_verified_and_the_tail_seq() {
    let dir = tempfile::tempdir().unwrap();
    sprawling::assembly::init_city(dir.path()).unwrap();
    let layout = kernel::layout::CityLayout::new(dir.path());
    let ledger_dir = layout.ledger();

    let cas = storage::Cas::open(&layout.cas()).unwrap();
    let shipped = city::Library::scan(dir.path(), None, dir.path(), &|digest| {
        cas.get(digest).map_err(storage::StorageError::into_ax)
    })
    .unwrap()
    .all()
    .len();
    let lines = 3 + shipped;
    assert_eq!(
        verified_chain(&ledger_dir),
        Ok(format!(
            "chain verified: {lines} line(s), tail seq {}",
            lines - 1
        ))
    );

    let segment = storage::ledger_segments_at(&ledger_dir).unwrap().remove(0);
    let mut bytes = std::fs::read(&segment).unwrap();
    let third = bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(1)
        .map(|(at, _)| at + 1)
        .unwrap();
    let digit = bytes[third..]
        .windows(8)
        .position(|held| held == b"\"prev\":\"")
        .map(|key| third + key + 8)
        .unwrap();
    bytes[digit] = if bytes[digit] == b'0' { b'1' } else { b'0' };
    std::fs::write(&segment, bytes).unwrap();
    assert_eq!(
        verified_chain(&ledger_dir)
            .map_err(|refused| (*refused.code(), refused.subject().to_owned())),
        Err((kernel::AxCode::CasCorrupt, "line 3".to_owned()))
    );
}

#[test]
fn embedded_client_table_is_present_and_marked() {
    let index = CLIENT_FILES
        .iter()
        .find(|f| f.path == "index.html")
        .expect("the page shell is always embedded");
    assert!(!index.gz.is_empty());
    if CLIENT_COMPLETE {
        assert!(
            CLIENT_FILES
                .iter()
                .any(|f| f.path.starts_with("assets/") && f.path.ends_with(".js")),
            "a complete client carries a script"
        );
        assert!(
            CLIENT_FILES
                .iter()
                .any(|f| f.path.starts_with("assets/") && f.path.ends_with(".css")),
            "a complete client carries a stylesheet"
        );
    }
}

#[test]
fn the_embedded_client_is_the_bundle_the_workspace_built() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLIENT_BUNDLE_DIR);
    let mut on_disk = Vec::new();
    files_under(&dist, &dist, &mut on_disk);
    on_disk.sort();
    let built = on_disk.iter().any(|path| path == "index.html")
        && on_disk.iter().any(|path| path.starts_with("assets/"));
    let embedded: Vec<String> = CLIENT_FILES
        .iter()
        .filter(|_| CLIENT_COMPLETE)
        .map(|file| file.path.to_owned())
        .collect();
    let expected = if built { on_disk } else { Vec::new() };
    assert_eq!(
        embedded,
        expected,
        "the binary embeds exactly the bundle at {}",
        dist.display()
    );
}

fn files_under(root: &std::path::Path, dir: &std::path::Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(root, &path, found);
        } else {
            let relative = path.strip_prefix(root).unwrap();
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn each_way_a_call_ends_has_its_own_exit_code() {
    use super::calling::call;
    use super::exit::Exit;
    const CITY_VIEW: &str = r#"{"ask":{"ask_id":1,"query":"city_view"}}"#;
    const NO_SUCH_QUERY: &str = r#"{"ask":{"ask_id":1,"query":"no_such"}}"#;

    let vacant = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    let words = |line: &[&str]| line.iter().map(ToString::to_string).collect::<Vec<_>>();
    let table = [
        (words(&["call"]), Exit::Line),
        (words(&["call", "{not json", "--at", &vacant]), Exit::Line),
        (words(&["call", NO_SUCH_QUERY, "--at", &vacant]), Exit::Line),
        (
            words(&["call", CITY_VIEW, "--quiet-ms", "soon"]),
            Exit::Line,
        ),
        (
            words(&["call", CITY_VIEW, "--until", "no_such_kind"]),
            Exit::Line,
        ),
        (words(&["call", CITY_VIEW, "--at", &vacant]), Exit::NoCity),
    ];
    let observed = table
        .iter()
        .map(|(line, _)| (line.clone(), call(line)))
        .collect::<Vec<_>>();
    assert_eq!(observed, table.to_vec());
}

#[test]
fn a_refusal_reads_the_same_to_a_person_and_to_a_program() {
    use super::refusal::{Form, written};
    use kernel::{AxCode, AxError};
    let err = AxError::failure(AxCode::ToolUnknown, "call tool", "grep")
        .with_nearby(vec!["exec".into(), "edit".into()])
        .with_recovery("use one of the nearby tools");
    let human = written(&err, Form::Human);
    let json = written(&err, Form::Json);
    let back: AxError = serde_json::from_str(&json).unwrap();
    assert_eq!(
        (human.lines().last(), json.lines().count(), back),
        (Some("nearby: exec, edit"), 1, err)
    );
}

#[test]
fn a_supervised_child_is_forwarded_the_served_flags_with_their_values() {
    let line = [
        "up",
        "city",
        "--log",
        "debug",
        "--web-dir",
        "client/dist",
        "--supervise",
        "--no-open",
        "--console",
    ]
    .map(str::to_owned);
    assert_eq!(
        super::verbs::forwarded(&line),
        ["--log", "debug", "--web-dir", "client/dist"].map(str::to_owned)
    );
}

#[test]
fn whose_trace_names_the_run_its_calls_and_who_else_called_in_the_building() {
    use kernel::{EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
    use serde_json::json;

    let first = RunId::from_bytes([1; 16]);
    let second = RunId::from_bytes([2; 16]);
    let oid = |n: u8| format!("{n:02x}").repeat(20);
    let script = vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (first, Some("lab/room1"), EventKind::RunStarted, json!({})),
        (second, Some("lab/room2"), EventKind::RunStarted, json!({})),
        (
            first,
            Some("lab/room1"),
            EventKind::ModelCalled,
            json!({"segments": [], "model": "m"}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::ToolCalled,
            json!({"id": "c1", "name": "read", "args": {}, "subject": "src/a.rs", "effect": "read"}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::ToolResult,
            json!({"tool_use_id": "c1", "name": "read", "result": "fn main() {}\n"}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::CheckpointCommitted,
            json!({"oid": oid(1)}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::ToolCalled,
            json!({"id": "c2", "name": "edit", "args": {}, "subject": "src/b.rs",
                   "effect": {"write": {"domain": "lab"}}}),
        ),
        (
            second,
            Some("lab/room2"),
            EventKind::ToolCalled,
            json!({"id": "d1", "name": "exec", "args": {}, "effect": "egress"}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::ToolResult,
            json!({"tool_use_id": "c2", "name": "edit", "result": "done\nand more"}),
        ),
        (
            first,
            Some("lab/room1"),
            EventKind::CheckpointCommitted,
            json!({"oid": oid(3)}),
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let ledger_dir = kernel::layout::CityLayout::new(dir.path()).ledger();
    std::fs::create_dir_all(&ledger_dir).unwrap();
    let mut prev = kernel::GENESIS_PREV;
    let mut blob = Vec::new();
    for (seq, (run, addr, kind, data)) in (0u64..).zip(script) {
        let draft = EventDraft {
            run,
            t: TimeMs::new(1_778_749_200_000 + seq * 1_000),
            who: "tester".to_owned(),
            addr: addr.map(|raw| kernel::Address::parse(raw).unwrap()),
            kind,
            data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
            ig: false,
        };
        let line = EventRecord::from_draft(draft, Seq::new(seq), prev)
            .canonical_line()
            .unwrap();
        prev = kernel::ledger::chain_hash(&line);
        blob.extend_from_slice(&line);
        blob.push(b'\n');
    }
    std::fs::write(ledger_dir.join("ledger-00000000000000000000.jsonl"), blob).unwrap();

    let traced = accounting::trace::trace(dir.path(), kernel::GitOid::parse(&oid(3)).unwrap())
        .unwrap()
        .map(|traced| {
            let mut out = Vec::new();
            super::whose::write_trace(&traced, &mut out).unwrap();
            String::from_utf8(out).unwrap()
        });
    let expected = format!(
        "run     {first}\n\
         actor   lab/room1 (session room1)\n\
         model   not recorded (effort none)\n\
         ledger  seq 10\n\
         previous {} at seq 6\n\
         span    after seq 6, before seq 10\n\
         call    seq 7  2026-05-14T09:00:07Z  edit  {{\"write\":{{\"domain\":\"lab\"}}}}  answered  src/b.rs\n\
         \x20       > done\n\
         nearby  {second} at lab/room2: 1 call(s)\n\
         note    the calls are candidates; a nearby run may have written in the same span\n",
        oid(1)
    );
    assert_eq!(traced, Some(expected));
}
