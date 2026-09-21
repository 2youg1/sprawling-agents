// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a `/rerank` call looks like on the wire, in both directions
//! (shape 1 decision).
//!
//! **Rerank has no OpenAI shape to copy.** The face was defined by the
//! servers that serve it, and the two this city targets —
//! text-embeddings-inference and infinity — answer the same request:
//! one question, a list of passages, and an answer that is a bare
//! array of `{index, score}` in the server's own ranked order.
//!
//! The ranking is the server's and is never recomputed here: scores
//! from two models are not comparable, sorting them again would be a
//! second authority on which passage answers a question, and this city
//! would then disagree with the model it paid to ask.
//!
//! Source, read on 2026-09-21:
//! `huggingface/text-embeddings-inference` `docs/openapi.json`, the
//! `/rerank` path with schemas `RerankRequest`, `RerankResponse` and
//! `Rank`.

use kernel::{AxCode, AxError};
use serde_json::Value;

/// One call to a rerank face.
#[derive(Debug)]
pub struct RerankRequest {
    query: String,
    passages: Vec<String>,
}

impl RerankRequest {
    /// One question against the passages that might answer it.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the question is empty or no passage was
    /// given: a ranking of nothing is an answer no caller can use, and
    /// the servers differ in whether they refuse it or return an empty
    /// array.
    pub fn new(query: String, passages: Vec<String>) -> Result<RerankRequest, AxError> {
        if query.is_empty() || passages.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "ask for a ranking",
                "the request carries no question or no passage to rank",
            )
            .with_recovery(
                "send one question and at least one passage: a ranking of an empty list \
                 is not an answer this city can act on",
            ));
        }
        Ok(RerankRequest { query, passages })
    }

    /// How many passages were sent, which bounds every index the
    /// answer may carry.
    #[must_use]
    pub fn passages(&self) -> usize {
        self.passages.len()
    }

    /// The JSON body.
    ///
    /// `return_text` stays at the server's default of `false`: the
    /// caller already holds the passages it sent, and asking the
    /// server to send them back doubles the bytes to confirm what the
    /// index already states.
    #[must_use]
    pub fn body(&self) -> String {
        serde_json::json!({
            "query": self.query,
            "texts": self.passages,
        })
        .to_string()
    }
}

/// One passage's place in the answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rank {
    /// Which of the sent passages this row is about.
    pub passage: usize,
    /// What the model scored it. Comparable within one answer from one
    /// model, and meaningless between two.
    pub score: f64,
}

/// What a rerank face answered: the sent passages, best first.
#[derive(Debug)]
pub struct Ranking {
    ranks: Vec<Rank>,
}

impl Ranking {
    /// Read an answer against the request that asked for it.
    ///
    /// # Errors
    /// `E_PROVIDER` when the answer is not an array, when a row states
    /// no readable index or score, when an index names a passage that
    /// was never sent, or when one passage is ranked twice. An index
    /// out of range is the failure that matters: a caller using it to
    /// pick a passage would read the wrong one, or none.
    pub fn parse(answer: &Value, asked: &RerankRequest) -> Result<Ranking, AxError> {
        let rows = answer
            .as_array()
            .ok_or_else(|| unreadable("the answer is not the array of ranks this face returns"))?;
        let mut ranks = Vec::with_capacity(rows.len());
        let mut seen = vec![false; asked.passages()];
        for row in rows {
            let passage = row
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| unreadable("a rank came back without a readable `index`"))?;
            let score = row
                .get("score")
                .and_then(Value::as_f64)
                .ok_or_else(|| unreadable("a rank came back without a readable `score`"))?;
            let slot = seen
                .get_mut(passage)
                .ok_or_else(|| unreadable("a rank names a passage that was never sent"))?;
            if *slot {
                return Err(unreadable("one passage was ranked twice"));
            }
            *slot = true;
            ranks.push(Rank { passage, score });
        }
        Ok(Ranking { ranks })
    }

    /// The ranks in the order the model put them: the server sorts,
    /// and this city does not sort again.
    #[must_use]
    pub fn ranks(&self) -> &[Rank] {
        &self.ranks
    }

    /// The passage the model put first, or `None` when it ranked none.
    #[must_use]
    pub fn best(&self) -> Option<Rank> {
        self.ranks.first().copied()
    }
}

/// An answer this version cannot read, stated without quoting it.
fn unreadable(detail: &str) -> AxError {
    AxError::failure(AxCode::Provider, "read a ranking", detail.to_owned()).with_recovery(
        "check that this endpoint serves the text-embeddings-inference rerank face: a \
         rank naming a passage nobody sent would answer a question with somebody else's \
         text",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn asked() -> RerankRequest {
        RerankRequest::new(
            "What is deep learning?".to_owned(),
            vec![
                "Deep learning is ...".to_owned(),
                "Lunch is at one.".to_owned(),
                "A neural network ...".to_owned(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn the_request_sends_the_question_and_the_passages_and_asks_for_no_echo() {
        let body: Value = serde_json::from_str(&asked().body()).unwrap();
        assert_eq!(body["query"], "What is deep learning?");
        assert_eq!(body["texts"][1], "Lunch is at one.");
        assert!(
            body.get("return_text").is_none(),
            "the caller already holds what it sent"
        );
    }

    #[test]
    fn an_empty_question_or_an_empty_list_is_refused() {
        assert_eq!(
            *RerankRequest::new(String::new(), vec!["a".to_owned()])
                .unwrap_err()
                .code(),
            AxCode::InvalidArgs
        );
        assert!(RerankRequest::new("q".to_owned(), Vec::new()).is_err());
    }

    /// The model's order is the answer. A city that re-sorted would be
    /// a second opinion on the one thing it paid the model for.
    #[test]
    fn the_ranks_keep_the_order_the_model_put_them_in() {
        let answer = serde_json::json!([
            {"index": 2, "score": 0.98},
            {"index": 0, "score": 0.81},
            {"index": 1, "score": 0.02},
        ]);
        let read = Ranking::parse(&answer, &asked()).unwrap();
        assert_eq!(
            read.best(),
            Some(Rank {
                passage: 2,
                score: 0.98
            })
        );
        assert_eq!(read.ranks()[2].passage, 1);
    }

    #[test]
    fn a_rank_naming_a_passage_nobody_sent_is_refused() {
        let answer = serde_json::json!([{"index": 9, "score": 0.5}]);
        let refusal = Ranking::parse(&answer, &asked()).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::Provider);
        assert!(refusal.subject().contains("never sent"));
    }

    #[test]
    fn a_repeated_passage_and_an_unreadable_row_are_both_refused() {
        let twice = serde_json::json!([
            {"index": 1, "score": 0.5},
            {"index": 1, "score": 0.4},
        ]);
        assert!(Ranking::parse(&twice, &asked()).is_err());
        let scoreless = serde_json::json!([{"index": 1}]);
        assert!(Ranking::parse(&scoreless, &asked()).is_err());
        let object = serde_json::json!({"results": []});
        assert!(Ranking::parse(&object, &asked()).is_err());
    }

    /// A server that ranks only the passages it considers relevant
    /// answers with fewer rows than were sent, which is its judgement
    /// rather than a short answer.
    #[test]
    fn a_shorter_answer_than_the_request_is_a_ranking_too() {
        let answer = serde_json::json!([{"index": 0, "score": 0.9}]);
        let read = Ranking::parse(&answer, &asked()).unwrap();
        assert_eq!(read.ranks().len(), 1);
    }
}
