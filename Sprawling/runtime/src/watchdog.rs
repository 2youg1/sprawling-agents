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
//!
//! A provider failure is disposed of through the account round of the
//! model call it broke (`kernel::account_recovery`, kernel D54): this
//! module carries the round's step out as a disposal and decides nothing
//! about accounts itself (runtime D89).

mod round;

use std::num::NonZeroU64;

use kernel::account_recovery::{AccountStep, RoundEnd};
use kernel::event::record::{FiredAction, WatchdogFired};
use kernel::model::AccountRoster;
use kernel::{AxCode, AxError, Payload, Retries, RunId, ServerLabel, StallVerdict, TimeMs};

use round::Round;

/// One watchdog per run: it holds the correction history, the ceiling
/// the person set on retries, and the account round of the model call
/// in flight.
#[derive(Debug, Default)]
pub struct Watchdog {
    corrections: u32,
    provider_failures: u32,
    /// Failures since the provider last answered. The backoff and the
    /// person's ceiling both read this rather than the run's total: a
    /// provider that recovered owes the next outage no minute-long
    /// first wait, and a ceiling on retries is a ceiling on asking the
    /// same call again.
    streak: u32,
    retries: Retries,
    /// Folded from the run's id once, so the run's jitter is the same
    /// on every replay and different from its neighbours'.
    jitter_seed: u64,
    /// The round of the model call in flight; `None` between calls.
    round: Option<Round>,
}

