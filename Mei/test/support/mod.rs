// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Output};
use std::sync::OnceLock;

use serde_json::Value;

pub fn read_request_body(reader: &mut BufReader<std::net::TcpStream>) -> Vec<u8> {
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            panic!("client closed before sending complete request headers");
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            length = value.trim().parse::<usize>().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    body
}

#[expect(clippy::disallowed_methods, reason = "test gateway accept deadline")]
pub fn serve_with(
    count: usize,
    mut respond: impl FnMut(&Value, usize) -> Value + Send + 'static,
) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for index in 0..count {
            let started = std::time::Instant::now();
            let (stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            started.elapsed() < std::time::Duration::from_secs(10),
                            "gateway request {index} did not arrive"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(err) => panic!("gateway accept failed: {err}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            let mut reader = BufReader::new(stream);
            let body = read_request_body(&mut reader);
            let request = serde_json::from_slice(&body).unwrap();
            requests.push(request);
            let body = serde_json::to_vec(&respond(requests.last().unwrap(), index)).unwrap();
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
        }
        requests
    });
    (format!("http://{address}"), handle)
}

#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
pub fn serve(answers: Vec<Value>) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    let mut answers = answers.into_iter();
    let count = answers.len();
    serve_with(count, move |_, _| answers.next().unwrap())
}

/// What one served request answers.
#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
pub enum Answer {
    Json(Value),
    Hang(std::time::Duration),
}

/// Serves `count` requests on one listener, dispatching on the request path so
/// a catalog GET, a Messages POST and a route GET can answer differently.
#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
#[expect(clippy::disallowed_methods, reason = "test gateway accept deadline")]
pub fn serve_paths(
    count: usize,
    mut respond: impl FnMut(&str, &Value, usize) -> Answer + Send + 'static,
) -> (String, std::thread::JoinHandle<Vec<(String, Value)>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for index in 0..count {
            let started = std::time::Instant::now();
            let (stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            started.elapsed() < std::time::Duration::from_secs(10),
                            "gateway request {index} did not arrive"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(err) => panic!("gateway accept failed: {err}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            let mut reader = BufReader::new(stream);
            let (path, request) = read_request(&mut reader);
            let answer = respond(&path, &request, index);
            requests.push((path, request));
            match answer {
                Answer::Json(value) => {
                    let body = serde_json::to_vec(&value).unwrap();
                    let mut stream = reader.into_inner();
                    write!(
                        stream,
                        "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(&body).unwrap();
                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                }
                Answer::Hang(delay) => {
                    std::thread::sleep(delay);
                    reader
                        .into_inner()
                        .shutdown(std::net::Shutdown::Both)
                        .unwrap();
                }
            }
        }
        requests
    });
    (format!("http://{address}"), handle)
}

fn read_request(reader: &mut BufReader<std::net::TcpStream>) -> (String, Value) {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let path = match line.split_whitespace().nth(1) {
        Some(found) => found.to_owned(),
        None => String::new(),
    };
    let body = read_request_body(reader);
    let request = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (path, request)
}

#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
pub fn tool_result_text(content: &Value) -> String {
    content
        .as_str()
        .expect("tool_result content is text")
        .replace("\r\n", "\n")
}

pub struct FixtureCommand {
    command: Command,
    _doctor: tempfile::TempDir,
}

impl std::ops::Deref for FixtureCommand {
    type Target = Command;

    fn deref(&self) -> &Command {
        &self.command
    }
}

impl std::ops::DerefMut for FixtureCommand {
    fn deref_mut(&mut self) -> &mut Command {
        &mut self.command
    }
}

#[expect(clippy::disallowed_methods, reason = "test fixture process")]
pub fn command(base: &str, prompt: &str) -> FixtureCommand {
    let doctor = doctor_directory();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mei"));
    command
        .args(["-p", prompt])
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", base)
        .env("ANTHROPIC_API_KEY", "magpie")
        .env("MEI_DOCTOR_DIR", doctor.path())
        .env_remove("HARNESS_EXTRA")
        .env_remove("MEI_CAPTURE_DIR")
        .env_remove("MEI_CAPTURE_CREATED");
    FixtureCommand {
        command,
        _doctor: doctor,
    }
}

#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn doctor_directory() -> tempfile::TempDir {
    static PYTHON: OnceLock<mei::doctor::Program> = OnceLock::new();
    let python = PYTHON.get_or_init(|| {
        let probe = Command::new(if cfg!(windows) { "python" } else { "python3" })
            .args([
                "-I",
                "-c",
                "import json, os, platform, sys; raw = sys.executable.encode('utf-16-le') if os.name == 'nt' else os.fsencode(sys.executable); print(json.dumps({'version': platform.python_version(), 'executable': list(raw)}))",
            ])
            .output()
            .unwrap();
        assert!(probe.status.success());
        let value: Value = serde_json::from_slice(&probe.stdout).unwrap();
        let version = value["version"].as_str().unwrap();
        let executable = native_path(&value["executable"]);
        mei::doctor::Program::new(version, executable)
    });
    let root = tempfile::tempdir().unwrap();
    let environment = mei::doctor::Environment::new(python.clone(), python.clone());
    mei::doctor::write_cache(root.path(), &environment).unwrap();
    root
}

#[cfg(unix)]
fn native_path(value: &Value) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStringExt as _;
    let bytes = value
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
        .collect::<Vec<_>>();
    std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

#[cfg(windows)]
fn native_path(value: &Value) -> std::path::PathBuf {
    use std::os::windows::ffi::OsStringExt as _;
    let bytes = value
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
        .collect::<Vec<_>>();
    let (pairs, remainder) = bytes.as_chunks::<2>();
    assert!(remainder.is_empty());
    let wide = pairs
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect::<Vec<_>>();
    std::path::PathBuf::from(std::ffi::OsString::from_wide(&wide))
}

#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
pub fn run(base: &str, prompt: &str, harness: Option<&str>) -> Output {
    let mut command = command(base, prompt);
    if let Some(harness) = harness {
        command.env("HARNESS", harness);
    } else {
        command.env_remove("HARNESS");
    }
    command.output().unwrap()
}

#[allow(
    dead_code,
    reason = "shared support is compiled per integration test target"
)]
pub fn run_resume(base: &str, prompt: &str, path: &std::path::Path) -> Output {
    command(base, prompt)
        .arg("--resume")
        .arg(path)
        .output()
        .unwrap()
}
