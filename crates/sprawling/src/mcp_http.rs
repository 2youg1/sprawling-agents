// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An MCP server reached over HTTP: one request carries one message and
//! its answer comes back in the response body.
//!
//! The second transport, and the reason the seam earns its name. What
//! differs from the child process is only where the bytes go: framing,
//! deadlines and refusals keep the same shapes, so nothing above this
//! module knows which kind of server it is talking to.
//!
//! A hosted server may answer as an event stream rather than as one JSON
//! body. This reads the first data line of such a stream and refuses a
//! body it cannot read as one message, rather than guessing at a
//! concatenation - a wrong guess here would be a tool result assembled
//! out of two answers.
//!
//! **The session lives here**, because the specification puts it in the
//! transport rather than in the protocol: a server *may* answer
//! `initialize` with an `Mcp-Session-Id`, and a client that receives one
//! MUST send it back on every later request. A 404 means the server
//! ended the session, and the answer to that is a new handshake rather
//! than a refusal.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use crate::mcp_redeeming::{Redeemed, redeem};

/// What the far end told us about itself, and what has to travel back.
#[derive(Debug, Default)]
struct Session {
    /// Present only when the server chose to have one.
    id: Option<String>,
    /// The negotiated revision, learned from the answer that carried it.
    protocol_version: Option<String>,
}

/// A connection to one HTTP server.
///
/// Cloning shares the session, because one server is one session however
/// many of its tools a run holds: two clones sending two session ids
/// would be two conversations with one server, and the server is
/// entitled to know only about the one it opened.
#[derive(Clone)]
pub(crate) struct HttpServer {
    url: String,
    headers: Vec<Redeemed>,
    client: reqwest::blocking::Client,
    session: Arc<Mutex<Session>>,
}

impl std::fmt::Debug for HttpServer {
    /// Names the headers but never their values: a configured header may
    /// be a redeemed credential, and a `Debug` that printed it would be
    /// the leak `Sealed` exists to make unspellable.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpServer")
            .field("url", &self.url)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl HttpServer {
    /// # Errors
    /// Refuses a header with no name, a `secret:realm/name` reference
    /// the vault does not hold, and a client this machine cannot build.
    pub(crate) fn open(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
    ) -> Result<HttpServer, AxError> {
        // Redeemed before the first request rather than at it, because a
        // key the vault does not hold is a configuration error and not a
        // server that happens to be down.
        let headers = redeem(headers, resolve, "reach an mcp server")?;
        let mut builder = reqwest::blocking::Client::builder();
        if gateway::is_local(url) {
            // A server a person started on this machine is reached by
            // address, and a proxy in front of it answers for something
            // else entirely.
            builder = builder.no_proxy();
        }
        let client = builder
            // Named, because a hosted server sitting behind a content
            // delivery network refuses a client that will not say what
            // it is: reaching Exa's endpoint without this answers 403
            // `browser_signature_banned` before any MCP message is read.
            .user_agent(concat!("sprawling/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
            })?;
        Ok(HttpServer {
            url: url.to_owned(),
            headers,
            client,
            session: Arc::new(Mutex::new(Session::default())),
        })
    }

    /// One POST, with whatever the session says has to travel with it.
    fn post(&self, line: &str, patience: TimeoutMs) -> Result<Exchange, AxError> {
        let mut request = self
            .client
            .post(&self.url)
            .timeout(Duration::from_millis(patience.0))
            .header("content-type", "application/json")
            // Both shapes are declared as acceptable because the
            // specification requires a client to support either, and a
            // server that answers a shape it was never offered is a
            // server this city would refuse for a reason of its making.
            .header("accept", "application/json, text/event-stream")
            .body(line.to_owned());
        for header in &self.headers {
            // The disclosure point: plaintext exists for the length of
            // this call and never in the configuration, in a log, or in
            // this type's `Debug`.
            request = request.header(header.name(), header.plaintext());
        }
        if let Ok(session) = self.session.lock() {
            if let Some(id) = &session.id {
                request = request.header("mcp-session-id", id);
            }
            if let Some(version) = &session.protocol_version {
                request = request.header("mcp-protocol-version", version);
            }
        }
        let response = request.send().map_err(|err| self.unreachable(&err))?;
        let status = response.status().as_u16();
        let handed = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.text().map_err(|err| self.unreachable(&err))?;
        Ok(Exchange {
            status,
            handed,
            body,
        })
    }

    /// Records what an exchange taught the session.
    ///
    /// The protocol version is taken from the answer that carries one,
    /// which by the lifecycle is the answer to `initialize`: it is the
    /// first message of a connection, so no earlier answer can hold the
    /// field.
    fn learn(&self, exchange: &Exchange) {
        let Ok(mut session) = self.session.lock() else {
            return;
        };
        if let Some(id) = &exchange.handed {
            session.id = Some(id.clone());
        }
        if session.protocol_version.is_none()
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&exchange.body)
            && let Some(agreed) = value
                .pointer("/result/protocolVersion")
                .and_then(serde_json::Value::as_str)
        {
            session.protocol_version = Some(agreed.to_owned());
        }
    }

