// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{Address, AxCode, AxError, Proxying, SecretRef};

const KEY_REFERENCE: &str = "secret:mcp/composio";

pub fn broker_for(
    vault: Option<&Arc<Mutex<gateway::Custodian>>>,
    city: Option<&Address>,
) -> Result<Option<(agent_protocols::Broker, String)>, AxError> {
    let Some(key) = enrolled_key(vault)? else {
        return Ok(None);
    };

    let broker = agent_protocols::Broker::new(key, Proxying::ExceptLocal)?;
    Ok(Some((broker, broker_user(city))))
}

fn broker_user(city: Option<&Address>) -> String {
    city.map_or_else(|| "sprawling".to_owned(), |addr| addr.as_str().to_owned())
}

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
