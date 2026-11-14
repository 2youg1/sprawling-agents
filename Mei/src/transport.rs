// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The smallest non-streaming Anthropic HTTP adapter (`Mei/Spec.lean`
//! §5, §8-§10). No proxy, TLS, compression, redirect, chunked or SSE.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::Value;

use crate::error::{Code, Diagnostic, Error, Result, protocol};

/// Per-address TCP connect budget. The gateway is normally loopback, so a
/// handshake with no answer in this long is a dead gateway, not a slow one.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read and write budget for one complete non-streaming response. A model
/// may legitimately think for minutes, so this stays generous; it exists so
/// a gateway that accepts a connection and then stalls forever returns a
/// diagnostic instead of hanging the run.
const IO_TIMEOUT: Duration = Duration::from_secs(600);

/// A resolved plain-HTTP Messages client. The API key is never logged.
pub struct Client {
    authority: String,
    addresses: Vec<SocketAddr>,
    key: String,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl Client {
    /// Parses `http://authority` with one optional trailing slash.
    ///
    /// # Errors
    /// Rejects every other URL shape, header-breaking keys, and authorities
    /// which cannot be resolved.
    pub fn new(base: &str, key: String) -> Result<Client> {
        Client::with_timeouts(base, key, CONNECT_TIMEOUT, IO_TIMEOUT)
    }

    fn with_timeouts(
        base: &str,
        key: String,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Client> {
        let authority = authority(base)?;
        if key.contains(['\r', '\n']) {
            return Err(Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "read ANTHROPIC_API_KEY",
                "the key contains a line break",
                "use a single-line gateway key",
            )));
        }
        let addresses = authority
            .to_socket_addrs()
            .map_err(|err| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Config,
                        "resolve ANTHROPIC_BASE_URL",
                        err.to_string(),
                        "use a resolvable authority such as 127.0.0.1:3425",
                    )
                    .with_value(&authority),
                )
            })?
            .collect::<Vec<_>>();
        if addresses.is_empty() {
            return Err(Error::Diagnostic(
                Diagnostic::new(
                    Code::Config,
                    "resolve ANTHROPIC_BASE_URL",
                    "no address was returned",
                    "use a resolvable authority such as 127.0.0.1:3425",
                )
                .with_value(authority),
            ));
        }
        Ok(Client {
            authority,
            addresses,
            key,
            connect_timeout,
            io_timeout,
        })
    }

    /// Sends one complete request and reads its complete response to EOF.
    ///
    /// # Errors
    /// I/O, non-200, malformed HTTP, chunked transfer, and JSON failures are
    /// returned without a retry. Non-200 errors carry only the response body.
    pub fn post(&self, request: &Value) -> Result<Value> {
        let body = serde_json::to_vec(request).map_err(|err| {
            protocol(
                "serialize a Messages request",
                err.to_string(),
                "use JSON-compatible values in the request",
            )
        })?;
        let mut stream = self.connect()?;
        stream
            .set_read_timeout(Some(self.io_timeout))
            .and_then(|()| stream.set_write_timeout(Some(self.io_timeout)))
            .map_err(|err| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "arm the Messages socket timeouts",
                        err.to_string(),
                        "retry the request against the same gateway; no retry was attempted",
                    )
                    .with_path(format!("http://{}/v1/messages", self.authority)),
                )
            })?;
        write!(
            stream,
            "POST /v1/messages HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nx-api-key: {}\r\nanthropic-version: 2023-06-01\r\n\r\n",
            body.len(),
            self.key,
        ).and_then(|()| stream.write_all(&body)).and_then(|()| stream.flush()).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(Code::Transport, "write the Messages request", err.to_string(), "check the gateway connection; no retry was attempted")
                    .with_path(format!("http://{}/v1/messages", self.authority)),
            )
        })?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "read the Messages response to EOF",
                    err.to_string(),
                    "check that the gateway closes non-streaming responses without chunking; no retry was attempted",
                )
                .with_path(format!("http://{}/v1/messages", self.authority)),
            )
        })?;
        parse(&response)
    }

    /// Connects to the first gateway address that answers within the budget.
    fn connect(&self) -> Result<TcpStream> {
        let mut last = None;
        for address in &self.addresses {
            match TcpStream::connect_timeout(address, self.connect_timeout) {
                Ok(stream) => return Ok(stream),
                Err(err) => last = Some(err),
            }
        }
        Err(Error::Diagnostic(
            Diagnostic::new(
                Code::Transport,
                "connect to the Messages gateway",
                last.map_or_else(|| "no address was tried".to_owned(), |err| err.to_string()),
                "start magpie or set ANTHROPIC_BASE_URL to a reachable gateway; no retry was attempted",
            )
            .with_value(&self.authority),
        ))
    }
}

fn authority(base: &str) -> Result<String> {
    let Some(authority) = base.strip_prefix("http://") else {
        return Err(Error::Diagnostic(
            Diagnostic::new(
                Code::Config,
                "parse ANTHROPIC_BASE_URL",
                "only http:// is supported",
                "use http://authority, for example http://127.0.0.1:3425",
            )
            .with_value(base),
        ));
    };
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    if authority.is_empty() || authority.contains(['/', '?', '#', '@', '\r', '\n', '\t', ' ']) {
        return Err(Error::Diagnostic(
            Diagnostic::new(
                Code::Config,
                "parse ANTHROPIC_BASE_URL",
                "expected http://authority with no path, query, fragment or userinfo",
                "use http://authority, for example http://127.0.0.1:3425",
            )
            .with_value(base),
        ));
    }
    Ok(authority.to_owned())
}

