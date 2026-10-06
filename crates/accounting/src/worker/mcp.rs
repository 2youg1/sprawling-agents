// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reaching the MCP servers a building's configuration names: the
//! production `crate::Connectors`, and the one door that swaps it
//! for another (`crates/accounting/spec/Connectors.lean` §8-2).

use crate::Reached;
use kernel::{Address, AxError};

use super::RunWorker;

/// Every MCP server this worker has reached, kept connected between
/// runs, so a dispatch pays for a child and a handshake only when its
/// server was not already running (`crates/sprawling/Spec.lean` §8-4).
///
/// Keyed by the whole declaration and the run root the child started
/// in: a changed field is another server, and a child cannot move to
/// another working directory once it runs.
///
/// Each key has its own lock, and the table's lock is held only to find
/// or add a key: a lane still shaking hands with one server holds up
/// the lanes asking for that server and nobody else.
#[derive(Default)]
pub struct Residents {
    keys: std::sync::Mutex<Vec<std::sync::Arc<Keyed>>>,
}

/// One server this worker was asked for, connected or not.
struct Keyed {
    server: kernel::McpServer,
    root: std::path::PathBuf,
    connected: std::sync::Mutex<Option<Resident>>,
}

/// One connected server and what it offered when it connected.
struct Resident {
    link: agent_protocols::McpLink,
    /// Each tool under both its names, in the order the server listed
    /// them.
    listed: Vec<(kernel::ToolMeta, String)>,
}

impl crate::Connectors for Residents {
    fn connect(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError> {
        self.tools(server, write_root, confidential, resolve)
    }

    fn invalidate(&self, reference: &kernel::SecretRef) {
        // A poisoned lock is cleared rather than skipped: what it guards
        // is a connection that may hold the replaced value, and dropping
        // it is the one outcome that is right whatever the panic left.
        let keys = self
            .keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for keyed in keys.iter().filter(|keyed| keyed.carries(reference)) {
            *keyed
                .connected
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }
}

impl Keyed {
    /// Whether one of the pairs this server is started or reached with
    /// names `reference`, read by the grammar redemption reads it with.
    fn carries(&self, reference: &kernel::SecretRef) -> bool {
        let pairs = match &self.server.transport {
            kernel::McpTransport::Stdio { env, .. } => env,
            kernel::McpTransport::Http { headers, .. }
            | kernel::McpTransport::Sse { headers, .. } => headers,
        };
        pairs.iter().any(|(_, value)| {
            kernel::SecretRef::parse(value.trim()).is_ok_and(|named| named == *reference)
        })
    }
}

impl Residents {
    /// The tools `server` offers, over the connection an earlier run left
    /// when its child still runs, and over a new one otherwise.
    ///
    /// An ended child or HTTP session is dropped from the table. Its
    /// replacement opens, handshakes and lists tools through the same
    /// path; the failed call itself is never replayed here.
    ///
    /// Takes `&self`, so lanes preparing dispatches at once can share
    /// one table; the port's `connect` reaches the same door.
    ///
    /// # Errors
    /// Propagates the transport's refusal to open, a failed handshake or
    /// listing, and `agent_protocols::McpTool::new`'s refusal on a confidential
    /// building.
    pub fn tools(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError> {
        let keyed = self.keyed(server, write_root)?;
        let mut connected = super::workbench::held(&keyed.connected, "reach an mcp server")?;
        if let Some(resident) = connected
            .take()
            .filter(|resident| !resident.link.has_ended())
        {
            let tools = resident.tools(confidential);
            *connected = Some(resident);
            return Ok((tools?, Reached::Resident));
        }
        let (resident, opened) = Resident::connect(server, write_root, resolve)?;
        let tools = resident.tools(confidential)?;
        *connected = Some(resident);
        Ok((tools, Reached::Connected(opened)))
    }

    /// The entry for `server` started in `write_root`, added when this
    /// is the first time it is asked for.
    fn keyed(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
    ) -> Result<std::sync::Arc<Keyed>, AxError> {
        let mut keys = super::workbench::held(&self.keys, "find an mcp server's entry")?;
        if let Some(found) = keys
            .iter()
            .find(|keyed| keyed.server == *server && keyed.root == write_root)
        {
            return Ok(std::sync::Arc::clone(found));
        }
        let added = std::sync::Arc::new(Keyed {
            server: server.clone(),
            root: write_root.to_path_buf(),
            connected: std::sync::Mutex::new(None),
        });
        keys.push(std::sync::Arc::clone(&added));
        Ok(added)
    }
}

impl RunWorker {
    /// The same worker, reaching every MCP server through `connectors`
    /// instead of starting the ones a building's configuration names.
    ///
    /// The door citysim and the dispatch tests drive a worker through:
    /// what a server offers is theirs to script, while refusing servers
    /// to a confidential building and leaving a failed one out stay the
    /// worker's.
    #[must_use]
    pub fn with_connectors(
        self,
        connectors: Box<dyn crate::Connectors + Send + Sync>,
    ) -> RunWorker {
        RunWorker {
            connectors: std::sync::Arc::from(connectors),
            ..self
        }
    }
}

impl Resident {
    /// Starts one server and asks what it offers.
    ///
    /// The connection opens with the lifecycle the specification defines -
    /// `initialize`, then `notifications/initialized` - and only then asks
    /// what it offers. What the handshake learns is written to the
    /// diagnostics rather than branched on: negotiating a version needs a
    /// second version this build can speak before it can decide anything.
    fn connect(
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Resident, agent_protocols::Handshake), AxError> {
        use agent_protocols::Outbound as _;

        // The run's own root, which exists whether or not this building
        // lends its runs a worktree.
        let mut link = agent_protocols::McpLink::open(&server.transport, write_root, resolve)?;
        let mut rpc = agent_protocols::Rpc::new();
        let opened = agent_protocols::handshake(
            &mut link,
            &mut rpc,
            agent_protocols::EXTERNAL_CALL_PATIENCE,
        )?;
        let listing = link.call(&rpc.list_tools(), agent_protocols::EXTERNAL_CALL_PATIENCE)?;
        let listed =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(&listing)?)?
                .into_iter()
                .map(|entry| (entry.meta, entry.remote))
                .collect();
        Ok((Resident { link, listed }, opened))
    }

    /// One handle per tool on the one connection: two connections would
    /// be two answers to what the same label offers.
    fn tools(&self, confidential: bool) -> Result<Vec<agent_protocols::McpTool>, AxError> {
        self.listed
            .iter()
            .map(|(meta, remote)| {
                agent_protocols::McpTool::new(
                    meta.clone(),
                    remote.clone(),
                    Box::new(self.link.clone()),
                    confidential,
                )
            })
            .collect()
    }
}

/// Turns the configured mount list into paths under the run's write
/// root. Read-only by construction: the sandbox job carries them as
/// readable, and what may be written is the write domain's answer.
pub(super) fn mounts_under(
    write_root: &std::path::Path,
    mounts: &[Address],
) -> Vec<runtime::Mount> {
    mounts
        .iter()
        .map(|addr| runtime::Mount {
            host: write_root.join(addr.as_str()),
            guest: format!("/{}", addr.as_str()),
            writable: false,
        })
        .collect()
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
