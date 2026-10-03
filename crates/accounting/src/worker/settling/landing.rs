// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::{AxError, Completion, RunId};

use crate::effect;
use crate::worker::waking::handing::GraphAfter;

use super::super::{Assignment, Dispatched, Ending, Handover, Landed, RunWorker, Site, held};

/// The action an `AxError` names when the succession desk cannot be
/// read. Written here rather than at the call site, where the
/// arm it sits in has no room for it.
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

mod discharging;
mod filing;

impl RunWorker {
    pub(in crate::worker) fn settle(
        &mut self,
        at: &Assignment,
        run: RunId,
        landing: effect::Landing,
        chain: &super::super::KnockChain,
    ) -> Result<(), AxError> {
        let then = landing.record(&mut |line: effect::Line| self.record_for(run, line))?;
        self.carry_out_landing(at, then, chain)
    }

    /// Carries out what a landing's lines, already on the ledger, ask of
    /// the city outside it. A caller that must act between the two — the
    /// plan desk closes its claims once their closing lines are written —
    /// records the landing itself and then calls this.
    pub(in crate::worker) fn carry_out_landing(
        &mut self,
        at: &Assignment,
        then: effect::Then,
        chain: &super::super::KnockChain,
    ) -> Result<(), AxError> {
        match then {
            effect::Then::Nothing => Ok(()),
            effect::Then::Deliver(signals) => {
                let knocks_mark = self.doorstep.knocks.len();
                let outcome = (|| {
                    for signal in &signals {
                        // The room table decides what a delivery means,
                        // including a room whose queue is out with a
                        // run and a room that sheds: the refusal is
                        // spelled once, there, because the line is
                        // already on the ledger and a line no queue
                        // holds is a torn city.
                        self.collaborating.rooms.deliver(signal)?;
                        // Somebody was spoken to. Whether that starts a run
                        // is decided in one place, so that the two ways of
                        // reaching a resident stay one decision. The
                        // speaking run's place in the conversation rides on,
                        // so the knock this queues is one hop further in.
                        self.knock(signal, &at.addr, at.policy, chain)?;
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
                        self.doorstep.knocks.truncate(knocks_mark);
                        Err(err)
                    }
                }
            }
            // The replacement is whole or not at all, so a refusal
            // leaves the file at `base` with nothing to roll back.
            effect::Then::Roadmap { path, base, text } => {
                (self.planning.write_plan)(&path, base.as_bytes(), text.as_bytes())
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
    pub(in crate::worker) fn conclude(
        &mut self,
        site: Site,
        at: &Assignment,
        ending: Ending<'_>,
    ) -> Result<Landed, AxError> {
        let Ending {
            driven,
            mut raised,
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
                tainted: !at.taint.is_empty(),
            },
            &mut raised,
        )?;
        // What this run handed down started at the call; anything the
        // city has not seen yet starts now, and a graph the run laid out
        // closes unless it landed Done or Limit (collab D7, D14).
        self.end_hand_over(run_id, GraphAfter::of(&driven))?;
        let frozen = driven?;
        let ending = frozen.completion().clone();
        // What it actually did, for the person reading afterwards. The
        // ledger holds the detail; this line is the pointer into it.
        self.note(
            runtime::diagnostics::Level::Effect,
            "runtime::run",
            &format!("dispatch at {} finished on {}", addr.as_str(), model),
        );
        // Who this run asked to replace it: itself, next run. Same
        // address, same work, and the same `parent` - not this run - so
        // the depth is conserved and the successor's table equals this
        // one's. A cancelled run is not succeeded, for the reason above.
        let replaced = match ending {
            Completion::Cancelled => None,
            Completion::Done(_) | Completion::Limit => held(succession, SUCCESSION_DESK)?.take(),
        };
        // The chain is as long as it may be: no successor is dispatched
        // and whoever asked hears why. The refusal is noted as well as
        // handed back, because an obligation with nobody behind it
        // reaches the diagnostics log and nothing else
        // (`crates/sprawling/Spec.lean` §8-46-12). The run itself still lands below,
        // so refusing a successor does not also take this run's ending
        // off the ledger.
        let onward = match replaced {
            None => None,
            Some(asked) => match owing.after_succession() {
                Ok(onward) => Some((asked, onward)),
                Err(refusal) => {
                    self.note(
                        runtime::diagnostics::Level::Refuse,
                        "runtime::succession",
                        &format!(
                            "{} cannot hand over again: {}",
                            addr.as_str(),
                            refusal.subject()
                        ),
                    );
                    self.hand_back(&owing.reply(), refusal);
                    None
                }
            },
        };
        if let Some((asked, onward)) = onward {
            self.note(
                runtime::diagnostics::Level::Effect,
                "runtime::succession",
                &format!("{} hands over: {}", addr.as_str(), asked.reason),
            );
            // The quality defence: what this run knows, asked of it
            // before it goes, so the successor's answers have something
            // to be compared with. Nobody discovers decay otherwise
            // until the third succession.
            let before = self.probe_before(adapter.as_mut(), &frozen, &who)?;
            if let Some(door) = adapter {
                self.warm.keep(addr.clone(), door);
            }
            let plan = frozen.plan();
            // **The obligation moves with the work.** A successor is the
            // same piece of work carrying on, so whoever was owed the
            // predecessor's ending is owed the successor's, and the
            // handback or the reply happens when the chain ends rather
            // than at each link. Its place in the chain moves on with it.
            self.dispatch_into_lane(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    effort: None,
                    model: None,
                    policy: at.policy,
                    origin: None,
                    parent: at.parent,
                    succession: Some(Handover {
                        predecessor: run_id,
                        before,
                    }),
                    taint: at.taint.clone(),
                    dispatched_by: kernel::event::Who::resident(addr.clone())?,
                },
                plan.task.clone(),
                plan.goal.clone(),
                onward,
            )?;
            return Ok(Landed::Elsewhere);
        }
        // What this run sent stays warm for the room's next run
        // (`crates/sprawling/Spec.lean` §8-112).
        if let Some(door) = adapter {
            self.warm.keep(addr.clone(), door);
        }
        self.discharge(
            owing,
            at,
            &Dispatched {
                run: run_id,
                addr,
                who,
                completion: ending,
            },
        )
    }
}
