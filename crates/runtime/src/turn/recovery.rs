// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model-call recovery pipeline: the call chain's middle error layer
//! (`crates/runtime/spec/Turn/Recovery.lean` §8-49). `gateway` decides whether the same request
//! may go out again; a segment repairs the failures an identical second
//! request would fail on again; `crate::Watchdog` disposes of the rest.
//! The segment contract is three exhaustive answers, and a skip hands
//! the failure on rather than swallowing it.

use kernel::event::record::ModelCalled;
use kernel::{
    AxCode, AxError, Increment, Ledger, Model, ModelRequest, ModelReturn, Payload, TimeMs,
};

use super::ledger::{Authored, Journal, Moment};
use super::speculation::{Generating, Speculated, call_ahead};

/// What one segment answered when it was offered a failed call.
/// Exhaustive and closed: a fourth answer forces every caller to decide
/// rather than fall through a catch-all.
#[derive(Debug)]
pub(super) enum SegmentOutcome {
    /// The repair took: the turn continues from this return.
    Recovered(ModelReturn),
    /// The segment recognized the failure, acted, and the act failed.
    /// The error is the repair's own, typed, so its stable code and
    /// recovery sentence travel with it.
    Failed(AxError),
    /// Not this segment's failure. The error it was offered leaves with
    /// it, field for field, so a skip hands the failure onward instead of
    /// swallowing it.
    Skipped(AxError),
}

/// The contract a recovery segment satisfies: offer it a failure and the
/// call being recovered, and it answers one of the three above.
pub(super) trait Segment {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_, '_>) -> SegmentOutcome;
}

/// What one model call settled into: the return the turn records, the
/// reads started early, and when the attempt that returned it first
/// reported content (`crates/runtime/spec/Turn/Recovery.lean` §8-50).
#[derive(Debug)]
pub(super) struct Settled {
    pub(super) returned: ModelReturn,
    pub(super) speculated: Speculated,
    pub(super) first_at: Option<TimeMs>,
    /// Microseconds from `sent_us` to the first content.
    pub(super) first_us: Option<u64>,
    /// The monotonic reading taken when the attempt that returned went
    /// out; the reply's whole duration is measured from it.
    pub(super) sent_us: Option<u64>,
}

/// Whether an attempt has reported content yet, and the reading taken
/// the moment it first did.
enum FirstContent {
    Unseen,
    Read(Result<Moment, AxError>),
}

impl FirstContent {
    /// Reads the clock for the first non-empty piece of prose or
    /// reasoning, and for nothing after it.
    fn note(&mut self, held: &Increment, journal: &mut Journal<'_>) {
        let content = match held {
            Increment::Said(text) | Increment::Thought(text) => !text.is_empty(),
        };
        if content && let FirstContent::Unseen = self {
            *self = FirstContent::Read(journal.read_moment());
        }
    }

    /// When the first content arrived; `None` when none did.
    fn moment(self) -> Result<Option<Moment>, AxError> {
        match self {
            FirstContent::Unseen => Ok(None),
            FirstContent::Read(reading) => reading.map(Some),
        }
    }
}

/// One provider call being recovered: what was sent, which door it went
/// out of, and the only way to ask again — which records the attempt
/// before it is made, so no repair can leave a silent repeat behind.
pub(super) struct ModelCall<'a, 'h> {
    journal: &'a mut Journal<'h>,
    ledger: &'a mut dyn Ledger,
    model: &'a mut dyn Model,
    request: &'a ModelRequest<'a>,
    streamed: bool,
    sent_us: Option<u64>,
}

impl<'a, 'h> ModelCall<'a, 'h> {
    /// Opens the call being recovered. `streamed` is decided by `ask`:
    /// it is a fact about how the failing attempt went out.
    pub(super) fn open(
        journal: &'a mut Journal<'h>,
        ledger: &'a mut dyn Ledger,
        model: &'a mut dyn Model,
        request: &'a ModelRequest<'a>,
    ) -> ModelCall<'a, 'h> {
        ModelCall {
            journal,
            ledger,
            model,
            request,
            streamed: false,
            sent_us: None,
        }
    }

    /// Whether the call that failed went out through the streaming door.
    pub(super) fn streamed(&self) -> bool {
        self.streamed
    }

    /// The same request again through the blocking door. The attempt is
    /// recorded before it is made, like every other call this turn makes.
    ///
    /// # Errors
    /// Whatever the blocking door fails with — the failure is its own,
    /// and a segment answers `SegmentOutcome::Failed` with it.
    pub(super) fn resend_blocking(&mut self) -> Result<ModelReturn, AxError> {
        self.record()?;
        self.model.call(self.request)
    }

