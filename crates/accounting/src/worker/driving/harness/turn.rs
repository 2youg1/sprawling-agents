// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One turn of a harness run over its session: the four callbacks the
//! session asks while the harness works, what counts as a cut, and the
//! city's words for what the harness said (`crates/sprawling/Spec.lean` §8-124).

use std::cell::{Cell, RefCell};

use agent_protocols::{Answer, Listener, PermissionAsk, Permit, PermitKind, StopReason, Update};
use kernel::event::record::{
    HarnessAnswered, HarnessPermit, HarnessPermitKind, HarnessReported, HarnessStop,
};
use kernel::{AxError, Ledger, TimeMs};
use runtime::run::{Cut, HarnessRun};

use super::super::lane::{DriveContext, scope_stopping};
use super::{HarnessHalf, Prompting};

/// What a turn leaves: the run, how the prompt ended, and the first line
/// the turn could not book.
type Turned<'a> = (HarnessRun<'a>, Result<Answer, AxError>, Option<AxError>);

/// Sends the prompt and books what the harness says until it stops.
pub(super) fn take_the_turn<'a, L: Ledger>(
    half: &HarnessHalf,
    turning: Turning<'a, '_, L>,
    prompting: &mut Prompting,
    context: &DriveContext,
) -> Turned<'a> {
    let clock = &*context.clock;
    let turning = RefCell::new(turning);
    let pending = Cell::new(None);
    let deadline = turning.borrow().deadline;
    let mut halted = || match cut_now(half, context, deadline) {
        Some(cut) => {
            pending.set(Some(cut));
            true
        }
        None => false,
    };
    let mut cancelling = || -> Result<(), AxError> {
        let t = clock.now()?;
        let mut held = turning.borrow_mut();
        let Turning { run, ledger, .. } = &mut *held;
        run.cancel(&mut **ledger, pending.get().unwrap_or(Cut::Halt), t)
    };
    let mut report = |update: Update| -> Result<(), AxError> {
        let t = clock.now()?;
        turning.borrow_mut().book(&reported(update), t)
    };
    let mut permit = |ask: &PermissionAsk| -> Permit {
        let chosen = permitted(ask);
        let booked = clock
            .now()
            .and_then(|t| turning.borrow_mut().book_permit(ask, &chosen, t));
        match booked {
            Ok(()) => chosen,
            Err(failed) => {
                turning.borrow_mut().failed.get_or_insert(failed);
                Permit::Cancelled
            }
        }
    };
    let prompted = prompting(
        &half.prompt,
        &mut Listener {
            halted: &mut halted,
            cancelling: &mut cancelling,
            report: &mut report,
            permit: &mut permit,
        },
    );
    let Turning { run, failed, .. } = turning.into_inner();
    (run, prompted, failed)
}

/// The run and the ledger it writes through, shared by the four
/// callbacks a session asks.
pub(super) struct Turning<'a, 'l, L> {
    pub(super) run: HarnessRun<'a>,
    pub(super) ledger: &'l mut L,
    /// When the building's ceiling cuts the turn.
    pub(super) deadline: TimeMs,
    /// The first line a callback that cannot fail could not book.
    pub(super) failed: Option<AxError>,
}

impl<L: Ledger> Turning<'_, '_, L> {
    fn book(&mut self, reported: &HarnessReported, t: TimeMs) -> Result<(), AxError> {
        self.run.report(&mut *self.ledger, reported, t)
    }

    /// A permission ask and the city's answer to it, one line each.
    fn book_permit(
        &mut self,
        ask: &PermissionAsk,
        chosen: &Permit,
        t: TimeMs,
    ) -> Result<(), AxError> {
        let asked = HarnessReported::PermissionAsked {
            title: ask.title.clone(),
            options: ask
                .options
                .iter()
                .map(|option| HarnessPermit {
                    id: option.id.clone(),
                    name: option.name.clone(),
                    kind: permit_kind(option.kind),
                })
                .collect(),
        };
        self.book(&asked, t)?;
        let answered = HarnessReported::PermissionAnswered {
            chosen: match chosen {
                Permit::Chosen(id) => Some(id.clone()),
                Permit::Cancelled => None,
            },
        };
        self.book(&answered, t)
    }
}

