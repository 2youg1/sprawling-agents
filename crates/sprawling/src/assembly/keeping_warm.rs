// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The keep-warm door a run's model calls go through, and the doors a
//! worker keeps once the run has landed (sprawling-SPEC.md 8-93).

use super::RunWorker;
use kernel::{AxError, TimeMs};
use std::collections::BTreeMap;

/// The chosen adapter, wrapped so every request a run sends enters the
/// keep-warm account (runtime-SPEC 8-4-2), timed on the city's one
/// sampling point.
pub(crate) type Door = runtime::prefix::warmth::Warmed<fn() -> Result<TimeMs, AxError>>;

/// The doors that still owe a renewal, one per room.
///
/// One per room rather than one per run: a later run in the same room
/// sent the prefix the next run will use, so the earlier door would only
/// pay to keep a cache nobody reads.
#[derive(Default)]
pub(in crate::assembly) struct Kept {
    doors: BTreeMap<String, Door>,
}

impl Kept {
    /// Keeps `door` for `room` when it owes a renewal, replacing that
    /// room's earlier door; a door that owes none is dropped here, which
    /// is why the default setting leaves this empty.
    pub(in crate::assembly) fn keep(&mut self, room: String, door: Door) {
        if door.next_due().is_some() {
            self.doors.insert(room, door);
        } else {
            self.doors.remove(&room);
        }
    }

    /// The earliest instant any kept door is due.
    pub(in crate::assembly) fn next_due(&self) -> Option<u64> {
        self.doors.values().filter_map(Door::next_due).min()
    }

    /// Sends every renewal due by `now_ms`, then lets go of the doors
    /// that owe nothing more.
    ///
    /// # Errors
    /// The first door's failure; the doors after it still renew.
    pub(in crate::assembly) fn renew_due(&mut self, _now_ms: u64) -> Result<(), AxError> {
        Ok(())
    }
}

impl RunWorker {
    /// When the next kept door is due, if any is kept.
    pub(crate) fn warm_due(&self) -> Option<u64> {
        self.warm.next_due()
    }

    /// Renews whatever is due at `now`. A failed renewal is written to
    /// the diagnostic log and does not stop the city, as a schedule that
    /// cannot be read does not.
    pub(crate) fn renew_warm(&mut self, now: TimeMs) {
        if let Err(err) = self.warm.renew_due(now.value()) {
            self.note(
                runtime::diagnostics::Level::Effect,
                "bin::assembly::keeping_warm",
                &format!("a keep-warm renewal failed: {err}"),
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, reason = "test")]
mod tests {
    use super::*;
    use kernel::consts_external::PROMPT_CACHE_TTL_SECS;
    use kernel::{
        B3Hash, BuildingPolicy, Ceiling, ChatRequest, ContentBlock, KeepWarm, Model, ModelRequest,
        ModelReturn,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const USED_AT: u64 = 1_000_000;
    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;

    /// Counts the calls that reach the provider.
    struct Provider(Arc<AtomicUsize>);

    impl Model for Provider {
        fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "warm".to_owned(),
                }])
                .unwrap(),
                Vec::new(),
            ))
        }
    }

    /// A worker that kept the door of one run which sent one request,
    /// woken at every second through two cache lifetimes; returns how
    /// many calls reached the provider in all.
    fn calls_after_one_run(setting: KeepWarm) -> usize {
        let calls = Arc::new(AtomicUsize::new(0));
        let clock: fn() -> Result<TimeMs, AxError> = || Ok(TimeMs::new(USED_AT));
        let mut door = Door::new(Box::new(Provider(calls.clone())), setting, clock);
        door.call(&ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"prefix"); 4],
            chat: ChatRequest::empty("script", Ceiling::new(512).unwrap()),
        })
        .unwrap();
        let mut kept = Kept::default();
        kept.keep("city/room".to_owned(), door);
        for now in (USED_AT..=USED_AT + 2 * TTL_MS).step_by(1_000) {
            kept.renew_due(now).unwrap();
        }
        assert_eq!(kept.next_due(), None);
        calls.load(Ordering::SeqCst)
    }

    #[test]
    fn a_worker_on_the_default_setting_sends_nothing_after_a_run() {
        assert_eq!(calls_after_one_run(KeepWarm::Off), 1);
    }

    #[test]
    fn a_worker_on_five_minute_renews_a_landed_run_once_when_due() {
        assert_eq!(calls_after_one_run(KeepWarm::FiveMinute), 2);
    }
}
