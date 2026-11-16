// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, B3Hash, Effort, EventDraft, EventRecord, Ledger as _, RunId, TimeMs};
use storage::{Checkpoint, ModelChoice, Provenance, WorktreeName, Worktrees};

use super::reading::{Load, Reading, SubMetric, Taken};

pub(crate) const PINNED_DRAFTS: u64 = 1_000;

pub(crate) struct Fixture {

    pub(crate) fold_records: u64,
    pub(crate) fold_rounds: u32,

    pub(crate) tree_files: u32,
    pub(crate) tree_file_bytes: u32,

    pub(crate) placements: u32,
    pub(crate) forward_events: u32,

    pub(crate) pinned: &'static str,
}

impl Fixture {

    pub(crate) fn digest(&self, scratch: &Path) -> Result<B3Hash, String> {
        let dir = scratch.join("fixture-digest");
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => return Err(format!("clear {}: {why}", dir.display())),
        }
        std::fs::create_dir_all(&dir).map_err(|why| format!("{}: {why}", dir.display()))?;
        let (mut ledger, _report) =
            storage::JsonlLedger::open(&dir, TimeMs::new(1_700_000_000_000))
                .map_err(|why| format!("{}", why.into_ax()))?;
        let drafts: Vec<EventDraft> = (0..PINNED_DRAFTS)
            .map(super::draft)
            .collect::<Result<_, _>>()?;
        ledger.append_all(drafts).map_err(|why| format!("{why}"))?;
        drop(ledger);
        let written = citysim::ledger_digest(&dir).map_err(|why| format!("{why}"))?;
        let mut bytes = written.as_bytes().to_vec();
        bytes.extend_from_slice(&self.fold_records.to_le_bytes());
        bytes.extend_from_slice(&self.fold_rounds.to_le_bytes());
        bytes.extend_from_slice(&self.tree_files.to_le_bytes());
        bytes.extend_from_slice(&self.tree_file_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.placements.to_le_bytes());
        bytes.extend_from_slice(&self.forward_events.to_le_bytes());
        Ok(B3Hash::digest(&bytes))
    }
}

pub(crate) const REGISTERED: Fixture = Fixture {
    fold_records: 50_000,
    fold_rounds: 5,
    tree_files: 512,
    tree_file_bytes: 16_384,
    placements: 4,
    forward_events: 2_000,
    pinned: "1dc560ad5766de65f16ac021f4a5b9cf2dd089cfae4cfc1378cb142e05b6ed28",
};

pub(crate) fn all(scratch: &Path, fixture: &Fixture, taken: Taken) -> Result<Vec<Reading>, String> {
    let mut readings = Vec::new();
    for load in Load::ALL {
        let of_load = match load {
            Load::LargeLedgerFold => large_ledger_fold(scratch, fixture, taken)?,
            Load::LargeWorktreePlacement => large_worktree_placement(scratch, fixture, taken)?,
            Load::KeptWorktreeReclaim => kept_worktree_reclaim(scratch, fixture, taken)?,
            Load::LongSessionForwarding => long_session_forwarding(fixture, taken)?,
        };
        readings.extend(of_load);
    }
    Ok(readings)
}

fn large_ledger_fold(
    scratch: &Path,
    fixture: &Fixture,
    taken: Taken,
) -> Result<Vec<Reading>, String> {
    let city_root = scratch.join("fold");
    let dir = kernel::layout::CityLayout::new(&city_root).ledger();
    std::fs::create_dir_all(&dir).map_err(|why| format!("{why}"))?;
    let (mut ledger, _report) = storage::JsonlLedger::open(&dir, TimeMs::new(1_700_000_000_000))
        .map_err(|why| format!("{}", why.into_ax()))?;
    let drafts: Vec<EventDraft> = (0..fixture.fold_records)
        .map(super::draft)
        .collect::<Result<_, _>>()?;
    ledger.append_all(drafts).map_err(|why| format!("{why}"))?;
    let mut times = Vec::new();
    for _ in 0..fixture.fold_rounds {
        let t0 = super::stamp();
        let answer = accounting::views::ask(&city_root, &wire::Query::CityView)
            .map_err(|why| format!("{why}"))?;
        times.push(t0.elapsed());
        std::hint::black_box(answer);
    }
    Ok(vec![Reading::of(
        Load::LargeLedgerFold,
        SubMetric::Harness,
        taken,
        times,
    )?])
}

