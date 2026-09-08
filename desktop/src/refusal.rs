// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this server says when it will not do something.
//!
//! A refusal has three parts — what was refused, why, and what can be
//! done instead — and all three travel to the caller. The stable code is
//! spelled exactly as the city spells it in `kernel::error::code`, but it
//! is defined here rather than imported: this package sits outside the
//! workspace so that the Win32 boundary of card 7.2 may relax
//! `unsafe_code` at one call site, and importing a workspace crate to
//! obtain six string constants would give that reason away
//! (desktop-SPEC.md section 8.5, first pair). The rule that keeps the two
//! definitions from drifting is that this file only ever *quotes* a code
//! the city already has; a new one is minted in `kernel` first.

use serde_json::{Value, json};

/// The codes this server can answer with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalCode {
    /// A method or a tool name this build does not know.
    ToolUnknown,
    /// The shape is final, this machine or this build cannot carry it out.
    ToolUnavailable,
    /// The call was read, and its arguments were not usable.
    InvalidArgs,
    /// The scope file does not admit this.
    GateDenied,
    /// The scope file exists and cannot be read.
    ConfigInvalid,
    /// The line was not a JSON-RPC message this version can read.
    WireMismatch,
}

impl RefusalCode {
    /// The spelling, identical to the city's own.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            RefusalCode::ToolUnknown => "E_TOOL_UNKNOWN",
            RefusalCode::ToolUnavailable => "E_TOOL_UNAVAILABLE",
            RefusalCode::InvalidArgs => "E_INVALID_ARGS",
            RefusalCode::GateDenied => "E_GATE_DENIED",
            RefusalCode::ConfigInvalid => "E_CONFIG_INVALID",
            RefusalCode::WireMismatch => "E_WIRE_MISMATCH",
        }
    }

    /// The JSON-RPC number. The reserved range is used where JSON-RPC
    /// itself defines the situation; everything this server decides for
    /// itself sits in the implementation-defined range at `-32000`, so a
    /// generic client can still tell a protocol fault from a refusal.
    pub(crate) fn json_rpc(self) -> i64 {
        match self {
            RefusalCode::WireMismatch => -32600,
            RefusalCode::ToolUnknown => -32601,
            RefusalCode::InvalidArgs => -32602,
            RefusalCode::ToolUnavailable | RefusalCode::GateDenied | RefusalCode::ConfigInvalid => {
                -32000
            }
        }
    }
}

/// One refusal, in three parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    code: RefusalCode,
    action: String,
    subject: String,
    recovery: String,
}

impl Refusal {
    /// Every part is required, because a refusal missing one of them is
    /// the kind a reader has to guess at.
    pub(crate) fn new(
        code: RefusalCode,
        action: &str,
        subject: impl Into<String>,
        recovery: &str,
    ) -> Refusal {
        Refusal {
            code,
            action: action.to_owned(),
            subject: subject.into(),
            recovery: recovery.to_owned(),
        }
    }

    /// The JSON-RPC `error` object.
    ///
    /// This is the only way out of a refusal, for tests as much as for
    /// the wire: an accessor that exists so a test can read a field is a
    /// second door onto the same value. `message` is the one-line summary a
    /// person reads; `data` carries the three parts and the stable code
    /// for anything that decides on them.
    pub(crate) fn as_error(&self) -> Value {
        json!({
            "code": self.code.json_rpc(),
            "message": format!("{}: cannot {} — {}", self.code.as_str(), self.action, self.subject),
            "data": {
                "code": self.code.as_str(),
                "action": self.action,
                "subject": self.subject,
                "recovery": self.recovery,
            },
        })
    }
}

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

    /// A caller that reads `data.code` gets the same spelling the city
    /// uses, which is what lets one vocabulary cover both sides.
    #[test]
    fn a_refusal_carries_the_citys_own_spelling_and_all_three_parts() {
        let refusal = Refusal::new(
            RefusalCode::GateDenied,
            "act on a window",
            "`Ledger` is not in this scope",
            "add its title to DESKTOP.toml",
        );
        let error = refusal.as_error();
        assert_eq!(error["code"], -32000);
        assert_eq!(error["data"]["recovery"], "add its title to DESKTOP.toml");
        assert_eq!(error["data"]["code"], "E_GATE_DENIED");
        assert_eq!(error["data"]["action"], "act on a window");
        assert!(
            error["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("not in this scope")
        );
        assert!(error["message"].as_str().unwrap().contains("E_GATE_DENIED"));
    }

    /// A protocol fault and a refusal are different things to a generic
    /// client, so they are different numbers.
    #[test]
    fn a_protocol_fault_and_a_refusal_are_not_the_same_number() {
        assert_eq!(RefusalCode::ToolUnknown.json_rpc(), -32601);
        assert_eq!(RefusalCode::InvalidArgs.json_rpc(), -32602);
        assert_eq!(RefusalCode::WireMismatch.json_rpc(), -32600);
        assert_eq!(RefusalCode::ToolUnavailable.json_rpc(), -32000);
        for code in [
            RefusalCode::ToolUnknown,
            RefusalCode::ToolUnavailable,
            RefusalCode::InvalidArgs,
            RefusalCode::GateDenied,
            RefusalCode::ConfigInvalid,
            RefusalCode::WireMismatch,
        ] {
            assert!(code.as_str().starts_with("E_"), "{}", code.as_str());
        }
    }
}
