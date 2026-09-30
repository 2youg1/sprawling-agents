// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who may answer, what is waiting, what has already been allowed, and
//! what each waiting item is holding up.
//!
//! One definition, folded the same way wherever it is held: the reading
//! side keeps one inside the `Views` a page is answered from, and the
//! judging side keeps one inside the worker that writes. Both are this
//! fold, and `what_a_worker_holds_is_what_a_restart_rebuilds` holds
//! them equal.

use kernel::event::Scope;
use kernel::event::record::{
    Admittance, AutonomyChanged, CityHalted, GoverningDocument, RulesChanged,
};
use kernel::{Address, AxError, EventKind, Payload, RunId};

/// The work an answered item was holding up.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockedJob {
    pub addr: Address,
    pub task: String,
    pub goal: String,
}

/// What a run was sent out to do. Read back from `run_started`, which is
/// the record that carries both halves.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sent {
    pub task: String,
    pub goal: String,
}

/// Who may answer, what is waiting, what has already been allowed, and
/// what each waiting item is holding up.
///
/// One fold, two readers. `Standing::fold` shows it every line of a
/// history it did not write; `RunWorker::govern` shows it every line the
/// running city writes. These were once two implementations that
/// happened to agree, and `set_admission` and `answer_approval` each
/// held a third by writing a field directly.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Governance {
    pub pending: std::collections::BTreeMap<String, kernel::ApprovalItem>,
    pub autonomy: kernel::Autonomy,
    pub granted: Vec<kernel::ClusterKey>,
    /// The scopes a person has shut. Folded from the ledger like
    /// everything else the panel shows, so a restarted city is still
    /// halted.
    pub halted: std::collections::BTreeSet<Scope>,
    /// What each governing document was last booked as, by scope and
    /// document. Folded from `rules_changed`, so a restarted city knows
    /// what the account already covers and books only what moved.
    pub rules: std::collections::BTreeMap<(Scope, GoverningDocument), kernel::B3Hash>,
    /// What each run was sent to do, by run.
    ///
    /// Never pruned, and one short entry per run - the same growth class
    /// as the tombstones `storage::HotView` keeps for evicted runs. It cannot
    /// be pruned on `run_frozen`: `freeze` writes inside the drive while
    /// the assembly records waiting items after it, so the ledger order
    /// is `run_started … run_frozen … approval_requested` and pruning
    /// there would drop the entry one line before it is read.
    sent: std::collections::BTreeMap<RunId, Sent>,
    /// What each waiting item is holding up, by approval id. Pruned when
    /// the item is answered, because an answered item holds nothing up.
    pub origins: std::collections::BTreeMap<String, BlockedJob>,
}

impl Governance {
    /// A city nobody has governed yet.
    pub fn empty() -> Governance {
        Governance {
            pending: std::collections::BTreeMap::new(),
            autonomy: kernel::consts_policy::AUTONOMY_DEFAULT,
            granted: Vec::new(),
            halted: std::collections::BTreeSet::new(),
            rules: std::collections::BTreeMap::new(),
            sent: std::collections::BTreeMap::new(),
            origins: std::collections::BTreeMap::new(),
        }
    }

    /// Registers what a run was sent to do.
    ///
    /// Two callers, one shape: the dispatch that is about to build the
    /// `RunPlan` out of these very values, and `absorb` reading them back
    /// out of `run_started`. `what_a_worker_holds_is_what_a_restart_rebuilds`
    /// is what holds the two to the same answer.
    pub fn sent(&mut self, run: RunId, task: &str, goal: &str) {
        self.sent.insert(
            run,
            Sent {
                task: task.to_owned(),
                goal: goal.to_owned(),
            },
        );
    }

    /// Folds one line in, from whichever of the two directions it came.
    ///
    /// The envelope arrives beside the payload because two of these arms
    /// need it: a run is named by the record it started, and a waiting
    /// item is held against the room that raised it.
    ///
    /// # Errors
    /// Refuses a line this build cannot read. The three governance
    /// payloads are read through the one kernel type each was written
    /// from, so this fold and `views::answered` cannot answer
    /// differently about the same line. Dropping such a line in silence
    /// would open a history written by another build as a city with work
    /// missing from its account, approvals nobody would ever be asked,
    /// and an allowance narrower than the person gave
    /// (sprawling-SPEC.md 8-74).
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "a few kinds move this fold; the rest of the event vocabulary does not"
    )]
    pub(crate) fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        payload: &Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::RunStarted => {
                let started = payload.read::<kernel::event::record::RunStarted>()?;
                self.sent(run, &started.task, &started.goal);
            }
            EventKind::ApprovalRequested => {
                let item = payload.read::<kernel::ApprovalItem>()?;
                // What this item is holding up, joined here rather than
                // hunted for later. Both halves come from the history:
                // the room from this record's envelope, the work from
                // the `run_started` of the run that raised it.
                if let (Some(addr), Some(sent)) = (addr, self.sent.get(&run)) {
                    self.origins.insert(
                        item.id.as_str().to_owned(),
                        BlockedJob {
                            addr: addr.clone(),
                            task: sent.task.clone(),
                            goal: sent.goal.clone(),
                        },
                    );
                }
                self.pending.insert(item.id.as_str().to_owned(), item);
            }
            EventKind::ApprovalResolved => {
                let ruled = payload.read::<kernel::event::record::ApprovalResolved>()?;
                self.pending.remove(ruled.id.as_str());
                self.origins.remove(ruled.id.as_str());
                // An allowance carries the group the person answered,
                // so a resumed run may act inside it without asking
                // again. A ruling whose group this build cannot read is
                // refused above rather than granted narrower or wider
                // than the person meant.
                if ruled.verdict == kernel::Ruling::Allow {
                    self.granted.push(ruled.cluster);
                }
            }
            EventKind::AutonomyChanged => {
                self.autonomy = payload.read::<AutonomyChanged>()?.autonomy;
            }
            // One kind for both directions: halting and releasing are
            // one fact changing value, and a second kind would let a
            // reader see a release with no halt before it.
            EventKind::CityHalted => {
                let shut = payload.read::<CityHalted>()?;
                match shut.state {
                    Admittance::Halted => {
                        self.halted.insert(shut.scope);
                    }
                    Admittance::Released => {
                        self.halted.remove(&shut.scope);
                    }
                }
            }
            EventKind::RulesChanged => {
                let changed = payload.read::<RulesChanged>()?;
                self.rules
                    .insert((changed.scope, changed.which), changed.after);
            }
            _ => {}
        }
        Ok(())
    }
}
