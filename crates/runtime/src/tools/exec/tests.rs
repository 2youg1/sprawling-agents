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

/// The call as the agent writes it: an arm, and where it runs.
fn call_at(arm: Value, placement: &str) -> ToolCall {
    let mut args = Map::new();
    args.insert("arm".to_owned(), arm);
    args.insert("where".to_owned(), Value::String(placement.to_owned()));
    ToolCall {
        id: "c1".to_owned(),
        name: ToolName::parse("exec").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

/// A working directory of the test's own.
///
/// It is small on purpose. A sandboxed command runs in a copy of this
/// directory, and pointing the tools at this machine's temporary
/// directory - which holds whatever the machine left there - would make
/// every one of these tests copy gigabytes.
fn setup(workdir: &std::path::Path, python: Option<PathBuf>, shell: Option<PathBuf>) -> ExecSetup {
    ExecSetup {
        workdir: workdir.to_path_buf(),
        mounts: Vec::new(),
        python_wasm: python,
        shell,
        fuel: Fuel(1_000_000),
        env_passthrough: Vec::new(),
        domain: Address::parse("work").unwrap(),
    }
}

fn a_tool(
    workdir: &std::path::Path,
    python: Option<PathBuf>,
    sandbox: Box<dyn Sandbox>,
    shell: Option<PathBuf>,
) -> ExecTool {
    ExecTool::new(setup(workdir, python, shell), sandbox, Backlog::new()).unwrap()
}

/// One variable this process really has, which no building declares by
/// default and which no rule refuses. Picked from the parent environment
/// rather than written down, so the test does not depend on which
/// machine it runs on; taken in sorted order so it picks the same one
/// twice.
fn a_name_this_machine_sets() -> (String, String) {
    let mut candidates: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for (name, value) in std::env::vars() {
        let admissible = !ENV_ALLOWLIST.contains(&name.as_str())
            && kernel::EnvVarName::parse(&name).is_ok()
            && !value.is_empty()
            && value.is_ascii()
            && !value.contains('%')
            && !value.contains('"');
        if admissible {
            candidates.insert(name, value);
        }
    }
    candidates
        .into_iter()
        .next()
        .expect("every supported host sets at least one ordinary variable")
}

fn reads_the_variable(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec!["/C".to_owned(), format!("echo %{name}%")],
        )
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), format!("printf %s \"${name}\"")],
        )
    }
}

#[test]
fn a_child_sees_the_names_its_building_declared_and_no_others() {
    let (name, value) = a_name_this_machine_sets();
    let (path, args) = reads_the_variable(&name);

    let chamber = tempfile::tempdir().unwrap();
    let declared = ExecSetup {
        env_passthrough: vec![kernel::EnvVarName::parse(&name).unwrap()],
        ..setup(chamber.path(), None, None)
    };
    let mut tool = ExecTool::new(declared, Box::new(EchoSandbox::new()), Backlog::new()).unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "program": { "path": path.clone(), "args": args.clone() }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert!(
        result["stdout"].as_str().unwrap().contains(&value),
        "a declared name reaches the child: {result}"
    );
    let listed: Vec<String> = serde_json::from_value(result["env"].clone()).unwrap();
    assert!(
        listed.contains(&name),
        "the ledger says what it inherited: {result}"
    );

    // The same command in a building that declared nothing sees nothing.
    let bare_chamber = tempfile::tempdir().unwrap();
    let mut bare = a_tool(
        bare_chamber.path(),
        None,
        Box::new(EchoSandbox::new()),
        None,
    );
    let outcome = bare
        .invoke(&call(serde_json::json!({
            "program": { "path": path, "args": args }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert!(
        !result["stdout"].as_str().unwrap().contains(&value),
        "an undeclared name must not reach the child: {result}"
    );
    let listed: Vec<String> = serde_json::from_value(result["env"].clone()).unwrap();
    assert!(!listed.contains(&name), "{result}");
}

#[test]
fn a_missing_component_refuses_and_names_the_alternative() {
    let chamber = tempfile::tempdir().unwrap();
    let mut tool = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
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
    let chamber = tempfile::tempdir().unwrap();
    let mut echo = EchoSandbox::new();
    echo.queue_stdout(b"42\n".to_vec());
    let mut tool = a_tool(
        chamber.path(),
        Some(PathBuf::from("python.wasm")),
        Box::new(echo),
        None,
    );
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
    let chamber = tempfile::tempdir().unwrap();
    let sandbox = FaultSandbox::new(vec![
        SandboxExit::FuelExhausted,
        SandboxExit::Trap {
            message: "unreachable".to_owned(),
        },
    ]);
    let mut tool = a_tool(
        chamber.path(),
        Some(PathBuf::from("python.wasm")),
        Box::new(sandbox),
        None,
    );
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
    let chamber = tempfile::tempdir().unwrap();
    let mut tool = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
    let err = match tool.invoke(&call(serde_json::json!({ "bash": { "text": "ls" } }))) {
        Err(err) => err,
        Ok(_) => panic!("an unknown arm must refuse"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

#[test]
fn the_program_arm_runs_a_real_child_with_a_scrubbed_environment() {
    let chamber = tempfile::tempdir().unwrap();
    let mut tool = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
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

/// A command that writes a file beside itself, and prints what it
/// wrote. Both halves matter: the first proves it could write where it
/// ran, the second proves where it ran.
fn writes_beside_itself(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec![
                "/C".to_owned(),
                format!("echo written> {name} && type {name}"),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                format!("echo written > {name}; cat {name}"),
            ],
        )
    }
}

/// The one promise the floor arm makes, asked of a real command on a
/// real tree: it may write, and what it writes does not reach the tree
/// the person has.
#[test]
fn a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone() {
    let chamber = tempfile::tempdir().unwrap();
    std::fs::write(chamber.path().join("kept.txt"), b"the person's own\n").unwrap();
    let mut tool = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
    let (path, args) = writes_beside_itself("made.txt");
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "program": { "path": path, "args": args }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["exit_code"], 0, "{result}");
    assert!(
        result["stdout"].as_str().unwrap().contains("written"),
        "the command wrote where it ran: {result}"
    );
    assert!(
        !chamber.path().join("made.txt").exists(),
        "a sandboxed write must not land in the tree the person has"
    );
    assert_eq!(
        std::fs::read_to_string(chamber.path().join("kept.txt")).unwrap(),
        "the person's own\n",
        "the tree it read is the tree the person has"
    );

    // The same command with the host placement is what reaches the tree,
    // which is what makes the assertion above a capability decision
    // rather than a command that never wrote anything.
    let mut host = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
    let (path, args) = writes_beside_itself("made.txt");
    host.invoke(&call_at(
        serde_json::json!({
            "program": { "path": path, "args": args }
        }),
        "host",
    ))
    .unwrap();
    assert!(
        chamber.path().join("made.txt").exists(),
        "the host placement is the one a person decides on, and it is the one that writes"
    );
}

#[test]
fn the_python_arm_has_no_sandbox_host_form() {
    let chamber = tempfile::tempdir().unwrap();
    let mut tool = a_tool(
        chamber.path(),
        Some(PathBuf::from("python.wasm")),
        Box::new(EchoSandbox::new()),
        None,
    );
    let err = match tool.invoke(&call_at(
        serde_json::json!({ "python": { "code": "print(1)" } }),
        "host",
    )) {
        Err(err) => err,
        Ok(_) => panic!("the wasip1 guest has no host interpreter to run it"),
    };
    assert_eq!(*err.code(), AxCode::SandboxDenied);
    assert!(
        err.recovery().contains("arm: program"),
        "{}",
        err.recovery()
    );
}