/// Whether the turn is cut now, and why: the ceiling first, because a
/// run that ran out of time hit something; then a stopping scope and a
/// person's cancel, which are a halt. A clock that cannot be read says
/// nothing about the ceiling, and the next tick asks again.
///
/// A steer is taken and not delivered: ACP gives a client nothing to
/// send an agent in the middle of a turn, so the line says it went
/// nowhere rather than letting it vanish (`crates/sprawling/Spec.lean` §8-124).
fn cut_now(half: &HarnessHalf, context: &DriveContext, deadline: TimeMs) -> Option<Cut> {
    if context
        .clock
        .now()
        .is_ok_and(|now| now.value() >= deadline.value())
    {
        return Some(Cut::Deadline);
    }
    if scope_stopping(&context.backlog, half.member) {
        return Some(Cut::Halt);
    }
    let asked = context
        .person
        .as_ref()
        .map_or(runtime::Interrupt::None, |ask| ask(half.chartered.run));
    match asked {
        runtime::Interrupt::Cancel => Some(Cut::Halt),
        runtime::Interrupt::None => None,
        runtime::Interrupt::Steer { source, .. } => {
            half.notes.write(
                runtime::diagnostics::Level::Refuse,
                half.staged_at,
                "accounting::worker::driving::harness",
                &format!(
                    "{} is a harness's turn and takes no steer: the one from {source} was not \
                     delivered",
                    half.chartered.run
                ),
            );
            None
        }
    }
}

/// The city's answer to a permission ask (`crates/sprawling/Spec.lean` §8-4e rule
/// 9): the first "allow once", else the first "reject once", else
/// cancelled. Never "always": that would decide for later calls.
fn permitted(ask: &PermissionAsk) -> Permit {
    let first = |kind: PermitKind| {
        ask.options
            .iter()
            .find(|option| option.kind == kind)
            .map(|option| Permit::Chosen(option.id.clone()))
    };
    first(PermitKind::AllowOnce)
        .or_else(|| first(PermitKind::RejectOnce))
        .unwrap_or(Permit::Cancelled)
}

/// One report in the city's own words (kernel D9).
fn reported(update: Update) -> HarnessReported {
    match update {
        Update::Text(text) => HarnessReported::Said { text },
        Update::Thought(text) => HarnessReported::Thought { text },
        Update::ToolCall { id, title, kind } => HarnessReported::ToolCall {
            call: id,
            title,
            kind,
        },
        Update::ToolCallStatus { id, status } => {
            HarnessReported::ToolCallStatus { call: id, status }
        }
        Update::Other { variant } => HarnessReported::Other { variant },
    }
}

fn permit_kind(kind: PermitKind) -> HarnessPermitKind {
    match kind {
        PermitKind::AllowOnce => HarnessPermitKind::AllowOnce,
        PermitKind::AllowAlways => HarnessPermitKind::AllowAlways,
        PermitKind::RejectOnce => HarnessPermitKind::RejectOnce,
        PermitKind::RejectAlways => HarnessPermitKind::RejectAlways,
    }
}

/// The answer in the city's own words.
pub(super) fn answered(answer: Answer) -> HarnessAnswered {
    HarnessAnswered {
        stop: match answer.stop {
            StopReason::EndTurn => HarnessStop::EndTurn,
            StopReason::MaxTokens => HarnessStop::MaxTokens,
            StopReason::MaxTurnRequests => HarnessStop::MaxTurnRequests,
            StopReason::Refusal => HarnessStop::Refusal,
            StopReason::Cancelled => HarnessStop::Cancelled,
        },
        text: answer.text,
    }
}
