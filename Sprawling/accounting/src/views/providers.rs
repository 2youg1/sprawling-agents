// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::config::SearchConfiguration;
use kernel::event::record::{ModelFacts, ProviderAccount};
use kernel::{Address, AxError, DialectKind};

use super::prepared::unavailable_because;

type Vault = Arc<Mutex<gateway::Custodian>>;

pub enum ProviderAsk {
    Endpoints {
        held: wire::EndpointsAnswer,
        vault: Option<Vault>,
    },

    Config {
        city_root: PathBuf,
        addr: Address,
        vault: Option<Vault>,
    },
}

impl ProviderAsk {
    pub(super) fn answer(self) -> wire::Answer {
        match self {
            ProviderAsk::Endpoints { held, vault } => match with_keys(held, vault.as_ref()) {
                Ok(answer) => wire::Answer::Endpoints(answer),
                Err(stopped) => unavailable_because("EndpointView".to_owned(), &stopped),
            },
            ProviderAsk::Config {
                city_root,
                addr,
                vault,
            } => match config_answer(&city_root, &addr, vault.as_ref()) {
                Ok(answer) => wire::Answer::Config(Box::new(answer)),
                Err(stopped) => unavailable_because(format!("Config({})", addr.as_str()), &stopped),
            },
        }
    }
}

pub(super) fn endpoints_answer(book: &gateway::EndpointBook) -> wire::EndpointsAnswer {
    let endpoints = book
        .endpoints()
        .map(|endpoint| wire::EndpointSummary {
            name: endpoint.name.clone(),
            label: endpoint.label().to_owned(),
            base_url: endpoint.base_url.clone(),
            dialect: endpoint.dialect,
            connection_kind: endpoint.connection_kind.as_str().to_owned(),
            models: endpoint
                .models
                .iter()
                .map(|row| wire::ModelFactsSummary {
                    id: row.id.clone(),
                    context_tokens: row.context_tokens.and_then(kernel::Window::new),
                    max_output_tokens: row.max_output_tokens,
                    input_modalities: row.input_modalities.clone(),
                    input_price: row.input_price.clone(),
                    output_price: row.output_price.clone(),
                    thinking: thinking_summary(gateway::thinking_offer(endpoint, &row.id)),
                    generation: generation_summary(gateway::generation_answer(endpoint, &row.id)),
                    canonical: gateway::canonical_model(row),
                    listing: row.listing,
                })
                .collect(),
            local: endpoint.is_local(),
            has_credential: endpoint.has_credential(),
            tuning: crate::tuning::tuning_as_attached(&endpoint.tuning),
            account_status: Vec::new(),
        })
        .collect();
    let chosen = book
        .choices()
        .map(|(tag, endpoint, entry)| wire::ChosenSummary {
            tag,
            endpoint: endpoint.to_owned(),
            model: entry.id.clone(),
            max_output_tokens: entry.max_output_tokens,
        })
        .collect();
    wire::EndpointsAnswer { endpoints, chosen }
}

fn parameter_summary(answer: gateway::ParameterAnswer) -> wire::ParameterAnswer {
    wire::ParameterAnswer {
        support: answer.support,
        source: answer.source,
    }
}

fn generation_summary(answer: gateway::GenerationAnswer) -> wire::GenerationAnswer {
    wire::GenerationAnswer {
        settings: answer.settings,
        output_tokens: answer.output_tokens,
        temperature: parameter_summary(answer.temperature),
        top_p: parameter_summary(answer.top_p),
        top_k: parameter_summary(answer.top_k),
        output_pointer: answer.output_pointer.to_owned(),
    }
}

fn thinking_summary(offer: gateway::ThinkingOffer) -> wire::ThinkingOffer {
    wire::ThinkingOffer {
        levels: offer.levels.iter().collect(),
        on: match offer.on {
            gateway::Switch::Allowed => wire::Switch::Allowed,
            gateway::Switch::Refused => wire::Switch::Refused,
            gateway::Switch::Unknown => wire::Switch::Unknown,
        },
        default: offer.default,
        default_on: offer.default_on,
        words: Vec::new(),
        from: match offer.from {
            gateway::OfferSource::Person => wire::OfferSource::Person,
            gateway::OfferSource::Upstream => wire::OfferSource::Upstream,
            gateway::OfferSource::Preset => wire::OfferSource::Preset,
            gateway::OfferSource::Unknown => wire::OfferSource::Unknown,
        },
        source: offer.source.map(str::to_owned),
    }
}

