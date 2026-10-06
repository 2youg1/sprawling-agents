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

const TEST_MEMORY_BYTES: usize = 0x1000_0000;

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
            "memory": TEST_MEMORY_BYTES,
        }))
        .unwrap(),
    )
    .unwrap();
    let mut host_allocation = Vec::<u8>::new();
    host_allocation
        .try_reserve_exact(TEST_MEMORY_BYTES)
        .unwrap();
    drop(host_allocation);
    let owner = RunId::from_bytes([0x71; 16]);
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(3_000, 5)).with_shares(
        Shares::CpuAndMemory {
            limit: std::num::NonZeroU64::new(u64::try_from(TEST_MEMORY_BYTES).unwrap()).unwrap(),
        },
    );
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
    .unwrap()
    .confined(crate::tools::exec::confinement::Confined::with_arm(
        crate::tools::exec::confinement::Confinement::WindowsJobObject,
        Some(std::env::temp_dir()),
    ));
    let call = ToolCall {
        id: "native-production".to_owned(), name: ToolName::parse("exec").unwrap(),
        args: Payload::new(serde_json::json!({"arm":{"program":{
            "path":".\\probe.exe", "args":["--exact", "tools::exec::native_windows::tests::native_windows_child_axes", "--ignored", "--nocapture"]
        }}}).as_object().unwrap().clone()).unwrap(),
    };
    let outcome = tool.invoke(&call).unwrap();
    let result = serde_json::to_value(outcome.result).unwrap();
    assert_eq!(result["exit_code"], 0, "{result}");
    assert_eq!(
        (
            &result["memory_ceiling"]["state"],
            &result["memory_ceiling"]["limit_bytes"]
        ),
        (
            &Value::from("hit"),
            &Value::from(u64::try_from(TEST_MEMORY_BYTES).unwrap())
        ),
        "the probe's refused allocation is reported as reaching the ceiling: {result}"
    );
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

#[test]
#[ignore = "writes AppContainer profiles and disposable ACLs; explicit Windows Actions acceptance only"]
fn native_windows_disposable_pwsh_initializes_network_types() {
    assert_eq!(std::env::var("SPRAWLING_DISPOSABLE_NATIVE").unwrap(), "1");
    let copy = tempfile::tempdir().unwrap();
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(6_000, 20));
    for (index, program) in ["powershell.exe", "pwsh"].into_iter().enumerate() {
        let mut command = Command::new(program);
        command.env_clear().env("PATH", std::env::var_os("PATH").unwrap())
        .current_dir(copy.path()).args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference='Stop'; [System.Net.ServicePointManager]::SecurityProtocol | Out-Null; [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass"]);
        let result = backlog
            .run_native(
                RunId::from_bytes([u8::try_from(index + 0x73).unwrap(); 16]),
                &Address::parse("work").unwrap(),
                "pwsh initialization".to_owned(),
                command,
            )
            .unwrap();
        let result = match result {
            crate::Started::Settled {
                exit,
                stdout,
                stderr,
                ceiling,
            } => crate::Started::Settled {
                exit,
                stdout: stdout.trim().to_owned(),
                stderr,
                ceiling,
            },
            background @ crate::Started::Backgrounded { .. } => background,
        };
        assert_eq!(
            result,
            crate::Started::Settled {
                exit: crate::Exit::Ended { code: 0 },
                stdout: "BelowNormal".to_owned(),
                stderr: String::new(),
                ceiling: None,
            }
        );
    }
}

#[test]
fn native_memory_is_unrequested_without_a_user_ceiling() {
    for shares in [Shares::Unset, Shares::Cpu] {
        assert_eq!(limits(shares).unwrap().memory, None);
    }
}

