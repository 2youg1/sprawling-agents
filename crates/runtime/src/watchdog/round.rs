// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One model call's account round as the watchdog holds it: opened from
//! the adapter's roster, with what each account met, which is what the
//! exhausted refusal names (`crates/runtime/spec/Watchdog.lean` §8-9).

use std::collections::BTreeMap;

use kernel::account_recovery::{AccountRound, Roster, RoundEnd};
use kernel::model::{AccountRoster, RosterEntry};
use kernel::{AxCode, AxError, Retries, ServerLabel};

/// One model call's account round, with what each account met in it:
/// the subject of `E_PROVIDER_ACCOUNTS_EXHAUSTED` is read from here.
#[derive(Debug)]
pub(super) struct Round {
    pub(super) accounts: AccountRound,
    provider: String,
    /// The roster as it was when the round opened, in priority order.
    listed: Vec<RosterEntry>,
    /// The last failure each account met in this round, as its code and
    /// kind.
    met: BTreeMap<ServerLabel, String>,
}

impl Round {
    /// The round a call opens: several accounts when the roster lists two
    /// or more, a single-account round otherwise, which disposes of a
    /// failure as this watchdog always has.
    pub(super) fn open(roster: Option<AccountRoster>, cap: Retries) -> Result<Round, AxError> {
        let (provider, listed, held, retries) = match roster {
            Some(roster) if roster.accounts.len() > 1 => (
                roster.provider,
                roster.accounts,
                Some(roster.current),
                Some(roster.retries),
            ),
            Some(_) | None => (String::new(), Vec::new(), None, None),
        };
        let shape = match retries {
            Some(retries) => Roster::Several {
                accounts: listed.iter().map(|entry| entry.account.clone()).collect(),
                retries,
            },
            None => Roster::Single,
        };
        let usable = listed
            .iter()
            .filter(|entry| entry.usable)
            .map(|entry| entry.account.clone())
            .collect();
        match AccountRound::start(shape, usable, cap, held) {
            Ok(accounts) => Ok(Round {
                accounts,
                provider,
                listed,
                met: BTreeMap::new(),
            }),
            Err(RoundEnd::Exhausted | RoundEnd::Refused | RoundEnd::Unknown | RoundEnd::Cap) => {
                Err(exhausted_error(&provider, &listed, &BTreeMap::new()))
            }
        }
    }

    /// Notes what the current account met, for the exhausted refusal.
    pub(super) fn note(&mut self, failure: &AxError) {
        if let Some(account) = self.accounts.current() {
            self.met.insert(account.clone(), met(failure));
        }
    }

    /// The round after the repair segment sent the call again.
    pub(super) fn repaired(self) -> Round {
        Round {
            accounts: self.accounts.repaired(),
            ..self
        }
    }

    /// `E_PROVIDER_ACCOUNTS_EXHAUSTED` for this round.
    pub(super) fn exhausted(&self) -> AxError {
        exhausted_error(&self.provider, &self.listed, &self.met)
    }
}

/// `E_PROVIDER_ACCOUNTS_EXHAUSTED` when no round is open to name the
/// accounts.
pub(super) fn nothing_listed() -> AxError {
    exhausted_error("", &[], &BTreeMap::new())
}

/// What one account met, for the exhausted subject: the stable code and,
/// for a model call's failure, its kind in Rust's own `Debug` spelling,
/// so no second spelling of the wire's words is written here.
fn met(failure: &AxError) -> String {
    match failure.provider_failure() {
        Some(kind) => format!("{} {kind:?}", failure.code()),
        None => failure.code().to_string(),
    }
}

/// `E_PROVIDER_ACCOUNTS_EXHAUSTED` for `provider`: each listed account in
/// priority order, with what it met or that it could not be redeemed.
fn exhausted_error(
    provider: &str,
    listed: &[RosterEntry],
    met: &BTreeMap<ServerLabel, String>,
) -> AxError {
    let accounts = listed
        .iter()
        .filter_map(|entry| match (met.get(&entry.account), entry.usable) {
            (Some(failure), _) => Some(format!("{} {failure}", entry.account)),
            (None, false) => Some(format!("{} not redeemable", entry.account)),
            (None, true) => None,
        })
        .collect::<Vec<_>>()
        .join("; ");
    AxError::failure(
        AxCode::ProviderAccountsExhausted,
        "call the model",
        format!("{provider}: {accounts}"),
    )
    .with_recovery(
        "on the model settings page, file a key or add credit for one of this provider's \
         accounts, or add another account, then dispatch again",
    )
}
