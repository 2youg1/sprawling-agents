// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The smallest non-streaming Anthropic HTTP adapter (`Mei/Spec.lean`
//! §5, §8-§10). No proxy, TLS, compression, redirect, chunked or SSE.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use serde_json::Value;

use crate::error::{Diagnostic, Error, Result, protocol};

/// A resolved plain-HTTP Messages client. The API key is never logged.
pub struct Client {
    authority: String,
    addresses: Vec<SocketAddr>,
    key: String,
}

impl Client {
    /// Parses `http://authority` with one optional trailing slash.
    ///
    /// # Errors
    /// Rejects every other URL shape, header-breaking keys, and authorities
    /// which cannot be resolved.
    pub fn new(base: &str, key: String) -> Result<Client> {
        let authority = authority(base)?;
        if key.contains(['\r', '\n']) {
            return Err(Error::Diagnostic(Diagnostic::new(
                "E_MEI_CONFIG",
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
                        "E_MEI_CONFIG",
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
                    "E_MEI_CONFIG",
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
        let mut stream = TcpStream::connect(self.addresses.as_slice()).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    "E_MEI_TRANSPORT",
                    "connect to the Messages gateway",
                    err.to_string(),
                    "start magpie or set ANTHROPIC_BASE_URL to a reachable gateway",
                )
                .with_value(&self.authority),
            )
        })?;
        write!(
            stream,
            "POST /v1/messages HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nx-api-key: {}\r\nanthropic-version: 2023-06-01\r\n\r\n",
            body.len(),
            self.key,
        ).and_then(|()| stream.write_all(&body)).and_then(|()| stream.flush()).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new("E_MEI_TRANSPORT", "write the Messages request", err.to_string(), "check the gateway connection; no retry was attempted")
                    .with_path(format!("http://{}/v1/messages", self.authority)),
            )
        })?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    "E_MEI_TRANSPORT",
                    "read the Messages response to EOF",
                    err.to_string(),
                    "check that the gateway closes non-streaming responses without chunking",
                )
                .with_path(format!("http://{}/v1/messages", self.authority)),
            )
        })?;
        parse(&response)
    }
}

fn authority(base: &str) -> Result<String> {
    let Some(authority) = base.strip_prefix("http://") else {
        return Err(Error::Diagnostic(
            Diagnostic::new(
                "E_MEI_CONFIG",
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
                "E_MEI_CONFIG",
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
        ));
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
        ));
    }
    let body = response
        .get(boundary.saturating_add(4)..)
        .unwrap_or_default();
    if code != 200 {
        return Err(Error::Body(String::from_utf8_lossy(body).into_owned()));
    }
    serde_json::from_slice(body).map_err(|err| {
        protocol(
            "parse the Messages JSON body",
            format!(
                "{err}; first bytes: {}",
                String::from_utf8_lossy(body.get(..200).unwrap_or(body))
            ),
            "return one complete JSON object as the HTTP body",
        )
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
}
