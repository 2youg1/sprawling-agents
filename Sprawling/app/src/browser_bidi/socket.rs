// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use browser::{BrowserPort, Frame, Reply};
use kernel::{AxCode, AxError};

pub(crate) struct BidiSocket {
    runtime: tokio::runtime::Runtime,
    socket: WebSocket,

    patience: std::time::Duration,
}

type WebSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

impl BidiSocket {
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
                #[expect(
                    clippy::wildcard_enum_match_arm,
                    reason = "tungstenite's frame kinds are the transport's; only text carries a reply"
                )]
                let text = match message {
                    tokio_tungstenite::tungstenite::Message::Text(text) => text,

                    _ => continue,
                };

                if let Ok(reply) = Reply::parse(&text)
                    && reply.id() == wanted
                {
                    return Ok(reply);
                }
            }
        })
    }
}
