// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use std::process::Command;

fn native_tool(source: &std::path::Path, scratch: &std::path::Path) -> ExecTool {
    ExecTool::new(
        setup(source, None, Some(PathBuf::from("/bin/sh"))),
        Box::new(EchoSandbox::new()),
        patient(),
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
    let tool = native_tool(source.path(), scratch.path());
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
    let tool = native_tool(source.path(), scratch.path());
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
    let tool = native_tool(source.path(), scratch.path());
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
