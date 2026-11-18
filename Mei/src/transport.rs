// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The smallest non-streaming Anthropic HTTP adapter (`Mei/Spec.lean`
//! §5, §8-§10). No proxy, TLS, compression, redirect, chunked or SSE.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::catalog::{self, Selection};
use crate::error::{Code, Diagnostic, Error, Result, protocol};
use crate::observation;

/// Per-address TCP connect budget. The gateway is normally loopback, so a
/// handshake with no answer in this long is a dead gateway, not a slow one.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read and write budget for one complete non-streaming response. A model
/// may legitimately think for minutes, so this stays generous; it exists so
/// a gateway that accepts a connection and then stalls forever returns a
/// diagnostic instead of hanging the run.
const IO_TIMEOUT: Duration = Duration::from_secs(600);

/// How long a blocked read may stay silent before a `mei.obs` pulse line.
const PULSE_INTERVAL: Duration = Duration::from_secs(5);

/// The completed route is an auxiliary observation taken after the answer
/// arrived, so it gets its own short budget: a gateway that accepts and stays
/// silent delays the answer by this long rather than by [`IO_TIMEOUT`].
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Connect, IO, and pulse budgets that travel together on one client.
struct Timeouts {
    connect: Duration,
    io: Duration,
    pulse: Duration,
}

/// A resolved plain-HTTP Messages client. The API key is never logged.
pub struct Client {
    authority: String,
    addresses: Vec<SocketAddr>,
    key: String,
    session: Option<String>,
    timeouts: Timeouts,
    probe: Timeouts,
}

impl Client {
    /// Parses `http://authority` with one optional trailing slash.
    ///
    /// # Errors
    /// Rejects every other URL shape, header-breaking keys, and authorities
    /// which cannot be resolved.
    pub fn new(base: &str, key: String) -> Result<Client> {
        Client::with_timeouts(
            base,
            key,
            Timeouts {
                connect: CONNECT_TIMEOUT,
                io: IO_TIMEOUT,
                pulse: PULSE_INTERVAL,
            },
        )
    }

