// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The socket the frames cross: one frame out, replies in, events read
//! past.

use browser::{BrowserPort, Frame, Reply};
use kernel::{AxCode, AxError};

/// The production adapter of `browser::port`: one frame out, replies in,
/// events dropped.
///
/// Events carry no id and nothing above this asks for them, so they are
/// read past rather than queued. A queue nobody drains is a leak, and
/// the one event stream this city acts on is its own ledger.
pub(crate) struct BidiSocket {
    runtime: tokio::runtime::Runtime,
    socket: WebSocket,
    /// How long one command may take before the caller is told the
    /// browser is unavailable. A browser that is thinking and a browser
    /// that is gone read the same on a socket.
    patience: std::time::Duration,
}

type WebSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

impl BidiSocket {
    /// Connects to an engine that is already up.
    ///
    /// # Errors
    /// Reports a socket that will not open, which is what a browser
    /// still starting looks like; the caller retries or gives up.
    pub(crate) fn connect(url: &str, patience: std::time::Duration) -> Result<BidiSocket, AxError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| unreachable_browser(url, &err.to_string()))?;
        let (socket, _) = runtime
            .block_on(tokio_tungstenite::connect_async(url))
            .map_err(|err| unreachable_browser(url, &err.to_string()))?;
        Ok(BidiSocket {
            runtime,
            socket,
            patience,
        })
    }
}

fn unreachable_browser(url: &str, why: &str) -> AxError {
    AxError::failure(
        AxCode::BrowserUnavailable,
        "reach a browser",
        format!("{url}: {why}"),
    )
    .with_recovery("start the engine first; a browser still starting refuses connections")
}

impl BrowserPort for BidiSocket {
    fn send(&mut self, frame: &Frame) -> Result<Reply, AxError> {
        use futures_util::{SinkExt, StreamExt};

        let wanted = frame.id();
        let patience = self.patience;
        let socket = &mut self.socket;
        self.runtime.block_on(async move {
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    frame.to_wire().into(),
                ))
                .await
                .map_err(|err| unreachable_browser("the open session", &err.to_string()))?;
            let deadline = tokio::time::Instant::now()
                .checked_add(patience)
                .ok_or_else(|| {
                    unreachable_browser("the open session", "the deadline overflowed")
                })?;
            loop {
                let next = tokio::time::timeout_at(deadline, socket.next())
                    .await
                    .map_err(|_| unreachable_browser("the open session", "no reply in time"))?;
                let message = next
                    .ok_or_else(|| unreachable_browser("the open session", "the socket closed"))?
                    .map_err(|err| unreachable_browser("the open session", &err.to_string()))?;
                let text = match message {
                    tokio_tungstenite::tungstenite::Message::Text(text) => text,
                    // Anything else is the transport talking to itself.
                    _ => continue,
                };
                // An event carries no id, so it cannot be the answer to
                // this frame; reading past it is what keeps one command
                // one command.
                if let Ok(reply) = Reply::parse(&text)
                    && reply.id() == wanted
                {
                    return Ok(reply);
                }
            }
        })
    }
}
