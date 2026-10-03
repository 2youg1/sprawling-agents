// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external-provider adapter: a self-written wire-format client over
//! a plain HTTP transport. One call, one HTTP
//! round trip: whether to try again is the watchdog's decision, never a
//! hidden loop here.
//!
//! Credentials resolve at the last moment: the `Sealed` value is exposed
//! only while the auth header is written, then dropped (zeroized).

mod adapter;
mod auth;
mod call;
pub(crate) mod config;
pub(crate) mod failure;
#[cfg(test)]
pub(crate) mod fakes;
mod header;
mod model;
mod models;
mod permit;
pub(crate) mod redemption;
mod stream;
mod transport;

pub use adapter::adapter_for;
/// The stand-in rounds of `reach::resolve` compare the prompt cache key
/// a body carried with the conversation id of the request they sent.
#[cfg(test)]
pub(crate) use call::conversation_id;
pub use config::{AuthSpec, Endpoint, EndpointConfig};
pub use header::HeaderValue;
pub use kernel::event::record::ModelFacts;
pub use redemption::{Redemption, SecretResolver};
pub(crate) use transport::{ClientShape, Transport};
pub use transport::WarmUp;
