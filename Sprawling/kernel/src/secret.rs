// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod hex_run;
mod scan;
mod sealed;
mod span;

pub use scan::{EntropyAllowlist, names_a_credential, scan};
pub use sealed::Sealed;
pub use span::{SecretRef, SecretSpan};
