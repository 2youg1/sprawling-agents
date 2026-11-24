// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::Arc;

use agent_protocols::{AcpSession, Answer, Consented, HarnessProcess, Listener};
use kernel::event::Who;
use kernel::{Address, AxError, Completion, Ledger, Locator, Payload, RunId, TimeMs};
use runtime::run::{Charter, Conclusion, HarnessRun};

use super::lane::DriveContext;
use crate::worker::workbench::{Lending, Placing, lend_tree, tree_scope};
use crate::worker::{Stamping, recording::Notes};

mod turn;

use turn::{Turning, answered, take_the_turn};

pub(crate) type Prompting =
    Box<dyn FnMut(&str, &mut Listener<'_>) -> Result<Answer, AxError> + Send>;

pub(crate) struct Started {
    pub(crate) prompting: Prompting,
    pub(crate) version: Option<String>,
}

pub(crate) type StartHarness =
    Arc<dyn Fn(&Consented, &Path) -> Result<Started, AxError> + Send + Sync>;

pub(in crate::worker) fn on_this_machine() -> StartHarness {
    Arc::new(
        |agent: &Consented, cwd: &Path| -> Result<Started, AxError> {
            let (process, session) = HarnessProcess::start(agent, cwd)?;
            let version = session.introduced().version.clone();
            let mut seated = Seated {
                session,
                _process: process,
            };
            Ok(Started {
                prompting: Box::new(move |text: &str, listener: &mut Listener<'_>| {
                    seated.prompt(text, listener)
                }),
                version,
            })
        },
    )
}

struct Seated {
    session: AcpSession<ChildStdin>,
    _process: HarnessProcess,
}

impl Seated {
    fn prompt(&mut self, text: &str, listener: &mut Listener<'_>) -> Result<Answer, AxError> {
        self.session.prompt(text, listener)
    }
}

pub(in crate::worker) struct Chartered {
    pub(in crate::worker) run: RunId,
    pub(in crate::worker) who: String,
    pub(in crate::worker) addr: Address,
    pub(in crate::worker) task: String,
    pub(in crate::worker) goal: String,
    pub(in crate::worker) job: Locator,
    pub(in crate::worker) parent: Option<RunId>,
    pub(in crate::worker) predecessor: Option<RunId>,
    pub(in crate::worker) dispatched_by: Who,
    pub(in crate::worker) policy: kernel::RunPolicy,
}

impl Chartered {
    fn charter(&self) -> Charter<'_> {
        Charter {
            run: self.run,
            who: &self.who,
            addr: &self.addr,
            task: &self.task,
            goal: &self.goal,
            job: &self.job,
            parent: self.parent,
            predecessor: self.predecessor,
            dispatched_by: &self.dispatched_by,
            skills: &[],
            policy: self.policy,

            naming: None,

            opening: None,

            effort: None,
            generation: None,
            routing: None,

            agent: None,
            pictures: &[],
        }
    }
}

pub(in crate::worker) struct HarnessHalf {
    pub(in crate::worker) harness: Consented,
    pub(in crate::worker) start: StartHarness,
    pub(in crate::worker) chartered: Chartered,
    pub(in crate::worker) building: city::Building,

    pub(in crate::worker) entropy_allow: kernel::EntropyAllowlist,

    pub(in crate::worker) prompt: String,
    pub(in crate::worker) handoff: runtime::handoff::Handoff,
    pub(in crate::worker) city_root: PathBuf,

    pub(in crate::worker) city: kernel::B3Hash,

    pub(in crate::worker) command: Option<kernel::IdemKey>,

    pub(in crate::worker) member: Option<runtime::BacklogId>,

    pub(in crate::worker) ceiling_ms: u64,
    pub(in crate::worker) notes: Notes,

    pub(in crate::worker) staged_at: kernel::Seq,
}

pub(crate) struct HarnessDriven {
    pub(in crate::worker) run: RunId,
    pub(in crate::worker) who: String,
    pub(in crate::worker) building: city::Building,
    pub(in crate::worker) city: kernel::B3Hash,
    pub(in crate::worker) entropy_allow: kernel::EntropyAllowlist,
    pub(in crate::worker) member: Option<runtime::BacklogId>,

    pub(in crate::worker) lease: Option<storage::WorktreeLease>,
    pub(in crate::worker) outcome: Result<Completion, AxError>,
}

