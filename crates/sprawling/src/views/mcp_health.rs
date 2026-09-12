// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where each tool server one address reaches stands, asked at the
//! moment somebody wants to know.
//!
//! It is here rather than beside the folded answers because it folds
//! nothing: a server's state is a fact about now - a program that
//! starts, a host that answers, an account that is still valid - and a
//! remembered one would tell a person their server is up an hour after
//! it stopped. The configuration is read the same way `Document` and
//! `Listing` read the tree, and the handshake is the one a run opens
//! with, so what a person is shown and what a model is given cannot
//! disagree.
//!
//! **This is the one read that costs seconds**, and a page asks it when
//! somebody opens the MCP page or adds a server rather than on a timer.

use kernel::{Address, AxCode, AxError, TimeoutMs};

use super::holding::Views;
use crate::assembly::McpLink;

/// How long one server is given to finish `initialize` and list what it
/// offers. Shorter than a tool call's patience, because a person is
/// waiting in front of this answer rather than a model: a server that
/// has said nothing in this long is one they need told about.
const HANDSHAKE_PATIENCE: TimeoutMs = TimeoutMs(15_000);

impl Views {
    /// Reaches every server this address's configuration names, in the
    /// order it names them.
    ///
    /// A configuration this city cannot read answers an empty list
    /// rather than a refusal: the address exists, it reaches no server,
    /// and the page draws that as a building with none.
    pub(super) fn mcp_health_answer(&self, addr: &Address) -> channels::McpHealthAnswer {
        let servers = match city::load_config(&self.city_root, addr) {
            Ok(config) => config
                .mcp
                .iter()
                .map(|server| self.reached(server))
                .collect(),
            Err(_) => Vec::new(),
        };
        channels::McpHealthAnswer {
            addr: addr.clone(),
            servers,
        }
    }

    /// One server: how it is reached, and what happened when it was.
    fn reached(&self, server: &kernel::McpServer) -> channels::McpServerHealth {
        let (transport, target) = spelled(&server.transport);
        channels::McpServerHealth {
            label: server.label.clone(),
            transport,
            target,
            state: self.handshake(server),
        }
    }

    /// The handshake, turned into the three answers a person acts on
    /// differently.
    ///
    /// `CredentialMissing` is `Authenticating` rather than a failure
    /// because the server is up and understood the request: what to do
    /// about it is sign in, not check the address. Every transport
    /// raises that code from one place, so this mapping has nothing to
    /// guess (`bin::mcp_http`, `bin::mcp_sse`).
    fn handshake(&self, server: &kernel::McpServer) -> channels::McpState {
        match self.listed(server) {
            Ok(state) => state,
            Err(err) if err.code() == &AxCode::CredentialMissing => {
                channels::McpState::Authenticating {
                    recovery: err.recovery().to_owned(),
                }
            }
            Err(err) => channels::McpState::Failed {
                refusal: Box::new(err),
            },
        }
    }

    /// Opens the connection, agrees a revision, and asks what it offers.
    ///
    /// # Errors
    /// Propagates the transport's refusal to open, the far end's refusal
    /// to handshake, and a tool list this build cannot read - each of
    /// which already names the action, the subject and a recovery.
    fn listed(&self, server: &kernel::McpServer) -> Result<channels::McpState, AxError> {
        use protocol::Outbound as _;

        let vault = self.vault.as_ref().ok_or_else(|| {
            AxError::failure(
                AxCode::CredentialMissing,
                "reach an mcp server",
                server.label.as_str().to_owned(),
            )
            .with_recovery("this city has no vault open; serve it and ask again")
        })?;
        let resolve = crate::assembly::resolving(std::sync::Arc::clone(vault));
        // A server is reached from the city root rather than from a
        // run's worktree: nothing is running, and the tree a run would
        // have does not exist yet.
        let mut link = McpLink::open(&server.transport, &self.city_root, &resolve)?;
        let mut rpc = protocol::Rpc::new();
        let opened = protocol::handshake(&mut link, &mut rpc, HANDSHAKE_PATIENCE)?;
        let listing = link.call(&rpc.list_tools(), HANDSHAKE_PATIENCE)?;
        let listed = protocol::tools_from(&server.label, &protocol::Rpc::read(&listing)?)?;
        Ok(channels::McpState::Connected {
            protocol_version: opened.protocol_version,
            server: opened.server,
            tools: listed
                .into_iter()
                .map(|entry| channels::McpToolLine {
                    remote: entry.remote,
                    name: entry.meta.name.as_str().to_owned(),
                    disclosure: entry.meta.disclosure,
                    input_schema: entry.meta.params,
                })
                .collect(),
        })
    }
}

/// The word a person chose for this transport, and the one line that
/// says where it goes.
///
/// Exhaustive, so a transport added to the configuration cannot reach a
/// page as a blank target.
fn spelled(transport: &kernel::McpTransport) -> (String, String) {
    match *transport {
        kernel::McpTransport::Stdio {
            ref command,
            ref args,
            ..
        } => {
            let mut line = command.clone();
            for arg in args {
                line.push(' ');
                line.push_str(arg);
            }
            ("stdio".to_owned(), line)
        }
        kernel::McpTransport::Http { ref url, .. } => ("http".to_owned(), url.clone()),
        kernel::McpTransport::Sse { ref url, .. } => ("sse".to_owned(), url.clone()),
    }
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

    /// The target line is what a person compares against what they
    /// typed, so a command carries its arguments and a url is itself.
    #[test]
    fn every_transport_spells_its_word_and_the_line_that_says_where_it_goes() {
        assert_eq!(
            spelled(&kernel::McpTransport::Stdio {
                command: "mcp-apps".to_owned(),
                args: vec!["--stdio".to_owned(), "--quiet".to_owned()],
                env: vec![("API_KEY".to_owned(), "secret:mcp/apps".to_owned())],
            }),
            ("stdio".to_owned(), "mcp-apps --stdio --quiet".to_owned())
        );
        assert_eq!(
            spelled(&kernel::McpTransport::Http {
                url: "https://example.test/mcp".to_owned(),
                headers: Vec::new(),
            }),
            ("http".to_owned(), "https://example.test/mcp".to_owned())
        );
        assert_eq!(
            spelled(&kernel::McpTransport::Sse {
                url: "https://example.test/sse".to_owned(),
                headers: Vec::new(),
            }),
            ("sse".to_owned(), "https://example.test/sse".to_owned())
        );
    }

    /// An address with no configuration reaches no server. That is a
    /// building with none, not a city that could not look.
    #[test]
    fn an_address_that_configures_no_server_answers_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let views = Views::new(dir.path());
        let addr = Address::parse("lab/room1").unwrap();
        assert_eq!(
            views.mcp_health_answer(&addr),
            channels::McpHealthAnswer {
                addr,
                servers: Vec::new(),
            }
        );
    }
}
