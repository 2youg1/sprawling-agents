// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Connecting an outside application, and the three facts both sides of
//! that need.
//!
//! A page reads the shelf and a command opens a consent session. They
//! would otherwise each decide where the project key is enrolled, which
//! proxy rule reaches the broker, and who this city is to it - three
//! rules with two definitions each, and a page that disagreed with the
//! command behind its own button.

use std::sync::{Arc, Mutex};

use channels::ToolkitSlug;
use kernel::{Address, AxCode, AxError, EventKind, Payload, Proxying, SecretRef};

use super::RunWorker;
use super::genesis::city_address;

/// Where the broker's project key is enrolled.
///
/// One spelling, shared with the page that enrols it. A second would be
/// a key stored under a name nothing reads.
const KEY_REFERENCE: &str = "secret:mcp/composio";

/// A broker bound to this city's key, and the name this city connects
/// under.
///
/// `Ok(None)` means nobody has enrolled a key yet, which is the
/// ordinary state of a city nobody has configured and not a failure.
/// `Err` means the vault itself could not answer, which is.
pub(crate) fn broker_for(
    vault: Option<&Arc<Mutex<gateway::Custodian>>>,
    city: Option<&Address>,
) -> Result<Option<(gateway::Broker, String)>, AxError> {
    let Some(key) = enrolled_key(vault)? else {
        return Ok(None);
    };
    // The machine's proxy rule, unmodified. The broker is not an
    // endpoint and carries no tuning of its own, and `ExceptLocal` is
    // what somebody who has not thought about proxies wants.
    let broker = gateway::Broker::new(key, Proxying::ExceptLocal)?;
    Ok(Some((broker, broker_user(city))))
}

/// Who this city is to the broker.
///
/// The city's own name, fixed by its first record and changed by
/// nothing. **Never typed by a person**: an id somebody has to invent
/// and remember is a step in a path that is meant to have none, and one
/// they would spell differently on the second city. A city with no name
/// yet answers under a constant, which connects nothing until the city
/// exists.
fn broker_user(city: Option<&Address>) -> String {
    city.map_or_else(|| "sprawling".to_owned(), |addr| addr.as_str().to_owned())
}

/// The enrolled project key, absent when nobody has enrolled one.
///
/// The two absences are told apart deliberately: a vault this city
/// cannot open is something a person must act on, while a vault that
/// does not hold this particular key is a city nobody has given one to.
fn enrolled_key(
    vault: Option<&Arc<Mutex<gateway::Custodian>>>,
) -> Result<Option<kernel::Sealed<String>>, AxError> {
    let reference = SecretRef::parse(KEY_REFERENCE)?;
    let Some(vault) = vault else {
        return Ok(None);
    };
    let held = vault.lock().map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the broker's project key",
            "the vault lock is poisoned",
        )
        .with_recovery("restart the server; the vault reopens with it")
    })?;
    match held.resolve(&reference) {
        Ok(key) => Ok(Some(key)),
        Err(refusal) if *refusal.code() == AxCode::CredentialMissing => Ok(None),
        Err(refusal) => Err(refusal),
    }
}

impl RunWorker {
    /// Opens a consent session for one outside application.
    ///
    /// **What is recorded is the request, never the standing and never
    /// the consent url.** Where an application stands is a fact about
    /// now and belongs to the broker (channels-SPEC.md section 8-31); a
    /// consent url is a capability, and anybody replaying this log would
    /// be holding one. The page reads both back from the broker with
    /// `Query::Toolkits` immediately afterwards.
    ///
    /// # Errors
    /// A city with no project key enrolled, a vault that cannot answer,
    /// a broker that refuses, and a payload the ledger will not take.
    pub(in crate::assembly) fn connect_toolkit(
        &mut self,
        toolkit: &ToolkitSlug,
    ) -> Result<(), AxError> {
        let city = city_address(&self.city_root);
        let held = broker_for(Some(&self.vault), city.as_ref())?.ok_or_else(|| {
            AxError::failure(
                AxCode::CredentialMissing,
                "connect an outside application",
                "this city holds no project key for the broker",
            )
            .with_recovery("store the broker's project key on the MCP page, then try again")
        })?;
        let (broker, user) = held;
        broker.connect(toolkit.as_str(), &user)?;
        self.record(EventKind::ToolkitLinkOpened, link_payload(toolkit)?)
    }
}

/// What the ledger is told about one request to connect.
///
/// The slug and nothing else. Which application a person asked for is
/// the fact worth keeping; everything else about that moment is either
/// the broker's to answer now or a capability that must not be kept.
fn link_payload(toolkit: &ToolkitSlug) -> Result<Payload, AxError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "toolkit".to_owned(),
        serde_json::Value::String(toolkit.as_str().to_owned()),
    );
    Payload::new(map)
}
