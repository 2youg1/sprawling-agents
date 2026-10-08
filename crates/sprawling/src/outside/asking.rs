// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door's verbs on the wire (`crates/sprawling/spec/Outside.lean`
//! §8-140, remote_access D4): `CloseRemoteDoor` is carried at once;
//! `OpenRemoteDoor` and `ReplaceCityKey` only print a code at the city's
//! console and answer `E_APPROVAL_PENDING`; `ConfirmRemoteDoor` with that
//! code, before it expires, does the verb the code was printed for.
//!
//! Any local client can send these frames, including a resident's browser
//! tool driving the page, and none of them reads the console, which is
//! the whole guard (`remote_access::confirm`). Each verb, once allowed, is
//! the console's own line carried by `console::perform`, so the page and
//! `/remote` do one thing.
//!
//! The door is kept only by a serve that has a console. Without one,
//! nobody could read a code, so every verb but closing a door that was
//! never opened is refused.

use std::sync::{Arc, Mutex, OnceLock};

use kernel::{AxCode, AxError, TimeMs};
use remote_access::confirm::{CONFIRM_BYTES, Confirm};
use wire::{Command, Delivered, Reply, WireCommand};

use super::console::{Lasting, Remote, RemoteLine, perform};

/// How long a printed code answers (`crates/remote_access/Spec.lean` §8-13).
const CONFIRM_MS: u64 = 2 * 60 * 1000;

/// What the city's listener hands each Command to.
pub(crate) type Commands = Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>;

/// Where a line meant for the User goes: the city's console.
pub(crate) type Show = Arc<dyn Fn(&str) + Send + Sync>;

/// The door this serve's console keeps, once the console starts.
#[derive(Clone, Default)]
pub(crate) struct Front(Arc<OnceLock<Asking>>);

struct Asking {
    remote: Result<Remote, AxError>,
    waiting: Mutex<Confirm<Asked>>,
    show: Show,
}

/// A verb that waits for its code.
#[derive(Debug, Clone, Copy)]
enum Asked {
    Open(Lasting),
    ReplaceKey,
}

/// One of the four door verbs a frame carried.
enum Verb {
    Open(u64),
    ReplaceKey,
    Confirm(String),
    Close,
}

impl Front {
    /// The Commands the city's listener is given: the four door verbs
    /// answered here, every other Command handed to `desk`.
    pub(crate) fn commands(
        &self,
        desk: impl Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync + 'static,
    ) -> Commands {
        let front = self.clone();
        Arc::new(move |command, reply| match door_verb(command) {
            Ok(verb) => front.answer(verb, reply),
            Err(command) => desk(*command, reply),
        })
    }

    /// Hands the door the console keeps to the wire; a second call keeps
    /// the first door.
    pub(crate) fn attend(&self, remote: &Result<Remote, AxError>, show: Show) {
        self.0.get_or_init(|| Asking {
            remote: remote.clone(),
            waiting: Mutex::new(Confirm::default()),
            show,
        });
    }

    fn answer(&self, verb: Verb, reply: Reply) -> Result<(), AxError> {
        match (self.0.get(), verb) {
            (None, Verb::Close) => Ok(()),
            (None, Verb::Open(_) | Verb::ReplaceKey | Verb::Confirm(_)) => Err(no_console()),
            (Some(asking), Verb::Open(ms)) => asking.ask(Asked::Open(
                Lasting::of_ms(ms).ok_or_else(|| unlasting(ms))?,
            )),
            (Some(asking), Verb::ReplaceKey) => asking.ask(Asked::ReplaceKey),
            (Some(asking), Verb::Confirm(code)) => {
                let line = match asking.confirmed(&code)? {
                    Asked::Open(lasting) => RemoteLine::Open(lasting),
                    Asked::ReplaceKey => RemoteLine::ReplaceKey,
                };
                asking.later(line, reply);
                Ok(())
            }
            (Some(asking), Verb::Close) => {
                asking.later(RemoteLine::Close, reply);
                Ok(())
            }
        }
    }
}

impl Asking {
    /// Prints a code for `asked` and answers that it waits.
    fn ask(&self, asked: Asked) -> Result<(), AxError> {
        let senses = self.remote.as_ref().map_err(Clone::clone)?.doorway.senses();
        let mut entropy = [0u8; CONFIRM_BYTES];
        (senses.entropy)(&mut entropy)?;
        let expires = TimeMs::new((senses.clock)()?.value().saturating_add(CONFIRM_MS));
        let code = self.waiting()?.request(asked, entropy, expires);
        (self.show)(&format!(
            "  a page asks to {}; to allow it, type {code} into that page within two minutes",
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

    fn confirmed(&self, code: &str) -> Result<Asked, AxError> {
        let senses = self.remote.as_ref().map_err(Clone::clone)?.doorway.senses();
        let now = (senses.clock)()?;
        self.waiting()?.confirm(code, now)
    }

    /// Carries `line` off the listener's task, because opening a route
    /// waits for it to be reachable; the console reads what it did, and
    /// the page that asked reads a refusal.
    fn later(&self, line: RemoteLine, reply: Reply) {
        let remote = self.remote.clone();
        let show = Arc::clone(&self.show);
        std::thread::spawn(move || match perform(&remote, line) {
            Ok(said) => show(&said),
            Err(refused) => match reply.refuse(refused.clone()) {
                Delivered::ToThePeer => {}
                Delivered::NobodyAsked | Delivered::PeerGone => {
                    show(&format!("  {refused}\n  {}", refused.recovery()));
                }
            },
        });
    }

    fn waiting(&self) -> Result<std::sync::MutexGuard<'_, Confirm<Asked>>, AxError> {
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

/// The door verb `command` carries, or the Command itself for the desk.
fn door_verb(command: WireCommand) -> Result<Verb, Box<WireCommand>> {
    match command {
        Command::OpenRemoteDoor(opening) => Ok(Verb::Open(opening.lasting_ms)),
        Command::ReplaceCityKey(_) => Ok(Verb::ReplaceKey),
        Command::ConfirmRemoteDoor(answer) => Ok(Verb::Confirm(answer.code)),
        Command::CloseRemoteDoor(_) => Ok(Verb::Close),
        Command::Dispatch { .. }
        | Command::ProbeEndpoint { .. }
        | Command::ConfigureBuilding { .. }
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
    /// `serve` alone has no console, so the recovery names the two
    /// commands that start a city with one, in one sentence: the wrapped
    /// source line leaves no run of spaces in what a person reads.
    #[test]
    fn the_recovery_is_one_sentence_naming_a_command_that_has_a_console() {
        let refused = super::no_console();
        assert_eq!(
            refused.recovery(),
            "close this city and start it again in a terminal of your own with `sprawling up` or `sprawling serve --console`"
        );
    }
}
