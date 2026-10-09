// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verbs a person sends, and what each one does to the city.

use kernel::TimeMs;
use kernel::event::record::Admittance;
use kernel::{AxCode, AxError};

use crate::guide;
use crate::tuning::tuning_of;

use super::super::{Assignment, Chosen, Credential, Entered, Owing, RunWorker, Stated, Unasked};
use super::unbuilt::Unbuilt;

/// What a Cancel or a Steer is told when no run answers to the id it
/// names.
///
/// Both reach this file only after `Desk::interrupt_for` failed to lift
/// them off the queue, so the subject is the id rather than the verb:
/// the verb is built, and the run is what is missing.
#[derive(Clone, Copy)]
enum Unanswered {
    Cancel,
    Steer,
}

impl Unanswered {
    fn refusal(self, run: kernel::RunId) -> AxError {
        let (action, recovery) = match self {
            Unanswered::Cancel => (
                "cancel a run",
                "no run in flight answers to that id: it has already finished, or it never started",
            ),
            Unanswered::Steer => (
                "steer a run",
                "no run in flight answers to that id: steer one while it runs, or dispatch a new one",
            ),
        };
        AxError::failure(AxCode::InvalidArgs, action, run.to_string()).with_recovery(recovery)
    }
}

/// The serving binary holds the remote door and answers its four verbs
/// before they reach the desk (`crates/sprawling/spec/Outside.lean`
/// §8-140), so one arriving here came by a path that skipped it.
fn the_door_is_held_elsewhere() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "work the remote door",
        "the run worker does not hold the remote door",
    )
    .with_recovery("send it to the city's own listener, which serves the remote door")
}

/// A verb the serving binary carries out on its own listener, ahead of
/// the desk (`crates/sprawling/spec/Serving.lean`), so one arriving here
/// came by a path that skipped it.
#[derive(Clone, Copy)]
enum OnTheListener {
    CloseCity,
    AddAgent,
    ForgetDevice,
}

impl OnTheListener {
    fn refusal(self) -> AxError {
        let (action, subject) = match self {
            OnTheListener::CloseCity => {
                ("close the city", "the run worker does not close the city")
            }
            OnTheListener::AddAgent => (
                "add an ACP agent",
                "the run worker does not hold the offers a consent names",
            ),
            OnTheListener::ForgetDevice => (
                "forget a paired browser",
                "the run worker does not hold the paired browsers",
            ),
        };
        AxError::failure(AxCode::ToolUnavailable, action, subject)
            .with_recovery("send it to the city's own listener, which carries it out")
    }
}

/// The serving binary answers host privacy operations on its own listener
/// (`crates/sprawling/spec/Privacy/Service.lean`), so one arriving here came
/// by a path that skipped it.
fn privacy_is_served_elsewhere() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "carry out a privacy operation",
        "the run worker does not change host privacy controls",
    )
    .with_recovery("send it to the city's own listener, which serves the privacy page")
}

/// The task and the goal a person typed, travelling together.
///
/// An empty goal is not a missing field: no job file is written and the
/// prefix tells the model a person is at the other end, which is
/// exactly the shape a single sentence typed into the composer has.
pub(in crate::worker) struct Asked {
    pub(in crate::worker) task: String,
    pub(in crate::worker) goal: String,
}

impl RunWorker {
    /// Takes into a lane the work a person asked for.
    ///
    /// The session and the effort travel into the dispatch rather than
    /// being spent here, because opening a room and writing its
    /// configuration are the first two things this city puts on disk,
    /// and nothing may be written until the city has agreed to take the
    /// work. Doing either here would leave the entrances that dispatch
    /// without a person holding an older, wrong rule.
    ///
    /// # Errors
    /// Propagates every refusal a dispatch can owe before it costs
    /// anything.
    fn dispatch_asked(
        &mut self,
        at: Assignment,
        asked: Asked,
        reply: wire::Reply,
    ) -> Result<(), AxError> {
        self.room_for_new_work()?;
        self.dispatch_into_lane(at, asked.task, asked.goal, Owing::asked(reply))
            .map(drop)
    }

