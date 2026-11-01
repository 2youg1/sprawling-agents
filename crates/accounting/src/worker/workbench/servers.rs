// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external tools a building's configuration names, pinned to the
//! versions its references approved before they reach the catalog.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{CatalogDrift, CatalogSubject};
use kernel::{Address, AxError, B3Hash, McpReference, McpServer, ToolDef};

use crate::Reached;

use super::{ConnectorJob, Laying, held};

impl Laying {
    /// The external tools this run may reach, each connected and checked
    /// against its building's pins.
    ///
    /// A server that cannot be started or listed is left out without
    /// inventing removals. A listed tool whose pin is missing or different
    /// is withheld; a legacy reference adopts and persists its first list.
    pub(in crate::worker) fn mcp_tools(
        &self,
        job: ConnectorJob<'_>,
    ) -> Vec<agent_protocols::McpTool> {
        if job.confidential && !job.servers.is_empty() {
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
        // The half of `[prepare_dispatch]` this file owns: starting
        // every server this building declares and shaking hands with
        // each of them. Held apart from the whole-phase reading because
        // it is the part the resident connection table removes on every
        // dispatch after the first, and a figure that mixed the two could
        // not say how much.
        let began = (self.monotonic)();
        let mut offered = Vec::new();
        let resolve = crate::held_vault::resolving(std::sync::Arc::clone(&self.vault));
        for server in job.servers {
            // The module a reader is sent to is the transport that
            // failed, not whichever one was written first.
            let site = agent_protocols::McpLink::site(&server.transport);
            match self
                .connectors
                .connect(server, job.write_root, job.confidential, &resolve)
            {
                Ok((tools, reached)) => {
                    let listed = tools.len();
                    let reference = job
                        .references
                        .iter()
                        .find(|reference| reference.label == server.label);
                    offered.extend(self.select_listing(&job, server, reference, tools));
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
                            "{} is {how}, listed {listed} tool(s)",
                            server.label.as_str(),
                        ),
                    );
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
                job.servers.len(),
                offered.len()
            ),
        );
        offered
    }

    /// Applies one server's reference to a successful listing. The binary's
    /// desktop server has no configuration reference and is already governed
    /// by its own rule and allowlist.
    fn select_listing(
        &self,
        job: &ConnectorJob<'_>,
        server: &McpServer,
        reference: Option<&McpReference>,
        tools: Vec<agent_protocols::McpTool>,
    ) -> Vec<agent_protocols::McpTool> {
        let Some(reference) = reference else {
            return tools;
        };
        let definition = match kernel::config::definition_digest(server) {
            Ok(digest) => Some(digest),
            Err(err) => {
                self.refused("pin a connector definition", &err);
                None
            }
        };
        if let Some(pinned) = reference.definition {
            self.compare(
                job.building,
                CatalogSubject::Server {
                    label: server.label.clone(),
                },
                Some(pinned),
                definition,
            );
        }

        let legacy_tools = reference.tools.is_none();
        let mut found = BTreeMap::new();
        let mut names = BTreeSet::new();
        let mut offered = Vec::new();
        for tool in tools {
            match self.pin_tool(&tool) {
                Ok((name, digest)) => {
                    names.insert(name.clone());
                    found.insert(name.clone(), digest);
                    let pinned = reference
                        .tools
                        .as_ref()
                        .and_then(|pins| pins.get(&name))
                        .copied();
                    if legacy_tools || pinned == Some(digest) {
                        offered.push(tool);
                    } else {
                        self.compare(
                            job.building,
                            CatalogSubject::Tool {
                                label: server.label.clone(),
                                name,
                            },
                            pinned,
                            Some(digest),
                        );
                    }
                }
                Err(err) => self.refused("pin a connector tool", &err),
            }
        }
        if let Some(pinned) = &reference.tools {
            for (name, digest) in pinned {
                if !names.contains(name) {
                    self.compare(
                        job.building,
                        CatalogSubject::Tool {
                            label: server.label.clone(),
                            name: name.clone(),
                        },
                        Some(*digest),
                        None,
                    );
                }
            }
        }
        if reference.is_legacy() {
            let migrated = McpReference {
                label: reference.label.clone(),
                definition: definition.or(reference.definition),
                tools: Some(reference.tools.clone().unwrap_or(found)),
            };
            if let Err(err) = city::migrate_mcp_reference(&self.city_root, job.building, &migrated)
            {
                self.refused("migrate connector pins", &err);
            }
            if let Err(err) =
                city::migrate_session_mcp_reference(&self.city_root, job.session, &migrated)
            {
                self.refused("freeze the session's first connector pins", &err);
            }
        }
        offered
    }

    /// Stores the canonical definition and returns its catalog name and CAS
    /// address. The same bytes later answer the page's old/new comparison.
    fn pin_tool(&self, tool: &agent_protocols::McpTool) -> Result<(String, B3Hash), AxError> {
        let meta = kernel::Tool::meta(tool);
        let bytes = ToolDef {
            name: meta.name.clone(),
            description: meta.disclosure.clone(),
            input_schema: meta.params.clone(),
        }
        .pin_bytes()?;
        let digest = held(&self.store, "pin a connector tool")?
            .put(&bytes)
            .map_err(storage::StorageError::into_ax)?;
        Ok((meta.name.as_str().to_owned(), digest))
    }

    /// Collects one actual difference. A pin failure is diagnostic like a
    /// failed connection and never turns into an invented listing state.
    fn compare(
        &self,
        building: &Address,
        subject: CatalogSubject,
        pinned: Option<B3Hash>,
        found: Option<B3Hash>,
    ) {
        if let Some(drift) = CatalogDrift::between(subject, pinned, found)
            && let Err(err) = self.note_drift(building, drift)
        {
            self.refused("collect connector drift", &err);
        }
    }

    fn refused(&self, action: &str, err: &AxError) {
        self.note(
            runtime::diagnostics::Level::Refuse,
            "accounting::worker",
            &format!("{action}: {err}; {}", err.recovery()),
        );
    }
}
