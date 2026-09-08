// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handoff probe, asked of a predecessor and of its successor, and
//! the line that records what survived the crossing.
//!
//! `eval::probe` owns the questions and the comparison and is never in
//! the loop that produces what it measures; this module is that loop.
//! The first reading is taken over the predecessor's transcript before
//! it hands over, the second over the successor's frozen prefix before
//! its first working turn, and the two are compared position by
//! position. The result is an `eval_run` line carrying both sets of
//! answers: `lost` says where to look, and a person reads the two
//! answers themselves rather than a summary of them.
//!
//! **A reading that could not be taken is a row of empty answers**, not
//! an absent reading. The comparison then marks every position lost,
//! which is the truth about a successor nobody could ask.

use kernel::{AxError, EventKind, Model, RunId};
use serde_json::{Map, Value};

use crate::effect;

use super::{Handover, RunWorker, Site};

/// The most a probe answer may cost. Four one-line answers do not need
/// more, and a ceiling is what stops a model that decided to explain
/// itself from charging a person for the measurement.
const PROBE_TOKENS: u64 = 256;

/// How the model is asked to answer, so the two readings are shaped
/// alike enough to be compared: one line per question, in order.
const PROBE_FRAME: &str = "Answer each numbered question on its own line, in order, in as few \
     words as the question allows. Write nothing else.";

fn questions_block(probe: &eval::Probe) -> String {
    let mut text = String::from(PROBE_FRAME);
    text.push('\n');
    for (index, question) in probe.questions().iter().enumerate() {
        text.push_str(&format!("{}. {question}\n", index.saturating_add(1)));
    }
    text
}

/// One line per question, padded with empty strings so the count always
/// matches: an answer the model did not give is a loss, not a refusal
/// to compare.
fn answers_from(said: &str, count: usize) -> Vec<String> {
    let mut answers: Vec<String> = said
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(count)
        .map(|line| {
            line.split_once(". ")
                .filter(|(number, _)| number.chars().all(|glyph| glyph.is_ascii_digit()))
                .map_or(line, |(_, rest)| rest)
                .to_owned()
        })
        .collect();
    answers.resize(count, String::new());
    answers
}

