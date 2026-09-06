// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::{AxCode, AxError, EventKind};
use kernel::{Payload, RunId};

use crate::effect;

use super::super::{Assignment, Dispatched, Ending, RunWorker, Site, new_inbox};
use super::desks::write_plan;

impl RunWorker {
    pub(in crate::assembly) fn settle(
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
    pub(in crate::assembly) fn conclude(
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
