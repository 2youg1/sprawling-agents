// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use kernel::event::EventKind;
use kernel::event::Scope;
use kernel::event::record::GoverningDocument;

use super::super::RunWorker;

impl RunWorker {

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

        let threshold = context_second_threshold
            .map(kernel::config::SecondThreshold::parse)
            .transpose()?;
        if let Some(references) = mcp_references {
            city::check_mcp_references(&self.city_root, building.addr(), references)?;
        }
        if let Some(names) = reading_room {
            city::check_reading_room(&self.city_root, building.addr(), names)?;
        }

        if let Some(pins) = skill_pins {
            admit_external_pins(&self.city_root, building.addr(), pins)?;
        }

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
    let holdings = shelves.all();
    for (name, pin) in pins {
        let current = holdings
            .iter()
            .find(|holding| holding.name == *name)
            .map(|holding| holding.hash);
        if current == Some(*pin) {
            continue;
        }
        held_pin_must_answer(&cas, name, pin)?;
    }
    for holding in holdings.into_iter().filter(|holding| {
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

fn held_pin_must_answer(
    cas: &storage::Cas,
    name: &str,
    pin: &kernel::B3Hash,
) -> Result<(), AxError> {
    match cas.get(pin) {
        Ok(_) => Ok(()),
        Err(storage::StorageError::CasMissing { .. }) => Err(AxError::failure(
            kernel::AxCode::VersionConflict,
            "save the building catalog",
            format!(
                "{name} changed after the page read it, and the pinned version was never admitted"
            ),
        )
        .with_recovery("open the catalog again and accept or keep the version it shows now")),
        Err(stopped) => Err(stopped.into_ax()),
    }
}

impl RunWorker {

    pub(in crate::worker) fn put_rules(&mut self, write: &wire::RulesWrite) -> Result<(), AxError> {
        let building = city::Building::of(&write.building)?;
        city::write_rules_against_with(
            &self.city_root,
            building.addr(),
            city::RulesEdit {
                base: &write.base,
                text: &write.body,
            },
            |rules| self.validate_workspace_models(rules),
        )?;
        self.book_document(
            Scope::Building(building.addr().clone()),
            GoverningDocument::Rules,
            &city::rules_path(&self.city_root, building.addr()),
        )
    }

    pub(in crate::worker) fn set_workspace_choice(
        &mut self,
        write: &wire::WorkspaceChoiceWrite,
    ) -> Result<(), AxError> {
        let building = city::Building::of(&write.building)?;
        let body = city::inscribe_choice(&write.base, &write.endpoint, &write.model, write.effort);
        city::write_rules_against_with(
            &self.city_root,
            building.addr(),
            city::RulesEdit {
                base: &write.base,
                text: &body,
            },
            |rules| self.validate_workspace_models(rules),
        )?;
        self.book_document(
            Scope::Building(building.addr().clone()),
            GoverningDocument::Rules,
            &city::rules_path(&self.city_root, building.addr()),
        )
    }

    pub(in crate::worker) fn configure_city(
        &mut self,
        settings: wire::CitySettings,
    ) -> Result<(), AxError> {
        if let Some(table) = &settings.catalog {
            city::check_catalog(table)?;
        }
        if let Some(shelves) = settings.skill_shelves.as_deref() {
            city::check_skill_shelves(shelves, crate::home::Home::detect()?.path())?;
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
            .chain(settings.effort.map(city::CitySetting::Effort))
            .chain(
                settings
                    .skill_shelves
                    .as_deref()
                    .map(city::CitySetting::SkillShelves),
            );
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
