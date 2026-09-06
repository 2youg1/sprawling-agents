// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Four things happen here and nothing else: the key this listener will
//! present at its door is settled before a socket exists, the vault is
//! opened and asked what it really is, the one writer thread is started
//! with the ledger inside it, and the socket is handed the four sinks it
//! may reach the city through.
//!
//! **The writer thread is the city's one writer.** The ledger is opened
//! inside it and never leaves, so the type never has to cross a thread
//! boundary to prove that a city has one writer (ARCHITECTURE section
//! 10). Everything a socket does reaches it as a `Command` on a desk,
//! one at a time.
//!
//! Randomness is drawn here rather than in `bin::keying`, which is pure:
//! this crate draws entropy in one place, and a key a third party can
//! predict is a door a third party can open.

use kernel::RunId;
use runtime::Interrupt;

/// A URL-safe random string of `bytes` bytes of OS entropy.
///
/// Deliberately not the simulator's seeded randomness: a verifier a
/// third party can predict is a login a third party can finish. This is
/// the one place in the binary where reproducibility would be a defect.
/// Where commands wait between the socket and the worker.
///
/// A desk rather than a channel, because a channel hands an item to
/// whoever is blocked on it, and during a run that is nobody: the
/// worker is inside a dispatch. A Cancel that waits for the run it
/// cancels is not a Cancel. The desk keeps arrival order, and the run
/// looks at it only at its own safe points.
pub(crate) struct CommandDesk {
    waiting: std::sync::Mutex<Waiting>,
    arrived: std::sync::Condvar,
    /// Set once, by whoever decided the city stops. Read at the same
    /// point the queue is read, so a close lands between commands and
    /// never inside one.
    closing: std::sync::atomic::AtomicBool,
}

/// What the desk holds, under one lock.
///
/// The queue and the keys are read together and must agree: a key that
/// left the queue between two locks would let the same ask through
/// twice, which is the whole of what this pair prevents.
struct Waiting {
    queue: std::collections::VecDeque<Posted>,
    /// Every key that is either still in `queue` or being carried out.
    /// `kernel::gate` says the seen set belongs to the caller and it
    /// only judges membership; this is that set for commands, as
    /// `ToolBench::seen` is that set for tool calls.
    keys: std::collections::BTreeSet<kernel::IdemKey>,
}

/// The key of the command somebody took off the desk, held until the
/// work is over.
///
/// A guard rather than a call the drainer has to remember: the key is in
/// flight for exactly as long as this value lives, including when the
/// loop that took it leaves early.
pub(crate) struct Underway<'desk> {
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
    /// Takes the command at `at` out of the line and releases its key.
    ///
    /// A command consumed as an interrupt is over the moment it is
    /// taken, and every client mints one key per run for cancelling, so
    /// a key left behind here would make a second Cancel unspeakable
    /// for the life of the city.
    fn forget(&mut self, at: usize) -> Option<Posted> {
        let posted = self.queue.remove(at)?;
        if let Some(key) = posted.command.idem() {
            self.keys.remove(key);
        }
        Some(posted)
    }
}

/// A command and the address its refusal goes back to.
///
/// The two travel together because they are separated by a thread and
/// by minutes: by the time the worker refuses, the socket task that
/// accepted the command has long returned.
pub(crate) struct Posted {
    pub(crate) command: channels::Command,
    pub(crate) reply: channels::Reply,
}

/// What the worker found when it looked at the desk. Exhaustive, because
/// "nothing arrived" and "nobody will ever arrive again" are different
/// facts and the loop does different things about them.
pub(crate) enum DeskWait<'desk> {
    /// Boxed because this variant is the only one that carries
    /// anything: a `Posted` holds a whole `Command`, and an enum shaped
    /// like this one is returned by every idle tick as well.
    Command(Box<Posted>, Underway<'desk>),
    Idle,
    /// A person chose to stop. Distinct from `Gone`, which is the desk
    /// itself breaking: one of these deserves a handoff and the other is
    /// a city that can no longer write one.
    Close,
    Gone,
}

/// How long the worker waits before looking at the schedule. Short
/// enough that a job stated to the minute starts within the minute,
/// long enough that an idle city is idle.
pub(super) const SCHEDULE_TICK: std::time::Duration = std::time::Duration::from_secs(20);

