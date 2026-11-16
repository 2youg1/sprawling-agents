// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use crate::error::{AccountDisposition, AxError, Retry};
use crate::retries::Retries;
use crate::tool::ServerLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Roster {
    Single,

    Several {
        accounts: Vec<ServerLabel>,
        retries: AccountRetries,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundEnd {
    Refused,

    Unknown,

    Cap,

    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStep {
    Resend,

    Switch { to: ServerLabel },

    Stop { why: RoundEnd },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRound {
    roster: Roster,

    usable: BTreeSet<ServerLabel>,
    cap: Retries,
    current: Option<ServerLabel>,

    tried: BTreeSet<ServerLabel>,

    here: u32,

    total: u32,
}

impl AccountRound {
    pub fn start(
        roster: Roster,
        usable: BTreeSet<ServerLabel>,
        cap: Retries,
        held: Option<ServerLabel>,
    ) -> Result<AccountRound, RoundEnd> {
        let current = match &roster {
            Roster::Single => None,
            Roster::Several { accounts, .. } => {
                let opening = held
                    .filter(|account| accounts.contains(account) && usable.contains(account))
                    .or_else(|| {
                        accounts
                            .iter()
                            .find(|account| usable.contains(*account))
                            .cloned()
                    });
                Some(opening.ok_or(RoundEnd::Exhausted)?)
            }
        };
        Ok(AccountRound {
            roster,
            usable,
            cap,
            tried: current.iter().cloned().collect(),
            current,
            here: 1,
            total: 1,
        })
    }

    #[must_use]
    pub fn current(&self) -> Option<&ServerLabel> {
        self.current.as_ref()
    }

    #[must_use]
    pub fn next(&self, failure: &AxError) -> AccountStep {
        match &self.roster {
            Roster::Single => match failure.retry() {
                Retry::No => AccountStep::Stop {
                    why: RoundEnd::Refused,
                },
                Retry::Yes | Retry::Unknown => self.resend(),
            },
            Roster::Several { retries, .. } => {
                let spare = self.here <= retries.count();
                match (failure.retry(), failure.account()) {
                    (Retry::Unknown, AccountDisposition::Keep | AccountDisposition::Advance) => {
                        if spare {
                            self.resend()
                        } else {
                            AccountStep::Stop {
                                why: RoundEnd::Unknown,
                            }
                        }
                    }
                    (Retry::Yes | Retry::No, AccountDisposition::Advance) => self.switch(),
                    (Retry::Yes, AccountDisposition::Keep) => {
                        if spare {
                            self.resend()
                        } else {
                            self.switch()
                        }
                    }
                    (Retry::No, AccountDisposition::Keep) => AccountStep::Stop {
                        why: RoundEnd::Refused,
                    },
                }
            }
        }
    }

    #[must_use]
    pub fn apply(self, step: &AccountStep) -> AccountRound {
        match step {
            AccountStep::Resend => AccountRound {
                here: self.here.saturating_add(1),
                total: self.total.saturating_add(1),
                ..self
            },
            AccountStep::Switch { to } => {
                let mut tried = self.tried;
                tried.insert(to.clone());
                AccountRound {
                    current: Some(to.clone()),
                    tried,
                    here: 1,
                    total: self.total.saturating_add(1),
                    ..self
                }
            }
            AccountStep::Stop { .. } => self,
        }
    }

    #[must_use]
    pub fn admits_repair(&self) -> bool {
        match &self.roster {
            Roster::Single => true,
            Roster::Several { retries, .. } => {
                self.here <= retries.count() && admits(self.cap, self.total)
            }
        }
    }

    #[must_use]
    pub fn repaired(self) -> AccountRound {
        match self.roster {
            Roster::Single => self,
            Roster::Several { .. } => self.apply(&AccountStep::Resend),
        }
    }

    fn resend(&self) -> AccountStep {
        if admits(self.cap, self.total) {
            AccountStep::Resend
        } else {
            AccountStep::Stop { why: RoundEnd::Cap }
        }
    }

    fn switch(&self) -> AccountStep {
        let Roster::Several { accounts, .. } = &self.roster else {
            return AccountStep::Stop {
                why: RoundEnd::Exhausted,
            };
        };
        match accounts
            .iter()
            .find(|account| self.usable.contains(*account) && !self.tried.contains(*account))
        {
            None => AccountStep::Stop {
                why: RoundEnd::Exhausted,
            },
            Some(to) if admits(self.cap, self.total) => AccountStep::Switch { to: to.clone() },
            Some(_) => AccountStep::Stop { why: RoundEnd::Cap },
        }
    }
}

fn admits(cap: Retries, sent: u32) -> bool {
    match cap {
        Retries::UntilHalted => true,
        Retries::AtMost(retries) => sent <= retries,
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
mod tests;
