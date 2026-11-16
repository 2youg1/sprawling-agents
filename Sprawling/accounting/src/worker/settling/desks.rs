// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use crate::effect;

use super::super::{Assignment, Desks, Reporter, RunWorker, Settling, Site, held};

impl RunWorker {

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
        let building = &site.building;

        let unread = held(&desks.signals, "settle the signal desk")?.take_unread();
        self.knock_for_returned(&unread, at.policy, &chain)?;

        if let Some(swept) = sweep.swept {
            let base = swept.base;
            let discarded = swept.discarded?;
            let swept = discarded.len();
            let lost = effect::Landing::discards(discarded, addr, who);
            self.settle(at, run_id, lost, &chain)?;

            if swept
                > usize::try_from(kernel::consts_policy::DISCARD_FILES_MAX).unwrap_or(usize::MAX)
            {
                let item = kernel::ApprovalItem {

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

        let (claim_effects, plan_changed) = {
            let mut desk = held(&desks.plan, "settle the plan desk")?;
            desk.abandon()?;
            (desk.take_effects(), desk.roadmap().is_some())
        };
        if plan_changed {
            let on_disk = city::roadmap(&self.city_root, building.addr())?;

            let mut close =
                |closing: effect::Closing| self.record_closing(run_id, closing, open_claims);
            match effect::Claims::of(&claim_effects, &on_disk, desks.plan_path.clone(), addr, who)?
            {
                effect::Claims::Landed(taken) => {
                    let then = taken.record(&mut close)?;
                    self.carry_out_landing(at, run_id, then, &chain)?;
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
                    self.carry_out_landing(at, run_id, then, &chain)?;
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
        Ok(())
    }
}
