// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one reconfiguration of a building's own layer writes.
//!
//! The faces travel together: one command speaks for all of them, and a
//! call site spelling four bare booleans says nothing about which face is
//! which. They arrive as one value and are recorded as one event.

use kernel::event::EventKind;
use kernel::{Address, AxError};

use super::super::RunWorker;

/// The faces of a building's own layer one reconfiguration speaks for.
///
/// `None` is "this caller said nothing about that face", which is not the
/// same as an empty statement: `mcp: Some(&[])` writes "this building
/// reaches no server", while `None` leaves that face as it was.
#[derive(Debug, Clone, Copy)]
pub(super) struct Reconfiguration<'a> {
    pub(super) sandbox: Option<&'a kernel::SandboxLimits>,
    pub(super) mcp: Option<&'a [kernel::McpServer]>,
    pub(super) desktop: Option<&'a str>,
    /// A whole percent of the window. The domain is
    /// `kernel::config::SecondThreshold`'s one construction point, so an
    /// out-of-domain value is refused there rather than here.
    pub(super) context_second_threshold: Option<u64>,
}

impl RunWorker {
    /// Writes what a building's runs may reach into that building's own
    /// configuration layer.
    ///
    /// **The contents are not recorded; the change is.** `CONFIG.toml`
    /// is the authority for what a run is governed by, and an event
    /// carrying the same fact would be a second one - that part of the
    /// original ruling stands. What it missed is that "somebody
    /// reconfigured this building" is not in `CONFIG.toml` at all: it is
    /// an effect the city carried out, and the Ledger is the only
    /// history of those. For a whole stage this wrote a diagnostic line
    /// and nothing else, so a page had no way to learn that a building
    /// it was showing had moved, and the MCP screen guessed with a timer
    /// three hundred milliseconds after it sent the command.
    ///
    /// # Errors
    /// Propagates a configuration this build cannot read or write, and
    /// the refusal of a second rung outside its domain.
    pub(super) fn configure_building(
        &mut self,
        addr: &Address,
        reconfigured: Reconfiguration<'_>,
    ) -> Result<(), AxError> {
        let Reconfiguration {
            sandbox,
            mcp,
            desktop,
            context_second_threshold,
        } = reconfigured;
        let building = city::Building::of(addr)?;
        if let Some(limits) = sandbox {
            city::write_sandbox(
                &self.city_root,
                building.addr(),
                city::Layer::Building,
                limits,
            )?;
        }
        if let Some(servers) = mcp {
            city::write_mcp(
                &self.city_root,
                building.addr(),
                city::Layer::Building,
                servers,
            )?;
        }
        // Taken before written: the domain is `SecondThreshold`'s one
        // construction point, so an out-of-domain percent is refused
        // here rather than written and refused at the next read.
        let mut context = false;
        if let Some(percent) = context_second_threshold {
            let threshold = kernel::config::SecondThreshold::parse(percent)?;
            city::write_second_threshold(
                &self.city_root,
                building.addr(),
                city::Layer::Building,
                threshold,
            )?;
            context = true;
        }
        // Written whole and never parsed here: the connector that reads
        // it at start-up is the authority on its syntax and fails closed,
        // so a second reading on this side would be a second authority
        // (city-SPEC.md 8-26).
        if let Some(allowlist) = desktop {
            city::write_desktop_scope(&self.city_root, building.addr(), allowlist)?;
        }
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::config_layers",
            &format!("{} was reconfigured", building.addr().as_str()),
        );
        let payload = city::building_configured_payload(
            &building,
            city::Written {
                sandbox: sandbox.is_some(),
                mcp: mcp.is_some(),
                desktop: desktop.is_some(),
                context,
            },
        )?;
        self.record(EventKind::BuildingConfigured, payload)
    }
}
