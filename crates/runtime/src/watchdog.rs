// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The disposal surface. The stall verdict is
//! `kernel::stall`'s alone; this module never re-derives it and never
//! forwards its internals — it answers one question: what happens next.
//! Disposal is graded on purpose: a corrective steer first (name the
//! repetition to the model), freezing only when correction failed.
//! Terminal-only watchdogs kill recoverable sessions; that lesson is the
//! reason this type exists.

use kernel::{AxCode, AxError, Payload, StallVerdict, TimeMs};
use serde_json::{Map, Value};

/// How many times a run may make a failed call again.
///
/// Exhaustive rather than a count with a sentinel: "keep trying until
/// somebody stops this" and "try four times" are different intentions,
/// and a number cannot spell the first one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Retries {
    /// No ceiling. What stops the run is `Halt`, the city's one brake.
    /// This is what a person who settled nothing asked for.
    #[default]
    UntilHalted,
    /// Freeze once this many retries have been spent. Zero is a real
    /// answer and means "once, then report".
    AtMost(u32),
}

/// One watchdog per run: it holds the correction history and the ceiling
/// the person set on retries, nothing else.
#[derive(Debug, Default)]
pub struct Watchdog {
    corrections: u32,
    provider_failures: u32,
    retries: Retries,
}

/// Deliberately exhaustive: a new disposal must force every run loop to
/// decide, not fall through a catch-all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposal {
    Proceed,
    CorrectiveSteer {
        text: String,
    },
    /// Try the same call again, not before this moment. The moment is
    /// the provider's own, carried in from `gateway::admission`.
    BackOff {
        until: TimeMs,
    },
    Freeze {
        reason: FreezeReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeReason {
    Stall,
    ProviderRefused,
}

impl FreezeReason {
    fn as_str(self) -> &'static str {
        match self {
            FreezeReason::Stall => "stall",
            FreezeReason::ProviderRefused => "provider_refused",
        }
    }
}

impl Watchdog {
    #[must_use]
    pub fn new(retries: Retries) -> Watchdog {
        Watchdog {
            retries,
            ..Watchdog::default()
        }
    }

    /// Consumes the stall verdict verbatim. First hit: a corrective
    /// steer naming the repetition. Second hit: freeze.
    pub fn on_stall(&mut self, verdict: &StallVerdict) -> Disposal {
        match verdict {
            StallVerdict::Ok => Disposal::Proceed,
            StallVerdict::Stall { repeats } => {
                if self.corrections == 0 {
                    self.corrections = 1;
                    Disposal::CorrectiveSteer {
                        text: format!(
                            "You have repeated the same call {repeats} times in a row. \
                             Change the approach or report why the goal cannot be met."
                        ),
                    }
                } else {
                    Disposal::Freeze {
                        reason: FreezeReason::Stall,
                    }
                }
            }
        }
    }

    /// Classifies one provider failure. The producer of the error
    /// already decided whether the same call is worth making again, so
    /// this reads `AxError::is_retriable` rather than guessing a second
    /// time from a count of attempts.
    ///
    /// A non-retriable failure freezes on the first one: repeating a
    /// request the provider has already rejected on its shape buys the
    /// same rejection again. A retriable one backs off to `not_before`
    /// — which the caller takes from `gateway::admission`, where the
    /// provider's own `retry-after` was folded in.
    ///
    /// **A retriable failure freezes only against a ceiling the person
    /// set.** Under [`Retries::UntilHalted`] what stops a run that keeps
    /// failing is `Halt`, the city's one brake; under
    /// [`Retries::AtMost`] it is the number they entered, because a
    /// number entered on a form that nothing reads is worse than no
    /// field at all.
    pub fn on_provider_failure(&mut self, failure: &AxError, not_before: TimeMs) -> Disposal {
        self.provider_failures = self.provider_failures.saturating_add(1);
        let refused = Disposal::Freeze {
            reason: FreezeReason::ProviderRefused,
        };
        if !failure.is_retriable() {
            return refused;
        }
        match self.retries {
            Retries::UntilHalted => Disposal::BackOff { until: not_before },
            // Counted from the first failure, so `AtMost(0)` spends its
            // one attempt and reports.
            Retries::AtMost(ceiling) if self.provider_failures <= ceiling => {
                Disposal::BackOff { until: not_before }
            }
            Retries::AtMost(_) => refused,
        }
    }

