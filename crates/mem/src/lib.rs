// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thin FFI adapter over the Zig performance kernel in `zig/`. The
//! kernel holds byte-level leaves only: `(ptr, len)` in, one flat struct
//! and one integer code out. Domain rules, canonical bytes and every
//! error sentence stay in Rust (mem-SPEC.md section 1).

mod envelope;

pub use envelope::{EnvelopeKey, EnvelopeSpans, scan};
