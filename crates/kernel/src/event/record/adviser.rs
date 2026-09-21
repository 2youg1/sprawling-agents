// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one consultation of the adviser records: the question put to it,
//! the answer it gave, and the fallback when no answer was used.
//!
//! **The fallback line is the point of the family.** An adviser that is
//! slow, absent or unreadable must not leave the city's behaviour to be
//! inferred: the deterministic strategy answered instead, and that is a
//! fact about the move the city made. Without this line a window that no
//! adviser shaped and a window an adviser shaped would fold to the same
//! history.
//!
//! **Two questions never appear here.** The model a run talks to and the
//! effort it thinks with are frozen before the prefix exists, so an
//! adviser never answers them; the one `Choice` this vocabulary holds is
//! asked when a session's model is chosen, before anything is frozen.

use serde::{Deserialize, Serialize};

/// The three questions the adviser port takes. Closed: the answer types
/// below pair with these variant for variant, so a reader can never pair
/// an answer with a question that was not asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserAsk {
    /// "Is this still needed?" - asked of one item that is about to
    /// leave or stay in the window.
    Noul,
    /// "How dense is this text?" - the figure the window's budget
    /// spends before it cuts.
    Score,
    /// "Which of these?" - asked when a session's model is chosen, and
    /// nowhere else.
    Choice,
}

/// `adviser_asked`: which question was put, and about what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserAsked {
    /// The question.
    pub ask: AdviserAsk,
    /// What it was about, in the asking side's own words - a tool call's
    /// id, a slot's name. The caller owns the spelling, as it does for
    /// `ToolCalled.id`.
    pub subject: String,
}

/// How the adviser answered.
///
/// The tag is the question, and the answer types are the three answer
/// shapes: this is why one enum rather than one answer type with three
/// optional fields, which would let a line hold an answer to a question
/// nobody asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "ask", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserAnswer {
    /// `keep` is the answer to "is this still needed?"; `confidence_bp`
    /// is how sure the adviser was, in basis points of one. A
    /// probability reaches the ledger scaled to an integer, because no
    /// payload here carries a float.
    Noul { keep: bool, confidence_bp: u16 },
    /// The density, in basis points of one.
    Score { score_bp: u16 },
    /// The chosen option, as the asking side spelled it in the question.
    Choice { chosen: String },
}

/// `adviser_answered`: what the adviser said, and how long it took.
///
/// This is the line a replay reads instead of asking again: an answer
/// that reached the window is as much a recorded fact as the reply a
/// model returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserAnswered {
    /// What the question was about, spelled as [`AdviserAsked`] spelled
    /// it.
    pub subject: String,
    #[serde(flatten)]
    pub answer: AdviserAnswer,
    /// How long the adviser took. Recorded because a second opinion on
    /// the city's critical path is a cost, and one that grew has to be
    /// visible without re-running it.
    pub elapsed_ms: u64,
}

/// Why no adviser answer reached the window. Closed: a fallback this
/// vocabulary cannot name is a fallback nobody can count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserFailure {
    /// No endpoint was attached for the adviser, or the port refused the
    /// call.
    Unavailable,
    /// The port gave up waiting.
    Timeout,
    /// An answer arrived and it is not the answer to the question that
    /// was asked. The words are not kept: what a reader needs is that
    /// the deterministic strategy answered instead.
    Unreadable,
}

/// `adviser_fell_back`: the deterministic strategy answered, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserFellBack {
    /// What the question was about, spelled as [`AdviserAsked`] spelled
    /// it. What was asked is not repeated here: the asked line carries
    /// it, and two homes for it could disagree.
    pub subject: String,
    /// Why the adviser could not answer.
    pub reason: AdviserFailure,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::event::Payload;

    #[test]
    fn an_asked_line_carries_the_question_and_its_subject() {
        let asked = AdviserAsked {
            ask: AdviserAsk::Noul,
            subject: "toolu_01".to_owned(),
        };
        let payload = Payload::of(&asked).unwrap();
        assert_eq!(
            serde_json::to_string(&payload).unwrap(),
            "{\"ask\":\"noul\",\"subject\":\"toolu_01\"}"
        );
        assert_eq!(payload.read::<AdviserAsked>().unwrap(), asked);
    }

    /// The three answer shapes, one line each: the tag says which
    /// question the answer belongs to, so a reader never has to find the
    /// asked line to interpret it.
    #[test]
    fn each_answer_shape_keeps_its_own_keys_under_one_tag() {
        for (answer, wire) in [
            (
                AdviserAnswer::Noul {
                    keep: false,
                    confidence_bp: 8_750,
                },
                "{\"ask\":\"noul\",\"confidence_bp\":8750,\"elapsed_ms\":41,\"keep\":false,\"subject\":\"toolu_01\"}",
            ),
            (
                AdviserAnswer::Score { score_bp: 420 },
                "{\"ask\":\"score\",\"elapsed_ms\":41,\"score_bp\":420,\"subject\":\"toolu_01\"}",
            ),
            (
                AdviserAnswer::Choice {
                    chosen: "claude-sonnet-5".to_owned(),
                },
                "{\"ask\":\"choice\",\"chosen\":\"claude-sonnet-5\",\"elapsed_ms\":41,\"subject\":\"toolu_01\"}",
            ),
        ] {
            let answered = AdviserAnswered {
                subject: "toolu_01".to_owned(),
                answer,
                elapsed_ms: 41,
            };
            let payload = Payload::of(&answered).unwrap();
            assert_eq!(serde_json::to_string(&payload).unwrap(), wire);
            assert_eq!(payload.read::<AdviserAnswered>().unwrap(), answered);
        }
    }

    #[test]
    fn a_fallback_line_names_the_subject_and_the_reason() {
        let fell = AdviserFellBack {
            subject: "toolu_01".to_owned(),
            reason: AdviserFailure::Timeout,
        };
        let payload = Payload::of(&fell).unwrap();
        assert_eq!(
            serde_json::to_string(&payload).unwrap(),
            "{\"reason\":\"timeout\",\"subject\":\"toolu_01\"}"
        );
        assert_eq!(payload.read::<AdviserFellBack>().unwrap(), fell);
    }
}
