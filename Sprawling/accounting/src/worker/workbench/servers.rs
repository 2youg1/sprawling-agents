// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{CatalogDrift, CatalogSubject};
use kernel::{Address, AxError, B3Hash, McpReference, McpServer, ToolDef};

use crate::Reached;

use super::{ConnectorJob, Laying, held};

impl Laying {
    pub(in crate::worker) fn mcp_tools(
        &self,
        job: ConnectorJob<'_>,
    ) -> Vec<agent_protocols::McpTool> {
        if job.confidential && !job.servers.is_empty() {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "accounting::worker",
                "this building is confidential; no external server is started for it",
            );
            return Vec::new();
        }

        let began = (self.monotonic)();
        let mut offered = Vec::new();
        let resolve = crate::held_vault::resolving(std::sync::Arc::clone(&self.vault));
        for server in job.servers {
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
