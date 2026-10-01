// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One reachable server, whichever of the three transports reaches it.
//!
//! Whatever the transport, a call lost after its request was handed over
//! leaves the effect unknown and is never sent again by itself; that rule
//! is proved in `crates/agent_protocols/spec/Mcp/Link.lean`.

use kernel::{AxError, McpTransport, TimeoutMs};

use super::http::HttpServer;
use super::redeeming::redeem;
use super::sse::SseServer;
use super::stdio::StdioServer;

/// One reachable server, whichever way it is reached.
///
/// The three transports differ in where the bytes go and in nothing
/// else, so the difference is spent here and the wiring above stays one
/// path. A clone is a second handle on the same connection.
#[derive(Clone)]
pub struct McpLink(Reach);

#[derive(Clone)]
enum Reach {
    Stdio(StdioServer),
    Http(HttpServer),
    Sse(SseServer),
}

impl McpLink {
    /// Opens one server as its transport says it is reached.
    ///
    /// `write_root` is the run's own root, which exists whether or not
    /// the building lends its runs a worktree; a child process starts
    /// there and nowhere else.
    ///
    /// # Errors
    /// Propagates each transport's own refusal to open, every one of
    /// which names the server and what a person can do about it.
    pub fn open(
        transport: &McpTransport,
        write_root: &std::path::Path,
        resolve: &gateway::SecretResolver,
    ) -> Result<McpLink, AxError> {
        match *transport {
            McpTransport::Stdio {
                ref command,
                ref args,
                ref env,
            } => {
                let env = redeem(env, resolve, "start an mcp server")?;
                Ok(McpLink(Reach::Stdio(StdioServer::start(
                    command, args, &env, write_root,
                )?)))
            }
            McpTransport::Http {
                ref url,
                ref headers,
            } => Ok(McpLink(Reach::Http(HttpServer::open(
                url, headers, resolve,
            )?))),
            McpTransport::Sse {
                ref url,
                ref headers,
            } => Ok(McpLink(Reach::Sse(SseServer::open(url, headers, resolve)?))),
        }
    }

    /// The module a reader should open when a server reached this way
    /// misbehaves.
    #[must_use]
    pub fn site(transport: &McpTransport) -> &'static str {
        match *transport {
            McpTransport::Stdio { .. } => "agent_protocols::mcp::stdio",
            McpTransport::Http { .. } => "agent_protocols::mcp::http",
            McpTransport::Sse { .. } => "agent_protocols::mcp::sse",
        }
    }

    /// Whether the server behind this link is gone for good: a stdio
    /// child that exited. A url is reached again on each call, so it
    /// never ends here.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        match self.0 {
            Reach::Stdio(ref held) => held.has_ended(),
            Reach::Http(_) | Reach::Sse(_) => false,
        }
    }
}

impl crate::Outbound for McpLink {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        match self.0 {
            Reach::Stdio(ref mut held) => held.call(line, patience),
            Reach::Http(ref mut held) => held.call(line, patience),
            Reach::Sse(ref mut held) => held.call(line, patience),
        }
    }

    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError> {
        match self.0 {
            Reach::Stdio(ref mut held) => held.notify(line, patience),
            Reach::Http(ref mut held) => held.notify(line, patience),
            Reach::Sse(ref mut held) => held.notify(line, patience),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failure is filed under the module that reached the server, and
    /// that module lives in `agent_protocols` beside the handshake it speaks.
    #[test]
    fn a_misbehaving_server_is_filed_under_the_transport_that_reached_it() {
        let stdio = McpTransport::Stdio {
            command: String::new(),
            args: Vec::new(),
            env: Vec::new(),
        };
        let http = McpTransport::Http {
            url: String::new(),
            headers: Vec::new(),
        };
        let sse = McpTransport::Sse {
            url: String::new(),
            headers: Vec::new(),
        };
        assert_eq!(
            [&stdio, &http, &sse].map(McpLink::site),
            [
                "agent_protocols::mcp::stdio",
                "agent_protocols::mcp::http",
                "agent_protocols::mcp::sse"
            ]
        );
    }
}
