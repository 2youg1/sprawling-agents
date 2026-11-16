// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
mod prewarm;
pub(crate) mod redemption;
mod stream;
mod transport;

pub use adapter::adapter_for;

#[cfg(test)]
pub(crate) use call::conversation_id;
pub use config::{AuthSpec, Endpoint, EndpointConfig};
pub use header::HeaderValue;
pub use kernel::event::record::ModelFacts;
pub use redemption::{Redemption, SecretResolver};
pub use transport::WarmUp;
pub(crate) use transport::{ClientShape, Transport};
