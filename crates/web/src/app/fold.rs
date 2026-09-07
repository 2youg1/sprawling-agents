// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One event forward: the fold that advances what the client believes.

use channels::{ApprovalItem, EventKind, EventRecord, RunId, Tokens, UsdMicros};

use crate::app::rows::{ProviderHealth, RunRow, gate_named_by, session_named_by};
use crate::app::snapshot::Snapshot;
use crate::phase::Phase;

impl Snapshot {
    pub fn apply(&mut self, event: &EventRecord) -> bool {
        if let Some(seen) = self.applied_through
            && event.seq() <= seen
        {
            return false;
        }
        self.applied_through = Some(event.seq());
        self.absorb(event);
        true
    }

    fn absorb(&mut self, event: &EventRecord) {
        let run = event.run();
        // Every record moves the run's high-water mark, whatever else it
        // does. Held here rather than in each arm below, because a fact
        // that is true of all of them written once cannot be forgotten
        // by the next arm somebody adds.
        if let Some(row) = self.runs.get_mut(&run) {
            row.last_seq = event.seq();
        }
        match event.kind() {
            EventKind::CityInitialized => self.city = event.addr().cloned(),
            EventKind::RunStarted | EventKind::RunForked => {
                let addr = event.addr().cloned();
                self.runs.insert(
                    run,
                    RunRow {
                        session: addr.as_ref().and_then(session_named_by),
                        addr,
                        parent: event
                            .data()
                            .as_map()
                            .get("parent")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|raw| RunId::parse(raw).ok()),
                        phase: Phase::Running,
                        steps_done: 0,
                        steps_planned: None,
                        started_at_seq: event.seq(),
                        last_seq: event.seq(),
                        turns: 0,
                        handoff_at_turn: None,
                        gate: None,
                        spent: None,
                        said: None,
                        task: event
                            .data()
                            .as_map()
                            .get("task")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                            .filter(|asked| !asked.trim().is_empty()),
                    },
                );
            }
            EventKind::ToolResult => {
                if let Some(row) = self.runs.get_mut(&run) {
                    row.steps_done = row.steps_done.saturating_add(1);
                }
            }
            EventKind::ModelReturned => {
                let said = event
                    .data()
                    .as_map()
                    .get("message")
                    .and_then(crate::turn::said_in);
                let billed = event
                    .data()
                    .as_map()
                    .get("billed_usd_micros")
                    .and_then(serde_json::Value::as_u64);
                // The call settled, so the buffer of what it was saying
                // is thrown away and the record speaks. This is the
                // whole of the rule that the settled text wins.
                self.saying.remove(&run);
                if let Some(row) = self.runs.get_mut(&run) {
                    row.steps_done = row.steps_done.saturating_add(1);
                    // A turn is a model call and whatever it caused, so
                    // the return is what closes one. Counting records
                    // here would make a turn that ran six tools six
                    // times longer than one that ran none.
                    row.turns = row.turns.saturating_add(1);
                    if let Some(said) = said {
                        row.said = Some(said);
                    }
                    if let Some(billed) = billed {
                        let held = row.spent.unwrap_or_default().get();
                        row.spent = Some(UsdMicros::new(held.saturating_add(billed)));
                    }
                }
                self.absorb_call(event);
            }
            // What a person asked for and did not get yet. The handoff is
            // the scene the next holder finds, and how long ago it was
            // written is what says whether that scene is still current.
            EventKind::HandoffWritten => {
                if let Some(row) = self.runs.get_mut(&run) {
                    row.handoff_at_turn = Some(row.turns);
                }
            }
            EventKind::ApprovalRequested => {
                // The payload is the item, because that is what the writer
                // serialised. A count would not survive a reload and could
                // not be grouped; the item can do both.
                let value = serde_json::Value::Object(event.data().as_map().clone());
                let mut gate = None;
                match serde_json::from_value::<ApprovalItem>(value) {
                    Ok(item) => {
                        gate = Some(gate_named_by(&item));
                        self.approvals.insert(item.id.as_str().to_owned(), item);
                    }
                    Err(_) => {
                        self.unreadable_approvals = self.unreadable_approvals.saturating_add(1);
                    }
                }
                if let Some(row) = self.runs.get_mut(&run) {
                    row.phase = Phase::Waiting;
                    row.gate = gate;
                }
            }
            EventKind::ApprovalResolved => {
                if let Some(id) = event
                    .data()
                    .as_map()
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                {
                    self.approvals.remove(id);
                }
                if let Some(row) = self.runs.get_mut(&run) {
                    row.phase = Phase::Running;
                    row.gate = None;
                }
            }
            // The ending is in the record: `kernel::completion` writes
            // which of the three it was, and a run a person stopped
            // reads differently from one that ran out of turns. The
            // client showed both as "frozen" while the word that told
            // them apart was in the payload it was already folding.
            EventKind::RunFrozen => {
                let ending = event
                    .data()
                    .as_map()
                    .get("completion")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                self.set_phase(&run, Phase::ended_as(ending.as_deref()));
            }
            EventKind::BudgetLimit => self.set_phase(&run, Phase::Frozen),
            EventKind::CityHalted => {
                self.halted = true;
                // Only what was still moving. A session that ended
                // yesterday did not stop because the city did, and
                // rewriting its ending would lose the one a person needs
                // in order to decide whether to pick it back up.
                for row in self.runs.values_mut() {
                    if row.phase.in_flight() {
                        row.phase = Phase::Halted;
                    }
                }
            }
            // Counted, not read. What waits in a room is the city's
            // answer; this only says that answer may have moved, so the
            // page showing a room asks again.
            EventKind::SignalEnqueued | EventKind::SignalConsumed => {
                self.signals_seen = self.signals_seen.saturating_add(1);
            }
            EventKind::LoginStarted => {
                self.login_url = event
                    .data()
                    .as_map()
                    .get("auth_url")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
            }
            // The login that just produced a credential is finished, so
            // the page stops asking a person to open a URL they already
            // opened.
            EventKind::SecretCaptured => self.login_url = None,
            EventKind::ProviderDegraded => self.provider = ProviderHealth::Degraded,
            EventKind::EndpointLost => self.provider = ProviderHealth::Lost,
            EventKind::EndpointAttached => self.provider = ProviderHealth::Healthy,
            EventKind::EndpointProbed => {
                let data = event.data();
                let map = data.as_map();
                if let Some(base_url) = map.get("base_url").and_then(serde_json::Value::as_str) {
                    let models = map
                        .get("models")
                        .and_then(serde_json::Value::as_array)
                        .map(|list| {
                            list.iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                    self.served.insert(base_url.to_owned(), models);
                }
            }
            // Skipped on purpose - see the module note. A view models what
            // it can show; the Ledger keeps everything either way.
            _ => {}
        }
    }

    fn set_phase(&mut self, run: &RunId, phase: Phase) {
        if let Some(row) = self.runs.get_mut(run) {
            row.phase = phase;
        }
    }

    /// Folds one model call's cost and consumption.
    ///
    /// `billed_usd_micros` is written only when the provider reported an
    /// amount or the pinned price sheet could settle one. Its absence is
    /// the fact that this call has no price anybody knows, and it is kept
    /// as its own count rather than added to the total as a zero.
    fn absorb_call(&mut self, event: &EventRecord) {
        let data = event.data().as_map();
        match data
            .get("billed_usd_micros")
            .and_then(serde_json::Value::as_u64)
        {
            Some(billed) => {
                self.spent = UsdMicros::new(self.spent.get().saturating_add(billed));
                self.usage.priced_calls = self.usage.priced_calls.saturating_add(1);
            }
            None => {
                self.usage.unpriced_calls = self.usage.unpriced_calls.saturating_add(1);
            }
        }
        let Some(usage) = data.get("usage").and_then(serde_json::Value::as_object) else {
            return;
        };
        let read = |key: &str| usage.get(key).and_then(serde_json::Value::as_u64);
        let add = |held: Tokens, more: Option<u64>| {
            Tokens::new(held.get().saturating_add(more.unwrap_or_default()))
        };
        self.usage.input = add(self.usage.input, read("input_tokens"));
        self.usage.output = add(self.usage.output, read("output_tokens"));
        self.usage.cache_read = add(self.usage.cache_read, read("cache_read_tokens"));
    }
}