fn large_worktree_placement(
    scratch: &Path,
    fixture: &Fixture,
    taken: Taken,
) -> Result<Vec<Reading>, String> {
    let trees = placement_city(&scratch.join("placement"), fixture)?;
    let mut times = Vec::new();
    for i in 0..fixture.placements {
        let name = node(i)?;
        trees.stock().map_err(|why| format!("{}", why.into_ax()))?;
        let t0 = super::stamp();
        let lease = trees
            .claim(&name, &[])
            .map_err(|why| format!("{}", why.into_ax()))?;
        times.push(t0.elapsed());
        trees
            .release(lease)
            .map_err(|why| format!("{}", why.into_ax()))?;
    }
    Ok(vec![Reading::of(
        Load::LargeWorktreePlacement,
        SubMetric::Whole,
        taken,
        times,
    )?])
}

fn kept_worktree_reclaim(
    scratch: &Path,
    fixture: &Fixture,
    taken: Taken,
) -> Result<Vec<Reading>, String> {
    let trees = placement_city(&scratch.join("reclaim"), fixture)?;
    let name = node(0)?;
    let first = trees
        .claim(&name, &[])
        .map_err(|why| format!("{}", why.into_ax()))?;
    trees
        .release(first)
        .map_err(|why| format!("{}", why.into_ax()))?;
    let mut times = Vec::new();
    for _ in 0..fixture.placements {
        let t0 = super::stamp();
        let lease = trees
            .claim(&name, &[])
            .map_err(|why| format!("{}", why.into_ax()))?;
        times.push(t0.elapsed());
        trees
            .release(lease)
            .map_err(|why| format!("{}", why.into_ax()))?;
    }
    Ok(vec![Reading::of(
        Load::KeptWorktreeReclaim,
        SubMetric::Whole,
        taken,
        times,
    )?])
}

fn placement_city(city_root: &Path, fixture: &Fixture) -> Result<Worktrees, String> {
    let bulk = city_root.join("bulk");
    std::fs::create_dir_all(&bulk).map_err(|why| format!("{why}"))?;
    let bytes = usize::try_from(fixture.tree_file_bytes).map_err(|why| format!("{why}"))?;
    let file = vec![b'x'; bytes];
    for i in 0..fixture.tree_files {
        std::fs::write(bulk.join(format!("file-{i}.bin")), &file)
            .map_err(|why| format!("{why}"))?;
    }
    let owner = Provenance::new(
        RunId::CITY,
        Address::parse("lab/owner").map_err(|why| format!("{why}"))?,
        B3Hash::digest(b"bench placement"),
        ModelChoice {
            id: "bench".to_owned(),
            effort: Some(Effort::Low),
        },
    );
    Checkpoint::open(city_root)
        .map_err(|why| format!("{}", why.into_ax()))?
        .ensure_base(&["bulk".to_owned()], TimeMs::new(1_700_000_000_000), &owner)
        .map_err(|why| format!("{}", why.into_ax()))?;
    Worktrees::open(city_root).map_err(|why| format!("{}", why.into_ax()))
}

fn node(i: u32) -> Result<WorktreeName, String> {
    WorktreeName::parse(&format!("node-{i}")).map_err(|why| format!("{}", why.into_ax()))
}

fn long_session_forwarding(fixture: &Fixture, taken: Taken) -> Result<Vec<Reading>, String> {
    let mut ledger = citysim::MemLedger::new();
    for n in 0..fixture.forward_events {
        ledger
            .append(super::draft(u64::from(n))?)
            .map_err(|why| format!("{why}"))?;
    }
    let mut times = Vec::new();
    for raw in ledger.raw_lines() {
        let record = EventRecord::parse_line(raw).map_err(|why| format!("{why}"))?;
        let t0 = super::stamp();
        let frame = wire::ServerFrame::Event(Box::new(record));
        let bytes = serde_json::to_vec(&frame).map_err(|why| format!("{why}"))?;
        times.push(t0.elapsed());
        std::hint::black_box(bytes);
    }
    Ok(vec![Reading::of(
        Load::LongSessionForwarding,
        SubMetric::Harness,
        taken,
        times,
    )?])
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