pub(super) struct TableRows<'q> {
    pub(super) endpoint: &'q str,
    pub(super) base_url: &'q str,
    pub(super) face: DialectKind,
    pub(super) models: &'q [wire::FiguresAsked],
}

pub(super) fn figures_answer(book: &gateway::EndpointBook, asked: &TableRows<'_>) -> wire::Answer {
    match figures_of(book, asked) {
        Ok(models) => wire::Answer::ModelFigures(wire::ModelFiguresAnswer { models }),
        Err(stopped) => unavailable_because("ModelFigures".to_owned(), &stopped),
    }
}

fn figures_of(
    book: &gateway::EndpointBook,
    asked: &TableRows<'_>,
) -> Result<Vec<wire::FiguresLine>, AxError> {
    let called = gateway::normalise_entered(asked.base_url, gateway::DialectHint::of(asked.face))?;
    let face = called.face();
    let catalogue = gateway::MarketSnapshot::builtin()?;
    Ok(asked
        .models
        .iter()
        .map(|row| {
            let listed = ModelFacts {
                id: row.id.clone(),
                context_tokens: row.context_tokens.map(kernel::Window::get),
                max_output_tokens: row.max_output_tokens,
                ..ModelFacts::default()
            };
            let pinned = catalogue.lookup(&row.id);
            let figures = gateway::Figures::resolve(
                gateway::Statements {
                    listed: Some(&listed),
                    registered: row
                        .tag
                        .and_then(|tag| book.registered(tag, asked.endpoint, &row.id)),
                    ..gateway::Statements::default()
                },
                pinned,
                gateway::Target {
                    base_url: &called.base_url,
                    id: &row.id,
                    wire: face,
                },
            );
            wire::FiguresLine {
                id: row.id.clone(),
                window: figures.window.map(|window| wire::WindowLine {
                    tokens: window.tokens,
                    from: figure_from(window.from),
                }),
                ceiling: figures.ceiling.map(ceiling_line),
                input: gateway::accepted_input(
                    None,
                    pinned.map(|entry| entry.input),
                    &called.base_url,
                    &row.id,
                ),
            }
        })
        .collect())
}

fn ceiling_line(ceiling: gateway::OutputCeiling) -> wire::CeilingLine {
    match ceiling {
        gateway::OutputCeiling::Sent { tokens, from } => wire::CeilingLine::Sent {
            tokens,
            from: figure_from(from),
        },
        gateway::OutputCeiling::ProviderDefault => wire::CeilingLine::ProviderDefault,
    }
}

fn figure_from(source: gateway::FigureSource) -> wire::FigureFrom {
    match source {
        gateway::FigureSource::Person => wire::FigureFrom::Person,
        gateway::FigureSource::Upstream => wire::FigureFrom::Upstream,
        gateway::FigureSource::Registered => wire::FigureFrom::Registered,
        gateway::FigureSource::Preset => wire::FigureFrom::Preset,
        gateway::FigureSource::Policy => wire::FigureFrom::Policy,
    }
}

fn with_keys(
    held: wire::EndpointsAnswer,
    vault: Option<&Vault>,
) -> Result<wire::EndpointsAnswer, AxError> {
    let custodian = vault.map(lock).transpose()?;
    let endpoints = held
        .endpoints
        .into_iter()
        .map(|endpoint| wire::EndpointSummary {
            account_status: endpoint
                .tuning
                .accounts
                .as_deref()
                .map_or_else(Vec::new, |listed| statuses(listed, custodian.as_deref())),
            ..endpoint
        })
        .collect();
    Ok(wire::EndpointsAnswer {
        endpoints,
        chosen: held.chosen,
    })
}

