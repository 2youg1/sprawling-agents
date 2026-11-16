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
//! the reader. As on the child's pipe, it reads one line at a time
//! through `read_one_message` and hands it over a channel with no
//! queue, so the stream holds at most one line this city has read and
//! nobody has taken (`crates/agent_protocols/Spec.lean` §8-15).
//!
//! **A post is not an answer.** The far end acknowledges a post with
//! 202 and says nothing; the answer arrives as a later event. So a call
//! posts, then waits on the stream, and a notification posts and is
//! done.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::http::{WholeRequest, client_for, standing};
use super::reading::answer_unread;
use super::redeeming::{Redeemed, redeem};
use super::{Received, read_one_message};

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
    events: Arc<Mutex<Receiver<Result<String, AxError>>>>,
}

impl std::fmt::Debug for SseServer {
    /// Names the headers but never their values, for the reason
    /// `redeeming::Redeemed` gives.
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
        SseServer::open_within(url, headers, resolve, ANNOUNCEMENT_PATIENCE)
    }

    /// [`SseServer::open`] with the announcement's deadline named, so a
    /// test can wait on a stream for less than the fifteen seconds a
    /// person is given.
    fn open_within(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
        announcement: Duration,
    ) -> Result<SseServer, AxError> {
        let headers = Arc::new(redeem(headers, resolve, "reach an mcp server")?);
        let client = client_for(url, WholeRequest::Unbounded)?;
        // No request-level timeout: reqwest counts it until the body is
        // read to the end, and this body is the whole conversation.
        let mut request = client.get(url).header("accept", "text/event-stream");
        for header in headers.iter() {
            request = request.header(header.name(), header.plaintext());
        }
        let events = read_in_a_thread(url, request)?;
        let announced = events
            .recv_timeout(announcement)
            .map_err(|_| silent(url))??;
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

impl crate::Outbound for SseServer {
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
                .effect_unknown()
                .with_recovery(
                    "the server took the message and said nothing, and may have acted on it; \
                     check what it was asked to do before asking again",
                ),
                RecvTimeoutError::Disconnected => AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an mcp server",
                    format!("{}: the stream ended", self.messages),
                )
                .effect_unknown()
                .with_recovery(
                    "the stream ended after the server took the message, and it may have acted \
                     on it; check what it was asked to do, then dispatch again to open a new \
                     stream",
                ),
            })?
            // A refusal here is the server's answer, read and refused:
            // it took the call, so what it did is unknown.
            .map_err(|refused| answer_unread(&refused))
    }

    /// A notification is posted and nothing is read back, because the
    /// far end will not answer it.
    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        self.post(line, patience)
    }
}

/// The reader thread: it opens the stream, then passes one `data:` line
/// at a time onto a channel this side can wait on with a deadline.
///
/// Opening happens on the thread so that a server that accepts the
/// connection and never answers is bounded by the deadline this side
/// waits with, not by a timeout on the request that would also end the
/// stream. A stream that cannot be opened sends its refusal as the
/// first and only message.
///
/// Comments, event names and the reconnection hints of the event-stream
/// grammar are dropped rather than parsed: this transport carries JSON
/// messages, and every one of them is a `data:` line. Every line is read
/// under `MESSAGE_CEILING`, and a line the reader refuses is its last
/// message: the rest of that line cannot be told from the next one.
fn read_in_a_thread(
    url: &str,
    request: reqwest::blocking::RequestBuilder,
) -> Result<Receiver<Result<String, AxError>>, AxError> {
    let (sender, events) = std::sync::mpsc::sync_channel(0);
    let opened_at = url.to_owned();
    std::thread::Builder::new()
        .name("mcp-sse".to_owned())
        .spawn(move || {
            let response = match opened(&opened_at, request) {
                Ok(response) => response,
                Err(refusal) => {
                    // Nobody may be waiting any more; the refusal has
                    // then been reported as silence already.
                    drop(sender.send(Err(refusal)));
                    return;
                }
            };
            let mut reader = std::io::BufReader::new(response);
            loop {
                // Either end finishing ends the reader: a closed stream
                // means the server is gone, a refused line is the last
                // thing it can read, and a closed channel means this city
                // stopped listening.
                let data = match read_one_message(&mut reader, &opened_at) {
                    Ok(Received::Message(line)) => match line.strip_prefix("data:") {
                        Some(data) => Ok(data.trim().to_owned()),
                        None => continue,
                    },
                    Ok(Received::EndOfInput) => break,
                    Err(refused) => Err(refused),
                };
                let last = data.is_err();
                if sender.send(data).is_err() || last {
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

fn opened(
    url: &str,
    request: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::Response, AxError> {
    let response = request.send().map_err(|err| unreachable(url, &err))?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    Err(refused(url, status))
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
        return standing(
            status,
            AxError::failure(
                AxCode::CredentialMissing,
                "reach an mcp server",
                format!("{url}: the server answered {status}"),
            ),
        )
        .with_recovery(
            "this server wants an account; store its key in the vault and name it in `headers`, \
             or sign in to it",
        );
    }
    // The body is not quoted: a server's error page is other people's
    // text and this refusal is read by a person.
    standing(
        status,
        AxError::failure(
            AxCode::ToolUnavailable,
            "reach an mcp server",
            format!("{url}: the server answered {status}"),
        ),
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
mod tests;
