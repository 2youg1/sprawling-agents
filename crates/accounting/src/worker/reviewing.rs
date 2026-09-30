// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run offers a building it may not write in, and what merging it costs.

use kernel::{AxError, EventKind};

use crate::effect;

use super::{Assignment, RunWorker, Site, held};

impl RunWorker {
    /// Settles what a run asked of the request register.
    ///
    /// Opening commits the run's own tree first, because the record names
    /// the commit a verifier will be judging. Checking merges, because
    /// that is what a passed check means, and a verified request nobody
    /// merged would be a third state for a person to chase. Both put the
    /// line before the change, the merge by way of `storage::PlannedMerge`
    /// (section 8-30).
    ///
    /// # Errors
    /// Propagates a worktree that will not open, a checkpoint that will not
    /// commit, a merge the trunk has moved past, and any line the ledger
    /// refuses.
    pub(super) fn settle_requests(
        &mut self,
        site: &Site,
        at: &Assignment,
        pr: &std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
        produced: &runtime::Produced,
    ) -> Result<(), AxError> {
        let (addr, who, run_id, mode) = (&at.addr, site.who.as_str(), site.run_id, at.mode);
        let write_root = site.write_root.as_path();
        let scopes = site.checkpoint_scope()?;
        let checkpoint_scope = scopes.join(" ");
        let checkpoint_scope = checkpoint_scope.as_str();
        // What the run asked of the request register. Opening commits
        // the run's own tree first, because the record names the commit
        // a verifier will be judging; checking merges, because that is
        // what a passed check means and a verified request nobody merged
        // would be a third state for a person to chase.
        let pr_effects = held(pr, "settle the pull request desk")?.take_effects();
        if !pr_effects.is_empty() {
            // What every commit this settlement makes is signed with.
            // Read once here rather than per effect: the city's genesis
            // line does not change while a settlement runs.
            let of = site.provenance(self.city_hash()?, addr);
            let trees = storage::Worktrees::open(&self.city_root)
                .map_err(storage::StorageError::into_ax)?;
            for effect in pr_effects {
                match effect {
                    collab::PrEffect::Opened { branch } => {
                        // Landed rather than checkpointed: what a verifier
                        // judges has to be on the run's own branch, and
                        // a wave checkpoint is a dangling commit nobody can
                        // merge (storage-SPEC 8-8).
                        let at = storage::Checkpoint::open(write_root)
                            .map_err(storage::StorageError::into_ax)?
                            .land(
                                &scopes,
                                self.clock.now()?,
                                &of,
                                &format!("offer: {checkpoint_scope}"),
                            )
                            .map_err(storage::StorageError::into_ax)?;
                        let request = collab::OpenRequest {
                            node: collab::NodeId::parse(&branch)?,
                            implementer: who.to_owned(),
                            branch,
                            commit: at,
                        };
                        self.record_for(
                            run_id,
                            effect::Line {
                                who: who.to_owned(),
                                addr: addr.clone(),
                                kind: EventKind::PrOpened,
                                data: request.payload()?,
                            },
                        )?;
                        self.collaborating.requests.push(request);
                    }
                    collab::PrEffect::Merged { request, by } => {
                        // The last gate before work becomes the
                        // building's. Verification says a person other
                        // than the author looked; admission says the
                        // evidence this mode demands is present. They
                        // are different questions, and the second one is
                        // the only place a mode means anything.
                        if let runtime::Admission::Refused {
                            because,
                            alternative,
                        } = runtime::admits(mode, produced)
                        {
                            self.record_for(
                                run_id,
                                effect::Line {
                                    who: who.to_owned(),
                                    addr: addr.clone(),
                                    kind: EventKind::PrRejected,
                                    data: request.rejected_payload(
                                        by,
                                        format!("{because}; {alternative}"),
                                    )?,
                                },
                            )?;
                            self.collaborating
                                .requests
                                .retain(|held| held.branch != request.branch);
                            continue;
                        }
                        let name = storage::WorktreeName::parse(&request.branch)
                            .map_err(storage::StorageError::into_ax)?;
                        // Decided first, announced second, made third.
                        // The refusal this merge can carry - a trunk that
                        // moved after the node branched - happens inside
                        // `plan_merge`, so no line is ever written for a
                        // merge that was going to be refused; and the
                        // trunk cannot move before the line, because
                        // moving it needs a value only that call returns.
                        let planned = trees
                            .plan_merge(&name)
                            .map_err(storage::StorageError::into_ax)?;
                        // The merged line's keys live once, on
                        // `OpenRequest::merged_payload`: the commit that
                        // was reviewed and the commit that landed are
                        // two facts and therefore two keys.
                        let data =
                            request.merged_payload(planned.commit(), by, of.attribution())?;
                        self.record_for(
                            run_id,
                            effect::Line {
                                who: who.to_owned(),
                                addr: addr.clone(),
                                kind: EventKind::PrMerged,
                                data,
                            },
                        )?;
                        // A second resident verified this, which is what
                        // `PrEffect::Merged` means and what a review
                        // building is for - not a person. A
                        // `Reviewed-by:` trailer here would put the name
                        // in this machine's git config on work that
                        // person never read (sprawling-SPEC.md 8-49).
                        planned
                            .apply(&storage::Landing {
                                t: self.clock.now()?,
                                of: &of,
                                subject: &format!("merge: {}", request.branch),
                                reviewed_by_person: false,
                            })
                            .map_err(storage::StorageError::into_ax)?;
                        self.collaborating
                            .requests
                            .retain(|held| held.branch != request.branch);
                    }
                    collab::PrEffect::Rejected { request, by, why } => {
                        self.record_for(
                            run_id,
                            effect::Line {
                                who: who.to_owned(),
                                addr: addr.clone(),
                                kind: EventKind::PrRejected,
                                data: request.rejected_payload(by, why)?,
                            },
                        )?;
                        self.collaborating
                            .requests
                            .retain(|held| held.branch != request.branch);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
