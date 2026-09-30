// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The keep-warm door a run's model calls go through, and the doors a
//! worker keeps once the run has landed (sprawling-SPEC.md 8-112).

use super::RunWorker;
use kernel::event::Payload;
use kernel::event::record::CacheRenewed;
use kernel::{Address, AxError, EventKind, ModelReturn, TimeMs};
use std::collections::BTreeMap;

/// The chosen adapter, wrapped so every request a run sends enters the
/// keep-warm account (runtime-SPEC 8-4-2), timed on the worker's own
/// clock (accounting-SPEC.md 8-3).
pub(crate) type Door = runtime::prefix::warmth::Warmed<DoorClock>;

/// What a door reads the time through: the worker's clock, carried into
/// the lane the run is driven on.
pub(crate) type DoorClock = Box<dyn FnMut() -> Result<TimeMs, AxError> + Send>;

/// The doors that still owe a renewal, one per room.
///
/// One per room rather than one per run: a later run in the same room
/// sent the prefix the next run will use, so the earlier door would only
/// pay to keep a cache nobody reads.
#[derive(Default)]
pub(in crate::worker) struct Kept {
    doors: BTreeMap<Address, Door>,
}

impl Kept {
    /// Keeps `door` for `room` when it owes a renewal, replacing that
    /// room's earlier door; a door that owes none is dropped here, which
    /// is why the default setting leaves this empty.
    pub(in crate::worker) fn keep(&mut self, room: Address, door: Door) {
        if door.next_due().is_some() {
            self.doors.insert(room, door);
        } else {
            self.doors.remove(&room);
        }
    }

    /// The earliest instant any kept door is due.
    pub(in crate::worker) fn next_due(&self) -> Option<u64> {
        self.doors.values().filter_map(Door::next_due).min()
    }

    /// Sends every renewal due by `now_ms` and returns what each one
    /// cost or why it was refused, by room; then lets go of the doors
    /// that owe nothing more and of the doors whose renewal failed: a
    /// provider that refused a renewal is not asked again every wake.
    pub(in crate::worker) fn renew_due(&mut self, now_ms: u64) -> Vec<(Address, CacheRenewed)> {
        let mut renewals = Vec::new();
        self.doors.retain(|room, door| {
            let renewed = match door.next_due() {
                Some(due) if due <= now_ms => door.renew_due(now_ms),
                Some(_) | None => Ok(Vec::new()),
            };
            match renewed {
                Ok(answers) => {
                    renewals.extend(
                        answers
                            .into_iter()
                            .map(|answer| (room.clone(), answered(answer))),
                    );
                    door.next_due().is_some()
                }
                Err(refused) => {
                    renewals.push((room.clone(), CacheRenewed::Refused { refused }));
                    false
                }
            }
        });
        renewals
    }
}

/// The cost a provider reported for one renewal.
fn answered(answer: ModelReturn) -> CacheRenewed {
    CacheRenewed::Answered {
        usage: answer.usage,
        billed_usd_micros: answer.billed_usd_micros,
    }
}

impl RunWorker {
    /// When the next kept door is due, if any is kept.
    pub(crate) fn warm_due(&self) -> Option<u64> {
        self.warm.next_due()
    }

    /// Renews whatever is due at `now` and writes one `cache_renewed`
    /// line per renewal under its room, answered or refused. Only a line
    /// the ledger would not take goes to the diagnostic log, and it does
    /// not stop the city, as a schedule that cannot be read does not.
    pub(crate) fn renew_warm(&mut self, now: TimeMs) {
        for (room, renewed) in self.warm.renew_due(now.value()) {
            let written = Payload::of(&renewed)
                .and_then(|line| self.record_at(EventKind::CacheRenewed, room, line));
            if let Err(err) = written {
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "crate::worker::keeping_warm",
                    &format!("a keep-warm renewal was not recorded: {err}"),
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test"
)]
mod tests {
    use super::*;
    use kernel::consts_external::PROMPT_CACHE_TTL_SECS;
    use kernel::{
        AxCode, B3Hash, BuildingPolicy, Ceiling, ChatRequest, ContentBlock, KeepWarm, Model,
        ModelRequest, ModelUsage, Tokens,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const USED_AT: u64 = 1_000_000;
    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;

    /// What the answering provider reports each call cost.
    fn usage() -> ModelUsage {
        ModelUsage {
            input_tokens: Tokens::new(3),
            output_tokens: Tokens::new(1),
            cache_read_tokens: Tokens::new(4096),
            cache_write_tokens: Tokens::new(0),
            dialect: None,
        }
    }

    fn refusal() -> AxError {
        AxError::failure(AxCode::Provider, "renew a cached prefix", "rate limited")
            .with_recovery("renewal is skipped until the next real request")
    }

    /// Counts the calls that reach the provider; from the call numbered
    /// `refuses_from` on, it refuses.
    struct Provider {
        calls: Arc<AtomicUsize>,
        refuses_from: usize,
    }

    impl Model for Provider {
        fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
            if self.calls.fetch_add(1, Ordering::SeqCst) >= self.refuses_from {
                return Err(refusal());
            }
            let mut answer = ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "warm".to_owned(),
                }])
                .unwrap(),
                Vec::new(),
            );
            answer.usage = Some(usage());
            Ok(answer)
        }
    }

    /// A door that sent one real request at `USED_AT`.
    fn door_after_one_run(setting: KeepWarm, provider: Provider) -> Door {
        let clock: DoorClock = Box::new(|| Ok(TimeMs::new(USED_AT)));
        let mut door = Door::new(Box::new(provider), setting, clock);
        door.call(&ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"prefix"); 4],
            chat: ChatRequest::empty("script", Ceiling::new(512).unwrap()),
        })
        .unwrap();
        door
    }

    /// Wakes `kept` at every second through two cache lifetimes and
    /// returns every renewal it reported.
    fn renewals_through_two_lifetimes(kept: &mut Kept) -> Vec<(Address, CacheRenewed)> {
        let renewals = (USED_AT..=USED_AT + 2 * TTL_MS)
            .step_by(1_000)
            .flat_map(|now| kept.renew_due(now))
            .collect();
        assert_eq!(kept.next_due(), None);
        renewals
    }

    /// A worker that kept the door of one run which sent one request;
    /// returns how many calls reached the provider in all.
    fn calls_after_one_run(setting: KeepWarm) -> usize {
        let calls = Arc::new(AtomicUsize::new(0));
        let door = door_after_one_run(
            setting,
            Provider {
                calls: calls.clone(),
                refuses_from: usize::MAX,
            },
        );
        let mut kept = Kept::default();
        kept.keep(Address::parse("city/room").unwrap(), door);
        renewals_through_two_lifetimes(&mut kept);
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

    #[test]
    fn every_renewal_reports_its_cost_or_its_refusal_under_its_room() {
        let (answering, refusing) = (
            Address::parse("city/answering").unwrap(),
            Address::parse("city/refusing").unwrap(),
        );
        let mut kept = Kept::default();
        for (room, refuses_from) in [(&answering, usize::MAX), (&refusing, 1)] {
            let provider = Provider {
                calls: Arc::new(AtomicUsize::new(0)),
                refuses_from,
            };
            kept.keep(
                room.clone(),
                door_after_one_run(KeepWarm::FiveMinute, provider),
            );
        }
        assert_eq!(
            renewals_through_two_lifetimes(&mut kept),
            vec![
                (
                    answering,
                    CacheRenewed::Answered {
                        usage: Some(usage()),
                        billed_usd_micros: None,
                    }
                ),
                (refusing, CacheRenewed::Refused { refused: refusal() }),
            ]
        );
    }
}
