// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, TimeoutMs};

use super::prepared::LiveAsk;
use agent_protocols::McpLink;

const HANDSHAKE_PATIENCE: TimeoutMs = TimeoutMs(15_000);

impl LiveAsk {

    pub(super) fn mcp_health_answer(&self, addr: &Address) -> wire::McpHealthAnswer {
        let servers = match city::load_config(&self.city_root, addr) {
            Ok(config) => config
                .mcp
                .iter()
                .map(|server| self.reached(server))
                .collect(),
            Err(_) => Vec::new(),
        };
        wire::McpHealthAnswer {
            addr: addr.clone(),
            servers,
        }
    }

    fn reached(&self, server: &kernel::McpServer) -> wire::McpServerHealth {
        let (transport, target) = spelled(&server.transport);
        wire::McpServerHealth {
            label: server.label.clone(),
            transport,
            target,
            state: self.handshake(server),
        }
    }

    fn handshake(&self, server: &kernel::McpServer) -> wire::McpState {
        match self.listed(server) {
            Ok(state) => state,
            Err(err) if err.code() == &AxCode::CredentialMissing => {
                wire::McpState::Authenticating {
                    recovery: err.recovery().to_owned(),
                }
            }
            Err(err) => wire::McpState::Failed {
                refusal: Box::new(err),
            },
        }
    }

    fn listed(&self, server: &kernel::McpServer) -> Result<wire::McpState, AxError> {
        use agent_protocols::Outbound as _;

        let vault = self.vault.as_ref().ok_or_else(|| {
            AxError::failure(
                AxCode::CredentialMissing,
                "reach an mcp server",
                server.label.as_str().to_owned(),
            )
            .with_recovery("this city has no vault open; serve it and ask again")
        })?;
        let resolve = crate::held_vault::resolving(std::sync::Arc::clone(vault));

        let mut link = McpLink::open(&server.transport, &self.city_root, &resolve)?;
        let mut rpc = agent_protocols::Rpc::new();
        let opened = agent_protocols::handshake(&mut link, &mut rpc, HANDSHAKE_PATIENCE)?;
        let listing = link.call(&rpc.list_tools(), HANDSHAKE_PATIENCE)?;
        let listed =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(&listing)?)?;
        Ok(wire::McpState::Connected {
            protocol_version: opened.protocol_version,
            server: opened.server,
            tools: listed
                .into_iter()
                .map(|entry| wire::McpToolLine {
                    remote: entry.remote,
                    name: entry.meta.name.as_str().to_owned(),
                    disclosure: entry.meta.disclosure,
                    input_schema: entry.meta.params,
                })
                .collect(),
        })
    }
}

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

    #[test]
    fn an_address_that_configures_no_server_answers_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let live = LiveAsk {
            city_root: dir.path().to_path_buf(),
            city: None,
            vault: None,
        };
        let addr = Address::parse("lab/room1").unwrap();
        assert_eq!(
            live.mcp_health_answer(&addr),
            wire::McpHealthAnswer {
                addr,
                servers: Vec::new(),
            }
        );
    }
}
