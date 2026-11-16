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
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use kernel::config::SearchSupplier;

    #[test]
    fn an_unstated_second_rung_is_answered_with_the_default_as_its_layer() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = serde_json::to_value(config_answer(dir.path(), &addr, None).unwrap()).unwrap();
        assert_eq!(
            answer.get("second"),
            Some(&serde_json::json!({
                "percent": kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
                "from": "default",
                "domain": {
                    "min": kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
                    "max": kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
                },
            }))
        );
    }

    #[test]
    fn the_first_rung_is_answered_beside_the_second() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = config_answer(dir.path(), &addr, None).unwrap();
        assert_eq!(
            answer.first,
            Some(kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT)
        );
    }

    #[test]
    fn the_tuning_defaults_are_answered_with_the_default_as_their_layer() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = serde_json::to_value(config_answer(dir.path(), &addr, None).unwrap()).unwrap();
        assert_eq!(
            answer.pointer("/tuning/from"),
            Some(&serde_json::json!("default"))
        );
    }

    fn account(id: &str, keyed: bool) -> ProviderAccount {
        ProviderAccount {
            id: kernel::ServerLabel::parse(id).unwrap(),
            reference: keyed.then(|| kernel::SecretRef::new("fixture", id).unwrap()),
            header: keyed.then(|| "x-api-key".to_owned()),
        }
    }

    #[test]
    fn every_key_state_is_read_from_what_the_vault_describes() {
        let mut vault = gateway::Custodian::in_memory().with_env_reader(Box::new(|key| {
            if key.ends_with("SHADED") {
                Ok("sk-from-the-environment".to_owned())
            } else if key.ends_with("SPOILT") {
                Err(std::env::VarError::NotUnicode(std::ffi::OsString::new()))
            } else {
                Err(std::env::VarError::NotPresent)
            }
        }));
        vault
            .set(
                &kernel::SecretRef::new("fixture", "stored").unwrap(),
                zeroize::Zeroizing::new("sk-stored".to_owned()),
            )
            .unwrap();
        let listed = [
            account("stored", true),
            account("missing", true),
            account("shaded", true),
            account("spoilt", true),
            account("open", false),
        ];
        let state = |id: &str, key| wire::AccountStatus {
            id: kernel::ServerLabel::parse(id).unwrap(),
            key,
        };

        assert_eq!(
            (statuses(&listed, Some(&vault)), statuses(&listed, None)),
            (
                vec![
                    state("stored", wire::KeyState::Stored),
                    state("missing", wire::KeyState::Missing),
                    state("shaded", wire::KeyState::Environment),
                    state("spoilt", wire::KeyState::EnvironmentUnusable),
                    state("open", wire::KeyState::Anonymous),
                ],
                vec![
                    state("stored", wire::KeyState::Unread),
                    state("missing", wire::KeyState::Unread),
                    state("shaded", wire::KeyState::Unread),
                    state("spoilt", wire::KeyState::Unread),
                    state("open", wire::KeyState::Anonymous),
                ],
            )
        );
    }

    #[test]
    fn the_search_answer_names_the_city_value_apart_from_a_building_override() {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        let custom = SearchConfiguration::Custom {
            selected: kernel::ServerLabel::parse("brave").unwrap(),
            suppliers: vec![SearchSupplier {
                id: kernel::ServerLabel::parse("brave").unwrap(),
                url: "https://search.example/mcp".to_owned(),
                remote: "brave_web_search".to_owned(),
                query_field: "query".to_owned(),
                objective_field: None,
                count_field: Some("count".to_owned()),
                accounts: vec![account("main", true)],
            }],
        };
        city::write_search(dir.path(), &custom).unwrap();
        let building =
            kernel::layout::CityLayout::new(dir.path()).config(&Address::parse("lab").unwrap());
        std::fs::create_dir_all(building.parent().unwrap()).unwrap();
        std::fs::write(&building, "[search]\nchoice = \"off\"\n").unwrap();
        let vault = Arc::new(Mutex::new(gateway::Custodian::in_memory()));

        let answered = config_answer(dir.path(), &room, Some(&vault)).unwrap();

        assert_eq!(
            answered.search,
            wire::SettledSearch {
                configuration: SearchConfiguration::Off,
                from: wire::ConfigLayer::Building,
                city: Some(custom),
                default_url: city::default_search_supplier().unwrap().url,
                account_status: vec![wire::SupplierAccounts {
                    supplier: kernel::ServerLabel::parse("brave").unwrap(),
                    accounts: vec![wire::AccountStatus {
                        id: kernel::ServerLabel::parse("main").unwrap(),
                        key: wire::KeyState::Missing,
                    }],
                }],
            }
        );
    }

    #[test]
    fn a_relay_forwarding_a_vendors_id_answers_the_vendors_window() {
        let book = gateway::EndpointBook::new();
        let sonnet = [wire::FiguresAsked {
            id: "claude-sonnet-4-5".to_owned(),
            context_tokens: None,
            max_output_tokens: None,
            tag: None,
        }];
        let ask = |base_url: &str, face| {
            figures_answer(
                &book,
                &TableRows {
                    endpoint: "relay",
                    base_url,
                    face,
                    models: &sonnet,
                },
            )
        };
        let line = |window: Option<u64>, ceiling, input| {
            wire::Answer::ModelFigures(wire::ModelFiguresAnswer {
                models: vec![wire::FiguresLine {
                    id: "claude-sonnet-4-5".to_owned(),
                    window: window
                        .and_then(kernel::Window::new)
                        .map(|tokens| wire::WindowLine {
                            tokens,
                            from: wire::FigureFrom::Preset,
                        }),
                    ceiling: Some(ceiling),
                    input,
                }],
            })
        };
        let vendor = wire::CeilingLine::Sent {
            tokens: kernel::Ceiling::new(64_000).unwrap(),
            from: wire::FigureFrom::Preset,
        };
        assert_eq!(
            [
                ask("relay.example.test/v1", DialectKind::OpenAi),
                ask("https://relay.example.test/v1", DialectKind::Anthropic),
                ask("http://127.0.0.1:8811/v1", DialectKind::OpenAi),
            ],
            [
                line(
                    Some(200_000),
                    wire::CeilingLine::ProviderDefault,
                    kernel::InputKinds::TextImage,
                ),
                line(Some(200_000), vendor, kernel::InputKinds::TextImage),
                line(
                    None,
                    wire::CeilingLine::ProviderDefault,
                    kernel::InputKinds::Text,
                ),
            ]
        );
        assert!(matches!(
            ask("mailto:somebody", DialectKind::OpenAi),
            wire::Answer::Unavailable {
                reason: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn each_rung_travels_under_the_word_the_ledger_records() {
        let sources = [
            gateway::FigureSource::Person,
            gateway::FigureSource::Upstream,
            gateway::FigureSource::Registered,
            gateway::FigureSource::Preset,
            gateway::FigureSource::Policy,
        ];
        assert_eq!(
            sources.map(|source| serde_json::to_value(figure_from(source)).unwrap()),
            sources.map(|source| serde_json::json!(source.as_str()))
        );
    }
}
