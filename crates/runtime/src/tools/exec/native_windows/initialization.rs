// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Disposable AppContainer initialization of both Windows PowerShell runtimes,
//! specified by `crates/runtime/spec/Tools/Exec/NativeWindows.lean`.
use super::*;
use kernel::{Address, RunId};

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
        if !matches!(
            result,
            crate::Started::Settled {
                exit: crate::Exit::Ended { code: 0 },
                ..
            }
        ) {
            diagnose_framework(&backlog, copy.path());
        }
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

fn diagnose_framework(backlog: &crate::Backlog, directory: &Path) {
    let source = directory.join("framework.cs");
    let exe = directory.join("framework.exe");
    std::fs::write(&source, r#"
class Probe {
    static int Main() {
        try { System.Console.WriteLine(System.Net.ServicePointManager.SecurityProtocol); return 0; }
        catch (System.Exception error) { System.Console.Error.WriteLine(error.ToString()); return 1; }
    }
}"#).unwrap();
    let compiler = desktop_ffi::confinement::windows_root()
        .unwrap()
        .join("Microsoft.NET/Framework64/v4.0.30319/csc.exe");
    let output = Command::new(compiler)
        .arg("/nologo")
        .arg(format!("/out:{}", exe.display()))
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let mut command = Command::new(exe);
    command.env_clear().current_dir(directory);
    let result = backlog
        .run_native(
            RunId::from_bytes([0x79; 16]),
            &Address::parse("work").unwrap(),
            "framework initialization diagnostic".to_owned(),
            command,
        )
        .unwrap();
    eprintln!("FRAMEWORK_INITIALIZATION={result:?}");
}
