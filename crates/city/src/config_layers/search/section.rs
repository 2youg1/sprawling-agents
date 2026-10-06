// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[search]` table as a file spells it. The reader and the writer
//! share these sections, so the file's syntax has one definition; what a
//! value may say is [`super::check`]'s answer.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4c.

use std::path::Path;

use kernel::config::{SearchConfiguration, SearchSupplier};
use kernel::event::record::ProviderAccount;
use kernel::{AxCode, AxError, SecretRef, ServerLabel};
use serde::{Deserialize, Serialize};

use super::super::refuse::refuse;
use super::{SEARCH_KEY, check};

/// Reads one layer's `[search]` table into the value it states.
///
/// # Errors
/// `E_CONFIG_INVALID` for a label or reference `kernel` refuses,
/// `selected` or `suppliers` beside `default` or `off`, a `custom` value
/// without `selected`, and every value [`check`] refuses.
pub(crate) fn stated(section: SearchSection) -> Result<SearchConfiguration, AxError> {
    let configuration = match (section.choice, section.selected, section.suppliers) {
        (Choice::Default, None, None) => SearchConfiguration::Default,
        (Choice::Off, None, None) => SearchConfiguration::Off,
        (Choice::Default | Choice::Off, _, _) => {
            return Err(refuse(format!(
                "{SEARCH_KEY}: `selected` and `suppliers` belong to `choice = \"custom\"`"
            )));
        }
        (Choice::Custom, None, _) => {
            return Err(refuse(format!(
                "{SEARCH_KEY}: `choice = \"custom\"` names no `selected` supplier"
            )));
        }
        (Choice::Custom, Some(selected), suppliers) => SearchConfiguration::Custom {
            selected: label(&selected)?,
            suppliers: suppliers
                .unwrap_or_default()
                .into_iter()
                .map(SupplierSection::read)
                .collect::<Result<_, _>>()?,
        },
    };
    check(&configuration)?;
    Ok(configuration)
}

/// `configuration` in the file's own syntax, for the write face.
///
/// # Errors
/// `E_CONFIG_INVALID` when the value cannot be spelled as TOML.
pub(crate) fn spelled(
    configuration: &SearchConfiguration,
    file: &Path,
) -> Result<toml::Value, AxError> {
    let section = match configuration {
        SearchConfiguration::Default => SearchSection::bare(Choice::Default),
        SearchConfiguration::Off => SearchSection::bare(Choice::Off),
        SearchConfiguration::Custom {
            selected,
            suppliers,
        } => SearchSection {
            choice: Choice::Custom,
            selected: Some(selected.as_str().to_owned()),
            suppliers: Some(suppliers.iter().map(SupplierSection::of).collect()),
        },
    };
    toml::Value::try_from(section).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "write a configuration layer",
            format!("{}: {err}", file.display()),
        )
        .with_recovery("fix that file by hand, or delete it and choose again")
    })
}

fn label(raw: &str) -> Result<ServerLabel, AxError> {
    ServerLabel::parse(raw)
        .map_err(|err| refuse(format!("{SEARCH_KEY}: {raw}: {}", err.recovery())))
}

/// The `[search]` table as a file spells it; the reader and the writer
/// share it, so the file's syntax has one definition.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchSection {
    choice: Choice,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    suppliers: Option<Vec<SupplierSection>>,
}

impl SearchSection {
    fn bare(choice: Choice) -> SearchSection {
        SearchSection {
            choice,
            selected: None,
            suppliers: None,
        }
    }
}

/// The three intents, each spelled once.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Choice {
    Default,
    Custom,
    Off,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SupplierSection {
    id: String,
    url: String,
    remote: String,
    query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    objective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    count: Option<String>,
    #[serde(default)]
    accounts: Vec<AccountSection>,
}

impl SupplierSection {
    fn read(self) -> Result<SearchSupplier, AxError> {
        Ok(SearchSupplier {
            id: label(&self.id)?,
            url: self.url,
            remote: self.remote,
            query_field: self.query,
            objective_field: self.objective,
            count_field: self.count,
            accounts: self
                .accounts
                .into_iter()
                .map(AccountSection::read)
                .collect::<Result<_, _>>()?,
        })
    }

    fn of(supplier: &SearchSupplier) -> SupplierSection {
        SupplierSection {
            id: supplier.id.as_str().to_owned(),
            url: supplier.url.clone(),
            remote: supplier.remote.clone(),
            query: supplier.query_field.clone(),
            objective: supplier.objective_field.clone(),
            count: supplier.count_field.clone(),
            accounts: supplier.accounts.iter().map(AccountSection::of).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountSection {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    header: Option<String>,
}

impl AccountSection {
    fn read(self) -> Result<ProviderAccount, AxError> {
        Ok(ProviderAccount {
            id: label(&self.id)?,
            reference: self
                .reference
                .map(|raw| {
                    SecretRef::parse(&raw).map_err(|err| {
                        refuse(format!(
                            "{SEARCH_KEY}: account `{}`: {}",
                            self.id,
                            err.recovery()
                        ))
                    })
                })
                .transpose()?,
            header: self.header,
        })
    }

    fn of(account: &ProviderAccount) -> AccountSection {
        AccountSection {
            id: account.id.as_str().to_owned(),
            reference: account.reference.as_ref().map(ToString::to_string),
            header: account.header.clone(),
        }
    }
}
