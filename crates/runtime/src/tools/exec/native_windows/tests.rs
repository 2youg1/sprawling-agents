// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Disposable-runner conformance through the same ExecTool invocation as production.

use super::*;
use crate::sandbox::{EchoSandbox, Fuel};
use crate::tools::exec::{ExecSetup, ExecTool, Shell};
use kernel::{Address, Payload, RunId, Tool, ToolCall, ToolName};
use serde_json::Value;

#[test]
fn default_windows_native_admission_requires_all_five_axes() {
    use crate::tools::exec::confinement::{Confinement, Guarantee, Kept};
    let chosen = Confinement::detect();
    for axis in Guarantee::ALL {
        assert_eq!(chosen.assurances().of(axis), Kept::Yes, "{axis:?}");
    }
}

#[test]
#[ignore = "writes AppContainer profiles and disposable ACLs; explicit Windows Actions acceptance only"]
fn native_windows_disposable_production_axes_and_cleanup() {
    assert_eq!(std::env::var("SPRAWLING_DISPOSABLE_NATIVE").unwrap(), "1");
    let work = tempfile::tempdir().unwrap();
    let host = tempfile::tempdir().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    assert!(std::net::TcpStream::connect(address).is_ok());
    std::fs::copy(
        std::env::current_exe().unwrap(),
        work.path().join("probe.exe"),
    )
    .unwrap();
    std::fs::write(
        work.path().join("native-input.json"),
        serde_json::to_vec(&serde_json::json!({
            "outside": host.path().join("escape"), "address": address.to_string(),
            "memory": DEFAULT_MEMORY_BYTES,
        }))
        .unwrap(),
    )
    .unwrap();
    let mut host_allocation = Vec::<u8>::new();
    host_allocation
        .try_reserve_exact(DEFAULT_MEMORY_BYTES)
        .unwrap();
    drop(host_allocation);
    let owner = RunId::from_bytes([0x71; 16]);
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(3_000, 5));
    let tool = ExecTool::new(
        ExecSetup {
            workdir: work.path().to_path_buf(),
            mounts: Vec::new(),
            python_wasm: None,
            shell: Shell::Absent,
            fuel: Fuel(1),
            env_passthrough: Vec::new(),
            domain: Address::parse("work").unwrap(),
            run: owner,
            policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
        },
        Box::new(EchoSandbox::new()),
        backlog.clone(),
    )
    .unwrap();
    let call = ToolCall {
        id: "native-production".to_owned(), name: ToolName::parse("exec").unwrap(),
        args: Payload::new(serde_json::json!({"arm":{"program":{
            "path":".\\probe.exe", "args":["--exact", "tools::exec::native_windows::tests::native_windows_child_axes", "--ignored", "--nocapture"]
        }}}).as_object().unwrap().clone()).unwrap(),
    };
    let outcome = tool.invoke(&call).unwrap();
    let result = serde_json::to_value(outcome.result).unwrap();
    assert_eq!(result["exit_code"], 0, "{result}");
    assert!(
        result["stdout"]
            .as_str()
            .unwrap()
            .contains("NATIVE_FIVE_AXES_PASSED"),
        "{result}"
    );
    assert!(!host.path().join("escape").exists());
    assert!(!work.path().join("native-copy-marker").exists());
    assert!(
        backlog
            .processes()
            .unwrap()
            .values()
            .all(|run| run.pids.is_empty())
    );
    assert!(
        std::net::TcpStream::connect(address).is_ok(),
        "harness/provider network remains available"
    );
}

#[test]
#[ignore = "child probe entered only by native_windows_disposable_production_axes_and_cleanup"]
fn native_windows_child_axes() {
    use std::os::windows::process::CommandExt;
    let input: Value =
        serde_json::from_slice(&std::fs::read("native-input.json").unwrap()).unwrap();
    std::fs::write("native-copy-marker", "copy write permitted").unwrap();
    assert!(std::fs::write(input["outside"].as_str().unwrap(), "escape").is_err());
    let address = input["address"].as_str().unwrap().parse().unwrap();
    assert!(
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(200))
            .is_err()
    );
    let bytes = usize::try_from(input["memory"].as_u64().unwrap()).unwrap();
    let mut allocation = Vec::<u8>::new();
    assert!(
        allocation.try_reserve_exact(bytes).is_err(),
        "committed Job memory is a hard ceiling"
    );
    assert!(
        Command::new(std::env::current_exe().unwrap())
            .creation_flags(0x0100_0000)
            .arg("--list")
            .spawn()
            .is_err(),
        "breakaway is refused"
    );
    let mut descendant = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tools::exec::native_windows::tests::native_windows_child_waits",
            "--ignored",
        ])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap();
    assert!(descendant.try_wait().unwrap().is_none());
    println!("NATIVE_DESCENDANT_PID={}", descendant.id());
    println!("NATIVE_FIVE_AXES_PASSED");
}

#[test]
#[ignore = "descendant probe owned and terminated by the native Job"]
fn native_windows_child_waits() {
    std::thread::park_timeout(std::time::Duration::from_secs(60));
    std::fs::write("native-survived", "must not survive its owner").unwrap();
}

#[test]
#[ignore = "writes AppContainer profiles and disposable ACLs; explicit Windows Actions acceptance only"]
fn native_windows_disposable_cancellation_owns_the_tree() {
    assert_eq!(std::env::var("SPRAWLING_DISPOSABLE_NATIVE").unwrap(), "1");
    let copy = tempfile::tempdir().unwrap();
    let exe = copy.path().join("probe.exe");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(1, 1));
    let owner = RunId::from_bytes([0x72; 16]);
    let scope = Address::parse("work").unwrap();
    let mut command = Command::new(exe);
    command.env_clear().current_dir(copy.path()).args([
        "--exact",
        "tools::exec::native_windows::tests::native_windows_child_waits",
        "--ignored",
    ]);
    let started = backlog
        .run_native(owner, &scope, "native cancellation".to_owned(), command)
        .unwrap();
    assert!(matches!(started, crate::Started::Backgrounded { .. }));
    assert_eq!(backlog.release(owner), 1);
    for _ in 0..200 {
        backlog.harvest(owner).unwrap();
        if backlog.standing(&scope).unwrap().is_empty() {
            assert!(!copy.path().join("native-survived").exists());
            return;
        }
        std::thread::park_timeout(std::time::Duration::from_millis(5));
    }
    panic!("native cancellation must reap its owned Job");
}
