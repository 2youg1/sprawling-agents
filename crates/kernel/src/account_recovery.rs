// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which account of a provider a request goes out on, and after a failure
//! whether it goes out again on the same account, moves to another, or
//! stops.
//! The part `crates/kernel/spec/AccountRecovery.lean` specifies this
//! module (`crates/kernel/Spec.lean` §8-86, kernel D54 and D55).
//!
//! **One round is one logical request**, from its first send to an
//! answer, a stop or a `Halt`. Both drivers - the model path in the
//! runtime and the `web_search` tool in accounting - only carry out the
//! [`AccountStep`] a round gives them, so neither grows a retry loop or
//! an account choice of its own. [`AccountRound::next`] answers and
//! changes nothing; [`AccountRound::apply`] changes and answers nothing.
//!
//! A round is neither recorded nor persisted: a restarted process opens
//! the next round from the binding the ledger rebuilds.

use std::collections::BTreeSet;

use crate::error::{AccountDisposition, AxError, Retry};
use crate::retries::Retries;
use crate::tool::ServerLabel;

/// How many more times one account is asked the same request before the
/// round moves on. A person picks one or two on the provider's advanced
/// form; it is read only where the roster has two accounts or more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRetries {
    One,
    Two,
}

impl AccountRetries {
    const fn count(self) -> u32 {
        match self {
            AccountRetries::One => 1,
            AccountRetries::Two => 2,
        }
    }
}

/// The accounts a round can send on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Roster {
    /// A registration with one credential, or with one listed account:
    /// there is no second account, so a failure is handled as it was
    /// before accounts existed - `retry` alone decides, up to the
    /// person's ceiling.
    Single,
    /// Two accounts or more. The order is the priority.
    Several {
        accounts: Vec<ServerLabel>,
        retries: AccountRetries,
    },
}

/// Why a round stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundEnd {
    /// The request itself is wrong, or the store every account shares
    /// failed: sent again anywhere it would fail the same way.
    Refused,
    /// The answer was lost on the last account's last allowed send, and
    /// whether its effect landed is not known.
    Unknown,
    /// The person's ceiling on retries was reached.
    Cap,
    /// No account is left to switch to.
    Exhausted,
}

/// What the driver does after one failed send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStep {
    /// Send the same request again on the same account, after the
    /// driver's back-off.
    Resend,
    /// Send the same request on this account at once.
    Switch { to: ServerLabel },
    /// Send nothing more; hand the failure on.
    Stop { why: RoundEnd },
}

/// One round's state. Every field is private: a round moves only through
/// [`AccountRound::apply`] and [`AccountRound::repaired`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRound {
    roster: Roster,
    /// The accounts whose reference could be redeemed when the round
    /// opened (D55); asked once, never re-read mid-round.
    usable: BTreeSet<ServerLabel>,
    cap: Retries,
    current: Option<ServerLabel>,
    /// The accounts this round has sent on.
    tried: BTreeSet<ServerLabel>,
    /// Sends on the current account.
    here: u32,
    /// Sends in this round.
    total: u32,
}

impl AccountRound {
    /// Opens a round before its first send.
    ///
    /// # Errors
    /// [`RoundEnd::Exhausted`] when no listed account is redeemable.
    pub fn start(
        roster: Roster,
        usable: BTreeSet<ServerLabel>,
        cap: Retries,
        held: Option<ServerLabel>,
    ) -> Result<AccountRound, RoundEnd> {
        Ok(AccountRound {
            roster,
            usable,
            cap,
            current: held,
            tried: BTreeSet::new(),
            here: 0,
            total: 0,
        })
    }

    #[must_use]
    pub fn current(&self) -> Option<&ServerLabel> {
        self.current.as_ref()
    }

    #[must_use]
    pub fn next(&self, _failure: &AxError) -> AccountStep {
        AccountStep::Resend
    }

    #[must_use]
    pub fn apply(self, _step: &AccountStep) -> AccountRound {
        self
    }

    #[must_use]
    pub fn admits_repair(&self) -> bool {
        true
    }