#[test]
#[ignore = "writes AppContainer profiles and disposable ACLs; explicit Windows Actions acceptance only"]
fn native_windows_disposable_argv_and_unrequested_memory() {
    use std::os::windows::ffi::OsStringExt;
    assert_eq!(std::env::var("SPRAWLING_DISPOSABLE_NATIVE").unwrap(), "1");
    let copy = tempfile::tempdir().unwrap();
    let directory = copy.path().join("path with spaces");
    std::fs::create_dir(&directory).unwrap();
    let source = directory.join("probe.rs");
    let exe = directory.join("probe.exe");
    std::fs::write(&source, r#"
use std::os::windows::ffi::OsStrExt;
fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("grant")) {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_HOME").unwrap());
        while !root.join(std::env::args_os().nth(2).unwrap()).exists() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert_eq!(std::fs::read_to_string(root.join("input")).unwrap(), "read only");
        assert!(std::fs::write(root.join("forbidden"), "write").is_err());
        println!("read only");
    } else if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("allocate")) {
        let mut bytes = Vec::<u8>::new();
        bytes.try_reserve_exact(320 * 1024 * 1024).unwrap();
        bytes.resize(320 * 1024 * 1024, 7);
        println!("{}", bytes.iter().map(|v| usize::from(*v)).sum::<usize>());
    } else {
        println!("{:?}", std::env::args_os().skip(1).map(|arg| arg.encode_wide().collect::<Vec<_>>()).collect::<Vec<_>>());
    }
}
"#).unwrap();
    let built = Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");
    let args = vec![
        vec![],
        vec![32, 9],
        vec![34, 92, 34],
        vec![92, 92],
        vec![0xd800, 32, 0xdc00],
        vec![97, 92, 34, 98, 92],
    ];
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(10_000, 5));
    let scope = Address::parse("work").unwrap();
    let mut command = Command::new(&exe);
    command
        .env_clear()
        .current_dir(&directory)
        .args(args.iter().map(|units| OsString::from_wide(units)));
    let result = backlog
        .run_native(
            RunId::from_bytes([0x74; 16]),
            &scope,
            "argv round trip".to_owned(),
            command,
        )
        .unwrap();
    let crate::Started::Settled {
        exit,
        stdout,
        stderr,
    } = result
    else {
        panic!("argv child must settle: {result:?}");
    };
    assert_eq!(exit, crate::Exit::Ended { code: 0 }, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Vec<Vec<u16>>>(&stdout).unwrap(),
        args
    );
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("input"), "read only").unwrap();
    let before = Command::new("pwsh")
        .env("NATIVE_ACL_FIXTURE", home.path())
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-Acl -LiteralPath $env:NATIVE_ACL_FIXTURE).Sddl",
        ])
        .output()
        .unwrap();
    assert!(before.status.success(), "{before:?}");
    let backlog = crate::Backlog::with_window(crate::PollBudget::new(100, 5));
    let copies = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
    for (index, copy) in copies.iter().enumerate() {
        let index = u8::try_from(index).unwrap();
        let probe = copy.path().join("probe.exe");
        std::fs::copy(&exe, &probe).unwrap();
        let mut command = Command::new(probe);
        command
            .env_clear()
            .env("CARGO_HOME", home.path())
            .current_dir(copy.path())
            .args(["grant", &index.to_string()]);
        assert!(matches!(
            backlog
                .run_native(
                    RunId::from_bytes([index + 0x80; 16]),
                    &scope,
                    "concurrent read-execute grant".to_owned(),
                    command
                )
                .unwrap(),
            crate::Started::Backgrounded { .. }
        ));
    }
    for index in 0..2_u8 {
        std::fs::write(home.path().join(index.to_string()), "open").unwrap();
        let owner = RunId::from_bytes([index + 0x80; 16]);
        let mut finished = None;
        for _ in 0..2_000 {
            if let Some(result) = backlog.harvest(owner).unwrap().into_iter().next() {
                finished = Some(result);
                break;
            }
            std::thread::park_timeout(std::time::Duration::from_millis(5));
        }
        let result = finished.expect("declared home reader settles");
        assert_eq!(
            (result.exit, result.stdout.trim(), result.stderr),
            (crate::Exit::Ended { code: 0 }, "read only", String::new())
        );
        backlog.release(owner);
    }
    assert!(!home.path().join("forbidden").exists());
    let after = Command::new("pwsh")
        .env("NATIVE_ACL_FIXTURE", home.path())
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-Acl -LiteralPath $env:NATIVE_ACL_FIXTURE).Sddl",
        ])
        .output()
        .unwrap();
    assert!(after.status.success(), "{after:?}");
    assert_eq!(
        after.stdout, before.stdout,
        "both unique SID grants are revoked"
    );
    for (index, shares) in [Shares::Unset, Shares::Cpu].into_iter().enumerate() {
        let backlog =
            crate::Backlog::with_window(crate::PollBudget::new(10_000, 5)).with_shares(shares);
        let mut command = Command::new(&exe);
        command.env_clear().current_dir(&directory).arg("allocate");
        let result = backlog
            .run_native(
                RunId::from_bytes([u8::try_from(index).unwrap(); 16]),
                &scope,
                "unrequested memory".to_owned(),
                command,
            )
            .unwrap();
        assert_eq!(
            result,
            crate::Started::Settled {
                exit: crate::Exit::Ended { code: 0 },
                stdout: "2348810240\n".to_owned(),
                stderr: String::new(),
                ceiling: None,
            }
        );
    }
}
