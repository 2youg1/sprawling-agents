// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rows themselves: which host serves what, and where each
//! statement was read.
//!
//! Split from the lookups beside them because a row is data a person
//! checks against a vendor's page and a lookup is code a person reads
//! for its rule, and one file holding both is read twice for two
//! reasons.

use kernel::DialectKind;

use crate::market::InputKinds;

use super::{HostPreset, ModelPreset};

/// The hosts this city knows without asking.
///
/// Short on purpose. A host belongs here when this city can cite what
/// it serves; everything else reaches the same facts through the
/// endpoint's own model list, which is the rung above this table.
pub const PRESETS: [HostPreset; 7] = [
    HostPreset {
        host: "api.anthropic.com",
        base_path: "/v1",
        dialect: Some(DialectKind::Anthropic),
        models: ANTHROPIC_MODELS,
        source: "https://platform.claude.com/docs/en/api/messages",
    },
    HostPreset {
        host: "api.openai.com",
        base_path: "/v1",
        // Chat completions and responses are both served here, and the
        // person's own toggle says which one this registration means.
        dialect: None,
        models: OPENAI_MODELS,
        source: "https://platform.openai.com/docs/api-reference/chat",
    },
    HostPreset {
        host: "openrouter.ai",
        // Not `/v1`: this aggregator serves under `/api/v1`, and the
        // rule that appends `/v1` to every path-less URL answers 404
        // here.
        base_path: "/api/v1",
        dialect: Some(DialectKind::OpenAi),
        // This gateway states window, ceiling, modalities and prices in
        // its own model list, so a row here would be a second home for
        // a fact the endpoint already publishes.
        models: &[],
        source: "https://openrouter.ai/docs/api-reference/overview",
    },
    HostPreset {
        host: "generativelanguage.googleapis.com",
        base_path: "/v1beta",
        // The OpenAI-compatible face hangs below this prefix; which
        // face a registration means is the person's to say.
        dialect: None,
        models: &[],
        source: "https://ai.google.dev/api",
    },
    HostPreset {
        // The Kimi Code subscription, whose API does not hang under
        // `/v1` at all. Appending `/v1` to this host answers 404,
        // which is the same defect `openrouter.ai` is listed for.
        host: "api.kimi.com",
        base_path: "/coding/v1",
        // Called through the OpenAI client library with this base URL
        // (`packages/kosong/src/kosong/chat_provider/openai_common.py`
        // constructs `AsyncOpenAI(base_url=...)`), so the chat face is
        // the OpenAI-compatible one.
        dialect: Some(DialectKind::OpenAi),
        // Pending: the ceilings of the Kimi models are stated by this
        // endpoint's own model list, which needs a subscription token
        // to read, and no vendor page reachable from this machine
        // states them.
        models: &[],
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "api.moonshot.cn",
        base_path: "/v1",
        dialect: Some(DialectKind::OpenAi),
        // Pending: as above, the model list is the statement and it
        // needs a key.
        models: &[],
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        // The same platform served outside mainland China; one row per
        // host because a host is what a person pastes.
        host: "api.moonshot.ai",
        base_path: "/v1",
        dialect: Some(DialectKind::OpenAi),
        models: &[],
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
];

/// Anthropic's documented ceilings. The Messages API requires
/// `max_tokens` in every request, so a missing row here is a call that
/// cannot be written at all.
const ANTHROPIC_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "claude-opus-4",
        context_tokens: 200_000,
        max_output_tokens: 32_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-sonnet-4",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-7-sonnet",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-5-haiku",
        context_tokens: 200_000,
        max_output_tokens: 8_192,
        input: InputKinds::TextImage,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
];

/// OpenAI's documented ceilings.
const OPENAI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "gpt-4.1",
        context_tokens: 1_047_576,
        max_output_tokens: 32_768,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/gpt-4.1",
    },
    ModelPreset {
        id_prefix: "gpt-4o",
        context_tokens: 128_000,
        max_output_tokens: 16_384,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/gpt-4o",
    },
    ModelPreset {
        id_prefix: "o3",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/o3",
    },
    ModelPreset {
        id_prefix: "o4-mini",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        source: "https://platform.openai.com/docs/models/o4-mini",
    },
];
