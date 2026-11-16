// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod attached;
mod book;
mod normalise;
mod payload;
mod tuning;

pub use attached::AttachedEndpoint;
pub(crate) use attached::join;
pub use book::{Chosen, EndpointBook};
pub(crate) use normalise::HostDefaults;
pub use normalise::{DialectHint, normalise_entered};
pub use payload::{attached_payload, selected_payload};
pub use tuning::{EndpointTuning, TuningDefaults, validate_accounts};
