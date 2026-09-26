// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One MCP server reached over whichever transport its configuration
//! names (sprawling-SPEC.md section 8-92).
//!
//! Beside the three transports it joins, because it is their union and
//! nothing more: the assembly point opens one for a run's tool loop and
//! the MCP health view opens one to ask a server what it offers, and
//! both then speak `protocol` over it.

use kernel::AxError;

/// One reachable server, whichever way it is reached.
///
/// The three transports differ in where the bytes go and in nothing
/// else, so the difference is spent here and the wiring above stays one
/// path.
#[derive(Clone)]
pub(crate) enum McpLink {
    Stdio(crate::mcp_stdio::StdioServer),
    Http(crate::mcp_http::HttpServer),
    Sse(crate::mcp_sse::SseServer),
}

impl McpLink {
    /// Opens one server as its transport says it is reached.
    ///
    /// `write_root` is the run's own root, which exists whether or not
    /// this building lends its runs a worktree; a child process starts
    /// there and nowhere else.
    ///
    /// # Errors
    /// Propagates each transport's own refusal to open, every one of
    /// which names the server and what a person can do about it.
    pub(crate) fn open(
        transport: &kernel::McpTransport,
        write_root: &std::path::Path,
        resolve: &gateway::SecretResolver,
    ) -> Result<McpLink, AxError> {
        match *transport {
            kernel::McpTransport::Stdio {
                ref command,
                ref args,
                ref env,
            } => {
                let env = crate::mcp_redeeming::redeem(env, resolve, "start an mcp server")?;
                Ok(McpLink::Stdio(crate::mcp_stdio::StdioServer::start(
                    command, args, &env, write_root,
                )?))
            }
            kernel::McpTransport::Http {
                ref url,
                ref headers,
            } => Ok(McpLink::Http(crate::mcp_http::HttpServer::open(
                url, headers, resolve,
            )?)),
            kernel::McpTransport::Sse {
                ref url,
                ref headers,
            } => Ok(McpLink::Sse(crate::mcp_sse::SseServer::open(
                url, headers, resolve,
            )?)),
        }
    }
}

impl protocol::Outbound for McpLink {
    fn call(&mut self, line: &str, patience: kernel::TimeoutMs) -> Result<String, AxError> {
        match *self {
            McpLink::Stdio(ref mut held) => held.call(line, patience),
            McpLink::Http(ref mut held) => held.call(line, patience),
            McpLink::Sse(ref mut held) => held.call(line, patience),
        }
    }

    fn notify(&mut self, line: &str, patience: kernel::TimeoutMs) -> Result<(), AxError> {
        match *self {
            McpLink::Stdio(ref mut held) => held.notify(line, patience),
            McpLink::Http(ref mut held) => held.notify(line, patience),
            McpLink::Sse(ref mut held) => held.notify(line, patience),
        }
    }
}
