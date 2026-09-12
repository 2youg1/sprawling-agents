// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person settled about one endpoint, as the book keeps it.
//!
//! The registration says where an endpoint is and how to prove who is
//! calling. This says how the call is made: what to call the endpoint on
//! screen, how long it may take, how many times a failed request is made
//! again, how long a streamed answer may run, the headers every request
//! adds, and the body fields every request writes.
//!
//! **An override's value is kept as text.** A ledger that held `0.2`
//! would hold a float, and this city keeps floats out of its records;
//! the text a person typed round-trips exactly, and what it means as
//! JSON is answered here, once, by [`EndpointTuning::applied_overrides`].

use kernel::Proxying;
use serde_json::Value;

/// How a person set one endpoint up.
///
/// `Default` is "nothing was settled", which is what the caller sends
/// when it has no opinion and what an older record replays as.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointTuning {
    /// What to call this endpoint on screen; absent means its id.
    pub label: Option<String>,
    /// How long one settled request may take.
    pub timeout_ms: Option<u64>,
    /// How many times a request worth making again is made again.
    pub request_max_retries: Option<u32>,
    /// How long a streamed request may run before the city gives up on
    /// it. **This is the whole request's deadline, not an idle timer**:
    /// the blocking transport hands over a body reader with no hook
    /// between chunks, so the city can bound how long an answer takes
    /// and cannot bound how long one silence inside it lasts. A stall
    /// therefore ends here, later than an idle timer would end it.
    pub stream_deadline_ms: Option<u64>,
    /// Headers every request to this endpoint adds, name and value. A
    /// value may be a `secret:realm/name` reference.
    pub extra_headers: Vec<(String, String)>,
    /// Body fields every request writes: a JSON pointer, and the text
    /// of the value to write there.
    pub overrides: Vec<(String, String)>,
    /// Which of this endpoint's calls go through the machine's proxy.
    /// A value rather than an `Option`, because every call has to make
    /// this decision and "nobody settled it" is the default, not a
    /// third state a caller has to handle.
    pub proxying: Proxying,
}

impl EndpointTuning {
    /// Whether this endpoint is called the way any endpoint would be.
    ///
    /// The local adapter has no header surface and no override surface,
    /// so an endpoint that asks for either must take the general path
    /// even when it sits on this machine. Asking here keeps that
    /// judgement in one place rather than in the routing that reads it.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.extra_headers.is_empty() && self.overrides.is_empty()
    }

    /// The overrides as the values they spell.
    ///
    /// The reading is total and has one rule: text that parses as JSON
    /// is that JSON, and text that does not is the string it looks
    /// like. That is what makes `/reasoning/effort` = `high` mean
    /// `"high"` while `/temperature` = `0.2` means the number, without
    /// asking a person to quote anything.
    #[must_use]
    pub fn applied_overrides(&self) -> Vec<(String, Value)> {
        self.overrides
            .iter()
            .map(|(pointer, text)| {
                let value = serde_json::from_str::<Value>(text)
                    .unwrap_or_else(|_| Value::String(text.clone()));
                (pointer.clone(), value)
            })
            .collect()
    }
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
    fn an_override_means_the_json_it_spells_and_otherwise_the_words_it_is() {
        let tuning = EndpointTuning {
            overrides: vec![
                ("/temperature".to_owned(), "0.2".to_owned()),
                ("/reasoning/effort".to_owned(), "high".to_owned()),
                (
                    "/stream_options/include_usage".to_owned(),
                    "true".to_owned(),
                ),
                ("/metadata".to_owned(), r#"{"user":"city"}"#.to_owned()),
            ],
            ..EndpointTuning::default()
        };
        assert_eq!(
            tuning.applied_overrides(),
            vec![
                ("/temperature".to_owned(), serde_json::json!(0.2)),
                ("/reasoning/effort".to_owned(), serde_json::json!("high")),
                (
                    "/stream_options/include_usage".to_owned(),
                    serde_json::json!(true)
                ),
                ("/metadata".to_owned(), serde_json::json!({"user": "city"})),
            ]
        );
    }

    /// A local server reached with a header of its own is still reached
    /// through the general adapter, which is the only one that can send
    /// the header.
    #[test]
    fn an_endpoint_that_asks_for_a_header_is_not_called_plainly() {
        assert!(EndpointTuning::default().is_plain());
        assert!(
            !EndpointTuning {
                extra_headers: vec![("x-tenant".to_owned(), "east".to_owned())],
                ..EndpointTuning::default()
            }
            .is_plain()
        );
    }
}
