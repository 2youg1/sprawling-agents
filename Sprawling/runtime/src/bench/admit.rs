// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

impl ToolBench {

    #[must_use]
    pub fn limit_egress(mut self, list: kernel::EgressAllowlist) -> ToolBench {
        self.egress = Some(list);
        self
    }

    pub(super) fn admit(
        &mut self,
        call: &ToolCall,
        name: &str,
        effect: &Effect,
        subject: &GateSubject,
    ) -> Result<Option<BenchOutcome>, AxError> {
        match effect {
            Effect::Read => {}

            Effect::Write { domain: area } => {
                let verdict = kernel::gate::reach(&self.domain, area, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
                let named = match subject {
                    GateSubject::Area(at) | GateSubject::Room(at) => Some(at),
                    GateSubject::Scope(_) | GateSubject::Host(_) | GateSubject::None => None,
                };
                if let Some(at) = named {
                    let verdict = kernel::gate::reach(&self.domain, at, &self.taint);
                    if let Some(answered) = self.settled(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::Connector { label } => {

                let spans = kernel::secret::scan(&scanned(call, "scan connector args")?);
                let verdict = kernel::gate::egress(
                    &spans,
                    &EgressTarget::Connector {
                        label: label.clone(),
                    },
                    self.prior_public_egress,
                );
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }

                let called = ToolName::parse(name)?;
                let reaching = kernel::ConnectorCall {
                    label,
                    tool: &called,
                };
                if kernel::gate::reaches_the_undoable(&reaching) {
                    let verdict = kernel::gate::undoable(&reaching, &self.sandbox, &self.taint);
                    if let Some(answered) = self.settled(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::Egress => {

                let target = bytes_target(name, subject, "send bytes out")?;
                if let Some(answered) = self.listed(&target) {
                    return Ok(Some(answered));
                }
                let spans = kernel::secret::scan(&scanned(call, "scan egress args")?);
                let verdict = kernel::gate::egress(&spans, &target, self.prior_public_egress);
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::AttachUserBrowser { address } => {

                let endpoint = address.as_deref().map(kernel::gate::target_of);
                let verdict = kernel::gate::attach(endpoint.as_ref());
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }

                let page = bytes_target(name, subject, "drive a browser")?;
                if let Some(answered) = self.listed(&page) {
                    return Ok(Some(answered));
                }
                let spans = kernel::secret::scan(&scanned(call, "scan browser args")?);
                let verdict = kernel::gate::egress(&spans, &page, self.prior_public_egress);
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Spawn => {

            }
            Effect::Govern => {

                let scope = match subject {
                    GateSubject::Scope(scope) => scope.clone(),

                    GateSubject::None => {
                        return Err(subject_not_for(name, subject, "change what governs"));
                    }
                    other
                    @ (GateSubject::Area(_) | GateSubject::Room(_) | GateSubject::Host(_)) => {
                        return Err(subject_not_for(name, other, "change what governs"));
                    }
                };
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "invoke tool",
                    format!(
                        "`{name}` declares Govern, and a run may not change what governs it \
                         ({scope})"
                    ),
                )
                .with_recovery(
                    "edit the scope's `CONFIG.toml`, or the building's `RULES.toml`, and \
                     dispatch again",
                ));
            }
            Effect::Spend => {

                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "invoke tool",
                    format!("`{name}` declares Spend, which no tool in this build carries"),
                )
                .with_recovery(
                    "do this work with a tool that spends nothing; no tool in this build \
                     can move money",
                ));
            }
        }
        Ok(None)
    }

    pub(super) fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome> {
        match outcome {
            GateOutcome::Allow => None,
            GateOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
            GateOutcome::Ask { question } => Some(BenchOutcome::Refused { refusal: question }),
        }
    }

    fn listed(&self, target: &EgressTarget) -> Option<BenchOutcome> {
        let list = self.egress.as_ref()?;
        match kernel::gate::egress_target(list, target) {
            EgressOutcome::Allow { .. } => None,
            EgressOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
        }
    }

    fn crossed(&mut self, outcome: EgressOutcome) -> Option<BenchOutcome> {
        match outcome {
            EgressOutcome::Allow {
                first_public_egress,
            } => {
                if first_public_egress {
                    self.prior_public_egress = true;
                }
                None
            }
            EgressOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
        }
    }
}

fn bytes_target(name: &str, subject: &GateSubject, doing: &str) -> Result<EgressTarget, AxError> {
    match subject {
        GateSubject::Host(host) => Ok(kernel::gate::target_of(host)),
        GateSubject::None => Ok(EgressTarget::Loopback),
        other @ (GateSubject::Area(_) | GateSubject::Room(_) | GateSubject::Scope(_)) => {
            Err(subject_not_for(name, other, doing))
        }
    }
}

fn subject_not_for(name: &str, subject: &GateSubject, doing: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "invoke tool",
        format!(
            "`{name}` declares {doing} and its `subject` answered {}",
            spell(subject)
        ),
    )
    .with_recovery(
        "report this against the tool: `Tool::subject` and `ToolMeta::effect` disagree, \
         and the bench reads the subject the tool answers with",
    )
}

fn spell(subject: &GateSubject) -> &'static str {
    match subject {
        GateSubject::Area(_) => "an area",
        GateSubject::Room(_) => "a room",
        GateSubject::Scope(_) => "a scope",
        GateSubject::Host(_) => "a host",
        GateSubject::None => "no subject",
    }
}

fn scanned(call: &ToolCall, doing: &'static str) -> Result<Vec<u8>, AxError> {
    serde_json::to_vec(&call.args).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, doing, err.to_string()).with_recovery(
            "call the tool again with arguments made of strings and whole numbers, \
                 which is all this city's payloads carry",
        )
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::{RunId, Seq};

