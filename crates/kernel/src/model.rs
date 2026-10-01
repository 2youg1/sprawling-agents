// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model seam (seam registry ARCHITECTURE 3).
//! Every provider call carries a `BuildingPolicy` value; the policy is
//! *defined* here (kernel cannot name outer crates) and *evaluated* by
//! `city::policy` — dependency inversion, same as the ledger seam.
//! The part `crates/kernel/spec/Model.lean` specifies this module.

mod image;
mod mode;
mod seam;
mod usage;
mod window;
mod wire;

pub use image::{ImageRef, ImageType};
pub use mode::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy};
pub use seam::{ModelRequest, ModelReturn, content_from_message, message_payload};
pub use usage::ModelUsage;
pub use window::Window;
pub use wire::{
    BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ChatResponse, ContentBlock, DialectKind,
    Effort, Increment, MessageBreakpoint, ModelTag, Role, StopReason, SystemBlock, ToolDef,
};

use crate::error::AxError;
use crate::tool::ToolCall;

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
/// by the client.
pub type Increments<'a> = &'a mut dyn FnMut(&Increment);

/// A tool call handed over the moment its block is complete, before the
/// call it belongs to has settled.
///
/// Separate from [`Increments`] because a caller acts on it: it may start
/// a read-only tool while the model is still generating. **What arrives
/// here is not history either.** The ledger records tool calls from
/// [`ModelReturn`] after the call settles; when the call fails, a caller
/// discards whatever it derived from these. What a caller may start, and
/// in which order it records the results, is fixed by
/// `crates/runtime/spec/Turn/Speculation.lean`.
pub type EarlyCalls<'a> = &'a mut dyn FnMut(&ToolCall);

/// The model port. Production adapter: gateway::endpoint;
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

    /// The streaming call, also handing over each tool call as soon as it
    /// is complete.
    ///
    /// The default hands nothing over and answers with
    /// [`Model::call_streaming`], which is honest for an adapter whose
    /// wire cannot say when a call is complete: the caller simply gets no
    /// head start. An adapter that overrides this owes the same
    /// `ModelReturn` and failures as `call_streaming`, and every call it
    /// hands over equals the one in that `ModelReturn`.
    ///
    /// # Errors
    /// Whatever [`Model::call_streaming`] would fail with.
    fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
        _early: EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.call_streaming(req, onto)
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
