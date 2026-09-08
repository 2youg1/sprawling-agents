// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run offers a building it may not write in, and what merging it costs.

use kernel::{AxError, EventKind, Payload};

use crate::effect;

use super::{Assignment, RunWorker, Site, now_ms};

impl RunWorker {
    /// Settles what a run asked of the request register.
    ///
    /// Opening commits the run's own tree first, because the record names
    /// the commit a verifier will be judging. Checking merges, because
    /// that is what a passed check means, and a verified request nobody
    /// merged would be a third state for a person to chase. Both put the
    /// line before the change, the merge by way of `memory::PlannedMerge`
    /// (section 8-30).
    ///
    /// # Errors
    /// Propagates a worktree that will not open, a fence that will not
    /// commit, a merge the trunk has moved past, and any line the ledger
    /// refuses.
    pub(super) fn settle_requests(
        &mut self,
        site: &Site,
        at: &Assignment,
        pr: &std::rc::Rc<std::cell::RefCell<collab::PrDesk>>,
        produced: &runtime::Produced,
    ) -> Result<(), AxError> {
        let (addr, who, run_id, mode) = (&at.addr, site.who.as_str(), site.run_id, at.mode);
        let write_root = site.write_root.as_path();
        let fence_scope = site.fence_scope(addr);
        let fence_scope = fence_scope.as_str();
        // What the run asked of the request register. Opening commits
        // the run's own tree first, because the record names the commit
        // a verifier will be judging; checking merges, because that is
        // what a passed check means and a verified request nobody merged
        // would be a third state for a person to chase.
        let pr_effects = pr.borrow_mut().take_effects();
        if !pr_effects.is_empty() {
            // What every commit this settlement makes is signed with.
            // Read once here rather than per effect: the city's genesis
            // line does not change while a settlement runs.
            let of = site.provenance(&self.city_root, addr)?;
            let trees =
                memory::Worktrees::open(&self.city_root).map_err(memory::MemoryError::into_ax)?;
            for effect in pr_effects {
                match effect {
                    collab::PrEffect::Opened { branch } => {
                        // Landed rather than fenced: what a verifier
                        // judges has to be on the run's own branch, and
                        // a wave fence is a dangling commit nobody can
                        // merge (memory-SPEC 8-8, card-2.3).
                        let at = memory::Checkpoint::open(write_root)
                            .map_err(memory::MemoryError::into_ax)?
                            .land(now_ms()?, &of, &format!("offer: {fence_scope}"))
                            .map_err(memory::MemoryError::into_ax)?;
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
                        self.requests.push(request);
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
                            let mut data = request.payload()?.as_map().clone();
                            data.insert("by".to_owned(), serde_json::Value::String(by));
                            data.insert(
                                "why".to_owned(),
                                serde_json::Value::String(format!("{because}; {alternative}")),
                            );
                            self.record_for(
                                run_id,
                                effect::Line {
                                    who: who.to_owned(),
                                    addr: addr.clone(),
                                    kind: EventKind::PrRejected,
                                    data: Payload::new(data)?,
                                },
                            )?;
                            self.requests.retain(|held| held.branch != request.branch);
                            continue;
                        }
                        let name = memory::WorktreeName::parse(&request.branch)
                            .map_err(memory::MemoryError::into_ax)?;
                        // Decided first, announced second, made third.
                        // The refusal this merge can carry - a trunk that
                        // moved after the node branched - happens inside
                        // `plan_merge`, so no line is ever written for a
                        // merge that was going to be refused; and the
                        // trunk cannot move before the line, because
                        // moving it needs a value only that call returns.
                        let planned = trees
                            .plan_merge(&name)
                            .map_err(memory::MemoryError::into_ax)?;
                        let mut data = request.payload()?.as_map().clone();
                        data.insert("verified_by".to_owned(), serde_json::Value::String(by));
                        data.insert(
                            "commit".to_owned(),
                            serde_json::Value::String(planned.commit()),
                        );
                        // What the merge commit's own trailers carry and
                        // this record cannot say for itself, so "which
                        // run wrote this commit" is answered from the
                        // ledger rather than from git (card-2.4).
                        data.extend(of.model_fields());
                        self.record_for(
                            run_id,
                            effect::Line {
                                who: who.to_owned(),
                                addr: addr.clone(),
                                kind: EventKind::PrMerged,
                                data: Payload::new(data)?,
                            },
                        )?;
                        // A person other than the author verified this,
                        // which is what `PrEffect::Merged` means; the
                        // name is taken from the repository's own git
                        // config or left out entirely.
                        planned
                            .apply(&memory::Landing {
                                t: now_ms()?,
                                of: &of,
                                subject: &format!("merge: {}", request.branch),
                                reviewed_by_person: true,
                            })
                            .map_err(memory::MemoryError::into_ax)?;
                        self.requests.retain(|held| held.branch != request.branch);
                    }
                    collab::PrEffect::Rejected { request, by, why } => {
                        let mut data = request.payload()?.as_map().clone();
                        data.insert("by".to_owned(), serde_json::Value::String(by));
                        data.insert("why".to_owned(), serde_json::Value::String(why));
                        self.record_for(
                            run_id,
                            effect::Line {
                                who: who.to_owned(),
                                addr: addr.clone(),
                                kind: EventKind::PrRejected,
                                data: Payload::new(data)?,
                            },
                        )?;
                        self.requests.retain(|held| held.branch != request.branch);
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
