// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three facts both sides of connecting an outside application need
//! (sprawling-SPEC.md section 8-92).
//!
//! A page reads the shelf and a command opens a consent session. They
//! would otherwise each decide where the project key is enrolled, which
//! proxy rule reaches the broker, and who this city is to it - three
//! rules with two definitions each, and a page that disagreed with the
//! command behind its own button.

use std::sync::{Arc, Mutex};

use kernel::{Address, AxCode, AxError, Proxying, SecretRef};

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
