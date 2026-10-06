// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the settings page reads back about providers and the
//! configuration ladder: every attached endpoint with the tuning it was
//! attached with, the ladder of one address with its `[search]`, and
//! whether each listed account's key is there.
//!
//! Both answers ask the vault, and asking the vault asks the platform's
//! credential service, so both finish after the snapshot is let go. The
//! one reading of an account into a [`wire::KeyState`] lives here, for
//! both (`crates/accounting/spec/Views.lean` §8-36).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::config::SearchConfiguration;
use kernel::event::record::ProviderAccount;
use kernel::{Address, AxError};

use super::prepared::unavailable_because;

/// The vault the worker lent the views.
type Vault = Arc<Mutex<gateway::Custodian>>;

/// What the two provider questions copied out of the snapshot.
pub enum ProviderAsk {
    /// The endpoint book as it stood, each row's keys still to read.
    Endpoints {
        held: wire::EndpointsAnswer,
        vault: Option<Vault>,
    },
    /// The configuration ladder of one address.
    Config {
        city_root: PathBuf,
        addr: Address,
        vault: Option<Vault>,
    },
}

impl ProviderAsk {
    /// The answer, read with the snapshot let go.
    ///
    /// A ladder, a `[search]` table or a vault lock that cannot be read
    /// is "I could not look", with what stopped it, rather than an answer
    /// missing a part.
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

/// The settings page's read of the endpoint book, while the snapshot is
/// held: every row with the tuning it was attached with, and no key
/// read yet.
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

/// The book's rows with each listed account's key read, in the order
/// the row lists them.
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

/// What one address is governed by, value by value, with the file each
/// value came from.
///
/// The ladder is climbed once, by the module that owns it, and the rung
/// it answers with is carried through rather than re-derived: a page
/// told only the resolved setting would have to read all three files and
/// climb the same ladder a second time, and two climbs of one ladder are
/// two answers to one question.
///
/// The figures an endpoint nobody tuned is called with are read out of
/// `gateway::EndpointTuning::DEFAULTS`, which is their one home; a form
/// that printed its own numbers into empty boxes is what this answer
/// exists to retire.
///
/// # Errors
/// Propagates a ladder or a `[search]` table that cannot be read, and a
/// poisoned vault lock.
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

/// `[search]` at `addr`, the city's own statement of it, and the keys
/// of the suppliers the city lists.
fn settled_search(
    city_root: &Path,
    addr: &Address,
    vault: Option<&Vault>,
) -> Result<wire::SettledSearch, AxError> {
    let (_, _, _) = (city_root, addr, vault);
    Ok(wire::SettledSearch {
        configuration: SearchConfiguration::Default,
        from: wire::ConfigLayer::Default,
        city: None,
        default_url: city::default_search_supplier()?.url,
        account_status: Vec::new(),
    })
}

/// The one place the ladder's rung becomes the wire's. Exhaustive, so a
/// rung added to the ladder is a compiler error here rather than a page
/// that silently reports the wrong file.
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

/// Each account's key, in the order listed.
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

/// Where one account's key stands. `describe` answers `writable: false`
/// only for a key an environment variable supplies, so that is the
/// environment's pair of states, and the vault's pair is read by
/// whether it holds a value.
fn key_state(account: &ProviderAccount, custodian: Option<&gateway::Custodian>) -> wire::KeyState {
    match (&account.reference, custodian) {
        (None, _) => wire::KeyState::Anonymous,
        (Some(_), _) => wire::KeyState::Unread,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use kernel::config::SearchSupplier;

    /// A value no file states is still answered, with the default named
    /// as the layer it came from and the domain a file may state; a page
    /// told `null` has to keep its own copy of both to draw anything.
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

    /// Both rungs the context ring draws travel on the one answer it
    /// reads; the page keeps no copy of the first (`crates/wire/Spec.lean`
    /// §8-77).
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

    /// The figures an untuned endpoint is called with name their layer
    /// too: no file on the ladder states them, so the answer says they
    /// are this build's own rather than leaving the page to guess.
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

    /// Each of the six words is what the vault says about one account:
    /// a key stored, none, one an environment variable supplies, one it
    /// spoils, no reference at all, and no vault to ask.
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

    /// A building that states its own `[search]` governs the room, and
    /// the page is still handed the city's own value to edit, with the
    /// keys of the suppliers that value lists and the default's address.
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
}
