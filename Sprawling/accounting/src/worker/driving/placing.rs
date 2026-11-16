// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use kernel::{
    AxError, Effect, IdemKey, RunId, Seq, Temporal, TimeMs, Tool, ToolCall, ToolOutcome, Writes,
};
use runtime::bench::{BenchOutcome, Clearance, Ticket, ToolBench};
use runtime::{
    Admitted, Catalog, ConcurrentInvoke, ReadShown, RuleCandidates, RuleContext, ToolTool,
};

use super::Sieving;
use crate::prompt_chain::RuleDelivery;

pub(super) struct Checkpointing {
    pub(super) checkpointed: RefCell<Vec<String>>,

    wrote: RefCell<Writes>,
}

impl Checkpointing {
    pub(super) fn opened() -> Checkpointing {
        Checkpointing {
            checkpointed: RefCell::new(Vec::new()),
            wrote: RefCell::new(Writes::Domain),
        }
    }

    pub(super) fn take_scope(&self, domain: &[String]) -> Vec<String> {
        match self.wrote.replace(Writes::Nothing) {
            Writes::Paths(paths) => paths.iter().map(|path| path.as_str().to_owned()).collect(),
            Writes::Nothing | Writes::Domain => domain.to_vec(),
        }
    }

    fn add(&self, this: Writes) {
        let mut held = self.wrote.borrow_mut();
        *held = std::mem::replace(&mut *held, Writes::Nothing).and(this);
    }

    fn widen(&self) {
        self.add(Writes::Domain);
    }
}

pub(super) struct Placing<'f> {
    bench: ToolBench,
    sieving: Sieving,
    run: RunId,

    next: u64,

    ran: (u32, u32),
    checkpointing: &'f Checkpointing,

    catalog: Arc<Mutex<Catalog>>,
    shown: ReadShown,
    rules: RuleDelivery,
    rule_candidates: Option<RuleCandidates>,
    halted: Option<&'f dyn Fn() -> bool>,
}

impl<'f> Placing<'f> {
    pub(super) fn new(
        bench: ToolBench,
        sieving: Sieving,
        run: RunId,
        checkpointing: &'f Checkpointing,
    ) -> Placing<'f> {
        Placing {
            bench,
            sieving,
            run,
            next: 0,
            ran: (0, 0),
            checkpointing,
            catalog: Arc::new(Mutex::new(Catalog::new())),
            shown: ReadShown::default(),
            rules: RuleDelivery::default(),
            rule_candidates: None,
            halted: None,
        }
    }

    pub(super) fn stopping(mut self, halted: &'f dyn Fn() -> bool) -> Self {
        self.halted = Some(halted);
        self
    }

    #[must_use]
    pub(super) fn resolving(mut self, catalog: Arc<Mutex<Catalog>>) -> Placing<'f> {
        self.catalog = catalog;
        self
    }

    #[must_use]
    pub(super) fn ruling(mut self, candidates: RuleCandidates) -> Self {
        self.rule_candidates = Some(candidates);
        self
    }

    #[must_use]
    pub(super) fn reading(
        mut self,
        shown: ReadShown,
        delivered: std::collections::BTreeSet<kernel::Address>,
    ) -> Self {
        self.shown = shown;
        self.rules.delivered = delivered;
        self
    }

    pub(super) fn ran(&self) -> (u32, u32) {
        self.ran
    }

    fn unsieved(&mut self, call: &ToolCall, outcome: ToolOutcome) -> Result<ToolOutcome, AxError> {
        match self.bench.meta_of(&call.name).map(|meta| &meta.effect) {
            Some(Effect::Connector { .. }) => self.sieving.package_connector(outcome),
            Some(
                Effect::Read
                | Effect::Write { .. }
                | Effect::Spawn
                | Effect::Govern
                | Effect::Spend
                | Effect::Egress
                | Effect::AttachUserBrowser { .. },
            )
            | None => Ok(outcome),
        }
    }

    fn temporal_of(&self, call: &ToolCall) -> Temporal {
        self.bench
            .meta_of(&call.name)
            .map_or(Temporal::Timeless, |meta| meta.temporal)
    }

    fn prepare_read(
        &mut self,
        call: &ToolCall,
        mut outcome: ToolOutcome,
    ) -> Result<ToolOutcome, AxError> {
        if call.name.as_str() != runtime::ReadTool::NAME {
            return Ok(outcome);
        }
        let mut result = outcome.result.as_map().clone();
        let context = result
            .remove(runtime::ReadTool::RULE_CONTEXT)
            .and_then(|value| RuleContext::from_json(&value));
        let candidates = match (context, self.rule_candidates.as_ref()) {
            (Some(context), Some(candidates)) => candidates(
                &context.admitted,
                &context.landed,
                &self.rules.seen(),
                &mut || self.halted.is_some_and(|halted| halted()),
            ),
            _ => Vec::new(),
        };
        let selected = self.rules.answer(&call.id, candidates);
        if selected.is_empty() {
            result.remove("rules");
        } else {
            result.insert(
                "rules".to_owned(),
                serde_json::Value::Array(
                    selected
                        .into_iter()
                        .map(runtime::RuleCandidate::into_json)
                        .collect(),
                ),
            );
        }
        outcome.result = kernel::Payload::new(result)?;
        Ok(outcome)
    }

    fn answered(&mut self, call: &ToolCall, decided: BenchOutcome) -> Result<ToolOutcome, AxError> {
        match decided {
            BenchOutcome::Ran {
                outcome,
                checkpointed,
                wrote,
            } => {
                if let Some(oid) = checkpointed {
                    self.checkpointing.checkpointed.borrow_mut().push(oid);
                }
                self.checkpointing.add(wrote);
                if call.name.as_str() != kernel::ToolName::EXEC {
                    return self.unsieved(call, outcome);
                }

                let failed = match outcome
                    .result
                    .as_map()
                    .get("exit_code")
                    .and_then(serde_json::Value::as_i64)
                {
                    Some(code) => code != 0,
                    None => true,
                };
                if failed {
                    self.ran.1 = self.ran.1.saturating_add(1);
                } else {
                    self.ran.0 = self.ran.0.saturating_add(1);
                }
                self.sieving.package(call, outcome, self.temporal_of(call))
            }
            BenchOutcome::Refused { refusal } => Err(*refusal),

            BenchOutcome::Duplicate { outcome } => {
                if call.name.as_str() == runtime::ReadTool::NAME {
                    self.rules.replay(&call.id, &outcome.result);
                }
                if call.name.as_str() == kernel::ToolName::EXEC {
                    self.sieving.package(call, outcome, self.temporal_of(call))
                } else {
                    self.unsieved(call, outcome)
                }
            }
        }
    }
}

