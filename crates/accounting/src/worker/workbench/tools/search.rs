// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `web_search`: the city's web search tool, sent to the one supplier
//! the run's frozen `[search]` names, over MCP streamable HTTP, through
//! the ordinary connector gate (`crates/accounting/spec/Connectors.lean`
//! §8-35, accounting D52).
//!
//! Which account a call goes out on, whether a failed call goes again,
//! on which account, and when it stops are the answers of kernel's
//! `AccountRound`; this tool only carries out the step it is handed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::account_recovery::{AccountRound, AccountStep, Roster, RoundEnd};
use kernel::config::{SearchConfiguration, SearchSupplier};
use kernel::event::record::ProviderAccount;
use kernel::{
    AxCode, AxError, RunId, ServerLabel, TimeoutMs, Tool, ToolCall, ToolMeta, ToolOutcome,
};

use super::{Laying, Site};

mod mapping;

use mapping::Asked;

/// The name the model calls the tool by, whichever supplier answers.
const NAME: &str = "web_search";

impl Laying {
    /// The `web_search` this run is offered, or `None` when it is offered
    /// none: in a confidential building, under `Off`, and when the frozen
    /// value names a supplier it does not list.
    ///
    /// # Errors
    /// Propagates a tool that refuses to be built.
    pub(super) fn search_tool(&self, site: &Site) -> Result<Option<WebSearch>, AxError> {
        self.web_search(
            site.rules.policy(),
            &site.config.search,
            &site.write_root,
            site.run_id,
        )
    }

    /// [`Laying::search_tool`] over the values it reads from the run's
    /// site.
    ///
    /// A confidential building's data may come in and may not go out,
    /// and a search sends the run's own words to a service outside the
    /// city, so such a building is offered none, whatever `[search]`
    /// says. A supplier `city` refuses to resolve is named in the
    /// diagnostics and the run goes on without the tool, as it does
    /// without an MCP server that would not start.
    fn web_search(
        &self,
        policy: &kernel::BuildingPolicy,
        configuration: &SearchConfiguration,
        root: &Path,
        run: RunId,
    ) -> Result<Option<WebSearch>, AxError> {
        if policy.confidential {
            return Ok(None);
        }
        let supplier = match city::search_supplier(configuration) {
            Ok(Some(supplier)) => supplier,
            Ok(None) => return Ok(None),
            Err(err) => {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "accounting::worker",
                    &format!("{NAME} is not offered: {err}; {}", err.recovery()),
                );
                return Ok(None);
            }
        };
        WebSearch::new(
            supplier,
            Reaching {
                root: root.to_path_buf(),
                run,
                vault: Arc::clone(&self.vault),
                connectors: Arc::clone(&self.connectors),
                monotonic: self.monotonic,
            },
        )
        .map(Some)
    }
}

/// `web_search` over one supplier, frozen with the run.
pub(super) struct WebSearch {
    meta: ToolMeta,
    supplier: SearchSupplier,
    reaching: Reaching,
}

/// What a search needs to reach its supplier and to wait between
/// attempts, taken from the laying once.
struct Reaching {
    root: PathBuf,
    /// Seeds the back-off schedule, so a run's waits replay alike.
    run: RunId,
    vault: Arc<Mutex<gateway::Custodian>>,
    connectors: Arc<dyn crate::Connectors + Send + Sync>,
    /// What a round's deadline and its waits are measured on.
    monotonic: fn() -> std::time::Instant,
}

impl WebSearch {
    fn new(supplier: SearchSupplier, reaching: Reaching) -> Result<WebSearch, AxError> {
        Ok(WebSearch {
            meta: ToolMeta {
                name: kernel::ToolName::parse(NAME)?,
                disclosure: format!(
                    "Search the web through {}. The result is the service's own answer, as it \
                     gave it.",
                    supplier.id.as_str()
                ),
                params: kernel::Payload::new(mapping::parameters(&supplier))?,
                effect: kernel::Effect::Connector {
                    label: supplier.id.clone(),
                },
                cost_tier: kernel::CostTier::Heavy,
                timeout: Some(agent_protocols::EXTERNAL_CALL_PATIENCE),
                render: kernel::RenderIntent::Generic,
                temporal: kernel::Temporal::Timestamped,
            },
            supplier,
            reaching,
        })
    }