pub(crate) fn drive_harness<L: Ledger>(
    half: HarnessHalf,
    ledger: &mut L,
    context: DriveContext,
) -> HarnessDriven {
    let mut lease = None;
    let outcome = drive_turn(&half, &mut lease, ledger, &context);
    HarnessDriven {
        run: half.chartered.run,
        who: half.chartered.who,
        building: half.building,
        city: half.city,
        entropy_allow: half.entropy_allow,
        member: half.member,
        lease,
        outcome,
    }
}

pub(in crate::worker) fn harness_provenance(
    city: kernel::B3Hash,
    run: RunId,
    addr: &Address,
    predecessor: Option<RunId>,
) -> storage::Provenance {
    let signed = storage::Provenance::new(
        run,
        addr.clone(),
        city,
        storage::ModelChoice {
            id: String::new(),
            effort: None,
        },
    );
    match predecessor {
        Some(predecessor) => signed.succeeding(predecessor),
        None => signed,
    }
}

fn drive_turn<L: Ledger>(
    half: &HarnessHalf,
    lease: &mut Option<storage::WorktreeLease>,
    ledger: &mut L,
    context: &DriveContext,
) -> Result<Completion, AxError> {
    let clock = &*context.clock;
    let of = harness_provenance(
        half.city,
        half.chartered.run,
        &half.chartered.addr,
        half.chartered.predecessor,
    );
    let tree = lend_tree(
        &Lending {
            addr: &half.chartered.addr,
            building: &half.building,
            run_id: half.chartered.run,
            who: &half.chartered.who,
            of: &of,
            entropy_allow: &half.entropy_allow,
        },
        &Placing {
            city_root: &half.city_root,
            city: half.city,
            clock,
        },
        &mut Stamping {
            ledger: &mut *ledger,
            command: half.command,
            clock,
        },
    )?;
    let root = tree.path().to_path_buf();
    *lease = Some(tree);
    let Started {
        mut prompting,
        version,
    } = (half.start)(&half.harness, &root)?;

    let agent = kernel::event::record::AgentRunIdentity {
        id: half.harness.entry().id.as_str().to_owned(),
        version,
        launch_digest: half.harness.entry().launch.digest(),
    };
    let mut now = || clock.now();
    let charter = runtime::run::Charter {
        agent: Some(&agent),
        ..half.chartered.charter()
    };
    let run = HarnessRun::open(charter, &mut *ledger, &mut now)?;
    let deadline = TimeMs::new(clock.now()?.value().saturating_add(half.ceiling_ms));
    let turning = Turning {
        run,
        ledger: &mut *ledger,
        deadline,
        failed: None,
    };
    let (run, prompted, failed) = take_the_turn(half, turning, &mut prompting, context);

    drop(prompting);
    let failure = match (prompted, failed) {
        (Ok(answer), None) => {
            let scope = tree_scope(&half.building);
            match commit(
                &HarnessCheckpoint {
                    city: &half.city_root,
                    root: &root,
                    scope: &scope,
                    of: &of,
                    entropy_allow: &half.entropy_allow,
                },
                context,
            ) {
                Ok(committed) => {
                    return run.conclude(
                        &mut *ledger,
                        Conclusion {
                            committed,
                            answered: answered(answer),
                            handoff: &half.handoff,
                        },
                        clock.now()?,
                    );
                }
                Err(uncommitted) => {
                    if let Some(report) = uncommitted.refusal() {
                        run.refusal(&mut *ledger, report, clock.now()?)?;
                    }
                    uncommitted.into_cause()
                }
            }
        }

        (Ok(_) | Err(_), Some(booking)) => booking,
        (Err(session), None) => session,
    };
    run.abandon(&mut *ledger, &half.handoff, Some(&failure), clock.now()?)?;
    Err(failure)
}

struct HarnessCheckpoint<'a> {
    city: &'a Path,
    root: &'a Path,
    scope: &'a [String],
    of: &'a storage::Provenance,
    entropy_allow: &'a kernel::EntropyAllowlist,
}

fn commit(
    plan: &HarnessCheckpoint<'_>,
    context: &DriveContext,
) -> Result<Payload, runtime::CheckpointFailure> {
    let mut own = storage::ProjectRepository::run_checkpoint(
        plan.city,
        plan.root,
        plan.of.run(),
        plan.of.actor(),
    )
    .map_err(runtime::CheckpointFailure::from)?
    .with_entropy_allowlist(plan.entropy_allow.clone());
    let committed = own
        .wave_pre(plan.scope, context.clock.now()?, plan.of)
        .map_err(runtime::CheckpointFailure::from);

    let closed = own.close_writer().map_err(runtime::CheckpointFailure::from);
    match committed {
        Ok(payload) => {
            closed?;
            Ok(payload)
        }
        Err(refused) => Err(refused),
    }
}
