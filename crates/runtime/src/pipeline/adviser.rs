// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The window's adviser: three typed questions, one recorded answer, and
//! the deterministic strategy that answers when no adviser does.
//!
//! **The adviser is the window's, never the prefix's.** The port below
//! takes a [`Window`] and nothing else — no `FrozenConfig`, no frozen
//! prefix, no call shape — so a question that would move the cached half
//! of a request cannot be spelled. Freezing the model and the effort is
//! the session's business: a consultant asked to re-decide either would
//! invalidate the message cache breakpoints on every turn, which is the
//! one thing the frozen half exists to prevent.
//!
//! **The call happens outside the sieve, and its verdict enters
//! [`crate::pipeline::package`].** The sieve chain is synchronous and
//! fallible in no place a consultant could be awaited; asking before the
//! packaging decision and handing the answer in keeps the chain as it
//! is.
//!
//! **A fallback is never silent.** When no adviser is attached, when it
//! does not answer, or when it answers a question that was not asked,
//! the consultation says so, and [`Consultation::payloads`] gives the
//! caller the two ledger lines (`adviser_asked` then `adviser_answered`
//! or `adviser_fell_back`) that make the fallback a recorded fact. The
//! answer the city then acts on is the same one it acted on before any
//! adviser existed, because a consultation with no answer changes
//! nothing.
//!
//! The port is a boxed function rather than a `pub trait` because the
//! seam table in ARCHITECTURE §4 is add-only and lists every file a
//! `pub trait` may live in. The implementations are still plural — an
//! attached endpoint in `bin::assembly`, a script here — and the
//! function value carries exactly the two arguments a trait method
//! would.

use kernel::event::record::{
    AdviserAnswer, AdviserAnswered, AdviserAsked, AdviserFailure, AdviserFellBack,
};
use kernel::{AxError, Payload};

use crate::window::Window;

/// Which of the three questions is being asked. Re-exported from
/// `kernel::event::record`, whose payload vocabulary is the question's
/// one home; an answer is validated against it before anything acts.
pub use kernel::event::record::AdviserAsk;

/// One question, spelled for the adviser.
///
/// `material` is the text under judgement — a tool result about to be
/// packaged — and `options` is non-empty only for a `Choice`. The two
/// are fields of the question rather than a second argument, so a caller
/// cannot pair a `Choice` with material or a density score with options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub kind: AdviserAsk,
    pub subject: String,
    pub options: Vec<String>,
    pub material: Option<String>,
}

impl Ask {
    /// "Is this still needed?" about one item on its way into the window.
    pub fn noul(subject: impl Into<String>, material: impl Into<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Noul,
            subject: subject.into(),
            options: Vec::new(),
            material: Some(material.into()),
        }
    }

    /// "How dense is this text?" — the figure the window's budget spends
    /// before it cuts.
    pub fn score(subject: impl Into<String>, material: impl Into<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Score,
            subject: subject.into(),
            options: Vec::new(),
            material: Some(material.into()),
        }
    }

    /// "Which of these?" — asked when a session's model is chosen, before
    /// anything is frozen, and nowhere else.
    pub fn choice(subject: impl Into<String>, options: Vec<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Choice,
            subject: subject.into(),
            options,
            material: None,
        }
    }
}

/// What one consultation produced, as the ledger keeps it and as
/// [`crate::pipeline::package`] acts on it.
///
/// Exhaustive: an answer and a fallback are different facts about the
/// move the city made, and a single struct with an optional answer would
/// let a reader forget which one they are looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Consultation {
    Answered {
        ask: AdviserAsk,
        subject: String,
        answer: AdviserAnswer,
        elapsed_ms: u64,
    },
    FellBack {
        ask: AdviserAsk,
        subject: String,
        reason: AdviserFailure,
    },
}

impl Consultation {
    /// The answer to act on, or `None` when the deterministic strategy
    /// stands. A caller that acts only on `Some` cannot accidentally act
    /// on a fallback.
    #[must_use]
    pub fn answer(&self) -> Option<&AdviserAnswer> {
        match self {
            Consultation::Answered { answer, .. } => Some(answer),
            Consultation::FellBack { .. } => None,
        }
    }

