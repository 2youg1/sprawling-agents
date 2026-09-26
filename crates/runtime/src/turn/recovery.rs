// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model-call recovery pipeline: the call chain's middle error layer
//! (runtime-SPEC.md §8-44). `gateway` decides whether the same request
//! may go out again; a segment repairs the failures an identical second
//! request would fail on again; `crate::Watchdog` disposes of the rest.
//! The segment contract is three exhaustive answers, and a skip hands
//! the failure on rather than swallowing it.

use kernel::event::record::ModelCalled;
use kernel::{AxCode, AxError, Increment, Ledger, Model, ModelRequest, ModelReturn, Payload};

use super::ledger::{Authored, Journal};
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
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_>) -> SegmentOutcome;
}

/// One provider call being recovered: what was sent, which door it went
/// out of, and the only way to ask again — which records the attempt
/// before it is made, so no repair can leave a silent repeat behind.
pub(super) struct ModelCall<'a> {
    journal: &'a mut Journal,
    ledger: &'a mut dyn Ledger,
    model: &'a mut dyn Model,
    request: &'a ModelRequest<'a>,
    streamed: bool,
}

impl<'a> ModelCall<'a> {
    /// Opens the call being recovered. `streamed` is decided by `ask`:
    /// it is a fact about how the failing attempt went out.
    pub(super) fn open(
        journal: &'a mut Journal,
        ledger: &'a mut dyn Ledger,
        model: &'a mut dyn Model,
        request: &'a ModelRequest<'a>,
    ) -> ModelCall<'a> {
        ModelCall {
            journal,
            ledger,
            model,
            request,
            streamed: false,
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
    ) -> Result<(ModelReturn, Speculated), AxError> {
        self.record()?;
        let settled = |value| (value, Speculated::default());
        let (streamed, first) = match generating {
            Generating::Unwatched => (false, self.model.call(self.request).map(settled)),
            Generating::Watched(sink) => (
                true,
                self.model.call_streaming(self.request, sink).map(settled),
            ),
            Generating::Speculating { deltas, tools } => (
                true,
                match deltas {
                    Some(sink) => call_ahead(self.model, self.request, sink, tools),
                    None => call_ahead(self.model, self.request, &mut |_: &Increment| {}, tools),
                },
            ),
        };
        self.streamed = streamed;
        match first {
            Ok(value) => Ok(value),
            Err(failure) => match recover(segments, self, failure) {
                SegmentOutcome::Recovered(value) => Ok(settled(value)),
                SegmentOutcome::Failed(err) | SegmentOutcome::Skipped(err) => Err(err),
            },
        }
    }

    /// Appends `model_called` before the effect it announces. Every
    /// attempt lands on the ledger, so a repaired resend reads as the
    /// second `model_called` in the history rather than as silence.
    fn record(&mut self) -> Result<(), AxError> {
        let called = ModelCalled {
            segments: self.request.segments.to_vec(),
            model: self.request.chat.model.clone(),
        };
        self.journal
            .append_authored(self.ledger, Authored::ModelCalled, Payload::of(&called)?)?;
        Ok(())
    }
}

/// Runs the segments in the order they are listed. A skip hands its
/// error to the next segment; the first recovery or repair failure ends
/// the relay; when every segment skips, the failure that comes out is
/// the one that went in.
pub(super) fn recover(
    segments: &mut [&mut dyn Segment],
    call: &mut ModelCall<'_>,
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
pub(super) struct BlockingResend;

impl Segment for BlockingResend {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_>) -> SegmentOutcome {
        if !call.streamed() || *failure.code() != AxCode::WireMismatch {
            return SegmentOutcome::Skipped(failure.clone());
        }
        match call.resend_blocking() {
            Ok(value) => SegmentOutcome::Recovered(value),
            Err(err) => SegmentOutcome::Failed(err),
        }
    }
}
