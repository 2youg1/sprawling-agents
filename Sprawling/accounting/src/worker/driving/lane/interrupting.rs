// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::RunId;
use runtime::Interrupt;

pub(super) struct Interrupting {
    pub(super) run_id: RunId,
    pub(super) member: Option<runtime::BacklogId>,
    pub(super) backlog: runtime::Backlog,
    pub(super) person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    pub(super) steers: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,

    pub(super) policy: crate::worker::rooms::PolicySlot,

    pub(super) held: Option<Interrupt>,
}

impl Interrupting {
    pub(super) fn halted(&mut self) -> bool {
        if let Some(held) = &self.held {
            return matches!(held, Interrupt::Cancel) || self.scope_stopping();
        }
        match self.ask() {
            Interrupt::Cancel => {
                self.held = Some(Interrupt::Cancel);
                true
            }
            Interrupt::None => false,
            held @ (Interrupt::Steer { .. } | Interrupt::Policy { .. }) => {
                self.held = Some(held);
                false
            }
        }
    }

    fn scope_stopping(&self) -> bool {
        super::scope_stopping(&self.backlog, self.member)
    }

    pub(super) fn answered(&mut self) {
        let Ok(mut desk) = self.steers.lock() else {
            return;
        };
        match desk.answered() {
            Ok(()) | Err(_) => {}
        }
    }

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
                        request: None,
                        pictures: Vec::new(),
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

        let Ok(mut desk) = self.steers.lock() else {
            return Interrupt::None;
        };
        match desk.take_steer() {
            Ok(Some(letter)) => steer_of(&letter),
            Ok(None) => Interrupt::None,

            Err(_) => Interrupt::None,
        }
    }
}

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
        request: None,
        pictures: Vec::new(),
    }
}