    /// Carries out one command, with the address its refusal goes back
    /// to.
    ///
    /// The reply is a parameter rather than something the caller holds
    /// onto, because one verb outlives this call: a `Dispatch` starts a
    /// run in a lane and returns, so a refusal that arrives after the
    /// drive has to know where to go (`crates/sprawling/Spec.lean` §8-46-2).
    pub(in crate::worker) fn run_command(
        &mut self,
        command: wire::Command,
        reply: wire::Reply,
    ) -> Result<(), AxError> {
        match command {
            wire::Command::Dispatch {
                addr,
                task,
                goal,
                policy,
                session,
                effort,
                model,
                ..
            } => self.dispatch_asked(
                Assignment {
                    // Read from the room's own history rather than sent by the page: a session
                    // that branched off another said so once, at the moment it began.
                    origin: self.origins.get(&addr),
                    // The session's last change, on the ledger, outranks
                    // the copy the page sent (`crates/sprawling/spec/Accounting/Worker.lean` §8-133).
                    policy: self.origins.policy(&addr).unwrap_or(policy),
                    addr,
                    session,
                    effort,
                    model,
                    parent: None,
                    succession: None,
                    taint: kernel::TaintSet::empty(),
                    dispatched_by: kernel::event::Who::Person,
                },
                Asked { task, goal },
                reply,
            ),
            wire::Command::Wake {
                source,
                subject,
                body,
                ..
            } => self.wake(&source, &subject, &body),
            wire::Command::ConnectToolkit { toolkit, .. } => self.connect_toolkit(&toolkit),
            wire::Command::ConfigureBuilding {
                addr,
                sandbox,
                mcp,
                desktop,
                context_second_threshold,
                ..
            } => self.configure_building(
                &addr,
                super::configure::Reconfiguration {
                    sandbox: sandbox.as_ref(),
                    mcp: mcp.as_deref(),
                    desktop: desktop.as_deref(),
                    context_second_threshold,
                },
            ),
            wire::Command::ProbeEndpoint {
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
                tuning: tuning_of(tuning)?,
            }),
            wire::Command::AttachEndpoint {
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
                    tuning: tuning_of(tuning)?,
                },
                &admit,
            ),
            wire::Command::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                input,
                ..
            } => self.select_model(
                Chosen {
                    endpoint: endpoint.as_str().to_owned(),
                    model,
                    tag,
                },
                Stated::new(context_tokens, max_output_tokens, input),
            ),
            wire::Command::PutSecret { realm, name, value } => self.put_secret(
                &kernel::SecretRef::new(&realm, &name)?,
                value,
                crate::worker::credentials::signing::Arrival::Enrolment,
            ),
            wire::Command::ForgetSecret(forgetting) => self.forget_secret(&forgetting.reference),
            wire::Command::CloseCity(_) => Err(OnTheListener::CloseCity.refusal()),
            wire::Command::AddAgent(_) => Err(OnTheListener::AddAgent.refusal()),
            wire::Command::AgentLogin(it) => Err(Unbuilt::AgentLogin(it.agent).not_built()),
            wire::Command::ForgetDevice(_) => Err(OnTheListener::ForgetDevice.refusal()),
            wire::Command::CreateBuilding { addr, template, .. } => {
                self.create_building(addr, template.as_str())
            }
            wire::Command::RemoveBuilding { addr, .. } => self.remove_building(&addr),
            // The person's entrance, so the answerer is a human; a delegate has its own tool.
            wire::Command::Approve { item, verdict, .. } => {
                self.answer_approval(&item, verdict, &kernel::Answerer::Human)
            }
            wire::Command::SetAutonomy {
                scope, autonomy, ..
            } => self.set_autonomy(&scope, autonomy),
            wire::Command::HandOff { item, .. } => Err(Unbuilt::HandOff(&item).not_built()),
            // Not recorded: the person's own layer, which no run observes and a copied city
            // must not carry to another machine.
            wire::Command::PutPreferences { patch, .. } => crate::person::put(patch),
            wire::Command::PutShelved { name, .. } => Err(Unbuilt::PutShelved(name).not_built()),
            wire::Command::Pursue { addr, step, .. } => self.set_pursuit(&addr, step),
            wire::Command::OpenSession {
                addr, carry, from, ..
            } => self.open_session(&addr, carry, from),
            wire::Command::NameSession(it) => self.name_session(&it.room, it.began, it.name),
            wire::Command::ChangeRunPolicy(it) => self.change_run_policy(&it.room, it.policy),
            wire::Command::PutDocument {
                which,
                ref base,
                ref body,
                ..
            } => self.put_document(which, base, body),
            wire::Command::PutIdentity {
                ref card, ref base, ..
            } => self.put_identity(card, base),
            wire::Command::PutRules(write) => {
                self.put_rules(&write.building, &write.base, &write.body)
            }
            wire::Command::RestoreFile { ref at, point, .. } => self.take_back(at, point),
            wire::Command::PutGuide { progress, .. } => guide::put(&self.city_root, &progress),
            wire::Command::ConfigureCity(settings) => self.configure_city(settings),
            wire::Command::PutSpine {
                building: ref at,
                which,
                ref base,
                ref body,
                ..
            } => self.put_spine(at, which, base, body),
            wire::Command::PutRange(ref write) => self.put_range(write),
            wire::Command::DecideProposals(ref decisions) => self.decide_proposals(decisions),
            wire::Command::Halt { scope, .. } => self.set_admission(&scope, Admittance::Halted),
            wire::Command::Reveal { at, .. } => (self.reveal)(&self.city_root, &at),
            wire::Command::RestoreDiscard {
                ref restoration, ..
            } => self.restore_discard(restoration),
            wire::Command::DoctorInstall { ref item, .. } => self.doctor_install(item),
            wire::Command::DoctorRefresh { .. } => {
                self.look_at_this_machine();
                Ok(())
            }
            wire::Command::Release { scope, .. } => {
                self.set_admission(&scope, Admittance::Released)
            }
            // Cancel and Steer have a second door. `Desk::interrupt_for` lifts them off the
            // queue at the next safe point of the run they name, so arriving here means no run
            // answered - which is what the refusal says, instead of naming the verb.
            wire::Command::Cancel { run, .. } => Err(Unanswered::Cancel.refusal(run)),
            wire::Command::Steer { run, .. } => Err(Unanswered::Steer.refusal(run)),
            wire::Command::OpenRemoteDoor { .. }
            | wire::Command::ReplaceCityKey { .. }
            | wire::Command::ConfirmRemoteDoor { .. }
            | wire::Command::CloseRemoteDoor { .. } => Err(the_door_is_held_elsewhere()),
            wire::Command::PrivacyOperation { .. } => Err(privacy_is_served_elsewhere()),
            // Verbs the wire spells and this city cannot perform, one arm each and no
            // catch-all, so a Command added without an executor stops the build here.
            wire::Command::BatchByBuilding { addr, .. } => {
                Err(Unbuilt::BatchByBuilding(&addr).not_built())
            }
            // The handshake is where a peer proves who it is: `Hello`
            // carries the pairing token and `wire::server` judges it
            // before any command is read. A second door for the same
            // question would be a second authority on it.
            wire::Command::Auth { .. } => Err(Unbuilt::Auth.not_built()),
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
    /// **Every job due in the window is started, and one that cannot be
    /// is noted rather than swallowing the rest.** Returning on the first
    /// refusal after the window had closed would let one mistyped
    /// building address make every other job due that minute disappear
    /// with nothing recorded (`crates/sprawling/Spec.lean` §8-46-2).
    ///
    /// # Errors
    /// Propagates the schedule's own refusal to parse. A job that cannot
    /// be started does not fail this call: nobody is waiting on the
    /// answer, and the jobs behind it are owed their run.
    pub fn tick(&mut self, now: TimeMs) -> Result<u32, AxError> {
        let schedule = city::Schedule::load(&self.city_root)?;
        let due = schedule.due_after(self.last_tick, now);
        let mut started: u32 = 0;
        for (addr, task, goal) in due {
            if self
                .start_unasked(addr, task, goal, Unasked::Schedule)
                .is_some()
            {
                started = started.saturating_add(1);
            }
        }
        // The window closes once it has been walked, never before: a
        // schedule read that stopped halfway would leave `last_tick` past
        // jobs no lane ever took.
        self.last_tick = now;
        Ok(started)
    }

    /// Replaces the file manager, so a test can see what a reveal asks
    /// for without starting a program on the host.
    #[cfg(test)]
    pub(in crate::worker) fn reveal_with(
        &mut self,
        reveal: fn(&std::path::Path, &kernel::Address) -> Result<(), AxError>,
    ) {
        self.reveal = reveal;
    }
}
