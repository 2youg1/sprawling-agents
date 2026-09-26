// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The four load scenarios: multi-run parallel, large-ledger fold,
//! large-worktree placement, and long-session streaming forward.
//!
//! Each is a thin measuring loop over the product's own public faces
//! (citysim-SPEC.md section 8-6). The load every recorded reading was
//! taken under is `REGISTERED`, and readings are comparable only within
//! one fixture. This module lives in the bench Main's tree: the
//! simulator's scenarios keep their counted clock, and every `Instant`
//! outside `bin::assembly` is this tree's alone.

use std::path::Path;
use std::time::Duration;

use kernel::{Address, B3Hash, Effort, EventDraft, EventRecord, Ledger as _, RunId, TimeMs};
use memory::{Checkpoint, ModelChoice, Provenance, WorktreeName, Worktrees};

use super::reading::{Load, MachineClass, Reading, SubMetric};

/// The fixed load each scenario runs under. Data, not policy: change a
/// field and the new readings are not comparable with the register's.
pub(crate) struct Fixture {
    /// Driving lanes writing at once, and the records each one writes.
    pub(crate) lanes: u32,
    pub(crate) records_per_lane: u32,
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
    lanes: 4,
    records_per_lane: 250,
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
            Load::MultiRunParallel => multi_run_parallel(scratch, fixture, machine)?,
            Load::LargeLedgerFold => large_ledger_fold(scratch, fixture, machine)?,
            Load::LargeWorktreePlacement => large_worktree_placement(scratch, fixture, machine)?,
            Load::LongSessionForwarding => long_session_forwarding(fixture, machine)?,
        };
        readings.extend(of_load);
    }
    Ok(readings)
}

/// Multi-run parallel: lanes write at once and one accounting thread
/// takes every write, once through each Ledger adapter.
fn multi_run_parallel(
    scratch: &Path,
    fixture: &Fixture,
    machine: MachineClass,
) -> Result<Vec<Reading>, String> {
    let harness = accounting(&mut citysim::MemLedger::new(), fixture)?;
    let dir = scratch.join("multi-run");
    std::fs::create_dir_all(&dir).map_err(|why| format!("{why}"))?;
    let (mut ledger, _report) = memory::JsonlLedger::open(&dir, TimeMs::new(1_700_000_000_000))
        .map_err(|why| format!("{}", why.into_ax()))?;
    let persist = accounting(&mut ledger, fixture)?;
    Ok(vec![
        Reading::of(Load::MultiRunParallel, SubMetric::Harness, machine, harness)?,
        Reading::of(Load::MultiRunParallel, SubMetric::Persist, machine, persist)?,
    ])
}

/// The shape production drives in: lanes hand drafts to one accounting
/// thread and wait, so the per-record time is what every lane's write
/// waits for. The relay face is `pub(crate)` in `sprawling::serving`, so
/// this reproduces its shape over the same `kernel::Ledger` port.
fn accounting<L: kernel::Ledger>(
    ledger: &mut L,
    fixture: &Fixture,
) -> Result<Vec<Duration>, String> {
    let mut times = Vec::new();
    let mut failure = None;
    std::thread::scope(|scope| {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut lanes = Vec::new();
        for lane in 0..fixture.lanes {
            let tx = tx.clone();
            let per = u64::from(fixture.records_per_lane);
            lanes.push(scope.spawn(move || -> Result<(), String> {
                for i in 0..per {
                    let n = u64::from(lane).saturating_mul(per).saturating_add(i);
                    tx.send(super::draft(n)?)
                        .map_err(|why| format!("hand a draft to the accounting thread: {why}"))?;
                }
                Ok(())
            }));
        }
        drop(tx);
        for draft in rx {
            if failure.is_some() {
                continue;
            }
            let t0 = super::stamp();
            match ledger.append(draft) {
                Ok(committed) => {
                    std::hint::black_box(committed);
                    times.push(t0.elapsed());
                }
                Err(why) => failure = Some(format!("{why}")),
            }
        }
        for lane in lanes {
            let joined = match lane.join() {
                Ok(Ok(())) => continue,
                Ok(Err(why)) => why,
                Err(_) => "a driving lane panicked before its records were sent".to_owned(),
            };
            if failure.is_none() {
                failure = Some(joined);
            }
        }
    });
    failure.map_or(Ok(times), Err)
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
    let (mut ledger, _report) = memory::JsonlLedger::open(&dir, TimeMs::new(1_700_000_000_000))
        .map_err(|why| format!("{}", why.into_ax()))?;
    let drafts: Vec<EventDraft> = (0..fixture.fold_records)
        .map(super::draft)
        .collect::<Result<_, _>>()?;
    ledger.append_all(drafts).map_err(|why| format!("{why}"))?;
    let mut times = Vec::new();
    for _ in 0..fixture.fold_rounds {
        let t0 = super::stamp();
        let answer = sprawling::ask(&city_root, &channels::Query::CityView)
            .map_err(|why| format!("{why}"))?;
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
    let city_root = scratch.join("placement");
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
    let mut checkpoint =
        Checkpoint::open(&city_root).map_err(|why| format!("{}", why.into_ax()))?;
    checkpoint
        .ensure_base(&["bulk".to_owned()], TimeMs::new(1_700_000_000_000), &owner)
        .map_err(|why| format!("{}", why.into_ax()))?;
    let trees = Worktrees::open(&city_root).map_err(|why| format!("{}", why.into_ax()))?;
    let mut times = Vec::new();
    for i in 0..fixture.placements {
        let name = WorktreeName::parse(&format!("node-{i}"))
            .map_err(|why| format!("{}", why.into_ax()))?;
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
        let frame = channels::ServerFrame::Event(Box::new(record));
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
