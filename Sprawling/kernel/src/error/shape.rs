// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use super::code::AxCode;
use super::provider::{AccountDisposition, ProviderFailureKind};
use super::refusal::GateRefusal;

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
    #[serde(alias = "retriable")]
    retry: Retry,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "AccountDisposition::is_keep", default)]
    account: AccountDisposition,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    gate: Option<GateRefusal>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    provider: Option<ProviderFailureKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Retry {
    Yes,

    No,

    Unknown,
}

impl<'de> Deserialize<'de> for Retry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Spelling {
            Word(String),
            Flag(bool),
        }
        match Spelling::deserialize(deserializer)? {
            Spelling::Flag(true) => Ok(Retry::Yes),
            Spelling::Flag(false) => Ok(Retry::No),
            Spelling::Word(word) => match word.as_str() {
                "yes" => Ok(Retry::Yes),
                "no" => Ok(Retry::No),
                "unknown" => Ok(Retry::Unknown),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &["yes", "no", "unknown"],
                )),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "an ErrorDraft becomes an AxError only through with_recovery"]
pub struct ErrorDraft {
    pending: AxError,
}

impl AxError {
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
                    retry: Retry::No,
                    retry_after_ms: None,
                    account: AccountDisposition::Keep,
                    gate: None,
                    provider: None,
                }),
            },
        }
    }

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

    pub fn provider(
        kind: ProviderFailureKind,
        action: impl Into<String>,
        subject: impl Into<String>,
    ) -> ErrorDraft {
        let mut draft = AxError::failure(AxCode::Provider, action, subject);
        draft.pending.detail.provider = Some(kind);
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

    pub fn retry(&self) -> Retry {
        self.detail.retry
    }

    pub fn retry_after_ms(&self) -> Option<u64> {
        self.detail.retry_after_ms
    }

    pub fn account(&self) -> AccountDisposition {
        self.detail.account
    }

    pub fn gate(&self) -> Option<&GateRefusal> {
        self.detail.gate.as_ref()
    }

    pub fn provider_failure(&self) -> Option<ProviderFailureKind> {
        self.detail.provider
    }

    #[must_use]
    pub fn rewrite_recovery(mut self, recovery: impl Into<String>) -> Self {
        self.detail.recovery = recovery.into();
        self
    }
}

impl ErrorDraft {
    pub fn with_nearby(mut self, nearby: Vec<String>) -> Self {
        self.pending.detail.nearby = nearby;
        self
    }

    pub fn retriable(mut self) -> Self {
        self.pending.detail.retry = Retry::Yes;
        self.pending.detail.account = AccountDisposition::Keep;
        self
    }

    pub fn effect_unknown(mut self) -> Self {
        self.pending.detail.retry = Retry::Unknown;
        self.pending.detail.retry_after_ms = None;
        self.pending.detail.account = AccountDisposition::Keep;
        self
    }

    pub fn account_unusable(mut self) -> Self {
        self.pending.detail.retry = Retry::No;
        self.pending.detail.retry_after_ms = None;
        self.pending.detail.account = AccountDisposition::Advance;
        self
    }