    /// The round's roster: one account keeps the single-account rules,
    /// two or more are ordered, and the per-account budget is the one
    /// default the model path reads too (kernel D54).
    fn roster(&self) -> Roster {
        match self.supplier.accounts.as_slice() {
            [] | [_] => Roster::Single,
            several => Roster::Several {
                accounts: several.iter().map(|account| account.id.clone()).collect(),
                retries: gateway::EndpointTuning::DEFAULTS.account_retries,
            },
        }
    }

    /// The accounts whose credential can be redeemed now, asked once as
    /// the round opens without reading any key (kernel D55). An account
    /// with no reference is anonymous, and always can.
    fn usable(&self) -> Result<BTreeSet<ServerLabel>, AxError> {
        let vault = self
            .reaching
            .vault
            .lock()
            .map_err(|_| crate::held_vault::poisoned_vault())?;
        Ok(self
            .supplier
            .accounts
            .iter()
            .filter(|account| {
                account
                    .reference
                    .as_ref()
                    .is_none_or(|reference| vault.describe(reference).configured)
            })
            .map(|account| account.id.clone())
            .collect())
    }

    /// The account the round says goes next. A single-account roster
    /// names none, and its one account is the answer.
    fn account(&self, current: Option<&ServerLabel>) -> Result<&ProviderAccount, AxError> {
        let found = match current {
            Some(id) => self
                .supplier
                .accounts
                .iter()
                .find(|account| account.id == *id),
            None => self.supplier.accounts.first(),
        };
        found.ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "search the web",
                format!("{}: no account to send from", self.supplier.id.as_str()),
            )
            .with_recovery("give this supplier an account in the web search settings")
        })
    }

    /// One attempt on `account`: the supplier written as an MCP server
    /// carrying that account's header, its tools reached through the
    /// connectors, the mapping judged against the remote schema, and one
    /// `tools/call`.
    fn attempt(
        &self,
        account: &ProviderAccount,
        asked: &Asked,
        call: &ToolCall,
    ) -> Result<ToolOutcome, AxError> {
        let server = declaration(&self.supplier, account)?;
        let resolve = crate::held_vault::resolving(Arc::clone(&self.reaching.vault));
        let (tools, _) =
            self.reaching
                .connectors
                .connect(&server, &self.reaching.root, false, &resolve)?;
        let remote = tools
            .into_iter()
            .find(|tool| tool.remote() == self.supplier.remote)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "search the web",
                    format!(
                        "{} lists no tool named {}",
                        self.supplier.id.as_str(),
                        self.supplier.remote
                    ),
                )
                .with_recovery(
                    "name the remote tool this supplier's tools/list offers in the web search \
                     settings",
                )
            })?;
        let args = asked.mapped(&self.supplier, remote.meta().params.as_map())?;
        remote.invoke(&ToolCall {
            name: remote.meta().name.clone(),
            args,
            ..call.clone()
        })
    }
}

/// The supplier as one MCP server declaration under its own label: an
/// account with a reference carries it in the header it names, and an
/// anonymous account carries no header. The reference is part of the
/// declaration, so two accounts are two resident connections.
fn declaration(
    supplier: &SearchSupplier,
    account: &ProviderAccount,
) -> Result<kernel::McpServer, AxError> {
    let headers = match (&account.reference, &account.header) {
        (None, _) => Vec::new(),
        (Some(reference), Some(header)) => vec![(header.clone(), reference.to_string())],
        (Some(_), None) => {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "search the web",
                format!(
                    "{}: account {} has a key and no header to carry it",
                    supplier.id.as_str(),
                    account.id.as_str()
                ),
            )
            .with_recovery("name the HTTP header this supplier reads its key from"));
        }
    };
    Ok(kernel::McpServer {
        label: supplier.id.clone(),
        transport: kernel::McpTransport::Http {
            url: supplier.url.clone(),
            headers,
        },
    })
}