fn invalid_status(status_line: &str) -> Error {
    protocol(
        "parse the HTTP status line",
        format!("invalid status line: {status_line}"),
        "return an HTTP status line with a three-digit status code",
    )
    .into()
}

fn parse(response: &[u8]) -> Result<Value> {
    let Some(boundary) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err(protocol(
            "parse the HTTP header/body boundary",
            format!(
                "response has no CRLF header/body boundary; first bytes: {}",
                String::from_utf8_lossy(response.get(..200).unwrap_or(response))
            ),
            "return a complete HTTP response ending headers with CRLF CRLF",
        )
        .into());
    };
    let head = response.get(..boundary).unwrap_or_default();
    let status = head
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(head.len());
    let status_line = String::from_utf8_lossy(head.get(..status).unwrap_or_default());
    let code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| invalid_status(&status_line))?;
    let code = code
        .parse::<u16>()
        .map_err(|_| invalid_status(&status_line))?;
    let headers = String::from_utf8_lossy(head.get(status.saturating_add(2)..).unwrap_or_default());
    if headers
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("transfer-encoding:"))
    {
        return Err(protocol(
            "parse HTTP transfer headers",
            "Transfer-Encoding is not supported",
            "return a non-chunked JSON body with Connection: close",
        )
        .into());
    }
    let body = response
        .get(boundary.saturating_add(4)..)
        .unwrap_or_default();
    if code != 200 {
        return Err(Error::Body(String::from_utf8_lossy(body).into_owned()));
    }
    serde_json::from_slice(body).map_err(|err| {
        Error::from(protocol(
            "parse the Messages JSON body",
            format!(
                "{err}; first bytes: {}",
                String::from_utf8_lossy(body.get(..200).unwrap_or(body))
            ),
            "return one complete JSON object as the HTTP body",
        ))
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::thread;

    use super::*;

    fn server(response: &'static str) -> (String, thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.strip_prefix("Content-Length: ") {
                    length = value.trim().parse::<usize>().unwrap();
                }
                request.push_str(&line);
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            request.push_str(&String::from_utf8_lossy(&body));
            let mut stream = reader.into_inner();
            stream.write_all(response.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn a_post_writes_one_non_streaming_request_and_reads_json_to_eof() {
        let (base, handle) =
            server("HTTP/1.0 200 OK\r\nContent-Length: 25\r\n\r\n{\"stop_reason\":\"end_turn\"}");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let answer = client
            .post(&serde_json::json!({"model": "测试", "messages": []}))
            .unwrap();
        assert_eq!(answer["stop_reason"], "end_turn");
        let request = handle.join().unwrap();
        assert!(request.starts_with("POST /v1/messages HTTP/1.0\r\n"));
        assert!(request.contains("Content-Length: 32\r\n"));
        assert!(request.contains("x-api-key: magpie\r\n"));
        assert!(request.contains("anthropic-version: 2023-06-01\r\n"));
        assert!(!request.contains("Accept-Encoding"));
        let body = request.get(request.find('{').unwrap()..).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!({"model": "测试", "messages": []})
        );
    }

    #[test]
    fn a_non_200_returns_the_body_without_wrapping_it() {
        let (base, handle) = server("HTTP/1.0 429 Slow down\r\n\r\nrate limited");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        assert_eq!(
            client.post(&serde_json::json!({})),
            Err(Error::Body("rate limited".to_owned()))
        );
        handle.join().unwrap();
    }

    #[test]
    fn malformed_boundaries_and_chunking_are_named() {
        let (base, handle) = server("not an HTTP response");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        let error = client.post(&serde_json::json!({})).unwrap_err().to_string();
        assert!(error.contains("boundary") && error.contains("not an HTTP response"));
        handle.join().unwrap();
        let (base, handle) =
            server("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");
        let client = Client::new(&base, "magpie".to_owned()).unwrap();
        assert!(
            client
                .post(&serde_json::json!({}))
                .unwrap_err()
                .to_string()
                .contains("Transfer-Encoding")
        );
        handle.join().unwrap();
    }

    #[test]
    fn url_and_key_validation_is_fail_closed() {
        for base in [
            "https://localhost:3425",
            "http://localhost:3425/v1",
            "http://user@localhost",
            "http://localhost?q=1",
            "http://",
        ] {
            assert!(Client::new(base, "magpie".to_owned()).is_err(), "{base}");
        }
        assert!(Client::new("http://127.0.0.1:3425/", "magpie".to_owned()).is_ok());
        assert!(Client::new("http://127.0.0.1:3425", "bad\nkey".to_owned()).is_err());
    }

    #[test]
    fn a_gateway_that_accepts_and_stalls_returns_instead_of_hanging() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stalled = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut byte = [0_u8; 1];
            // Never answer; hold the connection until the client gives up.
            while let Ok(count) = stream.read(&mut byte) {
                if count == 0 {
                    break;
                }
            }
        });
        let client = Client::with_timeouts(
            &format!("http://{address}"),
            "magpie".to_owned(),
            Duration::from_secs(2),
            Duration::from_millis(200),
        )
        .unwrap();
        let error = client.post(&serde_json::json!({})).unwrap_err().to_string();
        assert!(error.contains("E_MEI_TRANSPORT"), "{error}");
        assert!(error.contains("read the Messages response"), "{error}");
        stalled.join().unwrap();
    }
}
