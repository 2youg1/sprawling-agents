// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whose a content block is, answered from the ledger lines of one run
//! and its predecessor (runtime-SPEC section 8-29-5).
//!
//! The ledger is read when a `cas:` Locator is asked for and not before,
//! so a run that never reads a block never pays for the scan.

use std::path::Path;

use kernel::{Address, B3Hash, RunId};
use serde_json::Value;

/// The city's block store, and the owner lookup over this lineage.
pub(super) fn of_lineage(
    city_root: &Path,
    run: RunId,
    predecessor: Option<RunId>,
) -> runtime::Blocks {
    let layout = kernel::layout::CityLayout::new(city_root);
    let ledger = layout.ledger();
    let lineage: Vec<String> = std::iter::once(run)
        .chain(predecessor)
        .map(|id| id.to_string())
        .collect();
    runtime::Blocks {
        store: layout.cas(),
        owner: std::sync::Arc::new(move |hash: &B3Hash| {
            let needle = format!("cas:b3-{hash}");
            let lines = memory::read_raw_lines_at(&ledger).map_err(memory::MemoryError::into_ax)?;
            Ok(lines
                .iter()
                .find_map(|raw| referrer(raw, &lineage, &needle)))
        }),
    }
}

/// The address of one ledger line when it belongs to the lineage and
/// its payload spells the Locator; a line that does not parse or names
/// no address answers nothing, because it cannot say whose bytes these are.
fn referrer(raw: &[u8], lineage: &[String], needle: &str) -> Option<Address> {
    let line: Value = serde_json::from_slice(raw).ok()?;
    let run = line.get("run")?.as_str()?;
    if !lineage.iter().any(|id| id == run) {
        return None;
    }
    let spelled = line.get("data")?.to_string().contains(needle);
    spelled.then(|| Address::parse(line.get("addr")?.as_str()?).ok())?
}
