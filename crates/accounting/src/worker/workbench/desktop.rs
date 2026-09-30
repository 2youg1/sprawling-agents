// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The servers a run connects to: the ones its building's configuration
//! names, and the desktop server this binary carries when the building's
//! rules ask for it (sprawling-SPEC.md 8-4d).
//!
//! The desktop server is this same executable started as a child with
//! the `desktop` verb, so from here on it is an ordinary stdio server:
//! the connection table, the handshake, the frozen tool table and the
//! gate that escalates every `desktop.` tool all treat it like any
//! other. The child boundary is kept on purpose: COM, `SendInput` and
//! every `unsafe` block run there, never in the process that writes the
//! Ledger.

use kernel::{McpServer, McpTransport, ServerLabel};

use super::Laying;
use crate::assembly::Site;

/// The label the city's own desktop server travels under, and so the
/// prefix of the six tool names the model is offered.
pub(in crate::assembly) const DESKTOP_LABEL: &str = "desktop";

/// The verb that serves the desktop on stdio.
const DESKTOP_VERB: &str = "desktop";

impl Laying {
    /// Every server this run connects to: its building's own, then this
    /// binary's desktop server when the rules give the building's
    /// residents this machine's desktop.
    ///
    /// A building that names a server `desktop` itself keeps that one:
    /// two servers under one label would put one tool name in front of
    /// two processes, and the row a person wrote is the more deliberate
    /// statement. A program this process cannot name leaves the desktop
    /// out for this run, said in the diagnostics, and the dispatch goes
    /// on, the answer an external server that will not start gets.
    pub(in crate::assembly) fn servers(&self, site: &Site) -> Vec<McpServer> {
        let mut servers = site.config.mcp.clone();
        if !site.rules.desktop() {
            return servers;
        }
        if servers
            .iter()
            .any(|server| server.label.as_str() == DESKTOP_LABEL)
        {
            self.note(
                runtime::diagnostics::Level::Decide,
                "bin::assembly::workbench::desktop",
                "this building names its own `desktop` server, so the one this binary \
                 carries is not started; remove that [[mcp]] row to use the built-in one",
            );
            return servers;
        }
        match self.desktop_server(site) {
            Ok(server) => servers.push(server),
            Err(err) => self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly::workbench::desktop",
                &format!("the desktop is not offered: {err}; {}", err.recovery()),
            ),
        }
        servers
    }

    /// This binary started as the desktop server for the run's building,
    /// scoped by that building's own `DESKTOP.toml`.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when this process cannot say where its own
    /// executable is, or when that path or the scope file's is not UTF-8,
    /// which a command line in a building's configuration cannot carry.
    fn desktop_server(&self, site: &Site) -> Result<McpServer, kernel::AxError> {
        let program = (self.desktop_program)().map_err(|err| unavailable(&err.to_string()))?;
        let scope = city::desktop_scope_path(&self.city_root, site.building.addr());
        let spelled = |path: &std::path::Path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| unavailable(&format!("{} is not UTF-8", path.display())))
        };
        Ok(McpServer {
            label: ServerLabel::parse(DESKTOP_LABEL)?,
            transport: McpTransport::Stdio {
                command: spelled(&program)?,
                args: vec![DESKTOP_VERB.to_owned(), spelled(&scope)?],
                env: Vec::new(),
            },
        })
    }
}

fn unavailable(subject: &str) -> kernel::AxError {
    kernel::AxError::failure(
        kernel::AxCode::ToolUnavailable,
        "start this machine's desktop server",
        subject.to_owned(),
    )
    .with_recovery("run sprawling from a folder whose path is plain UTF-8, then dispatch again")
}
