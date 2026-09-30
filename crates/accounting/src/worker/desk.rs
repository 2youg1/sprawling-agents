// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where commands wait between the socket and the worker, and the two
//! verbs a run reads off that queue at its own safe points.

use kernel::RunId;
use runtime::Interrupt;

use super::relay::Wake;
use crate::worker::Closing;

/// Where commands wait between the socket and the worker.
///
/// A desk rather than a channel, because a channel hands an item to
/// whoever is blocked on it, and during a run that is nobody: the
/// worker is inside a dispatch. A Cancel that waits for the run it
/// cancels is not a Cancel. The desk keeps arrival order, and the run
/// looks at it only at its own safe points.
#[derive(Default)]
pub struct CommandDesk {
    waiting: std::sync::Mutex<Waiting>,
    /// The accounting thread's one queue, once a thread attends this
    /// desk. A post rings it so that thread wakes on the post itself
    /// rather than on a timer.
    bell: std::sync::Mutex<Option<std::sync::mpsc::Sender<Wake>>>,
    /// Set once, by whoever decided the city stops, with the reason
    /// they stopped it. Read at the same point the queue is read, so a
    /// close lands between commands and never inside one.
    closing: std::sync::OnceLock<Closing>,
}

/// What the desk holds, under one lock.
///
/// The queue and the keys are read together and must agree: a key that
/// left the queue between two locks would let the same ask through
/// twice, which is the whole of what this pair prevents.
#[derive(Default)]
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
pub struct Posted {
    pub command: wire::Command,
    pub reply: wire::Reply,
}

/// What the worker found when it looked at the desk. Exhaustive, because
/// "nothing arrived" and "nobody will ever arrive again" are different
/// facts and the loop does different things about them.
pub enum DeskWait<'desk> {
    /// Boxed because this variant is the only one that carries
    /// anything: a `Posted` holds a whole `Command`, and an enum shaped
    /// like this one is returned by every idle tick as well.
    Command(Box<Posted>, Underway<'desk>),
    Idle,
    /// The city is stopping, for the reason it carries. Distinct from
    /// `Gone`, which is the desk itself breaking: one of these deserves
    /// a handoff and the other is a city that can no longer write one.
    Close(&'desk Closing),
    Gone,
}

/// How long the worker may sleep before it looks at the schedule. Short
/// enough that a job stated to the minute starts within the minute,
/// long enough that an idle city is idle: this deadline is the only
/// wake an idle city has that nobody asked for.
///
/// In milliseconds because the loop compares it against the clock it
/// samples, and a rhythm stated twice is a rhythm that can disagree
/// with itself.
pub(super) const SCHEDULE_TICK_MS: u64 = 20_000;

impl CommandDesk {
    /// Says the city is stopping, and wakes the worker so it hears.
    ///
    /// Not a Command: closing is not something a peer asks the city for,
    /// it is the process's own end, and a wire frame that could spell it
    /// would be a stranger's way to stop somebody's city. The worker
    /// reads it where it reads the queue, so whatever is running
    /// finishes first. The first reason given stands: a second close
    /// does not rewrite why the city stopped.
    pub fn close(&self, why: Closing) {
        self.closing.get_or_init(|| why);
        self.ring(Wake::Close);
    }

    /// Rings `bell` from now on whenever a command is posted or the city
    /// closes. What was posted before is already on the desk, and the
    /// thread that attends it looks there before it first waits.
    pub fn ring_through(&self, bell: std::sync::mpsc::Sender<Wake>) {
        if let Ok(mut ringing) = self.bell.lock() {
            *ringing = Some(bell);
        }
    }

    fn ring(&self, wake: Wake) {
        if let Ok(ringing) = self.bell.lock()
            && let Some(bell) = ringing.as_ref()
        {
            // A thread that stopped attending has nothing to wake; what
            // was posted stays on the desk either way.
            drop(bell.send(wake));
        }
    }

    /// Puts a command in line, unless the same one is already being
    /// dealt with.
    ///
    /// A key is in flight from here until the command it belongs to has
    /// been carried out, and a second frame carrying that key is
    /// dropped: the sender asked for one thing and one thing is
    /// happening. Every client mints the key from what the person
    /// entered, so a double-click, a transport that resent a frame and
    /// an editor retrying after a timeout all arrive as one ask rather
    /// than as a second run against a paid provider.
    /// Once the work is over the key is forgotten, so asking for the
    /// same work again is a second piece of work rather than silence.
    pub fn post(&self, command: wire::Command, reply: wire::Reply) {
        if let Ok(mut waiting) = self.waiting.lock() {
            if let Some(key) = command.idem() {
                // The claim is the insertion: a key already in the set
                // is a command this desk already took, and taking it
                // again would be a second run against a paid provider.
                if kernel::idem::claim(&mut waiting.keys, *key).is_err() {
                    return;
                }
            }
            waiting.queue.push_back(Posted { command, reply });
        }
        self.ring(Wake::Command);
    }

    /// The next command, without waiting for one.
    ///
    /// Work already accepted is finished first: a close that dropped a
    /// queued command would make "stopped" and "lost" the same thing in
    /// the record. Waiting is the one queue's job, which a post rings.
    ///
    /// A Cancel or Steer naming a run `driving` answers for stays where
    /// it is: it is that lane's to lift at its next safe point, through
    /// [`Self::interrupt_for`]. The thread asking here wakes on every
    /// post and a lane reads only at its safe points, so handing it out
    /// here would refuse it as though no run answered
    /// (sprawling-SPEC.md 8-42-1).
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

    /// Hands out one command together with the guard that keeps its key
    /// in flight until whoever took it is done.
    fn carrying(&self, posted: Posted) -> DeskWait<'_> {
        let key = posted.command.idem().copied();
        DeskWait::Command(Box::new(posted), Underway { desk: self, key })
    }

    /// Takes one command if any is waiting, without waiting for one.
    /// Used where a test drives the desk directly; the worker loop waits.
    #[cfg(test)]
    pub fn take(&self) -> Option<wire::Command> {
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
    pub fn interrupt_for(&self, run: RunId) -> Interrupt {
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
