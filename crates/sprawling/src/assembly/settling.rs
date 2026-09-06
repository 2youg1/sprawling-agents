// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::{AxCode, AxError, EventKind};
use kernel::{Locator, Payload, RunId};

use crate::effect;

use super::{Assignment, Desks, Dispatched, Reporter, RunWorker, Site, new_inbox, now_ms};

/// What the sweep after a drive works on.
///
/// The three arrive together because they answer one question - what a
/// wave left in the tree that the history has not accounted for yet: the
/// commits to restore a discarded file from, the escalations a gate
/// raised while the driver held the ledger, and the pin of the work an
/// escalation interrupted. `raised` is borrowed mutably because the
/// sweep adds to it: a discard a person has to answer is raised here
/// rather than during the drive.
pub(super) struct Sweep<'a> {
    pub(super) fenced: &'a [String],
    pub(super) raised: &'a mut Vec<kernel::ApprovalItem>,
    pub(super) job_locator: &'a Locator,
}

/// What one drive ended with, as the conclusion reads it.
///
/// The drive's own outcome stays a `Result` here rather than being
/// propagated: a run that failed still has a tree to give back and
/// approvals to file, and both are worse left undone than the failure
/// that caused them.
pub(super) struct Ending<'a> {
    pub(super) driven: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    pub(super) raised: Vec<kernel::ApprovalItem>,
    pub(super) delegates: &'a std::rc::Rc<std::cell::RefCell<collab::DelegateDesk>>,
}

