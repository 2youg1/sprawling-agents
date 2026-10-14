// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The pre-warm one call would send for the prefix its building shares,
//! the key that prefix is cached under, and the calls that wait for
//! none (`crates/gateway/spec/Concurrency.lean` §8-40, D38, D39, D42).
//!
//! The request is the call's own, cut to its shared part: the same
//! model, tools and effort, the system blocks `ChatRequest::shared_system`
//! names with their cache marks, one placeholder message carrying no
//! breakpoint, and one output token. Everything else is the endpoint's
//! own body writer, so a pre-warm writes exactly the bytes the calls
//! behind it read.

use std::borrow::Cow;
use std::time::Duration;

use kernel::consts_policy::BYTES_PER_TOKEN;
use kernel::{
    AxError, Ceiling, ChatMessage, ChatRequest, ContentBlock, DialectKind, MessageBreakpoint,
    ModelRequest, ModelReturn, Role,
};
use serde_json::Value;

use super::config::{Endpoint, apply_override};
use crate::hold::PrefixKey;

/// The message a pre-warm carries: the smallest a provider accepts.
const PLACEHOLDER: &str = ".";

/// The one output token a pre-warm asks for (gateway D42).
const PREWARM_TOKENS: u64 = 1;

/// A pre-warm ready to send, and the key the calls that wait for it
/// share.
pub(super) struct Prewarming {
    pub(super) key: PrefixKey,
    request: ModelRequest<'static>,
    body: Value,
}

impl Endpoint {
    /// The pre-warm `req` waits behind when it goes out on `account`, or
    /// `None` when gateway D39 holds it for nothing: a face that does not
    /// charge for cache writes, a server on this machine, a confidential
    /// building, a request that shares nothing, a shared part shorter
    /// than the model caches, or a body this endpoint cannot write - the
    /// real call then meets the same refusal and reports it.
    pub(super) fn prewarm_for(
        &self,
        account: Option<&kernel::ServerLabel>,
        req: &ModelRequest,
    ) -> Option<Prewarming> {
        let caches_by_breakpoint = match self.config.dialect {
            DialectKind::Anthropic => true,
            DialectKind::OpenAi | DialectKind::OpenAiResponses => false,
        };
        if !caches_by_breakpoint
            || crate::reach::is_local(&self.config.base_url)
            || req.policy.confidential
        {
            return None;
        }
        let request = ModelRequest {
            policy: req.policy.clone(),
            segments: req.segments,
            chat: ChatRequest {
                model: req.chat.model.clone(),
                max_tokens: Ceiling::new(PREWARM_TOKENS),
                system: req.chat.shared_system()?.to_vec(),
                messages: Cow::Owned(vec![ChatMessage {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: PLACEHOLDER.to_owned(),
                    }],
                }]),
                tools: Cow::Owned(req.chat.tools.to_vec()),
                breakpoint: MessageBreakpoint::Unmarked,
                shared: req.chat.shared,
                effort: req.chat.effort,
            },
        };
        let body = self.prewarm_body(&request)?;
        let bytes = serde_json::to_vec(&body).ok()?;
        let tokens = u64::try_from(bytes.len())
            .ok()?
            .checked_div(BYTES_PER_TOKEN)?;
        if tokens
            < crate::provider::preset::cache_min_tokens_for(
                &self.config.base_url,
                &self.config.model,
            )
        {
            return None;
        }
        Some(Prewarming {
            key: PrefixKey::of(account.map_or("", kernel::ServerLabel::as_str), &bytes),
            request,
            body,
        })
    }

    /// Sends `prewarming` through the blocking door, read whole within
    /// `timeout`.
    ///
    /// # Errors
    /// Whatever [`Endpoint::post_settled`] fails with.
    pub(super) fn prewarm(
        &self,
        prewarming: &Prewarming,
        timeout: Duration,
    ) -> Result<ModelReturn, AxError> {
        self.post_settled(&prewarming.request, &prewarming.body, timeout)
    }

    /// The body this endpoint writes for `request`, with the output
    /// ceiling written after the person's overrides, because a pre-warm
    /// that writes text only costs more and buys nothing (gateway D42).
    /// `None` when the body cannot be written: the real call is refused
    /// for the same reason and says why.
    fn prewarm_body(&self, request: &ModelRequest) -> Option<Value> {
        let mut body = self.wire_request(request).ok()?;
        apply_override(&mut body, "/max_tokens", &Value::from(PREWARM_TOKENS)).ok()?;
        Some(body)
    }
}
