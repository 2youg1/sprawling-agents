// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod custodian;
mod identity;
mod vault;

pub use custodian::{Custodian, Custody, Store};
pub use identity::{bind_platform_identity, verify_identity_binding, verify_platform_identity};
pub use vault::Persistence;