    pub fn retriable_after(mut self, wait_ms: u64) -> Self {
        self.pending.detail.retry_after_ms = Some(wait_ms);
        self.retriable()
    }

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
        assert_eq!(err.retry(), Retry::No);
        assert_eq!(err.code(), &AxCode::PathNotFound);
    }

    #[test]
    fn builders_extend_without_touching_the_rest() {
        let err = AxError::failure(AxCode::ToolUnknown, "call tool", "grep")
            .with_nearby(vec!["exec".into(), "edit".into(), "status".into()])
            .retriable()
            .with_recovery("use an L0 tool");
        assert_eq!(err.retry(), Retry::Yes);
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "E_TOOL_UNKNOWN");
        assert_eq!(json["nearby"][0], "exec");
        assert_eq!(json["recovery"], "use an L0 tool");
        assert_eq!(json.get("gate"), None);
    }

    #[test]
    fn a_stated_wait_reads_back_until_the_effect_is_declared_unknown() {
        let draft = AxError::failure(AxCode::Provider, "call model", "429").retriable_after(1500);
        let waiting = draft.with_recovery("wait, then send the request again");
        assert_eq!(
            (waiting.retry(), waiting.retry_after_ms()),
            (Retry::Yes, Some(1500))
        );
        let unknown = AxError::failure(AxCode::Provider, "call model", "cut")
            .retriable_after(1500)
            .effect_unknown()
            .with_recovery("read the ledger before sending it again");
        assert_eq!(
            (unknown.retry(), unknown.retry_after_ms()),
            (Retry::Unknown, None)
        );
    }

    #[test]
    fn only_a_provider_error_names_a_provider_failure() {
        let kind = ProviderFailureKind::Refused { status: 503 };
        let named = AxError::provider(kind, "call model", "503").with_recovery("try later");
        assert_eq!(
            (named.code(), named.provider_failure()),
            (&AxCode::Provider, Some(kind))
        );
        let plain = AxError::failure(AxCode::Provider, "call model", "503").with_recovery("r");
        assert_eq!(plain.provider_failure(), None);
    }

    #[derive(Debug, Clone)]
    enum Builder {
        Retriable,
        RetriableAfter(u64),
        EffectUnknown,
        AccountUnusable,
    }

    fn builder() -> impl proptest::strategy::Strategy<Value = Builder> {
        use proptest::prelude::*;
        prop_oneof![
            Just(Builder::Retriable),
            any::<u64>().prop_map(Builder::RetriableAfter),
            Just(Builder::EffectUnknown),
            Just(Builder::AccountUnusable),
        ]
    }

    proptest::proptest! {
        #[test]
        fn no_order_of_calls_advances_a_resendable_failure(
            steps in proptest::collection::vec(builder(), 0..12)
        ) {
            let draft = steps.into_iter().fold(
                AxError::failure(AxCode::Provider, "call model", "s"),
                |draft, step| match step {
                    Builder::Retriable => draft.retriable(),
                    Builder::RetriableAfter(wait_ms) => draft.retriable_after(wait_ms),
                    Builder::EffectUnknown => draft.effect_unknown(),
                    Builder::AccountUnusable => draft.account_unusable(),
                },
            );
            let err = draft.with_recovery("r");
            if err.account() == AccountDisposition::Advance {
                proptest::prop_assert_eq!((err.retry(), err.retry_after_ms()), (Retry::No, None));
            }
            if err.retry_after_ms().is_some() {
                proptest::prop_assert_eq!(err.retry(), Retry::Yes);
            }
        }
    }

    #[test]
    fn only_a_switch_is_written_and_an_old_record_reads_as_keep() {
        let kept = AxError::failure(AxCode::Provider, "call model", "s").with_recovery("r");
        let advanced = AxError::failure(AxCode::Provider, "call model", "s")
            .retriable_after(500)
            .account_unusable()
            .with_recovery("r");
        let old = serde_json::json!({"code": "E_PROVIDER", "action": "call model",
            "subject": "s", "nearby": [], "recovery": "r", "retry": "no"});
        assert_eq!(
            (
                serde_json::to_value(&kept).unwrap(),
                serde_json::to_value(&advanced).unwrap()["account"].clone(),
                serde_json::from_value::<AxError>(old.clone())
                    .unwrap()
                    .account(),
            ),
            (old, serde_json::json!("advance"), AccountDisposition::Keep)
        );
        assert_eq!(
            serde_json::from_value::<AxError>(serde_json::to_value(&advanced).unwrap()).unwrap(),
            advanced
        );
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
        let retriable_at = json.find("\"retry\"").unwrap();
        assert!(code_at < action_at && action_at < subject_at && subject_at < retriable_at);
    }

    #[test]
    fn a_ledger_record_written_with_the_retriable_flag_still_reads() {
        let record = |flag: bool| {
            serde_json::json!({"code": "E_PROVIDER", "action": "call model", "subject": "s",
                "nearby": [], "recovery": "r", "retriable": flag})
        };
        let retry = |flag| {
            serde_json::from_value::<AxError>(record(flag))
                .unwrap()
                .retry()
        };
        assert_eq!([retry(true), retry(false)], [Retry::Yes, Retry::No]);
    }

    #[test]
    fn an_unknown_effect_carries_no_wait_whatever_the_builder_order() {
        let err = AxError::failure(AxCode::Provider, "call model", "s")
            .retriable_after(1_500)
            .effect_unknown()
            .with_recovery("r");
        assert_eq!((err.retry(), err.retry_after_ms()), (Retry::Unknown, None));
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
