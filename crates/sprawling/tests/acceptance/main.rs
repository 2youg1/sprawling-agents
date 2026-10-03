// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Acceptance coverage (`crates/accounting/Spec.lean` section 2, the seventh to
//! the ninth assertion).
//!
//! Each test drives a worker that was handed a scripted `ModelFactory`.
//! The script speaks for the model and nothing else: the workbench, the
//! effect layer, the checkpoints, the ledger and the shelves are the
//! production ones. What ought to be covered is read from what the
//! model was offered, never from a list kept here, so a tool the
//! workbench registers without an episode in `episodes.rs` is named by
//! the failure.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

mod catalogue;
mod city;
mod crash;
mod episodes;
mod held_reply;
mod ocr;
mod playback;
mod script;
mod shelf_outside;
mod skills;
