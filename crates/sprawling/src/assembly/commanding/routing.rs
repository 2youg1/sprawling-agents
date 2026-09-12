// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verbs a person sends, and what each one does to the city.

use kernel::TimeMs;
use kernel::{AxCode, AxError};

use super::super::{
    Assignment, Ceilings, Chosen, Credential, Entered, HALTED, RELEASED, RunWorker, mode_of,
    not_built, tuning_of,
};

/// What a Cancel or a Steer is told when no run answers to the id it
/// names.
///
/// Both reach this file only after `Desk::interrupt_for` failed to lift
/// them off the queue, so the subject is the id rather than the verb:
/// the verb is built, and the run is what is missing.
fn no_run_answers(action: &'static str, run: kernel::RunId, recovery: &'static str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, action, run.to_string()).with_recovery(recovery)
}

impl RunWorker {
    /// Carries out one command, with the address its refusal goes back
    /// to.
    ///
    /// The reply is a parameter rather than something the caller holds
    /// onto, because one verb outlives this call: a `Dispatch` starts a
    /// run in a lane and returns, so a refusal that arrives after the
    /// drive has to know where to go (sprawling-SPEC.md 8-46-2).
    pub(in crate::assembly) fn run_command(
        &mut self,
        command: channels::Command,
        reply: channels::Reply,
    ) -> Result<(), AxError> {
        match command {
            channels::Command::Dispatch {
                addr,
                task,
                goal,
                mode,
                session,
                effort,
                ..
            } => {
                // An empty goal is not a missing field: no job file is
                // written and the prefix tells the model a person is at
                // the other end, which is exactly the shape a single
                // sentence typed into the composer has.
                //
                // The session and the effort travel into the dispatch
                // rather than being spent here, because opening a room
                // and writing its configuration are the first two things
                // this city puts on disk, and nothing may be written
                // until the city has agreed to take the work. Doing
                // either here would leave the entrances that dispatch
                // without a person holding an older, wrong rule.
                self.dispatch_into_lane(
                    Assignment {
                        addr,
                        session,
                        effort,
                        mode: mode_of(&mode),
                        parent: None,
                        succession: None,
                    },
                    task,
                    goal,
                    reply,
                )
                .map(drop)
            }
            channels::Command::Wake {
                source,
                subject,
                body,
                ..
            } => self.wake(&source, &subject, &body),
            channels::Command::ConnectToolkit { toolkit, .. } => self.connect_toolkit(&toolkit),
            channels::Command::ConfigureBuilding {
                addr,
                sandbox,
                mcp,
                desktop,
                ..
            } => {
                self.configure_building(&addr, sandbox.as_ref(), mcp.as_deref(), desktop.as_deref())
            }
            channels::Command::ProbeEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                tuning,
                ..
            } => self.probe_endpoint(Entered {
                name: name.as_str().to_owned(),
                base_url,
                dialect,
                credential: Credential::entered(secret, auth_header),
                tuning: tuning_of(tuning),
            }),
            channels::Command::AttachEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                admit,
                tuning,
                ..
            } => self.attach_endpoint(
                Entered {
                    name: name.as_str().to_owned(),
                    base_url,
                    dialect,
                    credential: Credential::entered(secret, auth_header),
                    tuning: tuning_of(tuning),
                },
                &admit,
            ),
            channels::Command::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                ..
            } => self.select_model(
                Chosen {
                    endpoint: endpoint.as_str().to_owned(),
                    model,
                    tag,
                },
                Ceilings {
                    context_tokens,
                    max_output_tokens,
                },
            ),
            channels::Command::PutSecret { realm, name, value } => {
                self.put_secret(realm, name, value)
            }
            channels::Command::Login { provider, step, .. } => self.login(provider.as_str(), step),
            channels::Command::CreateBuilding { addr, template, .. } => {
                self.create_building(addr, template.as_str())
            }
            channels::Command::Approve { item, verdict, .. } => {
                // The control surface is the person's entrance, so the
                // answerer is a human here by construction. A resident
                // answering as a delegate arrives with the tool that
                // lets it, and takes the same door.
                self.answer_approval(&item, verdict, &kernel::Answerer::Human)
            }
            channels::Command::SetAutonomy {
                scope, autonomy, ..
            } => self.set_autonomy(&scope, autonomy),
            channels::Command::Pursue { addr, step, .. } => self.set_pursuit(&addr, step),
            channels::Command::Fork {
                run, at_seq, addr, ..
            } => self.fork(run, at_seq, addr).map(|_| ()),
            channels::Command::PutDocument {
                which, ref body, ..
            } => self.put_document(which, body),
            channels::Command::Halt { scope, .. } => self.set_admission(&scope, HALTED),
            channels::Command::Reveal { at, .. } => crate::revealing::reveal(&self.city_root, &at),
            channels::Command::DoctorInstall { ref item, .. } => self.doctor_install(item),
            channels::Command::DoctorRefresh { .. } => {
                self.look_at_this_machine();
                Ok(())
            }
            channels::Command::Release { scope, .. } => self.set_admission(&scope, RELEASED),
            // Cancel and Steer have a second door. `Desk::interrupt_for`
            // lifts them off the queue at the next safe point of the run
            // they name, so arriving here means no run answered - which
            // is what the refusal says, instead of naming the verb.
            channels::Command::Cancel { run, .. } => Err(no_run_answers(
                "cancel a run",
                run,
                "no run in flight answers to that id: it has already finished, or it never started",
            )),
            channels::Command::Steer { run, .. } => Err(no_run_answers(
                "steer a run",
                run,
                "no run in flight answers to that id: steer one while it runs, or dispatch a new one",
            )),
            // Six verbs the wire spells and this city cannot perform.
            // Answered one at a time rather than by a catch-all, so that
            // a Command added without an executor stops the build here:
            // `channels::Command` is deliberately not `non_exhaustive`,
            // and this match is what that decision buys.
            channels::Command::Takeover { run, .. } => Err(not_built(
                "take over a run",
                run.to_string(),
                "steer the run instead; taking the wheel from it is not built",
            )),
            channels::Command::Rollback { checkpoint, .. } => Err(not_built(
                "roll a checkpoint back",
                checkpoint.to_string(),
                "the checkpoint stands and its contents are readable; undoing it is not built",
            )),
            channels::Command::CreatePolicy { from_item, .. } => Err(not_built(
                "turn an answer into a policy",
                from_item.as_str().to_owned(),
                "answer each request as it arrives; standing policies are not built",
            )),
            channels::Command::Attach { upload, .. } => Err(not_built(
                "attach an upload to a run",
                upload.as_str().to_owned(),
                "paste the text into the task instead; attaching a file is not built",
            )),
            channels::Command::BatchByBuilding { addr, .. } => Err(not_built(
                "run a building's work as one batch",
                addr.as_str().to_owned(),
                "dispatch the rooms one at a time; batching a building is not built",
            )),
            // The handshake is where a peer proves who it is: `Hello`
            // carries the pairing token and `channels::server` judges it
            // before any command is read. A second door for the same
            // question would be a second authority on it.
            channels::Command::Auth { .. } => Err(not_built(
                "authenticate over the command channel",
                "Auth".to_owned(),
                "the pairing token is proved in the handshake, not in a command",
            )),
        }
    }

    /// Starts whatever the schedule says should have started since the
    /// last tick, and returns how many runs that was.
    ///
    /// `now` arrives as a parameter, so a test drives a year of
    /// schedule in three calls and the city behaves the same way it
    /// would over a real year. A fresh worker begins owing from the
    /// moment it opened: a city that was off does not spend its first
    /// minute running yesterday, and the ledger says when it woke.
    ///
    /// # Errors
    /// Propagates the schedule's own refusal to parse, and the first
    /// dispatch that fails - a scheduled run that cannot start is not
    /// swallowed just because nobody typed it.
    pub(crate) fn tick(&mut self, now: TimeMs) -> Result<u32, AxError> {
        let schedule = city::Schedule::load(&self.city_root)?;
        let due = schedule.due_after(self.last_tick, now);
        self.last_tick = now;
        let mut started: u32 = 0;
        for (addr, task, goal) in due {
            self.dispatch(addr, task, goal)?;
            started = started.saturating_add(1);
        }
        Ok(started)
    }
}
