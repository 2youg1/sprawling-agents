// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Reading Room: session-frozen names and pins, with the current
//! shelves read only to discover candidates and drift.

use kernel::event::record::{CatalogDrift, CatalogSubject};
use kernel::{Address, AxCode, AxError, B3Hash, SkillManifest};

use super::held;
use crate::worker::workbench::{Laying, Site};

impl Laying {
    /// Admits the versions this session pinned. A changed candidate does not
    /// replace them; only a legacy name without a pin adopts the candidate.
    pub(super) fn admit_reading_room(
        &self,
        catalog: &std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
        site: &Site,
        addr: &Address,
    ) -> Result<(), AxError> {
        let home = crate::home::Home::detect()?;
        let store = kernel::layout::CityLayout::new(&self.city_root).cas();
        let mut cas = storage::Cas::open(&store).map_err(storage::StorageError::into_ax)?;
        let shelves = {
            let read = |digest: &B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
            city::Library::scan(
                &self.city_root,
                Some(site.building.addr()),
                home.path(),
                &read,
            )?
        };
        let external = city::city_shelves(&self.city_root, home.path())?;
        for name in &site.reading_room {
            let candidate = shelves
                .reading_room(std::slice::from_ref(name))
                .into_iter()
                .next();
            let pinned = site.config.skill_pins.get(name).copied();
            let found = candidate.map(|holding| holding.hash);
            if let Some(drift) =
                CatalogDrift::between(CatalogSubject::Skill { name: name.clone() }, pinned, found)
            {
                self.note_drift(site.building.addr(), drift)?;
            }
            let Some(digest) = pinned.or(found) else {
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "city::library",
                    &format!(
                        "{} admits `{name}`, which is not on the shelves",
                        addr.as_str()
                    ),
                );
                continue;
            };
            if pinned.is_none()
                && let Some(city::Shelf::External { index, .. }) =
                    candidate.map(|holding| &holding.shelf)
            {
                let at = usize::try_from(*index).map_err(|_| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "admit an external skill",
                        format!("shelf index {index} does not fit this machine"),
                    )
                    .with_recovery("name fewer external shelves in the city's CONFIG.toml")
                })?;
                let root = external.get(at).ok_or_else(|| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "admit an external skill",
                        format!("shelf {index} is no longer configured"),
                    )
                    .with_recovery("restore the shelf entry, then start the run again")
                })?;
                let admitted = city::admit_external(&root.join(name), &mut |bytes| {
                    cas.put(bytes).map_err(storage::StorageError::into_ax)
                })?;
                if admitted.hash != digest {
                    return Err(AxError::failure(
                        AxCode::VersionConflict,
                        "admit an external skill",
                        format!("`{name}` changed between its scan and admission"),
                    )
                    .with_recovery("start the run again to scan and admit one version"));
                }
            }
            let document = skill_document(&cas, &digest, name)?;
            held(catalog, "admit the reading room")?.admit_skill(runtime::CatalogEntry {
                name: name.clone(),
                disclosure: city::skill_disclosure(&document),
                expansion: digest.to_string(),
                hash: Some(digest),
                document,
            })?;
            if pinned.is_none() {
                city::migrate_skill_pin(&self.city_root, site.building.addr(), name, digest)?;
            }
        }
        Ok(())
    }
}

/// Reads SKILL.md through the same verified manifest as the run's other
/// package files. The disclosure and full text therefore share one version.
fn skill_document(cas: &storage::Cas, digest: &B3Hash, name: &str) -> Result<String, AxError> {
    let manifest =
        SkillManifest::decode(&cas.get(digest).map_err(storage::StorageError::into_ax)?)?;
    let document = manifest.document().ok_or_else(|| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a skill from a shelf",
            format!("{name} has no SKILL.md in its admitted manifest"),
        )
        .with_recovery("reinstall the skill through the city's install door")
    })?;
    String::from_utf8(cas.get(document).map_err(storage::StorageError::into_ax)?).map_err(|_| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a skill from a shelf",
            format!("{name}'s SKILL.md is not UTF-8"),
        )
        .with_recovery("reinstall the skill as UTF-8 text through the install door")
    })
}
