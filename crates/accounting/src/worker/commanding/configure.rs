// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one reconfiguration of a building's own layer writes, and what
//! the settings page writes into the city's own.
//!
//! A building's faces travel together: one command speaks for all of
//! them, as one `wire::BuildingSettings`, and they are recorded as one
//! event.

use kernel::AxError;
use kernel::event::EventKind;
use kernel::event::Scope;
use kernel::event::record::GoverningDocument;

use super::super::RunWorker;

impl RunWorker {
    /// Writes what a building's runs may reach into that building's own
    /// configuration layer. `None` is "the page said nothing about that
    /// face", which is not the same as an empty statement: `mcp: Some(vec![])`
    /// writes "this building reaches no server", while `None` leaves that
    /// face as it was.
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
        settings: &wire::BuildingSettings,
    ) -> Result<(), AxError> {
        let wire::BuildingSettings {
            addr,
            sandbox,
            mcp,
            desktop,
            context_second_threshold,
            catalog,
            mcp_references,
            skill_pins,
            reading_room,
            idem: _,
        } = settings;
        let building = city::Building::of(addr)?;
        // Taken before anything is written: the domain is
        // `SecondThreshold`'s one construction point, so an out-of-domain
        // percent refuses the frame here rather than after the faces
        // beside it landed, or at the next read.
        let threshold = context_second_threshold
            .map(kernel::config::SecondThreshold::parse)
            .transpose()?;
        if let Some(references) = mcp_references {
            city::check_mcp_references(&self.city_root, building.addr(), references)?;
        }
        if let Some(names) = reading_room {
            city::check_reading_room(&self.city_root, building.addr(), names)?;
        }
        // Accepting an external candidate copies that exact scanned version
        // before its pin moves. Keeping an old pin never admits the candidate.
        if let Some(pins) = skill_pins {
            admit_external_pins(&self.city_root, building.addr(), pins)?;
        }
        // Judged before any other face is written, so a table the city
        // refuses leaves the building as it was.
        if let Some(table) = catalog {
            city::write_catalog(&self.city_root, building.addr(), table)?;
        }
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
        if let Some(references) = mcp_references {
            city::write_mcp_references(&self.city_root, building.addr(), references)?;
        }
        if let Some(pins) = skill_pins {
            city::write_skill_pins(&self.city_root, building.addr(), pins)?;
        }
        if let Some(names) = reading_room {
            city::write_reading_room(&self.city_root, building.addr(), names)?;
        }
        if let Some(threshold) = threshold {
            city::write_second_threshold(
                &self.city_root,
                building.addr(),
                city::Layer::Building,
                threshold,
            )?;
        }
        // Written whole and never parsed here: the connector that reads
        // it at start-up is the authority on its syntax and fails closed,
        // so a second reading on this side would be a second authority
        // (`crates/city/Spec.lean` §8-26).
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
                mcp: mcp.is_some() || mcp_references.is_some(),
                desktop: desktop.is_some(),
                context: threshold.is_some(),
                catalog: catalog.is_some()
                    || mcp_references.is_some()
                    || skill_pins.is_some()
                    || reading_room.is_some(),
            },
        )?;
        self.record(EventKind::BuildingConfigured, payload)?;
        if reading_room.is_some() {
            self.book_document(
                Scope::Building(building.addr().clone()),
                GoverningDocument::Rules,
                &city::rules_path(&self.city_root, building.addr()),
            )?;
        }
        Ok(())
    }
}