    /// The `watchdog_fired` payload (E_LOOP_SUSPECTED's carrier when the
    /// reason is a stall). Proceed never fires.
    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError> {
        let mut map = Map::new();
        match disposal {
            Disposal::Proceed => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "encode watchdog_fired",
                    "Proceed does not fire",
                ));
            }
            Disposal::CorrectiveSteer { text } => {
                map.insert("action".to_owned(), Value::String("steer".to_owned()));
                map.insert("text".to_owned(), Value::String(text.clone()));
            }
            Disposal::BackOff { until } => {
                map.insert("action".to_owned(), Value::String("back_off".to_owned()));
                map.insert("until_ms".to_owned(), Value::Number(until.value().into()));
            }
            Disposal::Freeze { reason } => {
                map.insert("action".to_owned(), Value::String("freeze".to_owned()));
                map.insert(
                    "reason".to_owned(),
                    Value::String(reason.as_str().to_owned()),
                );
            }
        }
        map.insert(
            "corrections".to_owned(),
            Value::Number(self.corrections.into()),
        );
        map.insert(
            "provider_failures".to_owned(),
            Value::Number(self.provider_failures.into()),
        );
        Payload::new(map)
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
    use kernel::{ActionFingerprint, observe};

    #[test]
    fn disposal_is_graded_steer_first_freeze_second() {
        let mut dog = Watchdog::new(Retries::UntilHalted);
        let same = ActionFingerprint::derive(b"exec identical");
        let sample = vec![same, same, same];
        let verdict = observe(&sample);
        let first = dog.on_stall(&verdict);
        match &first {
            Disposal::CorrectiveSteer { text } => {
                assert!(text.contains("repeated the same call 3 times"))
            }
            other => panic!("first hit must steer, got {other:?}"),
        }
        let second = dog.on_stall(&verdict);
        assert_eq!(
            second,
            Disposal::Freeze {
                reason: FreezeReason::Stall
            }
        );
        // The payload names the action; Proceed refuses to fire.
        let payload = serde_json::to_value(dog.fired_payload(&second).unwrap()).unwrap();
        assert_eq!(payload["action"], "freeze");
        assert_eq!(payload["reason"], "stall");
        assert!(dog.fired_payload(&Disposal::Proceed).is_err());
    }

    #[test]
    fn ok_verdicts_never_dispose() {
        let mut dog = Watchdog::new(Retries::UntilHalted);
        assert_eq!(dog.on_stall(&StallVerdict::Ok), Disposal::Proceed);
        assert_eq!(dog.on_stall(&StallVerdict::Ok), Disposal::Proceed);
    }

    fn provider_error(retriable: bool) -> AxError {
        let err = AxError::failure(AxCode::Provider, "call the model", "the provider said no");
        if retriable { err.retriable() } else { err }
    }

    #[test]
    fn a_failure_the_provider_will_repeat_stops_after_one() {
        let mut dog = Watchdog::new(Retries::UntilHalted);
        assert_eq!(
            dog.on_provider_failure(&provider_error(false), TimeMs::new(9_000)),
            Disposal::Freeze {
                reason: FreezeReason::ProviderRefused
            },
            "a request the provider rejected on its shape buys the same rejection again"
        );
    }

    #[test]
    fn a_retriable_failure_backs_off_and_never_freezes_by_itself() {
        let mut dog = Watchdog::new(Retries::UntilHalted);
        for round in 0..64u64 {
            let until = TimeMs::new(round.saturating_mul(250).saturating_add(1_000));
            assert_eq!(
                dog.on_provider_failure(&provider_error(true), until),
                Disposal::BackOff { until },
                "only Halt stops a run that is waiting out a provider"
            );
        }
        let payload = serde_json::to_value(
            dog.fired_payload(&Disposal::BackOff {
                until: TimeMs::new(1_000),
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["action"], "back_off");
        assert_eq!(payload["until_ms"], 1_000);
        assert_eq!(payload["provider_failures"], 64);
    }
}
