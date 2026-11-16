// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, McpTransport, TimeoutMs};

use super::http::HttpServer;
use super::redeeming::redeem;
use super::sse::SseServer;
use super::stdio::StdioServer;

#[derive(Clone)]
pub struct McpLink(Reach);

#[derive(Clone)]
enum Reach {
    Stdio(StdioServer),
    Http(HttpServer),
    Sse(SseServer),
}

impl McpLink {

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

    #[must_use]
    pub fn site(transport: &McpTransport) -> &'static str {
        match *transport {
            McpTransport::Stdio { .. } => "agent_protocols::mcp::stdio",
            McpTransport::Http { .. } => "agent_protocols::mcp::http",
            McpTransport::Sse { .. } => "agent_protocols::mcp::sse",
        }
    }

    #[must_use]
    pub fn has_ended(&self) -> bool {
        match self.0 {
            Reach::Stdio(ref held) => held.has_ended(),
            Reach::Http(ref held) => held.has_ended(),
            Reach::Sse(_) => false,
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
