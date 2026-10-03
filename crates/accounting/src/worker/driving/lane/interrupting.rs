// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who may interrupt one drive, and the envelope a resident's letter
//! reaches the run in (collab D16).

use kernel::RunId;
use runtime::Interrupt;

/// Who may interrupt one drive, in rank order: the halt that reached
/// its backlog member, the person, then a neighbour's steer.
///
/// Two speakers, one landing, and the person outranks the resident.
/// What keeps them apart where the model reads them is
/// `runtime::conversation::Speaker`: only the person's entrance builds
/// `Speaker::Person`, rendered as `user`, and a resident's words land in
/// a `<letter>` naming its own address - the address a reply is sent to
/// (collab D16). A run that could not tell the two apart would answer the
/// person by signalling them, and answer a neighbour by talking to
/// nobody.
pub(super) struct Interrupting {
    pub(super) run_id: RunId,
    pub(super) member: Option<runtime::BacklogId>,
    pub(super) backlog: runtime::Backlog,
    pub(super) person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    pub(super) steers: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    /// Where the User's change of this room's run policy waits.
    pub(super) policy: crate::worker::rooms::PolicySlot,
    /// What arrived while the run waited out a provider and was not a
    /// halt. The desk hands each steer out once, so one taken during a
    /// wait is kept here for the safe point that follows it.
    pub(super) held: Option<Interrupt>,
}

impl Interrupting {
    /// Whether a halt has reached the run, asked while it waits out a
    /// provider. Anything else that arrives is held for the next safe
    /// point rather than answered here.
    pub(super) fn halted(&mut self) -> bool {
        if self.held.is_some() {
            return self.scope_stopping();
        }
        match self.ask() {
            Interrupt::Cancel => true,
            Interrupt::None => false,
            held @ (Interrupt::Steer { .. } | Interrupt::Policy { .. }) => {
                self.held = Some(held);
                false
            }
        }
    }

    /// A stopped scope outranks anything a person or a neighbour still
    /// has to say to the run.
    fn scope_stopping(&self) -> bool {
        super::scope_stopping(&self.backlog, self.member)
    }

    /// A model answer has just landed (`model_returned` is on the
    /// ledger before the wave it asked for), so every steer this run
    /// held has been read and is recorded as consumed (collab D8).
    ///
    /// A post that refuses leaves the signals held, and they go back to
    /// the room when the run leaves: a signal recorded as read that no
    /// answer read is the loss D8 exists to prevent, and the run's next
    /// append meets the same refusal and ends it.
    pub(super) fn answered(&mut self) {
        let Ok(mut desk) = self.steers.lock() else {
            return;
        };
        match desk.answered() {
            Ok(()) | Err(_) => {}
        }
    }

    /// Holds the run at its `BeforeAssemble` safe point while it waits for
    /// a reply (collab D9), making no model call, and answers how the
    /// wait ended: the reply or the timeout as a steer from the room
    /// waited on, or `Cancel` when the run is stopped meanwhile, which
    /// ends the wait as left. `None` when the run is not waiting.
    ///
    /// Only the person and the scope are asked while it waits, not the
    /// desk's steers: taking a steer moves the slot into the queue, and
    /// the reply must stay in the slot for the desk to see it. A clock,
    /// desk or post that fails lets the run go on rather than stopping
    /// it at a safe point; the wait is kept, and the drive's own clock
    /// and ledger reads report the failure.
    pub(super) fn wait_for_reply(&mut self, clock: &dyn crate::Clock) -> Option<Interrupt> {
        loop {
            let at = clock.now().ok()?;
            let turn = self.steers.lock().ok()?.wait_out(at).ok()?;
            match turn {
                collab::WaitTurn::Idle => return None,
                collab::WaitTurn::Replied(letter) => return Some(steer_of(&letter)),
                collab::WaitTurn::TimedOut { text } => {
                    return Some(Interrupt::Steer {
                        speaker: runtime::conversation::Speaker::City,
                        text,
                    });
                }
                collab::WaitTurn::Waiting => {}
            }
            if self.stopped_while_waiting() {
                let mut desk = self.steers.lock().ok()?;
                return Some(match desk.wait_left() {
                    Ok(()) | Err(_) => Interrupt::Cancel,
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(super::HALT_SLICE_MS));
        }
    }

    fn stopped_while_waiting(&mut self) -> bool {
        if self.scope_stopping() {
            return true;
        }
        if self.held.is_some() {
            return false;
        }
        match self.person.as_ref().map(|ask| ask(self.run_id)) {
            Some(Interrupt::Cancel) => true,
            Some(held @ (Interrupt::Steer { .. } | Interrupt::Policy { .. })) => {
                self.held = Some(held);
                false
            }
            Some(Interrupt::None) | None => false,
        }
    }

    pub(super) fn ask(&mut self) -> Interrupt {
        if self.scope_stopping() {
            return Interrupt::Cancel;
        }
        if let Some(held) = self.held.take() {
            return held;
        }
        let from_person = match self.person.as_ref() {
            Some(ask) => ask(self.run_id),
            None => Interrupt::None,
        };
        if !matches!(from_person, Interrupt::None) {
            return from_person;
        }
        if let Some(policy) = self.policy.take() {
            return Interrupt::Policy { policy };
        }
        // A desk nobody can take answers nothing rather than refusing:
        // a safe point is the wrong place to fail over a lock, and the
        // drive's own end will report it.
        let Ok(mut desk) = self.steers.lock() else {
            return Interrupt::None;
        };
        match desk.take_steer() {
            Ok(Some(letter)) => steer_of(&letter),
            Ok(None) => Interrupt::None,
            // A signal the desk took out of the queue and could not read as
            // a steer does not interrupt, and the ledger still has it: the
            // desk records the consumption before it reports the refusal.
            // A safe point is not the place to stop a run over a message it
            // cannot act on, which is the answer the person's own entrance
            // gives an empty steer (crate::worker::desk). What must not happen is
            // the two arriving here as one case; they do not.
            Err(_) => Interrupt::None,
        }
    }
}

/// A resident's letter as the runtime renders it: inside an envelope,
/// never as the User (collab D16).
fn steer_of(letter: &collab::Letter) -> Interrupt {
    Interrupt::Steer {
        speaker: runtime::conversation::Speaker::Resident(runtime::conversation::Letter {
            from: letter.from().to_owned(),
            run: letter.run(),
            kind: match letter.kind() {
                collab::LetterKind::Steer => runtime::conversation::LetterKind::Steer,
                collab::LetterKind::Reply => runtime::conversation::LetterKind::Reply,
            },
            sender: letter.sender().map(|state| state.as_str().to_owned()),
        }),
        text: letter.text().to_owned(),
    }
}
