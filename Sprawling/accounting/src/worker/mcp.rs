// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::Reached;
use agent_protocols::Outbound as _;
use kernel::AxError;

use super::RunWorker;

#[derive(Default)]
pub struct Residents {
    keys: std::sync::Mutex<Vec<std::sync::Arc<Keyed>>>,
}

struct Keyed {
    server: kernel::McpServer,
    root: std::path::PathBuf,
    connected: std::sync::Mutex<Option<Resident>>,
}

struct Resident {
    label: kernel::ServerLabel,
    link: agent_protocols::McpLink,

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

    fn release_root(&self, write_root: &std::path::Path) {
        let mut keys = self
            .keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        keys.retain(|keyed| keyed.root != write_root);
    }

    fn invalidate(&self, reference: &kernel::SecretRef) {
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
    fn carries(&self, reference: &kernel::SecretRef) -> bool {
        names(&self.server, reference)
    }
}

pub(in crate::worker) fn names(server: &kernel::McpServer, reference: &kernel::SecretRef) -> bool {
    let pairs = match &server.transport {
        kernel::McpTransport::Stdio { env, .. } => env,
        kernel::McpTransport::Http { headers, .. } | kernel::McpTransport::Sse { headers, .. } => {
            headers
        }
    };
    pairs.iter().any(|(_, value)| {
        kernel::SecretRef::parse(value.trim()).is_ok_and(|named| named == *reference)
    })
}

impl Residents {
    pub fn tools(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError> {
        let keyed = self.keyed(server, write_root)?;
        let mut connected = super::workbench::held(&keyed.connected, "reach an mcp server")?;
        if let Some(mut resident) = connected
            .take()
            .filter(|resident| !resident.link.has_ended())
        {
            let tools = resident.refresh(confidential);
            *connected = Some(resident);
            return Ok((tools?, Reached::Resident));
        }
        let (resident, opened) = Resident::connect(server, write_root, resolve)?;
        let tools = resident.tools(confidential)?;
        *connected = Some(resident);
        Ok((tools, Reached::Connected(opened)))
    }

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
    fn connect(
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Resident, agent_protocols::Handshake), AxError> {
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
        Ok((
            Resident {
                label: server.label.clone(),
                link,
                listed,
            },
            opened,
        ))
    }

    fn refresh(&mut self, confidential: bool) -> Result<Vec<agent_protocols::McpTool>, AxError> {
        let mut rpc = agent_protocols::Rpc::new();
        let listing = self
            .link
            .call(&rpc.list_tools(), agent_protocols::EXTERNAL_CALL_PATIENCE)?;
        self.listed =
            agent_protocols::tools_from(&self.label, &agent_protocols::Rpc::read(&listing)?)?
                .into_iter()
                .map(|entry| (entry.meta, entry.remote))
                .collect();
        self.tools(confidential)
    }

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
