// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod code;
mod provider;
mod refusal;
mod shape;

pub use code::{AxCode, Carrier};
pub use provider::{AccountDisposition, ProviderFailureKind};
pub use refusal::GateRefusal;
pub use shape::{AxError, ErrorDraft, Retry};
