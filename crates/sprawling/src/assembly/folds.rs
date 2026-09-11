// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::{Address, AxError, EventKind};
use kernel::{EventRecord, Payload, RunId};

use crate::views::Views;

use super::{Entrance, read_autonomy};

mod collaboration;

use collaboration::CollaborationFold;
pub(super) use collaboration::{Collaboration, artifact_of, new_inbox};

/// Rebuilds what the worker answers approvals from. Same disposability
/// as every other view: delete it, replay, get the same answers.
///
/// # Errors
/// Propagates chain verification failures.
/// The work an answered item was holding up.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct BlockedJob {
    pub(super) addr: Address,
    pub(super) task: String,
    pub(super) goal: String,
}

/// What a run was sent out to do. Read back from `run_started`, which is
/// the record that carries both halves.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Sent {
    pub(super) task: String,
    pub(super) goal: String,
}

/// Who may answer, what is waiting, what has already been allowed, and
/// what each waiting item is holding up.
///
/// One fold, two readers. `Standing::fold` shows it every line of a
/// history it did not write; `RunWorker::govern` shows it every line the
/// running city writes. These were once two implementations that
/// happened to agree, and `set_admission` and `answer_approval` each
/// held a third by writing a field directly.
pub(super) struct Governance {
    pub(super) pending: std::collections::BTreeMap<String, kernel::ApprovalItem>,
    pub(super) autonomy: kernel::Autonomy,
    pub(super) granted: Vec<kernel::ClusterKey>,
    /// The scopes a person has shut, by the name `scope_name` gives
    /// them. Folded from the ledger like everything else the panel
    /// shows, so a restarted city is still halted.
    pub(super) halted: std::collections::BTreeSet<String>,
    /// What each run was sent to do, by run.
    ///
    /// Never pruned, and one short entry per run - the same growth class
    /// as `memory::HotView`, which is also one entry per run. It cannot
    /// be pruned on `run_frozen`: `freeze` writes inside the drive while
    /// the assembly records waiting items after it, so the ledger order
    /// is `run_started … run_frozen … approval_requested` and pruning
    /// there would drop the entry one line before it is read.
    sent: std::collections::BTreeMap<RunId, Sent>,
    /// What each waiting item is holding up, by approval id. Pruned when
    /// the item is answered, because an answered item holds nothing up.
    pub(super) origins: std::collections::BTreeMap<String, BlockedJob>,
}

impl Governance {
    /// A city nobody has governed yet.
    fn empty() -> Governance {
        Governance {
            pending: std::collections::BTreeMap::new(),
            autonomy: kernel::consts_policy::AUTONOMY_DEFAULT,
            granted: Vec::new(),
            halted: std::collections::BTreeSet::new(),
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
    pub(super) fn sent(&mut self, run: RunId, task: &str, goal: &str) {
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
    pub(super) fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        payload: &Payload,
    ) {
        let data = payload.as_map();
        let text = |key: &str| {
            data.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        match kind {
            EventKind::RunStarted => {
                self.sent(run, &text("task"), &text("goal"));
            }
            EventKind::ApprovalRequested => {
                let Ok(item) = serde_json::from_value::<kernel::ApprovalItem>(
                    serde_json::Value::Object(data.clone()),
                ) else {
                    return;
                };
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
                if let Some(id) = data.get("id").and_then(serde_json::Value::as_str) {
                    self.pending.remove(id);
                    self.origins.remove(id);
                }
                let allowed =
                    data.get("verdict").and_then(serde_json::Value::as_str) == Some("allow");
                if allowed
                    && let Some(cluster) = data.get("cluster")
                    && let Ok(key) = serde_json::from_value::<kernel::ClusterKey>(cluster.clone())
                {
                    self.granted.push(key);
                }
            }
            EventKind::AutonomyChanged => {
                if let Some(name) = data.get("autonomy").and_then(serde_json::Value::as_str) {
                    self.autonomy = read_autonomy(name);
                }
            }
            // One kind for both directions: halting and releasing are
            // one fact changing value, and a second kind would let a
            // reader see a release with no halt before it.
            EventKind::CityHalted => {
                let Some(scope) = data.get("scope").and_then(serde_json::Value::as_str) else {
                    return;
                };
                if data.get("state").and_then(serde_json::Value::as_str) == Some(HALTED) {
                    self.halted.insert(scope.to_owned());
                } else {
                    self.halted.remove(scope);
                }
            }
            _ => {}
        }
    }
}

/// The value of a halt record's `state` field when the scope is shut.
pub(crate) const HALTED: &str = "halted";

/// And when it is open again.
pub(crate) const RELEASED: &str = "released";

/// Everything a worker inherits from a history it did not write.
pub(crate) struct Standing {
    pub(crate) book: gateway::EndpointBook,
    pub(super) governance: Governance,
    pub(super) collaboration: Collaboration,
    /// The keys of the commands this history already carried out. On
    /// the same pass as the other three: recognising a repeat across a
    /// restart must not cost a second read of the whole history.
    pub(super) entrance: Entrance,
}

impl Standing {
    /// One verified pass, three folds.
    ///
    /// Until this existed the three were three functions, and opening a
    /// worker read, parsed and chain-verified the same bytes three times
    /// over to answer three questions about them. The answers never
    /// disagreed, which `what_a_worker_holds_is_what_a_restart_rebuilds`
    /// is what now holds, so the two extra passes bought nothing but the
    /// time and the memory of reading a whole history twice more.
    ///
    /// A line is parsed once here and shown to each fold. Verification
    /// stays where it was: a history that does not verify is not one any
    /// of these three views may be built from.
    ///
    /// # Errors
    /// Propagates chain verification and whatever a fold says about a
    /// payload it cannot read.
    pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        let mut book = gateway::EndpointBook::new();
        let mut governance = Governance::empty();
        let mut collaboration = CollaborationFold::default();
        let mut entrance = Entrance::default();
        if ledger_dir.exists() {
            let verified = runtime::replay::verify_ledger_dir(ledger_dir)?;
            for line in verified.raw_lines() {
                let record = EventRecord::parse_line(line)?;
                book.apply(&record)?;
                governance.absorb(record.kind(), record.run(), record.addr(), record.data());
                collaboration.absorb(&record)?;
                entrance.absorb(record.data());
            }
        }
        Ok(Standing {
            book,
            governance,
            collaboration: collaboration.settle()?,
            entrance,
        })
    }
}

/// Rebuilds the views from the ledger on disk. This is the disposability
/// of a projection exercised on every start: nothing is persisted, and
/// the answer is the same as if the process had been running all along.
///
/// # Errors
/// Propagates chain verification failures; a city whose history does not
/// verify is not one whose views should be served.
pub(crate) fn rebuild_views(ledger_dir: &Path) -> Result<Views, AxError> {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir)?;
    let city_root = ledger_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(ledger_dir);
    let mut views = Views::new(city_root);
    for line in verified.raw_lines() {
        let record = EventRecord::parse_line(line)?;
        views.apply(&record)?;
    }
    Ok(views)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