    /// Makes the call and runs the recovery pipeline over its failure.
    /// `generating` picks the door; every door owes the same
    /// `ModelReturn`, and the record below is written from that return
    /// in each case. The three answers fold to one `Result` here:
    /// whether a segment skipped is the pipeline's own account, and the
    /// turn wants the return or the one error that carries the story. A
    /// repaired return carries no reads started early: they belong to
    /// the attempt that failed.
    ///
    /// # Errors
    /// The failure no segment repaired (`Skipped`, unchanged), or the
    /// repair's own failure (`Failed`).
    pub(super) fn ask(
        &mut self,
        segments: &mut [&mut dyn Segment],
        generating: Generating<'_, '_>,
    ) -> Result<Settled, AxError> {
        self.record()?;
        let settled = |value| (value, Speculated::default());
        let mut first = FirstContent::Unseen;
        let (streamed, attempt) = {
            let (journal, model, request) = (&mut *self.journal, &mut *self.model, self.request);
            let mut noted = |held: &Increment| first.note(held, journal);
            match generating {
                Generating::Unwatched => (false, model.call(request).map(settled)),
                Generating::Watched(sink) => (
                    true,
                    model
                        .call_streaming(request, &mut |held: &Increment| {
                            noted(held);
                            sink(held);
                        })
                        .map(settled),
                ),
                Generating::Speculating { deltas, tools } => (
                    true,
                    match deltas {
                        Some(sink) => call_ahead(
                            model,
                            request,
                            &mut |held: &Increment| {
                                noted(held);
                                sink(held);
                            },
                            tools,
                        ),
                        None => call_ahead(model, request, &mut noted, tools),
                    },
                ),
            }
        };
        self.streamed = streamed;
        match attempt {
            Ok((returned, speculated)) => {
                let first = first.moment()?;
                Ok(Settled {
                    returned,
                    speculated,
                    first_at: first.map(|moment| moment.at),
                    first_us: first.and_then(|moment| moment.since(self.sent_us)),
                    sent_us: self.sent_us,
                })
            }
            // The failed attempt's first content, and any failure to read
            // the clock for it, belong to no record: the repair's return
            // came through a door with no stream (`crates/runtime/spec/Turn/Recovery.lean` §8-50).
            Err(failure) => match recover(segments, self, failure) {
                SegmentOutcome::Recovered(returned) => Ok(Settled {
                    returned,
                    speculated: Speculated::default(),
                    first_at: None,
                    first_us: None,
                    sent_us: self.sent_us,
                }),
                SegmentOutcome::Failed(err) | SegmentOutcome::Skipped(err) => Err(err),
            },
        }
    }

    /// Appends `model_called` before the effect it announces, at the
    /// moment the attempt goes out, and makes it durable together with
    /// every line the turn held before it: a model call is an outside
    /// effect, so this is one of the turn's barriers (runtime D24). Every
    /// attempt lands on the ledger, so
    /// a repaired resend reads as the second `model_called` in the
    /// history rather than as silence.
    fn record(&mut self) -> Result<(), AxError> {
        let called = ModelCalled {
            provider_account: self.model.provider_account(),
            segments: self.request.segments.to_vec(),
            model: self.request.chat.model.clone(),
        };
        let sent = self.journal.read_moment()?;
        self.sent_us = sent.us;
        self.journal
            .append_authored(Authored::ModelCalled { at: sent.at }, Payload::of(&called)?);
        self.journal.barrier(self.ledger)
    }
}

/// Runs the segments in the order they are listed. A skip hands its
/// error to the next segment; the first recovery or repair failure ends
/// the relay; when every segment skips, the failure that comes out is
/// the one that went in.
pub(super) fn recover(
    segments: &mut [&mut dyn Segment],
    call: &mut ModelCall<'_, '_>,
    failure: AxError,
) -> SegmentOutcome {
    let mut carried = failure;
    for slot in segments.iter_mut() {
        match slot.attempt(&carried, call) {
            SegmentOutcome::Skipped(next) => carried = next,
            SegmentOutcome::Recovered(value) => return SegmentOutcome::Recovered(value),
            SegmentOutcome::Failed(err) => return SegmentOutcome::Failed(err),
        }
    }
    SegmentOutcome::Skipped(carried)
}

/// The one repair this city attempts today. The two doors owe the same
/// answer, but streaming assembly and whole-body parsing are two code
/// paths: a reply the streaming parser refused is asked for through the
/// one that reads a complete body. Anything else is skipped — a
/// retriable failure is the watchdog's to dispose of, and re-sending a
/// refusal only buys the same refusal.
///
/// The resend is one more send on the call's account, so it goes out
/// only when the round the watchdog holds admits it, and is counted there
/// when it does (kernel D54).
pub(super) struct BlockingResend<'w> {
    pub(super) round: &'w mut crate::Watchdog,
}

impl Segment for BlockingResend<'_> {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_, '_>) -> SegmentOutcome {
        if !call.streamed()
            || *failure.code() != AxCode::WireMismatch
            || !self.round.admits_repair()
        {
            return SegmentOutcome::Skipped(failure.clone());
        }
        self.round.repaired();
        match call.resend_blocking() {
            Ok(value) => SegmentOutcome::Recovered(value),
            Err(err) => SegmentOutcome::Failed(err),
        }
    }
}
