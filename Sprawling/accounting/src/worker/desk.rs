// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::RunId;
use runtime::Interrupt;

use super::relay::Wake;
use crate::worker::Closing;

#[derive(Default)]
pub struct CommandDesk {
    waiting: std::sync::Mutex<Waiting>,

    bell: std::sync::Mutex<Option<std::sync::mpsc::Sender<Wake>>>,

    closing: std::sync::OnceLock<Closing>,

    interrupting: std::sync::atomic::AtomicBool,
}

#[derive(Default)]
struct Waiting {
    queue: std::collections::VecDeque<Posted>,

    keys: std::collections::BTreeSet<kernel::IdemKey>,
}

pub struct Underway<'desk> {
    desk: &'desk CommandDesk,
    key: Option<kernel::IdemKey>,
}

impl Drop for Underway<'_> {
    fn drop(&mut self) {
        let Some(key) = self.key else { return };
        if let Ok(mut waiting) = self.desk.waiting.lock() {
            waiting.keys.remove(&key);
        }
    }
}

impl Waiting {

    fn forget(&mut self, at: usize) -> Option<Posted> {
        let posted = self.queue.remove(at)?;
        if let Some(key) = posted.command.idem() {
            self.keys.remove(key);
        }
        Some(posted)
    }
}

pub struct Posted {
    pub command: wire::Command,
    pub reply: wire::Reply,
}

pub enum DeskWait<'desk> {

    Command(Box<Posted>, Underway<'desk>),
    Idle,

    Close(&'desk Closing),
    Gone,
}

pub(super) const SCHEDULE_TICK_MS: u64 = 20_000;

impl CommandDesk {

    pub fn close(&self, why: Closing) {
        if let Closing::Chosen {
            mode: wire::CloseMode::Interrupt,
            ..
        } = &why
        {
            self.interrupt();
        }
        self.closing.get_or_init(|| why);
        self.ring(Wake::Close);
    }

    pub fn interrupt(&self) {
        self.interrupting
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.ring(Wake::Close);
    }

    pub fn interrupting(&self) -> bool {
        self.interrupting.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn ring_through(&self, bell: std::sync::mpsc::Sender<Wake>) {
        if let Ok(mut ringing) = self.bell.lock() {
            *ringing = Some(bell);
        }
    }

    fn ring(&self, wake: Wake) {
        if let Ok(ringing) = self.bell.lock()
            && let Some(bell) = ringing.as_ref()
        {

            drop(bell.send(wake));
        }
    }

    pub fn post(&self, command: wire::Command, reply: wire::Reply) {
        if let Ok(mut waiting) = self.waiting.lock() {
            if let Some(key) = command.idem() {

                if kernel::idem::claim(&mut waiting.keys, *key).is_err() {
                    return;
                }
            }
            waiting.queue.push_back(Posted { command, reply });
        }
        self.ring(Wake::Command);
    }

    pub fn next(&self, driving: impl Fn(RunId) -> bool) -> DeskWait<'_> {
        let Ok(mut waiting) = self.waiting.lock() else {
            return DeskWait::Gone;
        };
        let for_a_lane = |posted: &Posted| match wire::classify(&posted.command) {
            wire::ControlVerdict::Intervene { run: Some(run), .. } => driving(run),
            wire::ControlVerdict::Intervene { run: None, .. }
            | wire::ControlVerdict::NotAnIntervention
            | wire::ControlVerdict::Refuse(_) => false,
        };
        let first = waiting.queue.iter().position(|posted| !for_a_lane(posted));
        match first.and_then(|at| waiting.queue.remove(at)) {
            Some(posted) => self.carrying(posted),
            None => match self.closing.get() {
                Some(why) => DeskWait::Close(why),
                None => DeskWait::Idle,
            },
        }
    }

    fn carrying(&self, posted: Posted) -> DeskWait<'_> {
        let key = posted.command.idem().copied();
        DeskWait::Command(Box::new(posted), Underway { desk: self, key })
    }

    #[cfg(test)]
    pub fn take(&self) -> Option<wire::Command> {
        let mut waiting = self.waiting.lock().ok()?;
        let posted = waiting.queue.pop_front()?;
        if let Some(key) = posted.command.idem() {
            waiting.keys.remove(key);
        }
        Some(posted.command)
    }

    pub fn interrupt_for(&self, run: RunId) -> Interrupt {
        if self.interrupting() {
            return Interrupt::Cancel;
        }
        let Ok(mut waiting) = self.waiting.lock() else {
            return Interrupt::None;
        };
        let cancel = waiting.queue.iter().position(
            |posted| matches!(&posted.command, wire::Command::Cancel { run: r, .. } if *r == run),
        );
        if let Some(at) = cancel {
            waiting.forget(at);
            return Interrupt::Cancel;
        }
        let steer = waiting.queue.iter().position(
            |posted| matches!(&posted.command, wire::Command::Steer { run: r, .. } if *r == run),
        );
        let Some(at) = steer else {
            return Interrupt::None;
        };
        let Some(Posted {
            command: wire::Command::Steer { text, .. },
            ..
        }) = waiting.forget(at)
        else {
            return Interrupt::None;
        };

        match collab::Steer::from_person(&text) {
            Ok(steer) => Interrupt::Steer {
                speaker: runtime::conversation::Speaker::Person,
                text: steer.text().to_owned(),
            },
            Err(_) => Interrupt::None,
        }
    }
}
