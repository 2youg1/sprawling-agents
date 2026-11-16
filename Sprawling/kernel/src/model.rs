// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod generation;
mod image;
mod mode;
mod seam;
mod usage;
mod window;
mod wire;

pub use generation::{
    FrozenGeneration, GenerationSettings, GenerationSupport, ModelGeneration, ParameterSource,
    ParameterSupport,
};
pub use image::{ImageRef, ImageType};
pub use mode::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy};
pub use seam::{ModelRequest, ModelReturn, content_from_message, message_payload};
pub use usage::{CacheCount, ModelUsage};
pub use window::Window;
pub use wire::{
    BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ChatResponse, ContentBlock, DialectKind,
    Effort, Increment, MessageBreakpoint, ModelRef, ModelTag, Role, SharedPrefix, StopReason,
    SystemBlock, ToolDef,
};

use crate::error::AxError;
use crate::tool::ToolCall;

pub type Increments<'a> = &'a mut dyn FnMut(&Increment);

pub type EarlyCalls<'a> = &'a mut dyn FnMut(&ToolCall);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRoster {
    pub provider: String,

    pub accounts: Vec<RosterEntry>,

    pub current: crate::ServerLabel,

    pub retries: crate::account_recovery::AccountRetries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub account: crate::ServerLabel,
    pub usable: bool,
}

pub trait Model {

    fn provider_account(&self) -> Option<crate::event::record::ProviderAccountBinding> {
        None
    }

    fn account_roster(&self) -> Option<AccountRoster> {
        None
    }

    fn select_account(&mut self, _account: &crate::ServerLabel) {}

    fn prefix_held(&self) -> Option<crate::event::record::PrefixHeld> {
        None
    }
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        _onto: Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.call(req)
    }

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