pub(crate) fn config_answer(
    city_root: &Path,
    addr: &Address,
    vault: Option<&Vault>,
) -> Result<wire::ConfigAnswer, AxError> {
    let defaults = gateway::EndpointTuning::DEFAULTS;
    let domain = wire::SecondDomain {
        min: kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
        max: kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
    };
    let building = city::Building::of(addr)?;
    let context_lock_tokens = city::load(city_root, building.addr())?
        .context_lock()
        .map(|lock| lock.tokens().get());
    Ok(wire::ConfigAnswer {
        addr: addr.clone(),
        effort: city::settled_effort(city_root, addr)?.map(|(effort, layer)| wire::SettledEffort {
            effort,
            from: rung_of(layer),
        }),
        second: city::settled_second(city_root, addr)?.map_or(
            wire::SettledSecond {
                percent: kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
                from: wire::ConfigLayer::Default,
                domain,
            },
            |(threshold, layer)| wire::SettledSecond {
                percent: u64::from(threshold),
                from: rung_of(layer),
                domain,
            },
        ),
        first: Some(kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT),
        context_lock_tokens,
        tuning: wire::TuningDefaults {
            from: wire::ConfigLayer::Default,
            timeout_ms: defaults.timeout_ms,
            request_max_retries: defaults.retries.stated(),
            stream_idle_timeout_ms: defaults.stream_idle_timeout_ms,
            proxying: kernel::Proxying::default(),
            account_retries: defaults.account_retries,
        },
        search: settled_search(city_root, addr, vault)?,
    })
}

fn settled_search(
    city_root: &Path,
    addr: &Address,
    vault: Option<&Vault>,
) -> Result<wire::SettledSearch, AxError> {
    let (configuration, from) = city::settled_search(city_root, addr)?.map_or(
        (SearchConfiguration::Default, wire::ConfigLayer::Default),
        |(configuration, layer)| (configuration, rung_of(layer)),
    );
    let city = city::city_search(city_root)?;
    let account_status = match &city {
        Some(SearchConfiguration::Custom { suppliers, .. }) => {
            let custodian = vault.map(lock).transpose()?;
            suppliers
                .iter()
                .map(|supplier| wire::SupplierAccounts {
                    supplier: supplier.id.clone(),
                    accounts: statuses(&supplier.accounts, custodian.as_deref()),
                })
                .collect()
        }
        Some(SearchConfiguration::Default | SearchConfiguration::Off) | None => Vec::new(),
    };
    Ok(wire::SettledSearch {
        configuration,
        from,
        city,
        default_url: city::default_search_supplier()?.url,
        account_status,
    })
}

fn rung_of(layer: city::Layer) -> wire::ConfigLayer {
    match layer {
        city::Layer::City => wire::ConfigLayer::City,
        city::Layer::Building => wire::ConfigLayer::Building,
        city::Layer::Resident => wire::ConfigLayer::Resident,
    }
}

fn lock(vault: &Vault) -> Result<std::sync::MutexGuard<'_, gateway::Custodian>, AxError> {
    vault
        .lock()
        .map_err(|_poisoned| crate::held_vault::poisoned_vault())
}

fn statuses(
    accounts: &[ProviderAccount],
    custodian: Option<&gateway::Custodian>,
) -> Vec<wire::AccountStatus> {
    accounts
        .iter()
        .map(|account| wire::AccountStatus {
            id: account.id.clone(),
            key: key_state(account, custodian),
        })
        .collect()
}

fn key_state(account: &ProviderAccount, custodian: Option<&gateway::Custodian>) -> wire::KeyState {
    let Some(reference) = &account.reference else {
        return wire::KeyState::Anonymous;
    };
    let Some(custodian) = custodian else {
        return wire::KeyState::Unread;
    };
    let described = custodian.describe(reference);
    match (described.writable, described.configured) {
        (true, true) => wire::KeyState::Stored,
        (true, false) => wire::KeyState::Missing,
        (false, true) => wire::KeyState::Environment,
        (false, false) => wire::KeyState::EnvironmentUnusable,
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
