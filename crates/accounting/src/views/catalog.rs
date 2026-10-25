// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How each tool a building admits reaches its runs, for the building
//! page's capabilities section (`crates/wire/spec/Answer/Catalog.lean`
//! §8-99, accounting D55).
//!
//! **Nothing here lays out a workbench.** The rows are the tool table the
//! building's newest run recorded in its `prompt_assembled`: assembling
//! one now would shake hands with every server the building names and
//! may start a browser, to answer a page. What each row is exposed as is
//! read against the `[catalog]` tables on disk now, through the same
//! `runtime::catalog::exposure` the next run's catalog calls, so the
//! page and the next run cannot disagree.

use kernel::event::record::{PromptAssembled, PromptTools};
use kernel::{Address, EventKind, EventRecord, RunId, Seq};

use super::holding::Views;
use super::prepared::{LedgerAsk, Prepared, unavailable_because};
use super::usage::ToolEvidence;

/// How many of a building's newest runs the evidence of use is counted
/// over (accounting D55).
const EVIDENCE_WINDOW: usize = 50;

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
            newest: self.building_prompts.get(building).copied(),
        })
    }
}

impl CatalogAsk {
    /// The building's tools, or `Unavailable` with the city's reason for
    /// an address that is not a building, a configuration that will not
    /// read, and a ledger that will not open.
    pub(super) fn answer(self) -> wire::Answer {
        match self.read() {
            Ok(answer) => wire::Answer::Catalog(Box::new(answer)),
            Err(stopped) => unavailable_because(format!("Catalog({})", self.building), &stopped),
        }
    }

    fn read(&self) -> Result<wire::CatalogAnswer, kernel::AxError> {
        let building = city::Building::of(&self.building)?;
        if building.addr() != &self.building {
            return Err(kernel::AxError::failure(
                kernel::AxCode::InvalidArgs,
                "read a building's catalog",
                self.building.as_str(),
            )
            .with_recovery("ask about the building, not one of its rooms"));
        }
        let stated = city::load_config(&self.ledger.city_root, building.addr())?.catalog;
        let frozen = self.newest_table()?;
        let evidence = match &frozen {
            Some(_) => self
                .ledger
                .usage()
                .map_err(storage::StorageError::into_ax)?
                .tool_evidence(building.addr(), EVIDENCE_WINDOW),
            None => ToolEvidence::default(),
        };
        let (run, table) = frozen.map_or((None, None), |(run, table)| (Some(run), table));
        let table = table.unwrap_or(PromptTools {
            hash: kernel::B3Hash::digest(b""),
            len: 0,
            index: 0,
            rows: Vec::new(),
        });
        let tools = table
            .rows
            .into_iter()
            .map(|row| {
                let (exposure, from) =
                    runtime::catalog::exposure::exposure_of(&stated, &row.name, &row.namespace);
                let used = evidence.by_tool.get(&row.name);
                wire::CatalogTool {
                    fallback: runtime::catalog::exposure::fallback(&row.name).0,
                    runs_used: used.map_or(0, |(runs, _)| *runs),
                    last_used: used.map(|(_, at)| *at),
                    name: row.name,
                    namespace: row.namespace,
                    hint: row.hint,
                    bytes: row.len,
                    exposure,
                    from,
                }
            })
            .collect();
        Ok(wire::CatalogAnswer {
            building: building.addr().clone(),
            run,
            core_bytes: table.len,
            index_bytes: table.index,
            runs_seen: evidence.runs_seen,
            tools,
            city: stated.city,
            stated: stated.building,
        })
    }

    /// The newest run's id and the tool table its first prompt recorded;
    /// `None` before the building's first run, and a `None` table for a
    /// run frozen before the table was recorded.
    fn newest_table(&self) -> Result<Option<(RunId, Option<PromptTools>)>, kernel::AxError> {
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
