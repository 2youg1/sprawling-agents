// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{AdviserAnswer, AdviserAsk, AdviserFailure};
use kernel::model::content_from_message;
use kernel::{
    AxCode, AxError, B3Hash, BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ContentBlock,
    Model, ModelRequest, Role, SystemBlock,
};
use serde_json::Value;

use crate::endpoint::Redemption;
use crate::router::Chosen;

const INSTRUCTION: &str = "\
You are a consultant to a coding agent. You are shown a conversation and one \
question about it. Answer with a single JSON object and nothing else.\n\
question noul: {\"keep\": <true|false>, \"confidence_bp\": <0..10000>} — is the \
material still needed?\n\
question score: {\"score_bp\": <0..10000>} — how dense is the material, as a \
share of one?\n\
question choice: {\"chosen\": \"<one of the options>\"} — which option should \
be used? Answer with that object only, never with prose.";

const ADVISER_MAX_TOKENS: u64 = 256;

pub struct Question<'a> {
    pub ask: AdviserAsk,
    pub subject: &'a str,
    pub options: &'a [String],
    pub material: Option<&'a str>,
}

pub struct AdviserClient {
    model: Box<dyn Model + Send>,
    model_id: String,
}

impl AdviserClient {
    pub fn attached(
        chosen: &Chosen<'_>,
        redemption: Redemption,
        dialect_headers: Vec<(String, String)>,
        monotonic: fn() -> std::time::Instant,
    ) -> Result<AdviserClient, AxError> {
        let model_id = chosen.entry.id.clone();
        let model = crate::adapter_for(chosen, redemption, dialect_headers, monotonic)?;
        Ok(AdviserClient { model, model_id })
    }

    pub fn ask(
        &mut self,
        question: Question<'_>,
        policy: &BuildingPolicy,
        window: &[ChatMessage],
    ) -> Result<AdviserAnswer, AdviserFailure> {
        let word = ask_word(question.ask)?;
        let mut messages = window.to_vec();
        messages.push(ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: question_text(&word, &question),
            }],
        });
        let request = ModelRequest {
            policy: policy.clone(),

            segments: [B3Hash::digest(INSTRUCTION.as_bytes()); 4],
            chat: ChatRequest {
                model: self.model_id.clone(),
                max_tokens: Ceiling::new(ADVISER_MAX_TOKENS),
                system: vec![SystemBlock {
                    text: INSTRUCTION.to_owned(),
                    cache: false,
                }],
                messages: std::borrow::Cow::Owned(messages),
                tools: std::borrow::Cow::Borrowed(&[]),
                breakpoint: kernel::MessageBreakpoint::Unmarked,
                shared: kernel::SharedPrefix::Unshared,
                effort: None,
                generation: kernel::GenerationSettings::default(),
            },
        };
        let returned = self.model.call(&request).map_err(|err| {
            if *err.code() == AxCode::Timeout {
                AdviserFailure::Timeout
            } else {
                AdviserFailure::Unavailable
            }
        })?;
        let blocks =
            content_from_message(&returned.message).map_err(|_| AdviserFailure::Unreadable)?;
        let text = blocks
            .into_iter()
            .find_map(|block| match block {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. }
                | ContentBlock::Image(_)
                | ContentBlock::ToolResult { .. } => None,
            })
            .ok_or(AdviserFailure::Unreadable)?;
        parse_answer(question.ask, question.options, &text)
    }
}

fn ask_word(ask: AdviserAsk) -> Result<String, AdviserFailure> {
    match serde_json::to_value(ask) {
        Ok(Value::String(word)) => Ok(word),
        Ok(_) | Err(_) => Err(AdviserFailure::Unreadable),
    }
}

fn question_text(word: &str, question: &Question<'_>) -> String {
    let mut text = format!("question: {word}\nsubject: {}", question.subject);
    if !question.options.is_empty() {
        text.push_str("\noptions:");
        for option in question.options {
            text.push_str("\n- ");
            text.push_str(option);
        }
    }
    if let Some(material) = question.material {
        text.push_str("\nmaterial:\n");
        text.push_str(material);
    }
    text
}

