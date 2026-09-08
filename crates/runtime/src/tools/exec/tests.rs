// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::sandbox::{EchoSandbox, FaultSandbox};
use kernel::Address;

fn call(arm: Value) -> ToolCall {
    let mut args = Map::new();
    args.insert("arm".to_owned(), arm);
    ToolCall {
        id: "c1".to_owned(),
        name: ToolName::parse("exec").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn tool(python: Option<PathBuf>, sandbox: Box<dyn Sandbox>, shell: Option<PathBuf>) -> ExecTool {
    ExecTool::new(
        std::env::temp_dir(),
        Vec::new(),
        python,
        sandbox,
        shell,
        Fuel(1_000_000),
        Address::parse("work").unwrap(),
    )
    .unwrap()
}

#[test]
fn a_missing_component_refuses_and_names_the_alternative() {
    let mut tool = tool(None, Box::new(EchoSandbox::new()), None);
    let err = match tool.invoke(&call(
        serde_json::json!({ "python": { "code": "print(1)" } }),
    )) {
        Err(err) => err,
        Ok(_) => panic!("a missing component must refuse"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
    assert!(
        err.recovery().contains("program arm"),
        "the refusal carries the alternative"
    );

    // Shell refuses too rather than silently becoming a program run.
    let err = match tool.invoke(&call(serde_json::json!({ "shell": { "text": "ls" } }))) {
        Err(err) => err,
        Ok(_) => panic!("a missing shell must refuse"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
}

#[test]
fn the_python_arm_runs_in_the_sandbox_and_reports_its_own_exit() {
    let mut echo = EchoSandbox::new();
    echo.queue_stdout(b"42\n".to_vec());
    let mut tool = tool(Some(PathBuf::from("python.wasm")), Box::new(echo), None);
    let outcome = tool
        .invoke(&call(
            serde_json::json!({ "python": { "code": "print(42)" } }),
        ))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["stdout"], "42\n");
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["arm"], "python");
}

#[test]
fn exhaustion_and_traps_reach_the_caller_as_themselves() {
    let sandbox = FaultSandbox::new(vec![
        SandboxExit::FuelExhausted,
        SandboxExit::Trap {
            message: "unreachable".to_owned(),
        },
    ]);
    let mut tool = tool(Some(PathBuf::from("python.wasm")), Box::new(sandbox), None);
    let first = tool
        .invoke(&call(
            serde_json::json!({ "python": { "code": "while True: pass" } }),
        ))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&first.result).unwrap()["outcome"],
        "fuel_exhausted"
    );
    let second = tool
        .invoke(&call(serde_json::json!({ "python": { "code": "boom" } })))
        .unwrap();
    let value = serde_json::to_value(&second.result).unwrap();
    assert_eq!(value["outcome"], "trap");
    assert_eq!(value["detail"], "unreachable");
}

#[test]
fn an_unrecognised_arm_is_refused_not_guessed() {
    let mut tool = tool(None, Box::new(EchoSandbox::new()), None);
    let err = match tool.invoke(&call(serde_json::json!({ "bash": { "text": "ls" } }))) {
        Err(err) => err,
        Ok(_) => panic!("an unknown arm must refuse"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

#[test]
fn the_program_arm_runs_a_real_child_with_a_scrubbed_environment() {
    let mut tool = tool(None, Box::new(EchoSandbox::new()), None);
    // A program every supported host has, printing nothing useful:
    // the assertion is that it ran and reported its own exit code.
    let (path, args) = if cfg!(windows) {
        ("cmd", vec!["/C".to_owned(), "exit 3".to_owned()])
    } else {
        ("sh", vec!["-c".to_owned(), "exit 3".to_owned()])
    };
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "program": { "path": path, "args": args }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["exit_code"], 3, "{result}");
    assert_eq!(result["arm"], "program");
}