    fn with_timeouts(base: &str, key: String, timeouts: Timeouts) -> Result<Client> {
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
            session: None,
            probe: Timeouts {
                connect: timeouts.connect.min(PROBE_TIMEOUT),
                io: timeouts.io.min(PROBE_TIMEOUT),
                pulse: timeouts.pulse.min(PROBE_TIMEOUT),
            },
            timeouts,
        })
    }

    /// Selects Magpie's preferred visible route and enables per-run route observation.
    ///
    /// # Errors
    /// The catalog is unreachable, malformed, or contains no usable chat model.
    pub fn discover(&mut self, now: u64) -> Result<Selection> {
        let answer = self.get("/v1/models", &self.timeouts)?;
        let selection = catalog::select(&answer)?;
        self.session = Some(format!("mei-{}-{now}", std::process::id()));
        Ok(selection)
    }

    /// Sends one complete request and reads its complete response to EOF.
    /// While the read is blocked past [`PULSE_INTERVAL`], writes one
    /// `mei.obs` pulse line to `pulse` without changing the request.
    ///
    /// # Errors
    /// I/O, non-200, malformed HTTP, chunked transfer, and JSON failures are
    /// returned without a retry. Non-200 errors carry only the response body.
    pub fn post(&self, request: &Value, pulse: &mut dyn Write) -> Result<Value> {
        let body = serde_json::to_vec(request).map_err(|err| {
            protocol(
                "serialize a Messages request",
                err.to_string(),
                "use JSON-compatible values in the request",
            )
        })?;
        let path = format!("http://{}/v1/messages", self.authority);
        let session = match &self.session {
            Some(session) => format!("X-Magpie-Session: {session}\r\n"),
            None => String::new(),
        };
        let mut stream = self.connect(&self.timeouts)?;
        stream
            .set_write_timeout(Some(self.timeouts.io))
            .map_err(|err| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "arm the Messages socket timeouts",
                        err.to_string(),
                        "retry the request against the same gateway; no retry was attempted",
                    )
                    .with_path(path.clone()),
                )
            })?;
        write!(
            stream,
            "POST /v1/messages HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nx-api-key: {}\r\nanthropic-version: 2023-06-01\r\n{}\r\n",
            body.len(),
            self.key,
            session,
        ).and_then(|()| stream.write_all(&body)).and_then(|()| stream.flush()).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(Code::Transport, "write the Messages request", err.to_string(), "check the gateway connection; no retry was attempted")
                    .with_path(path.clone()),
            )
        })?;
        let response = read_to_eof(&mut stream, &self.timeouts, pulse, &path)?;
        let answer = parse(&response)?;
        if self.session.is_some() {
            let route = match self.route_observation() {
                Ok(route) => route,
                Err(error) => json!({
                    "type": "mei.obs",
                    "kind": "route",
                    "available": false,
                    "reason": error.to_string(),
                }),
            };
            observation::write(pulse, &route, "trace a Magpie route observation")?;
        }
        Ok(answer)
    }

    fn route_observation(&self) -> Result<Value> {
        let Some(session) = &self.session else {
            return Err(Error::Diagnostic(Diagnostic::new(
                Code::Protocol,
                "observe a Magpie route without a session",
                "the client has no X-Magpie-Session id",
                "use discover before post, or leave HARNESS_MODEL explicit",
            )));
        };
        let target = format!("/v1/magpie/route?session={session}");
        let answer = self.get(&target, &self.probe)?;
        let route = catalog::route(&answer)?;
        let effort_source = if route.effort.is_some() {
            "actual"
        } else {
            "unstated"
        };
        Ok(json!({
            "type": "mei.obs",
            "kind": "route",
            "available": true,
            "model": route.model,
            "effort": route.effort,
            "effort_source": effort_source,
        }))
    }

    fn get(&self, target: &str, budget: &Timeouts) -> Result<Value> {
        let path = format!("http://{}{}", self.authority, target);
        let mut stream = self.connect(budget)?;
        stream
            .set_write_timeout(Some(budget.io))
            .map_err(|err| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "arm the Magpie socket timeouts",
                        err.to_string(),
                        "retry against the same gateway; no retry was attempted",
                    )
                    .with_path(path.clone()),
                )
            })?;
        write!(
            stream,
            "GET {target} HTTP/1.0\r\nConnection: close\r\nx-api-key: {}\r\n\r\n",
            self.key,
        )
        .and_then(|()| stream.flush())
        .map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "write the Magpie request",
                    err.to_string(),
                    "check the gateway connection; no retry was attempted",
                )
                .with_path(path.clone()),
            )
        })?;
        let mut sink = std::io::sink();
        let response = read_to_eof(&mut stream, budget, &mut sink, &path)?;
        parse(&response)
    }

    /// Connects to the first gateway address that answers within the budget.
    fn connect(&self, budget: &Timeouts) -> Result<TcpStream> {
        let mut last = None;
        for address in &self.addresses {
            match TcpStream::connect_timeout(address, budget.connect) {
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

fn read_to_eof(
    stream: &mut TcpStream,
    timeouts: &Timeouts,
    pulse: &mut dyn Write,
    path: &str,
) -> Result<Vec<u8>> {
    let mut remaining = timeouts.io;
    let mut response = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        if remaining.is_zero() {
            return Err(Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "read the Messages response to EOF",
                    "the IO budget elapsed before EOF",
                    "check that the gateway closes non-streaming responses without chunking; no retry was attempted",
                )
                .with_path(path),
            ));
        }
        let slice = remaining.min(timeouts.pulse);
        stream.set_read_timeout(Some(slice)).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "arm the Messages socket timeouts",
                    err.to_string(),
                    "retry the request against the same gateway; no retry was attempted",
                )
                .with_path(path),
            )
        })?;
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(response),
            Ok(count) => response.extend_from_slice(chunk.get(..count).ok_or_else(|| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "read the Messages response to EOF",
                        "read returned a length past the buffer",
                        "retry the request against the same gateway; no retry was attempted",
                    )
                    .with_path(path),
                )
            })?),
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                remaining = remaining.saturating_sub(slice);
                if remaining.is_zero() {
                    return Err(Error::Diagnostic(
                        Diagnostic::new(
                            Code::Transport,
                            "read the Messages response to EOF",
                            err.to_string(),
                            "check that the gateway closes non-streaming responses without chunking; no retry was attempted",
                        )
                        .with_path(path),
                    ));
                }
                write_pulse(pulse, path)?;
            }
            Err(err) => {
                return Err(Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "read the Messages response to EOF",
                        err.to_string(),
                        "check that the gateway closes non-streaming responses without chunking; no retry was attempted",
                    )
                    .with_path(path),
                ));
            }
        }
    }
}

fn write_pulse(pulse: &mut dyn Write, path: &str) -> Result<()> {
    serde_json::to_writer(&mut *pulse, &json!({"type": "mei.obs", "kind": "pulse"})).map_err(
        |err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "write a mei.obs pulse",
                    err.to_string(),
                    "keep stderr writable during a blocked Messages read",
                )
                .with_path(path),
            )
        },
    )?;
    pulse
        .write_all(b"\n")
        .and_then(|()| pulse.flush())
        .map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "write a mei.obs pulse",
                    err.to_string(),
                    "keep stderr writable during a blocked Messages read",
                )
                .with_path(path),
            )
        })
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
#[path = "transport_tests.rs"]
mod tests;