/// Copies only external candidates whose current digest the page accepted.
/// Internal candidates are already in CAS; old pins kept across a candidate
/// change do not copy or approve the new content.
fn admit_external_pins(
    city_root: &std::path::Path,
    building: &kernel::Address,
    pins: &std::collections::BTreeMap<String, kernel::B3Hash>,
) -> Result<(), AxError> {
    if pins.is_empty() {
        return Ok(());
    }
    let home = crate::home::Home::detect()?;
    let store = kernel::layout::CityLayout::new(city_root).cas();
    let mut cas = storage::Cas::open(&store).map_err(storage::StorageError::into_ax)?;
    let read = |digest: &kernel::B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
    let shelves = city::Library::scan(city_root, Some(building), home.path(), &read)?;
    let external = city::city_shelves(city_root, home.path())?;
    for holding in shelves.all().into_iter().filter(|holding| {
        holding.shelf.is_external() && pins.get(&holding.name) == Some(&holding.hash)
    }) {
        let city::Shelf::External { index, .. } = holding.shelf else {
            continue;
        };
        let at = usize::try_from(index).map_err(|_| {
            AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "accept an external skill",
                format!("{}: shelf index {index} does not fit", holding.name),
            )
            .with_recovery("name fewer external shelves, then save again")
        })?;
        let root = external.get(at).ok_or_else(|| {
            AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "accept an external skill",
                format!("{}: shelf {index} is not configured", holding.name),
            )
            .with_recovery("restore the shelf, then save the catalog again")
        })?;
        let admitted = city::admit_external(&root.join(&holding.name), &mut |bytes| {
            cas.put(bytes).map_err(storage::StorageError::into_ax)
        })?;
        if admitted.hash != holding.hash {
            return Err(AxError::failure(
                kernel::AxCode::VersionConflict,
                "accept an external skill",
                format!("{} changed between its scan and admission", holding.name),
            )
            .with_recovery("save the catalog again to accept one scanned version"));
        }
    }
    Ok(())
}

impl RunWorker {
    /// Writes a building's `RULES.toml` whole from a page, evaluated
    /// first and only over the text the page started from, and books the
    /// change (`crates/wire/Spec.lean` §8-60).
    ///
    /// # Errors
    /// Propagates the evaluation's refusal, a file that is no longer
    /// `base`, a file that cannot be written or read back, and a history
    /// that will not take the line.
    pub(in crate::worker) fn put_rules(&mut self, write: &wire::RulesWrite) -> Result<(), AxError> {
        let building = city::Building::of(&write.building)?;
        city::write_rules_against(&self.city_root, building.addr(), &write.base, &write.body)?;
        self.book_document(
            Scope::Building(building.addr().clone()),
            GoverningDocument::Rules,
            &city::rules_path(&self.city_root, building.addr()),
        )
    }

    /// Writes the city's own layer from the settings page, one fact at a
    /// time, and books the change once (`crates/wire/spec/Command/Step.lean`
    /// §8-61). A frame that states nothing writes nothing and books
    /// nothing.
    ///
    /// `[search]` and `[catalog]` are the facts the city refuses for what
    /// they say, so both are judged before the first byte lands: the
    /// catalog by `city::check_catalog`, the search by its own writer,
    /// which judges before it writes. A refused frame leaves the file as
    /// it was.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for a `[search]` value or a `[catalog]` key the
    /// reader would refuse;
    /// propagates a city layer this build cannot read or write, and a
    /// history that will not take the line.
    pub(in crate::worker) fn configure_city(
        &mut self,
        settings: wire::CitySettings,
    ) -> Result<(), AxError> {
        if let Some(table) = &settings.catalog {
            city::check_catalog(table)?;
        }
        let search = settings
            .search
            .map(|search| city::write_search(&self.city_root, &search));
        let others = settings
            .catalog
            .as_ref()
            .map(city::CitySetting::Catalog)
            .into_iter()
            .chain(settings.keep_warm.map(city::CitySetting::KeepWarm))
            .chain(settings.effort.map(city::CitySetting::Effort));
        let mut written = search.transpose()?.is_some();
        for setting in others {
            city::write_city_setting(&self.city_root, setting)?;
            written = true;
        }
        if !written {
            return Ok(());
        }
        self.book_document(
            Scope::City,
            GoverningDocument::Config,
            &kernel::layout::CityLayout::new(&self.city_root).city_config(),
        )
    }
}
