// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::DialectKind;

use super::{
    CeilingField, ChatSpelling, EffortField, Face, GenerationPreset, HostPreset, ReasoningReturn,
    ToolPictureCarry,
};

mod models;

use models::*;

pub const PRESETS: [HostPreset; 13] = [
    HostPreset {
        host: "api.anthropic.com",
        faces: &[Face {
            dialect: DialectKind::Anthropic,
            path: "/v1",
        }],
        models: ANTHROPIC_MODELS,
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset {
            anthropic: super::ANTHROPIC_GENERATION,
            ..GenerationPreset::UNKNOWN
        },
        source: "https://platform.claude.com/docs/en/api/messages",
    },
    HostPreset {
        host: "api.openai.com",

        faces: &[
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/v1",
            },
            Face {
                dialect: DialectKind::OpenAi,
                path: "/v1",
            },
        ],
        models: OPENAI_MODELS,

        chat: ChatSpelling {
            ceiling: CeilingField::MaxCompletionTokens,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,

        cache_key_field: Some("prompt_cache_key"),
        generation: GenerationPreset {
            open_ai: super::OPENAI_GENERATION,
            responses: super::OPENAI_GENERATION,
            ..GenerationPreset::UNKNOWN
        },
        source: "https://github.com/openai/openai-openapi/blob/master/openapi.yaml",
    },
    HostPreset {
        host: "api.deepseek.com",

        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/",
            },
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/anthropic/v1",
            },
        ],
        models: DEEPSEEK_MODELS,

        chat: ChatSpelling {
            effort: EffortField::ThinkingToggle,
            reasoning: ReasoningReturn::AsReasoningContent,
            chat_tool_pictures: ToolPictureCarry::FollowingUser,
            responses_tool_pictures: ToolPictureCarry::InToolContent,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://api-docs.deepseek.com/guides/thinking_mode",
    },
    HostPreset {
        host: "api.x.ai",

        faces: &[
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/v1",
            },
            Face {
                dialect: DialectKind::OpenAi,
                path: "/v1",
            },
        ],
        models: &[],

        chat: ChatSpelling {
            ceiling: CeilingField::MaxCompletionTokens,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://docs.x.ai/developers/rest-api-reference/inference/chat-completions",
    },
    HostPreset {
        host: "openrouter.ai",

        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/v1",
        }],

        models: &[],

        chat: ChatSpelling {
            ceiling: CeilingField::MaxTokens,
            effort: EffortField::ReasoningObject,
            reasoning: ReasoningReturn::AsReasoningContent,
            chat_tool_pictures: ToolPictureCarry::FollowingUser,
            responses_tool_pictures: ToolPictureCarry::FollowingUser,
        },
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://openrouter.ai/docs/guides/best-practices/reasoning-tokens",
    },
    HostPreset {
        host: "generativelanguage.googleapis.com",

        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1beta/openai",
        }],
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://ai.google.dev/gemini-api/docs/openai",
    },
    HostPreset {
        host: "open.bigmodel.cn",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/paas/v4",
        }],
        models: &[],
        chat: ZHIPU_CHAT,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://docs.bigmodel.cn/api-reference/%E6%A8%A1%E5%9E%8B-api/%E5%AF%B9%E8%AF%9D%E8%A1%A5%E5%85%A8",
    },
    HostPreset {
        host: "api.z.ai",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/api/paas/v4",
        }],
        models: &[],
        chat: ZHIPU_CHAT,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://docs.z.ai/api-reference/llm/chat-completion",
    },
    HostPreset {
        host: "opencode.ai",

        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/zen/v1",
            },
            Face {
                dialect: DialectKind::OpenAiResponses,
                path: "/zen/v1",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/zen/v1",
            },
        ],
        models: &[],
        chat: ChatSpelling::DOCUMENTED,
        session_header: Some("x-opencode-session"),
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://opencode.ai/docs/go/",
    },
    HostPreset {
        host: "api.kimi.com",

        faces: &[
            Face {
                dialect: DialectKind::OpenAi,
                path: "/coding/v1",
            },
            Face {
                dialect: DialectKind::Anthropic,
                path: "/coding/v1",
            },
        ],

        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "api.moonshot.cn",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1",
        }],

        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "api.moonshot.ai",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/v1",
        }],
        models: &[],
        chat: MOONSHOT_CHAT,
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py",
    },
    HostPreset {
        host: "dashscope-us.aliyuncs.com",
        faces: &[Face {
            dialect: DialectKind::OpenAi,
            path: "/compatible-mode/v1",
        }],
        models: &[],

        chat: ChatSpelling {
            effort: EffortField::EnableThinking,
            ..ChatSpelling::DOCUMENTED
        },
        session_header: None,
        cache_key_field: None,
        generation: GenerationPreset::UNKNOWN,
        source: "https://help.aliyun.com/zh/model-studio/compatibility-of-openai-with-dashscope",
    },
];
