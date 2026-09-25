// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this asserts: a background command's output returns to the run
//! that started it and to no other run in the city.
//!
//! The table is one per city, and every building's `exec` holds a clone
//! of it, so the only thing that can keep one run's output out of
//! another run's tool result is the table itself. The two runs here
//! share one table exactly as two buildings of a served city do.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::PathBuf;

use kernel::{Address, Payload, Tool, ToolCall, ToolName};
use runtime::{Backlog, EchoSandbox, ExecSetup, ExecTool, Fuel, PollBudget};
use serde_json::{Value, json};

const MARK: &str = "only-for-the-run-that-started-it";

fn a_run_at(backlog: &Backlog, building: &str, workdir: &std::path::Path) -> ExecTool {
    let setup = ExecSetup {
        workdir: workdir.to_path_buf(),
        mounts: Vec::new(),
        // The sandbox is the echo double, which never reads this path:
        // the python arm is here as a call that settles without a child
        // process, so it reads the table and nothing else.
        python_wasm: Some(PathBuf::from("python.wasm")),
        shell: None,
        fuel: Fuel(1_000_000),
        env_passthrough: Vec::new(),
        domain: Address::parse(building).unwrap(),
    };
    ExecTool::new(setup, Box::new(EchoSandbox::new()), backlog.clone()).unwrap()
}

fn invoke(tool: &mut ExecTool, args: Value) -> Value {
    let Value::Object(args) = args else {
        panic!("the arguments are an object")
    };
    let outcome = tool
        .invoke(&ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("exec").unwrap(),
            args: Payload::new(args).unwrap(),
        })
        .unwrap();
    serde_json::to_value(&outcome.result).unwrap()
}

/// A command that outlives a one-poll window and then prints the mark.
fn slow_then_mark() -> Value {
    let (path, args) = if cfg!(windows) {
        (
            "cmd",
            vec![
                "/C".to_owned(),
                format!("ping -n 2 127.0.0.1 >NUL & echo {MARK}"),
            ],
        )
    } else {
        ("sh", vec!["-c".to_owned(), format!("sleep 1; echo {MARK}")])
    };
    json!({ "arm": { "program": { "path": path, "args": args } }, "where": "host" })
}

fn quick_look() -> Value {
    json!({ "arm": { "python": { "code": "pass" } } })
}

#[test]
fn a_background_result_reaches_only_the_run_that_started_it() {
    let work = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(PollBudget::new(1, 1));
    let mut starter = a_run_at(&backlog, "vault", work.path());
    let mut stranger = a_run_at(&backlog, "lab", work.path());

    let started = invoke(&mut starter, slow_then_mark());
    assert_eq!(started["outcome"], "backgrounded", "{started}");

    // The stranger looks first on every round, so the moment the command
    // ends the first harvest after it is the stranger's.
    let mut delivered = None;
    for _ in 0..500 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let seen = invoke(&mut stranger, quick_look());
        assert!(
            seen.get("background").is_none(),
            "another run's output reached this run: {seen}"
        );
        let mine = invoke(&mut starter, quick_look());
        if let Some(rows) = mine.get("background") {
            delivered = Some(rows.clone());
            break;
        }
    }
    let rows = delivered.expect("the run that started the command is handed its result");
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "{rows}");
    assert!(rows[0]["stdout"].as_str().unwrap().contains(MARK), "{rows}");
}
