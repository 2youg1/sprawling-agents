// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::CheckpointCommitted;
use kernel::{AxError, EventKind, Payload};

use crate::effect;

use super::{Assignment, RunWorker, Site, held};

impl RunWorker {

    pub(super) fn settle_requests(
        &mut self,
        site: &Site,
        at: &Assignment,
        pr: &std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
        produced: &runtime::Produced,
    ) -> Result<(), AxError> {
        let (addr, who, run_id) = (&at.addr, site.who.as_str(), site.run_id);
        let write_root = site.write_root.as_path();
        let scopes = site.checkpoint_scope()?;

        let pr_effects = held(pr, "settle the pull request desk")?.take_effects();
        if !pr_effects.is_empty() {

            let of = site.provenance(self.city_hash()?, addr);
            let trees = storage::Worktrees::open(&self.city_root)
                .map_err(storage::StorageError::into_ax)?;
            for effect in pr_effects {
                match effect {
                    collab::PrEffect::Opened { branch } => self.offer(
                        &Offering {
                            tree: write_root,
                            scopes: &scopes,
                            of: &of,
                            who,
                            addr,
                            run_id,
                            entropy_allow: site.rules.checkpoint_entropy_allow(),
                        },
                        branch,
                    )?,
                    collab::PrEffect::Merged { request, by } => {

                        if let runtime::Admission::Refused {
                            because,
                            alternative,
                        } = runtime::admits(&at.policy, produced)
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

                        let planned = trees
                            .plan_merge(&name)
                            .map_err(storage::StorageError::into_ax)?;

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

pub(super) struct Offering<'a> {
    pub(super) tree: &'a std::path::Path,
    pub(super) scopes: &'a [String],
    pub(super) of: &'a storage::Provenance,
    pub(super) who: &'a str,
    pub(super) addr: &'a kernel::Address,
    pub(super) run_id: kernel::RunId,
    pub(super) entropy_allow: &'a kernel::EntropyAllowlist,
}

impl RunWorker {

    pub(super) fn offer(&mut self, offering: &Offering<'_>, branch: String) -> Result<(), AxError> {
        let landed = match storage::Checkpoint::open(offering.tree)
            .map_err(storage::StorageError::into_ax)
            .and_then(|checkpoint| {
                let at = self.clock.now()?;
                checkpoint
                    .with_entropy_allowlist(offering.entropy_allow.clone())
                    .land(
                        offering.scopes,
                        at,
                        offering.of,
                        &format!("offer: {}", offering.scopes.join(" ")),
                    )
                    .map_err(storage::StorageError::into_ax)
            }) {
            Ok(landed) => landed,
            Err(err) => {

                let failure = runtime::CheckpointFailure::from(err);
                if let Some(report) = failure.refusal() {
                    self.record_for(
                        offering.run_id,
                        effect::Line {
                            who: offering.who.to_owned(),
                            addr: offering.addr.clone(),
                            kind: EventKind::SecretEgressBlocked,
                            data: Payload::of(report)?,
                        },
                    )?;
                }
                return Err(failure.into_cause());
            }
        };

        let commit = landed.oid.to_string();
        self.record_for(
            offering.run_id,
            effect::Line {
                who: offering.who.to_owned(),
                addr: offering.addr.clone(),
                kind: EventKind::CheckpointCommitted,
                data: Payload::of(&CheckpointCommitted::Committed(landed))?,
            },
        )?;
        let request = collab::OpenRequest {
            node: collab::NodeId::parse(&branch)?,
            implementer: offering.who.to_owned(),
            branch,
            commit,
        };
        self.record_for(
            offering.run_id,
            effect::Line {
                who: offering.who.to_owned(),
                addr: offering.addr.clone(),
                kind: EventKind::PrOpened,
                data: request.payload()?,
            },
        )?;
        self.collaborating.requests.push(request);
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
