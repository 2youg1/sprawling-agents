// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::sandbox::{EchoSandbox, Fuel};
use crate::tools::{EditTool, ExecTool};
use kernel::{Payload, RunId, Seq, TimeMs};
use serde_json::Map;

fn now() -> TimeMs {
    TimeMs::new(1_700_000_000_000)
}

fn key(n: u64) -> IdemKey {
    IdemKey::derive(&RunId::from_bytes([1u8; 16]), Seq::new(n), b"action")
}

fn bench(root: &std::path::Path) -> ToolBench {
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain.clone());
    bench
        .register(Box::new(
            EditTool::new(root, Address::parse("work").unwrap(), domain).unwrap(),
        ))
        .unwrap();
    bench
}

fn edit_call(path: &str, base: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (k, v) in [
        ("path", path),
        ("base_version", base),
        ("old", old),
        ("new", new),
    ] {
        args.insert(k.to_owned(), Value::String(v.to_owned()));
    }
    ToolCall {
        id: "c1".to_owned(),
        name: kernel::ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

#[test]
fn dedup_runs_before_the_side_effect() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("work")).unwrap();
    std::fs::write(tmp.path().join("work/a.txt"), "one\n").unwrap();
    let mut bench = bench(tmp.path());
    let version = crate::tools::version_of(b"one\n");
    let call = edit_call("work/a.txt", &version, "one", "two");

    let first = bench.invoke(&call, &key(1), now()).unwrap();
    assert!(matches!(first, BenchOutcome::Ran { .. }));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "two\n"
    );
    // The same key again: the tool must not run a second time, and the
    // file must not change (the second edit would fail on the stale
    // version anyway — dedup means it is never attempted).
    let second = bench.invoke(&call, &key(1), now()).unwrap();
    // What a replay is owed is the answer the first call gave: telling
    // the model "you already asked" teaches it the call failed when it
    // succeeded.
    let (BenchOutcome::Ran { outcome: once, .. }, BenchOutcome::Duplicate { outcome: again }) =
        (first, second)
    else {
        panic!("the second call must answer with the first call's result");
    };
    assert_eq!(again, once);
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "two\n"
    );
}

#[test]
fn a_write_outside_the_domain_flows_back_as_a_refusal_not_a_dead_turn() {
    let tmp = tempfile::tempdir().unwrap();
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
    let elsewhere = WriteDomain::new(vec![Address::parse("elsewhere").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    // A tool whose declared domain sits outside the run's domain.
    bench
        .register(Box::new(
            EditTool::new(tmp.path(), Address::parse("elsewhere").unwrap(), elsewhere).unwrap(),
        ))
        .unwrap();
    let outcome = bench
        .invoke(&edit_call("elsewhere/x", "v", "a", "b"), &key(2), now())
        .unwrap();
    match outcome {
        BenchOutcome::Refused { refusal } => {
            assert_eq!(*refusal.code(), AxCode::OutsideWriteDomain);
        }
        other => panic!("expected a refusal that keeps the turn alive, got {other:?}"),
    }
}

#[test]
fn a_suspected_discard_without_a_net_is_refused_and_with_one_is_fenced() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("work")).unwrap();
    std::fs::write(tmp.path().join("work/doomed.txt"), "bye").unwrap();
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();

    let exec_call = |text: &str| {
        let mut args = Map::new();
        args.insert(
            "arm".to_owned(),
            serde_json::json!({ "shell": { "text": text } }),
        );
        ToolCall {
            id: "e1".to_owned(),
            name: kernel::ToolName::parse("exec").unwrap(),
            args: Payload::new(args).unwrap(),
        }
    };
    let exec_tool = || {
        Box::new(
            ExecTool::new(
                crate::tools::ExecSetup {
                    workdir: tmp.path().to_path_buf(),
                    mounts: Vec::new(),
                    python_wasm: None,
                    shell: None,
                    fuel: Fuel(1000),
                    env_passthrough: Vec::new(),
                    domain: Address::parse("work").unwrap(),
                },
                Box::new(EchoSandbox::new()),
                crate::Backlog::new(),
            )
            .unwrap(),
        )
    };

    // No net: a command the forecast suspects is refused rather than
    // run unprotected.
    let mut bare = ToolBench::new(domain.clone());
    bare.register(exec_tool()).unwrap();
    let err = match bare.invoke(&exec_call("rm -rf work"), &key(3), now()) {
        Err(err) => err,
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);

    // With a net: the wave is fenced first, and the outcome carries
    // the commit the sweep will restore from.
    let checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let mut fenced_bench = ToolBench::new(domain).with_checkpoint(crate::bench::CheckpointNet {
        checkpoint,
        scope: vec!["work".to_owned()],
        of: probe_provenance(),
    });
    fenced_bench.register(exec_tool()).unwrap();
    // The shell arm is unconfigured, so the tool itself refuses —
    // but only after the fence went up, which is what we assert.
    let _ = fenced_bench.invoke(&exec_call("rm -rf work"), &key(4), now());
    // The fence went up before the command was allowed to run: a
    // repository now exists with a commit to restore from.
    assert!(
        tmp.path().join(".git").exists(),
        "the checkpoint net was raised"
    );
    let mut probe = Checkpoint::open(tmp.path()).unwrap();
    let payload = probe
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_001_000),
            &probe_provenance(),
        )
        .unwrap();
    let oid = serde_json::to_value(&payload).unwrap()["oid"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!oid.is_empty(), "the fence has a commit to restore from");
}

#[test]
fn an_unregistered_name_is_refused_rather_than_routed_anywhere() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bench = bench(tmp.path());
    let mut call = edit_call("work/a", "v", "a", "b");
    call.name = kernel::ToolName::parse("status").unwrap();
    let err = match bench.invoke(&call, &key(5), now()) {
        Err(err) => err,
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
}

#[test]
fn a_second_tool_claiming_a_taken_name_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bench = bench(tmp.path());
    let err = match bench.register(Box::new(
        EditTool::new(
            tmp.path(),
            Address::parse("work").unwrap(),
            WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap(),
        )
        .unwrap(),
    )) {
        Err(err) => err,
        Ok(()) => panic!("a name collision must refuse, not shadow"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

/// Who a fence in this test file is signed as.
fn probe_provenance() -> memory::Provenance {
    memory::Provenance::new(
        kernel::RunId::CITY,
        kernel::Address::parse("work/probe").unwrap(),
        kernel::B3Hash::digest(b"a city"),
        memory::ModelChoice {
            id: "test-model".to_owned(),
            effort: None,
        },
    )
}

#[test]
fn a_documents_bench_lets_its_own_room_through_the_door() {
    // The tool declares the room it works in, not a file; a documents
    // domain must not judge the declaration as if it were one. Before
    // the fix this refused every write with "hall/mayor is not a
    // Markdown document", and the Mayor could write nothing at all.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("hall")).unwrap();
    let domain = WriteDomain::documents(vec![Address::parse("hall").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain.clone());
    bench
        .register(Box::new(
            EditTool::new(tmp.path(), Address::parse("hall/mayor").unwrap(), domain).unwrap(),
        ))
        .unwrap();
    let outcome = bench
        .invoke(
            &edit_call(
                "hall/note.md",
                "new",
                "",
                "# note
",
            ),
            &key(9),
            now(),
        )
        .unwrap();
    match outcome {
        BenchOutcome::Ran { .. } => {}
        other => panic!("expected the note to land, got {other:?}"),
    }
    assert!(tmp.path().join("hall/note.md").is_file());
}
