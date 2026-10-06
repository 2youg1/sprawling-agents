// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use std::process::Command;

fn native_tool(source: &std::path::Path, scratch: &std::path::Path, backlog: Backlog) -> ExecTool {
    ExecTool::new(
        setup(source, None, Some(PathBuf::from("/bin/sh"))),
        Box::new(EchoSandbox::new()),
        backlog,
    )
    .unwrap()
    .confined(Confined::with_arm(
        Confinement::MacosSeatbelt {
            wrapper: PathBuf::from("/usr/bin/sandbox-exec"),
        },
        Some(scratch.to_path_buf()),
    ))
}

fn invoke_python(tool: &ExecTool, script: &str, args: &[String]) -> Value {
    let mut arguments = vec!["-c".to_owned(), script.to_owned()];
    arguments.extend_from_slice(args);
    let result = tool
        .invoke(&call(serde_json::json!({
            "program": {"path": "/usr/bin/python3", "args": arguments}
        })))
        .unwrap();
    serde_json::to_value(result.result).unwrap()
}

#[test]
fn native_macos_exec_writes_only_the_copy_and_preserves_output_and_exit() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let tool = native_tool(source.path(), scratch.path(), patient());
    let outcome = tool.invoke(&call(serde_json::json!({
        "program": {"path": "/bin/sh", "args": ["-c", "printf copy > written; printf 'target-output\n'; printf 'target-error\n' >&2; exit 23"]}
    }))).unwrap();
    let result = serde_json::to_value(outcome.result).unwrap();
    assert_eq!(
        (
            result["exit_code"].as_i64(),
            result["stdout"].as_str(),
            result["stderr"].as_str()
        ),
        (Some(23), Some("target-output\n"), Some("target-error\n")),
        "{result}"
    );
    assert!(!source.path().join("written").exists());
    assert!(tool.meta().disclosure.contains("macos_seatbelt"));
    assert!(
        tool.meta()
            .disclosure
            .contains(crate::tools::Guarantee::Resources.unkept())
    );
    let entries: Vec<_> = std::fs::read_dir(scratch.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        std::fs::read_to_string(entries[0].join("written")).unwrap(),
        "copy"
    );
    drop(tool);
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn native_macos_exec_rejects_absolute_and_new_symlink_escape_writes() {
    let source = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let protected = outside.path().join("protected");
    std::fs::write(&protected, "before").unwrap();
    let tool = native_tool(source.path(), scratch.path(), patient());
    let script = "import pathlib,sys,os; p=pathlib.Path(sys.argv[1]); os.symlink(str(p),'escape'); pathlib.Path('escape').write_text('after')";
    let control = Command::new("/usr/bin/python3")
        .args([
            "-c",
            "import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('control')",
        ])
        .arg(&protected)
        .output()
        .unwrap();
    assert!(control.status.success());
    std::fs::write(&protected, "before").unwrap();
    for script in [
        script,
        "import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('after')",
        "import pathlib,sys,os; os.link(sys.argv[1],'hard-escape'); pathlib.Path('hard-escape').write_text('after')",
        "import os,sys; os.rename(sys.argv[1],'moved-in')",
    ] {
        let result = invoke_python(&tool, script, &[protected.display().to_string()]);
        assert_ne!(result["exit_code"], 0, "{result}");
        assert!(
            result["stderr"]
                .as_str()
                .unwrap()
                .contains("PermissionError"),
            "target must execute before refusal: {result}"
        );
        assert_eq!(std::fs::read_to_string(&protected).unwrap(), "before");
    }
}

#[test]
fn native_macos_exec_denies_tcp_udp_and_descendant_network_with_successful_controls() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let tool = native_tool(source.path(), scratch.path(), patient());
    let tcp4 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tcp6 = std::net::TcpListener::bind("[::1]:0").unwrap();
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let script = "import socket,sys\ntry:\n s=socket.socket(getattr(socket,sys.argv[1]),getattr(socket,sys.argv[2])); s.settimeout(2); s.connect((sys.argv[3],int(sys.argv[4]))); s.send(b'x'); print('CONNECTED')\nexcept OSError as e:\n print('DENIED:'+str(e.errno)); sys.exit(77)";
    for (family, kind, host, port) in [
        (
            "AF_INET",
            "SOCK_STREAM",
            "127.0.0.1",
            tcp4.local_addr().unwrap().port(),
        ),
        (
            "AF_INET6",
            "SOCK_STREAM",
            "::1",
            tcp6.local_addr().unwrap().port(),
        ),
        (
            "AF_INET",
            "SOCK_DGRAM",
            "127.0.0.1",
            udp.local_addr().unwrap().port(),
        ),
    ] {
        let args = vec![
            family.to_string(),
            kind.to_string(),
            host.to_owned(),
            port.to_string(),
        ];
        let direct = Command::new("/usr/bin/python3")
            .args(["-c", script])
            .args(&args)
            .output()
            .unwrap();
        assert!(
            direct.status.success(),
            "unconfined control failed: {direct:?}"
        );
        assert_eq!(String::from_utf8_lossy(&direct.stdout), "CONNECTED\n");
        let denied = invoke_python(&tool, script, &args);
        assert_eq!(denied["exit_code"], 77, "{denied}");
        assert!(
            denied["stdout"].as_str().unwrap().contains("DENIED:1")
                || denied["stdout"].as_str().unwrap().contains("DENIED:13"),
            "{denied}"
        );
        let parent = "import subprocess,sys; sys.exit(subprocess.run([sys.executable,'-c',sys.argv[1],*sys.argv[2:]]).returncode)";
        let mut child_args = vec![script.to_owned()];
        child_args.extend(args);
        let descendant = invoke_python(&tool, parent, &child_args);
        assert_eq!(descendant["exit_code"], 77, "{descendant}");
        assert!(
            descendant["stdout"].as_str().unwrap().contains("DENIED:"),
            "{descendant}"
        );
    }
}

