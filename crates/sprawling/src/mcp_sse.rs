// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An MCP server that answers on a stream of server-sent events: the
//! stream is opened first and names where messages go, and every answer
//! comes back down the stream rather than in the body of the request
//! that asked.
//!
//! The third transport, and the one whose request and answer are not the
//! same exchange. Three decisions are worth reading before changing
//! anything here.
//!
//! **The stream is opened before anything is said.** The specification
//! has the server announce its message endpoint as the first event, so
//! there is nowhere to post to until the stream has spoken. Opening is
//! therefore a round trip with a deadline of its own, and a server that
//! never announces one is refused rather than posted to at a guessed
//! path.
//!
//! **The reader is a thread, and the deadline is real.** Reading a
//! stream blocks with no deadline of its own, exactly as a child
//! process's pipe does, so the same answer is used: a thread turns the
//! blocking read into a channel this side waits on. It cannot leak -
//! dropping the last handle drops the receiver, and the next send ends
//! the reader.
//!
//! **A post is not an answer.** The far end acknowledges a post with
//! 202 and says nothing; the answer arrives as a later event. So a call
//! posts, then waits on the stream, and a notification posts and is
//! done.

use std::io::BufRead;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use crate::mcp_redeeming::{Redeemed, redeem};

/// How long the stream is given to announce where messages go. Shorter
/// than a call's patience on purpose: this is one line from a server
/// that has already accepted the connection, and a person waiting to
/// learn whether their url is an MCP server at all is waiting on it.
const ANNOUNCEMENT_PATIENCE: Duration = Duration::from_secs(15);

/// A connection to one server reached over a stream.
///
/// Cloning shares the stream, because one server is one conversation
/// however many of its tools a run holds: two clones reading two streams
/// would be two conversations with one server, and the answer to a call
/// could arrive on the stream the caller is not reading.
#[derive(Clone)]
pub(crate) struct SseServer {
    /// Where the server said messages go, already resolved against the
    /// stream's own url.
    messages: String,
    headers: Arc<Vec<Redeemed>>,
    client: reqwest::blocking::Client,
    events: Arc<Mutex<Receiver<String>>>,
}

impl std::fmt::Debug for SseServer {
    /// Names the headers but never their values, for the reason
    /// `mcp_redeeming::Redeemed` gives.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseServer")
            .field("messages", &self.messages)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl SseServer {
    /// Opens the stream and waits for it to say where messages go.
    ///
    /// # Errors
    /// Refuses a header with no name and a reference the vault does not
    /// hold, a url this machine cannot reach, a stream the server
    /// refuses to open, and a stream that never announces an endpoint.
    pub(crate) fn open(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
    ) -> Result<SseServer, AxError> {
        let headers = Arc::new(redeem(headers, resolve, "reach an mcp server")?);
        let client = client_for(url)?;
        let mut request = client
            .get(url)
            .header("accept", "text/event-stream")
            .timeout(ANNOUNCEMENT_PATIENCE);
        for header in headers.iter() {
            request = request.header(header.name(), header.plaintext());
        }
        let response = request.send().map_err(|err| unreachable(url, &err))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(refused(url, status));
        }
        let events = read_in_a_thread(url, response)?;
        let announced = events
            .recv_timeout(ANNOUNCEMENT_PATIENCE)
            .map_err(|_| silent(url))?;
        let messages = resolved_against(url, &announced)?;
        Ok(SseServer {
            messages,
            headers,
            client,
            events: Arc::new(Mutex::new(events)),
        })
    }

    /// Posts one message. The far end acknowledges and says nothing;
    /// whatever it has to say arrives on the stream.
    fn post(&self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        let mut request = self
            .client
            .post(&self.messages)
            .timeout(Duration::from_millis(patience.0))
            .header("content-type", "application/json")
            .body(line.to_owned());
        for header in self.headers.iter() {
            request = request.header(header.name(), header.plaintext());
        }
        let response = request
            .send()
            .map_err(|err| unreachable(&self.messages, &err))?;
        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(());
        }
        Err(refused(&self.messages, status))
    }
}

impl protocol::Outbound for SseServer {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        self.post(line, patience)?;
        let events = self.events.lock().map_err(|_| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                format!(
                    "{}: the stream was left locked by a thread that died",
                    self.messages
                ),
            )
            .with_recovery("restart this city; the stream is opened again with it")
        })?;
        events
            .recv_timeout(Duration::from_millis(patience.0))
            .map_err(|why| match why {
                RecvTimeoutError::Timeout => AxError::failure(
                    AxCode::Timeout,
                    "call an mcp server",
                    format!("{}: no answer within {} ms", self.messages, patience.0),
                )
                .with_recovery(
                    "the server took the message and said nothing; this run continues without it",
                )
                .retriable(),
                RecvTimeoutError::Disconnected => AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an mcp server",
                    format!("{}: the stream ended", self.messages),
                )
                .with_recovery("the server closed the stream; dispatch again to open a new one"),
            })
    }

    /// A notification is posted and nothing is read back, because the
    /// far end will not answer it.
    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        self.post(line, patience)
    }
}

