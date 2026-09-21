// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::{AxError, Completion, RunId};

use crate::effect;

use super::super::{
    Assignment, Dispatched, Ending, Handover, Landed, Owed, Owing, RunWorker, Site, held,
};
use super::desks::write_plan;

/// The action an `AxError` names when one of the two hand-down desks
/// cannot be read. Written here rather than at the call site, where the
/// arm it sits in has no room for it.
const DELEGATE_DESK: &str = "read the delegate desk";
const SUCCESSION_DESK: &str = "read the succession desk";

/// Who raised what, and where. The four travel together because an
/// item filed without any one of them cannot be found again by the
/// person answering it.
pub(super) struct Filing<'a> {
    pub(super) run_id: RunId,
    pub(super) addr: &'a kernel::Address,
    pub(super) who: &'a str,
    /// Whether the run began from content that came in from outside.
    pub(super) tainted: bool,
}

mod filing;

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
                        // The room table decides what a delivery means,
                        // including a room whose queue is out with a
                        // run and a room that sheds: the refusal is
                        // spelled once, there, because the line is
                        // already on the ledger and a line no queue
                        // holds is a torn city.
                        self.rooms.deliver(signal)?;
                        // Somebody was spoken to. Whether that starts a run
                        // is decided in one place, so that the two ways of
                        // reaching a resident stay one decision.
                        self.knock(signal, &at.addr, at.mode)?;
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
                // The rollback is best effort by construction: the
                // error a person must see is the one from the write,
                // and a rollback that also failed cannot be reported
                // here without replacing it.
                write_plan(&path, &text).inspect_err(|_| match before {
                    Some(text) => {
                        drop(std::fs::write(&path, text));
                    }
                    None => {
                        drop(std::fs::remove_file(&path));
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
                            Err(err.rewrite_recovery(format!(
                                "rollback also failed for: {}",
                                rollback_failed.join(", ")
                            )))
                        }
                    }
                }
            }
        }
    }

    /// Files what is waiting for a person, hands down the work this run
    /// asked for, and discharges what the city owed it.
    ///
    /// Everything before the drive's outcome is read happens whether the
    /// run finished or failed: an approval nobody filed is worse than
    /// the failure that caused it. What this run borrowed went back
    /// earlier still, in `return_borrowed`.
    ///
    /// # Errors
    /// Propagates the drive's own outcome, a waiting item that will not
    /// serialise, and whatever starting the work this run handed down
    /// reports.
    pub(in crate::assembly) fn conclude(
        &mut self,
        site: Site,
        at: &Assignment,
        ending: Ending<'_>,
    ) -> Result<Landed, AxError> {
        let Ending {
            driven,
            mut raised,
            delegates,
            succession,
            owing,
        } = ending;
        let Site {
            who,
            run_id,
            model,
            mut adapter,
            ..
        } = site;
        let (addr, model) = (at.addr.clone(), model.id.as_str());
        self.file_the_waiting(
            &Filing {
                run_id,
                addr: &addr,
                who: &who,
                tainted: at.tainted,
            },
            &mut raised,
        )?;
        let frozen = driven?;
        let ending = frozen.completion().clone();
        // What the model saw, beside the room it worked in. The freeze
        // is already on the ledger, so a room that will not take the
        // file is noted rather than turned into a failed run: the
        // transcript is the run's copy, not its record.
        match frozen
            .transcript()
            .and_then(|transcript| transcript.materialise(&mut self.cas, &self.city_root, &addr))
        {
            Ok(record) => self.note(
                runtime::diagnostics::Level::Effect,
                "runtime::transcript",
                &format!(
                    "{} written, {} spans redacted",
                    record.address.as_str(),
                    record.redacted
                ),
            ),
            Err(err) => self.note(
                runtime::diagnostics::Level::Refuse,
                "runtime::transcript",
                &format!("transcript not written: {err}"),
            ),
        }
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
            Completion::Cancelled => Vec::new(),
            Completion::Done(_) | Completion::Limit => held(delegates, DELEGATE_DESK)?.take(),
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
            //
            // Into a lane, and the handback follows when the child
            // lands rather than here: a parent that drove each of its
            // children to the end held this thread, and with it every
            // other lane's writes, for the depth of the whole tree
            // (sprawling-SPEC.md 8-46-2).
            self.dispatch_into_lane(
                Assignment {
                    addr: work.room,
                    session: None,
                    effort: None,
                    mode: at.mode,
                    parent: Some(run_id),
                    succession: None,
                    tainted: at.tainted,
                },
                work.task,
                work.goal,
                owing.child(addr.clone()),
            )?;
        }
        // Who this run asked to replace it: itself, next run. Same
        // address, same work, and the same `parent` - not this run - so
        // the depth is conserved and the successor's table equals this
        // one's. A cancelled run is not succeeded, for the reason above.
        let replaced = match ending {
            Completion::Cancelled => None,
            Completion::Done(_) | Completion::Limit => held(succession, SUCCESSION_DESK)?.take(),
        };
        if let Some(asked) = replaced {
            self.note(
                runtime::diagnostics::Level::Effect,
                "runtime::succession",
                &format!("{} hands over: {}", addr.as_str(), asked.reason),
            );
            // The quality defence: what this run knows, asked of it
            // before it goes, so the successor's answers have something
            // to be compared with. Nobody discovers decay otherwise
            // until the third succession.
            let before = self.probe_before(adapter.as_deref_mut(), &frozen, &who)?;
            let plan = frozen.plan();
            // **The obligation moves with the work.** A successor is the
            // same piece of work carrying on, so whoever was owed the
            // predecessor's ending is owed the successor's, and the
            // handback or the reply happens when the chain ends rather
            // than at each link.
            self.dispatch_into_lane(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    effort: None,
                    mode: at.mode,
                    parent: at.parent,
                    succession: Some(Handover {
                        predecessor: run_id,
                        before,
                    }),
                    tainted: at.tainted,
                },
                plan.task.clone(),
                plan.goal.clone(),
                owing,
            )?;
            return Ok(Landed::Elsewhere);
        }
        self.discharge(
            owing,
            &Dispatched {
                run: run_id,
                addr,
                who,
                completion: ending,
            },
        )
    }

    /// Pays what the city owed the run that has just ended.
    ///
    /// The one place an entrance's obligation is settled, which is why
    /// every entrance can share one dispatch path: what differs between
    /// a person's command, a scheduled job, a knock and a delegate is
    /// this match and nothing else.
    ///
    /// # Errors
    /// Propagates a handback the parent's room will not take.
    fn discharge(&mut self, owing: Owing, done: &Dispatched) -> Result<Landed, AxError> {
        match owing.owed() {
            Owed::Asked => Ok(Landed::Elsewhere),
            // Nobody typed a command for this one, so the history is
            // the only place the reason can appear. A run an operator
            // did not start is the run they most need a reason for.
            Owed::Unasked(because) => {
                let because = because.because();
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "bin::assembly",
                    &format!("a run the city started itself landed, because {because}"),
                );
                Ok(Landed::Elsewhere)
            }
            Owed::Row { addr, node } => Ok(Landed::Row {
                addr: addr.clone(),
                node: node.clone(),
            }),
            Owed::Child { parent } => {
                let parent = parent.clone();
                self.deliver_handback(&parent, done)?;
                Ok(Landed::Elsewhere)
            }
        }
    }
}
