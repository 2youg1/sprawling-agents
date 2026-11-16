// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Proxying;

use crate::concurrency::MaxInFlight;
use serde_json::Value;

use crate::endpoint::HeaderValue;

use kernel::Retries;
use kernel::account_recovery::AccountRetries;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuningDefaults {

    pub timeout_ms: u64,

    pub retries: Retries,

    pub stream_idle_timeout_ms: Option<u64>,

    pub account_retries: AccountRetries,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EndpointTuning {

    #[serde(default)]
    pub accounts: Option<Vec<kernel::event::record::ProviderAccount>>,

    pub label: Option<String>,

    pub timeout_ms: Option<u64>,

    pub request_max_retries: Retries,

    pub stream_idle_timeout_ms: Option<u64>,

    pub extra_headers: Vec<(String, HeaderValue)>,

    pub overrides: Vec<(String, String)>,

    pub proxying: Proxying,

    pub max_in_flight: Option<MaxInFlight>,

    #[serde(default)]
    pub account_retries: Option<AccountRetries>,

    #[serde(default)]
    pub generation: Vec<kernel::ModelGeneration>,
}

impl EndpointTuning {

    pub const DEFAULTS: TuningDefaults = TuningDefaults {
        timeout_ms: 120_000,
        retries: Retries::UntilHalted,
        stream_idle_timeout_ms: None,
        account_retries: AccountRetries::Two,
    };

    #[must_use]
    pub fn account_retries(&self) -> AccountRetries {
        self.account_retries
            .unwrap_or(Self::DEFAULTS.account_retries)
    }

    #[must_use]
    pub fn call_timeout_ms(&self) -> u64 {
        self.timeout_ms.unwrap_or(Self::DEFAULTS.timeout_ms)
    }

    #[must_use]
    pub fn idle_timeout_ms(&self) -> Option<u64> {
        self.stream_idle_timeout_ms
            .or(Self::DEFAULTS.stream_idle_timeout_ms)
    }

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
