// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The shell arm's interpreter and its failure tally
//! (`crates/runtime/Spec.lean` §8-13-2 D30).

use super::*;

/// A building that asked for PowerShell 7 on a machine without it is
/// refused by name, and the refusal says how to get either interpreter
/// back (`crates/runtime/Spec.lean` §8-13-2 D30).
#[test]
fn a_shell_line_under_a_missing_pwsh_is_refused_by_name_not_run_by_cmd() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = ExecTool::new(
        ExecSetup {
            shell: Shell::Missing {
                asked: kernel::Interpreter::Pwsh,
            },
            ..setup(chamber.path(), None, None)
        },
        Box::new(EchoSandbox::new()),
        patient(),
    )
    .unwrap();
    let Err(err) = tool.invoke(&call_at(
        serde_json::json!({ "shell": { "text": "echo hi" } }),
        "host",
    )) else {
        panic!("a missing pwsh refuses rather than running the line elsewhere");
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
    assert!(err.subject().contains("pwsh 7"), "{}", err.subject());
    assert!(
        err.recovery().contains("interpreter = \"system\""),
        "{}",
        err.recovery()
    );
}

/// The platform's own shell runs a line, and the result names which
/// program that was, so the tally can count it per interpreter.
#[test]
fn a_shell_result_names_the_interpreter_that_ran_it() {
    let chamber = tempfile::tempdir().unwrap();
    let program = if cfg!(windows) {
        std::env::var_os("COMSPEC").map_or_else(|| PathBuf::from("cmd.exe"), PathBuf::from)
    } else {
        PathBuf::from("/bin/sh")
    };
    let expected = if cfg!(windows) { "cmd" } else { "sh" };
    let tool = a_tool(
        chamber.path(),
        None,
        Box::new(EchoSandbox::new()),
        Some(program),
    );
    let answer = tool
        .invoke(&call_at(
            serde_json::json!({ "shell": { "text": "echo hi" } }),
            "host",
        ))
        .unwrap();
    let result = answer.result.as_map();
    assert_eq!(result["interpreter"], Value::String(expected.to_owned()));
    assert_eq!(result["exit_code"], Value::from(0));
}

/// One row per rule of D30, with the interpreter it applies to.
#[test]
fn each_interpreter_s_failures_fall_in_the_class_its_own_signal_names() {
    use FailureClass::{CommandNotFound, Encoding, Syntax};
    let rows: [(&str, i64, &str, &str, Option<FailureClass>); 14] = [
        ("cmd", 0, "\u{FFFD}", "", None),
        (
            "cmd",
            9009,
            "",
            "'x' is not recognized",
            Some(CommandNotFound),
        ),
        ("cmd", 1, "", "( was unexpected at this time.", Some(Syntax)),
        (
            "cmd",
            1,
            "",
            "The syntax of the command is incorrect.",
            Some(Syntax),
        ),
        ("cmd", 1, "\u{FFFD}\u{FFFD}", "", Some(Encoding)),
        ("cmd", 127, "", "", None),
        ("sh", 127, "", "sh: 1: x: not found", Some(CommandNotFound)),
        (
            "bash",
            2,
            "",
            "bash: -c: line 1: syntax error near `)'",
            Some(Syntax),
        ),
        ("bash", 1, "", "syntax error", None),
        (
            "pwsh",
            1,
            "",
            "CommandNotFoundException",
            Some(CommandNotFound),
        ),
        (
            "pwsh",
            1,
            "",
            "ParserError: Missing closing ')'",
            Some(Syntax),
        ),
        ("pwsh", 9009, "", "", None),
        ("system", 9009, "", "", Some(CommandNotFound)),
        ("system", 127, "", "", Some(CommandNotFound)),
    ];
    for (interpreter, code, stdout, stderr, class) in rows {
        assert_eq!(
            FailureClass::of(interpreter, code, stdout, stderr),
            class,
            "{interpreter} {code} {stderr:?}"
        );
    }
}

/// The tally counts shell lines that ended with a code, per interpreter,
/// and leaves every other result out.
#[test]
fn the_tally_counts_ended_shell_lines_per_interpreter_and_nothing_else() {
    let results = [
        serde_json::json!({ "arm": "shell", "interpreter": "pwsh", "exit_code": 0, "stdout": "", "stderr": "" }),
        serde_json::json!({ "arm": "shell", "interpreter": "pwsh", "exit_code": 1, "stdout": "", "stderr": "ParserError" }),
        serde_json::json!({ "arm": "shell", "interpreter": "cmd", "exit_code": 9009, "stdout": "", "stderr": "" }),
        serde_json::json!({ "arm": "shell", "exit_code": 127, "stdout": "", "stderr": "" }),
        serde_json::json!({ "arm": "shell", "interpreter": "cmd", "outcome": "backgrounded", "handle": "b1" }),
        serde_json::json!({ "arm": "program", "exit_code": 9009, "stdout": "", "stderr": "" }),
    ];
    let mut tally = ShellTally::default();
    for result in &results {
        tally.absorb(result.as_object().unwrap());
    }
    let counted: Vec<(&str, ShellCount)> = tally
        .interpreters()
        .map(|(name, count)| (name, count.clone()))
        .collect();
    let count = |calls, failures: &[(FailureClass, u64)]| ShellCount {
        calls,
        failures: failures.iter().copied().collect(),
    };
    assert_eq!(
        counted,
        vec![
            ("cmd", count(1, &[(FailureClass::CommandNotFound, 1)])),
            ("pwsh", count(2, &[(FailureClass::Syntax, 1)])),
            ("system", count(1, &[(FailureClass::CommandNotFound, 1)])),
        ]
    );
}

/// The description names the interpreter a shell line runs under, in
/// the spelling the result records, and says nothing of one that is not
/// offered.
#[test]
fn the_tool_description_names_the_interpreter_its_shell_lines_run_under() {
    let chamber = tempfile::tempdir().unwrap();
    let described = |shell: Shell| {
        ExecTool::new(
            ExecSetup {
                shell,
                ..setup(chamber.path(), None, None)
            },
            Box::new(EchoSandbox::new()),
            patient(),
        )
        .unwrap()
        .meta()
        .disclosure
        .clone()
    };
    let pwsh = described(Shell::Found {
        program: PathBuf::from("/opt/microsoft/powershell/7/pwsh"),
        interpreter: kernel::Interpreter::Pwsh,
    });
    assert!(pwsh.ends_with("A shell line runs under pwsh."), "{pwsh}");
    let absent = described(Shell::Absent);
    assert!(!absent.contains("A shell line runs under"), "{absent}");
}
