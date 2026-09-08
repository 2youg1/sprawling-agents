// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a tag does when the endpoint behind it will not answer
//! (shape 2): a two-armed value whose default is to stop.
//!
//! Resolving a tag answers two questions, not one — which model
//! answers, and what happens when it does not. The second answer used
//! to be absent, which in practice meant "whatever the call site felt
//! like". It is now a value the person owns, carried beside the choice
//! and rebuilt from the same ledger record.
//!
//! **The default is `None`.** Switching a person's model without being
//! asked changes the quality of the answer, the price of it, and which
//! company sees the data — the three things a person weighs when they
//! pick an endpoint themselves. A default value may not decide any of
//! them.

use kernel::{AxCode, AxError, ModelTag, Payload, TimeMs};
use serde_json::{Map, Value};

use crate::admission::{AdmissionState, AdmissionVerdict};

/// What a tag falls back to. Exhaustive on purpose: a third way of
/// handling a dead endpoint has to be spelled at every reader.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Fallback {
    /// Stop. The run freezes and the reason goes on the ledger.
    #[default]
    None,
    /// Retreat to a named endpoint and model, once the provider-side
    /// back-off allows it.
    ///
    /// Marked `non_exhaustive` so that outside this crate the struct
    /// literal cannot be written: an empty name is a retreat to
    /// nowhere, and [`Fallback::then`] is where that is refused.
    #[non_exhaustive]
    Then { endpoint: String, model: String },
}

/// What a caller does with a provider failure, given the fallback.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retreat {
    /// Freeze the run. No reason is carried here: the reason is the
    /// `AxError` the caller already holds, and copying it would give
    /// one fact two authorities.
    Freeze,
    /// Move the work, not before this moment. `non_exhaustive` for the
    /// same reason as [`Fallback::Then`]: only this module decides what
    /// a legal retreat looks like.
    #[non_exhaustive]
    MoveTo {
        endpoint: String,
        model: String,
        not_before: TimeMs,
    },
}

impl Fallback {
    /// The retreating arm.
    ///
    /// # Errors
    /// An empty endpoint name or model id is refused here rather than
    /// at the wire: a retreat to a name nobody can resolve is a silent
    /// freeze wearing the word "fallback".
    pub fn then(endpoint: &str, model: &str) -> Result<Fallback, AxError> {
        for (what, value) in [("endpoint", endpoint), ("model", model)] {
            if value.trim().is_empty() {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "set a fallback",
                    format!("the fallback {what} is empty"),
                )
                .with_recovery("name an attached endpoint and one of the models it serves"));
            }
        }
        Ok(Fallback::Then {
            endpoint: endpoint.to_owned(),
            model: model.to_owned(),
        })
    }

    /// The endpoint this tag retreats to, when it retreats at all.
    #[must_use]
    pub fn endpoint(&self) -> Option<&str> {
        match self {
            Fallback::None => Option::None,
            Fallback::Then { endpoint, .. } => Some(endpoint),
        }
    }

    /// The model this tag retreats to, when it retreats at all.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        match self {
            Fallback::None => Option::None,
            Fallback::Then { model, .. } => Some(model),
        }
    }

    /// What to do about a provider failure now.
    ///
    /// The back-off is not recomputed here: `admission` already holds
    /// the AIMD ladder and the provider's own `retry-after`, and a
    /// second ladder would be a second authority over one number.
    #[must_use]
    pub fn retreat(&self, now: TimeMs, admission: &AdmissionState) -> Retreat {
        match self {
            Fallback::None => Retreat::Freeze,
            Fallback::Then { endpoint, model } => Retreat::MoveTo {
                endpoint: endpoint.clone(),
                model: model.clone(),
                not_before: match admission.admit(now) {
                    AdmissionVerdict::Admit => now,
                    AdmissionVerdict::Hold { until } => until,
                },
            },
        }
    }
}

