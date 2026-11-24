// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kernel::{AxCode, AxError, IdemKey, TimeMs};
use remote_access::confirm::{CONFIRM_BYTES, Confirm, Miss};
use wire::{Command, Reply, WireCommand};

use super::console::{Lasting, Remote, RemoteLine, perform};

const CONFIRM_MS: u64 = 2 * 60 * 1000;

pub(crate) type Commands = Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>;

pub(crate) type Show = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone, Default)]
pub(crate) struct Front(Arc<OnceLock<Asking>>);

struct Asking {
    remote: Result<Remote, AxError>,
    waiting: Mutex<Confirm<Requested>>,

    carrying: Arc<Mutex<Carrying>>,
    show: Show,
}

#[derive(Default)]
struct Carrying {
    locked_out: bool,
}

#[derive(Debug, Clone, Copy)]
enum Asked {
    Open(Lasting),
    ReplaceKey,
}

struct Requested {
    action: Asked,
    key: IdemKey,
}

enum Verb {
    Open(u64, IdemKey),
    ReplaceKey(IdemKey),
    Confirm(String, Option<IdemKey>, IdemKey),
    Close(IdemKey),
}

impl Front {
    pub(crate) fn commands(
        &self,
        desk: impl Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync + 'static,
    ) -> Commands {
        let front = self.clone();
        Arc::new(move |command, reply| match door_verb(command) {
            Ok(verb) => {
                let key = match &verb {
                    Verb::Open(_, key)
                    | Verb::ReplaceKey(key)
                    | Verb::Confirm(_, _, key)
                    | Verb::Close(key) => *key,
                };
                let result = front.answer(verb, reply);
                if let (Some(asking), Err(refused)) = (front.0.get(), &result)
                    && refused.code() != &AxCode::ApprovalPending
                {
                    (asking.show)(&failed_at(refused, key));
                }
                result
            }
            Err(command) => desk(*command, reply),
        })
    }

    pub(crate) fn attend(&self, remote: &Result<Remote, AxError>, show: Show) {
        self.0.get_or_init(|| Asking {
            remote: remote.clone(),
            waiting: Mutex::new(Confirm::default()),
            carrying: Arc::default(),
            show,
        });
    }

    fn answer(&self, verb: Verb, reply: Reply) -> Result<(), AxError> {
        match (self.0.get(), verb) {
            (None, Verb::Close(idem)) => {
                let _delivered = reply.accept(idem);
                Ok(())
            }
            (None, Verb::Open(..) | Verb::ReplaceKey(_) | Verb::Confirm(..)) => Err(no_console()),
            (Some(asking), Verb::Open(ms, idem)) => asking.ask(
                Asked::Open(Lasting::of_ms(ms).ok_or_else(|| unlasting(ms))?),
                idem,
            ),
            (Some(asking), Verb::ReplaceKey(idem)) => asking.ask(Asked::ReplaceKey, idem),
            (Some(asking), Verb::Confirm(code, approval, idem)) => {
                let line = match asking.confirmed(&code, approval)? {
                    Asked::Open(lasting) => RemoteLine::Open(lasting),
                    Asked::ReplaceKey => RemoteLine::ReplaceKey,
                };
                asking.later(line, idem, reply);
                Ok(())
            }
            (Some(asking), Verb::Close(idem)) => {
                asking.later(RemoteLine::Close, idem, reply);
                Ok(())
            }
        }
    }
}

impl Asking {
    fn ask(&self, asked: Asked, key: IdemKey) -> Result<(), AxError> {
        let senses = self.remote.as_ref().map_err(Clone::clone)?.doorway.senses();
        let mut entropy = [0u8; CONFIRM_BYTES];
        (senses.entropy)(&mut entropy)?;
        let expires = TimeMs::new((senses.clock)()?.value().saturating_add(CONFIRM_MS));
        let code = self
            .waiting()?
            .request(Requested { action: asked, key }, entropy, expires)?;
        (self.show)(&format!(
            "  a page asks to {}; to allow it, type {code} into that page within two minutes\n  remote operation {key}",
            asked.what()
        ));
        Err(AxError::failure(
            AxCode::ApprovalPending,
            asked.action(),
            "a code is waiting on the city's console",
        )
        .with_recovery(
            "type the code the city's console printed; the console is the terminal running \
             `sprawling serve`",
        ))
    }

    fn confirmed(&self, code: &str, approval: Option<IdemKey>) -> Result<Asked, AxError> {
        let senses = self.remote.as_ref().map_err(Clone::clone)?.doorway.senses();
        let now = (senses.clock)()?;
        let mut waiting = self.waiting()?;
        let answered = waiting.confirm(code, now);
        let locked_now = matches!(answered, Err(Miss::Wrong)) && waiting.locked();
        drop(waiting);
        if locked_now {
            self.lock_out();
        }
        let approved = answered.map_err(Miss::refusal)?;
        if approval.is_some_and(|key| key != approved.key) {
            return Err(AxError::failure(
                AxCode::GateDenied,
                "confirm a remote door verb",
                "the console code belongs to a different request",
            )
            .with_recovery(
                "start again from the intended control and type the matching console code",
            ));
        }
        Ok(approved.action)
    }

