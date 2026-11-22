// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
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
            EditTool::new(
                root,
                Address::parse("work").unwrap(),
                domain,
                crate::tools::EditDesk {
                    policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work))
                        .reader(),
                    seen: Default::default(),
                    locks: Default::default(),
                },
            )
            .unwrap(),
        ))
        .unwrap();
    bench
}

fn edit_call(path: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (k, v) in [
        ("file_path", path),
        ("old_string", old),
        ("new_string", new),
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
    let mut bench = bench(tmp.path());
    let call = edit_call("work/a.txt", "", "two\n");

    let first = bench.invoke(&call, &key(1), now()).unwrap();
    assert!(matches!(first, BenchOutcome::Ran { .. }));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "two\n"
    );

    let second = bench.invoke(&call, &key(1), now()).unwrap();

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

    bench
        .register(Box::new(
            EditTool::new(
                tmp.path(),
                Address::parse("elsewhere").unwrap(),
                elsewhere,
                crate::tools::EditDesk {
                    policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work))
                        .reader(),
                    seen: Default::default(),
                    locks: Default::default(),
                },
            )
            .unwrap(),
        ))
        .unwrap();
    let outcome = bench
        .invoke(&edit_call("elsewhere/x", "a", "b"), &key(2), now())
        .unwrap();
    match outcome {
        BenchOutcome::Refused { refusal } => {
            assert_eq!(*refusal.code(), AxCode::OutsideWriteDomain);
        }
        other => panic!("expected a refusal that keeps the turn alive, got {other:?}"),
    }
}

#[test]
fn a_suspected_discard_without_a_net_is_refused_and_with_one_is_checkpointed() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("work")).unwrap();
    std::fs::write(tmp.path().join("work/doomed.txt"), "bye").unwrap();
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();

    let exec_call = |text: &str| {
        let mut args = Map::new();
        args.insert("command".to_owned(), Value::String(text.to_owned()));
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
                    python: None,
                    shell: crate::Shell::Absent,
                    env_passthrough: Vec::new(),
                    domain: Address::parse("work").unwrap(),
                    run: RunId::from_bytes([1u8; 16]),
                    policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work))
                        .reader(),
                },
                crate::Backlog::new(),
            )
            .unwrap(),
        )
    };

    let mut bare = ToolBench::new(domain.clone());
    bare.register(exec_tool()).unwrap();
    let err = match bare.invoke(&exec_call("rm -rf work"), &key(3), now()) {
        Err(err) => err,
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);

    let checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let mut checkpointed_bench =
        ToolBench::new(domain).with_checkpoint(crate::bench::CheckpointNet {
            checkpoint,
            scope: vec!["work".to_owned()],
            of: probe_provenance(),
        });
    checkpointed_bench.register(exec_tool()).unwrap();

    let _ = checkpointed_bench.invoke(&exec_call("rm -rf work"), &key(4), now());

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
    assert!(
        !oid.is_empty(),
        "the checkpoint has a commit to restore from"
    );
}

#[test]
fn an_unregistered_name_is_refused_rather_than_routed_anywhere() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bench = bench(tmp.path());
    let mut call = edit_call("work/a", "a", "b");
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
            crate::tools::EditDesk {
                policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
                seen: Default::default(),
                locks: Default::default(),
            },
        )
        .unwrap(),
    )) {
        Err(err) => err,
        Ok(()) => panic!("a name collision must refuse, not shadow"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

fn probe_provenance() -> storage::Provenance {
    storage::Provenance::new(
        kernel::RunId::CITY,
        kernel::Address::parse("work/probe").unwrap(),
        kernel::B3Hash::digest(b"a city"),
        storage::ModelChoice {
            id: "test-model".to_owned(),
            effort: None,
        },
    )
}

#[test]
fn a_documents_bench_lets_its_own_room_through_the_door() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("hall")).unwrap();
    let domain = WriteDomain::documents(vec![Address::parse("hall").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain.clone());
    bench
        .register(Box::new(
            EditTool::new(
                tmp.path(),
                Address::parse("hall/mayor").unwrap(),
                domain,
                crate::tools::EditDesk {
                    policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work))
                        .reader(),
                    seen: Default::default(),
                    locks: Default::default(),
                },
            )
            .unwrap(),
        ))
        .unwrap();
    let outcome = bench
        .invoke(&edit_call("hall/note.md", "", "# note\n"), &key(9), now())
        .unwrap();
    match outcome {
        BenchOutcome::Ran { .. } => {}
        other => panic!("expected the note to land, got {other:?}"),
    }
    assert!(tmp.path().join("hall/note.md").is_file());
}

fn exec_bench(root: &std::path::Path) -> ToolBench {
    std::fs::create_dir_all(root.join("work")).unwrap();
    let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    bench
        .register(Box::new(
            ExecTool::new(
                crate::tools::ExecSetup {
                    workdir: root.to_path_buf(),
                    python: None,
                    shell: crate::Shell::Absent,
                    env_passthrough: Vec::new(),
                    domain: Address::parse("work").unwrap(),
                    run: RunId::from_bytes([1u8; 16]),
                    policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work))
                        .reader(),
                },
                crate::Backlog::new(),
            )
            .unwrap(),
        ))
        .unwrap();
    bench
}

fn exec_call() -> ToolCall {
    let mut args = Map::new();
    args.insert("command".to_owned(), Value::String("echo hi".to_owned()));
    ToolCall {
        id: "e1".to_owned(),
        name: kernel::ToolName::parse("exec").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn assert_refused_as_tainted(answered: Result<BenchOutcome, AxError>) {
    match answered {
        Ok(BenchOutcome::Refused { refusal }) => {
            assert_eq!(*refusal.code(), AxCode::TaintedAction);
        }
        other => panic!("expected the tainted exec to be refused, got {other:?}"),
    }
}

#[test]
fn a_run_started_by_outside_content_is_refused_exec() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bench = exec_bench(tmp.path());
    *bench.taint_mut() = TaintSet::of(kernel::TaintSource::new("web:evil").unwrap());
    assert_refused_as_tainted(bench.invoke(&exec_call(), &key(10), now()));
}

struct Answering(kernel::ToolMeta);

impl Tool for Answering {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.0
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

#[test]
fn a_run_that_read_outside_content_is_refused_exec() {
    let tmp = tempfile::tempdir().unwrap();
    let mut bench = exec_bench(tmp.path());
    bench
        .register(Box::new(Answering(kernel::ToolMeta {
            name: kernel::ToolName::parse("page").unwrap(),
            disclosure: "answers with a web page".to_owned(),
            params: Payload::empty(),
            effect: Effect::Egress,
            cost_tier: kernel::CostTier::Free,
            timeout: None,
            render: kernel::RenderIntent::Generic,
            temporal: kernel::Temporal::Timeless,
        })))
        .unwrap();
    let page = ToolCall {
        id: "p1".to_owned(),
        name: kernel::ToolName::parse("page").unwrap(),
        args: Payload::empty(),
    };
    assert!(matches!(
        bench.invoke(&page, &key(11), now()),
        Ok(BenchOutcome::Ran { .. })
    ));
    assert_refused_as_tainted(bench.invoke(&exec_call(), &key(12), now()));
}
