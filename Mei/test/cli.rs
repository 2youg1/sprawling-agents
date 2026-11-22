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

mod agent_loop;
mod support;

use std::process::Command;

use serde_json::Value;

#[cfg(unix)]
fn non_unicode_argument() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![0xff])
}

#[cfg(windows)]
fn non_unicode_argument() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt as _;
    std::ffi::OsString::from_wide(&[0xd800])
}

#[test]
#[expect(clippy::disallowed_methods, reason = "argument fixture process")]
fn cli_non_unicode_prompt_is_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .arg("-p")
        .arg(non_unicode_argument())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_USAGE"), "{error}");
    assert!(!error.contains("panicked"), "{error}");
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "TTY fixture process for crates.io face hint")]
fn tty_mei_without_prompt_points_at_the_calyx_install() {
    let mei = env!("CARGO_BIN_EXE_mei");
    let mut command = Command::new("script");
    #[cfg(target_os = "linux")]
    {
        command.args(["-q", "-e", "-c", mei, "/dev/null"]);
    }
    #[cfg(not(target_os = "linux"))]
    {
        command.args(["-q", "/dev/null", mei]);
    }
    let output = command
        .output()
        .expect("script provides a PTY for the headless mei TTY path");
    let error = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{error}{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        combined.contains("E_MEI_USAGE"),
        "status={} stderr={error} stdout={}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        combined.contains("@sprawling/mei") || combined.contains("calyx"),
        "{combined}"
    );
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn invalid_gateway_configuration_precedes_doctor_side_effects() {
    let directory = tempfile::tempdir().unwrap();
    let blocked = directory.path().join("not-a-directory");
    std::fs::write(&blocked, "block").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["-p", "hello"])
        .current_dir(directory.path())
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", "https://gateway.invalid")
        .env("ANTHROPIC_API_KEY", "magpie")
        .env("MEI_DOCTOR_DIR", &blocked)
        .env_remove("HARNESS_EXTRA")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_CONFIG"), "{error}");
    assert!(!error.contains("E_MEI_DOCTOR"), "{error}");
}

#[test]
fn absent_model_uses_the_magpie_route_and_reports_actual_effort() {
    use std::io::{BufReader, Write as _};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let gateway = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for index in 0..3 {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            std::io::BufRead::read_line(&mut reader, &mut request_line).unwrap();
            let body = crate::support::read_request_body(&mut reader);
            let request = if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice::<Value>(&body).unwrap()
            };
            seen.push((request_line.clone(), request));
            let response = match index {
                0 => serde_json::json!({
                    "first_id": "group/plan-first",
                    "data": [{"id": "group/plan-first", "context_window": 1048576, "reasoning_levels": ["none", "low", "high", "max"]}]
                }),
                1 => serde_json::json!({
                    "stop_reason": "end_turn",
                    "content": "done",
                    "usage": {"input_tokens": 231, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 256}
                }),
                2 => {
                    serde_json::json!({"route": {"model": "kimi-code-cn/kimi-for-coding", "effort": "high"}})
                }
                _ => panic!("unexpected gateway request"),
            };
            let bytes = serde_json::to_vec(&response).unwrap();
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
        }
        seen
    });
    let output = crate::support::command(&base, "hello")
        .env_remove("HARNESS_MODEL")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"done");
    let seen = gateway.join().unwrap();
    assert!(seen[0].0.starts_with("GET /v1/models "));
    assert!(seen[1].0.starts_with("POST /v1/messages "));
    assert_eq!(seen[1].1["model"], "group/plan-first");
    assert_eq!(
        seen[1].1["thinking"],
        serde_json::json!({"type": "adaptive"})
    );
    assert_eq!(seen[1].1["output_config"]["effort"], "high");
    assert!(seen[2].0.starts_with("GET /v1/magpie/route?session=mei-"));
    let trace = String::from_utf8(output.stderr).unwrap();
    assert!(trace.contains("\"kind\":\"model\""), "{trace}");
    assert!(trace.contains("\"context_tokens\":1048576"), "{trace}");
    assert!(trace.contains("\"kind\":\"route\""), "{trace}");
    assert!(trace.contains("kimi-code-cn/kimi-for-coding"), "{trace}");
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn empty_model_is_an_actionable_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .arg("-p")
        .arg("hello")
        .env("HARNESS_MODEL", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_CONFIG"));
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn non_unicode_configuration_is_not_treated_as_unset() {
    use std::os::unix::ffi::OsStringExt;

    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["-p", "hello"])
        .env("HARNESS_MODEL", "test/model")
        .env("HARNESS_EXTRA", std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_CONFIG"), "{error}");
    assert!(error.contains("not Unicode"), "{error}");
}
