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

use kernel::{Address, EnvVarName, Payload, RunId, Tool, ToolCall, ToolName};
use runtime::{Backlog, ExecSetup, ExecTool};

#[cfg(windows)]
const DECLARED: [&str; 8] = [
    "ProgramFiles(x86)",
    "ProgramFiles",
    "ProgramData",
    "SystemRoot",
    "SystemDrive",
    "ComSpec",
    "PATHEXT",
    "LOCALAPPDATA",
];

#[cfg(not(windows))]
const DECLARED: [&str; 2] = ["HOME", "USER"];

#[test]
fn a_building_that_declares_the_names_can_build_a_rust_program() {
    let cargo = which_cargo().expect("the Rust build regression requires cargo on PATH");
    let work = tempfile::tempdir().unwrap();
    let crate_dir = work.path().join("hello");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"hello\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(crate_dir.join("src/main.rs"), "fn main() {}\n").unwrap();

    let mut declared: Vec<EnvVarName> = DECLARED
        .iter()
        .map(|name| EnvVarName::parse(name).unwrap())
        .collect();

    for optional in ["CARGO_HOME", "RUSTUP_HOME", "USERPROFILE"] {
        if std::env::var(optional).is_ok() {
            declared.push(EnvVarName::parse(optional).unwrap());
        }
    }

    let run = RunId::from_bytes([1; 16]);
    let setup = ExecSetup {
        workdir: crate_dir,
        python: None,
        shell: runtime::Shell::Absent,
        env_passthrough: declared,
        domain: Address::parse("lab").unwrap(),
        run,
        policy: runtime::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
    };

    let backlog = Backlog::new();
    let tool = ExecTool::new(setup, backlog.clone()).unwrap();
    #[cfg(windows)]
    let tool = if std::env::var("SPRAWLING_DISPOSABLE_NATIVE").as_deref() == Ok("1") {
        let tool = tool.confined(runtime::tools::Confined::with_arm(
            runtime::tools::Confinement::WindowsJobObject,
            Some(std::env::temp_dir()),
        ));
        assert!(tool.meta().disclosure.contains("windows_job_object"));
        eprintln!("NATIVE_REGRESSION_ARM=windows_job_object");
        tool
    } else {
        tool
    };

    let mut args = serde_json::Map::new();
    args.insert(
        "command".to_owned(),
        serde_json::Value::String(format!("\"{cargo}\" build --offline --target-dir target")),
    );
    let outcome = tool
        .invoke(&ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("exec").unwrap(),
            args: Payload::new(args).unwrap(),
        })
        .unwrap();
    let mut result = serde_json::to_value(&outcome.result).unwrap();

    if result.get("exit_code").is_none()
        && let Some(finished) = result["background"]
            .as_array()
            .and_then(|rows| rows.first())
            .cloned()
    {
        result = finished;
    }
    if result.get("outcome").and_then(serde_json::Value::as_str) == Some("backgrounded") {
        for _ in 0..1_500 {
            let done = backlog.harvest(run).unwrap();
            if let Some(finished) = done.first() {
                eprintln!("exec build harvest: {finished:?}");
                result["stdout"] = serde_json::json!(finished.stdout);
                result["stderr"] = serde_json::json!(finished.stderr);
                result["exit_code"] = match finished.exit {
                    runtime::Exit::Ended { code } => serde_json::json!(code),
                    other @ (runtime::Exit::Signalled | runtime::Exit::Unknown { .. }) => {
                        serde_json::json!(other.as_str())
                    }
                };
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    eprintln!("exec build result: {result}");
    assert_eq!(
        result["exit_code"], 0,
        "a declared building builds Rust: {result}"
    );
}

fn which_cargo() -> Result<String, ()> {
    let name = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    Err(())
}
