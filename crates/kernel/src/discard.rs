// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Discard: deletion as an effect class, judged by effect and never by
//! intent. C14 in the type: a Discard without a
//! restoration plan cannot be constructed; the unplanned path exists only
//! as a request shape for exec forecasts — and the door denies it.
//! This module is the fifth door's sole authority; gate::discard
//! delegates wholly and only shapes the refusal.

mod forecast;
mod request;
mod verdict;

pub use forecast::{DiscardForecast, forecast};
pub use request::{DenyReason, Discard, DiscardRequest, EscalateReason, Restoration};
pub use verdict::{DiscardVerdict, decide};
