// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The built-in tools a building's catalog page lists before any run has
//! frozen a table (`crates/accounting/spec/Views/Usage.lean` D55).
//!
//! Each row's definition is that tool's `definition`, the same value its
//! constructor writes into `ToolMeta`. Connectors, browsers, and the
//! tools that exist only after an endpoint-book selection stay out: listing
//! them would open a server or a client this question does not open.

use kernel::config::{CatalogExposure, Exposure};
use kernel::{AxCode, AxError, B3Hash, ToolDef};

use super::super::usage::CatalogEvidence;
use super::counted_bytes;

/// The tools this building's first run admits without leaving the process,
/// in name order, at the exposure the tables on disk give them now.
pub(super) fn rows(
    config: &kernel::FrozenConfig,
    rules: &city::BuildingRules,
    evidence: &CatalogEvidence,
) -> Result<Vec<wire::CatalogTool>, AxError> {
    let search = if rules.policy().confidential {
        None
    } else {
        offered_search(&config.search)
    };
    let mut tools = builtin(city::vocation_of(rules.addr()), config.sandbox.shell)?;
    tools.extend(crate::worker::catalog_builtins(search.as_ref())?);
    let mut rows = Vec::with_capacity(tools.len());
    for (namespace, defined) in tools {
        rows.push(row(namespace, defined, &config.catalog, evidence)?);
    }
    rows.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(rows)
}

/// `always`, `dormant` and the sum of the core rows' definition bytes.
/// The dormant index and the core array's own encoding are what a run
/// records; neither exists until one does.
pub(super) fn tier_counts(tools: &[wire::CatalogTool]) -> Result<(u32, u32, u64), AxError> {
    let mut always = 0u32;
    let mut dormant = 0u32;
    let mut core_bytes = 0u64;
    for tool in tools {
        match tool.exposure {
            Exposure::Core => {
                always = always
                    .checked_add(1)
                    .ok_or_else(|| overflow("core tools"))?;
                let bytes = tool.current.as_ref().map_or(0, |version| version.bytes);
                core_bytes = core_bytes
                    .checked_add(bytes)
                    .ok_or_else(|| overflow("core definition bytes"))?;
            }
            Exposure::Dormant => {
                dormant = dormant
                    .checked_add(1)
                    .ok_or_else(|| overflow("dormant tools"))?;
            }
            Exposure::Off => {}
        }
    }
    Ok((always, dormant, core_bytes))
}

type Stood = (&'static str, ToolDef);

fn builtin(vocation: city::Vocation, shell: bool) -> Result<Vec<Stood>, AxError> {
    let sprawling = kernel::ReservedNamespace::Sprawling.as_str();
    let mut tools = vec![
        (sprawling, collab::ArchiveTool::definition()?),
        (sprawling, collab::ClaimTool::definition()?),
        (sprawling, runtime::EditTool::definition()?),
        (sprawling, runtime::StatusTool::definition()?),
        (sprawling, collab::SignalTool::definition()?),
        (sprawling, collab::GoalTool::definition()?),
        (sprawling, collab::PrTool::definition()?),
        (sprawling, city::RulesTool::definition()?),
        (sprawling, city::NeighboursTool::definition()?),
        (sprawling, runtime::ReadTool::definition()?),
        (sprawling, runtime::SearchTool::definition()?),
        (sprawling, runtime::SucceedTool::definition()?),
        (sprawling, runtime::ToolTool::definition()?),
    ];
    match vocation {
        city::Vocation::Builds => {
            tools.push((sprawling, runtime::ExecTool::standing_definition(shell)?));
            tools.push((sprawling, collab::DelegateTool::definition()?));
            tools.push((sprawling, collab::WorkshopTool::definition()?));
        }
        city::Vocation::Plans => {
            tools.push((sprawling, city::CityTool::definition()?));
        }
    }
    Ok(tools)
}

fn row(
    namespace: &str,
    defined: ToolDef,
    stated: &CatalogExposure,
    evidence: &CatalogEvidence,
) -> Result<wire::CatalogTool, AxError> {
    let name = defined.name.as_str().to_owned();
    let bytes = defined.pin_bytes()?;
    let digest = B3Hash::digest(&bytes);
    let hint = runtime::catalog::hint(&defined.description).to_owned();
    let len = counted_bytes(bytes.len(), "standing tool definition")?;
    let (exposure, from) = runtime::catalog::exposure::exposure_of(stated, &name, namespace);
    let used = evidence.by_tool.get(&name);
    Ok(wire::CatalogTool {
        fallback: runtime::catalog::exposure::fallback(&name).0,
        runs_used: used.map_or(0, |(runs, _)| *runs),
        last_used: used.map(|(_, at)| *at),
        current: Some(wire::CatalogVersion {
            digest,
            hint,
            bytes: len,
            description: defined.description,
            input_schema: defined.input_schema,
        }),
        pinned: None,
        change: None,
        exposure,
        from,
        name,
        namespace: namespace.to_owned(),
    })
}

/// A supplier the file cannot name is the same omission a run makes: it
/// goes on without `web_search`. The page has no field for that refusal.
#[expect(
    clippy::manual_unwrap_or_default,
    reason = "the Err arm is the run's omission, not Option's default"
)]
fn offered_search(
    configuration: &kernel::config::SearchConfiguration,
) -> Option<kernel::config::SearchSupplier> {
    match city::search_supplier(configuration) {
        Ok(supplier) => supplier,
        Err(_unnamed) => None,
    }
}

fn overflow(subject: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "count standing catalog rows", subject)
        .with_recovery("ask for fewer catalog rows")
}
