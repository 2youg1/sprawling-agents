// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! EventRecord and the closed EventKind set: the value
//! layer of "the Ledger is the only history".
//!
//! Invariants owned here:
//! - every kind belongs to exactly one window class; the single criterion
//!   is "does its payload decide model-request bytes". The authority is
//!   [`EventKind::window_class`] — an exhaustive match, no catch-all, so a
//!   new variant forces an explicit classification.
//! - canonical bytes have one producer: [`EventRecord::canonical_line`].
//!   Struct key order = declaration order; payload keys sort (BTreeMap);
//!   `addr: None` and `ig: false` are omitted; no line terminator. The
//!   chain hashes exactly these bytes (ledger module).
//! - Ledger payloads never carry floats (determinism rule 6): [`Payload`]
//!   rejects them at construction and again on deserialize.
//! - [`EventRef`] has private fields and no public constructor; minting
//!   requires holding a whole record (append path or replay after chain
//!   verification, 15.3-1).
//! - kernel neither samples clocks nor generates ids: [`TimeMs`] and
//!   [`RunId`] arrive as parameters. uuid v7 is human-readable identity
//!   only; [`RunId::CITY`] (nil) marks city-level records.

mod identity;
mod kind;
mod payload;

pub use identity::{RunId, Seq, TimeMs};
pub use kind::{EventKind, WindowClass};
pub use payload::{EventDraft, EventRecord, EventRef, Payload};
