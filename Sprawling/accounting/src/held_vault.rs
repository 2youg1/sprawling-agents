// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One vault held behind a lock, as the parts that read it reach it
//! (`crates/accounting/spec/HeldVault.lean` §8-9).
//!
//! The assembly point, the served worker and the MCP health view each
//! need a resolver over the city's vault, and each must refuse the same
//! way when the lock is poisoned; both answers are written here once.

use std::sync::{Arc, Mutex};

use kernel::{AxCode, AxError};

/// The refusal when a thread panicked while holding the vault. What was
/// enrolled is on disk and unaffected; only this process lost the lock.
pub fn poisoned_vault() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "reach the vault",
        "the vault lock is poisoned",
    )
    .with_recovery("restart the server; enrolled credentials are unaffected")
}

/// One resolver over one vault. A fresh one per operation, because
/// `SecretResolver` is spent by the endpoint it is handed to.
pub fn resolving(vault: Arc<Mutex<gateway::Custodian>>) -> gateway::SecretResolver {
    Box::new(move |reference: &kernel::SecretRef| {
        let held = vault.lock().map_err(|_| poisoned_vault())?;
        held.resolve(reference)
    })
}
