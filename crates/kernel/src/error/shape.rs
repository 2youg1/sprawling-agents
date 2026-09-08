// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! AxError: the one error shape of the whole city.

use serde::{Deserialize, Serialize};

use super::code::AxCode;
use super::refusal::GateRefusal;

/// The unified error shape: seven wire fields, serialized in declaration
/// order (determinism rule 6). The model is the recovery subject: `nearby`
/// and `recovery` must hold directly executable information, not apologies.
///
/// Everything but `code` sits behind one Box so the type stays cheap in
/// every seam's return slot (`result_large_err`); serde flatten keeps the
/// wire shape flat and the field order unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: cannot {} on {}", detail.action, detail.subject)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AxError {
    code: AxCode,
    #[serde(flatten)]
    detail: Box<ErrorDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct ErrorDetail {
    action: String,
    subject: String,
    nearby: Vec<String>,
    recovery: String,
    retriable: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    gate: Option<GateRefusal>,
}

impl AxError {
    /// Non-gate failure. `retriable` starts false (fail-closed); `nearby`
    /// and `recovery` start empty and grow via the builders.
    pub fn failure(code: AxCode, action: impl Into<String>, subject: impl Into<String>) -> Self {
        AxError {
            code,
            detail: Box::new(ErrorDetail {
                action: action.into(),
                subject: subject.into(),
                nearby: Vec::new(),
                recovery: String::new(),
                retriable: false,
                gate: None,
            }),
        }
    }

    /// Gate refusal: the only constructor that sets the three mandatory
    /// parts. Gate-carrier codes must come through here (kernel::gate is
    /// their sole producer from S2 on).
    pub fn refusal(
        code: AxCode,
        action: impl Into<String>,
        subject: impl Into<String>,
        gate: GateRefusal,
    ) -> Self {
        let mut err = AxError::failure(code, action, subject);
        err.detail.gate = Some(gate);
        err
    }

    pub fn with_nearby(mut self, nearby: Vec<String>) -> Self {
        self.detail.nearby = nearby;
        self
    }

    pub fn with_recovery(mut self, recovery: impl Into<String>) -> Self {
        self.detail.recovery = recovery.into();
        self
    }

    /// Declares the action safe to retry as-is. Explicit opt-in only.
    pub fn retriable(mut self) -> Self {
        self.detail.retriable = true;
        self
    }

    pub fn code(&self) -> &AxCode {
        &self.code
    }

    pub fn action(&self) -> &str {
        &self.detail.action
    }

    pub fn subject(&self) -> &str {
        &self.detail.subject
    }

    pub fn nearby(&self) -> &[String] {
        &self.detail.nearby
    }

    pub fn recovery(&self) -> &str {
        &self.detail.recovery
    }

    pub fn is_retriable(&self) -> bool {
        self.detail.retriable
    }

    pub fn gate(&self) -> Option<&GateRefusal> {
        self.detail.gate.as_ref()
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
    fn failure_has_no_gate_and_is_not_retriable_by_default() {
        let err = AxError::failure(AxCode::PathNotFound, "read file", "docs/a.md");
        assert!(err.gate().is_none());
        assert!(!err.is_retriable());
        assert_eq!(err.code(), &AxCode::PathNotFound);
    }

    #[test]
    fn builders_extend_without_touching_the_rest() {
        let err = AxError::failure(AxCode::ToolUnknown, "call tool", "grep")
            .with_nearby(vec!["exec".into(), "edit".into(), "status".into()])
            .with_recovery("use an L0 tool")
            .retriable();
        assert!(err.is_retriable());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "E_TOOL_UNKNOWN");
        assert_eq!(json["nearby"][0], "exec");
        assert_eq!(json["recovery"], "use an L0 tool");
        assert_eq!(json.get("gate"), None);
    }

    #[test]
    fn axerror_serde_field_order_is_declaration_order() {
        let err = AxError::failure(AxCode::Timeout, "run exec", "build.sh");
        let json = serde_json::to_string(&err).unwrap();
        let code_at = json.find("\"code\"").unwrap();
        let action_at = json.find("\"action\"").unwrap();
        let subject_at = json.find("\"subject\"").unwrap();
        let retriable_at = json.find("\"retriable\"").unwrap();
        assert!(code_at < action_at && action_at < subject_at && subject_at < retriable_at);
    }

    #[test]
    fn display_names_code_action_subject() {
        let err = AxError::failure(AxCode::BudgetExhausted, "call model", "run-1");
        let text = err.to_string();
        assert!(text.contains("E_BUDGET_EXHAUSTED"));
        assert!(text.contains("call model"));
        assert!(text.contains("run-1"));
    }

    #[test]
    fn refusal_carries_the_three_mandatory_parts() {
        let gate = GateRefusal::new("rule", "violation", "alternative");
        let err = AxError::refusal(AxCode::GateDenied, "wire funds", "acct-9", gate);
        let got = err.gate().expect("gate refusal must carry the three parts");
        assert_eq!(got.rule(), "rule");
        assert_eq!(got.violation(), "violation");
        assert_eq!(got.alternative(), "alternative");
    }
}
