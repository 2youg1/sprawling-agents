// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use kernel::config::{SearchConfiguration, SearchSupplier};
use kernel::event::record::{ProviderAccount, validate_provider_accounts};
use kernel::layout::CityLayout;
use kernel::{Address, AxError, ServerLabel};

use super::ConfigLayer;
use super::ladder::{Ladder, Layer};
use super::refuse::{refuse, refuse_with};
use super::write::{Change, change_at};

mod section;
pub(super) use section::{SearchSection, spelled, stated};

pub(crate) const SEARCH_KEY: &str = "[search]";

pub fn search_supplier(
    configuration: &SearchConfiguration,
) -> Result<Option<SearchSupplier>, AxError> {
    match configuration {
        SearchConfiguration::Default => default_search_supplier().map(Some),
        SearchConfiguration::Custom {
            selected,
            suppliers,
        } => suppliers
            .iter()
            .find(|supplier| supplier.id == *selected)
            .cloned()
            .map(Some)
            .ok_or_else(|| unlisted(selected)),
        SearchConfiguration::Off => Ok(None),
    }
}

pub fn default_search_supplier() -> Result<SearchSupplier, AxError> {
    Ok(SearchSupplier {
        id: ServerLabel::parse("exa")?,
        url: "https://mcp.exa.ai/mcp".to_owned(),
        remote: "web_search_exa".to_owned(),
        query_field: "query".to_owned(),
        objective_field: Some("objective".to_owned()),
        count_field: Some("numResults".to_owned()),
        accounts: vec![ProviderAccount {
            id: ServerLabel::parse("anonymous")?,
            reference: None,
            header: None,
        }],
    })
}

pub fn settled_search(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(SearchConfiguration, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(|layer| layer.search().cloned())
        .resolve()
        .cloned())
}

pub fn city_search(city_root: &Path) -> Result<Option<SearchConfiguration>, AxError> {
    Ok(
        super::ladder::stated(&CityLayout::new(city_root).city_config(), Layer::City)?
            .search()
            .cloned(),
    )
}

pub fn write_search(city_root: &Path, configuration: &SearchConfiguration) -> Result<(), AxError> {
    check(configuration)?;
    change_at(
        &CityLayout::new(city_root).city_config(),
        Change::Search(configuration),
    )
}

fn check(configuration: &SearchConfiguration) -> Result<(), AxError> {
    let SearchConfiguration::Custom {
        selected,
        suppliers,
    } = configuration
    else {
        return Ok(());
    };
    if suppliers.is_empty() {
        return Err(refuse(format!("{SEARCH_KEY}: `custom` lists no supplier")));
    }
    let mut ids = BTreeSet::new();
    for listed in suppliers {
        if !ids.insert(listed.id.as_str()) {
            return Err(refuse(format!(
                "{SEARCH_KEY}: two suppliers are named `{}`",
                listed.id.as_str()
            )));
        }
        supplier(listed)?;
    }
    if !ids.contains(selected.as_str()) {
        return Err(unlisted(selected));
    }
    Ok(())
}

fn supplier(supplier: &SearchSupplier) -> Result<(), AxError> {
    let id = supplier.id.as_str();
    super::mcp::check_url(&supplier.id, &supplier.url)?;
    let fields = [
        Some(("remote", &supplier.remote)),
        Some(("query", &supplier.query_field)),
        supplier
            .objective_field
            .as_ref()
            .map(|field| ("objective", field)),
        supplier.count_field.as_ref().map(|field| ("count", field)),
    ];
    let mut named = BTreeSet::new();
    for (key, value) in fields.into_iter().flatten() {
        if value.trim().is_empty() {
            return Err(refuse(format!(
                "{SEARCH_KEY} supplier `{id}`: `{key}` is empty"
            )));
        }
        if key != "remote" && !named.insert(value.as_str()) {
            return Err(refuse(format!(
                "{SEARCH_KEY} supplier `{id}`: `{value}` is mapped twice"
            )));
        }
    }
    validate_provider_accounts(Some(&supplier.accounts)).map_err(|err| {
        refuse_with(
            format!("{SEARCH_KEY} supplier `{id}`: {}", err.subject()),
            err.recovery(),
        )
    })?;

    match supplier
        .accounts
        .iter()
        .find(|account| account.reference.is_some() && account.header.is_none())
    {
        Some(account) => Err(refuse_with(
            format!(
                "{SEARCH_KEY} supplier `{id}`: account `{}` has a key and no header",
                account.id.as_str()
            ),
            "name the header the service reads its key from, such as `x-api-key`",
        )),
        None => Ok(()),
    }
}

fn unlisted(selected: &ServerLabel) -> AxError {
    refuse(format!(
        "{SEARCH_KEY}: `selected = \"{}\"` names no listed supplier",
        selected.as_str()
    ))
}

impl ConfigLayer {

    #[must_use]
    pub fn search(&self) -> Option<&SearchConfiguration> {
        self.search.as_ref()
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
