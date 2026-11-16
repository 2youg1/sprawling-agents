// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::{Opened, Permanence, PublicUrl, Route, Running};

const POLL_MS: u64 = 200;
const POLL: Duration = Duration::from_millis(POLL_MS);

const NAME_MAX_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tunnel {
    pub program: PathBuf,
    pub name: TunnelName,

    pub url: PublicUrl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelName(String);

pub struct NamedTunnel {
    tunnel: Tunnel,
    patience: TimeoutMs,
    running: Option<Running>,
}

impl TunnelName {
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
    #[must_use]
    pub fn new(tunnel: Tunnel, patience: TimeoutMs) -> Self {
        Self {
            tunnel,
            patience,
            running: None,
        }
    }

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
            child::command(&self.tunnel.program)
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

fn ask_ready(metrics: SocketAddr) -> std::io::Result<bool> {
    let mut stream = TcpStream::connect_timeout(&metrics, POLL)?;
    stream.set_read_timeout(Some(POLL))?;
    stream.set_write_timeout(Some(POLL))?;
    stream.write_all(b"GET /ready HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")?;
    let mut status = [0u8; 12];
    stream.read_exact(&mut status)?;
    Ok(status.starts_with(b"HTTP/1.") && status.ends_with(b" 200"))
}
