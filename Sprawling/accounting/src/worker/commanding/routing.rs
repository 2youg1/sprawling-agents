// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::TimeMs;
use kernel::event::record::Admittance;
use kernel::{AxCode, AxError};

use crate::guide;
use crate::tuning::tuning_of;

use super::super::{Assignment, Chosen, Credential, Entered, Owing, RunWorker, Stated, Unasked};
use super::unbuilt::Unbuilt;

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

fn the_door_is_held_elsewhere() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "work the remote door",
        "the run worker does not hold the remote door",
    )
    .with_recovery("send it to the city's own listener, which serves the remote door")
}

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

fn privacy_is_served_elsewhere() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "carry out a privacy operation",
        "the run worker does not change host privacy controls",
    )
    .with_recovery("send it to the city's own listener, which serves the privacy page")
}

pub(in crate::worker) struct Asked {
    pub(in crate::worker) task: String,
    pub(in crate::worker) goal: String,
}

impl RunWorker {

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

                    origin: self.origins.get(&addr),

                    policy: self.origins.policy(&addr).unwrap_or(policy),
                    addr,
                    session,
                    effort,
                    difficulty: None,
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
            wire::Command::ConfigureBuilding(ref settings) => self.configure_building(settings),
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
                admit: ticked,
                named,
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
                &super::super::credentials::Admission { ticked, named },
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
            wire::Command::CreateBuilding {
                addr,
                template,
                rules,
                source,
                ..
            } => self.raise_building(
                &addr,
                template.as_str(),
                rules.as_deref(),
                source.as_deref().map(std::path::Path::new),
            ),
            wire::Command::RemoveBuilding { addr, .. } => self.remove_building(&addr),

            wire::Command::Approve { item, verdict, .. } => {
                self.answer_approval(&item, verdict, &kernel::Answerer::Human)
            }
            wire::Command::SetAutonomy {
                scope, autonomy, ..
            } => self.set_autonomy(&scope, autonomy),
            wire::Command::HandOff { item, .. } => Err(Unbuilt::HandOff(&item).not_built()),

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
            wire::Command::PutRules(ref write) => self.put_rules(write),
            wire::Command::SetWorkspaceChoice(ref write) => self.set_workspace_choice(write),
            wire::Command::PutPrompt(ref write) => self.put_prompt(write),
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
            wire::Command::RestoreDiscard { restoration, .. } => self.restore_discard(&restoration),
            wire::Command::DoctorInstall { ref item, .. } => self.doctor_install(item),
            wire::Command::DoctorRefresh { .. } => {
                self.look_at_this_machine();
                Ok(())
            }
            wire::Command::Release { scope, .. } => {
                self.set_admission(&scope, Admittance::Released)
            }

            wire::Command::Cancel { run, .. } => Err(Unanswered::Cancel.refusal(run)),
            wire::Command::Steer { run, .. } => Err(Unanswered::Steer.refusal(run)),
            wire::Command::OpenRemoteDoor { .. }
            | wire::Command::ReplaceCityKey { .. }
            | wire::Command::ConfirmRemoteDoor { .. }
            | wire::Command::CloseRemoteDoor { .. } => Err(the_door_is_held_elsewhere()),
            wire::Command::PrivacyOperation { .. } => Err(privacy_is_served_elsewhere()),

            wire::Command::BatchByBuilding { addr, .. } => {
                Err(Unbuilt::BatchByBuilding(&addr).not_built())
            }

            wire::Command::Auth { .. } => Err(Unbuilt::Auth.not_built()),
        }
    }

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

        self.last_tick = now;
        Ok(started)
    }

    #[cfg(test)]
    pub(in crate::worker) fn reveal_with(
        &mut self,
        reveal: fn(&std::path::Path, &kernel::Address) -> Result<(), AxError>,
    ) {
        self.reveal = reveal;
    }
}
