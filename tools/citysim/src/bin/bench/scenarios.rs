// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Four of the five load scenarios: large-ledger fold, large-worktree
//! placement, kept-worktree reclaim, and long-session streaming forward.
//!
//! The fifth, multi-run parallel, is measured inside `sprawling` by
//! `instrument_relay_round_trip`, which drives the accounting loop the
//! city runs. Its relay face is `pub(crate)`, so a scenario here could
//! only time a copy of that loop, and a copy reads what the copy costs
//! (citysim-SPEC.md section 8-6).
//!
//! Each is a thin measuring loop over the product's own public faces
//! (citysim-SPEC.md section 8-6). The load every recorded reading was
//! taken under is `REGISTERED`, and readings are comparable only within
//! one fixture. This module lives in the bench Main's tree: the
//! simulator's scenarios keep their counted clock, and every `Instant`
//! outside `bin::assembly` is this tree's alone.

use std::path::Path;

use kernel::{Address, B3Hash, Effort, EventDraft, EventRecord, Ledger as _, RunId, TimeMs};
use storage::{Checkpoint, ModelChoice, Provenance, WorktreeName, Worktrees};

use super::reading::{Load, MachineClass, Reading, SubMetric};

/// The fixed load each scenario runs under. Data, not policy: change a
/// field and the new readings are not comparable with the register's.
pub(crate) struct Fixture {
    /// The ledger the fold walks, and how many rebuilds are sampled.
    pub(crate) fold_records: u64,
    pub(crate) fold_rounds: u32,
    /// The placed tree: how many files, how large each one is.
    pub(crate) tree_files: u32,
    pub(crate) tree_file_bytes: u32,
    /// How many trees are placed, and how many events are forwarded.
    pub(crate) placements: u32,
    pub(crate) forward_events: u32,
}

/// The registered load: the fixture every baseline reading names.
pub(crate) const REGISTERED: Fixture = Fixture {
    fold_records: 50_000,
    fold_rounds: 5,
    tree_files: 512,
    tree_file_bytes: 16_384,
    placements: 4,
    forward_events: 2_000,
};

/// Runs every load scenario and returns its readings.
///
/// # Errors
/// Propagates whatever a scenario's public face refused.
pub(crate) fn all(
    scratch: &Path,
    fixture: &Fixture,
    machine: MachineClass,
) -> Result<Vec<Reading>, String> {
    let mut readings = Vec::new();
    for load in Load::ALL {
        let of_load = match load {
            Load::LargeLedgerFold => large_ledger_fold(scratch, fixture, machine)?,
            Load::LargeWorktreePlacement => large_worktree_placement(scratch, fixture, machine)?,
            Load::KeptWorktreeReclaim => kept_worktree_reclaim(scratch, fixture, machine)?,
            Load::LongSessionForwarding => long_session_forwarding(fixture, machine)?,
        };
        readings.extend(of_load);
    }
    Ok(readings)
}

/// Large-ledger fold: the whole production rebuild over a large ledger -
/// verify the chain, parse every line, fold it into every view.
fn large_ledger_fold(
    scratch: &Path,
    fixture: &Fixture,
    machine: MachineClass,
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
        let answer =
            sprawling::ask(&city_root, &wire::Query::CityView).map_err(|why| format!("{why}"))?;
        times.push(t0.elapsed());
        std::hint::black_box(answer);
    }
    Ok(vec![Reading::of(
        Load::LargeLedgerFold,
        SubMetric::Harness,
        machine,
        times,
    )?])
}

/// Large-worktree placement: one claim places the whole tree. The seam
/// does not split weighing from copying, so the reading is priced whole.
fn large_worktree_placement(
    scratch: &Path,
    fixture: &Fixture,
    machine: MachineClass,
) -> Result<Vec<Reading>, String> {
    let trees = placement_city(&scratch.join("placement"), fixture)?;
    let mut times = Vec::new();
    for i in 0..fixture.placements {
        let name = node(i)?;
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
        machine,
        times,
    )?])
}

/// Kept-worktree reclaim: a node's second run takes back the tree its
/// first run left, with the trunk unmoved in between. Priced whole for
/// the same reason as the first placement.
fn kept_worktree_reclaim(
    scratch: &Path,
    fixture: &Fixture,
    machine: MachineClass,
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
        machine,
        times,
    )?])
}

/// A city of `fixture.tree_files` files under `bulk`, with the base
/// commit every worktree branches from.
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

/// Long-session streaming forward: every event of one long session,
/// framed and serialised the way the socket would send it. The network
/// is deliberately outside the timed span - the reading is the local
/// processing latency this harness owns.
fn long_session_forwarding(
    fixture: &Fixture,
    machine: MachineClass,
) -> Result<Vec<Reading>, String> {
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
        machine,
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