    #[must_use]
    pub fn repaired(self) -> AccountRound {
        self
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;
    use crate::error::AxCode;

    /// What a driver meets in a round: the four failures the error
    /// builders can spell (`Advance` only ever comes with `Retry::No`,
    /// kernel D53), and the repair stage wanting to send again.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Busy,
        Unusable,
        Refused,
        Lost,
        Repair,
    }

    fn failure(event: Event) -> Option<AxError> {
        let draft = AxError::failure(AxCode::Provider, "call model", "account");
        match event {
            Event::Busy => Some(draft.retriable().with_recovery("r")),
            Event::Unusable => Some(draft.account_unusable().with_recovery("r")),
            Event::Refused => Some(draft.with_recovery("r")),
            Event::Lost => Some(draft.effect_unknown().with_recovery("r")),
            Event::Repair => None,
        }
    }

    fn label(name: &str) -> ServerLabel {
        ServerLabel::parse(name).unwrap()
    }

    fn several(names: &[&str], retries: AccountRetries) -> Roster {
        Roster::Several {
            accounts: names.iter().map(|name| label(name)).collect(),
            retries,
        }
    }

    /// Lean's `AccountRound.decisions`: a failure asks `next` and stops
    /// on `Stop`; a repair asks `admits_repair`, and is `None` when it is
    /// not let through.
    fn decisions(mut round: AccountRound, events: &[Event]) -> Vec<(Event, Option<AccountStep>)> {
        let mut out = Vec::new();
        for &event in events {
            match failure(event) {
                None if round.admits_repair() => {
                    out.push((event, Some(AccountStep::Resend)));
                    round = round.repaired();
                }
                None => out.push((event, None)),
                Some(err) => {
                    let step = round.next(&err);
                    out.push((event, Some(step.clone())));
                    if let AccountStep::Stop { .. } = step {
                        break;
                    }
                    round = round.apply(&step);
                }
            }
        }
        out
    }

    type Decided = Option<(Option<ServerLabel>, Vec<(Event, Option<AccountStep>)>)>;

    /// Lean's `vector`: open the round, then walk the events.
    fn vector(
        roster: Roster,
        missing: &[&str],
        cap: Retries,
        held: Option<&str>,
        events: &[Event],
    ) -> Decided {
        let usable = match &roster {
            Roster::Single => BTreeSet::new(),
            Roster::Several { accounts, .. } => accounts
                .iter()
                .filter(|account| !missing.contains(&account.as_str()))
                .cloned()
                .collect(),
        };
        let round = AccountRound::start(roster, usable, cap, held.map(label)).ok()?;
        Some((round.current().cloned(), decisions(round, events)))
    }

    fn resend() -> Option<AccountStep> {
        Some(AccountStep::Resend)
    }

    fn switch(to: &str) -> Option<AccountStep> {
        Some(AccountStep::Switch { to: label(to) })
    }

    fn stop(why: RoundEnd) -> Option<AccountStep> {
        Some(AccountStep::Stop { why })
    }

