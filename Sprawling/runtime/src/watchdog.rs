// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod round;

use std::num::NonZeroU64;

use kernel::account_recovery::{AccountStep, RoundEnd};
use kernel::event::record::{FiredAction, WatchdogFired};
use kernel::model::AccountRoster;
use kernel::{AxCode, AxError, Payload, Retries, RunId, ServerLabel, StallVerdict, TimeMs};

use round::Round;

#[derive(Debug, Default)]
pub struct Watchdog {
    corrections: u32,
    provider_failures: u32,

    streak: u32,
    retries: Retries,

    jitter_seed: u64,

    round: Option<Round>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposal {
    Proceed,
    CorrectiveSteer {
        text: String,
    },

    BackOff {
        until: TimeMs,
        code: AxCode,
        subject: String,
    },

    Switch {
        to: ServerLabel,
        code: AxCode,
        subject: String,
    },
    Freeze {
        reason: FreezeReason,
    },
}

impl Disposal {
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

    AccountsExhausted,
}

impl FreezeReason {
    fn as_str(self) -> &'static str {
        match self {
            FreezeReason::Stall => "stall",
            FreezeReason::ProviderRefused => "provider_refused",
            FreezeReason::AccountsExhausted => "accounts_exhausted",
        }
    }
}

impl Watchdog {
    #[must_use]
    pub fn new(retries: Retries, run: RunId) -> Watchdog {
        Watchdog {
            retries,
            jitter_seed: seed_of(run),
            ..Watchdog::default()
        }
    }

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

    pub fn open_round(
        &mut self,
        roster: Option<AccountRoster>,
    ) -> Result<Option<ServerLabel>, AxError> {
        let round = Round::open(roster, self.retries)?;
        let current = round.accounts.current().cloned();
        self.round = Some(round);
        Ok(current)
    }

    #[must_use]
    pub fn round_open(&self) -> bool {
        self.round.is_some()
    }

    pub fn on_provider_failure(&mut self, failure: &AxError, now: TimeMs) -> Disposal {
        self.provider_failures = self.provider_failures.saturating_add(1);
        self.streak = self.streak.saturating_add(1);
        let wait = wait_ms(self.jitter_seed, self.streak, failure);
        let not_before = TimeMs::new(now.value().saturating_add(wait));
        let mut round = match self
            .round
            .take()
            .map_or_else(|| Round::open(None, self.retries), Ok)
        {
            Ok(round) => round,

            Err(_exhausted) => {
                return Disposal::Freeze {
                    reason: FreezeReason::AccountsExhausted,
                };
            }
        };
        round.note(failure);
        let step = round.accounts.next(failure);
        let disposal = match &step {
            AccountStep::Resend => Disposal::backing_off(not_before, failure),
            AccountStep::Switch { to } => {
                self.streak = 0;
                Disposal::Switch {
                    to: to.clone(),
                    code: *failure.code(),
                    subject: failure.subject().to_owned(),
                }
            }
            AccountStep::Stop {
                why: RoundEnd::Refused | RoundEnd::Unknown | RoundEnd::Cap,
            } => Disposal::Freeze {
                reason: FreezeReason::ProviderRefused,
            },
            AccountStep::Stop {
                why: RoundEnd::Exhausted,
            } => Disposal::Freeze {
                reason: FreezeReason::AccountsExhausted,
            },
        };
        round.accounts = round.accounts.apply(&step);
        self.round = Some(round);
        disposal
    }

    pub fn on_provider_answered(&mut self) {
        self.streak = 0;
        self.round = None;
    }

    #[must_use]
    pub fn admits_repair(&self) -> bool {
        self.round
            .as_ref()
            .is_none_or(|round| round.accounts.admits_repair())
    }

    pub fn repaired(&mut self) {
        self.round = self.round.take().map(Round::repaired);
    }

    #[must_use]
    pub fn exhausted(&self) -> AxError {
        self.round
            .as_ref()
            .map_or_else(round::nothing_listed, Round::exhausted)
    }

    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError> {
        let action = match disposal {
            Disposal::Proceed => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "encode watchdog_fired",
                    "Proceed does not fire",
                )
                .with_recovery(
                    "report this against runtime::watchdog: `Proceed` is the verdict \
                     that records nothing, and the caller asked it for a payload",
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
            Disposal::Switch { to, code, subject } => FiredAction::Switch {
                to: to.clone(),
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

#[must_use]
pub fn backoff_ms(run: RunId, failures_in_a_row: u32, failure: &AxError) -> u64 {
    wait_ms(seed_of(run), failures_in_a_row, failure)
}

fn wait_ms(seed: u64, streak: u32, failure: &AxError) -> u64 {
    const FIRST_MS: u64 = 500;
    const CEILING_MS: u64 = 60_000;
    let doublings = streak.saturating_sub(1).min(16);
    let base = FIRST_MS
        .checked_shl(doublings)
        .map_or(CEILING_MS, |wait| wait.min(CEILING_MS));
    let spread = NonZeroU64::MIN.saturating_add(base >> 1);
    let own = base.saturating_add(mixed(seed ^ u64::from(streak)) % spread);
    failure.retry_after_ms().map_or(own, |told| own.max(told))
}

fn seed_of(run: RunId) -> u64 {
    run.as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn mixed(word: u64) -> u64 {
    let word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    word ^ (word >> 31)
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
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
