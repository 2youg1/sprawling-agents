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
use serde::{Deserialize, Serialize};

pub use kernel::Retries;

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
    /// the provider's own, carried in from `gateway::admission`, and
    /// the failure travels with it: "backed off" without what it backed
    /// off from is a line nobody can act on, and the run loop used to
    /// write that half of the fact from a second place.
    BackOff {
        until: TimeMs,
        code: AxCode,
        subject: String,
    },
    Freeze {
        reason: FreezeReason,
    },
}

impl Disposal {
    /// The verdict that asks the same call again, holding what failed.
    fn backing_off(until: TimeMs, failure: &AxError) -> Disposal {
        Disposal::BackOff {
            until,
            code: *failure.code(),
            subject: failure.subject().to_owned(),
        }
    }
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
            Retries::UntilHalted => Disposal::backing_off(not_before, failure),
            // Counted from the first failure, so `AtMost(0)` spends its
            // one attempt and reports.
            Retries::AtMost(ceiling) if self.provider_failures <= ceiling => {
                Disposal::backing_off(not_before, failure)
            }
            Retries::AtMost(_) => refused,
        }
    }

    /// The `watchdog_fired` payload (E_LOOP_SUSPECTED's carrier when the
    /// reason is a stall). Proceed never fires.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for [`Disposal::Proceed`], which records
    /// nothing, and whatever `Payload::of` says about the encoding.
    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError> {
        let action = match disposal {
            Disposal::Proceed => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "encode watchdog_fired",
                    "Proceed does not fire",
                )
                .with_recovery(
                    "report this against runtime::watchdog: `Proceed` is the verdict                      that records nothing, and the caller asked it for a payload",
                ));
            }
            Disposal::CorrectiveSteer { text } => FiredAction::Steer { text: text.clone() },
            Disposal::BackOff {
                until,
                code,
                subject,
            } => FiredAction::BackOff {
                until_ms: until.value(),
                code: code.as_str().to_owned(),
                subject: subject.clone(),
            },
            Disposal::Freeze { reason } => FiredAction::Freeze {
                reason: reason.as_str().to_owned(),
            },
        };
        Payload::of(&WatchdogFired {
            action,
            corrections: self.corrections,
            provider_failures: self.provider_failures,
        })
    }
}

/// `watchdog_fired`: what the watchdog did, and how often it has had to.
///
/// The one authority for this line's keys. The run loop used to write a
/// second, narrower shape for the same kind and the same `back_off`
/// word, so one history held two answers to "what does a watchdog line
/// look like".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchdogFired {
    #[serde(flatten)]
    pub action: FiredAction,
    /// How many corrective steers this run has been given.
    pub corrections: u32,
    /// How many provider failures this run has met.
    pub provider_failures: u32,
}

/// What the watchdog did, in the word the line carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FiredAction {
    /// The model was told it is repeating itself.
    Steer { text: String },
    /// The same call will be made again, no earlier than `until_ms`,
    /// because of the failure named here.
    BackOff {
        until_ms: u64,
        code: String,
        subject: String,
    },
    /// The run was frozen.
    Freeze { reason: String },
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::ActionFingerprint;
    use kernel::stall::observe;

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
        let draft = AxError::failure(AxCode::Provider, "call the model", "the provider said no");
        let draft = if retriable { draft.retriable() } else { draft };
        draft.with_recovery("dispatch again, or attach a second endpoint")
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
                Disposal::BackOff {
                    until,
                    code: AxCode::Provider,
                    subject: "the provider said no".to_owned(),
                },
                "only Halt stops a run that is waiting out a provider"
            );
        }
        let payload = serde_json::to_value(
            dog.fired_payload(&Disposal::BackOff {
                until: TimeMs::new(1_000),
                code: AxCode::Provider,
                subject: "the provider said no".to_owned(),
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["action"], "back_off");
        assert_eq!(payload["until_ms"], 1_000);
        assert_eq!(payload["provider_failures"], 64);
        assert_eq!(
            payload["subject"], "the provider said no",
            "a line saying it backed off says what it backed off from"
        );
    }
}