/// The `provider_degraded` payload for one retreat, whichever arm it
/// took. Both arms are recorded: a move that leaves no trace is exactly
/// the silent model switch this design refuses.
///
/// # Errors
/// Propagates payload construction failure.
pub fn retreat_payload(
    tag: ModelTag,
    from: &str,
    retreat: &Retreat,
    because: &AxError,
) -> Result<Payload, AxError> {
    let mut map = Map::new();
    map.insert("tag".to_owned(), Value::String(tag.as_str().to_owned()));
    map.insert("from".to_owned(), Value::String(from.to_owned()));
    match retreat {
        Retreat::Freeze => {
            map.insert("action".to_owned(), Value::String("freeze".to_owned()));
        }
        Retreat::MoveTo {
            endpoint,
            model,
            not_before,
        } => {
            map.insert("action".to_owned(), Value::String("move_to".to_owned()));
            map.insert("to_endpoint".to_owned(), Value::String(endpoint.clone()));
            map.insert("to_model".to_owned(), Value::String(model.clone()));
            map.insert(
                "not_before_ms".to_owned(),
                Value::Number(not_before.value().into()),
            );
        }
    }
    map.insert(
        "code".to_owned(),
        Value::String(because.code().as_str().to_owned()),
    );
    map.insert(
        "subject".to_owned(),
        Value::String(because.subject().to_owned()),
    );
    Payload::new(map)
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
    use crate::admission::ProviderOutcome;

    fn refused() -> AxError {
        AxError::failure(AxCode::Provider, "call the model", "429 from house")
    }

    #[test]
    fn the_default_is_to_stop_rather_than_to_switch() {
        assert_eq!(Fallback::default(), Fallback::None);
        let verdict = Fallback::default().retreat(TimeMs::new(1_000), &AdmissionState::new());
        assert_eq!(
            verdict,
            Retreat::Freeze,
            "a tag nobody gave a fallback to must freeze, never pick a model itself"
        );
    }

    #[test]
    fn a_retreat_to_an_empty_name_is_refused_at_the_construction_point() {
        let err = Fallback::then("", "m-small").unwrap_err();
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
        assert!(Fallback::then("spare", "  ").is_err());
        let good = Fallback::then("spare", "m-small").unwrap();
        assert_eq!(good.endpoint(), Some("spare"));
        assert_eq!(good.model(), Some("m-small"));
    }

    #[test]
    fn the_move_waits_for_the_back_off_admission_already_computed() {
        let spare = Fallback::then("spare", "m-small").unwrap();
        let mut admission = AdmissionState::new();
        admission.on_dispatch(TimeMs::new(1_000)).unwrap();
        admission.on_outcome(
            ProviderOutcome::RateLimited {
                retry_after_ms: Some(30_000),
            },
            TimeMs::new(1_100),
        );
        assert_eq!(
            spare.retreat(TimeMs::new(1_200), &admission),
            Retreat::MoveTo {
                endpoint: "spare".to_owned(),
                model: "m-small".to_owned(),
                not_before: TimeMs::new(31_100),
            },
            "the provider's own retry-after decides when the work moves"
        );
    }

    #[test]
    fn a_move_with_nothing_holding_it_back_goes_now() {
        let spare = Fallback::then("spare", "m-small").unwrap();
        assert_eq!(
            spare.retreat(TimeMs::new(4_000), &AdmissionState::new()),
            Retreat::MoveTo {
                endpoint: "spare".to_owned(),
                model: "m-small".to_owned(),
                not_before: TimeMs::new(4_000),
            }
        );
    }

    #[test]
    fn both_arms_leave_a_record_and_the_move_says_where_it_went() {
        let freeze =
            retreat_payload(ModelTag::Main, "house", &Retreat::Freeze, &refused()).unwrap();
        let frozen = serde_json::to_value(&freeze).unwrap();
        assert_eq!(frozen["action"], "freeze");
        assert_eq!(frozen["code"], "E_PROVIDER");
        assert_eq!(frozen["from"], "house");
        assert!(frozen.get("to_endpoint").is_none());

        let spare = Fallback::then("spare", "m-small").unwrap();
        let moved = retreat_payload(
            ModelTag::Main,
            "house",
            &spare.retreat(TimeMs::new(7), &AdmissionState::new()),
            &refused(),
        )
        .unwrap();
        let moved = serde_json::to_value(&moved).unwrap();
        assert_eq!(moved["action"], "move_to");
        assert_eq!(moved["to_endpoint"], "spare");
        assert_eq!(moved["to_model"], "m-small");
        assert_eq!(moved["not_before_ms"], 7);
    }
}
