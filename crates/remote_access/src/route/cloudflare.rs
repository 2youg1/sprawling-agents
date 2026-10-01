// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The default route: a Cloudflare named tunnel the person made once,
//! run for as long as the route is open (crates/remote_access/Spec.lean §8-8).
//!
//! The tunnel is managed locally: its credentials stay in `cloudflared`'s
//! own directory and never enter the city, and its origin is given on
//! each open with `--url`, so the remote listener needs no fixed port
//! (remote_access D16). Ready means `cloudflared`'s metrics server answers `/ready`
//! with 200, which it does once it holds a connection to the edge.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::{Opened, Permanence, PublicUrl, Route, Running};

/// How often readiness is asked, and how long one asking may take.
const POLL_MS: u64 = 200;
const POLL: Duration = Duration::from_millis(POLL_MS);

/// The longest tunnel name taken: it is written on a command line.
const NAME_MAX_CHARS: usize = 64;

/// A locally managed named tunnel, as the person made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tunnel {
    /// `cloudflared`, or its full path.
    pub program: PathBuf,
    pub name: TunnelName,
    /// `https://` and the host name the tunnel's DNS route points at.
    pub url: PublicUrl,
}

/// A tunnel's name as `cloudflared` takes it on its command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelName(String);

/// The Cloudflare named tunnel route.
pub struct NamedTunnel {
    tunnel: Tunnel,
    patience: TimeoutMs,
    running: Option<Running>,
}

impl TunnelName {
    /// # Errors
    /// `E_CONFIG_INVALID` for an empty name, one over 64 characters, one
    /// with anything but ASCII letters, digits, `.`, `_` and `-`, and one
    /// that starts with `-`, which `cloudflared` would read as an option.
    pub fn parse(text: &str) -> Result<Self, AxError> {
        let spelled = text
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || matches!(each, '.' | '_' | '-'));
        let sized = (1..=NAME_MAX_CHARS).contains(&text.chars().count());
        if spelled && sized && !text.starts_with('-') {
            Ok(Self(text.to_owned()))
        } else {
            Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read a Cloudflare tunnel name",
                text.to_owned(),
            )
            .with_recovery(
                "name the tunnel as `cloudflared tunnel create` made it: 1 to 64 letters, digits, \
                 `.`, `_` or `-`, not starting with `-`",
            ))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl NamedTunnel {
    /// A closed route to `tunnel`, which waits up to `patience` for
    /// `cloudflared` to reach the edge each time it opens.
    #[must_use]
    pub fn new(tunnel: Tunnel, patience: TimeoutMs) -> Self {
        Self {
            tunnel,
            patience,
            running: None,
        }
    }

    /// The line a person runs to see why the tunnel would not start.
    fn by_hand(&self, local: SocketAddr) -> String {
        format!(
            "run `cloudflared tunnel --url http://{local} run {name}` to see why; the tunnel is \
             made once with `cloudflared tunnel login`, `cloudflared tunnel create {name}` and \
             `cloudflared tunnel route dns {name} <host>`",
            name = self.tunnel.name.as_str()
        )
    }
}

impl Route for NamedTunnel {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        self.close()?;
        let metrics = free_loopback_address()?;
        let mut running = Running::start(
            Command::new(&self.tunnel.program)
                .args(["tunnel", "--no-autoupdate", "--metrics"])
                .arg(metrics.to_string())
                .arg("--url")
                .arg(format!("http://{local}"))
                .args(["run", self.tunnel.name.as_str()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
            "install cloudflared, or give the route its full path",
        )?;
        for _ in 0..self.patience.0.div_ceil(POLL_MS).max(1) {
            if let Some(status) = running.exited()? {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "open a Cloudflare named tunnel",
                    format!(
                        "{} stopped before it was ready ({status})",
                        running.program()
                    ),
                )
                .with_recovery(self.by_hand(local)));
            }
            if matches!(ask_ready(metrics), Ok(true)) {
                self.running = Some(running);
                return Ok(Opened {
                    url: self.tunnel.url.clone(),
                    permanence: Permanence::Fixed,
                });
            }
            std::thread::sleep(POLL);
        }
        running.stop()?;
        Err(AxError::failure(
            AxCode::Timeout,
            "open a Cloudflare named tunnel",
            format!("not ready within {} ms", self.patience.0),
        )
        .with_recovery(format!(
            "check that this computer reaches Cloudflare; the tunnel was stopped. {}",
            self.by_hand(local)
        )))
    }

    fn close(&mut self) -> Result<(), AxError> {
        self.running
            .take()
            .map_or(Ok(()), |mut running| running.stop())
    }
}

/// A loopback port nothing listens on now: bound, read, released.
fn free_loopback_address() -> Result<SocketAddr, AxError> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|listener| listener.local_addr())
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "open a Cloudflare named tunnel",
                format!("a loopback port for its metrics: {err}"),
            )
            .with_recovery("free a loopback port, or close programs holding many of them")
        })
}

/// Whether the metrics server says the tunnel holds a connection. A
/// failure to ask means not yet, not a refusal: the server listens only
/// once `cloudflared` has started, and the next poll asks again.
fn ask_ready(metrics: SocketAddr) -> std::io::Result<bool> {
    let mut stream = TcpStream::connect_timeout(&metrics, POLL)?;
    stream.set_read_timeout(Some(POLL))?;
    stream.set_write_timeout(Some(POLL))?;
    stream.write_all(b"GET /ready HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")?;
    let mut status = [0u8; 12];
    stream.read_exact(&mut status)?;
    Ok(status.starts_with(b"HTTP/1.") && status.ends_with(b" 200"))
}
