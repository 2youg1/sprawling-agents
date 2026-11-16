// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::ModelCalled;
use kernel::{
    AxCode, AxError, Increment, Ledger, Model, ModelRequest, ModelReturn, Payload, TimeMs,
};

use super::ledger::{Authored, Carried, Journal, Moment};
use super::speculation::{Generating, Speculated, call_ahead};

#[derive(Debug)]
pub(super) enum SegmentOutcome {
    Recovered(ModelReturn),

    Failed(AxError),

    Skipped(AxError),
}

pub(super) trait Segment {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_, '_>) -> SegmentOutcome;
}

#[derive(Debug)]
pub(super) struct Settled {
    pub(super) returned: ModelReturn,
    pub(super) speculated: Speculated,
    pub(super) first_at: Option<TimeMs>,

    pub(super) first_us: Option<u64>,

    pub(super) sent_us: Option<u64>,
}

enum FirstContent {
    Unseen,
    Read(Result<Moment, AxError>),
}

impl FirstContent {
    fn note(&mut self, held: &Increment, journal: &mut Journal<'_>) {
        let content = match held {
            Increment::Said(text) | Increment::Thought(text) => !text.is_empty(),
        };
        if content && let FirstContent::Unseen = self {
            *self = FirstContent::Read(journal.read_moment());
        }
    }

    fn moment(self) -> Result<Option<Moment>, AxError> {
        match self {
            FirstContent::Unseen => Ok(None),
            FirstContent::Read(reading) => reading.map(Some),
        }
    }
}

pub(super) struct ModelCall<'a, 'h> {
    journal: &'a mut Journal<'h>,
    ledger: &'a mut dyn Ledger,
    model: &'a mut dyn Model,
    request: &'a ModelRequest<'a>,
    streamed: bool,
    sent_us: Option<u64>,
}

impl<'a, 'h> ModelCall<'a, 'h> {
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

    pub(super) fn streamed(&self) -> bool {
        self.streamed
    }

    pub(super) fn resend_blocking(&mut self) -> Result<ModelReturn, AxError> {
        self.record()?;
        let attempt = self.model.call(self.request);
        self.note_hold()?;
        attempt
    }

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
        self.note_hold()?;
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

    fn note_hold(&mut self) -> Result<(), AxError> {
        let Some(held) = self.model.prefix_held() else {
            return Ok(());
        };
        let at = self.journal.read_moment()?.at;
        self.journal
            .append_redacted(Carried::PrefixHeld { at }, Payload::of(&held)?)?;
        Ok(())
    }

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
