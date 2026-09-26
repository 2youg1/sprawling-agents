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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use kernel::{EventDraft, EventKind, Ledger, Payload, TimeMs};

    use super::*;

    fn line(run: RunId, addr: &str, spelled: &str) -> EventDraft {
        let mut data = serde_json::Map::new();
        data.insert("result".to_owned(), Value::String(spelled.to_owned()));
        EventDraft {
            run,
            t: TimeMs::new(1),
            who: "resident".to_owned(),
            addr: Some(Address::parse(addr).unwrap()),
            kind: EventKind::ToolCalled,
            data: Payload::new(data).unwrap(),
            ig: false,
        }
    }

    /// A block is whose the first line of this lineage that spelled it
    /// says, and a line of another run says nothing about this one.
    #[test]
    fn a_block_belongs_to_the_line_of_this_lineage_that_referenced_it() {
        let dir = tempfile::tempdir().unwrap();
        let (run, before, other) = (
            RunId::from_bytes([1; 16]),
            RunId::from_bytes([2; 16]),
            RunId::from_bytes([3; 16]),
        );
        let (mine, inherited, foreign) = (
            B3Hash::digest(b"mine"),
            B3Hash::digest(b"inherited"),
            B3Hash::digest(b"foreign"),
        );
        let ledger_dir = kernel::layout::CityLayout::new(dir.path()).ledger();
        let (mut ledger, _) = memory::JsonlLedger::open(&ledger_dir, TimeMs::new(1)).unwrap();
        ledger
            .append(line(other, "vault", &format!("cas:b3-{foreign}")))
            .unwrap();
        ledger
            .append(line(other, "vault", &format!("see cas:b3-{mine}")))
            .unwrap();
        ledger
            .append(line(run, "lab", &format!("see cas:b3-{mine}")))
            .unwrap();
        ledger
            .append(line(before, "lab/room", &format!("cas:b3-{inherited}")))
            .unwrap();
        drop(ledger);

        let blocks = of_lineage(dir.path(), run, Some(before));
        let owner_of = |hash: &B3Hash| (blocks.owner)(hash).unwrap().map(|a| a.as_str().to_owned());
        assert_eq!(
            [owner_of(&mine), owner_of(&inherited), owner_of(&foreign)],
            [Some("lab".to_owned()), Some("lab/room".to_owned()), None]
        );
    }
}
