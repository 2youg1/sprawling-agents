// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The keep-warm door a run's model calls go through, and the doors a
//! worker keeps once the run has landed (`crates/sprawling/Spec.lean` §8-112).
//!
//! Which prefix is due is decided inside each door by
//! `kernel::keep_warm`, and the `cache_renewed` lines are written on the
//! accounting thread; the renewal itself is sent from a thread of its
//! own, so a silent provider holds up nothing but its own door.

use super::RunWorker;
use super::errand::Answer;
use kernel::event::Payload;
use kernel::event::record::CacheRenewed;
use kernel::{Address, AxError, EventKind, ModelReturn, TimeMs};
use runtime::prefix::warmth::Lead;
use std::collections::{BTreeMap, BTreeSet};

/// The chosen adapter, wrapped so every request a run sends enters the
/// keep-warm account (`crates/runtime/Spec.lean` §8-4-2), timed on the worker's own
/// clock (`crates/accounting/spec/Clock.lean` §8-3).
pub(crate) type Door = runtime::prefix::warmth::Warmed<DoorClock>;

/// What a door reads the time through: the worker's clock, carried into
/// the lane the run is driven on.
pub(crate) type DoorClock = Box<dyn FnMut() -> Result<TimeMs, AxError> + Send>;

/// The doors that still owe a renewal, one per room, and the lead each
/// endpoint's renewals leave with.
///
/// One per room rather than one per run: a later run in the same room
/// sent the prefix the next run will use, so the earlier door would only
/// pay to keep a cache nobody reads.
#[derive(Default)]
pub(in crate::worker) struct Kept {
    doors: BTreeMap<Address, Door>,
    /// One reading per endpoint name, shared by every door that reaches
    /// it, so a renewal's round trip times every room on that endpoint
    /// (`crates/runtime/spec/Prefix.lean` D98).
    leads: BTreeMap<String, Lead>,
    /// The rooms whose door is out on a renewal. A room leaves this set
    /// when its door comes back or when a newer door is kept for it, so
    /// a door that comes back to a room no longer here was superseded.
    away: BTreeSet<Address>,
}

/// A door back from its renewal: whose room it keeps, the door itself,
/// and what the provider answered to each renewal it sent.
pub(crate) struct Renewed {
    pub(in crate::worker) room: Address,
    pub(in crate::worker) door: Door,
    pub(in crate::worker) answers: Result<Vec<ModelReturn>, AxError>,
}

impl Kept {
    /// The lead of the endpoint named `endpoint`, opened at no reading
    /// the first time a door reaches it.
    pub(in crate::worker) fn lead_of(&mut self, endpoint: &str) -> Lead {
        self.leads.entry(endpoint.to_owned()).or_default().clone()
    }

    /// Keeps `door` for `room` when it owes a renewal, replacing that
    /// room's earlier door, at home or out; a door that owes none is
    /// dropped here, which is why the default setting leaves this empty.
    pub(in crate::worker) fn keep(&mut self, room: Address, door: Door) {
        self.away.remove(&room);
        if door.next_due().is_some() {
            self.doors.insert(room, door);
        } else {
            self.doors.remove(&room);
        }
    }

    /// The earliest instant any door at home is due. A door out on a
    /// renewal is not counted, so nothing wakes for it twice.
    pub(in crate::worker) fn next_due(&self) -> Option<u64> {
        self.doors.values().filter_map(Door::next_due).min()
    }

    /// Takes out every door due by `now_ms`, each to renew on a thread
    /// of its own, and remembers its room as away.
    pub(in crate::worker) fn take_due(&mut self, now_ms: u64) -> Vec<(Address, Door)> {
        let due: Vec<Address> = self
            .doors
            .iter()
            .filter(|(_, door)| door.next_due().is_some_and(|due| due <= now_ms))
            .map(|(room, _)| room.clone())
            .collect();
        due.into_iter()
            .filter_map(|room| {
                let door = self.doors.remove(&room)?;
                self.away.insert(room.clone());
                Some((room, door))
            })
            .collect()
    }

