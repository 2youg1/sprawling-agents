// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One open answers what the safe path spends four opens and two
//! canonicalisations on (`crates/read_ffi/Spec.lean`).
//!
//! The leaf decides no policy: it answers facts — where the opened file
//! really lives, how big it is, when it last changed, and the bytes of
//! one window. Whether that place may be read, and what address it names
//! in the city, stays in `runtime::tools::chosen_path`.

mod window;

pub use window::{Ask, Refused, Window, read};

#[cfg(test)]
mod tests;