    /// The trace vectors `crates/kernel/spec/AccountRecovery.lean` prints
    /// with `#eval`, in its order, each with the answer Lean gave, and
    /// the `decide` example after them.
    #[test]
    fn the_lean_trace_vectors_replay_one_by_one() {
        use Event::{Busy, Lost, Refused, Repair, Unusable};
        let two = || several(&["a", "b"], AccountRetries::Two);
        let a = Some(label("a"));
        let b = Some(label("b"));
        let found = [
            vector(
                two(),
                &[],
                Retries::UntilHalted,
                None,
                &[Busy, Busy, Busy, Busy],
            ),
            vector(
                two(),
                &[],
                Retries::UntilHalted,
                None,
                &[Unusable, Unusable],
            ),
            vector(two(), &[], Retries::UntilHalted, None, &[Refused]),
            vector(two(), &[], Retries::UntilHalted, None, &[Lost, Lost, Lost]),
            vector(two(), &[], Retries::AtMost(1), None, &[Busy, Busy]),
            vector(
                several(&["a", "b", "c"], AccountRetries::One),
                &["b"],
                Retries::UntilHalted,
                Some("c"),
                &[Busy, Busy, Unusable],
            ),
            vector(
                several(&["a", "b"], AccountRetries::One),
                &[],
                Retries::UntilHalted,
                Some("b"),
                &[Repair, Busy, Repair],
            ),
            vector(two(), &["a", "b"], Retries::UntilHalted, None, &[]),
            vector(
                Roster::Single,
                &[],
                Retries::AtMost(2),
                None,
                &[Busy, Lost, Busy],
            ),
            vector(
                Roster::Single,
                &[],
                Retries::UntilHalted,
                None,
                &[Repair, Unusable],
            ),
            vector(two(), &[], Retries::UntilHalted, Some("b"), &[Busy]),
        ];
        let lean: [Decided; 11] = [
            Some((
                a.clone(),
                vec![
                    (Busy, resend()),
                    (Busy, resend()),
                    (Busy, switch("b")),
                    (Busy, resend()),
                ],
            )),
            Some((
                a.clone(),
                vec![
                    (Unusable, switch("b")),
                    (Unusable, stop(RoundEnd::Exhausted)),
                ],
            )),
            Some((a.clone(), vec![(Refused, stop(RoundEnd::Refused))])),
            Some((
                a.clone(),
                vec![
                    (Lost, resend()),
                    (Lost, resend()),
                    (Lost, stop(RoundEnd::Unknown)),
                ],
            )),
            Some((a, vec![(Busy, resend()), (Busy, stop(RoundEnd::Cap))])),
            Some((
                Some(label("c")),
                vec![
                    (Busy, resend()),
                    (Busy, switch("a")),
                    (Unusable, stop(RoundEnd::Exhausted)),
                ],
            )),
            Some((
                b.clone(),
                vec![(Repair, resend()), (Busy, switch("a")), (Repair, resend())],
            )),
            None,
            Some((
                None,
                vec![
                    (Busy, resend()),
                    (Lost, resend()),
                    (Busy, stop(RoundEnd::Cap)),
                ],
            )),
            Some((
                None,
                vec![(Repair, resend()), (Unusable, stop(RoundEnd::Refused))],
            )),
            Some((b, vec![(Busy, resend())])),
        ];
        assert_eq!(found, lean);
    }

    fn switched(step: &AccountStep) -> bool {
        matches!(step, AccountStep::Switch { .. })
    }

    fn event() -> impl Strategy<Value = Event> {
        prop_oneof![
            Just(Event::Busy),
            Just(Event::Unusable),
            Just(Event::Refused),
            Just(Event::Lost),
            Just(Event::Repair),
        ]
    }

    fn cap() -> impl Strategy<Value = Retries> {
        prop_oneof![
            Just(Retries::UntilHalted),
            (0u32..8).prop_map(Retries::AtMost)
        ]
    }

    /// Five names a roster draws from, and one it never lists, so a held
    /// account can be one the person removed.
    const NAMES: [&str; 6] = ["a", "b", "c", "d", "e", "z"];