/// Deliberately exhaustive: a new disposal must force every run loop to
/// decide, not fall through a catch-all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposal {
    Proceed,
    CorrectiveSteer {
        text: String,
    },
    /// Try the same call again, not before this moment. The moment
    /// comes from the backoff schedule [`Watchdog`] owns, and the
    /// failure travels with it: "backed off" without what it backed
    /// off from is a line nobody can act on, and a second writer of
    /// that half of the fact could drift from this one.
    BackOff {
        until: TimeMs,
        code: AxCode,
        subject: String,
    },
    /// Make the same call again at once on another account of the
    /// provider, because of this failure. No wait: the back-off was a
    /// statement about the account that failed.
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
    /// Every redeemable account of the provider was tried in one round
    /// and none answered; the run records
    /// [`Watchdog::exhausted`] rather than the last account's failure.
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

    /// Opens the account round of the next model call, before its first
    /// send, from the roster the adapter answers. The round's opening
    /// account is the roster's current one while it is redeemable, which
    /// on a run's first call is the Session's binding.
    ///
    /// Answers the account the call must go out on, which the caller
    /// selects on the model before sending; `None` for a single-account
    /// round, which names none.
    ///
    /// # Errors
    /// `E_PROVIDER_ACCOUNTS_EXHAUSTED` when no listed account can be
    /// redeemed: nothing is sent, and no round is open.
    pub fn open_round(
        &mut self,
        roster: Option<AccountRoster>,
    ) -> Result<Option<ServerLabel>, AxError> {
        let round = Round::open(roster, self.retries)?;
        let current = round.accounts.current().cloned();
        self.round = Some(round);
        Ok(current)
    }

    /// Whether a model call's round is open: between an answer and the
    /// next call there is none, and the caller opens one.
    #[must_use]
    pub fn round_open(&self) -> bool {
        self.round.is_some()
    }

    /// Disposes of one provider failure by the step the call's account
    /// round gives (kernel D54). The producer of the error already
    /// decided whether the same call is worth making again and whether
    /// the account can still take it, so this reads `AxError::retry` and
    /// `AxError::account` through the round rather than guessing.
    ///
    /// With one account, `Retry::No` freezes on the first one: repeating
    /// a request the provider rejected on its shape buys the same
    /// rejection again. `Yes` and `Unknown` back off (a model call's only
    /// effect is an answer the city never received) from `now` by the
    /// schedule [`backoff_ms`] holds, or by the provider's own
    /// `retry_after_ms` when that is longer: asking before the time it
    /// named buys one more refusal. With several accounts a resend backs
    /// off the same way, a switch goes at once and starts the schedule
    /// over, and a round with no account left freezes as
    /// [`FreezeReason::AccountsExhausted`].
    ///
    /// **A retriable failure freezes only against a ceiling the person
    /// set.** Under [`Retries::UntilHalted`] what stops a run that keeps
    /// failing is `Halt`, the city's one brake; under
    /// [`Retries::AtMost`] it is the number they entered, because a
    /// number entered on a form that nothing reads is worse than no
    /// field at all. A failure outside an open round meets a
    /// single-account round opened for it.
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
            // A single-account round always opens; this arm answers a
            // roster that names no account it could ever send on.
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

    /// The provider answered, which ends the streak of failures and the
    /// call's account round.
    pub fn on_provider_answered(&mut self) {
        self.streak = 0;
        self.round = None;
    }

    /// Whether the turn's repair segment may send the failed call again
    /// through the blocking door: with several accounts that is one more
    /// send on the current account, inside its budget and the person's
    /// ceiling. Outside an open round it may, as it always could.
    #[must_use]
    pub fn admits_repair(&self) -> bool {
        self.round
            .as_ref()
            .is_none_or(|round| round.accounts.admits_repair())
    }

    /// The repair segment sent the call again: counted on the current
    /// account when the round has several.
    pub fn repaired(&mut self) {
        self.round = self.round.take().map(Round::repaired);
    }

    /// The failure a run records when every account of its provider
    /// failed: the provider, and each account with the last failure it
    /// met in the round, or that its reference could not be redeemed.
    /// Neither a key nor a reference is in it.
    #[must_use]
    pub fn exhausted(&self) -> AxError {
        self.round
            .as_ref()
            .map_or_else(round::nothing_listed, Round::exhausted)
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

/// How long `run` waits before asking again after `failure`, the
/// `failures_in_a_row`-th since its provider last answered: the longer of
/// the schedule below and the provider's own `retry-after`.
///
/// The one back-off table. [`Watchdog`] reads it for a model call, and
/// the `web_search` tool reads it for a search it sends again
/// (`crates/accounting/spec/Connectors.lean` §8-35), so the two kinds of
/// outside request wait alike.
#[must_use]
pub fn backoff_ms(run: RunId, failures_in_a_row: u32, failure: &AxError) -> u64 {
    wait_ms(seed_of(run), failures_in_a_row, failure)
}

/// The schedule behind [`backoff_ms`], over a seed already folded: a
/// base of 500 ms doubled for each failure in a row before this one,
/// capped at a minute, plus up to half the base again. Both numbers are
/// the provider's scale, the time an overloaded service takes to
/// recover, so no reading of this machine moves them. The jitter only
/// adds, so the base stays the floor; it is drawn from the run's seed
/// and the streak, so runs cut by one outage spread out and each
/// replays its own.
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

/// FNV-1a over the id's sixteen bytes. A uuid v7 keeps its random bits
/// at the end, which is where runs started in one millisecond differ,
/// and FNV-1a lets the last byte move the whole word.
fn seed_of(run: RunId) -> u64 {
    run.as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

/// splitmix64's finishing mix, so neighbouring streaks and seeds land
/// far apart before the remainder takes the low bits.
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
mod tests {
    use super::*;
    use kernel::ActionFingerprint;
    use kernel::stall::observe;

    /// The schedule's floor for each failure in a row, before jitter.
    const BASES: [u64; 9] = [
        500, 1_000, 2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000,
    ];

    /// A uuid v7 run id; runs that differ only in `tail` started in the
    /// same millisecond.
    fn run(tail: u8) -> RunId {
        let mut bytes = [
            0x01, 0x93, 0x2a, 0x7c, 0x10, 0x00, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
        ];
        bytes[15] = tail;
        RunId::from_bytes(bytes)
    }

    /// The waits one watchdog hands out for `count` retriable failures
    /// in a row, measured from zero.
    fn waits(dog: &mut Watchdog, count: usize) -> Vec<u64> {
        (0..count)
            .map(
                |_| match dog.on_provider_failure(&provider_error(true), TimeMs::new(0)) {
                    Disposal::BackOff { until, .. } => until.value(),
                    other => panic!("a retriable failure under its ceiling backs off: {other:?}"),
                },
            )
            .collect()
    }

    fn within_schedule(waits: &[u64]) -> bool {
        waits
            .iter()
            .zip(BASES)
            .all(|(wait, base)| (base..=base + base / 2).contains(wait))
    }

    /// Runs cut by the same outage in the same millisecond spread their
    /// next calls instead of arriving together, and one run replays its
    /// own waits byte for byte.
    #[test]
    fn each_run_waits_its_own_jittered_schedule() {
        let schedule = |tail: u8| waits(&mut Watchdog::new(Retries::UntilHalted, run(tail)), 9);
        let (first, second) = (schedule(1), schedule(2));
        assert!(
            within_schedule(&first) && within_schedule(&second),
            "{first:?} {second:?}"
        );
        assert_eq!(first, schedule(1), "the same run waits the same schedule");
        assert_ne!(first, second, "two runs do not ask again in step");
    }

    #[test]
    fn disposal_is_graded_steer_first_freeze_second() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
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
        // The refusal is one sentence: the wrapped source line leaves no
        // run of spaces in the recovery a person reads.
        assert_eq!(
            dog.fired_payload(&Disposal::Proceed)
                .unwrap_err()
                .recovery(),
            "report this against runtime::watchdog: `Proceed` is the verdict that records nothing, and the caller asked it for a payload"
        );
    }

    #[test]
    fn ok_verdicts_never_dispose() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
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
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        assert_eq!(
            dog.on_provider_failure(&provider_error(false), TimeMs::new(9_000)),
            Disposal::Freeze {
                reason: FreezeReason::ProviderRefused
            },
            "a request the provider rejected on its shape buys the same rejection again"
        );
    }

    #[test]
    fn an_answer_ends_the_streak_the_backoff_counts() {
        let mut dog = Watchdog::new(Retries::AtMost(2), run(1));
        let before = waits(&mut dog, 2);
        assert!(within_schedule(&before), "{before:?}");
        dog.on_provider_answered();
        assert_eq!(
            waits(&mut dog, 2),
            before,
            "an outage after a recovery starts from the first wait, under a fresh ceiling"
        );
    }

    #[test]
    fn a_retriable_failure_backs_off_and_never_freezes_by_itself() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        let waits: Vec<u64> = (0..64u64)
            .map(
                |_| match dog.on_provider_failure(&provider_error(true), TimeMs::new(1_000)) {
                    Disposal::BackOff { until, .. } => until.value().saturating_sub(1_000),
                    other => {
                        panic!("only Halt stops a run that is waiting out a provider: {other:?}")
                    }
                },
            )
            .collect();
        assert!(
            within_schedule(&waits),
            "each failure in a row doubles the wait, up to a minute: {waits:?}"
        );
        assert!(
            waits[9..]
                .iter()
                .all(|wait| (60_000..=90_000).contains(wait))
        );
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

    fn waits_of_fresh_run() -> Vec<u64> {
        waits(&mut Watchdog::new(Retries::UntilHalted, run(1)), 2)
    }

    use kernel::ProviderFailureKind;
    use kernel::account_recovery::AccountRetries;
    use kernel::model::{AccountRoster, RosterEntry};

    fn label(name: &str) -> ServerLabel {
        ServerLabel::parse(name).unwrap()
    }

    /// A roster of `accounts` (name, redeemable) whose next request goes
    /// out on `current`.
    fn roster(accounts: &[(&str, bool)], current: &str, retries: AccountRetries) -> AccountRoster {
        AccountRoster {
            provider: "house".to_owned(),
            accounts: accounts
                .iter()
                .map(|(name, usable)| RosterEntry {
                    account: label(name),
                    usable: *usable,
                })
                .collect(),
            current: label(current),
            retries,
        }
    }

    /// A rejected key: no resend helps it, another account may.
    fn rejected_key() -> AxError {
        AxError::provider(
            ProviderFailureKind::Refused { status: 401 },
            "call the model",
            "401 Unauthorized",
        )
        .account_unusable()
        .with_recovery("file another key")
    }

    /// A busy provider: the same account answers later.
    fn busy() -> AxError {
        AxError::provider(
            ProviderFailureKind::Refused { status: 429 },
            "call the model",
            "429 Too Many Requests",
        )
        .retriable()
        .with_recovery("wait, then ask again")
    }

    /// An answer lost after the request went out.
    fn lost() -> AxError {
        AxError::provider(ProviderFailureKind::Cut, "call the model", "stream cut")
            .effect_unknown()
            .with_recovery("ask again")
    }

    fn switched_to(name: &str, failure: &AxError) -> Disposal {
        Disposal::Switch {
            to: label(name),
            code: *failure.code(),
            subject: failure.subject().to_owned(),
        }
    }

    /// The round opens on the account the roster names while it can be
    /// redeemed, and on the first redeemable one in priority order when
    /// it cannot.
    #[test]
    fn a_round_opens_on_the_current_account_or_the_first_redeemable_one() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        let opened = [
            roster(&[("a", true), ("b", true)], "b", AccountRetries::Two),
            roster(&[("a", false), ("b", true)], "a", AccountRetries::Two),
        ]
        .map(|listed| {
            let account = dog.open_round(Some(listed)).unwrap();
            dog.on_provider_answered();
            account
        });
        assert_eq!(opened, [Some(label("b")), Some(label("b"))]);
        assert_eq!(
            dog.open_round(Some(roster(&[("solo", true)], "solo", AccountRetries::Two)))
                .unwrap(),
            None,
            "one account names nothing to select"
        );
    }

    /// A rejected key moves to the next account at once and starts the
    /// back-off schedule over; a busy account is asked again up to its
    /// budget, then the next one takes the request; and the line a switch
    /// writes names where it went and why.
    #[test]
    fn several_accounts_switch_on_a_rejected_key_and_after_the_busy_budget() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        dog.open_round(Some(roster(
            &[("a", true), ("b", true), ("c", true)],
            "a",
            AccountRetries::One,
        )))
        .unwrap();
        let first = dog.on_provider_failure(&rejected_key(), TimeMs::new(0));
        assert_eq!(first, switched_to("b", &rejected_key()));
        let payload = serde_json::to_value(dog.fired_payload(&first).unwrap()).unwrap();
        assert_eq!(
            (&payload["action"], &payload["to"], &payload["code"]),
            (
                &serde_json::json!("switch"),
                &serde_json::json!("b"),
                &serde_json::json!("E_PROVIDER")
            )
        );
        let waits = waits(&mut dog, 1);
        assert!(
            within_schedule(&waits),
            "the new account starts from the first wait: {waits:?}"
        );
        assert_eq!(
            dog.on_provider_failure(&busy(), TimeMs::new(0)),
            switched_to("c", &busy()),
            "one resend spent, the next account takes the request"
        );
    }

    /// Every account failing stops the round with its own reason, and the
    /// failure it records names each account and what it met, never a
    /// key or a reference; a roster with nothing redeemable sends nothing.
    #[test]
    fn a_round_with_no_account_left_freezes_as_exhausted() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        dog.open_round(Some(roster(
            &[("a", true), ("b", true), ("c", false)],
            "a",
            AccountRetries::Two,
        )))
        .unwrap();
        assert_eq!(
            dog.on_provider_failure(&rejected_key(), TimeMs::new(0)),
            switched_to("b", &rejected_key())
        );
        assert_eq!(
            dog.on_provider_failure(&rejected_key(), TimeMs::new(0)),
            Disposal::Freeze {
                reason: FreezeReason::AccountsExhausted
            }
        );
        let exhausted = dog.exhausted();
        assert_eq!(
            (*exhausted.code(), exhausted.subject()),
            (
                AxCode::ProviderAccountsExhausted,
                "house: a E_PROVIDER Refused { status: 401 }; \
                 b E_PROVIDER Refused { status: 401 }; c not redeemable"
            )
        );
        let none = Watchdog::new(Retries::UntilHalted, run(1)).open_round(Some(roster(
            &[("a", false), ("b", false)],
            "a",
            AccountRetries::Two,
        )));
        assert_eq!(
            none.map_err(|err| (*err.code(), err.subject().to_owned())),
            Err((
                AxCode::ProviderAccountsExhausted,
                "house: a not redeemable; b not redeemable".to_owned()
            ))
        );
    }

    /// A lost answer is asked again on its own account within the budget
    /// and then stops with the failure as it was; it never moves to
    /// another account, where it would be paid for twice.
    #[test]
    fn an_unknown_effect_never_switches_the_account() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        dog.open_round(Some(roster(
            &[("a", true), ("b", true)],
            "a",
            AccountRetries::One,
        )))
        .unwrap();
        assert!(matches!(
            dog.on_provider_failure(&lost(), TimeMs::new(0)),
            Disposal::BackOff { .. }
        ));
        assert_eq!(
            dog.on_provider_failure(&lost(), TimeMs::new(0)),
            Disposal::Freeze {
                reason: FreezeReason::ProviderRefused
            }
        );
    }

    /// The repair segment's resend is one more send on the account: it is
    /// admitted while the account has budget, and counted, so a busy
    /// failure after it moves on; with one account it is always admitted
    /// and never counted, as before accounts existed.
    #[test]
    fn a_repair_is_admitted_within_the_account_budget_and_counted() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        assert!(dog.admits_repair(), "no round: the repair goes out");
        dog.open_round(Some(roster(
            &[("a", true), ("b", true)],
            "a",
            AccountRetries::One,
        )))
        .unwrap();
        assert!(dog.admits_repair());
        dog.repaired();
        assert!(!dog.admits_repair(), "the account's one resend is spent");
        assert_eq!(
            dog.on_provider_failure(&busy(), TimeMs::new(0)),
            switched_to("b", &busy())
        );
        let mut single = Watchdog::new(Retries::AtMost(0), run(1));
        single.open_round(None).unwrap();
        single.repaired();
        assert!(single.admits_repair());
    }

    #[test]
    fn a_provider_that_names_its_wait_is_not_asked_sooner() {
        let mut dog = Watchdog::new(Retries::UntilHalted, run(1));
        let told = |wait_ms: u64| {
            AxError::failure(AxCode::Provider, "call the model", "answered 429")
                .retriable_after(wait_ms)
                .with_recovery("wait, then ask again")
        };
        let waits: Vec<u64> = [30_000, 10]
            .map(
                |wait_ms| match dog.on_provider_failure(&told(wait_ms), TimeMs::new(1_000)) {
                    Disposal::BackOff { until, .. } => until.value().saturating_sub(1_000),
                    other => {
                        panic!("a provider that says when to come back is asked again: {other:?}")
                    }
                },
            )
            .to_vec();
        let own = waits_of_fresh_run();
        assert_eq!(
            waits,
            [30_000, own[1]],
            "the longer of the provider's word and the schedule's own wait"
        );
    }
}
