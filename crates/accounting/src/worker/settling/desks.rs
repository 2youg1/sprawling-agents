// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::AxError;

use crate::effect;

use super::super::{Assignment, Desks, Reporter, RunWorker, Settling, Site, held};

impl RunWorker {
    /// Settles the three desks that leave lines behind, in the order the
    /// history takes them.
    ///
    /// Each one goes through `RunWorker::settle`, which appends before it
    /// changes anything: `effect::Then` has no source but
    /// `Landing::record`, so a change that outran its own line cannot be
    /// written from here (section 8-24).
    ///
    /// The room's queue is already home by the time this runs: it is
    /// given back in `return_borrowed`, before the drive's own outcome
    /// is read, because a queue returned only on the happy path is a
    /// queue lost exactly when the disk went wrong.
    ///
    /// # Errors
    /// Propagates a payload that will not build, any line the ledger
    /// refuses, and a shared plan that cannot be read or written.
    pub(in crate::worker) fn settle_desks(
        &mut self,
        site: &Site,
        at: &Assignment,
        desks: &Desks,
        settling: Settling<'_>,
    ) -> Result<(), AxError> {
        let Settling {
            sweep,
            chain,
            open_claims,
        } = settling;
        let (addr, who, run_id) = (&at.addr, site.who.as_str(), site.run_id);
        let (write_root, building) = (site.write_root.as_path(), &site.building);
        let signal_effects = held(&desks.signals, "settle the signal desk")?.take_effects();
        // Every desk below settles through one door, and that door
        // appends before it changes anything: `effect::Then` has no
        // other source than `Landing::record`, so a change that outran
        // its own line cannot be written here.
        let spoken = effect::Landing::signals(signal_effects, addr, who)?;
        self.settle(at, run_id, spoken, &chain)?;
        // The sweep the forecast cannot replace. A command can be
        // obfuscated past a text prediction; what is missing from the
        // working tree cannot be talked out of. The base is the first
        // checkpoint of this drive, so everything the whole drive deleted is
        // reported once rather than once per wave.
        let sweep_base = sweep.checkpointed.first().cloned();
        if let Some(base) = sweep_base {
            let discarded = storage::Checkpoint::open(write_root)
                .map_err(storage::StorageError::into_ax)?
                .wave_post(&base)
                .map_err(storage::StorageError::into_ax)?;
            let swept = discarded.len();
            let lost = effect::Landing::discards(discarded, addr, who);
            self.settle(at, run_id, lost, &chain)?;
            // Over the threshold a person is told, and the class is one
            // no policy can waive. Each file is restorable on its own;
            // what the count says is that nobody meant this.
            if swept
                > usize::try_from(kernel::consts_policy::DISCARD_FILES_MAX).unwrap_or(usize::MAX)
            {
                let item = kernel::ApprovalItem {
                    // The sweep's own slot in this run: one escalation
                    // per run, at a position the run's call counter
                    // never reaches (`crates/kernel/Spec.lean` §8-21).
                    id: kernel::ApprovalId::of_sweep(&run_id),
                    actor: who.to_owned(),
                    artifact: sweep.job_locator.clone(),
                    action_desc: format!("{swept} files were deleted in one dispatch"),
                    cluster_key: kernel::ClusterKey {
                        class: kernel::ApprovalClass::Question,
                        detail: addr.as_str().to_owned(),
                    },
                    created: self.clock.now()?,
                    tainted: false,
                };
                sweep.raised.push(item);
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "storage::checkpoint",
                    &format!(
                        "{swept} files deleted under {}; each one can be restored from {base}",
                        addr.as_str()
                    ),
                );
            }
        }
        // What the run did to the plan. This is the freeze path: a node
        // still held here was neither finished nor stopped, and the
        // `Held` dies with the desk, so it is spent on its one exit now
        // or the row stays `In progress` for ever.
        let (claim_effects, plan_changed) = {
            let mut desk = held(&desks.plan, "settle the plan desk")?;
            desk.abandon()?;
            (desk.take_effects(), desk.roadmap().is_some())
        };
        if plan_changed {
            let on_disk = city::roadmap(&self.city_root, building.addr())?;
            // A node's claim is closed the moment its closing line is on
            // the ledger, not when `Roadmap.md` is rewritten or the whole
            // landing is: a refusal part-way leaves owed only the nodes
            // whose last line still reads them as held (sprawling-SPEC.md
            // 8-42-8).
            let mut close =
                |closing: effect::Closing| self.record_closing(run_id, closing, open_claims);
            match effect::Claims::of(&claim_effects, &on_disk, desks.plan_path.clone(), addr, who)?
            {
                effect::Claims::Landed(taken) => {
                    let then = taken.record(&mut close)?;
                    self.carry_out_landing(at, then, &chain)?;
                    self.tell_whoever_is_behind(
                        at,
                        Reporter {
                            run_id,
                            building: building.addr(),
                            who,
                        },
                        &claim_effects,
                        &chain,
                    )?;
                }
                effect::Claims::Stale { node, released } => {
                    let then = released.record(&mut close)?;
                    self.carry_out_landing(at, then, &chain)?;
                    self.note(
                        runtime::diagnostics::Level::Refuse,
                        "collab::claim_tool",
                        &format!(
                            "node {node} moved before this run's claim landed; nothing was \
                             written"
                        ),
                    );
                }
            }
        }
        // What the run asked the building to remember, filed after the
        // drive like every other effect - and inside the checkpoint. A
        // building under review is not the owner of what a run decided
        // until somebody checks it, and a shelf entry is exactly the
        // kind of thing a later run reads as the building's settled
        // knowledge.
        let remembered = effect::Landing::shelf(
            held(&desks.shelf, "settle the shelf")?.take_effects(),
            write_root,
            building.addr(),
            self.clock.now()?,
            addr,
            who,
        )?;
        self.settle(at, run_id, remembered, &chain)?;
        Ok(())
    }
}