impl ConcurrentInvoke for Placing<'_> {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.bench.meta_of(&call.name)
    }

    fn resolve_call(&self, call: ToolCall) -> ToolCall {
        if call.name.as_str() != ToolTool::NAME {
            return call;
        }

        match self.catalog.lock() {
            Ok(catalog) => catalog.resolve_run(&call).unwrap_or(call),
            Err(_) => call,
        }
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        let at = self.next;
        self.next = at.saturating_add(1);

        let key = match call.action() {
            Ok(action) => IdemKey::derive(&self.run, Seq::new(at), &action),
            Err(unreadable) => {
                self.shown.discard(&call.id);
                return Admitted::Answered(Err(unreadable));
            }
        };
        match self.bench.clear(call, &key, t) {
            Ok(Clearance::Cleared(ticket)) => Admitted::Cleared(ticket),
            Ok(Clearance::Answered(decided)) => {
                let answered = self.answered(call, decided);
                self.shown.discard(&call.id);
                Admitted::Answered(answered)
            }
            Err(refused) => {
                self.checkpointing.widen();
                self.shown.discard(&call.id);
                Admitted::Answered(Err(refused))
            }
        }
    }

    fn ahead(&self, call: &ToolCall) -> Option<&dyn Tool> {
        self.bench.tool_named(&call.name)
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        let answered = match answered {
            Ok(outcome) => self.prepare_read(call, outcome),
            Err(err) => Err(err),
        };
        if answered.is_err() {
            self.shown.discard(&call.id);
        }
        let decided = self.bench.account(ticket, answered).inspect_err(|_| {
            self.checkpointing.widen();
            self.shown.discard(&call.id);
        })?;
        let answered = self.answered(call, decided);
        if answered.is_ok() {
            self.shown.admit(&call.id);
        } else {
            self.shown.discard(&call.id);
        }
        answered
    }

    fn complete_wave(&mut self, shortened: &std::collections::BTreeSet<String>) {
        self.rules.complete_wave(shortened);
        self.shown.confirm(shortened);
    }
}
