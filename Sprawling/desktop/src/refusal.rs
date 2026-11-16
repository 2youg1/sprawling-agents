// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use agent_protocols::EFFECT_META_KEY;
use kernel::AxCode;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalCode {

    ToolUnknown,

    ToolUnavailable,

    InvalidArgs,

    GateDenied,

    ConfigInvalid,

    WireMismatch,
}

impl RefusalCode {

    pub(crate) fn code(self) -> AxCode {
        match self {
            RefusalCode::ToolUnknown => AxCode::ToolUnknown,
            RefusalCode::ToolUnavailable => AxCode::ToolUnavailable,
            RefusalCode::InvalidArgs => AxCode::InvalidArgs,
            RefusalCode::GateDenied => AxCode::GateDenied,
            RefusalCode::ConfigInvalid => AxCode::ConfigInvalid,
            RefusalCode::WireMismatch => AxCode::WireMismatch,
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    code: RefusalCode,
    action: String,
    subject: String,
    recovery: String,
    aftermath: Aftermath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aftermath {

    Known,

    Unknown,
}

impl Refusal {

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
            aftermath: Aftermath::Known,
        }
    }

    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "the one caller is the partial-input path of platform::windows::act"
        )
    )]
    pub(crate) fn effect_unknown(self) -> Refusal {
        Refusal {
            aftermath: Aftermath::Unknown,
            ..self
        }
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "{}: cannot {} — {}",
            self.code.code().as_str(),
            self.action,
            self.subject
        )
    }

    pub(crate) fn as_error(&self) -> Value {
        json!({
            "code": self.code.json_rpc(),
            "message": self.summary(),
            "data": {
                "code": self.code.code().as_str(),
                "action": self.action,
                "subject": self.subject,
                "recovery": self.recovery,
            },
        })
    }

    pub(crate) fn as_tool_result(&self) -> Value {
        let text = format!("{}\ninstead: {}", self.summary(), self.recovery);
        let content = json!([{ "type": "text", "text": text }]);
        match self.aftermath {
            Aftermath::Known => json!({ "content": content, "isError": true }),
            Aftermath::Unknown => json!({
                "content": content,
                "isError": true,
                "_meta": { EFFECT_META_KEY: true },
            }),
        }
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

    #[test]
    fn a_refusal_reaches_a_model_as_text_marked_as_an_error() {
        let refusal = Refusal::new(
            RefusalCode::GateDenied,
            "act on a window",
            "`Ledger` is not in this scope",
            "add its title to DESKTOP.toml",
        );
        assert_eq!(
            refusal.as_tool_result(),
            json!({
                "content": [{ "type": "text", "text":
                    format!(
                        "{}: cannot act on a window — `Ledger` is not in this scope\ninstead: add its title to DESKTOP.toml",
                        "E_GATE_DENIED",
                    ) }],
                "isError": true,
            })
        );
    }

    #[test]
    fn a_refusal_whose_effect_is_unknown_says_so_in_meta() {
        let refusal = Refusal::new(
            RefusalCode::ToolUnavailable,
            "act on a window",
            "the desktop took 3 of the 6 input events",
            "look before acting again",
        )
        .effect_unknown();
        assert_eq!(
            refusal.as_tool_result(),
            json!({
                "content": [{ "type": "text", "text":
                    format!(
                        "{}: cannot act on a window — the desktop took 3 of the 6 input events\ninstead: look before acting again",
                        "E_TOOL_UNAVAILABLE",
                    ) }],
                "isError": true,
                "_meta": { "sprawling/effect-unknown": true },
            })
        );
    }

    #[test]
    fn a_protocol_fault_and_a_refusal_are_not_the_same_number() {
        assert_eq!(RefusalCode::ToolUnknown.json_rpc(), -32601);
        assert_eq!(RefusalCode::InvalidArgs.json_rpc(), -32602);
        assert_eq!(RefusalCode::WireMismatch.json_rpc(), -32600);
        assert_eq!(RefusalCode::ToolUnavailable.json_rpc(), -32000);
    }
}
