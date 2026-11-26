// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{ErrorKind, IoSlice, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::{Duration, Instant};

mod response;
use response::Response;

use serde_json::{Value, json};

use crate::catalog::{self, Selection};
use crate::error::{Code, Diagnostic, Error, Result, protocol};
use crate::observation;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

const IO_TIMEOUT: Duration = Duration::from_secs(600);

const PULSE_INTERVAL: Duration = Duration::from_secs(5);

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const WSA_IO_PENDING: i32 = 997;
const PENDING_READ_BACKOFF: Duration = Duration::from_millis(5);

struct Timeouts {
    connect: Duration,
    io: Duration,
    pulse: Duration,
}

struct ResponseTrace<'a> {
    pulse: &'a mut dyn Write,
    path: &'a str,
}

pub struct Client {
    authority: String,
    addresses: Vec<SocketAddr>,
    key: String,
    session: Option<String>,
    timeouts: Timeouts,
    probe: Timeouts,
    idle: Mutex<Option<TcpStream>>,
}

impl Client {
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
            idle: Mutex::new(None),
            probe: Timeouts {
                connect: timeouts.connect.min(PROBE_TIMEOUT),
                io: timeouts.io.min(PROBE_TIMEOUT),
                pulse: timeouts.pulse.min(PROBE_TIMEOUT),
            },
            timeouts,
        })
    }

    pub fn discover(&mut self, now: u64) -> Result<Selection> {
        let answer = self.get("/v1/models", &self.timeouts)?;
        let selection = catalog::select(&answer)?;
        self.session = Some(format!("mei-{}-{now}", std::process::id()));
        Ok(selection)
    }

    pub fn describe(&self, requested: Option<&str>) -> Result<Selection> {
        catalog::select_for(&self.get("/v1/models", &self.probe)?, requested)
    }

    pub fn post(&self, request: &Value, pulse: &mut dyn Write) -> Result<Value> {
        let body = serde_json::to_vec(request).map_err(|err| {
            protocol(
                "serialize a Messages request",
                err.to_string(),
                "use JSON-compatible values in the request",
            )
        })?;
        self.post_parts([&body, &[]], pulse)
    }

    pub(crate) fn post_parts(&self, parts: [&[u8]; 2], pulse: &mut dyn Write) -> Result<Value> {
        let path = format!("http://{}/v1/messages", self.authority);
        let session = match &self.session {
            Some(session) => format!("X-Magpie-Session: {session}\r\n"),
            None => String::new(),
        };
        let mut stream = self.open(&self.timeouts, &path)?;
        let head = format!(
            "POST /v1/messages HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\nx-api-key: {}\r\nanthropic-version: 2023-06-01\r\n{}\r\n",
            parts.iter().map(|part| part.len()).sum::<usize>(),
            self.key,
            session,
        );
        let [messages, settings] = parts;
        write_request(&mut stream, [head.as_bytes(), messages, settings]).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "write the Messages request",
                    err.to_string(),
                    "check the gateway connection; no retry was attempted",
                )
                .with_path(path.clone()),
            )
        })?;
        let answer = self.receive(stream, &self.timeouts, ResponseTrace { pulse, path: &path })?;
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
        let mut stream = self.open(budget, &path)?;
        let head = format!(
            "GET {target} HTTP/1.0\r\nConnection: keep-alive\r\nx-api-key: {}\r\n\r\n",
            self.key,
        );
        stream.write_all(head.as_bytes()).map_err(|err| {
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
        self.receive(
            stream,
            budget,
            ResponseTrace {
                pulse: &mut std::io::sink(),
                path: &path,
            },
        )
    }

    fn receive(
        &self,
        mut stream: TcpStream,
        budget: &Timeouts,
        trace: ResponseTrace<'_>,
    ) -> Result<Value> {
        let (value, reusable) = read_response(&mut stream, budget, trace, Read::read)?;
        if reusable {
            *self
                .idle
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(stream);
        }
        Ok(value)
    }

    fn open(&self, budget: &Timeouts, path: &str) -> Result<TcpStream> {
        let idle = self
            .idle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let stream = match idle.filter(idle_is_open) {
            Some(stream) => stream,
            None => self.connect(budget)?,
        };
        stream.set_write_timeout(Some(budget.io)).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "arm the Magpie socket timeouts",
                    err.to_string(),
                    "retry against the same gateway; no retry was attempted",
                )
                .with_path(path),
            )
        })?;
        Ok(stream)
    }

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

#[expect(
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    reason = "the IO budget is measured from the transport's own monotonic clock, and a Duration added to an Instant cannot wrap inside any configured budget"
)]
fn read_response(
    stream: &mut TcpStream,
    timeouts: &Timeouts,
    ResponseTrace { pulse, path }: ResponseTrace<'_>,
    mut read: impl FnMut(&mut TcpStream, &mut [u8]) -> std::io::Result<usize>,
) -> Result<(Value, bool)> {
    let mut last_pulse = Instant::now();
    let deadline = last_pulse + timeouts.io;
    let mut response = Response::default();
    let mut chunk = [0_u8; 8192];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(Error::Diagnostic(
                Diagnostic::new(
                    Code::Transport,
                    "read the Messages response",
                    "the IO budget elapsed before the response was complete",
                    "return the complete non-chunked response within the IO budget, with a length for keep-alive or EOF when closing; no retry was attempted",
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
        match read(stream, &mut chunk) {
            Ok(0) => return response.finish(),
            Ok(count) => {
                let bytes = chunk.get(..count).ok_or_else(|| {
                    protocol(
                        "read the Messages response",
                        "read returned a length past the buffer",
                        "return a complete HTTP response",
                    )
                })?;
                if response.extend(bytes)? {
                    return response.finish();
                }
            }
            Err(err) if cfg!(windows) && err.raw_os_error() == Some(WSA_IO_PENDING) => {
                if last_pulse.elapsed() >= timeouts.pulse {
                    write_pulse(pulse, path)?;
                    last_pulse = Instant::now();
                }
                std::thread::sleep(
                    PENDING_READ_BACKOFF.min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                write_pulse(pulse, path)?;
                last_pulse = Instant::now();
            }
            Err(err) => {
                return Err(Error::Diagnostic(
                    Diagnostic::new(
                        Code::Transport,
                        "read the Messages response",
                        err.to_string(),
                        "return the complete non-chunked response within the IO budget, with a length for keep-alive or EOF when closing; no retry was attempted",
                    )
                    .with_path(path),
                ));
            }
        }
    }
}

fn idle_is_open(stream: &TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return false;
    }
    let idle = stream
        .peek(&mut [0])
        .is_err_and(|err| err.kind() == ErrorKind::WouldBlock);
    stream.set_nonblocking(false).is_ok() && idle
}

fn write_request(stream: &mut impl Write, buffers: [&[u8]; 3]) -> std::io::Result<()> {
    #[cfg(windows)]
    if buffers
        .iter()
        .any(|buffer| u32::try_from(buffer.len()).is_err())
    {
        return buffers
            .iter()
            .try_for_each(|buffer| stream.write_all(buffer));
    }
    let mut slices = buffers.map(IoSlice::new);
    let mut parts = slices.as_mut_slice();
    while !parts.is_empty() {
        match stream.write_vectored(parts) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(count) => IoSlice::advance_slices(&mut parts, count),
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(())
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

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
