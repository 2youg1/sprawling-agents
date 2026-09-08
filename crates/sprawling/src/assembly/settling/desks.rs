// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::{AxCode, AxError};

use crate::effect;

use super::super::{Assignment, Desks, Reporter, RunWorker, Site, Sweep, held, now_ms};

/// Writes the plan back, creating nothing that was not there: a
/// building without a plan is a building whose residents have nothing
/// to claim, and inventing one here would put a denominator on screen
/// that no person wrote.
pub(in crate::assembly) fn write_plan(path: &std::path::Path, text: &str) -> Result<(), AxError> {
    std::fs::write(path, text).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "write the plan",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions; the claim was not recorded")
    })
}

impl RunWorker {
    /// Settles the four desks that leave lines behind, in the order the
    /// history takes them.
    ///
    /// Each one goes through `RunWorker::settle`, which appends before it
    /// changes anything: `effect::Then` has no source but
    /// `Landing::record`, so a change that outran its own line cannot be
    /// written from here (section 8-24).
    ///
    /// The room's queue comes home first and on both paths - an inbox
    /// left in a dropped desk is a queue the city forgot it had.
    ///
    /// # Errors
    /// Propagates a payload that will not build, any line the ledger
    /// refuses, and a shared plan that cannot be read or written.
    pub(in crate::assembly) fn settle_desks(
        &mut self,
        site: &Site,
        at: &Assignment,
        desks: &Desks,
        sweep: Sweep<'_>,
    ) -> Result<(), AxError> {
        let (addr, who, run_id) = (&at.addr, site.who.as_str(), site.run_id);
        let (write_root, building) = (site.write_root.as_path(), &site.building);
        // The lent queue comes home first, and on both paths: an inbox
        // left in a dropped desk is a queue the city forgot it had.
        let (signal_effects, returned) = {
            let mut desk = held(&desks.signals, "settle the signal desk")?;
            (desk.take_effects(), desk.take_inbox())
        };
        self.inboxes.insert(addr.clone(), returned);
        // Every desk below settles through one door, and that door
        // appends before it changes anything: `effect::Then` has no
        // other source than `Landing::record`, so a change that outran
        // its own line cannot be written here.
        let spoken = effect::Landing::signals(signal_effects, addr, who)?;
        self.settle(at, run_id, spoken)?;
        let ground = effect::Landing::goals(
            held(&desks.goals, "settle the goal desk")?.take_effects(),
            addr,
            who,
        )?;
        self.settle(at, run_id, ground)?;
        // The sweep the forecast cannot replace. A command can be
        // obfuscated past a text prediction; what is missing from the
        // working tree cannot be talked out of. The base is the first
        // fence of this drive, so everything the whole drive deleted is
        // reported once rather than once per wave.
        let sweep_base = sweep.fenced.first().cloned();
        if let Some(base) = sweep_base {
            let discarded = memory::Checkpoint::open(write_root)
                .map_err(memory::MemoryError::into_ax)?
                .wave_post(&base)
                .map_err(memory::MemoryError::into_ax)?;
            let swept = discarded.len();
            let lost = effect::Landing::discards(discarded, addr, who);
            self.settle(at, run_id, lost)?;
            // Over the threshold a person is told, and the class is one
            // no policy can waive. Each file is restorable on its own;
            // what the count says is that nobody meant this.
            if swept
                > usize::try_from(kernel::consts_policy::DISCARD_FILES_MAX).unwrap_or(usize::MAX)
            {
                let item = kernel::ApprovalItem {
                    id: kernel::ApprovalId::new(format!("discard-{run_id}")).ok_or_else(|| {
                        AxError::failure(AxCode::InvalidArgs, "mint approval id", "empty id")
                    })?,
                    source: kernel::ApprovalSource::Gate,
                    actor: who.to_owned(),
                    artifact: sweep.job_locator.clone(),
                    action_desc: format!("{swept} files were deleted in one dispatch"),
                    cluster_key: kernel::ClusterKey {
                        class: kernel::ApprovalClass::DiscardEscalate,
                        detail: addr.as_str().to_owned(),
                    },
                    created: now_ms()?,
                    tainted: false,
                };
                sweep.raised.push(item);
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "memory::checkpoint",
                    &format!(
                        "{swept} files deleted under {}; each one can be restored from {base}",
                        addr.as_str()
                    ),
                );
            }
        }
        // What the run did to the plan.
        let (claim_effects, plan_after) = {
            let mut desk = held(&desks.plan, "settle the plan desk")?;
            (desk.take_effects(), desk.roadmap().map(str::to_owned))
        };
        if let Some(text) = plan_after {
            let on_disk = city::roadmap(&self.city_root, building.addr())?;
            match effect::Claims::of(
                &claim_effects,
                &on_disk,
                text,
                desks.plan_path.clone(),
                addr,
                who,
            )? {
                effect::Claims::Landed(taken) => {
                    self.settle(at, run_id, *taken)?;
                    self.tell_whoever_is_behind(
                        at,
                        Reporter {
                            run_id,
                            building: building.addr(),
                            who,
                        },
                        &claim_effects,
                    )?;
                }
                effect::Claims::Stale(nodes) => {
                    for node in nodes {
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
        }
        // What the run asked the building to remember, filed after the
        // drive like every other effect - and inside the fence. A
        // building under review is not the owner of what a run decided
        // until somebody checks it, and a shelf entry is exactly the
        // kind of thing a later run reads as the building's settled
        // knowledge.
        let remembered = effect::Landing::shelf(
            held(&desks.shelf, "settle the shelf")?.take_effects(),
            write_root,
            building.addr(),
            now_ms()?,
            addr,
            who,
        )?;
        self.settle(at, run_id, remembered)?;
        Ok(())
    }
}
