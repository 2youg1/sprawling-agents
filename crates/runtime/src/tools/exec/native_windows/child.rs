// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Child protocol fixture specified by `crates/runtime/spec/Tools/Exec/NativeWindows.lean`.

pub(super) const SOURCE: &str = r#"
use std::os::windows::ffi::OsStrExt;
fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("exit")) {
        return;
    } else if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("grant")) {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_HOME").unwrap());
        while !root.join(std::env::args_os().nth(2).unwrap()).exists() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert_eq!(std::fs::read_to_string(root.join("protected/input")).unwrap(), "read only");
        assert!(std::fs::write(root.join("forbidden"), "write").is_err());
        println!("read only");
    } else if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("pipes")) {
        use std::process::{Command, Stdio};
        let program = std::env::current_exe().unwrap();
        let inherited = Command::new(&program).arg("exit").status().unwrap();
        assert!(inherited.success());
        println!("INHERITED_STDIO_EXIT=0");
        let captured = Command::new(&program).arg("exit")
            .stdin(Stdio::inherit()).stderr(Stdio::inherit()).stdout(Stdio::piped()).spawn();
        match captured {
            Ok(mut child) => println!("PIPED_STDIO_EXIT={:?}", child.wait().unwrap().code()),
            Err(error) => println!("PIPED_STDIO_ERROR={:?}", error.raw_os_error()),
        }
        match Command::new(&program).arg("exit").output() {
            Ok(output) => println!("DEFAULT_CAPTURE_EXIT={:?}", output.status.code()),
            Err(error) => println!("DEFAULT_CAPTURE_ERROR={:?}", error.raw_os_error()),
        }
        match std::fs::OpenOptions::new().read(true).write(true).open("NUL") {
            Ok(file) => { drop(file); println!("NUL_DEVICE_OPEN=OK"); }
            Err(error) => println!("NUL_DEVICE_ERROR={:?}", error.raw_os_error()),
        }
    } else if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("allocate")) {
        let mut bytes = Vec::<u8>::new();
        bytes.try_reserve_exact(320 * 1024 * 1024).unwrap();
        bytes.resize(320 * 1024 * 1024, 7);
        println!("{}", bytes.iter().map(|v| usize::from(*v)).sum::<usize>());
    } else {
        println!("{:?}", std::env::args_os().skip(1).map(|arg| arg.encode_wide().collect::<Vec<_>>()).collect::<Vec<_>>());
    }
}
"#;
