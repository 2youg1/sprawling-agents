// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub const CACHE_BREAKPOINTS_MAX: u32 = 4;

pub const PROMPT_CACHE_TTL_SECS: u64 = 300;

pub const PROMPT_CACHE_RENEWAL_MARGIN_SECS: u64 = 5;

pub const EVENT_LOG_V: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogVersion {
    Current,

    Older,

    Ahead,

    NotAVersion,
}

const EVENT_LOG_V_FIRST: u64 = 1;

#[must_use]
pub fn readable_log_v(v: u64) -> LogVersion {
    let current = u64::from(EVENT_LOG_V);
    if v < EVENT_LOG_V_FIRST {
        LogVersion::NotAVersion
    } else if v > current {
        LogVersion::Ahead
    } else if v == current {
        LogVersion::Current
    } else {
        LogVersion::Older
    }
}

pub const L0_TOOLS: [&str; 3] = [crate::ToolName::EXEC, "edit", "status"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretShape {
    pub provider: &'static str,
    pub prefix: &'static str,
    pub charset: SecretCharset,

    pub len: (u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretCharset {
    Base62,

    Base64Url,

    HexLower,

    UpperBase36,
}

pub const SECRET_SHAPES: [SecretShape; 11] = [
    SecretShape {
        provider: "anthropic",
        prefix: "sk-ant-",
        charset: SecretCharset::Base64Url,
        len: (24, 120),
    },
    SecretShape {
        provider: "openai",
        prefix: "sk-proj-",
        charset: SecretCharset::Base64Url,
        len: (24, 200),
    },
    SecretShape {
        provider: "github",
        prefix: "ghp_",
        charset: SecretCharset::Base62,
        len: (36, 40),
    },
    SecretShape {
        provider: "github-oauth",
        prefix: "gho_",
        charset: SecretCharset::Base62,
        len: (36, 40),
    },
    SecretShape {
        provider: "aws",
        prefix: "AKIA",
        charset: SecretCharset::UpperBase36,
        len: (16, 16),
    },
    SecretShape {
        provider: "gitlab",
        prefix: "glpat-",
        charset: SecretCharset::Base64Url,
        len: (20, 22),
    },
    SecretShape {
        provider: "slack",
        prefix: "xoxb-",
        charset: SecretCharset::Base62,
        len: (24, 80),
    },
    SecretShape {
        provider: "google",
        prefix: "AIza",
        charset: SecretCharset::Base64Url,
        len: (35, 35),
    },
    SecretShape {
        provider: "openrouter",
        prefix: "sk-or-v1-",
        charset: SecretCharset::HexLower,
        len: (40, 80),
    },
    SecretShape {
        provider: "zenmux",
        prefix: "sk-ai-v1-",
        charset: SecretCharset::HexLower,
        len: (40, 80),
    },
    SecretShape {
        provider: "groq",
        prefix: "gsk_",
        charset: SecretCharset::Base62,
        len: (32, 96),
    },
];

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_five_facts_hold_their_documented_values() {
        assert_eq!(CACHE_BREAKPOINTS_MAX, 4);
        assert_eq!(PROMPT_CACHE_TTL_SECS, 300);
        assert_eq!(EVENT_LOG_V, 2);
        assert_eq!(L0_TOOLS, ["exec", "edit", "status"]);
        assert_eq!(SECRET_SHAPES.len(), 11);
    }

    #[test]
    fn every_version_a_line_can_carry_lands_in_exactly_one_class() {
        assert_eq!(readable_log_v(0), LogVersion::NotAVersion);
        assert_eq!(readable_log_v(u64::from(EVENT_LOG_V)), LogVersion::Current);
        assert_eq!(
            readable_log_v(u64::from(EVENT_LOG_V).saturating_add(1)),
            LogVersion::Ahead
        );
        assert_eq!(readable_log_v(u64::MAX), LogVersion::Ahead);
        for v in EVENT_LOG_V_FIRST..u64::from(EVENT_LOG_V) {
            assert_eq!(readable_log_v(v), LogVersion::Older, "v{v}");
        }
    }

    #[test]
    fn secret_shapes_are_wellformed_data() {
        let prefixes: BTreeSet<&str> = SECRET_SHAPES.iter().map(|s| s.prefix).collect();
        assert_eq!(
            prefixes.len(),
            SECRET_SHAPES.len(),
            "prefixes must be unique"
        );
        for shape in &SECRET_SHAPES {
            assert!(!shape.prefix.is_empty());
            assert!(!shape.provider.is_empty());
            let (lo, hi) = shape.len;
            assert!(
                lo > 0 && lo <= hi,
                "{}: empty or inverted window",
                shape.provider
            );
        }
    }
}
