// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[search]` table: which outside service a run's `web_search`
//! reaches, read from the file, judged, written back, and resolved to
//! one supplier.
//!
//! The default supplier is declared here and nowhere else: the settings
//! page reads its address back through a query, and `kernel` holds only
//! the shape (city D24).
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4c.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::config::{SearchConfiguration, SearchSupplier};
use kernel::event::record::{ProviderAccount, validate_provider_accounts};
use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, ServerLabel};

use super::ConfigLayer;
use super::ladder::{Ladder, Layer};
use super::refuse::refuse;
use super::write::{Change, change_at};

mod section;
pub(super) use section::{SearchSection, spelled, stated};

/// The table's header, as a refusal names it.
pub(crate) const SEARCH_KEY: &str = "[search]";

/// The one supplier `web_search` reaches under `configuration`, or
/// `None` when this layer offers no web search.
///
/// `Custom` reaches exactly the supplier its `selected` names and never
/// falls back to the default or to another listed supplier: switching
/// would hand the run's queries to a receiver nobody chose (city D24).
///
/// # Errors
/// `E_CONFIG_INVALID` when `selected` names no listed supplier, which
/// only a value built around the reader can carry.
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

/// The supplier `Default` reaches, declared once.
///
/// The parameter names are the ones the service's own `tools/list`
/// reports for `web_search_exa`: `query` and `objective` required,
/// `numResults` optional (city D24).
///
/// # Errors
/// Propagates `kernel`'s refusal of a label, which a fixed label that
/// stays in its grammar never meets.
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

/// What the ladder settled for `[search]` at `addr`, and the rung that
/// said it. `None` is a ladder that states nothing, which resolves to
/// `Default`.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse, exactly as [`super::load`] does.
pub fn settled_search(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(SearchConfiguration, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(|layer| layer.search().cloned())
        .resolve()
        .cloned())
}

/// Writes the city's own `[search]`: the one rung the settings page
/// edits. `Default` is written out too, so what is read back is what
/// was written.
///
/// # Errors
/// `E_CONFIG_INVALID`, before any byte is written, for every value the
/// reader would refuse; propagates a city layer that exists and cannot
/// be read or parsed, and a directory that cannot be written.
pub fn write_search(city_root: &Path, configuration: &SearchConfiguration) -> Result<(), AxError> {
    check(configuration)?;
    change_at(
        &CityLayout::new(city_root).city_config(),
        Change::Search(configuration),
    )
}

/// Judges a whole configuration, for the reader and the writer alike.
///
/// # Errors
/// `E_CONFIG_INVALID` for a `Custom` value whose list is empty, repeats
/// an id or does not hold `selected`, and for any supplier [`supplier`]
/// refuses.
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

/// Judges one supplier: its address by the `[[mcp]]` url judgement, its
/// three parameter names, and its accounts by the provider account rules.
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
        AxError::failure(
            AxCode::ConfigInvalid,
            "read a configuration layer",
            format!("{SEARCH_KEY} supplier `{id}`: {}", err.subject()),
        )
        .with_recovery(err.recovery())
    })?;
    // A header value is redeemed whole, so a key reaches the service only
    // as the value of a header the account names.
    match supplier
        .accounts
        .iter()
        .find(|account| account.reference.is_some() && account.header.is_none())
    {
        Some(account) => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read a configuration layer",
            format!(
                "{SEARCH_KEY} supplier `{id}`: account `{}` has a key and no header",
                account.id.as_str()
            ),
        )
        .with_recovery("name the header the service reads its key from, such as `x-api-key`")),
        None => Ok(()),
    }
}

fn unlisted(selected: &ServerLabel) -> AxError {
    refuse(format!(
        "{SEARCH_KEY}: `selected = \"{}\"` names no listed supplier",
        selected.as_str()
    ))
}

/// `ConfigLayer`'s reading of its own `[search]`.
impl ConfigLayer {
    /// What this layer says about web search, as written.
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
