// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external-provider adapter: a self-written wire-format client over
//! a plain HTTP transport. One call, one HTTP
//! round trip: retries and failover are the watchdog's and admission's
//! decisions, never a hidden loop here.
//!
//! Credentials resolve at the last moment: the `Sealed` value is exposed
//! only while the auth header is written, then dropped (zeroized).

mod adapter;
mod auth;
mod call;
pub(crate) mod config;
#[cfg(test)]
pub(crate) mod fakes;
mod model;
pub(crate) mod redemption;
mod stream;

pub use adapter::{CALL_TIMEOUT_MS, adapter_for};
pub use config::{AuthSpec, Endpoint, EndpointConfig};
pub use redemption::{ImageResolver, Redemption, SecretResolver};
