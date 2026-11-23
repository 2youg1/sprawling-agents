// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{Address, Payload, RunId, Tool, ToolCall, ToolName};
use runtime::{Backlog, ExecSetup, ExecTool, PollBudget};
use serde_json::{Value, json};

const MARK: &str = "only-for-the-run-that-started-it";

fn a_run_at(backlog: &Backlog, building: &str, run: u8, workdir: &std::path::Path) -> ExecTool {
    let setup = ExecSetup {
        workdir: workdir.to_path_buf(),
        python: None,
        shell: runtime::Shell::Absent,
        env_passthrough: Vec::new(),
        domain: Address::parse(building).unwrap(),
        run: RunId::from_bytes([run; 16]),
        policy: runtime::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
    };
    ExecTool::new(setup, backlog.clone()).unwrap()
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

fn slow_then_mark() -> Value {
    let (path, args) = if cfg!(windows) {
        (
            "cmd",
            vec![
                "/C".to_owned(),
                format!("ping -n 4 127.0.0.1 >NUL & echo {MARK}"),
            ],
        )
    } else {
        ("sh", vec!["-c".to_owned(), format!("sleep 3; echo {MARK}")])
    };
    let quoted: Vec<String> = args.iter().map(|arg| format!("\"{arg}\"")).collect();
    json!({ "command": format!("{path} {}", quoted.join(" ")), "where": "host" })
}

fn quick_look() -> Value {
    let command = if cfg!(windows) {
        "cmd /C echo looking"
    } else {
        "echo looking"
    };
    json!({ "command": command, "where": "host" })
}

#[test]
fn a_background_result_reaches_only_the_run_that_started_it() {
    let work = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(PollBudget::new(50, 20));
    let mut starter = a_run_at(&backlog, "vault", 1, work.path());
    let mut stranger = a_run_at(&backlog, "lab", 2, work.path());

    let started = invoke(&mut starter, slow_then_mark());
    assert_eq!(started["outcome"], "backgrounded", "{started}");

    let mut delivered = None;
    for _ in 0..6_000 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let seen = invoke(&mut stranger, quick_look());
        assert!(
            seen.get("background").is_none(),
            "another run's output reached this run: {seen}"
        );
        assert!(
            seen.get("outcome").is_none(),
            "the poll itself must settle so it does not invent background rows: {seen}"
        );
        let mine = invoke(&mut starter, quick_look());
        assert!(
            mine.get("outcome").is_none(),
            "the poll itself must settle so a background row is only the slow command: {mine}"
        );
        if let Some(rows) = mine.get("background") {
            delivered = Some(rows.clone());
            break;
        }
    }
    let rows = delivered.expect("the run that started the command is handed its result");
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "{rows}");
    assert!(rows[0]["stdout"].as_str().unwrap().contains(MARK), "{rows}");
}
