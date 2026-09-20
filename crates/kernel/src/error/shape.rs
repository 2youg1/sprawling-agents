// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! AxError: the one error shape of the whole city, and ErrorDraft, the
//! only way to reach it.

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

/// An error that still owes the reader its recovery sentence.
///
/// Both constructors return this rather than an [`AxError`], and
/// [`ErrorDraft::with_recovery`] is the only way across, so a failure that
/// never says what to do next cannot be spelled. Shaping happens here;
/// an `AxError` is finished the moment it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "an ErrorDraft becomes an AxError only through with_recovery"]
pub struct ErrorDraft {
    pending: AxError,
}

impl AxError {
    /// Non-gate failure. `retriable` starts false (fail-closed) and
    /// `nearby` starts empty; both grow on the draft.
    pub fn failure(
        code: AxCode,
        action: impl Into<String>,
        subject: impl Into<String>,
    ) -> ErrorDraft {
        ErrorDraft {
            pending: AxError {
                code,
                detail: Box::new(ErrorDetail {
                    action: action.into(),
                    subject: subject.into(),
                    nearby: Vec::new(),
                    recovery: String::new(),
                    retriable: false,
                    gate: None,
                }),
            },
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
    ) -> ErrorDraft {
        let mut draft = AxError::failure(code, action, subject);
        draft.pending.detail.gate = Some(gate);
        draft
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

    /// Replaces the recovery sentence of an error raised further down,
    /// for a caller who knows a better next step than its raiser did.
    /// Named apart from [`ErrorDraft::with_recovery`] because it destroys
    /// a sentence rather than supplying the first one.
    #[must_use]
    pub fn rewrite_recovery(mut self, recovery: impl Into<String>) -> Self {
        self.detail.recovery = recovery.into();
        self
    }
}

impl ErrorDraft {
    /// Names the spellings the caller could have used instead. Directly
    /// usable values only, not a description of them.
    pub fn with_nearby(mut self, nearby: Vec<String>) -> Self {
        self.pending.detail.nearby = nearby;
        self
    }

    /// Declares the action safe to retry as-is. Explicit opt-in only.
    pub fn retriable(mut self) -> Self {
        self.pending.detail.retriable = true;
        self
    }

    /// Writes the one sentence that tells the reader what to do next, and
    /// hands back the finished error. The reader is as often a model as a
    /// person, so the sentence names a key, a file and line, or a command.
    pub fn with_recovery(mut self, recovery: impl Into<String>) -> AxError {
        self.pending.detail.recovery = recovery.into();
        self.pending
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
        let err = AxError::failure(AxCode::PathNotFound, "read file", "docs/a.md")
            .with_recovery("create docs/a.md, or read one of the paths `ls docs` lists");
        assert!(err.gate().is_none());
        assert!(!err.is_retriable());
        assert_eq!(err.code(), &AxCode::PathNotFound);
    }

    #[test]
    fn builders_extend_without_touching_the_rest() {
        let err = AxError::failure(AxCode::ToolUnknown, "call tool", "grep")
            .with_nearby(vec!["exec".into(), "edit".into(), "status".into()])
            .retriable()
            .with_recovery("use an L0 tool");
        assert!(err.is_retriable());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "E_TOOL_UNKNOWN");
        assert_eq!(json["nearby"][0], "exec");
        assert_eq!(json["recovery"], "use an L0 tool");
        assert_eq!(json.get("gate"), None);
    }

    #[test]
    fn axerror_serde_field_order_is_declaration_order() {
        let err = AxError::failure(AxCode::Timeout, "run exec", "build.sh").with_recovery(
            "raise `[tool] exec_timeout_ms` in CONFIG.toml, or run a shorter script",
        );
        let json = serde_json::to_string(&err).unwrap();
        let code_at = json.find("\"code\"").unwrap();
        let action_at = json.find("\"action\"").unwrap();
        let subject_at = json.find("\"subject\"").unwrap();
        let retriable_at = json.find("\"retriable\"").unwrap();
        assert!(code_at < action_at && action_at < subject_at && subject_at < retriable_at);
    }

    #[test]
    fn display_names_code_action_subject() {
        let err = AxError::failure(AxCode::BudgetExhausted, "call model", "run-1")
            .with_recovery("raise `[budget]` in CONFIG.toml, then resume run-1");
        let text = err.to_string();
        assert!(text.contains("E_BUDGET_EXHAUSTED"));
        assert!(text.contains("call model"));
        assert!(text.contains("run-1"));
    }

    #[test]
    fn refusal_carries_the_three_mandatory_parts() {
        let gate = GateRefusal::new("rule", "violation", "alternative");
        let err = AxError::refusal(AxCode::GateDenied, "wire funds", "acct-9", gate)
            .with_recovery("alternative");
        let got = err.gate().expect("gate refusal must carry the three parts");
        assert_eq!(got.rule(), "rule");
        assert_eq!(got.violation(), "violation");
        assert_eq!(got.alternative(), "alternative");
    }

    #[test]
    fn every_constructed_error_carries_a_recovery_sentence() {
        let failure = AxError::failure(AxCode::InvalidArgs, "parse address", "city//")
            .with_recovery("write `city/<building>/<room>`");
        let refusal = AxError::refusal(
            AxCode::GateDenied,
            "write file",
            "/etc/hosts",
            GateRefusal::new("rule", "violation", "alternative"),
        )
        .with_recovery("write inside the run's write domain");
        assert!(!failure.recovery().is_empty());
        assert!(!refusal.recovery().is_empty());
    }
}
