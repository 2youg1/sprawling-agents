// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{AxCode, AxError};

pub fn poisoned_vault() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "reach the vault",
        "the vault lock is poisoned",
    )
    .with_recovery("restart the server; enrolled credentials are unaffected")
}

pub fn resolving(vault: Arc<Mutex<gateway::Custodian>>) -> gateway::SecretResolver {
    Box::new(move |reference: &kernel::SecretRef| {
        let held = vault.lock().map_err(|_| poisoned_vault())?;
        held.resolve(reference)
    })
}
