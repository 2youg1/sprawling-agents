// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The desktop server's one FFI seam: the Win32 call groups that have no
//! admitted safe interface, carried out by a Zig leaf behind a
//! `(ptr, len)` boundary and offered to the server as safe functions
//! (`crates/desktop/Spec.lean` sections 8-12 and D12; the boundary's properties
//! are proved in `crates/desktop/ffi/Spec.lean`).
//!
//! Every `unsafe` in the desktop server's production code is one call
//! into the leaf, here, with the precondition that makes it sound
//! written beside it. What the server decides — which window, which
//! refusal, what a black picture means — stays on the Rust side; the
//! leaf owns no wording and no policy.

pub mod cpu_set;
pub mod step;

#[cfg(windows)]
pub mod boundary;
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod cpu;
#[cfg(windows)]
pub mod dpi;
#[cfg(windows)]
pub mod ended;
#[cfg(all(windows, feature = "fixture"))]
pub mod fixture;
#[cfg(windows)]
mod leaf;
#[cfg(all(windows, test))]
mod reference;
#[cfg(windows)]
pub mod top_level;
