// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The building's unified catalog answer (`crates/wire/spec/Answer/Catalog.lean`
//! §8-99, accounting D55, D112). It joins the newest frozen tool table,
//! current selections, Ledger drift and usage, and verified CAS versions.
//! Nothing here assembles a workbench or connects a server.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{CatalogDrift, CatalogSubject, PromptAssembled, PromptTools};
use kernel::{Address, AxCode, AxError, B3Hash, EventKind, EventRecord, RunId, Seq};

use super::holding::Views;
use super::prepared::{LedgerAsk, Prepared, unavailable_because};
use super::usage::{CatalogEvidence, Usage};

/// What a `Catalog` question takes out of the views: the building, and
/// where its newest run's first prompt sits on the ledger. The line, the
/// configuration and the evidence are read once the views are released.
pub struct CatalogAsk {
    ledger: LedgerAsk,
    building: Address,
    newest: Option<Seq>,
}

impl Views {
    pub(super) fn catalog_ask(&self, building: &Address) -> Prepared {
        Prepared::Catalog(CatalogAsk {
            ledger: self.ledger_ask(),
            building: building.clone(),
            newest: self.building_prompts.get(building).map(|held| held.prompt),
        })
    }
}

impl CatalogAsk {
    /// The building's catalog, or `Unavailable` with the city's reason for
    /// an address that is not a building, unreadable configuration, CAS or
    /// ledger. No external server is contacted.
    pub(super) fn answer(self) -> wire::Answer {
        match self.read() {
            Ok(answer) => wire::Answer::Catalog(Box::new(answer)),
            Err(stopped) => unavailable_because(format!("Catalog({})", self.building), &stopped),
        }
    }

    fn read(&self) -> Result<wire::CatalogAnswer, AxError> {
        let building = city::Building::of(&self.building)?;
        if building.addr() != &self.building {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a building's catalog",
                self.building.as_str(),
            )
            .with_recovery("ask about the building, not one of its rooms"));
        }
        let config = city::load_config(&self.ledger.city_root, building.addr())?;
        let rules = city::load(&self.ledger.city_root, building.addr())?;
        let current = city::SessionCatalog::current(&rules, &config);
        let open_sessions =
            city::previous_sessions(&self.ledger.city_root, building.addr(), &current)?;
        let frozen = self.newest_table()?;
        let usage = self
            .ledger
            .usage()
            .map_err(storage::StorageError::into_ax)?;
        let evidence = usage.catalog_evidence(building.addr());
        let drifts = usage.catalog_drifts(building.addr());
        let cas =
            storage::Cas::open(&kernel::layout::CityLayout::new(&self.ledger.city_root).cas())
                .map_err(storage::StorageError::into_ax)?;
        let (run, table) = frozen.map_or((None, None), |(run, table)| (Some(run), table));
        let table = table.unwrap_or(PromptTools {
            hash: B3Hash::digest(b""),
            len: 0,
            index: 0,
            reading: 0,
            always: 0,
            dormant: 0,
            skills: 0,
            rows: Vec::new(),
        });
        let tools = tool_rows(
            &table,
            ToolRead {
                stated: &config.catalog,
                references: &config.mcp_references,
                evidence: &evidence,
                drifts: &drifts,
                cas: &cas,
                newest: self.newest,
            },
        )?;
        let home = crate::home::Home::detect()?;
        let read = |digest: &B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
        let shelves = city::Library::scan(
            &self.ledger.city_root,
            Some(building.addr()),
            home.path(),
            &read,
        )?;
        let skill_rows = skill_rows(SkillRead {
            shelves: &shelves,
            reading_room: rules.reading_room(),
            pins: &config.skill_pins,
            stated: &config.catalog,
            evidence: &evidence,
            usage: &usage,
            cas: &cas,
        })?;
        let changed = tools
            .iter()
            .filter(|row| row.change.is_some())
            .count()
            .saturating_add(skill_rows.iter().filter(|row| row.change.is_some()).count());
        Ok(wire::CatalogAnswer {
            building: building.addr().clone(),
            run,
            core_bytes: table.len,
            index_bytes: table.index,
            reading_bytes: table.reading,
            always: table.always,
            dormant: table.dormant,
            skills: table.skills,
            changed: counted_u32(changed, "changed catalog rows")?,
            open_sessions,
            runs_seen: evidence.runs_seen,
            tools,
            skill_rows,
            references: config.mcp_references,
            skill_pins: config.skill_pins,
            reading_room: rules.reading_room().to_vec(),
            city: config.catalog.city,
            stated: config.catalog.building,
        })
    }

    /// The newest run's id and the tool table its first prompt recorded;
    /// `None` before the building's first run, and a `None` table for a
    /// run frozen before the table was recorded.
    fn newest_table(&self) -> Result<Option<(RunId, Option<PromptTools>)>, AxError> {
        let Some(seq) = self.newest else {
            return Ok(None);
        };
        let line = {
            let (index, dir) = self
                .ledger
                .indexed()
                .map_err(storage::StorageError::into_ax)?;
            index
                .reader(&dir)
                .line_at(seq)
                .map_err(storage::StorageError::into_ax)?
        };
        let record = EventRecord::parse_line(&line)?;
        if record.kind() != EventKind::PromptAssembled {
            return Ok(None);
        }
        Ok(Some((
            record.run(),
            record.data().read::<PromptAssembled>()?.tools,
        )))
    }
}

