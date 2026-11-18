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

pub fn serve_with(
    count: usize,
    mut respond: impl FnMut(&Value, usize) -> Value + Send + 'static,
) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for index in 0..count {
            let (stream, _) = listener.accept().unwrap();
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

#[expect(clippy::disallowed_methods, reason = "test fixture process")]
pub fn command(base: &str, prompt: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mei"));
    command
        .args(["-p", prompt])
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", base)
        .env("ANTHROPIC_API_KEY", "magpie")
        .env("MEI_DOCTOR_DIR", doctor_directory())
        .env_remove("HARNESS_EXTRA");
    command
}

/// A doctor cache shared by every test process of this build, under the
/// directory Cargo gives integration tests and `cargo clean` reclaims.
///
/// `write_environment` publishes by renaming a process-named candidate, so
/// concurrent first starts converge on the same record instead of exhausting
/// the temporary directory one `TempDir` per process.
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn doctor_directory() -> &'static std::path::Path {
    static FIXTURE: OnceLock<std::path::PathBuf> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("doctor");
            std::fs::create_dir_all(&root).unwrap();
            let probe = Command::new(if cfg!(windows) { "python" } else { "python3" })
                .args([
                    "-I",
                    "-c",
                    "import platform, sys; print(platform.python_version()); print(sys.executable)",
                ])
                .output()
                .unwrap();
            assert!(probe.status.success());
            let text = String::from_utf8(probe.stdout).unwrap();
            let mut lines = text.lines();
            let executable = std::path::PathBuf::from(lines.next_back().unwrap());
            let version = lines.next_back().unwrap();
            let python = mei::doctor::Program::new(version, executable.clone());
            // An interpreter answers `--version` with a numeric token, so the
            // same fixture stays usable if a cache is ever rebuilt on a machine
            // without uv: no test reaches the network for a stand-in.
            let uv = mei::doctor::Program::new(version, executable);
            mei::doctor::write_cache(&root, &mei::doctor::Environment::new(python, uv)).unwrap();
            root
        })
        .as_path()
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