impl CommandDesk {
    /// Visible to the crate so the console loop can be driven in a test
    /// through the door production uses, rather than through a second
    /// one opened for testing.
    pub(crate) fn new() -> CommandDesk {
        CommandDesk {
            waiting: std::sync::Mutex::new(Waiting {
                queue: std::collections::VecDeque::new(),
                keys: std::collections::BTreeSet::new(),
            }),
            arrived: std::sync::Condvar::new(),
            closing: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Says the city is stopping, and wakes the worker so it hears.
    ///
    /// Not a Command: closing is not something a peer asks the city for,
    /// it is the process's own end, and a wire frame that could spell it
    /// would be a stranger's way to stop somebody's city. The worker
    /// reads it where it reads the queue, so whatever is running
    /// finishes first.
    pub(crate) fn close(&self) {
        self.closing
            .store(true, std::sync::atomic::Ordering::Release);
        self.arrived.notify_all();
    }

    /// Puts a command in line, unless the same one is already being
    /// dealt with.
    ///
    /// A key is in flight from here until the command it belongs to has
    /// been carried out, and a second frame carrying that key is
    /// dropped: the sender asked for one thing and one thing is
    /// happening. Every client mints the key from what the person
    /// entered, so a double-click, a transport that resent a frame and
    /// an editor retrying after a timeout all arrive as one ask - and
    /// each of them used to be a second run against a paid provider.
    /// Once the work is over the key is forgotten, so asking for the
    /// same work again is a second piece of work rather than silence.
    pub(crate) fn post(&self, command: channels::Command, reply: channels::Reply) {
        if let Ok(mut waiting) = self.waiting.lock() {
            if let Some(key) = command.idem() {
                if kernel::dedup(&waiting.keys, key) == kernel::DedupVerdict::Duplicate {
                    return;
                }
                waiting.keys.insert(*key);
            }
            waiting.queue.push_back(Posted { command, reply });
            self.arrived.notify_one();
        }
    }

    /// Waits for a command, for at most `patience`.
    ///
    /// The wait has an end so that the worker gets its own idle moment:
    /// a city whose schedule says something starts at nine cannot depend
    /// on somebody clicking at nine.
    pub(crate) fn wait(&self, patience: std::time::Duration) -> DeskWait<'_> {
        let Ok(mut waiting) = self.waiting.lock() else {
            return DeskWait::Gone;
        };
        // Work already accepted is finished first: a close that dropped
        // a queued command would make "stopped" and "lost" the same
        // thing in the record.
        if let Some(posted) = waiting.queue.pop_front() {
            return self.carrying(posted);
        }
        if self.closing.load(std::sync::atomic::Ordering::Acquire) {
            return DeskWait::Close;
        }
        match self.arrived.wait_timeout(waiting, patience) {
            Ok((mut waiting, _)) => match waiting.queue.pop_front() {
                Some(posted) => self.carrying(posted),
                None if self.closing.load(std::sync::atomic::Ordering::Acquire) => DeskWait::Close,
                None => DeskWait::Idle,
            },
            Err(_) => DeskWait::Gone,
        }
    }

    /// Hands out one command together with the guard that keeps its key
    /// in flight until whoever took it is done.
    fn carrying(&self, posted: Posted) -> DeskWait<'_> {
        let key = posted.command.idem().copied();
        DeskWait::Command(Box::new(posted), Underway { desk: self, key })
    }

    /// Takes one command if any is waiting, without waiting for one.
    /// Used where a test drives the desk directly; the worker loop waits.
    #[cfg(test)]
    pub(crate) fn take(&self) -> Option<channels::Command> {
        let mut waiting = self.waiting.lock().ok()?;
        let posted = waiting.queue.pop_front()?;
        if let Some(key) = posted.command.idem() {
            waiting.keys.remove(key);
        }
        Some(posted.command)
    }

    /// What the run at `run` should do at this safe point, if anything.
    ///
    /// Cancel outranks Steer on the same boundary: stopping and changing
    /// course are mutually exclusive, and stopping is the one that cannot
    /// be taken back. Commands for other runs keep their place in line.
    pub(crate) fn interrupt_for(&self, run: RunId) -> Interrupt {
        let Ok(mut waiting) = self.waiting.lock() else {
            return Interrupt::None;
        };
        let cancel = waiting.queue.iter().position(
            |posted| matches!(&posted.command, channels::Command::Cancel { run: r, .. } if *r == run),
        );
        if let Some(at) = cancel {
            waiting.forget(at);
            return Interrupt::Cancel;
        }
        let steer = waiting.queue.iter().position(
            |posted| matches!(&posted.command, channels::Command::Steer { run: r, .. } if *r == run),
        );
        let Some(at) = steer else {
            return Interrupt::None;
        };
        let Some(Posted {
            command: channels::Command::Steer { text, .. },
            ..
        }) = waiting.forget(at)
        else {
            return Interrupt::None;
        };
        // The person's entrance is the only one that renders as `user`,
        // and an empty steer is not an interruption.
        match collab::Steer::from_person(&text) {
            Ok(steer) => Interrupt::Steer {
                source: steer.source().to_owned(),
                text: steer.text().to_owned(),
            },
            Err(_) => Interrupt::None,
        }
    }
}