/// Everything the tool projection reads besides the frozen table.
struct ToolRead<'a> {
    stated: &'a kernel::config::CatalogExposure,
    references: &'a [kernel::McpReference],
    evidence: &'a CatalogEvidence,
    drifts: &'a [(Seq, CatalogDrift)],
    cas: &'a storage::Cas,
    newest: Option<Seq>,
}

/// The last observed version of one tool. `found: None` is a removal;
/// `at` lets a newer frozen table supersede an older drift line.
struct Observed {
    namespace: String,
    at: Option<Seq>,
    found: Option<B3Hash>,
}

fn tool_rows(table: &PromptTools, read: ToolRead<'_>) -> Result<Vec<wire::CatalogTool>, AxError> {
    let mut observed = BTreeMap::new();
    for row in &table.rows {
        if let Some(digest) = row.digest {
            observed.insert(
                row.name.clone(),
                Observed {
                    namespace: row.namespace.clone(),
                    at: read.newest,
                    found: Some(digest),
                },
            );
        }
    }
    for (seq, drift) in read.drifts {
        let CatalogSubject::Tool { label, name } = &drift.subject else {
            continue;
        };
        let replace = observed
            .get(name)
            .is_none_or(|held: &Observed| held.at.is_none_or(|at| *seq > at));
        if replace {
            observed.insert(
                name.clone(),
                Observed {
                    namespace: label.as_str().to_owned(),
                    at: Some(*seq),
                    found: drift.found,
                },
            );
        }
    }
    observed
        .into_iter()
        .map(|(name, observed)| tool_row(name, observed, &read))
        .collect()
}

fn tool_row(
    name: String,
    observed: Observed,
    read: &ToolRead<'_>,
) -> Result<wire::CatalogTool, AxError> {
    let reference = read
        .references
        .iter()
        .find(|reference| reference.label.as_str() == observed.namespace);
    let pinned = reference.and_then(|reference| reference.tools.as_ref()?.get(&name).copied());
    let change = reference.and_then(|_| change_between(pinned, observed.found));
    let current = observed
        .found
        .map(|digest| tool_version(read.cas, &digest))
        .transpose()?;
    let pinned = change
        .and(pinned)
        .map(|digest| tool_version(read.cas, &digest))
        .transpose()?;
    let (exposure, from) =
        runtime::catalog::exposure::exposure_of(read.stated, &name, &observed.namespace);
    let used = read.evidence.by_tool.get(&name);
    Ok(wire::CatalogTool {
        fallback: runtime::catalog::exposure::fallback(&name).0,
        runs_used: used.map_or(0, |(runs, _)| *runs),
        last_used: used.map(|(_, at)| *at),
        name,
        namespace: observed.namespace,
        current,
        pinned,
        change,
        exposure,
        from,
    })
}

/// Reads a canonical definition by its CAS address. The full read verifies
/// the digest before any description or schema reaches the answer.
fn tool_version(cas: &storage::Cas, digest: &B3Hash) -> Result<wire::CatalogVersion, AxError> {
    let bytes = cas.get(digest).map_err(storage::StorageError::into_ax)?;
    let definition: kernel::ToolDef = serde_json::from_slice(&bytes).map_err(|err| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a catalog tool definition",
            format!("{digest}: {err}"),
        )
        .with_recovery("run the building again to store a canonical definition")
    })?;
    Ok(wire::CatalogVersion {
        digest: *digest,
        hint: runtime::catalog::hint(&definition.description).to_owned(),
        bytes: counted_bytes(bytes.len(), "tool definition bytes")?,
        description: definition.description,
        input_schema: definition.input_schema,
    })
}