    fn lock_out(&self) {
        (self.show)(
            "  ten wrong codes were typed into a page since the last right one: the page's \
             remote-door controls are off until this city restarts, and an open remote door \
             is closed; /remote at this console still works",
        );
        let remote = self.remote.clone();
        let show = Arc::clone(&self.show);
        let carrying = Arc::clone(&self.carrying);
        std::thread::spawn(move || {
            let mut turn = carrying.lock().unwrap_or_else(PoisonError::into_inner);
            turn.locked_out = true;
            let closed = perform(&remote, RemoteLine::Close);
            drop(turn);
            match closed {
                Ok(said) => show(&said),
                Err(refused) => show(&format!("  {refused}\n  {}", refused.recovery())),
            }
        });
    }

    fn later(&self, line: RemoteLine, idem: IdemKey, reply: Reply) {
        let remote = self.remote.clone();
        let show = Arc::clone(&self.show);
        let carrying = Arc::clone(&self.carrying);
        std::thread::spawn(move || {
            let turn = carrying.lock().unwrap_or_else(PoisonError::into_inner);
            let done = if turn.locked_out && matches!(line, RemoteLine::Open(_)) {
                Err(Miss::Locked.refusal())
            } else {
                perform(&remote, line)
            };
            drop(turn);
            match done {
                Ok(said) => {
                    show(&format!("{said}\n  remote operation {idem}"));
                    let _delivered = reply.accept(idem);
                }
                Err(refused) => {
                    show(&failed_at(&refused, idem));
                    let _delivered = reply.refuse(refused);
                }
            }
        });
    }

    fn waiting(&self) -> Result<std::sync::MutexGuard<'_, Confirm<Requested>>, AxError> {
        self.waiting.lock().map_err(|_| {
            AxError::failure(
                AxCode::Busy,
                "confirm a remote door verb",
                "an earlier confirmation stopped halfway",
            )
            .with_recovery("restart the city")
        })
    }
}

impl Asked {
    fn what(self) -> String {
        match self {
            Asked::Open(lasting) => format!(
                "open the remote door for {} minutes",
                lasting.ms().checked_div(60_000).unwrap_or_default()
            ),
            Asked::ReplaceKey => {
                "replace the city key, after which every paired device pairs again".to_owned()
            }
        }
    }

    fn action(self) -> &'static str {
        match self {
            Asked::Open(_) => "open the remote door",
            Asked::ReplaceKey => "replace the city key",
        }
    }
}

fn failed_at(refused: &AxError, key: IdemKey) -> String {
    format!(
        "  {refused}\n  {}\n  remote operation {key}",
        refused.recovery()
    )
}

fn no_console() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "work the remote door from the page",
        "this city has no console to print the confirmation code on",
    )
    .with_recovery(
        "close this city and start it again in a terminal of your own with `sprawling up` \
         or `sprawling serve --console`",
    )
}

fn unlasting(ms: u64) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "open the remote door",
        format!("{ms} ms is not between one minute and seven days"),
    )
    .with_recovery("choose how long the door stays open again")
}

fn door_verb(command: WireCommand) -> Result<Verb, Box<WireCommand>> {
    match command {
        Command::OpenRemoteDoor(opening) => Ok(Verb::Open(opening.lasting_ms, opening.idem)),
        Command::ReplaceCityKey(step) => Ok(Verb::ReplaceKey(step.idem)),
        Command::ConfirmRemoteDoor(answer) => {
            Ok(Verb::Confirm(answer.code, answer.approval, answer.idem))
        }
        Command::CloseRemoteDoor(step) => Ok(Verb::Close(step.idem)),
        Command::Dispatch { .. }
        | Command::ProbeEndpoint { .. }
        | Command::ConfigureBuilding(_)
        | Command::AttachEndpoint { .. }
        | Command::SelectModel { .. }
        | Command::OpenSession { .. }
        | Command::Reveal { .. }
        | Command::RestoreDiscard { .. }
        | Command::DoctorInstall { .. }
        | Command::DoctorRefresh { .. }
        | Command::ConnectToolkit { .. }
        | Command::PutSpine { .. }
        | Command::PutRange(_)
        | Command::PutPrompt(_)
        | Command::DecideProposals(_)
        | Command::CreateBuilding { .. }
        | Command::RemoveBuilding { .. }
        | Command::Steer { .. }
        | Command::Cancel { .. }
        | Command::Halt { .. }
        | Command::Release { .. }
        | Command::Approve { .. }
        | Command::SetAutonomy { .. }
        | Command::HandOff { .. }
        | Command::PutPreferences { .. }
        | Command::PutShelved { .. }
        | Command::Pursue { .. }
        | Command::PutDocument { .. }
        | Command::PutIdentity { .. }
        | Command::PutRules(_)
        | Command::SetWorkspaceChoice(_)
        | Command::ConfigureCity(_)
        | Command::RestoreFile { .. }
        | Command::PutGuide { .. }
        | Command::BatchByBuilding { .. }
        | Command::Wake { .. }
        | Command::Auth { .. }
        | Command::PutSecret { .. }
        | Command::ForgetSecret(_)
        | Command::NameSession(_)
        | Command::ChangeRunPolicy(_)
        | Command::PrivacyOperation { .. }
        | Command::CloseCity(_)
        | Command::AddAgent(_)
        | Command::AgentLogin(_)
        | Command::ForgetDevice(_) => Err(Box::new(command)),
    }
}

#[cfg(test)]
mod without_a_console {

    #[test]
    fn the_recovery_is_one_sentence_naming_a_command_that_has_a_console() {
        let refused = super::no_console();
        assert_eq!(
            refused.recovery(),
            "close this city and start it again in a terminal of your own with `sprawling up` or `sprawling serve --console`"
        );
    }
}
