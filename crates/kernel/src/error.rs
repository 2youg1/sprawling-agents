// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! AxError, the one error shape of the whole city, and
//! AxCode, the closed set of 35 error codes.
//!
//! Invariants owned here:
//! - every wire spelling (`E_…`) has exactly one authority: [`AxCode::as_str`];
//!   serde delegates to it in both directions, so enum and wire cannot drift.
//! - a gate refusal always carries the three mandatory parts (rule,
//!   violation, alternative): [`AxError::refusal`] is the only path that
//!   sets them and [`AxError::failure`] cannot.
//! - `retriable` defaults to false; a caller must opt in explicitly
//!   (fail-closed).
//!
//! The carrier-event declaration (`AxCode::carrier`) lives together with
//! `kernel::event` because it names `EventKind`.

mod code;
mod refusal;
mod shape;

pub use code::{AxCode, Carrier};
pub use refusal::GateRefusal;
pub use shape::AxError;
