// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserAsk {
    Noul,

    Score,

    Choice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserAsked {
    pub ask: AdviserAsk,

    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "ask", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserAnswer {
    Noul { keep: bool, confidence_bp: u16 },

    Score { score_bp: u16 },

    Choice { chosen: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserAnswered {
    pub subject: String,
    #[serde(flatten)]
    pub answer: AdviserAnswer,

    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdviserFailure {
    Unavailable,

    Timeout,

    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviserFellBack {
    pub subject: String,

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
