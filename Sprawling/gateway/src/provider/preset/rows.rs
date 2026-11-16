// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::DialectKind;

use crate::market::InputKinds;

use kernel::Effort;

use super::{
    CeilingField, ChatSpelling, EffortField, Face, GenerationPreset, HostPreset, ModelPreset,
    PresetThinking, ReasoningReturn, ToolPictureCarry,
};

const LOW_TO_MAX: [Effort; 5] = [
    Effort::Low,
    Effort::Medium,
    Effort::High,
    Effort::XHigh,
    Effort::Max,
];

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

const MOONSHOT_CHAT: ChatSpelling = ChatSpelling {
    ceiling: CeilingField::MaxCompletionTokens,
    effort: EffortField::ThinkingToggle,
    reasoning: ReasoningReturn::AsReasoningContent,
    chat_tool_pictures: ToolPictureCarry::InToolContent,
    responses_tool_pictures: ToolPictureCarry::InToolContent,
};

const ZHIPU_CHAT: ChatSpelling = ChatSpelling {
    ceiling: CeilingField::MaxTokens,
    effort: EffortField::ThinkingToggle,
    reasoning: ReasoningReturn::AsReasoningContent,
    chat_tool_pictures: ToolPictureCarry::FollowingUser,
    responses_tool_pictures: ToolPictureCarry::FollowingUser,
};

const ANTHROPIC_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "claude-fable-5-1",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-opus-5-5",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-sonnet-5-5",
        context_tokens: 1_000_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-haiku-4-5",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-opus-4",
        context_tokens: 200_000,
        max_output_tokens: 32_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-sonnet-4",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-7-sonnet",
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
    ModelPreset {
        id_prefix: "claude-3-5-haiku",
        context_tokens: 200_000,
        max_output_tokens: 8_192,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.claude.com/docs/en/about-claude/models/overview",
    },
];

const OPENAI_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "gpt-6.1-sol",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: Some(PresetThinking {
            levels: &LOW_TO_MAX,
            default: Some(Effort::Medium),
            source: "https://developers.openai.com/api/docs/models/gpt-6.1-sol",
        }),
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-6.1-sol",
    },
    ModelPreset {
        id_prefix: "gpt-6-astra",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: Some(PresetThinking {
            levels: &LOW_TO_MAX,
            default: None,
            source: "https://developers.openai.com/api/docs/models/gpt-6-astra",
        }),
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-6-astra",
    },
    ModelPreset {
        id_prefix: "gpt-6-sol",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-6-sol",
    },
    ModelPreset {
        id_prefix: "gpt-6-luna",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-6-luna",
    },
    ModelPreset {
        id_prefix: "gpt-5.6-sol",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: Some(PresetThinking {
            levels: &LOW_TO_MAX,
            default: Some(Effort::Medium),
            source: "https://developers.openai.com/api/docs/models/gpt-5.6-sol",
        }),
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-5.6-sol",
    },
    ModelPreset {
        id_prefix: "gpt-5.6-luna",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: Some(PresetThinking {
            levels: &LOW_TO_MAX,
            default: Some(Effort::Medium),
            source: "https://developers.openai.com/api/docs/models/gpt-5.6-luna",
        }),
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-5.6-luna",
    },
    ModelPreset {
        id_prefix: "gpt-5.6-terra",
        context_tokens: 1_050_000,
        max_output_tokens: 128_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: Some(PresetThinking {
            levels: &LOW_TO_MAX,
            default: Some(Effort::Medium),
            source: "https://developers.openai.com/api/docs/models/gpt-5.6-terra",
        }),
        cache_min_tokens: None,
        source: "https://developers.openai.com/api/docs/models/gpt-5.6-terra",
    },
    ModelPreset {
        id_prefix: "gpt-4.1",
        context_tokens: 1_047_576,
        max_output_tokens: 32_768,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.openai.com/docs/models/gpt-4.1",
    },
    ModelPreset {
        id_prefix: "gpt-4o",
        context_tokens: 128_000,
        max_output_tokens: 16_384,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.openai.com/docs/models/gpt-4o",
    },
    ModelPreset {
        id_prefix: "o3",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.openai.com/docs/models/o3",
    },
    ModelPreset {
        id_prefix: "o4-mini",
        context_tokens: 200_000,
        max_output_tokens: 100_000,
        input: InputKinds::TextImage,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://platform.openai.com/docs/models/o4-mini",
    },
];

const DEEPSEEK_MODELS: &[ModelPreset] = &[
    ModelPreset {
        id_prefix: "deepseek-flash",
        context_tokens: 1_000_000,
        max_output_tokens: 393_216,
        input: InputKinds::TextImage,
        image_max_side: std::num::NonZeroU32::new(8192),
        thinking: None,
        cache_min_tokens: None,
        source: "https://api-docs.deepseek.com/api/create-chat-completion",
    },
    ModelPreset {
        id_prefix: "deepseek-v4-flash",
        context_tokens: 1_000_000,
        max_output_tokens: 393_216,
        input: InputKinds::TextImage,
        image_max_side: std::num::NonZeroU32::new(8192),
        thinking: None,
        cache_min_tokens: None,
        source: "https://api-docs.deepseek.com/guides/vision",
    },
    ModelPreset {
        id_prefix: "deepseek-v4-pro",
        context_tokens: 1_000_000,
        max_output_tokens: 393_216,
        input: InputKinds::Text,
        image_max_side: None,
        thinking: None,
        cache_min_tokens: None,
        source: "https://api-docs.deepseek.com/quick_start/pricing",
    },
];
