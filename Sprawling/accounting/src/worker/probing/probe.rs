// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbeId {
    pub name: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Probe {
    id: ProbeId,
    questions: Vec<String>,
}

impl Probe {
    pub fn new(id: ProbeId, questions: Vec<String>) -> Result<Probe, AxError> {
        if questions.is_empty() {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "build a probe",
                format!("{} v{} asks nothing", id.name, id.version),
            )
            .with_recovery("ask at least one question, or do not build the probe"));
        }
        if questions.iter().any(|question| question.trim().is_empty()) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "build a probe",
                format!("{} v{} has a blank question", id.name, id.version),
            )
            .with_recovery(
                "remove the blank; a question nobody can answer always reads as a loss",
            ));
        }
        Ok(Probe { id, questions })
    }

    #[must_use]
    pub fn id(&self) -> &ProbeId {
        &self.id
    }

    #[must_use]
    pub fn questions(&self) -> &[String] {
        &self.questions
    }

    pub fn answered(&self, answers: Vec<String>) -> Result<Answers, AxError> {
        if answers.len() != self.questions.len() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "record probe answers",
                format!(
                    "{} answers for {} questions",
                    answers.len(),
                    self.questions.len()
                ),
            )
            .with_recovery(
                "answer every question, in order; an unanswered one is an empty string",
            ));
        }
        Ok(Answers {
            id: self.id.clone(),
            answers,
        })
    }
}

pub(crate) fn handoff_probe() -> Result<Probe, AxError> {
    Probe::new(
        ProbeId {
            name: "handoff".to_owned(),
            version: 1,
        },
        [
            "What is the task, in one line?",
            "What has been done so far, in one line?",
            "What is the next step, in one line?",
            "Which file must be read first? Answer with its path only.",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answers {
    id: ProbeId,
    answers: Vec<String>,
}

impl Answers {
    #[must_use]
    pub fn answers(&self) -> &[String] {
        &self.answers
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub kept: u32,

    pub lost: Vec<u32>,
}

pub fn compare(before: &Answers, after: &Answers) -> Result<Comparison, AxError> {
    if before.id != after.id {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "compare probe readings",
            format!(
                "{} v{} against {} v{}",
                before.id.name, before.id.version, after.id.name, after.id.version
            ),
        )
        .with_recovery(
            "compare readings of one edition; a probe whose questions changed is a different \
             instrument, and mixing the two measures the instrument",
        ));
    }
    let mut kept: u32 = 0;
    let mut lost = Vec::new();
    for (index, (was, now)) in before.answers.iter().zip(after.answers.iter()).enumerate() {
        let position = u32::try_from(index).unwrap_or(u32::MAX);
        if was.trim() == now.trim() {
            kept = kept.saturating_add(1);
        } else {
            lost.push(position);
        }
    }
    Ok(Comparison { kept, lost })
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
    fn the_shipped_handoff_probe_asks_four_questions_in_edition_one() {
        let probe = super::handoff_probe().unwrap();
        assert_eq!(probe.id().name, "handoff");
        assert_eq!(probe.id().version, 1);
        assert_eq!(probe.questions().len(), 4);
    }

    fn handoff_probe(version: u32) -> Probe {
        Probe::new(
            ProbeId {
                name: "handoff".to_owned(),
                version,
            },
            vec![
                "what is the next step".to_owned(),
                "which card is open".to_owned(),
                "what did the last red run find".to_owned(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn a_handoff_that_carried_everything_reads_as_intact() {
        let probe = handoff_probe(1);
        let answers = vec![
            "wire the pr tool".to_owned(),
            "the checkpoint before the merge".to_owned(),
            "the trunk moved under a waiting request".to_owned(),
        ];
        let before = probe.answered(answers.clone()).unwrap();
        let after = probe.answered(answers).unwrap();
        let comparison = compare(&before, &after).unwrap();
        assert!(comparison.lost.is_empty());
        assert_eq!(comparison.kept, 3);
    }

    #[test]
    fn a_loss_is_reported_by_position_rather_than_as_a_score() {
        let probe = handoff_probe(1);
        let before = probe
            .answered(vec![
                "wire the pr tool".to_owned(),
                "the checkpoint before the merge".to_owned(),
                "the trunk moved".to_owned(),
            ])
            .unwrap();
        let after = probe
            .answered(vec![
                "wire the pr tool".to_owned(),
                "the checkpoint before the merge".to_owned(),
                String::new(),
            ])
            .unwrap();
        let comparison = compare(&before, &after).unwrap();
        assert!(!comparison.lost.is_empty());
        assert_eq!(comparison.lost, vec![2]);
        assert_eq!(comparison.kept, 2);
    }

    #[test]
    fn two_editions_are_two_instruments() {
        let first = handoff_probe(1);
        let second = handoff_probe(2);
        let before = first
            .answered(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
            .unwrap();
        let after = second
            .answered(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
            .unwrap();
        let refused = compare(&before, &after).unwrap_err();
        assert_eq!(refused.code(), &AxCode::InvalidArgs);
        assert!(refused.recovery().contains("instrument"));
    }

    #[test]
    fn answers_must_line_up_with_questions() {
        let probe = handoff_probe(1);
        assert!(probe.answered(vec!["a".to_owned()]).is_err());
        assert!(
            Probe::new(
                ProbeId {
                    name: "empty".to_owned(),
                    version: 1
                },
                Vec::new()
            )
            .is_err()
        );
    }
}