/// What a model said as plain text, or nothing when it said nothing a
/// reader could compare.
fn text_of(returned: &kernel::ModelReturn) -> String {
    kernel::content_from_message(&returned.message)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|block| match block {
            kernel::ContentBlock::Text { text } => Some(text),
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("\n")
}

impl RunWorker {
    /// The first reading: the predecessor asked over its own transcript.
    ///
    /// A model that will not answer reads as a row of empty answers and
    /// the diagnostic log says why; the succession goes on.
    ///
    /// # Errors
    /// Propagates a shipped probe that does not construct, which is a
    /// build defect rather than a run's.
    pub(super) fn probe_before(
        &mut self,
        adapter: Option<&mut (dyn Model + Send + 'static)>,
        frozen: &runtime::Run<runtime::run::Frozen>,
        who: &str,
    ) -> Result<eval::Answers, AxError> {
        let probe = eval::handoff_probe()?;
        let count = probe.questions().len();
        let plan = frozen.plan();
        let mut messages: Vec<kernel::ChatMessage> = frozen
            .transcript()
            .map(|transcript| {
                transcript
                    .lines()
                    .iter()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect()
            })
            .unwrap_or_default();
        messages.push(kernel::ChatMessage {
            role: kernel::Role::User,
            content: vec![kernel::ContentBlock::Text {
                text: questions_block(&probe),
            }],
        });
        let answered = adapter.and_then(|model| {
            let request = kernel::ModelRequest {
                policy: plan.policy.clone(),
                segments: plan.prefix.segment_hashes(),
                chat: kernel::ChatRequest {
                    model: plan.shape.model.clone(),
                    max_tokens: PROBE_TOKENS,
                    system: plan.prefix.system_blocks().ok()?,
                    messages,
                    tools: Vec::new(),
                    effort: plan.shape.effort,
                },
            };
            model.call(&request).ok()
        });
        match answered {
            Some(returned) => probe.answered(answers_from(&text_of(&returned), count)),
            None => {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "eval::probe",
                    &format!("{who} could not be asked before handing over"),
                );
                empty_reading()
            }
        }
    }

    /// The second reading, over the successor's frozen prefix, and the
    /// `eval_run` line comparing it with the first.
    ///
    /// # Errors
    /// Propagates a ledger that refuses the line. The model call itself
    /// never fails this: an unanswerable successor reads as empty.
    pub(super) fn probe_after(
        &mut self,
        site: &mut Site,
        plan: &runtime::RunPlan,
        handed: &Handover,
    ) -> Result<(), AxError> {
        let Ok(probe) = eval::handoff_probe() else {
            return Ok(());
        };
        let count = probe.questions().len();
        let answered = site.adapter.as_mut().and_then(|model| {
            let request = kernel::ModelRequest {
                policy: plan.policy.clone(),
                segments: plan.prefix.segment_hashes(),
                chat: kernel::ChatRequest {
                    model: plan.shape.model.clone(),
                    max_tokens: PROBE_TOKENS,
                    system: plan.prefix.system_blocks().ok()?,
                    messages: vec![kernel::ChatMessage {
                        role: kernel::Role::User,
                        content: vec![kernel::ContentBlock::Text {
                            text: questions_block(&probe),
                        }],
                    }],
                    tools: Vec::new(),
                    effort: plan.shape.effort,
                },
            };
            model.call(&request).ok()
        });
        let answers = match answered {
            Some(returned) => answers_from(&text_of(&returned), count),
            None => vec![String::new(); count],
        };
        let after = probe.answered(answers)?;
        let comparison = eval::compare(&handed.before, &after)?;
        self.record_for(
            plan.run,
            effect::Line {
                who: site.who.clone(),
                addr: plan.addr.clone(),
                kind: EventKind::EvalRun,
                data: kernel::Payload::new(eval_payload(
                    &probe,
                    handed.predecessor,
                    (&handed.before, &after),
                    &comparison,
                ))?,
            },
        )
    }
}

/// A reading with nothing in it, for a probe nobody could ask.
///
/// # Errors
/// Propagates a shipped probe that does not construct, which is a
/// build defect rather than a run's.
fn empty_reading() -> Result<eval::Answers, AxError> {
    let probe = eval::handoff_probe()?;
    let count = probe.questions().len();
    probe.answered(vec![String::new(); count])
}

fn eval_payload(
    probe: &eval::Probe,
    predecessor: RunId,
    readings: (&eval::Answers, &eval::Answers),
    comparison: &eval::Comparison,
) -> Map<String, Value> {
    let (before, after) = readings;
    let strings = |items: &[String]| {
        Value::Array(
            items
                .iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        )
    };
    let mut map = Map::new();
    map.insert("probe".to_owned(), Value::String(probe.id().name.clone()));
    map.insert(
        "version".to_owned(),
        Value::Number(probe.id().version.into()),
    );
    map.insert(
        "predecessor".to_owned(),
        Value::String(predecessor.to_string()),
    );
    map.insert("kept".to_owned(), Value::Number(comparison.kept.into()));
    map.insert(
        "lost".to_owned(),
        Value::Array(
            comparison
                .lost
                .iter()
                .map(|index| Value::Number((*index).into()))
                .collect(),
        ),
    );
    map.insert("before".to_owned(), strings(before.answers()));
    map.insert("after".to_owned(), strings(after.answers()));
    map
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
    fn answers_are_read_one_per_line_and_padded_to_the_count() {
        let said = "1. write the parser\n2. the lexer\n\n3. tokens next";
        assert_eq!(
            answers_from(said, 4),
            vec!["write the parser", "the lexer", "tokens next", ""]
        );
    }

    #[test]
    fn a_reading_nobody_could_take_is_a_row_of_empty_answers() {
        let reading = empty_reading().unwrap();
        assert_eq!(reading.id().name, "handoff");
        assert!(reading.answers().iter().all(String::is_empty));
        assert_eq!(reading.answers().len(), 4);
    }
}
