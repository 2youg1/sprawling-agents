// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::CheckpointCommitted;
use kernel::{AxError, EventKind, Payload};

use crate::effect;

use super::{Assignment, RunWorker, Site, held};

impl RunWorker {
    pub(super) fn reconcile_project(
        &self,
        project: &storage::ProjectRepository,
        trees: &storage::Worktrees,
    ) -> Result<(), AxError> {
        trees.reconcile_landing(|point| {
            let mut state = storage::LandingRecord::Absent;
            runtime::replay::fold_ledger_dir(&kernel::layout::CityLayout::new(&self.city_root).ledger(), |record| {
                if record.kind() == EventKind::PrMerged {
                    let receipt: collab::MergedRequest = record.data().read()?;
                    if &receipt.project == project.building() && receipt.commit == point.to_string() {
                        state = storage::LandingRecord::Reviewed;
                    }
                } else if record.kind() == EventKind::CheckpointCommitted
                    && matches!(record.data().read()?, CheckpointCommitted::Committed(commit) if commit.oid == point) {
                    state = storage::LandingRecord::Completed;
                }
                Ok(())
            })?;
            Ok(state)
        }).map_err(storage::StorageError::into_ax)
    }

    pub(super) fn finish_work(&mut self, site: &Site, at: &Assignment) -> Result<(), AxError> {
        let Some(lease) = &site.lease else {
            return Ok(());
        };
        let of = site.provenance(self.city_hash()?, &at.addr);
        let now = self.clock.now()?;
        let mut checkpoint = storage::ProjectRepository::run_checkpoint(
            &self.city_root,
            lease.path(),
            site.run_id,
            of.actor(),
        )
        .map_err(storage::StorageError::into_ax)?
        .with_entropy_allowlist(site.rules.checkpoint_entropy_allow().clone());
        let changes = checkpoint.land_changes(&site.checkpoint_scope()?, now, &of, "run completed");
        checkpoint
            .close_writer()
            .map_err(storage::StorageError::into_ax)?;
        let Some(candidate) = self.scanned(changes, site.run_id, &site.who, &at.addr)? else {
            return Ok(());
        };
        if site.rules.review() || at.policy.landing == kernel::LandingPolicy::Experiment {
            let scopes = site.checkpoint_scope()?;
            return self.request_review(
                &Offering {
                    project: site.building.addr(),
                    tree: lease.path(),
                    scopes: &scopes,
                    of: &of,
                    who: &site.who,
                    addr: &at.addr,
                    run_id: site.run_id,
                    entropy_allow: site.rules.checkpoint_entropy_allow(),
                },
                lease.name().as_str().to_owned(),
                candidate,
            );
        }
        let project = storage::ProjectRepository::at(&self.city_root, site.building.addr())
            .map_err(storage::StorageError::into_ax)?;
        let trees = project.trees().map_err(storage::StorageError::into_ax)?;
        self.reconcile_project(&project, &trees)?;
        let plan = trees
            .plan_completed(lease.name(), &candidate.oid)
            .map_err(storage::StorageError::into_ax)?;
        plan.before_landing(
            &storage::Landing {
                t: now,
                of: &of,
                subject: "integrate completed run",
                reviewed_by_person: false,
            },
            |merge| {
                let mut integrated = candidate;
                integrated.oid = kernel::GitOid::parse(&merge).ok_or_else(|| {
                    AxError::failure(
                        kernel::AxCode::StorageFatal,
                        "read the prepared merge",
                        merge,
                    )
                    .with_recovery("verify the private project repository")
                })?;
                self.record_for(
                    site.run_id,
                    effect::Line {
                        who: site.who.clone(),
                        addr: at.addr.clone(),
                        kind: EventKind::CheckpointCommitted,
                        data: Payload::of(&CheckpointCommitted::Committed(integrated))?,
                    },
                )
            },
        )
        .map_err(storage::StorageError::into_ax)?;
        Ok(())
    }

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
            for effect in pr_effects {
                match effect {
                    collab::PrEffect::Opened { branch } => self.offer(
                        &Offering {
                            project: site.building.addr(),
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
                    collab::PrEffect::Merged { ref request, .. }
                    | collab::PrEffect::Rejected { ref request, .. }
                        if !self.collaborating.requests.contains(request) =>
                    {
                        continue;
                    }
                    collab::PrEffect::Merged { mut request, by } => {
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

                        let point = kernel::GitOid::parse(&request.commit).ok_or_else(|| {
                            AxError::failure(
                                kernel::AxCode::InvalidArgs,
                                "read the reviewed commit",
                                &request.commit,
                            )
                            .with_recovery("open a new review request for the committed work")
                        })?;
                        let project =
                            storage::ProjectRepository::at(&self.city_root, &request.project)
                                .map_err(storage::StorageError::into_ax)?;
                        let trees = project.trees().map_err(storage::StorageError::into_ax)?;
                        self.reconcile_project(&project, &trees)?;
                        let planned = match trees
                            .prepare_review(&name, &point)
                            .map_err(storage::StorageError::into_ax)?
                        {
                            storage::ReviewPlan::Ready(plan) => plan,
                            storage::ReviewPlan::Conflicted => {
                                self.record_for(run_id, effect::Line {
                                    who: who.to_owned(), addr: addr.clone(), kind: EventKind::PrRejected,
                                    data: request.rejected_payload(by, "The project has overlapping changes. Reconcile the saved candidate with the current trunk and submit it for review.".to_owned())?,
                                })?;
                                self.collaborating.requests.retain(|held| held != &request);
                                continue;
                            }
                            storage::ReviewPlan::Renewed(revision) => {
                                request.commit = revision.to_string();
                                self.record_for(
                                    run_id,
                                    effect::Line {
                                        who: who.to_owned(),
                                        addr: addr.clone(),
                                        kind: EventKind::PrOpened,
                                        data: request.payload()?,
                                    },
                                )?;
                                request.file(&mut self.collaborating.requests);
                                continue;
                            }
                        };

                        planned
                            .before_landing(
                                &storage::Landing {
                                    t: self.clock.now()?,
                                    of: &of,
                                    subject: &format!("merge: {}", request.branch),
                                    reviewed_by_person: false,
                                },
                                |merge| {
                                    let data =
                                        request.merged_payload(merge, by, of.attribution())?;
                                    self.record_for(
                                        run_id,
                                        effect::Line {
                                            who: who.to_owned(),
                                            addr: addr.clone(),
                                            kind: EventKind::PrMerged,
                                            data,
                                        },
                                    )
                                },
                            )
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
    pub(super) project: &'a kernel::Address,
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
        let checkpoint = storage::ProjectRepository::run_checkpoint(
            &self.city_root,
            offering.tree,
            offering.run_id,
            offering.addr,
        )
        .map_err(storage::StorageError::into_ax)?;
        let mut writer = checkpoint.with_entropy_allowlist(offering.entropy_allow.clone());
        let made = writer.land(
            offering.scopes,
            self.clock.now()?,
            offering.of,
            &format!("offer: {}", offering.scopes.join(" ")),
        );
        writer
            .close_writer()
            .map_err(storage::StorageError::into_ax)?;
        let landed = self.scanned(made, offering.run_id, offering.who, offering.addr)?;
        self.request_review(offering, branch, landed)
    }

    fn scanned<T>(
        &mut self,
        result: Result<T, storage::StorageError>,
        run: kernel::RunId,
        who: &str,
        addr: &kernel::Address,
    ) -> Result<T, AxError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let failure = runtime::CheckpointFailure::from(error);
                if let Some(report) = failure.refusal() {
                    self.record_for(
                        run,
                        effect::Line {
                            who: who.to_owned(),
                            addr: addr.clone(),
                            kind: EventKind::SecretEgressBlocked,
                            data: Payload::of(report)?,
                        },
                    )?;
                }
                Err(failure.into_cause())
            }
        }
    }

    fn request_review(
        &mut self,
        offering: &Offering<'_>,
        branch: String,
        landed: kernel::event::record::Commit,
    ) -> Result<(), AxError> {
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
            project: offering.project.clone(),
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
        request.file(&mut self.collaborating.requests);
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

pub struct PublicationTarget<'a> {
    pub repository: &'a std::path::Path,
    pub base: &'a str,
    pub branch: &'a str,
}

pub fn publish_project(
    city: &std::path::Path,
    addr: &kernel::Address,
    point: kernel::GitOid,
    target: &PublicationTarget<'_>,
) -> Result<String, AxError> {
    let project =
        storage::ProjectRepository::at(city, addr).map_err(storage::StorageError::into_ax)?;
    let mut authorized = false;
    let merge = point.to_string();
    runtime::replay::fold_ledger_dir(&kernel::layout::CityLayout::new(city).ledger(), |record| {
        if record.kind() == EventKind::PrMerged {
            let receipt: collab::MergedRequest = record.data().read()?;
            authorized |= &receipt.project == project.building() && receipt.commit == merge;
        }
        Ok(())
    })?;
    if !authorized {
        return Err(AxError::failure(
            kernel::AxCode::InvalidArgs,
            "publish reviewed work",
            format!("{addr} at {point}"),
        )
        .with_recovery("use the merge commit from a completed review of this project"));
    }
    project
        .publish(point, target.repository, target.base, target.branch)
        .map_err(storage::StorageError::into_ax)
}