/// The reader thread: one `data:` line at a time, onto a channel this
/// side can wait on with a deadline.
///
/// Comments, event names and the reconnection hints of the event-stream
/// grammar are dropped rather than parsed: this transport carries JSON
/// messages, and every one of them is a `data:` line.
fn read_in_a_thread(
    url: &str,
    response: reqwest::blocking::Response,
) -> Result<Receiver<String>, AxError> {
    let (sender, events) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("mcp-sse".to_owned())
        .spawn(move || {
            for line in std::io::BufReader::new(response).lines() {
                // Either end finishing ends the reader: a closed stream
                // means the server is gone, and a closed channel means
                // this city stopped listening.
                let Ok(text) = line else { break };
                let Some(data) = text.strip_prefix("data:") else {
                    continue;
                };
                if sender.send(data.trim().to_owned()).is_err() {
                    break;
                }
            }
        })
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "reach an mcp server",
                format!("{url}: {err}"),
            )
            .with_recovery("the machine refused a thread to read this server's stream")
        })?;
    Ok(events)
}

/// Where messages go, from what the stream announced.
///
/// The announcement is commonly a path rather than a whole url, so it is
/// resolved against the stream's own address: a server behind a proxy
/// names the path it knows, and it does not know where this city reached
/// it.
fn resolved_against(url: &str, announced: &str) -> Result<String, AxError> {
    let announced = announced.trim();
    if announced.starts_with("http://") || announced.starts_with("https://") {
        return Ok(announced.to_owned());
    }
    let base = reqwest::Url::parse(url).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "reach an mcp server",
            format!("{url}: {err}"),
        )
        .with_recovery("write the stream's address as a whole url")
    })?;
    base.join(announced)
        .map(|joined| joined.to_string())
        .map_err(|err| {
            AxError::failure(
                AxCode::WireMismatch,
                "reach an mcp server",
                format!("{url}: the stream announced an endpoint this city cannot read: {err}"),
            )
            .with_recovery("the far end is not an MCP server, or speaks a revision without one")
        })
}

fn client_for(url: &str) -> Result<reqwest::blocking::Client, AxError> {
    // The city's own rule, for the reason `bin::mcp_http` gives: a tool
    // server carries no setting of its own, and a proxy in front of a
    // server on this machine answers for something else entirely.
    gateway::client_for(kernel::Proxying::ExceptLocal, url)
        // Named for the reason `bin::mcp_http` gives: a hosted server
        // behind a content delivery network refuses a client that will
        // not say what it is.
        .user_agent(concat!("sprawling/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
        })
}

fn silent(url: &str) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "reach an mcp server",
        format!("{url}: the stream announced no message endpoint"),
    )
    .with_recovery("check the url; a server reached by `transport = \"sse\"` announces one first")
}

fn refused(url: &str, status: u16) -> AxError {
    if status == 401 || status == 403 {
        return AxError::failure(
            AxCode::CredentialMissing,
            "reach an mcp server",
            format!("{url}: the server answered {status}"),
        )
        .with_recovery(
            "this server wants an account; store its key in the vault and name it in `headers`, \
             or sign in to it",
        );
    }
    // The body is not quoted: a server's error page is other people's
    // text and this refusal is read by a person.
    AxError::failure(
        AxCode::ToolUnavailable,
        "reach an mcp server",
        format!("{url}: the server answered {status}"),
    )
    .with_recovery("check the url and the headers this building configured")
}

fn unreachable(url: &str, err: &reqwest::Error) -> AxError {
    let mut detail = err.to_string();
    let mut cause = std::error::Error::source(err);
    while let Some(link) = cause {
        detail.push_str(": ");
        detail.push_str(&link.to_string());
        cause = link.source();
    }
    AxError::failure(
        AxCode::ToolUnavailable,
        "reach an mcp server",
        format!("{url}: {detail}"),
    )
    .with_recovery("the server is not answering; this run continues without it")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A path is what a server behind a proxy announces, and it is
    /// resolved against the address this city actually reached.
    #[test]
    fn an_announced_path_is_resolved_against_the_streams_own_address() {
        assert_eq!(
            resolved_against("https://example.test/sse", "/messages?sessionId=7").unwrap(),
            "https://example.test/messages?sessionId=7"
        );
    }

    /// A whole url is taken as it stands: a server that names another
    /// host means that host.
    #[test]
    fn an_announced_url_is_taken_as_it_stands() {
        assert_eq!(
            resolved_against("https://example.test/sse", "https://other.test/messages").unwrap(),
            "https://other.test/messages"
        );
    }

    /// A server that answers 401 is up, understood the request and wants
    /// an account. That is a different answer from a server that is
    /// down, and the code says so.
    #[test]
    fn a_server_wanting_an_account_refuses_with_the_credential_code() {
        assert_eq!(
            refused("https://example.test/sse", 401).code(),
            &AxCode::CredentialMissing
        );
        assert_eq!(
            refused("https://example.test/sse", 502).code(),
            &AxCode::ToolUnavailable
        );
    }
}
