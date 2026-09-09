// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An engine nobody has started yet, and the port it will answer on.
//!
//! A building that admits the browser and never uses it should not have
//! a browser running, so the process starts on the first frame and stops
//! when the tool holding it is dropped.

use std::path::{Path, PathBuf};

use browser::{BrowserPort, Frame, Reply};
use kernel::{AxCode, AxError};

use super::engine::{Engaged, Engine};
use super::socket::BidiSocket;
use crate::doctor::host;

/// Derived from the city's own directory rather than drawn at random:
/// two cities on one machine need two ports, and a value derived from
/// what already distinguishes them needs no entropy and no clock. The
/// range is the private one, so nothing standard is displaced.
pub(crate) fn port_for(city_root: &Path) -> u16 {
    let digest = kernel::B3Hash::digest(city_root.display().to_string().as_bytes());
    let bytes = digest.as_bytes();
    let high = u16::from(bytes.first().copied().unwrap_or(0));
    let low = u16::from(bytes.get(1).copied().unwrap_or(0));
    let span = high.saturating_mul(256).saturating_add(low) % 20_000;
    span.saturating_add(40_000)
}

/// An engine nobody has started yet.
///
/// A building that admits the browser and never uses it should not have
/// a browser running, so the process starts on the first frame and stops
/// when this value is dropped.
pub(crate) struct LazyEngine {
    profile: PathBuf,
    headless: bool,
    port: u16,
    running: Option<Engaged>,
    socket: Option<BidiSocket>,
}

/// How long a browser has to answer one command.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(30);

/// How many times a starting browser is knocked on before the first
/// frame is refused, and how long between two knocks. Counted rather
/// than timed because the one place this city reads a clock is
/// `bin::assembly`, and a retry loop does not need to know the time of
/// day to know it has knocked eighty times.
const STARTUP_KNOCKS: u32 = 80;
const BETWEEN_KNOCKS: std::time::Duration = std::time::Duration::from_millis(250);

impl LazyEngine {
    pub(crate) fn new(profile: PathBuf, headless: bool, port: u16) -> LazyEngine {
        LazyEngine {
            profile,
            headless,
            port,
            running: None,
            socket: None,
        }
    }

    /// Starts an engine and connects to it.
    ///
    /// Which engine is here is `bin::doctor`'s answer, so a person who
    /// read the doctor's report and a run that meets this refusal are
    /// told the same thing about the same machine.
    ///
    /// # Errors
    /// Reports a machine with no engine, an engine that is here and
    /// will not start, and one that came up and would not answer.
    fn start(&mut self) -> Result<(), AxError> {
        let engine = Engine::choose(&host::firefox(), &host::chromedriver())?;
        let plan = engine.plan(&self.profile, self.port, self.headless);
        let started = Engine::launch(&plan)?;
        let url = plan.socket_url();
        let mut last = None;
        for knock in 0..STARTUP_KNOCKS {
            match BidiSocket::connect(&url, PATIENCE) {
                Ok(socket) => {
                    self.running = Some(started);
                    self.socket = Some(socket);
                    return Ok(());
                }
                Err(refusal) => {
                    last = Some(refusal);
                    if knock.saturating_add(1) < STARTUP_KNOCKS {
                        // A browser still opening its own window refuses
                        // connections, and that is not the same fact as
                        // having no browser.
                        std::thread::sleep(BETWEEN_KNOCKS);
                    }
                }
            }
        }
        Err(last.unwrap_or_else(|| {
            AxError::failure(
                AxCode::BrowserUnavailable,
                "reach a browser",
                "the engine started and never answered",
            )
            .with_recovery("run the browser by hand once; a first start may be asking a question")
        }))
    }
}

impl BrowserPort for LazyEngine {
    fn send(&mut self, frame: &Frame) -> Result<Reply, AxError> {
        if self.socket.is_none() {
            self.start()?;
        }
        let socket = self.socket.as_mut().ok_or_else(|| {
            AxError::failure(
                AxCode::BrowserUnavailable,
                "reach a browser",
                "the session did not open",
            )
            .with_recovery("start the browser again")
        })?;
        socket.send(frame)
    }
}
