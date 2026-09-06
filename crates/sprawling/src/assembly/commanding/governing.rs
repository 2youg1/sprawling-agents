// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verbs a person sends, and what each one does to the city.

use kernel::{Address, AxCode, AxError, EventKind};
use kernel::{Ledger, Locator, Payload, RunId};

use super::super::{
    Assignment, RunWorker, autonomy_name, ledger_dir, now_ms, run_id_for, scope_name,
};

impl RunWorker {
    /// Shuts a scope to new work, or opens it again.
    ///
    /// Halting is admission control and nothing else: what is already
    /// running keeps running, because stopping a run in flight is
    /// `Cancel` and one verb that did two things would leave a person
    /// unable to ask for either alone. The refusal a halted city gives a
    /// dispatch says which scope refused and how to open it.
    pub(in crate::assembly) fn set_admission(
        &mut self,
        scope: &channels::HaltScope,
        state: &str,
    ) -> Result<(), AxError> {
        let name = scope_name(scope);
        let mut map = serde_json::Map::new();
        map.insert("scope".to_owned(), serde_json::Value::String(name.clone()));
        map.insert(
            "state".to_owned(),
            serde_json::Value::String(state.to_owned()),
        );
        // Recorded and nothing else: the fold reads `city_halted` and
        // sets the scope, in the one place a restart reads it too.
        self.record(EventKind::CityHalted, Payload::new(map)?)
    }

    /// Which shut scope covers this address, if one does.
    ///
    /// The city covers everything; a building or a workshop covers what
    /// is inside it, by the same containment `WriteDomain` uses, so
    /// "inside" means one thing in this city rather than two.
    pub(in crate::assembly) fn halted_by(&self, addr: &Address) -> Option<String> {
        if self.governance.halted.contains("city") {
            return Some("city".to_owned());
        }
        self.governance
            .halted
            .iter()
            .find(|name| {
                name.split_once(':')
                    .and_then(|(_, rest)| Address::parse(rest).ok())
                    .is_some_and(|scope| addr.is_within(&scope))
            })
            .cloned()
    }

    /// Records who may answer for a scope from now on.
    ///
    /// The current value is folded from the ledger rather than mirrored
    /// anywhere: what the panel shows is what a replay can verify.
    pub(in crate::assembly) fn set_autonomy(
        &mut self,
        scope: &channels::HaltScope,
        autonomy: kernel::Autonomy,
    ) -> Result<(), AxError> {
        let mut map = serde_json::Map::new();
        map.insert(
            "scope".to_owned(),
            serde_json::Value::String(scope_name(scope)),
        );
        map.insert(
            "autonomy".to_owned(),
            serde_json::Value::String(autonomy_name(&autonomy)),
        );
        self.record(EventKind::AutonomyChanged, Payload::new(map)?)
    }

