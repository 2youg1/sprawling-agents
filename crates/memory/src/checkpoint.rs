// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint fences around a tool wave.
//!
//! Before the wave, everything inside the write domain is committed;
//! after it, every file the wave deleted is recorded as discarded with
//! that commit as its restoration address. The order is the whole
//! point — a deletion recorded against a commit that does not yet exist
//! is not restorable, so the fence goes up first and the accounting
//! follows.
//!
//! Two boundaries are hard. The scope is the write domain and nothing
//! wider: a whole-tree `add` would stage files the Run was never given,
//! which this design refuses outright. And time is a parameter,
//! never sampled — the git signature carries the injected instant, so
//! the same script replays to the same commits.
//!
//! Staged content is scanned before it can be committed. A hit refuses
//! the commit and reports positions only; echoing the matched bytes to
//! prove a secret leaked would be the leak.

mod fence;
mod scan;

pub use fence::Checkpoint;
