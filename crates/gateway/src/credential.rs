// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Credential custody. Plaintext exists in two places
//! only: the vault, and the last register before the wire. This module
//! owns the effect half of Custody — capture (replace plaintext with
//! `secret:` literals), redemption (resolve per operation, never cached),
//! describe (state without value), and the startup persistence probe that
//! never silently falls back.
//!
//! The inner seam `Vault` stays `pub(crate)`: two sentences of interface,
//! backends and their politics hidden. We never write our own encrypted
//! files — the platform credential service or session memory, nothing
//! between.

mod custodian;
mod oauth;
mod vault;

pub use custodian::{Captured, Custodian};
pub use oauth::{OauthPending, OauthTokens, TokenRequest, oauth_begin};
pub use oauth::{oauth_random, oauth_redeem, oauth_redeem_request, oauth_refresh};
pub use vault::{Described, EnvReader, Persistence};