    /// Answers one approval item.
    ///
    /// The rule lives in `kernel::approval`: humans answer everything, a
    /// resident answers only as the appointed delegate, never the three
    /// classes, never a tainted item, and never its own action. This
    /// method is where production consults it, so a delegate answering
    /// its own item is refused on the same path the person's answer
    /// takes rather than on a parallel one.
    pub(in crate::assembly) fn answer_approval(
        &mut self,
        item: &kernel::ApprovalId,
        verdict: kernel::PolicyVerdict,
        answerer: &kernel::Answerer,
    ) -> Result<(), AxError> {
        let pending = self
            .governance
            .pending
            .get(item.as_str())
            .cloned()
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "answer an approval",
                    item.as_str().to_owned(),
                )
                .with_recovery("this item is not waiting; the inbox lists the ones that are")
            })?;
        match kernel::may_answer(&self.governance.autonomy, &pending, answerer) {
            kernel::AnswerVerdict::May => {}
            refused => {
                return Err(AxError::failure(
                    AxCode::ApprovalDenied,
                    "answer an approval",
                    format!("{}: {refused:?}", item.as_str()),
                )
                .with_recovery(
                    "a resident answers only as the appointed delegate, and never its own \
                     action; this one is the person's to answer",
                ));
            }
        }
        let mut map = serde_json::Map::new();
        map.insert(
            "id".to_owned(),
            serde_json::Value::String(item.as_str().to_owned()),
        );
        map.insert(
            "verdict".to_owned(),
            serde_json::Value::String(format!("{verdict:?}").to_lowercase()),
        );
        // The cluster travels with the answer. The person was shown a
        // group and answered the group, so what a resumed run may do
        // without asking again is exactly that group — and reading it
        // back from the ledger is what makes the answer survive a
        // restart.
        map.insert(
            "cluster".to_owned(),
            serde_json::to_value(&pending.cluster_key).map_err(|err| {
                AxError::failure(AxCode::InvalidArgs, "record an answer", err.to_string())
            })?,
        );
        // Read before the answer is recorded, because recording it is
        // what closes the item: the fold drops the origin along with the
        // pending entry, and carrying the work on is this method's job
        // rather than the record's.
        let blocked = self.governance.origins.get(item.as_str()).cloned();
        self.record(EventKind::ApprovalResolved, Payload::new(map)?)?;
        // The cluster the person allowed and the closing of the item are
        // both folded from the line just written. Setting either field
        // here as well would be a second authority for a rule the fold
        // already holds.
        if verdict == kernel::PolicyVerdict::Allow
            && let Some(job) = blocked
        {
            // The work the person just unblocked carries on without
            // them: an answer that still needed the same command typed
            // again would make the inbox a place to acknowledge things
            // rather than a place to decide them. Under the ceiling it
            // was sent with, for the reason `knock` gives beside its own
            // budget - this is the same piece of work, interrupted.
            self.dispatch_in(
                Assignment {
                    // The room this work was interrupted in already
                    // exists: this is the same piece of work carrying
                    // on, not a session being opened.
                    addr: job.addr,
                    session: None,
                    effort: None,
                    mode: runtime::Mode::PlanGoal,
                    budget: job.budget,
                    parent: None,
                },
                job.task,
                job.goal,
            )
            .map(drop)?;
        }
        Ok(())
    }

    /// Records a fork: a new run identity branched from `from` at the
    /// event node `at_seq`. The lineage is the record; driving the new
    /// run is a Dispatch the person (or the interface) sends when ready.
    /// Prefix semantics are the replay layer's (`runtime::fork::prefix`);
    /// this method refuses a node the mother does not own.
    ///
    /// # Errors
    /// Refuses a node `from` does not own, and propagates whatever chain
    /// verification or the prefix bound reports.
    pub fn fork(
        &mut self,
        from: RunId,
        at_seq: kernel::Seq,
        addr: Option<Address>,
    ) -> Result<RunId, AxError> {
        let verified = runtime::replay::verify_ledger_dir(&ledger_dir(&self.city_root))?;
        // Validates the bound the same way a prefix build would.
        let _prefix = runtime::fork::prefix(&verified, at_seq)?;
        let index = usize::try_from(at_seq.value()).map_err(|_| {
            AxError::failure(
                AxCode::InvalidArgs,
                "fork",
                "at_seq does not fit this platform",
            )
        })?;
        let node_owner = verified.lines().get(index).and_then(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. } => Some(record.run()),
            _ => None,
        });
        if node_owner != Some(from) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "fork",
                format!("seq {} is not an event of run {from}", at_seq.value()),
            )
            .with_recovery("name an event node of the run you are forking"));
        }
        let fork_addr = match addr {
            Some(addr) => addr,
            None => verified
                .lines()
                .iter()
                .find_map(|line| match line {
                    runtime::replay::VerifiedLine::Known { record, .. }
                        if record.run() == from && record.kind() == EventKind::RunStarted =>
                    {
                        record.addr().cloned()
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "fork",
                        format!("{from} has no run_started in this ledger"),
                    )
                    .with_recovery("name the address to fork into")
                })?,
        };
        let now = now_ms()?;
        let new_run = run_id_for(
            &Locator::parse(&format!(
                "cas:b3-{}",
                kernel::B3Hash::digest(from.as_bytes())
            ))?,
            &fork_addr,
            now,
        );
        let draft = runtime::fork::fork_draft(from, at_seq, new_run, now, "owner".to_owned())?;
        self.ledger.append(draft)?;
        Ok(new_run)
    }
}