#[test]
fn native_macos_initialization_failure_never_runs_target_or_retains_copy() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let missing = source.path().join("missing-wrapper");
    let tool = ExecTool::new(
        setup(source.path(), None, Some(PathBuf::from("/bin/sh"))),
        Box::new(EchoSandbox::new()),
        patient(),
    )
    .unwrap()
    .confined(Confined::with_arm(
        Confinement::MacosSeatbelt { wrapper: missing },
        Some(scratch.path().to_path_buf()),
    ));
    let error = tool.invoke(&call(serde_json::json!({"program":{"path":"/usr/bin/touch","args":[source.path().join("side-effect")]}}))).unwrap_err();
    assert_eq!(*error.code(), AxCode::SandboxDenied);
    assert!(!source.path().join("side-effect").exists());
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

fn await_condition(mut condition: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !condition() {
        assert!(
            std::time::Instant::now() < deadline,
            "native lifecycle condition timed out"
        );
        std::thread::park_timeout(std::time::Duration::from_millis(5));
    }
}

fn gated_command(tool: &ExecTool, scratch: &std::path::Path) -> (Value, PathBuf, u32) {
    let result = invoke_python(tool,
        "import os,pathlib,time; pathlib.Path('ready').write_text(str(os.getpid()))
while not pathlib.Path('continue').exists(): time.sleep(0.01)
pathlib.Path('late').write_text('copy'); print('later-out'); print('later-err',file=__import__('sys').stderr); raise SystemExit(7)", &[]);
    assert_eq!(result["outcome"], "backgrounded", "{result}");
    let copies: Vec<_> = std::fs::read_dir(scratch)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(copies.len(), 1);
    let copy = copies[0].clone();
    await_condition(|| {
        std::fs::read_to_string(copy.join("ready")).is_ok_and(|text| text.parse::<u32>().is_ok())
    });
    let pid = std::fs::read_to_string(copy.join("ready"))
        .unwrap()
        .parse()
        .unwrap();
    (result["handle"].clone(), copy, pid)
}

#[test]
fn native_macos_background_keeps_its_copy_and_reports_one_terminal_result() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let (handle, copy, _) = gated_command(&tool, scratch.path());
    let owner = tool.setup.run;
    let stranger = kernel::RunId::from_bytes([2; 16]);
    assert!(backlog.harvest(stranger).unwrap().is_empty());
    assert!(backlog.harvest(owner).unwrap().is_empty());
    assert!(copy.is_dir());
    std::fs::write(copy.join("continue"), "go").unwrap();
    let mut finished = None;
    await_condition(|| {
        assert!(backlog.harvest(stranger).unwrap().is_empty());
        let result = invoke_python(&tool, "pass", &[]);
        finished = result["background"].as_array().and_then(|members| {
            members
                .iter()
                .find(|member| member["handle"] == handle)
                .cloned()
        });
        finished.is_some()
    });
    let finished = finished.unwrap();
    assert_eq!(
        (
            finished["exit_code"].as_i64(),
            finished["stdout"].as_str(),
            finished["stderr"].as_str()
        ),
        (
            Some(7),
            Some(
                "later-out
"
            ),
            Some(
                "later-err
"
            )
        ),
        "{finished}"
    );
    assert!(
        backlog
            .harvest(owner)
            .unwrap()
            .iter()
            .all(|member| member.id.to_string() != handle.as_str().unwrap())
    );
    assert!(!source.path().join("late").exists());
    drop(tool);
    await_condition(|| {
        backlog.harvest(stranger).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    });
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn native_macos_halt_and_tool_release_reap_the_owned_primary_and_copy() {
    for halt in [true, false] {
        let source = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
        let tool = native_tool(source.path(), scratch.path(), backlog.clone());
        let (_, copy, pid) = gated_command(&tool, scratch.path());
        if halt {
            assert_eq!(
                backlog
                    .halt(Some(&Address::parse("work").unwrap()))
                    .unwrap(),
                1
            );
        }
        drop(tool);
        await_condition(|| {
            backlog.harvest(kernel::RunId::from_bytes([2; 16])).unwrap();
            backlog
                .standing(&Address::parse("work").unwrap())
                .unwrap()
                .is_empty()
        });
        assert!(
            !Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .output()
                .unwrap()
                .status
                .success(),
            "owned primary still exists"
        );
        assert!(!copy.exists());
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
    }
}

#[test]
fn native_macos_release_leaves_another_owners_background_command_alive() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let other_scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let mut other_setup = setup(source.path(), None, Some(PathBuf::from("/bin/sh")));
    other_setup.run = kernel::RunId::from_bytes([2; 16]);
    let other = ExecTool::new(other_setup, Box::new(EchoSandbox::new()), backlog.clone())
        .unwrap()
        .confined(Confined::with_arm(
            Confinement::MacosSeatbelt {
                wrapper: PathBuf::from("/usr/bin/sandbox-exec"),
            },
            Some(other_scratch.path().to_path_buf()),
        ));
    gated_command(&tool, scratch.path());
    let (_, other_copy, other_pid) = gated_command(&other, other_scratch.path());
    drop(tool);
    await_condition(|| {
        backlog.harvest(kernel::RunId::from_bytes([3; 16])).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .len()
            == 1
    });
    assert!(
        Command::new("/bin/kill")
            .args(["-0", &other_pid.to_string()])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(other_copy.is_dir());
    drop(other);
    await_condition(|| {
        backlog.harvest(kernel::RunId::from_bytes([3; 16])).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    });
    assert!(!other_copy.exists());
}
