// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::reading::{answer_unread, read_whole_message};
use super::redeeming::{Redeemed, redeem};

#[derive(Debug, Default)]
struct Session {
    id: Option<String>,

    protocol_version: Option<String>,
    lifetime: Lifetime,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Lifetime {
    #[default]
    Live,
    Ended,
}

#[derive(Clone)]
pub(crate) struct HttpServer {
    url: String,
    headers: Vec<Redeemed>,
    client: reqwest::blocking::Client,
    session: Arc<Mutex<Session>>,
}

impl std::fmt::Debug for HttpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpServer")
            .field("url", &self.url)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl HttpServer {
    pub(crate) fn open(
        url: &str,
        headers: &[(String, String)],
        resolve: &gateway::SecretResolver,
    ) -> Result<HttpServer, AxError> {
        let headers = redeem(headers, resolve, "reach an mcp server")?;
        let client = client_for(url, WholeRequest::DefaultTimeout)?;
        Ok(HttpServer {
            url: url.to_owned(),
            headers,
            client,
            session: Arc::new(Mutex::new(Session::default())),
        })
    }

    fn post(&self, line: &str, patience: TimeoutMs) -> Result<Exchange, AxError> {
        let mut request = self
            .client
            .post(&self.url)
            .timeout(Duration::from_millis(patience.0))
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(line.to_owned());
        for header in &self.headers {
            request = request.header(header.name(), header.plaintext());
        }
        let mut carried = None;
        {
            let session = self.session.lock().map_err(|_| {
                AxError::failure(AxCode::ToolUnavailable, "call an mcp server", &self.url)
                    .with_recovery("the session lock was poisoned; dispatch again to reconnect")
            })?;
            if session.lifetime == Lifetime::Ended {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an mcp server",
                    &self.url,
                )
                .with_recovery("this session ended; dispatch again to open a new connection"));
            }
            if let Some(id) = &session.id {
                request = request.header("mcp-session-id", id);
                carried = Some(id.clone());
            }
            if let Some(version) = &session.protocol_version {
                request = request.header("mcp-protocol-version", version);
            }
        }
        let mut response = request.send().map_err(|err| self.unreachable(&err))?;
        let status = response.status().as_u16();
        let handed = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        let body = if (200..300).contains(&status) {
            read_whole_message(&mut response, &self.url)
                .map_err(|refused| answer_unread(&refused))?
        } else {
            String::new()
        };
        Ok(Exchange {
            status,
            carried,
            handed,
            body,
        })
    }

    fn learn(&self, exchange: &Exchange) {
        let Ok(mut session) = self.session.lock() else {
            return;
        };
        if session.lifetime == Lifetime::Ended {
            return;
        }
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

    fn forget(&self) {
        if let Ok(mut session) = self.session.lock() {
            session.id = None;
            session.protocol_version = None;
            session.lifetime = Lifetime::Ended;
        }
    }

    pub(crate) fn has_ended(&self) -> bool {
        match self.session.lock() {
            Ok(session) => session.lifetime == Lifetime::Ended,
            Err(_poisoned) => true,
        }
    }

    fn refused(&self, exchange: &Exchange) -> AxError {
        let status = exchange.status;

        if status == 401 || status == 403 {
            return standing(
                status,
                AxError::failure(
                    AxCode::CredentialMissing,
                    "call an mcp server",
                    format!("{}: the server answered {status}", self.url),
                ),
            )
            .with_recovery(
                "this server wants an account; store its key in the vault and name it in \
                 `headers`, or sign in to it",
            );
        }
        if status == 404 {
            return match &exchange.carried {
                Some(_) => {
                    self.forget();
                    AxError::failure(
                        AxCode::ToolUnavailable,
                        "call an mcp server",
                        format!("{}: the server ended this session", self.url),
                    )
                    .retriable()
                    .with_recovery("the session was dropped; dispatch again to open a new one")
                }
                None => AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an mcp server",
                    format!("{}: the server answered 404", self.url),
                )
                .with_recovery("nothing answers MCP at this url; check it in the MCP settings"),
            };
        }

        standing(
            status,
            AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                format!("{}: the server answered {status}", self.url),
            ),
        )
        .with_recovery("check the url and the header this building configured")
    }
}

pub(super) fn standing(status: u16, draft: kernel::ErrorDraft) -> kernel::ErrorDraft {
    match status {
        401 => draft.account_unusable(),
        408 | 429 | 502 | 503 | 504 => draft.retriable(),
        _ => draft,
    }
}

struct Exchange {
    status: u16,

    carried: Option<String>,

    handed: Option<String>,

    body: String,
}

impl crate::Outbound for HttpServer {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        let exchange = self.post(line, patience)?;
        if !(200..300).contains(&exchange.status) {
            return Err(self.refused(&exchange));
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

    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        let exchange = self.post(line, patience)?;
        if !(200..300).contains(&exchange.status) {
            return Err(self.refused(&exchange));
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

pub(super) enum WholeRequest {
    DefaultTimeout,

    Unbounded,
}

pub(super) fn client_for(
    url: &str,
    whole_request: WholeRequest,
) -> Result<reqwest::blocking::Client, AxError> {
    let builder = gateway::client_for(kernel::Proxying::ExceptLocal, url);
    match whole_request {
        WholeRequest::DefaultTimeout => builder,
        WholeRequest::Unbounded => builder.timeout(None),
    }
    .build()
    .map_err(|err| {
        AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string()).with_recovery(
            "check this server's url in the MCP settings and the proxy settings \
                     this machine exports (`HTTPS_PROXY`, `NO_PROXY`)",
        )
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
