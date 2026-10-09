// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A dispatched command starts below the core, on every platform
//! (`crates/runtime/spec/Tools/Exec.lean` §8-13-3).

#![allow(clippy::arithmetic_side_effects, reason = "test code")]

use super::*;

/// A command that prints the scheduling priority it itself runs at, and
/// the variables it needs to start: PowerShell does not start without
/// `SystemRoot`.
fn reads_its_own_priority() -> (String, Vec<String>, Vec<&'static str>) {
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-Command".to_owned(),
                "[System.Diagnostics.Process]::GetCurrentProcess().PriorityClass".to_owned(),
            ],
            vec!["SystemRoot"],
        )
    } else {
        // `ps` rather than a bare `nice`: BSD `nice` without a utility
        // prints nothing on macOS, while `ps -o nice=` answers on both.
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), "ps -o nice= -p $$".to_owned()],
            Vec::new(),
        )
    }
}

/// Where a printed priority stands, higher meaning more CPU: a Windows
/// priority class by name, or a Unix niceness, where more is less.
fn rank(printed: &str) -> i64 {
    let printed = printed.trim();
    match printed {
        "Idle" => 0,
        "BelowNormal" => 1,
        "Normal" => 2,
        "AboveNormal" => 3,
        "High" => 4,
        "RealTime" => 5,
        niceness => -niceness
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("not a priority: {printed:?}")),
    }
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn a_dispatched_command_runs_below_the_core() {
    let (path, args, needs) = reads_its_own_priority();
    let mut direct = std::process::Command::new(&path);
    direct.args(&args).env_clear();
    for name in ENV_ALLOWLIST.iter().chain(&needs) {
        if let Ok(value) = std::env::var(name) {
            direct.env(name, value);
        }
    }
    let core = direct.stdin(std::process::Stdio::null()).output().unwrap();
    assert!(
        core.status.success(),
        "the direct priority probe must succeed: {core:?}"
    );
    assert!(
        !core.stdout.is_empty(),
        "the direct priority probe must report a priority: {core:?}"
    );
    let core = String::from_utf8_lossy(&core.stdout).into_owned();

    let chamber = tempfile::tempdir().unwrap();
    let declared = ExecSetup {
        env_passthrough: needs
            .iter()
            .map(|name| kernel::EnvVarName::parse(name).unwrap())
            .collect(),
        ..setup(chamber.path(), None, None)
    };
    let tool = regression_boundary(
        ExecTool::new(declared, Box::new(EchoSandbox::new()), patient()).unwrap(),
    );
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "program": { "path": path, "args": args }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    let dispatched = result["stdout"].as_str().unwrap();
    assert_eq!(
        result["exit_code"], 0,
        "the dispatched priority probe must succeed: {result}"
    );
    assert!(
        !dispatched.trim().is_empty(),
        "the dispatched priority probe must report a priority: {result}"
    );

    // Windows sets an absolute class, so a core that a CI runner already
    // starts at BelowNormal leaves its commands beside it, not beneath it.
    let floor = rank("BelowNormal");
    assert!(
        rank(dispatched) < rank(&core) || (rank(dispatched) == rank(&core) && rank(&core) <= floor),
        "a dispatched command runs below the core: it ran at {dispatched:?}, the core at \
         {core:?}; {result}"
    );
}

/// `ionice` with no arguments prints the IO class it itself runs in, so
/// run through exec it reads back the level a dispatched command gets.
#[cfg(target_os = "linux")]
#[test]
fn a_dispatched_command_reads_and_writes_at_the_lowest_best_effort_io_level() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = ExecTool::new(
        setup(chamber.path(), None, None),
        Box::new(EchoSandbox::new()),
        patient(),
    )
    .unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "program": { "path": "ionice", "args": [] }
        })))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(
        result["stdout"].as_str().map(str::trim),
        Some("best-effort: prio 7"),
        "{result}"
    );
}

/// `nice` starts even when the program it is asked to run does not exist,
/// so without a lookup of its own a missing program would come back as a
/// settled exit 127 instead of the typed refusal with its recovery.
#[cfg(unix)]
#[test]
fn a_missing_program_still_refuses_under_nice() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), None, Box::new(EchoSandbox::new()), None);
    let err = match tool.invoke(&call(serde_json::json!({
        "program": { "path": "sprawling-no-such-program", "args": [] }
    }))) {
        Err(err) => err,
        Ok(outcome) => panic!(
            "a missing program must refuse: {}",
            serde_json::to_value(&outcome.result).unwrap()
        ),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
    assert!(err.recovery().contains("program name"), "{err:?}");
}
