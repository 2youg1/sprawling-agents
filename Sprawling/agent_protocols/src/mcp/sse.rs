// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::http::{WholeRequest, client_for, standing};
use super::reading::answer_unread;
use super::redeeming::{Redeemed, redeem};
use super::{Received, read_one_message};

const ANNOUNCEMENT_PATIENCE: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(crate) struct SseServer {

    messages: String,
    headers: Arc<Vec<Redeemed>>,
    client: reqwest::blocking::Client,
    events: Arc<Mutex<Receiver<Result<String, AxError>>>>,
}

impl std::fmt::Debug for SseServer {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseServer")
            .field("messages", &self.messages)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl SseServer {

    pub(crate) fn open(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
    ) -> Result<SseServer, AxError> {
        SseServer::open_within(url, headers, resolve, ANNOUNCEMENT_PATIENCE)
    }

    fn open_within(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
        announcement: Duration,
    ) -> Result<SseServer, AxError> {
        let headers = Arc::new(redeem(headers, resolve, "reach an mcp server")?);
        let client = client_for(url, WholeRequest::Unbounded)?;

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

            .map_err(|refused| answer_unread(&refused))
    }

    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        self.post(line, patience)
    }
}

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

                    drop(sender.send(Err(refusal)));
                    return;
                }
            };
            let mut reader = std::io::BufReader::new(response);
            loop {

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