    struct Stub(kernel::ToolMeta, GateSubject);

    fn stub(name: &str, effect: Effect, answer: GateSubject) -> Box<dyn Tool> {
        Box::new(Stub(
            kernel::ToolMeta {
                name: kernel::ToolName::parse(name).unwrap(),
                disclosure: "a stub the door tests drive".to_owned(),
                params: kernel::Payload::empty(),
                effect,
                cost_tier: kernel::CostTier::Free,
                timeout: None,
                render: kernel::RenderIntent::Generic,
                temporal: kernel::Temporal::Timeless,
            },
            answer,
        ))
    }

    impl Tool for Stub {
        fn meta(&self) -> &kernel::ToolMeta {
            &self.0
        }

        fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
            Ok(ToolOutcome {
                result: kernel::Payload::empty(),
                attachments: Vec::new(),
            })
        }

        fn subject(&self, _call: &ToolCall) -> Result<GateSubject, AxError> {
            Ok(self.1.clone())
        }
    }

    struct Governs(kernel::ToolMeta);

    impl Governs {
        fn op(call: &ToolCall) -> Option<&str> {
            call.args.as_map().get("op").and_then(Value::as_str)
        }
    }

    impl Tool for Governs {
        fn meta(&self) -> &kernel::ToolMeta {
            &self.0
        }

        fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
            Ok(ToolOutcome {
                result: kernel::Payload::empty(),
                attachments: Vec::new(),
            })
        }

        fn subject(&self, _call: &ToolCall) -> Result<GateSubject, AxError> {
            Ok(GateSubject::Scope("building:lab".to_owned()))
        }

        fn effect_of(&self, call: &ToolCall) -> Result<Effect, AxError> {
            Ok(match Self::op(call) {
                Some("read") => Effect::Read,
                Some(_) | None => Effect::Govern,
            })
        }
    }

    fn op_call(op: &str) -> ToolCall {
        let mut args = serde_json::Map::new();
        args.insert("op".to_owned(), Value::String(op.to_owned()));
        call("ruler", kernel::Payload::new(args).unwrap())
    }

    #[test]
    fn a_govern_tool_read_runs_in_a_run_and_its_rewrite_is_still_refused() {
        let meta = stub("ruler", Effect::Govern, GateSubject::None)
            .meta()
            .clone();
        let mut bench = bench_with(Box::new(Governs(meta)));
        let read = bench.invoke(&op_call("read"), &key(6), at()).unwrap();
        assert!(matches!(read, BenchOutcome::Ran { .. }), "{read:?}");
        let rewrite = bench
            .invoke(&op_call("propose"), &key(7), at())
            .unwrap_err();
        assert_eq!(rewrite.code(), &AxCode::GateDenied);
    }

    fn bench_with(tool: Box<dyn Tool>) -> ToolBench {
        let domain = WriteDomain::new(vec![Address::parse("work").unwrap()]).unwrap();
        let mut bench = ToolBench::new(domain);
        bench.register(tool).unwrap();
        bench
    }

    fn call(tool: &str, args: kernel::Payload) -> ToolCall {
        ToolCall {
            id: "c-1".to_owned(),
            name: kernel::ToolName::parse(tool).unwrap(),
            args,
        }
    }

    fn key(n: u64) -> IdemKey {
        IdemKey::derive(&RunId::from_bytes([1u8; 16]), Seq::new(n), b"door")
    }

    fn at() -> kernel::TimeMs {
        kernel::TimeMs::new(1_700_000_000_000)
    }

    #[test]
    fn an_egress_call_that_names_no_host_is_still_scanned() {
        let mut bench = bench_with(stub("webbish", Effect::Egress, GateSubject::None));
        let secret = format!("sk-ant-{}", "a1B2c3D4e5".repeat(9));
        let mut args = serde_json::Map::new();
        args.insert("text".to_owned(), Value::String(secret));
        let answered = bench
            .invoke(
                &call("webbish", kernel::Payload::new(args).unwrap()),
                &key(1),
                at(),
            )
            .unwrap();
        assert!(matches!(answered, BenchOutcome::Refused { .. }));
    }

    #[test]
    fn an_egress_call_that_names_no_host_still_runs_when_the_bytes_are_clean() {
        let mut bench = bench_with(stub("webbish", Effect::Egress, GateSubject::None));
        let answered = bench
            .invoke(&call("webbish", kernel::Payload::empty()), &key(2), at())
            .unwrap();
        assert!(matches!(answered, BenchOutcome::Ran { .. }));
    }

    fn listing(domains: &[&str]) -> kernel::EgressAllowlist {
        kernel::EgressAllowlist::new(domains.iter().map(|d| (*d).to_owned()).collect())
    }

    fn page(host: &str) -> GateSubject {
        GateSubject::Host(host.to_owned())
    }

    #[test]
    fn a_browser_call_reaches_only_the_hosts_the_building_lists() {
        let outcomes = [
            ("pastebin.test", Some(listing(&["example.com"]))),
            ("docs.example.com", Some(listing(&["example.com"]))),
            ("example.com", Some(listing(&[]))),
            ("pastebin.test", None),
            ("127.0.0.1", Some(listing(&[]))),
        ]
        .into_iter()
        .enumerate()
        .map(|(n, (host, list))| {
            let bench = bench_with(stub("webbish", Effect::Egress, page(host)));
            let mut bench = match list {
                Some(list) => bench.limit_egress(list),
                None => bench,
            };
            let seq = u64::try_from(n).unwrap() + 10;
            match bench.invoke(&call("webbish", kernel::Payload::empty()), &key(seq), at()) {
                Ok(BenchOutcome::Ran { .. }) => "ran".to_owned(),
                Ok(BenchOutcome::Refused { refusal }) => refusal.code().to_string(),
                Ok(BenchOutcome::Duplicate { .. }) => "duplicate".to_owned(),
                Err(failed) => format!("failed {}", failed.code()),
            }
        })
        .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            ["E_GATE_DENIED", "ran", "E_GATE_DENIED", "ran", "ran"]
        );
    }

    #[test]
    fn the_persons_browser_is_held_to_the_same_list() {
        let effect = Effect::AttachUserBrowser {
            address: Some("127.0.0.1".to_owned()),
        };
        let mut bench = bench_with(stub("drives", effect, page("pastebin.test")))
            .limit_egress(listing(&["example.com"]));
        let answered = bench
            .invoke(&call("drives", kernel::Payload::empty()), &key(30), at())
            .unwrap();
        let BenchOutcome::Refused { refusal } = answered else {
            panic!("expected a refusal, got {answered:?}");
        };
        assert_eq!(
            (refusal.code(), refusal.subject()),
            (&AxCode::GateDenied, "pastebin.test")
        );
    }

    #[test]
    fn an_egress_call_that_answers_with_an_area_is_refused_as_a_wiring_defect() {
        let area = Address::parse("work/room").unwrap();
        let mut bench = bench_with(stub("webbish", Effect::Egress, GateSubject::Area(area)));
        let refused = bench.invoke(&call("webbish", kernel::Payload::empty()), &key(3), at());
        assert!(refused.is_err());
    }

    #[test]
    fn a_govern_call_that_names_no_scope_is_refused_as_a_tool_that_named_none() {
        let mut bench = bench_with(stub("ruler", Effect::Govern, GateSubject::None));
        let refused = bench
            .invoke(&call("ruler", kernel::Payload::empty()), &key(4), at())
            .unwrap_err();
        assert!(refused.subject().contains("no subject"));
    }

    #[test]
    fn a_govern_refusal_tells_the_model_its_recovery_without_a_run_of_spaces() {
        let scope = GateSubject::Scope("work".to_owned());
        let mut bench = bench_with(stub("ruler", Effect::Govern, scope));
        let refused = bench
            .invoke(&call("ruler", kernel::Payload::empty()), &key(5), at())
            .unwrap_err();
        assert_eq!(
            refused.recovery(),
            "edit the scope's `CONFIG.toml`, or the building's `RULES.toml`, and dispatch again"
        );
    }
}