/// Writes the plan back, creating nothing that was not there: a
/// building without a plan is a building whose residents have nothing
/// to claim, and inventing one here would put a denominator on screen
/// that no person wrote.
pub(super) fn write_plan(path: &std::path::Path, text: &str) -> Result<(), AxError> {
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
    pub(super) fn settle_desks(
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
            let mut desk = desks.signals.borrow_mut();
            (desk.take_effects(), desk.take_inbox())
        };
        self.inboxes.insert(addr.clone(), returned);
        // Every desk below settles through one door, and that door
        // appends before it changes anything: `effect::Then` has no
        // other source than `Landing::record`, so a change that outran
        // its own line cannot be written here.
        let spoken = effect::Landing::signals(signal_effects, addr, who)?;
        self.settle(at, run_id, spoken)?;
        let ground = effect::Landing::goals(desks.goals.borrow_mut().take_effects(), addr, who)?;
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
            let mut desk = desks.plan.borrow_mut();
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
            desks.shelf.borrow_mut().take_effects(),
            write_root,
            building.addr(),
            now_ms()?,
            addr,
            who,
        )?;
        self.settle(at, run_id, remembered)?;
        Ok(())
    }

    pub(super) fn settle(
        &mut self,
        at: &Assignment,
        run: RunId,
        landing: effect::Landing,
    ) -> Result<(), AxError> {
        let then = landing.record(&mut |line: effect::Line| self.record_for(run, line))?;
        match then {
            effect::Then::Nothing => Ok(()),
            effect::Then::Deliver(signals) => {
                let knocks_mark = self.knocks.len();
                let outcome = (|| {
                    for signal in &signals {
                        let admitted = self
                            .inboxes
                            .entry(signal.room().clone())
                            .or_insert_with(new_inbox)
                            .deliver(signal)?;
                        // A shed delivery is a failure of this arm, not a
                        // quiet drop: the line is already on the ledger,
                        // and a line no queue holds is a torn city.
                        if matches!(admitted, kernel::Admission::Shed { .. }) {
                            return Err(AxError::failure(
                                AxCode::BackpressureShed,
                                "deliver a signal",
                                format!("room {} shed the signal", signal.room()),
                            )
                            .with_recovery(
                                "the queue is full; the speaker retries when the room drains",
                            ));
                        }
                        // Somebody was spoken to. Whether that starts a run
                        // is decided in one place, so that the two ways of
                        // reaching a resident stay one decision.
                        self.knock(signal, &at.addr, at.mode, at.budget)?;
                    }
                    Ok(())
                })();
                match outcome {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        // The lines are all on the ledger; the city is not
                        // torn: knocks pushed by the signals that did land
                        // are cut back to the mark. Delivered signals stay
                        // delivered — the queue has no recall — and the
                        // ledger says exactly which ones those are.
                        self.knocks.truncate(knocks_mark);
                        Err(err)
                    }
                }
            }
            effect::Then::Hold(entries) => {
                self.goals.extend(entries);
                Ok(())
            }
            effect::Then::Roadmap { path, text } => {
                let before = std::fs::read_to_string(&path).ok();
                write_plan(&path, &text).inspect_err(|_| match before {
                    Some(text) => {
                        let _ = std::fs::write(&path, text);
                    }
                    None => {
                        let _ = std::fs::remove_file(&path);
                    }
                })
            }
            effect::Then::Shelf(filings) => {
                let mut written = Vec::new();
                let outcome = (|| {
                    for filing in &filings {
                        city::file_archive(&filing.entry, &filing.body)?;
                        written.push(filing.entry.at.clone());
                    }
                    Ok(())
                })();
                match outcome {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        let mut rollback_failed = Vec::new();
                        for path in written {
                            if std::fs::remove_file(&path).is_err() {
                                rollback_failed.push(path.display().to_string());
                            }
                        }
                        if rollback_failed.is_empty() {
                            Err(err)
                        } else {
                            Err(err.with_recovery(format!(
                                "rollback also failed for: {}",
                                rollback_failed.join(", ")
                            )))
                        }
                    }
                }
            }
        }
    }

    /// Gives the tree back, files what is waiting for a person, and hands
    /// down the work this run asked for.
    ///
    /// Everything here happens whether the run finished or failed, which
    /// is why the drive's outcome arrives as a `Result` and is unwrapped
    /// only after the desks are settled: a lease left out and an approval
    /// nobody filed are both worse than the failure that caused them.
    ///
    /// # Errors
    /// Propagates the drive's own outcome, a tree that will not go back,
    /// a waiting item that will not serialise, and whatever a run handed
    /// down reports.
    pub(super) fn conclude(
        &mut self,
        site: Site,
        at: &Assignment,
        ending: Ending<'_>,
    ) -> Result<Dispatched, AxError> {
        let Ending {
            driven,
            mut raised,
            delegates,
        } = ending;
        let Site {
            lease,
            who,
            run_id,
            model,
            ..
        } = site;
        let (addr, model) = (at.addr.clone(), model.id.as_str());
        // The tree goes back whether the run finished or failed. What was
        // committed on its branch survives; what was not, does not.
        if let Some(held) = lease {
            memory::Worktrees::open(&self.city_root)
                .map_err(memory::MemoryError::into_ax)?
                .release(held)
                .map_err(memory::MemoryError::into_ax)?;
        }
        // An item that is waiting belongs in the inbox, not only in the
        // refusal the model saw.
        // Recorded against the run and the address that raised it, not
        // against the city: answering this item later has to be able to
        // find the work it was holding up.
        if self.tainted_arrival {
            // C15's marker bit, set where the reason for it is known. A
            // tainted item takes no policy and no delegate, so a run that
            // began with a stranger's text cannot have its approvals
            // waived by a rule somebody wrote for ordinary work.
            for item in raised.iter_mut() {
                item.tainted = true;
            }
        }
        for item in raised.iter() {
            let value = serde_json::to_value(item).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "record a waiting item",
                    err.to_string(),
                )
            })?;
            let map = value.as_object().cloned().ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "record a waiting item",
                    "an approval item is an object",
                )
            })?;
            self.record_for(
                run_id,
                effect::Line {
                    who: who.to_owned(),
                    addr: addr.clone(),
                    kind: EventKind::ApprovalRequested,
                    data: Payload::new(map)?,
                },
            )?;
        }
        let frozen = driven?;
        let ending = frozen.completion().clone();
        // What it actually did, for the person reading afterwards. The
        // ledger holds the detail; this line is the pointer into it.
        self.note(
            runtime::diagnostics::Level::Effect,
            "runtime::run",
            &format!("dispatch at {} finished on {}", addr.as_str(), model),
        );
        // Who this run handed work to. Started here rather than inside
        // the tool call, because a run is built by this layer and a tool
        // that drove one would be driving a run from inside another
        // run's tool bench. Each child is dispatched at `Delegated`, so
        // the gate refuses the grand-delegate without anybody having to
        // work out their own depth.
        //
        // A cancelled run hands nothing down. The fourth safe point is
        // what makes that reachable: a cancel arriving after the last
        // wave used to have no boundary left to land on, so work asked
        // for by a turn nobody wanted started anyway.
        let handed = match ending {
            kernel::Completion::Cancelled => Vec::new(),
            _ => delegates.borrow_mut().take(),
        };
        for work in handed {
            self.note(
                runtime::diagnostics::Level::Effect,
                "collab::delegate",
                &format!("{} handed work to {}", addr.as_str(), work.room.as_str()),
            );
            // Carried rather than defaulted, for the reason `knock`
            // states next to its own `budget`: work handed down is the
            // same piece of work, so it is done under the same ceiling.
            // Defaulting here told a delegate its budget was zero while
            // its parent had been told the truth.
            let child = self.dispatch_in(
                Assignment {
                    addr: work.room,
                    session: None,
                    effort: None,
                    mode: at.mode,
                    budget: at.budget,
                    parent: Some(run_id),
                },
                work.task,
                work.goal,
            )?;
            self.deliver_handback(&addr, &child)?;
        }
        Ok(Dispatched {
            run: run_id,
            addr,
            who,
            completion: ending,
        })
    }
}

#[cfg(test)]
mod tests;
