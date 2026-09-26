// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model a run or a naming call talks to, handed to the worker
//! rather than built by it (accounting-SPEC.md 8-1).

use kernel::AxError;

/// Builds the adapter one chosen model is reached through.
///
/// The worker has already chosen the model and redeemed the credential
/// by the time it asks, so an implementation decides only how the
/// adapter is made - never which model, and never when a credential is
/// renewed.
pub trait ModelFactory {
    /// # Errors
    /// Whatever building the adapter refuses: a malformed endpoint, or a
    /// confidential building's bytes asked to leave this machine.
    fn build(
        &self,
        chosen: &gateway::Chosen<'_>,
        redemption: gateway::Redemption,
    ) -> Result<Box<dyn kernel::Model + Send>, AxError>;
}
