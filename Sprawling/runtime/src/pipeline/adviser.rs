// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{
    AdviserAnswer, AdviserAnswered, AdviserAsked, AdviserFailure, AdviserFellBack,
};
use kernel::{AxError, Payload};

use crate::conversation::Conversation;

pub use kernel::event::record::AdviserAsk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub kind: AdviserAsk,
    pub subject: String,
    pub options: Vec<String>,
    pub material: Option<String>,
}

impl Ask {
    pub fn noul(subject: impl Into<String>, material: impl Into<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Noul,
            subject: subject.into(),
            options: Vec::new(),
            material: Some(material.into()),
        }
    }

    pub fn score(subject: impl Into<String>, material: impl Into<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Score,
            subject: subject.into(),
            options: Vec::new(),
            material: Some(material.into()),
        }
    }

    pub fn choice(subject: impl Into<String>, options: Vec<String>) -> Ask {
        Ask {
            kind: AdviserAsk::Choice,
            subject: subject.into(),
            options,
            material: None,
        }
    }
}

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
    #[must_use]
    pub fn answer(&self) -> Option<&AdviserAnswer> {
        match self {
            Consultation::Answered { answer, .. } => Some(answer),
            Consultation::FellBack { .. } => None,
        }
    }

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

type Answer = Box<
    dyn for<'a, 'b> FnMut(&'a Ask, &'b Conversation) -> Result<AdviserAnswer, AdviserFailure>
        + Send,
>;

pub struct Adviser {
    answer: Option<Answer>,
}

impl Adviser {
    #[must_use]
    pub fn none() -> Adviser {
        Adviser { answer: None }
    }

    pub fn with(
        answer: impl for<'a, 'b> FnMut(
            &'a Ask,
            &'b Conversation,
        ) -> Result<AdviserAnswer, AdviserFailure>
        + Send
        + 'static,
    ) -> Adviser {
        Adviser {
            answer: Some(Box::new(answer)),
        }
    }

    pub fn consult(
        &mut self,
        ask: Ask,
        conversation: &Conversation,
        elapsed_ms: u64,
    ) -> Consultation {
        let kind = ask.kind;
        let Some(answer) = self.answer.as_mut() else {
            return Consultation::FellBack {
                ask: kind,
                subject: ask.subject,
                reason: AdviserFailure::Unavailable,
            };
        };
        match answer(&ask, conversation) {
            Ok(answer) if accepts(kind, &answer, &ask.options) => Consultation::Answered {
                ask: kind,
                subject: ask.subject,
                answer,
                elapsed_ms,
            },

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

pub(crate) const BASIS_POINTS: u16 = 10_000;

#[cfg(test)]
mod tests;
