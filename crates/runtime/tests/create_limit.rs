// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this asserts: under the create write limit a file that already
//! exists keeps its bytes on every write path a run has - edit's two
//! arms, a second name that links to it, and a command on the host
//! (`crates/runtime/spec/Tools.lean` §8-55).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{Address, AxCode, Payload, Tool, ToolCall, ToolName};
use serde_json::{Map, Value};

fn call(path: &str, base: &str, old: &str, new: &str) -> ToolCall {
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
        id: "call-1".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

/// The create write limit, observed on the real write paths: a file
/// that already exists keeps its bytes whether a run reaches for it
/// through edit's replacing arm, through edit's creating arm, through a
/// second name that links to it, or through a command on the host
/// (`crates/runtime/spec/Tools.lean` §8-55).
#[test]
fn an_existing_file_is_unchanged_under_create_by_edit_exec_and_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let work = Address::parse("work").unwrap();
    std::fs::create_dir_all(root.join("work")).unwrap();
    let kept = root.join("work").join("kept.md");
    std::fs::write(&kept, "the person's own\n").unwrap();
    let tool = runtime::EditTool::new(
        root,
        work.clone(),
        kernel::WriteDomain::new(vec![work.clone()]).unwrap(),
        runtime::PolicyCell::new(kernel::RunPolicy {
            write: kernel::WriteLimit::Create,
            ..kernel::RunPolicy::of(kernel::Mode::Work)
        })
        .reader(),
    )
    .unwrap();
    let seen = runtime::version_of(b"the person's own\n");

    let replaced = tool.invoke(&call("work/kept.md", &seen, "own", "changed"));
    assert_eq!(
        replaced.err().map(|err| *err.code()),
        Some(AxCode::OutsideWriteDomain),
        "edit's replacing arm is refused under create"
    );
    let created = tool.invoke(&call("work/kept.md", "new", "", "over it"));
    assert_eq!(
        created.err().map(|err| *err.code()),
        Some(AxCode::VersionConflict),
        "a create onto a name that stands is refused"
    );
    if std::fs::hard_link(&kept, root.join("work").join("second.md")).is_ok() {
        let linked = tool.invoke(&call("work/second.md", &seen, "own", "changed"));
        assert!(
            linked.is_err(),
            "a second name for the file reaches it no further"
        );
    }

    let exec = runtime::ExecTool::new(
        runtime::ExecSetup {
            workdir: root.join("work"),
            mounts: Vec::new(),
            python_wasm: None,
            shell: runtime::Shell::Absent,
            fuel: runtime::Fuel(1_000_000),
            env_passthrough: Vec::new(),
            domain: work,
            run: kernel::RunId::from_bytes([1; 16]),
            policy: runtime::PolicyCell::new(kernel::RunPolicy {
                write: kernel::WriteLimit::Create,
                ..kernel::RunPolicy::of(kernel::Mode::Work)
            })
            .reader(),
        },
        Box::new(runtime::EchoSandbox::new()),
        runtime::Backlog::with_window(runtime::PollBudget::new(6_000, 20)),
    )
    .unwrap();
    let (program, args) = if cfg!(windows) {
        (
            "cmd",
            vec!["/C".to_owned(), "echo changed> kept.md".to_owned()],
        )
    } else {
        (
            "sh",
            vec!["-c".to_owned(), "echo changed > kept.md".to_owned()],
        )
    };
    let mut exec_args = Map::new();
    exec_args.insert(
        "arm".to_owned(),
        serde_json::json!({ "program": { "path": program, "args": args } }),
    );
    exec_args.insert("where".to_owned(), Value::String("host".to_owned()));
    let hosted = Tool::invoke(
        &exec,
        &ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("exec").unwrap(),
            args: Payload::new(exec_args).unwrap(),
        },
    );
    assert_eq!(
        hosted.err().map(|err| *err.code()),
        Some(AxCode::OutsideWriteDomain),
        "a command on the host could change any file, so it does not start"
    );

    assert_eq!(
        std::fs::read_to_string(&kept).unwrap(),
        "the person's own\n",
        "the file is byte-identical after every path"
    );
}
