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

#[test]
fn native_macos_background_keeps_its_copy_and_reports_one_terminal_result() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let tool = native_tool(
        source.path(),
        scratch.path(),
        Backlog::with_window(crate::backlog::PollBudget::new(1, 1)),
    );
    let result = tool.invoke(&call(serde_json::json!({
        "program": {"path": "/bin/sh", "args": ["-c", "sleep 1; printf late > late; printf 'later-out
'; printf 'later-err
' >&2; exit 7"]}
    }))).unwrap();
    let result = serde_json::to_value(result.result).unwrap();
    assert_eq!(result["outcome"], "backgrounded");
    let handle = result["handle"].clone();
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 1);
    let finished = (0..100)
        .find_map(|_| {
            let result = tool
                .invoke(&call(serde_json::json!({
                    "program": {"path": "/usr/bin/true", "args": []}
                })))
                .unwrap();
            let result = serde_json::to_value(result.result).unwrap();
            result["background"].as_array().and_then(|members| {
                members
                    .iter()
                    .find(|member| member["handle"] == handle)
                    .cloned()
            })
        })
        .expect("native background did not finish within bounded actual invocations");
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
        )
    );
    assert!(!source.path().join("late").exists());
    drop(tool);
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn native_macos_halt_and_tool_release_reap_the_owned_primary_and_copy() {
    for halt in [true, false] {
        let source = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
        let tool = native_tool(source.path(), scratch.path(), backlog.clone());
        let result = tool
            .invoke(&call(serde_json::json!({
                "program": {"path": "/bin/sh", "args": ["-c", "while :; do :; done"]}
            })))
            .unwrap();
        let result = serde_json::to_value(result.result).unwrap();
        assert_eq!(result["outcome"], "backgrounded");
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 1);
        if halt {
            assert_eq!(
                backlog
                    .halt(Some(&Address::parse("work").unwrap()))
                    .unwrap(),
                1
            );
        }
        drop(tool);
        let domain = Address::parse("work").unwrap();
        for _ in 0..1_000_000 {
            backlog.harvest(kernel::RunId::from_bytes([1; 16])).unwrap();
            if backlog.standing(&domain).unwrap().is_empty() {
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            backlog.standing(&domain).unwrap().is_empty(),
            "owned native primary did not stop"
        );
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
    }
}
