// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::account_recovery::{AccountRound, Roster, RoundEnd};
use kernel::model::{AccountRoster, RosterEntry};
use kernel::{AxCode, AxError, Retries, ServerLabel};

#[derive(Debug)]
pub(super) struct Round {
    pub(super) accounts: AccountRound,
    provider: String,

    listed: Vec<RosterEntry>,

    met: BTreeMap<ServerLabel, String>,
}

impl Round {

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

    pub(super) fn note(&mut self, failure: &AxError) {
        if let Some(account) = self.accounts.current() {
            self.met.insert(account.clone(), met(failure));
        }
    }

    pub(super) fn repaired(self) -> Round {
        Round {
            accounts: self.accounts.repaired(),
            ..self
        }
    }

    pub(super) fn exhausted(&self) -> AxError {
        exhausted_error(&self.provider, &self.listed, &self.met)
    }
}

pub(super) fn nothing_listed() -> AxError {
    exhausted_error("", &[], &BTreeMap::new())
}

fn met(failure: &AxError) -> String {
    match failure.provider_failure() {
        Some(kind) => format!("{} {kind:?}", failure.code()),
        None => failure.code().to_string(),
    }
}

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
