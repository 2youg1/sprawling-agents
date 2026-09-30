// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The socket and the record: reading one request off a connection,
//! writing its answer back, and appending the exchange to a file
//! (citysim-SPEC.md 8-10).
//!
//! **What counts as a request is settled in one place, `read_request`.**
//! A head up to its blank line and a body of `content-length` bytes is a
//! request; a connection that ended before that is not, and it is
//! neither answered nor recorded nor charged a reply, because a reply
//! spent on it would make every later turn answer the question before
//! it.

use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use super::{Asked, Replay, WireScript};

/// How long a connection may stay silent before it is given up on as
/// carrying no request. The city writes a whole request at once; a
/// connection quiet this long was opened by something else.
const QUIET: Duration = Duration::from_secs(30);

/// The largest request this provider reads. The city's longest prompt is
/// a few megabytes; a stream past this is not a request from it.
const LARGEST: usize = 64 * 1024 * 1024;

/// The value a credential header is recorded with.
const REDACTED: &str = "redacted";

/// A loopback provider playing one script, one connection at a time,
/// recording every exchange.
pub struct ScriptedProvider {
    listener: TcpListener,
    replay: Replay,
    record: Record,
    seq: u64,
}

/// The file every exchange of one playing is appended to, and the path
/// a failure to write it names.
struct Record {
    file: File,
    path: PathBuf,
}

impl Record {
    fn create(path: &Path) -> Result<Record, AxError> {
        File::create(path)
            .map(|file| Record {
                file,
                path: path.to_path_buf(),
            })
            .map_err(|err| unrecorded(path, &err))
    }

    /// One exchange, as one line.
    fn append(&mut self, exchange: &Value) -> Result<(), AxError> {
        self.file
            .write_all(
                format!(
                    "{exchange}
"
                )
                .as_bytes(),
            )
            .map_err(|err| unrecorded(&self.path, &err))
    }
}

impl ScriptedProvider {
    /// Takes the listener and empties `record`: one playing, one record.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the record cannot be created.
    pub fn open(
        listener: TcpListener,
        script: WireScript,
        record: &Path,
    ) -> Result<ScriptedProvider, AxError> {
        Ok(ScriptedProvider {
            listener,
            replay: Replay::new(script),
            record: Record::create(record)?,
            seq: 0,
        })
    }

    /// The base URL a city attaches this provider under.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the listener cannot say its address.
    pub fn url(&self) -> Result<String, AxError> {
        self.listener
            .local_addr()
            .map(|addr| format!("http://{addr}/v1"))
            .map_err(|err| socket("say where the provider listens", &err))
    }

    /// Accepts one connection, reads one request off it, records the
    /// exchange and writes the answer.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the record cannot be appended to, and
    /// `E_TOOL_UNAVAILABLE` when the connection fails; the first ends a
    /// playing, the second ends only that connection.
    pub fn answer_one(&mut self) -> Result<(), AxError> {
        let (mut stream, _peer) = self
            .listener
            .accept()
            .map_err(|err| socket("accept a connection", &err))?;
        stream
            .set_read_timeout(Some(QUIET))
            .map_err(|err| socket("bound a connection's silence", &err))?;
        let Some(request) = read_request(&mut stream) else {
            return Ok(());
        };
        let answer = self.replay.answer(Asked::of(&request.method));
        let (status, reason) = answer.status();
        let body = answer.body();
        self.seq = self.seq.saturating_add(1);
        self.record.append(&json!({
            "seq": self.seq,
            "method": request.method,
            "target": request.target,
            "headers": request.headers,
            "body": request.body,
            "status": status,
            "answer": body,
        }))?;
        let text = body.to_string();
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
            text.len()
        );
        stream
            .write_all(response.as_bytes())
            .and_then(|()| stream.flush())
            .map_err(|err| socket("write an answer", &err))
    }
}

/// One request as it arrived: the request line's two words, the headers
/// in arrival order with their names lowercased and credentials
/// redacted, and the body's text.
struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: String,
}

/// Reads one whole request, or `None` when the connection ended, fell
/// silent or grew past `LARGEST` before one arrived.
fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    let mut wanted: Option<(usize, usize)> = None;
    loop {
        if let Some((head_end, length)) = wanted
            && bytes.len() >= head_end.checked_add(length)?
        {
            return request_of(&bytes, head_end, length);
        }
        let read = match stream.read(&mut chunk) {
            Ok(0) => return None,
            Ok(read) => read,
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        bytes.extend_from_slice(chunk.get(..read)?);
        if bytes.len() > LARGEST {
            return None;
        }
        if wanted.is_none() {
            wanted =
                head_end(&bytes).and_then(|end| Some((end, content_length(bytes.get(..end)?)?)));
        }
    }
}

/// Where the head ends: the index just past its blank line.
fn head_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .and_then(|at| at.checked_add(4))
}

/// The body length the head states; absent is zero, unreadable is no
/// request at all.
fn content_length(head: &[u8]) -> Option<usize> {
    String::from_utf8_lossy(head)
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .map_or(Some(0), |(_, value)| value.trim().parse().ok())
}

fn request_of(bytes: &[u8], head_end: usize, length: usize) -> Option<Request> {
    let head = String::from_utf8_lossy(bytes.get(..head_end)?);
    let body = bytes.get(head_end..head_end.checked_add(length)?)?;
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let (method, target) = (first.next()?.to_owned(), first.next()?.to_owned());
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| {
            let name = name.trim().to_ascii_lowercase();
            let value = if is_credential(&name) {
                REDACTED.to_owned()
            } else {
                value.trim().to_owned()
            };
            (name, value)
        })
        .collect();
    Some(Request {
        method,
        target,
        headers,
        body: String::from_utf8_lossy(body).into_owned(),
    })
}

/// Whether a header carries a credential, whose value never reaches the
/// record: a check needs to know one was sent, not what it was.
fn is_credential(name: &str) -> bool {
    matches!(name, "authorization" | "proxy-authorization") || name.ends_with("api-key")
}

fn socket(action: &str, err: &std::io::Error) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, action.to_owned(), err.to_string()).with_recovery(
        "the scripted provider answers the next connection; a check that needed this one \
         sends its request again",
    )
}

fn unrecorded(record: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "record an exchange",
        format!("{}: {err}", record.display()),
    )
    .with_recovery(
        "give the provider a record path in a directory it can write, and start it again",
    )
}
