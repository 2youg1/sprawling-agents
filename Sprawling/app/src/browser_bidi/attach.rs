// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use browser::{BrowserPort, Frame, Reply};
use kernel::{AxCode, AxError};

use super::socket::BidiSocket;

const PATIENCE: Duration = Duration::from_secs(30);

pub(crate) struct AttachedBrowser {
    url: Option<String>,
    socket: Option<BidiSocket>,
}

impl AttachedBrowser {
    pub(crate) fn at(url: &str) -> AttachedBrowser {
        AttachedBrowser {
            url: Some(url.to_owned()),
            socket: None,
        }
    }

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
                "reach the User's browser",
                "no address is declared",
            )
            .with_recovery(
                "the User starts their browser with remote debugging and puts the \
                 ws:// address it prints on a `usersbrowser` key in RULES.toml",
            ));
        };
        if self.socket.is_none() {
            self.socket = Some(BidiSocket::connect(url, PATIENCE)?);
        }
        let socket = self.socket.as_mut().ok_or_else(|| {
            AxError::failure(
                AxCode::BrowserUnavailable,
                "reach the User's browser",
                "the attachment did not open",
            )
            .with_recovery("check that the browser is still running with remote debugging on")
        })?;
        socket.send(frame)
    }
}