impl Tool for WebSearch {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// One round (kernel D54): the first attempt on the first redeemable
    /// account, then whatever step the round hands back after each
    /// failure, until an answer or a stop.
    ///
    /// No Halt reaches into a tool call, so the round keeps to the
    /// tool's own deadline: a wait that would end past it is not taken,
    /// and the failure that asked for it is the result.
    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "search the web",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!("call `{NAME}`, the name this tool answers to")));
        }
        let asked = Asked::read(&self.supplier, &call.args)?;
        let began = (self.reaching.monotonic)();
        let patience = self
            .meta
            .timeout
            .unwrap_or(agent_protocols::EXTERNAL_CALL_PATIENCE);
        let mut last: BTreeMap<ServerLabel, AxCode> = BTreeMap::new();
        // A round that cannot open has no account to send from: `start`
        // answers nothing but `Exhausted`.
        let mut round = AccountRound::start(
            self.roster(),
            self.usable()?,
            kernel::Retries::UntilHalted,
            None,
        )
        .map_err(|_| self.exhausted(&last))?;
        let mut in_a_row: u32 = 0;
        loop {
            let account = self.account(round.current())?;
            let failure = match self.attempt(account, &asked, call) {
                Ok(answer) => return Ok(answer),
                Err(failure) => failure,
            };
            last.insert(account.id.clone(), *failure.code());
            let step = round.next(&failure);
            match &step {
                AccountStep::Stop {
                    why: RoundEnd::Exhausted,
                } => return Err(self.exhausted(&last)),
                AccountStep::Stop {
                    why: RoundEnd::Refused | RoundEnd::Unknown | RoundEnd::Cap,
                } => return Err(failure),
                AccountStep::Resend => {
                    in_a_row = in_a_row.saturating_add(1);
                    let wait = std::time::Duration::from_millis(runtime::backoff_ms(
                        self.reaching.run,
                        in_a_row,
                        &failure,
                    ));
                    if !within(began, patience, wait, self.reaching.monotonic) {
                        return Err(failure);
                    }
                    std::thread::sleep(wait);
                }
                // A switch goes at once: the back-off was a statement
                // about the account that failed.
                AccountStep::Switch { .. } => {
                    in_a_row = 0;
                    if !within(
                        began,
                        patience,
                        std::time::Duration::ZERO,
                        self.reaching.monotonic,
                    ) {
                        return Err(failure);
                    }
                }
            }
            round = round.apply(&step);
        }
    }
}

/// Whether waiting `wait` from now still ends inside the round's
/// deadline, `patience` after `began`.
fn within(
    began: std::time::Instant,
    patience: TimeoutMs,
    wait: std::time::Duration,
    monotonic: fn() -> std::time::Instant,
) -> bool {
    monotonic()
        .saturating_duration_since(began)
        .saturating_add(wait)
        < std::time::Duration::from_millis(patience.0)
}

impl WebSearch {
    /// The failure a round ends with when no account is left to send
    /// from: one `E_PROVIDER_ACCOUNTS_EXHAUSTED` naming the supplier and
    /// each account with the last failure it met, never a key or a
    /// reference.
    fn exhausted(&self, last: &BTreeMap<ServerLabel, AxCode>) -> AxError {
        let accounts = self
            .supplier
            .accounts
            .iter()
            .map(|account| match last.get(&account.id) {
                Some(code) => format!("{} ({})", account.id.as_str(), code.as_str()),
                None => format!("{} (no credential stored)", account.id.as_str()),
            })
            .collect::<Vec<String>>()
            .join(", ");
        AxError::failure(
            AxCode::ProviderAccountsExhausted,
            "search the web",
            format!("{}: {accounts}", self.supplier.id.as_str()),
        )
        .with_recovery(
            "store a key or add credit for one of these accounts, or add another account in the \
             web search settings, then search again",
        )
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
mod tests;