    /// Takes a door back from its renewal and returns its room with what
    /// each renewal cost or why it was refused. The door comes home when
    /// it still owes a renewal and no newer door replaced it meanwhile; a
    /// door whose renewal failed is let go, so a provider that refused a
    /// renewal is not asked again every wake.
    pub(in crate::worker) fn came_back(
        &mut self,
        renewed: Renewed,
    ) -> (Address, Vec<CacheRenewed>) {
        let Renewed {
            room,
            door,
            answers,
        } = renewed;
        let still_ours = self.away.remove(&room);
        let lines = match answers {
            Ok(answers) => {
                if still_ours && door.next_due().is_some() {
                    self.doors.insert(room.clone(), door);
                }
                answers.into_iter().map(answered).collect()
            }
            Err(refused) => vec![CacheRenewed::Refused { refused }],
        };
        (room, lines)
    }

    /// Forgets that `room`'s door is out: it never left, because its
    /// thread would not start.
    pub(in crate::worker) fn lost(&mut self, room: &Address) {
        self.away.remove(room);
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
    /// When the next kept door is due, if any is kept at home.
    pub(crate) fn warm_due(&self) -> Option<u64> {
        self.warm.next_due()
    }

    /// Hands every door due at `now` to a thread of its own, which sends
    /// its renewals; the answers are recorded by [`Self::record_renewal`]
    /// when they come back (`crates/sprawling/Spec.lean` §8-112, D78).
    ///
    /// A door whose thread will not start is recorded as refused and let
    /// go, as a door the provider refused is.
    pub(crate) fn renew_warm(&mut self, now: TimeMs) {
        for (room, mut door) in self.warm.take_due(now.value()) {
            let at = room.clone();
            let sent = self.flight.errands.send(move || {
                let answers = door.renew_due(now.value());
                Answer::Renewed(Renewed {
                    room: at,
                    door,
                    answers,
                })
            });
            if let Err(refused) = sent {
                self.warm.lost(&room);
                self.write_renewal(room, CacheRenewed::Refused { refused });
            }
        }
    }

    /// Takes a door back from its renewal and writes one `cache_renewed`
    /// line per renewal under its room, answered or refused.
    pub(in crate::worker) fn record_renewal(&mut self, renewed: Renewed) {
        let (room, lines) = self.warm.came_back(renewed);
        for line in lines {
            self.write_renewal(room.clone(), line);
        }
    }

    /// Writes one `cache_renewed` line. Only a line the ledger would not
    /// take goes to the diagnostic log, and it does not stop the city,
    /// as a schedule that cannot be read does not.
    fn write_renewal(&mut self, room: Address, renewed: CacheRenewed) {
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
            cache_read_tokens: kernel::CacheCount::Reported(Tokens::new(4096)),
            cache_write_tokens: kernel::CacheCount::Reported(Tokens::new(0)),
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
    fn door_after_one_run(setting: KeepWarm, provider: impl Model + Send + 'static) -> Door {
        let clock: DoorClock = Box::new(|| Ok(TimeMs::new(USED_AT)));
        let mut door = Door::new(Box::new(provider), setting, clock, Lead::default());
        door.call(&ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"prefix"); 4],
            chat: ChatRequest::empty("script", Ceiling::new(512).unwrap()),
        })
        .unwrap();
        door
    }

    /// Sends every renewal due at `now_ms` on this thread, as each
    /// door's own thread would, and takes every door back.
    fn renew_here(kept: &mut Kept, now_ms: u64) -> Vec<(Address, CacheRenewed)> {
        kept.take_due(now_ms)
            .into_iter()
            .flat_map(|(room, mut door)| {
                let answers = door.renew_due(now_ms);
                let (room, lines) = kept.came_back(Renewed {
                    room,
                    door,
                    answers,
                });
                lines.into_iter().map(move |line| (room.clone(), line))
            })
            .collect()
    }

    /// Wakes `kept` at every second through two cache lifetimes and
    /// returns every renewal it reported.
    fn renewals_through_two_lifetimes(kept: &mut Kept) -> Vec<(Address, CacheRenewed)> {
        let renewals = (USED_AT..=USED_AT + 2 * TTL_MS)
            .step_by(1_000)
            .flat_map(|now| renew_here(kept, now))
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

    /// How long a stand-in provider stays silent when nobody releases
    /// it: long enough that an accounting thread waiting on it is seen
    /// waiting, short enough that a red run ends on its assertion.
    const SILENCE: std::time::Duration = std::time::Duration::from_secs(5);

    /// Answers the first call at once, as the run's real request; every
    /// later call, a renewal, waits until released or until [`SILENCE`]
    /// has passed, and says so when it answers.
    struct Silent {
        calls: usize,
        release: std::sync::mpsc::Receiver<()>,
        said: std::sync::mpsc::Sender<&'static str>,
    }

    impl Model for Silent {
        fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.calls = self.calls.saturating_add(1);
            if self.calls > 1 {
                self.release.recv_timeout(SILENCE).unwrap_or(());
                self.said.send("provider answered").unwrap();
            }
            Provider {
                calls: Arc::new(AtomicUsize::new(0)),
                refuses_from: usize::MAX,
            }
            .call(req)
        }
    }

    /// A line a lane writes through its relay.
    fn lane_line() -> kernel::EventDraft {
        kernel::EventDraft {
            run: kernel::RunId::CITY,
            t: TimeMs::new(USED_AT),
            who: "city".to_owned(),
            addr: None,
            kind: EventKind::CityInitialized,
            data: Payload::empty(),
            ig: false,
        }
    }

    /// A renewal waits on its provider off the accounting thread: while
    /// the provider is silent, a relay request from another lane is
    /// still answered, and the renewal's line is written under its room
    /// once the provider answers.
    #[test]
    fn a_renewal_held_by_a_silent_provider_leaves_a_relay_answered() {
        let dir = tempfile::tempdir().unwrap();
        let report = crate::worker::fixture::init_city(dir.path()).unwrap();
        let mut worker = RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        let (release, held) = std::sync::mpsc::channel();
        let (said, heard) = std::sync::mpsc::channel();
        let room = Address::parse("city/room").unwrap();
        let provider = Silent {
            calls: 0,
            release: held,
            said: said.clone(),
        };
        let door = door_after_one_run(KeepWarm::FiveMinute, provider);
        worker.warm.keep(room.clone(), door);
        let due = worker.warm_due().unwrap();

        let mut relay = worker.relay();
        let lane = std::thread::spawn(move || {
            kernel::Ledger::append(&mut relay, lane_line()).unwrap();
            said.send("relay answered").unwrap();
        });
        worker.renew_warm(TimeMs::new(due));
        let mut heard_first = Vec::new();
        while heard_first.is_empty() && !lane.is_finished() {
            worker
                .serve_flight(crate::worker::relay::Patience::For(
                    std::time::Duration::from_millis(20),
                ))
                .unwrap();
            heard_first.extend(heard.try_recv());
        }
        // A provider that already stopped waiting has dropped its end.
        release.send(()).unwrap_or(());
        worker.land_the_rest().unwrap();
        lane.join().unwrap();
        let heard_in_order: Vec<&str> = heard_first.into_iter().chain(heard.try_iter()).collect();

        assert_eq!(heard_in_order, vec!["relay answered", "provider answered"]);
        let renewed: Vec<Option<serde_json::Value>> =
            runtime::replay::verify_ledger_dir(&report.ledger_dir)
                .unwrap()
                .raw_lines()
                .iter()
                .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
                .filter(|record| record.get("kind") == Some(&serde_json::json!("cache_renewed")))
                .map(|record| record.get("addr").cloned())
                .collect();
        assert_eq!(renewed, vec![Some(serde_json::json!(room.as_str()))]);
    }
}
