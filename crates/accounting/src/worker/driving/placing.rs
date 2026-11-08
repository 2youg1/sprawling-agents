// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run's bench as the tool face a wave drives in three stages
//! (`crates/runtime/Spec.lean` §8-3): each call's key is placed by its position when
//! it is admitted, and the commits it was checkpointed against, what it says
//! it wrote and the commands that ran are counted when it is accounted.
//! Both happen in call order, so a wave whose reads ran at once leaves
//! the same keys, the same checkpoint list, the same next checkpoint and the same
//! counts as one that ran them in turn.

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

/// What a run's calls leave for the checkpoints around them. Shared between
/// the tool face and the wave checkpoint, which the turn driver holds at once.
pub(super) struct Checkpointing {
    /// The commits the run was checkpointed against, in the order they went
    /// up. The wave checkpoint and the bench both add to it, so the sweep
    /// afterwards reads the first commit a deleted file can be restored
    /// from.
    pub(super) checkpointed: RefCell<Vec<String>>,
    /// What the calls since the last checkpoint said they wrote, which is
    /// what the next checkpoint stages (`crates/runtime/Spec.lean` §8-45).
    wrote: RefCell<Writes>,
}

impl Checkpointing {
    /// A run's checkpointing before its first call. What it wrote opens as the
    /// whole domain: no commit of this run vouches yet for the tree it
    /// opened on.
    pub(super) fn opened() -> Checkpointing {
        Checkpointing {
            checkpointed: RefCell::new(Vec::new()),
            wrote: RefCell::new(Writes::Domain),
        }
    }

    /// What the next checkpoint stages: the paths the calls since the last
    /// checkpoint said they wrote, or the whole write domain when one of them
    /// could not say. Taken, so the checkpoint after it starts from what the
    /// calls after this one say. `Nothing` stages the domain as well:
    /// staging less than the domain is safe only on the calls' own
    /// account of what they wrote, and there is none to go by.
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

    /// A call that failed may have failed partway through its writes, and
    /// its own account of them no longer holds, so the next checkpoint walks
    /// the whole domain (`crates/runtime/Spec.lean` §8-45).
    fn widen(&self) {
        self.add(Writes::Domain);
    }
}

pub(super) struct Placing<'f> {
    bench: ToolBench,
    sieving: Sieving,
    run: RunId,
    /// Where the next call sits in this run. The key derives from this
    /// position rather than from the turn's millisecond stamp and the
    /// tool's name: a stamp is a clock, which determinism rule 7 forbids
    /// outright, and a name ignores the arguments, so two `read`s of two
    /// different files in one turn would be one key and the second would
    /// come back "this call was already made". A model reads that as a
    /// fault in itself.
    next: u64,
    /// What the run's own commands did: (passed, failed). It is the only
    /// evidence of "the tests passed" the city can observe without being
    /// told, and being told is what a mode is supposed to check.
    ran: (u32, u32),
    checkpointing: &'f Checkpointing,
    /// What the run was told it can reach: the one authority on what a
    /// `tool` `run` stands for (`crates/runtime/Spec.lean` §8-61).
    catalog: Arc<Mutex<Catalog>>,
    shown: ReadShown,
    rules: RuleDelivery,
    rule_candidates: Option<RuleCandidates>,
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
        }
    }

    /// Resolves each `tool` `run` against the run's own catalog. A face
    /// built without it resolves nothing, and a `run` then reaches no
    /// tool.
    #[must_use]
    pub(super) fn resolving(mut self, catalog: Arc<Mutex<Catalog>>) -> Placing<'f> {
        self.catalog = catalog;
        self
    }

    /// Supplies the same stateless candidate source the read tool was
    /// built with; it runs here, in call order, rather than in `invoke`.
    #[must_use]
    pub(super) fn ruling(mut self, candidates: RuleCandidates) -> Self {
        self.rule_candidates = Some(candidates);
        self
    }

    /// Shares the read tool's window evidence and starts rule delivery
    /// with the rooms a branched run inherited.
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

    /// The commands this run ran: (passed, failed).
    pub(super) fn ran(&self) -> (u32, u32) {
        self.ran
    }

    /// What the model reads of a result no command produced: a
    /// connector's answer is packaged for the window
    /// (`crates/runtime/Spec.lean` §8-27-10); every other tool shapes its own.
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

    /// Whether the call's tool says its results are about now. A tool
    /// the bench does not know claims nothing, which is `Timeless`.
    fn temporal_of(&self, call: &ToolCall) -> Temporal {
        self.bench
            .meta_of(&call.name)
            .map_or(Temporal::Timeless, |meta| meta.temporal)
    }

    /// Finalizes a read before its first result enters the idempotency
    /// cache. Other tools continue directly to the bench.
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
            (Some(context), Some(candidates)) => {
                candidates(&context.admitted, &context.landed, &self.rules.seen())
            }
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

    /// What the model reads back for what the bench decided.
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
                // Absence of `exit_code` is a failure, not a success: a
                // command a signal stopped returns no code at all, and
                // reading that as zero would let a halted build count as
                // tests that passed.
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
            // A replay is answered with what the first call answered,
            // sieved the same way. An error here would tell the model its
            // call failed when it succeeded (`crates/runtime/Spec.lean` §8-35). The
            // command counters are not touched: nothing ran this time.
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
        // A call the catalog will not resolve goes on as it came and
        // reaches `tool` itself, which answers a `list`, `find` or
        // `detail`, and for a `run` asks the catalog the same question and
        // answers its refusal: the refusal has one author.
        match self.catalog.lock() {
            Ok(catalog) => catalog.resolve_run(&call).unwrap_or(call),
            Err(_) => call,
        }
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        let at = self.next;
        self.next = at.saturating_add(1);
        // What the action is, is the tool face's to say (`crates/kernel/Spec.lean`
        // §8-23). Two identical calls at two positions are two keys and
        // both run; the same position replayed is one key, which is what
        // deduplication is for.
        let key = match call.action() {
            Ok(action) => IdemKey::derive(&self.run, Seq::new(at), &action),
            Err(unreadable) => {
                self.shown.admit(&call.id);
                return Admitted::Answered(Err(unreadable));
            }
        };
        match self.bench.clear(call, &key, t) {
            Ok(Clearance::Cleared(ticket)) => Admitted::Cleared(ticket),
            Ok(Clearance::Answered(decided)) => {
                let answered = self.answered(call, decided);
                self.shown.admit(&call.id);
                Admitted::Answered(answered)
            }
            Err(refused) => {
                self.checkpointing.widen();
                self.shown.admit(&call.id);
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
        self.shown.admit(&call.id);
        let decided = self
            .bench
            .account(ticket, answered)
            .inspect_err(|_| self.checkpointing.widen())?;
        self.answered(call, decided)
    }

    fn complete_wave(&mut self, shortened: &std::collections::BTreeSet<String>) {
        self.rules.complete_wave(shortened);
        self.shown.confirm(shortened);
    }
}
