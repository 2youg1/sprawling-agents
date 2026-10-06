// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person settled about one endpoint, as the book keeps it,
//! and what holds for an endpoint they settled nothing about.
//!
//! The registration says where an endpoint is and how to prove who is
//! calling. This says how the call is made: what to call the endpoint on
//! screen, how long it may take, how many times a failed request is made
//! again, how long a streamed answer may go silent, the headers every
//! request adds, and the body fields every request writes.
//!
//! **An override's value is kept as text.** A ledger that held `0.2`
//! would hold a float, and this city keeps floats out of its records;
//! the text a person typed round-trips exactly, and what it means as
//! JSON is answered here, once, by [`EndpointTuning::applied_overrides`].

use kernel::Proxying;

use crate::concurrency::MaxInFlight;
use serde_json::Value;

use crate::endpoint::HeaderValue;

use kernel::Retries;

/// What every endpoint is called with until a person says otherwise.
///
/// **The one home of these three figures.** A form that printed its own
/// numbers into empty boxes made an endpoint attached from a form
/// behave differently from one attached from a configuration file,
/// while the person had filled in nothing either way; the form shows
/// these as the placeholders they are, through the configuration query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuningDefaults {
    /// How long one settled request may take, in total.
    pub timeout_ms: u64,
    /// The ceiling on making a failed request again.
    pub retries: Retries,
    /// How long a streamed answer may go silent. Absent means a stream
    /// is held to the same bound as a settled call, which is the only
    /// figure this city can state without inventing one.
    pub stream_idle_timeout_ms: Option<u64>,
}

/// How a person set one endpoint up.
///
/// `Default` is "nothing was settled", which is what the caller sends
/// when it has no opinion and what an older record replays as.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EndpointTuning {
    /// Explicit ordered accounts; absence leaves the existing accounts intact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<kernel::event::record::ProviderAccount>>,
    /// What to call this endpoint on screen; absent means its id.
    pub label: Option<String>,
    /// How long one settled request may take.
    pub timeout_ms: Option<u64>,
    /// How many times a request worth making again is made again.
    /// Absence is a value here rather than a question each reader
    /// answers for itself.
    pub request_max_retries: Retries,
    /// How long a streamed request may go without a byte arriving
    /// before the city gives up on it. **This is an idle bound, not a
    /// deadline on the whole answer**: the blocking transport applies
    /// it to each read of the body, so a model that keeps writing is
    /// never cut off for writing a long answer, and one that stops
    /// mid-answer is given up on this long after its last byte.
    pub stream_idle_timeout_ms: Option<u64>,
    /// Headers every request to this endpoint adds, name and value.
    pub extra_headers: Vec<(String, HeaderValue)>,
    /// Body fields every request writes: a JSON pointer, and the text
    /// of the value to write there.
    pub overrides: Vec<(String, String)>,
    /// Which of this endpoint's calls go through the machine's proxy.
    /// A value rather than an `Option`, because every call has to make
    /// this decision and "nobody settled it" is the default, not a
    /// third state a caller has to handle.
    pub proxying: Proxying,
    /// How many calls may be in flight at once; absent means the
    /// default for this kind of connection (`crates/gateway/Spec.lean` D21).
    pub max_in_flight: Option<MaxInFlight>,
}

impl EndpointTuning {
    /// What an endpoint nobody tuned is called with.
    ///
    /// The deadline is long because a reasoning model answering a hard
    /// question is not a stalled one; the retry ceiling is absent
    /// because `Halt` is this city's brake; the idle bound is absent
    /// because no figure this city could write down would be the
    /// provider's.
    pub const DEFAULTS: TuningDefaults = TuningDefaults {
        timeout_ms: 120_000,
        retries: Retries::UntilHalted,
        stream_idle_timeout_ms: None,
    };

    /// How long one settled call to this endpoint may take.
    #[must_use]
    pub fn call_timeout_ms(&self) -> u64 {
        self.timeout_ms.unwrap_or(Self::DEFAULTS.timeout_ms)
    }

    /// How long a streamed call to this endpoint may go silent, when
    /// that is not its call timeout.
    #[must_use]
    pub fn idle_timeout_ms(&self) -> Option<u64> {
        self.stream_idle_timeout_ms
            .or(Self::DEFAULTS.stream_idle_timeout_ms)
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

pub use kernel::event::record::validate_provider_accounts as validate_accounts;

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

    /// An endpoint nobody tuned is called with the defaults, and the
    /// defaults are read from the one place that states them.
    #[test]
    fn an_untouched_endpoint_takes_the_figures_this_module_states() {
        let untouched = EndpointTuning::default();
        assert_eq!(
            untouched.call_timeout_ms(),
            EndpointTuning::DEFAULTS.timeout_ms
        );
        assert_eq!(untouched.request_max_retries, Retries::UntilHalted);
        assert_eq!(untouched.idle_timeout_ms(), None);
        assert_eq!(
            EndpointTuning {
                timeout_ms: Some(9_000),
                stream_idle_timeout_ms: Some(30_000),
                ..EndpointTuning::default()
            }
            .call_timeout_ms(),
            9_000
        );
    }
    #[test]
    fn accounts_reject_empty_duplicate_and_anonymous_headers() {
        let account = kernel::event::record::ProviderAccount {
            id: kernel::ServerLabel::parse("one").unwrap(),
            reference: Some(kernel::SecretRef::new("providers", "one").unwrap()),
            header: None,
        };
        assert!(validate_accounts(Some(&[])).is_err());
        assert!(validate_accounts(Some(&[account.clone(), account.clone()])).is_err());
        let repeated_reference = kernel::event::record::ProviderAccount {
            id: kernel::ServerLabel::parse("two").unwrap(),
            ..account.clone()
        };
        assert!(validate_accounts(Some(&[account, repeated_reference])).is_err());
        assert!(
            validate_accounts(Some(&[kernel::event::record::ProviderAccount {
                id: kernel::ServerLabel::parse("anonymous").unwrap(),
                reference: None,
                header: Some("x-api-key".to_owned())
            }]))
            .is_err()
        );
    }
}
