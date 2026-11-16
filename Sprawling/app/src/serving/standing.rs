// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::RefCell;
use std::time::{Duration, Instant};

use accounting::person::CorePriority;

use super::journal::notice;

const MODULE: &str = "bin::serving::standing";

#[expect(
    clippy::disallowed_methods,
    reason = "the one monotonic sampling point: a core thread's valve measures spans"
)]
#[must_use]
pub fn monotonic_now() -> Instant {
    Instant::now()
}

pub(crate) const BUSY_LIMIT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standing {
    Raised,
    Normal(Held),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Held {
    ByTheSetting,

    Refused(String),
    ByTheValve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Keep,
    Lower,
}

const ONE_STEP_ABOVE_NORMAL: u8 = 70;
const NORMAL: u8 = 50;

#[derive(Debug)]
pub(crate) struct CoreThread {
    name: &'static str,
    standing: Standing,
    valve: Valve,

    lower: fn() -> Result<Standing, thread_priority::Error>,
    lowering: Lowering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lowering {
    Owed,

    Refused,
}

impl CoreThread {

    pub(crate) fn raise(name: &'static str, setting: CorePriority, now: Instant) -> Self {
        let standing = raise_this_thread(setting);
        if let Standing::Normal(Held::Refused(reason)) = &standing {
            notice(
                MODULE,
                &format!("thread {name} stays at normal priority: {reason}"),
            );
        }
        Self {
            name,
            standing,
            valve: Valve::new(BUSY_LIMIT, now),
            lower: lower_this_thread,
            lowering: Lowering::Owed,
        }
    }

    #[cfg(test)]
    fn raised_with(lower: fn() -> Result<Standing, thread_priority::Error>, now: Instant) -> Self {
        Self {
            name: "test",
            standing: Standing::Raised,
            valve: Valve::new(BUSY_LIMIT, now),
            lower,
            lowering: Lowering::Owed,
        }
    }

    pub(crate) fn record_turn_lowering_when_busy(&mut self, woke: Instant, slept: Instant) {
        self.valve.record(woke, slept);
        if self.standing == Standing::Raised
            && self.lowering == Lowering::Owed
            && self.valve.verdict() == Verdict::Lower
        {
            self.lower_telling_the_person();
        }
    }

    fn lower_telling_the_person(&mut self) {
        match (self.lower)() {
            Ok(standing) => {
                notice(
                    MODULE,
                    &format!(
                        "thread {} kept a core busy for {} s and is back at normal priority",
                        self.name,
                        BUSY_LIMIT.as_secs()
                    ),
                );
                self.standing = standing;
            }
            Err(err) => {
                notice(
                    MODULE,
                    &format!(
                        "thread {} kept a core busy for {} s and could not be lowered: {err}",
                        self.name,
                        BUSY_LIMIT.as_secs()
                    ),
                );
                self.lowering = Lowering::Refused;
            }
        }
    }
}

pub fn setting_telling_a_refusal() -> CorePriority {
    accounting::person::core_priority().unwrap_or_else(|err| {
        notice(MODULE, &format!("the core stays at normal priority: {err}"));
        CorePriority::Normal
    })
}

pub fn serving_runtime(setting: CorePriority) -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .on_thread_unpark(move || worker_woke(setting, monotonic_now()))
        .on_thread_park(|| worker_slept(monotonic_now()))
        .build()
}

thread_local! {

    static WORKER: RefCell<Option<(CoreThread, Instant)>> = const { RefCell::new(None) };
}

fn worker_woke(setting: CorePriority, now: Instant) {
    WORKER.with_borrow_mut(|worker| match worker {
        Some((_, woke)) => *woke = now,
        None => *worker = Some((CoreThread::raise("tokio-runtime-worker", setting, now), now)),
    });
}

fn worker_slept(now: Instant) {
    WORKER.with_borrow_mut(|worker| {
        if let Some((core, woke)) = worker {
            core.record_turn_lowering_when_busy(*woke, now);
        }
    });
}

pub(crate) fn raise_this_thread(setting: CorePriority) -> Standing {
    match setting {
        CorePriority::Normal => Standing::Normal(Held::ByTheSetting),
        CorePriority::Raised => match set_this_thread(ONE_STEP_ABOVE_NORMAL) {
            Ok(()) => Standing::Raised,
            Err(err) => Standing::Normal(Held::Refused(err.to_string())),
        },
    }
}

pub(crate) fn lower_this_thread() -> Result<Standing, thread_priority::Error> {
    set_this_thread(NORMAL).map(|()| Standing::Normal(Held::ByTheValve))
}

fn set_this_thread(level: u8) -> Result<(), thread_priority::Error> {
    let value = thread_priority::ThreadPriorityValue::try_from(level)
        .map_err(|_| thread_priority::Error::Priority("a level outside 0 to 99"))?;
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Crossplatform(
        value,
    ))
}

#[derive(Debug)]
pub(crate) struct Valve {
    limit: Duration,
    opened: Instant,
    busy: Duration,
    verdict: Verdict,
}

impl Valve {

    pub(crate) fn new(limit: Duration, now: Instant) -> Self {
        Self {
            limit,
            opened: now,
            busy: Duration::ZERO,
            verdict: Verdict::Keep,
        }
    }

    pub(crate) fn record(&mut self, woke: Instant, slept: Instant) {
        self.busy = self
            .busy
            .saturating_add(slept.saturating_duration_since(woke));
        let window = slept.saturating_duration_since(self.opened);
        if window < self.limit {
            return;
        }
        if self.busy.saturating_mul(16) >= window.saturating_mul(15) {
            self.verdict = Verdict::Lower;
        }
        self.opened = slept;
        self.busy = Duration::ZERO;
    }

    pub(crate) fn verdict(&self) -> Verdict {
        self.verdict
    }
}

#[cfg(test)]
mod tests;
