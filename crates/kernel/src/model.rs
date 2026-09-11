// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model seam (seam registry ARCHITECTURE 3).
//! Every provider call carries a `BuildingPolicy` value; the policy is
//! *defined* here (kernel cannot name outer crates) and *evaluated* by
//! `city::policy` (P1) — dependency inversion, same as the ledger seam.

mod image;
mod seam;
mod wire;

pub use image::{ImageRef, ImageType};
pub use seam::{ModelRequest, ModelReturn, content_from_message, message_payload, value_has_float};
pub use wire::{
    BuildingPolicy, ChatMessage, ChatRequest, ChatResponse, ContentBlock, DialectKind, Effort,
    ModelTag, ModelUsage, Role, StopReason, SystemBlock, ToolDef,
};

use crate::error::AxError;

/// Text arriving from a provider before the call has finished.
///
/// One argument and no return value, because an increment is not a
/// decision: nothing downstream may branch on it, and a sink that could
/// refuse would make a display detail able to fail a call.
///
/// **What arrives here is not history.** The record of what the model
/// said is written once, from [`ModelReturn`], after the call settles.
/// Increments are a thing to look at while waiting, and the two can
/// disagree — a provider may revise, and a cut stream leaves increments
/// behind that no `ModelReturn` ever confirms. Where they disagree the
/// settled text wins, and that rule is held on the far side of the wire
/// by `web::app`.
pub type Increments<'a> = &'a mut dyn FnMut(&str);

/// The model port. Production adapters: gateway::native, gateway::endpoint;
/// second adapter: citysim scripted model. Implementations never sample
/// clocks or read global state.
pub trait Model {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;

    /// The same call, reporting text as it arrives.
    ///
    /// The default answers by calling [`Model::call`] and reporting
    /// nothing, which is the honest behaviour for an adapter that has no
    /// stream: a caller gets the same `ModelReturn` at the same moment
    /// and simply sees no increments. An adapter that overrides this owes
    /// the same `ModelReturn` it would have returned from `call`,
    /// including the same failures — a stream cut halfway is a read
    /// error, never a shortened answer.
    ///
    /// # Errors
    /// Whatever [`Model::call`] would fail with.
    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        _onto: Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.call(req)
    }
}

#[cfg(feature = "conformance")]
pub mod conformance;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
