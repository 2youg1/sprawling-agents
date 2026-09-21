// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The adviser's transport: one attached endpoint, three typed
//! questions, and an answer parsed strictly enough that a wrong one is
//! called unreadable rather than believed.
//!
//! **No second provider table.** The adviser is an endpoint a person
//! attached through the ordinary `AttachEndpoint` command, and this
//! module takes the [`Chosen`] the router already produced. Credential
//! redemption, dialect, deadlines, headers and proxying are therefore
//! the registered ones, and an adviser cannot be reached a way no other
//! model call in this city can.
//!
//! **The adviser sees the window and nothing else.** The conversation is
//! the parameter; no system prefix, no frozen config, no tool defs
//! travel with it. That is the whole difference between an adviser and a
//! second model call, and it is enforced by what this signature can
//! name.
//!
//! **A failure is not an error here.** [`AdviserClient::ask`] answers
//! `Result<AdviserAnswer, AdviserFailure>`: an endpoint that is
//! unreachable, a deadline that passed and a reply that is not an answer
//! are all outcomes the caller records and falls back from, because the
//! worst case the city may have is the behaviour it had before advisers
//! existed.

use kernel::event::record::{AdviserAnswer, AdviserAsk, AdviserFailure};
use kernel::model::content_from_message;
use kernel::{
    AxCode, AxError, B3Hash, BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ContentBlock,
    Model, ModelRequest, Role, SystemBlock,
};
use serde_json::Value;

use crate::endpoint::Redemption;
use crate::router::Chosen;

/// The one system block every answer is asked under.
const INSTRUCTION: &str = "\
You are a consultant to a coding agent. You are shown a conversation and one \
question about it. Answer with a single JSON object and nothing else.\n\
question noul: {\"keep\": <true|false>, \"confidence_bp\": <0..10000>} — is the \
material still needed?\n\
question score: {\"score_bp\": <0..10000>} — how dense is the material, as a \
share of one?\n\
question choice: {\"chosen\": \"<one of the options>\"} — which option should \
be used? Answer with that object only, never with prose.";

/// How many tokens an answer may take. An adviser answers with one small
/// object, and a longer reply is a reply to something it was not asked.
const ADVISER_MAX_TOKENS: u64 = 256;

/// One question, as the wire-side client takes it.
///
/// `material` is the text under judgement, which is part of what was
/// asked and not a second conversation; `options` is non-empty only for
/// a choice.
pub struct Question<'a> {
    pub ask: AdviserAsk,
    pub subject: &'a str,
    pub options: &'a [String],
    pub material: Option<&'a str>,
}

/// A consulted endpoint: the adapter other calls use, held together with
/// the model id the question is addressed to.
pub struct AdviserClient {
    model: Box<dyn Model + Send>,
    model_id: String,
}

impl AdviserClient {
    /// Builds the client from the endpoint the router chose.
    ///
    /// # Errors
    /// Propagates whatever [`crate::adapter_for`] says about building the
    /// adapter — an unknown dialect, an unreadable credential reference,
    /// an endpoint that cannot be reached a legal way.
    pub fn attached(
        chosen: &Chosen<'_>,
        redemption: Redemption,
        dialect_headers: Vec<(String, String)>,
    ) -> Result<AdviserClient, AxError> {
        let model_id = chosen.entry.id.clone();
        let model = crate::adapter_for(chosen, redemption, dialect_headers)?;
        Ok(AdviserClient { model, model_id })
    }

    /// Asks one question and returns the answer, or why there is none.
    ///
    /// The reply is read for its text only: a consultant asked to answer
    /// with an object is not allowed to run the tools it may think it has,
    /// and a reply that arrives with tool calls still has to carry the
    /// object.
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
            // The adviser is not a run: it has no frozen prefix, so all
            // four segment slots carry the hash of the one block that
            // stands where a prefix would. Nothing writes this request to
            // a ledger — the answer is what enters one.
            segments: [B3Hash::digest(INSTRUCTION.as_bytes()); 4],
            chat: ChatRequest {
                model: self.model_id.clone(),
                max_tokens: Ceiling::new(ADVISER_MAX_TOKENS),
                system: vec![SystemBlock {
                    text: INSTRUCTION.to_owned(),
                    cache: false,
                }],
                messages,
                tools: Vec::new(),
                effort: None,
            },
        };
        let returned = self.model.call(&request).map_err(|err| {
            // A deadline is the one transport failure this side can name;
            // every other refusal, outage or unreadable body is the same
            // fact to the city — no answer arrived.
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

/// How the kernel spells one question, read from its own serde shape
/// rather than spelled again here: a second list of the three words would
/// be a place the wire and the question could disagree.
fn ask_word(ask: AdviserAsk) -> Result<String, AdviserFailure> {
    match serde_json::to_value(ask) {
        Ok(Value::String(word)) => Ok(word),
        Ok(_) | Err(_) => Err(AdviserFailure::Unreadable),
    }
}

/// One question as the model reads it: the word, the subject, the
/// options when there are any, and the material under judgement.
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

/// The answer in a reply, or `Unreadable`.
///
/// A model sometimes wraps its object in a fence or a sentence. The
/// object is taken when one is there and the reply is called unreadable
/// when it is not; nothing here guesses what an answer meant.
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

/// One basis-point figure. The range — a share of one — belongs to the
/// port that acts on the figure, so a value out of range is refused
/// there rather than here.
fn basis_points(value: &Value, key: &str) -> Result<u16, AdviserFailure> {
    let raw = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(AdviserFailure::Unreadable)?;
    u16::try_from(raw).map_err(|_| AdviserFailure::Unreadable)
}

/// The first JSON object in the reply, fences and prose around it
/// tolerated.
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