    proptest! {
        /// The quantified theorems of `crates/kernel/spec/AccountRecovery.lean`
        /// on any roster of two to five accounts, any redeemable subset,
        /// any ceiling, any held account (listed or not) and any trace:
        /// `a_request_error_is_never_sent_again`,
        /// `an_unusable_account_is_never_asked_again`,
        /// `an_unknown_effect_never_switches`,
        /// `sends_on_one_account_stay_within_the_budget`,
        /// `a_round_visits_each_account_once`, `the_cap_bounds_every_round`,
        /// `a_held_account_opens_the_round` and
        /// `a_roster_with_nothing_redeemable_sends_nothing`. The sends are
        /// counted here from the steps the driver carries out, not read
        /// from the round.
        #[test]
        fn every_round_keeps_the_lean_properties(
            listed in 2usize..=5,
            missing in proptest::collection::vec(any::<bool>(), 5),
            two in any::<bool>(),
            cap in cap(),
            held in proptest::option::of(0usize..6),
            events in proptest::collection::vec(event(), 0..40),
        ) {
            let accounts: Vec<ServerLabel> = NAMES[..listed].iter().map(|name| label(name)).collect();
            let usable: BTreeSet<ServerLabel> = accounts
                .iter()
                .zip(&missing)
                .filter(|(_, gone)| !**gone)
                .map(|(account, _)| account.clone())
                .collect();
            let retries = if two { AccountRetries::Two } else { AccountRetries::One };
            let budget = if two { 3 } else { 2 };
            let held = held.map(|at| label(NAMES[at]));
            let opened = AccountRound::start(
                Roster::Several { accounts: accounts.clone(), retries },
                usable.clone(),
                cap,
                held.clone(),
            );
            if usable.is_empty() {
                prop_assert_eq!(opened, Err(RoundEnd::Exhausted));
                return Ok(());
            }
            let mut round = opened.unwrap();
            let first = held
                .filter(|account| usable.contains(account))
                .or_else(|| accounts.iter().find(|account| usable.contains(*account)).cloned());
            prop_assert_eq!(round.current().cloned(), first.clone());
            let mut on = first.unwrap();
            let mut visited = vec![on.clone()];
            let mut sends = BTreeMap::from([(on.clone(), 1u32)]);
            let mut total = 1u32;
            for event in events {
                let Some(err) = failure(event) else {
                    if round.admits_repair() {
                        *sends.get_mut(&on).unwrap() += 1;
                        total += 1;
                        round = round.repaired();
                    }
                    continue;
                };
                let step = round.next(&err);
                match event {
                    Event::Refused => {
                        prop_assert_eq!(&step, &AccountStep::Stop { why: RoundEnd::Refused });
                    }
                    Event::Unusable => prop_assert_ne!(&step, &AccountStep::Resend),
                    Event::Lost => prop_assert_ne!(switched(&step), true),
                    Event::Busy | Event::Repair => {}
                }
                match &step {
                    AccountStep::Resend => {
                        *sends.get_mut(&on).unwrap() += 1;
                    }
                    AccountStep::Switch { to } => {
                        prop_assert!(
                            usable.contains(to) && !visited.contains(to),
                            "{to} was tried already or is not redeemable"
                        );
                        visited.push(to.clone());
                        sends.insert(to.clone(), 1);
                        on = to.clone();
                    }
                    AccountStep::Stop { .. } => break,
                }
                total += 1;
                round = round.apply(&step);
                prop_assert_eq!(round.current(), Some(&on));
            }
            prop_assert!(sends.values().all(|&sent| sent <= budget), "{sends:?} over {budget}");
            prop_assert!(total <= u32::try_from(listed).unwrap() * budget);
            if let Retries::AtMost(retries) = cap {
                prop_assert!(total <= retries + 1, "{total} sends under AtMost({retries})");
            }
        }

        /// A single-account roster reads `retry` alone
        /// (`a_single_account_ignores_the_disposition`): it never switches,
        /// a refusal stops it whether or not it says to switch, the repair
        /// stage is always let through uncounted, and the person's ceiling
        /// bounds its sends.
        #[test]
        fn a_single_account_round_reads_retry_alone(
            cap in cap(),
            events in proptest::collection::vec(event(), 0..40),
        ) {
            let mut round = AccountRound::start(Roster::Single, BTreeSet::new(), cap, None).unwrap();
            prop_assert_eq!(round.current(), None);
            let mut total = 1u32;
            for event in events {
                let Some(err) = failure(event) else {
                    prop_assert!(round.admits_repair());
                    round = round.repaired();
                    continue;
                };
                let step = round.next(&err);
                match event {
                    Event::Refused | Event::Unusable => {
                        prop_assert_eq!(&step, &AccountStep::Stop { why: RoundEnd::Refused });
                    }
                    Event::Busy | Event::Lost | Event::Repair => {
                        prop_assert_ne!(switched(&step), true);
                    }
                }
                if let AccountStep::Stop { .. } = step {
                    break;
                }
                total += 1;
                round = round.apply(&step);
            }
            if let Retries::AtMost(retries) = cap {
                prop_assert!(total <= retries + 1);
            }
        }
    }
}
