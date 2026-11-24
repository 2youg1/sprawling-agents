// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Completion, RunId};

use crate::effect;
use crate::worker::waking::handing::GraphAfter;

use super::super::{Assignment, Dispatched, Ending, Handover, Landed, RunWorker, Site, held};

const SUCCESSION_DESK: &str = "read the succession desk";

pub(super) struct Filing<'a> {
    pub(super) run_id: RunId,
    pub(super) addr: &'a kernel::Address,
    pub(super) who: &'a str,

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
        self.carry_out_landing(at, run, then, chain)
    }

    pub(in crate::worker) fn carry_out_landing(
        &mut self,
        at: &Assignment,
        run: RunId,
        then: effect::Then,
        chain: &super::super::KnockChain,
    ) -> Result<(), AxError> {
        match then {
            effect::Then::Nothing => Ok(()),
            effect::Then::Deliver(signals) => {
                let knocks_mark = self.doorstep.knocks.len();
                let outcome = (|| {
                    for signal in signals {
                        let arrival = self.arrive(signal)?;

                        self.knock(arrival.signal(), &at.addr, at.policy, chain)?;

                        let who = arrival.signal().from().to_owned();
                        self.record_landing(arrival, run, &who)?;
                    }
                    Ok(())
                })();
                match outcome {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        self.doorstep.knocks.truncate(knocks_mark);
                        Err(err)
                    }
                }
            }

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

        self.end_hand_over(run_id, GraphAfter::of(&driven))?;
        let frozen = driven?;
        let ending = frozen.completion().clone();

        self.note(
            runtime::diagnostics::Level::Effect,
            "runtime::run",
            &format!("dispatch at {} finished on {}", addr.as_str(), model),
        );

        let replaced = match ending {
            Completion::Cancelled => None,
            Completion::Done(_) | Completion::Limit => held(succession, SUCCESSION_DESK)?.take(),
        };

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

            let before = self.probe_before(adapter.as_mut(), &frozen, &who)?;
            if let Some(door) = adapter {
                self.warm.keep(addr.clone(), door);
            }
            let plan = frozen.plan();

            self.dispatch_into_lane(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    resume: None,
                    pictures: Vec::new(),
                    effort: None,
                    difficulty: None,
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
