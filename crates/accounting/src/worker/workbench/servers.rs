// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external tools a building's configuration names, each already
//! connected to its server or left out and named in the diagnostics.

use crate::Reached;

use super::Laying;

impl Laying {
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
    pub(in crate::worker) fn mcp_tools(
        &self,
        servers: &[kernel::McpServer],
        write_root: &std::path::Path,
        confidential: bool,
    ) -> Vec<agent_protocols::McpTool> {
        if confidential && !servers.is_empty() {
            // Process lifetime is this layer's own business, and an MCP
            // server is a program that may reach the network the moment
            // it starts. Nothing is started here. The tool-level refusal
            // in `agent_protocols::McpTool::new` stays the authority on whether
            // such a tool may exist; this is the earlier consequence of
            // that rule, not a second copy of it.
            self.note(
                runtime::diagnostics::Level::Refuse,
                "accounting::worker",
                "this building is confidential; no external server is started for it",
            );
            return Vec::new();
        }
        // The half of `[prepare_dispatch_ms]` this file owns: starting
        // every server this building declares and shaking hands with
        // each of them. Held apart from the whole-phase reading because
        // it is the part the resident connection table removes on every
        // dispatch after the first, and a figure that mixed the two could
        // not say how much.
        let began = (self.monotonic)();
        let mut offered = Vec::new();
        let resolve = crate::held_vault::resolving(std::sync::Arc::clone(&self.vault));
        for server in servers {
            // The module a reader is sent to is the transport that
            // failed, not whichever one was written first.
            let site = agent_protocols::McpLink::site(&server.transport);
            match self
                .connectors
                .connect(server, write_root, confidential, &resolve)
            {
                Ok((tools, reached)) => {
                    let how = match reached {
                        Reached::Connected(opened) => {
                            format!("{} speaking {}", opened.server, opened.protocol_version)
                        }
                        Reached::Resident => "already connected".to_owned(),
                    };
                    self.note(
                        runtime::diagnostics::Level::Effect,
                        site,
                        &format!(
                            "{} is {how}, offering {} tool(s)",
                            server.label.as_str(),
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
        let spent = (self.monotonic)()
            .saturating_duration_since(began)
            .as_millis();
        self.note(
            runtime::diagnostics::Level::Trace,
            "accounting::worker",
            &format!(
                "mcp_tools took {spent} ms over {} declared server(s), offering {} tool(s)",
                servers.len(),
                offered.len()
            ),
        );
        offered
    }
}
