// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external tools a building's configuration names, each already
//! connected to its server or left out and named in the diagnostics.

use super::super::{RunWorker, connect_mcp, transport_site};

impl RunWorker {
    /// The external tools this run may reach, each already connected to
    /// its server.
    ///
    /// A server that cannot be started, cannot be asked what it offers,
    /// or offers something this city cannot name is left out and named
    /// in the diagnostics. That is the answer `city::library` already
    /// gives a building admitting a skill which is not on the shelves,
    /// and it holds for the same reason: what the model is told exists
    /// must equal what actually runs, and a building whose external
    /// service is down today is still a building that can work today.
    pub(in crate::assembly) fn mcp_tools(
        &mut self,
        config: &kernel::FrozenConfig,
        write_root: &std::path::Path,
        confidential: bool,
    ) -> Vec<protocol::McpTool> {
        if confidential && !config.mcp.is_empty() {
            // Process lifetime is this layer's own business, and an MCP
            // server is a program that may reach the network the moment
            // it starts. Nothing is started here. The tool-level refusal
            // in `protocol::McpTool::new` stays the authority on whether
            // such a tool may exist; this is the earlier consequence of
            // that rule, not a second copy of it.
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                "this building is confidential; no external server is started for it",
            );
            return Vec::new();
        }
        let mut offered = Vec::new();
        let resolve = self.resolver();
        for server in &config.mcp {
            // The module a reader is sent to is the transport that
            // failed, not whichever one was written first: every MCP
            // failure used to be filed under `bin::mcp_stdio`, which
            // sent the last reader who followed it to the wrong file.
            let site = transport_site(&server.transport);
            match connect_mcp(server, write_root, confidential, &resolve) {
                Ok((tools, opened)) => {
                    self.note(
                        runtime::diagnostics::Level::Effect,
                        site,
                        &format!(
                            "{} is {} speaking {}, offering {} tool(s)",
                            server.label.as_str(),
                            opened.server,
                            opened.protocol_version,
                            tools.len()
                        ),
                    );
                    offered.extend(tools);
                }
                Err(err) => self.note(
                    runtime::diagnostics::Level::Refuse,
                    site,
                    &format!("{}: {err}; {}", server.label.as_str(), err.recovery()),
                ),
            }
        }
        offered
    }
}