    /// The two ledger lines this consultation writes: the question, then
    /// its answer or its fallback.
    ///
    /// # Errors
    /// Propagates what `Payload::of` says about the encoding.
    pub fn payloads(&self) -> Result<Vec<Payload>, AxError> {
        let (ask, subject) = match self {
            Consultation::Answered { ask, subject, .. }
            | Consultation::FellBack { ask, subject, .. } => (*ask, subject),
        };
        let asked = Payload::of(&AdviserAsked {
            ask,
            subject: subject.clone(),
        })?;
        let outcome = match self {
            Consultation::Answered {
                subject,
                answer,
                elapsed_ms,
                ..
            } => Payload::of(&AdviserAnswered {
                subject: subject.clone(),
                answer: answer.clone(),
                elapsed_ms: *elapsed_ms,
            })?,
            Consultation::FellBack {
                subject, reason, ..
            } => Payload::of(&AdviserFellBack {
                subject: subject.clone(),
                reason: *reason,
            })?,
        };
        Ok(vec![asked, outcome])
    }
}

/// A consultant that can fail. [`Consultation::Answered`] is only
/// reachable through [`Adviser::consult`], which checks the answer
/// against the question before anything believes it.
type Answer =
    Box<dyn for<'a, 'b> FnMut(&'a Ask, &'b Window) -> Result<AdviserAnswer, AdviserFailure> + Send>;

/// The window's adviser, as a city holds it: an answer function, or
/// nothing at all.
///
/// `none` is the default and the worst case is exactly the behaviour a
/// city without advisers always had: no answer means no adjustment.
pub struct Adviser {
    answer: Option<Answer>,
}

impl Adviser {
    /// No adviser attached. Every consultation falls back with
    /// `Unavailable`, which is what makes the missing consultant a fact
    /// in the ledger rather than an absence.
    #[must_use]
    pub fn none() -> Adviser {
        Adviser { answer: None }
    }

    /// An adviser implemented elsewhere — the attached endpoint in
    /// `bin::assembly`, a script in a test. The function value's second
    /// argument is the window: a closure cannot reach `FrozenConfig`
    /// through it, which is the whole type-level promise of this port.
    pub fn with(
        answer: impl for<'a, 'b> FnMut(&'a Ask, &'b Window) -> Result<AdviserAnswer, AdviserFailure>
        + Send
        + 'static,
    ) -> Adviser {
        Adviser {
            answer: Some(Box::new(answer)),
        }
    }

    /// Asks one question and writes down which answer the city used.
    ///
    /// `elapsed_ms` is the caller's, like every other measurement that
    /// reaches the ledger: a second opinion on the critical path is a
    /// cost, and the caller is the one who paid it.
    pub fn consult(&mut self, ask: Ask, window: &Window, elapsed_ms: u64) -> Consultation {
        let kind = ask.kind;
        let Some(answer) = self.answer.as_mut() else {
            return Consultation::FellBack {
                ask: kind,
                subject: ask.subject,
                reason: AdviserFailure::Unavailable,
            };
        };
        match answer(&ask, window) {
            Ok(answer) if accepts(kind, &answer, &ask.options) => Consultation::Answered {
                ask: kind,
                subject: ask.subject,
                answer,
                elapsed_ms,
            },
            // An answer arrived and it is not the answer to the question
            // that was asked. The words do not travel: what a reader
            // needs is that the deterministic strategy answered instead.
            Ok(_) => Consultation::FellBack {
                ask: kind,
                subject: ask.subject,
                reason: AdviserFailure::Unreadable,
            },
            Err(reason) => Consultation::FellBack {
                ask: kind,
                subject: ask.subject,
                reason,
            },
        }
    }
}

/// Whether an answer is an answer to this question at all — the pairing
/// `AdviserAnswer`'s tag declares, plus the range each figure must sit
/// in and the option list a `Choice` must name.
fn accepts(kind: AdviserAsk, answer: &AdviserAnswer, options: &[String]) -> bool {
    match (kind, answer) {
        (AdviserAsk::Noul, AdviserAnswer::Noul { confidence_bp, .. }) => {
            *confidence_bp <= BASIS_POINTS
        }
        (AdviserAsk::Score, AdviserAnswer::Score { score_bp }) => *score_bp <= BASIS_POINTS,
        (AdviserAsk::Choice, AdviserAnswer::Choice { chosen }) => {
            options.iter().any(|option| option == chosen)
        }
        (
            AdviserAsk::Noul | AdviserAsk::Score | AdviserAsk::Choice,
            AdviserAnswer::Noul { .. } | AdviserAnswer::Score { .. } | AdviserAnswer::Choice { .. },
        ) => false,
    }
}

/// Basis points of one: `10_000` is a whole, and the figure every
/// adviser answer is scaled to so no payload carries a float.
pub(crate) const BASIS_POINTS: u16 = 10_000;

#[cfg(test)]
mod tests;