/// Everything the skill projection reads. External candidates carry their
/// scanned SKILL.md; pinned and internal versions come through full CAS reads.
struct SkillRead<'a> {
    shelves: &'a city::Library,
    reading_room: &'a [String],
    pins: &'a BTreeMap<String, B3Hash>,
    stated: &'a kernel::config::CatalogExposure,
    evidence: &'a CatalogEvidence,
    usage: &'a Usage,
    cas: &'a storage::Cas,
}

fn skill_rows(read: SkillRead<'_>) -> Result<Vec<wire::CatalogSkill>, AxError> {
    let holdings: BTreeMap<String, &city::Holding> = read
        .shelves
        .all()
        .into_iter()
        .map(|holding| (holding.name.clone(), holding))
        .collect();
    let names: BTreeSet<String> = holdings
        .keys()
        .cloned()
        .chain(read.pins.keys().cloned())
        .chain(read.reading_room.iter().cloned())
        .collect();
    names
        .into_iter()
        .map(|name| {
            let holding = holdings.get(&name).copied();
            skill_row(name, holding, &read)
        })
        .collect()
}

fn skill_row(
    name: String,
    holding: Option<&city::Holding>,
    read: &SkillRead<'_>,
) -> Result<wire::CatalogSkill, AxError> {
    let candidate = holding.map(|holding| holding.hash);
    let pinned = read.pins.get(&name).copied();
    let admitted = read.reading_room.iter().any(|named| named == &name);
    let change = if admitted || pinned.is_some() {
        change_between(pinned, candidate)
    } else {
        None
    };
    let current_document = holding
        .map(|holding| {
            holding
                .carried
                .clone()
                .map_or_else(|| skill_document(read.cas, &holding.hash, &name), Ok)
        })
        .transpose()?;
    let pinned_document = change
        .and(pinned)
        .map(|digest| skill_document(read.cas, &digest, &name))
        .transpose()?;
    let disclosure = holding.map_or_else(
        || {
            pinned_document
                .as_deref()
                .map(city::skill_disclosure)
                .unwrap_or_default()
        },
        |holding| holding.disclosure.clone(),
    );
    let (mut exposure, mut from) = read.stated.stated_skill(&name).unwrap_or((
        kernel::config::Exposure::Dormant,
        kernel::config::ExposureFrom::Default,
    ));
    if !admitted {
        exposure = kernel::config::Exposure::Off;
        from = kernel::config::ExposureFrom::Default;
    }
    let (audit, audit_stale) = candidate.map_or((None, false), |digest| {
        read.usage.skill_audit(&name, &digest)
    });
    let used = read.evidence.by_skill.get(&name);
    Ok(wire::CatalogSkill {
        section: holding.map_or_else(String::new, |holding| holding.section.clone()),
        disclosure,
        current_document,
        pinned_document,
        candidate,
        pinned,
        change,
        exposure,
        from,
        audit,
        audit_stale,
        runs_used: used.map_or(0, |(runs, _)| *runs),
        last_used: used.map(|(_, at)| *at),
        name,
    })
}

fn skill_document(cas: &storage::Cas, digest: &B3Hash, name: &str) -> Result<String, AxError> {
    city::skill_document(
        &|digest| cas.get(digest).map_err(storage::StorageError::into_ax),
        digest,
        name,
    )
}

fn change_between(pinned: Option<B3Hash>, found: Option<B3Hash>) -> Option<wire::CatalogChange> {
    match (pinned, found) {
        (None, Some(_)) => Some(wire::CatalogChange::Added),
        (Some(_), None) => Some(wire::CatalogChange::Removed),
        (Some(pinned), Some(found)) if pinned != found => Some(wire::CatalogChange::Changed),
        _ => None,
    }
}

fn counted_u32(len: usize, subject: &str) -> Result<u32, AxError> {
    u32::try_from(len).map_err(|_| {
        AxError::failure(AxCode::InvalidArgs, "count a catalog answer", subject)
            .with_recovery("ask for fewer catalog rows")
    })
}

fn counted_bytes(len: usize, subject: &str) -> Result<u64, AxError> {
    u64::try_from(len).map_err(|_| {
        AxError::failure(AxCode::InvalidArgs, "measure a catalog answer", subject)
            .with_recovery("store a smaller definition")
    })
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