fn parse_answer(
    ask: AdviserAsk,
    options: &[String],
    text: &str,
) -> Result<AdviserAnswer, AdviserFailure> {
    let value = json_in(text)?;
    match ask {
        AdviserAsk::Noul => {
            let keep = value
                .get("keep")
                .and_then(Value::as_bool)
                .ok_or(AdviserFailure::Unreadable)?;
            let confidence_bp = basis_points(&value, "confidence_bp")?;
            Ok(AdviserAnswer::Noul {
                keep,
                confidence_bp,
            })
        }
        AdviserAsk::Score => Ok(AdviserAnswer::Score {
            score_bp: basis_points(&value, "score_bp")?,
        }),
        AdviserAsk::Choice => {
            let chosen = value
                .get("chosen")
                .and_then(Value::as_str)
                .ok_or(AdviserFailure::Unreadable)?
                .to_owned();
            if !options.iter().any(|option| option == &chosen) {
                return Err(AdviserFailure::Unreadable);
            }
            Ok(AdviserAnswer::Choice { chosen })
        }
    }
}

fn basis_points(value: &Value, key: &str) -> Result<u16, AdviserFailure> {
    let raw = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(AdviserFailure::Unreadable)?;
    u16::try_from(raw).map_err(|_| AdviserFailure::Unreadable)
}

fn json_in(text: &str) -> Result<Value, AdviserFailure> {
    let trimmed = text.trim();
    let opened = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map_or(trimmed, str::trim_start);
    let fenced = opened.strip_suffix("```").map_or(opened, str::trim_end);
    if let Ok(value) = serde_json::from_str::<Value>(fenced) {
        return Ok(value);
    }
    let start = fenced.find('{').ok_or(AdviserFailure::Unreadable)?;
    let end = fenced.rfind('}').ok_or(AdviserFailure::Unreadable)?;
    let object = fenced.get(start..=end).ok_or(AdviserFailure::Unreadable)?;
    serde_json::from_str(object).map_err(|_| AdviserFailure::Unreadable)
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

    #[test]
    fn every_question_word_comes_from_the_kernels_own_spelling() {
        assert_eq!(ask_word(AdviserAsk::Noul).unwrap(), "noul");
        assert_eq!(ask_word(AdviserAsk::Score).unwrap(), "score");
        assert_eq!(ask_word(AdviserAsk::Choice).unwrap(), "choice");
    }

    #[test]
    fn a_fenced_answer_between_sentences_is_still_read() {
        let answer = parse_answer(
            AdviserAsk::Noul,
            &[],
            "Sure.\n```json\n{\"keep\": true, \"confidence_bp\": 8750}\n```\n",
        )
        .unwrap();
        assert_eq!(
            answer,
            AdviserAnswer::Noul {
                keep: true,
                confidence_bp: 8_750
            }
        );
    }

    #[test]
    fn an_answer_to_another_question_is_unreadable() {
        let answer = parse_answer(
            AdviserAsk::Score,
            &[],
            "{\"keep\": true, \"confidence_bp\": 1}",
        );
        assert_eq!(answer, Err(AdviserFailure::Unreadable));
        let missing = parse_answer(AdviserAsk::Noul, &[], "{\"confidence_bp\": 1}");
        assert_eq!(missing, Err(AdviserFailure::Unreadable));
    }

    #[test]
    fn an_option_that_was_never_offered_is_unreadable() {
        let options = vec!["sonnet".to_owned(), "opus".to_owned()];
        let answer = parse_answer(AdviserAsk::Choice, &options, "{\"chosen\": \"haiku\"}");
        assert_eq!(answer, Err(AdviserFailure::Unreadable));
        let chosen = parse_answer(AdviserAsk::Choice, &options, "{\"chosen\": \"opus\"}").unwrap();
        assert_eq!(
            chosen,
            AdviserAnswer::Choice {
                chosen: "opus".to_owned()
            }
        );
    }

    #[test]
    fn the_question_carries_its_subject_options_and_material() {
        let options = vec!["a".to_owned(), "b".to_owned()];
        let question = Question {
            ask: AdviserAsk::Choice,
            subject: "dispatch",
            options: &options,
            material: None,
        };
        let text = question_text("choice", &question);
        assert!(text.starts_with("question: choice\nsubject: dispatch"));
        assert!(text.contains("- a") && text.contains("- b"));

        let question = Question {
            ask: AdviserAsk::Noul,
            subject: "toolu_01",
            options: &[],
            material: Some("the material under judgement"),
        };
        let text = question_text("noul", &question);
        assert!(text.ends_with("material:\nthe material under judgement"));
    }
}