    /// Forgets a session the server says is gone.
    ///
    /// The specification's answer to a 404 on a request carrying a
    /// session id is to open a new session, so the id is dropped here
    /// and the next handshake starts without one.
    fn forget(&self) {
        if let Ok(mut session) = self.session.lock() {
            session.id = None;
        }
    }

    /// Turns a status the server chose into the refusal a person reads.
    fn refused(&self, status: u16) -> AxError {
        // The one status a person answers differently: the server is up,
        // it understood the request, and it wants an account this city
        // does not yet hold. Carried as its own code so the health view
        // can say "authenticating" where it would otherwise say "failed"
        // (sprawling-SPEC.md 8-61).
        if status == 401 || status == 403 {
            return AxError::failure(
                AxCode::CredentialMissing,
                "call an mcp server",
                format!("{}: the server answered {status}", self.url),
            )
            .with_recovery(
                "this server wants an account; store its key in the vault and name it in \
                 `headers`, or sign in to it",
            );
        }
        if status == 404 {
            self.forget();
            return AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                format!("{}: the server ended this session", self.url),
            )
            .with_recovery("the session was dropped; dispatch again to open a new one")
            .retriable();
        }
        // The body is not quoted: a server's error page is other
        // people's text and this refusal is read by a person.
        AxError::failure(
            AxCode::ToolUnavailable,
            "call an mcp server",
            format!("{}: the server answered {status}", self.url),
        )
        .with_recovery("check the url and the header this building configured")
    }
}

/// One request and what came back, before anything is decided about it.
struct Exchange {
    status: u16,
    /// The `Mcp-Session-Id` the server handed out, if it opened one.
    handed: Option<String>,
    body: String,
}

impl protocol::Outbound for HttpServer {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        let exchange = self.post(line, patience)?;
        if !(200..300).contains(&exchange.status) {
            return Err(self.refused(exchange.status));
        }
        self.learn(&exchange);
        one_message(&exchange.body).ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read an mcp answer",
                format!("{}: the answer is not one message", self.url),
            )
            .with_recovery("this version reads one JSON body, or the first data line of a stream")
        })
    }

    /// A notification is answered with 202 and no body, so nothing is
    /// read back. A session id may still arrive here, and is kept.
    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        let exchange = self.post(line, patience)?;
        if !(200..300).contains(&exchange.status) {
            return Err(self.refused(exchange.status));
        }
        self.learn(&exchange);
        Ok(())
    }
}

impl HttpServer {
    fn unreachable(&self, err: &reqwest::Error) -> AxError {
        let mut detail = err.to_string();
        let mut cause = std::error::Error::source(err);
        while let Some(link) = cause {
            detail.push_str(": ");
            detail.push_str(&link.to_string());
            cause = link.source();
        }
        AxError::failure(
            AxCode::ToolUnavailable,
            "call an mcp server",
            format!("{}: {detail}", self.url),
        )
        .with_recovery("the server is not answering; this run continues without it")
    }
}

/// The one message a body carries, whether it arrived as a JSON body or
/// as the first data line of an event stream.
fn one_message(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed.to_owned());
    }
    trimmed
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .map(|line| line.trim().to_owned())
        .filter(|line| line.starts_with('{'))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
