// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A browser somebody else is running.
//!
//! `LazyEngine` starts a process and owns it: its `Drop` kills the child,
//! and its `running` field is the proof that a browser exists because
//! this city made it. Neither is true here, and both would be wrong. So
//! this port holds a URL and a socket and nothing else: connecting is
//! the whole of what it does, dropping the socket ends one session, and
//! the person's browser keeps running because it was never this city's
//! to stop.
//!
//! The address is Firefox's BiDi address — `ws://127.0.0.1:<port>/session`
//! — which is the address the person's browser prints when it is started
//! with remote debugging. The Chromium arm of `Engine` runs a driver and
//! is not this: a driver owns a session of its own, and there is nothing
//! on the other end of it for a run to attach to.

use std::time::Duration;

use browser::{BrowserPort, Frame, Reply};
use kernel::{AxCode, AxError};

use super::socket::BidiSocket;

/// How long the person's browser has to answer one command. The same
/// patience `LazyEngine` gives a starting browser: a browser that is
/// thinking and a browser that is gone read the same on a socket.
const PATIENCE: Duration = Duration::from_secs(30);

/// The port onto a browser this city did not start.
pub(crate) struct AttachedBrowser {
    /// Where the person said their browser answers, as they wrote it.
    url: Option<String>,
    socket: Option<BidiSocket>,
}

impl AttachedBrowser {
    /// The port onto the browser at `url`.
    pub(crate) fn at(url: &str) -> AttachedBrowser {
        AttachedBrowser {
            url: Some(url.to_owned()),
            socket: None,
        }
    }

    /// The port of a building that enabled the tool and declared no
    /// address. Every send says so; the gate asks first, so a run reads
    /// the question rather than this.
    pub(crate) fn waiting() -> AttachedBrowser {
        AttachedBrowser {
            url: None,
            socket: None,
        }
    }
}

impl BrowserPort for AttachedBrowser {
    fn send(&mut self, frame: &Frame) -> Result<Reply, AxError> {
        let Some(url) = self.url.as_deref() else {
            return Err(AxError::failure(
                AxCode::BrowserUnavailable,
                "reach the person's browser",
                "no address is declared",
            )
            .with_recovery(
                "the person starts their browser with remote debugging and puts the \
                 ws:// address it prints on a `usersbrowser` key in RULES.toml",
            ));
        };
        if self.socket.is_none() {
            self.socket = Some(BidiSocket::connect(url, PATIENCE)?);
        }
        let socket = self.socket.as_mut().ok_or_else(|| {
            AxError::failure(
                AxCode::BrowserUnavailable,
                "reach the person's browser",
                "the attachment did not open",
            )
            .with_recovery("check that the browser is still running with remote debugging on")
        })?;
        socket.send(frame)
    }
}
