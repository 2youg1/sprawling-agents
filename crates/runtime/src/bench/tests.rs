// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::sandbox::{EchoSandbox, Fuel};
use crate::tools::{EditTool, ExecTool};
use kernel::{ApprovalId, Payload, RunId, Seq, TimeMs};
use serde_json::Map;

fn ctx() -> GateContext {
    GateContext {
        actor: "resident".to_owned(),
        now: TimeMs::new(1_700_000_000_000),
        item_id: ApprovalId::new("item-1").expect("id"),
    }
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

    let first = bench.invoke(&call, &key(1), &ctx()).unwrap();
    assert!(matches!(first, BenchOutcome::Ran { .. }));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "two\n"
    );
    // The same key again: the tool must not run a second time, and
    // the file must not change (the second edit would fail on the
    // stale version anyway — dedup means it is never attempted).
    let second = bench.invoke(&call, &key(1), &ctx()).unwrap();
    assert!(matches!(second, BenchOutcome::Duplicate));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "two\n"
    );
}

/// A tool that declares `Spawn` and nothing else. The bench's job
/// here is the door, not the tool, so the tool does as little as a
/// tool can.
struct SpawnTool(kernel::ToolMeta);

impl SpawnTool {
    fn new() -> SpawnTool {
        SpawnTool(kernel::ToolMeta {
            name: kernel::ToolName::parse("delegate").unwrap(),
            disclosure: "hand work down".to_owned(),
            params: Payload::empty(),
            effect: Effect::Spawn,
            cost_tier: kernel::CostTier::Heavy,
            timeout: None,
            render: kernel::RenderIntent::Generic,
            temporal: kernel::Temporal::Timeless,
        })
    }
}

impl Tool for SpawnTool {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.0
    }

    fn invoke(&mut self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

fn spawn_call() -> ToolCall {
    let mut args = Map::new();
    args.insert("room".to_owned(), Value::String("work/helper".to_owned()));
    ToolCall {
        id: "c1".to_owned(),
        name: kernel::ToolName::parse("delegate").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn spawn_bench() -> ToolBench {
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain).for_job(
        Address::parse("work/room1").unwrap(),
        Locator::parse(&format!("file:work/room1/JOB.md@{}", "a".repeat(40))).unwrap(),
    );
    bench.register(Box::new(SpawnTool::new())).unwrap();
    bench
}

/// City.md told a model not to delegate unless the person allowed
/// it, and nothing checked. Now the first spawn stops at a door.
#[test]
fn a_spawn_waits_for_the_person_and_a_granted_cluster_walks_through() {
    let mut bench = spawn_bench();
    let waiting = bench.invoke(&spawn_call(), &key(1), &ctx()).unwrap();
    let BenchOutcome::Pending { item } = waiting else {
        panic!("the first spawn of a run is the person's to allow");
    };
    assert_eq!(item.cluster_key.class, kernel::ApprovalClass::Delegation);
    assert_eq!(
        item.cluster_key.detail, "work/room1",
        "the cluster is the resident asking, so one answer covers its whole session"
    );
    assert!(item.action_desc.contains("work/helper"), "{item:?}");

    let mut allowed = spawn_bench();
    allowed.grant(item.cluster_key.clone());
    assert!(matches!(
        allowed.invoke(&spawn_call(), &key(1), &ctx()).unwrap(),
        BenchOutcome::Ran { .. }
    ));
}

/// Fail-closed: a bench nobody told what work it serves cannot mint
/// an item a person could answer, so it refuses rather than letting
/// the spawn through unasked.
#[test]
fn a_spawn_on_a_bench_with_no_job_is_refused_rather_than_waved_through() {
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    bench.register(Box::new(SpawnTool::new())).unwrap();
    let err = bench.invoke(&spawn_call(), &key(1), &ctx()).unwrap_err();
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
    assert!(err.recovery().contains("for_job"));
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
        .invoke(&edit_call("elsewhere/x", "v", "a", "b"), &key(2), &ctx())
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
    let err = match bare.invoke(&exec_call("rm -rf work"), &key(3), &ctx()) {
        Err(err) => err,
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);

    // With a net: the wave is fenced first, and the outcome carries
    // the commit the sweep will restore from.
    let checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let mut fenced_bench = ToolBench::new(domain).with_checkpoint(crate::bench::CheckpointNet {
        checkpoint,
        scope: "work".to_owned(),
        of: probe_provenance(),
    });
    fenced_bench.register(exec_tool()).unwrap();
    // The shell arm is unconfigured, so the tool itself refuses —
    // but only after the fence went up, which is what we assert.
    let _ = fenced_bench.invoke(&exec_call("rm -rf work"), &key(4), &ctx());
    // The fence went up before the command was allowed to run: a
    // repository now exists with a commit to restore from.
    assert!(
        tmp.path().join(".git").exists(),
        "the checkpoint net was raised"
    );
    let mut probe = Checkpoint::open(tmp.path()).unwrap();
    let payload = probe
        .wave_pre("work", TimeMs::new(1_700_000_001_000), &probe_provenance())
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
    let err = match bench.invoke(&call, &key(5), &ctx()) {
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
