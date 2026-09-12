// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one run works at: where it stands, and the bench it is given.

//!
//! The values are declared here and the methods that build them live
//! below: `standing` settles the site, `desks` opens what the run
//! writes to, `tools` lays out the bench, `servers` connects what the
//! building's configuration names, and `engine` is the machine's own
//! half of `exec`. A child reads its parent's private fields, so the
//! split opened none of them.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, Model, RunId};
use runtime::bench::ToolBench;

use super::autonomy_name;

mod desks;
mod engine;
mod servers;
mod standing;
mod tools;

/// Who checks a delegate's own done check. Not the delegate: the whole
/// point of `Claim::verified` is that a producer's verdict on its own
/// work is not verification.
pub(super) const CITY_VERIFIER: &str = "city";

/// Where one run stands before anything is built for it: the rules over
/// it, the model it may reach, who it runs as, and the tree it writes in.
///
/// One value rather than three, because the three readings of this phase
/// interlock: the lease is named after the run, the run's id is minted
/// after a credential renewal that may go to the network, and that
/// renewal is chosen by the building's own rules. Splitting them would
/// move a clock sample, and where the one sampling point lands is not
/// something a structural change may alter (ARCHITECTURE.md section 10).
pub(super) struct Site {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    /// City, building and resident layers, resolved once and frozen for
    /// the whole run.
    pub(super) config: kernel::FrozenConfig,
    pub(super) model: gateway::ModelEntry,
    pub(super) adapter: Option<Box<dyn Model + Send>>,
    pub(super) identity: city::Identity,
    pub(super) who: String,
    pub(super) run_id: RunId,
    /// The run this one replaces, when a resident succeeded itself.
    /// Signed into every fence this run raises, so a commit can be
    /// asked for its lineage.
    pub(super) predecessor: Option<RunId>,
    /// Some when the building asks for review: the tree this run writes
    /// in, which goes back whether the run finished or failed.
    pub(super) lease: Option<memory::WorktreeLease>,
    pub(super) write_root: PathBuf,
    branch: Option<String>,
    /// What survives a command's output in this run, resolved from the
    /// city's and the building's `FILTERS.toml` and frozen with the run
    /// (sprawling-SPEC 8-43).
    pub(super) filters: runtime::FilterTable,
    /// Carried from the endpoint this run was given, frozen with
    /// everything else the run was set up with.
    pub(super) retries: runtime::Retries,
}

/// What the model may see, what routes what it calls, and who it may
/// hand work down to.
///
/// `ToolBench` routes one call; this is the whole bench a run works at:
/// the catalogue the model was told about, the router behind it, and
/// the delegate desk two of the tools write to. The catalogue is kept
/// rather than the tool definitions it renders, so there is one answer
/// to what this run admits rather than a list and a copy of it.
pub(super) struct Workbench {
    pub(super) catalog: std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
    /// The bench, until the drive takes it. `Option` is the vehicle of
    /// that move and not a second state - the same handling `Site`
    /// gives its adapter - because a drive owns everything it runs on
    /// and may leave this thread with it (sprawling-SPEC.md 8-46-1).
    bench: Option<ToolBench>,
    pub(super) delegates: std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    /// Whether this run asked to be replaced, read when it concludes.
    pub(super) succession: std::sync::Arc<std::sync::Mutex<runtime::SuccessionDesk>>,
}

/// Who this run can reach: the residents beside it, and the sub-agents
/// under it.
///
/// Both are read by one answer - what `status` tells the model about the
/// city around it - so they arrive as one value rather than as two
/// parameters a caller could hand over half of.
pub(super) struct Reach<'a> {
    pub(super) seen: &'a city::Neighbourhood,
    pub(super) delegates: &'a std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
}

/// Takes a desk, or says why it cannot be taken.
///
/// The one reading of a poisoned lock in this assembly. Under
/// `panic = "abort"` a thread cannot die holding a desk, so the error
/// arm is unreachable in the shipped binary and still written, because
/// a test build unwinds and a desk left locked there is a fact worth a
/// stable code rather than a second panic (sprawling-SPEC 8-44).
pub(super) fn held<'a, T>(
    desk: &'a std::sync::Mutex<T>,
    what: &'static str,
) -> Result<std::sync::MutexGuard<'a, T>, AxError> {
    desk.lock().map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            what,
            "the desk was left locked by a thread that died",
        )
        .with_recovery("restart this city")
    })
}

impl Workbench {
    /// Hands the bench to the drive, once.
    ///
    /// # Errors
    /// Refuses a second ask, which is a defect in the caller rather
    /// than a state a run can be in: one dispatch drives once.
    pub(super) fn take_bench(&mut self) -> Result<ToolBench, AxError> {
        self.bench.take().ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "carry the bench into the drive",
                "this workbench has already been driven",
            )
            .with_recovery("report this: one dispatch lays out one bench and drives it once")
        })
    }
}

impl Site {
    /// What a checkpoint fence covers for this run.
    ///
    /// Under review the worktree is this run's alone, so everything that
    /// changed inside it is this run's to offer - the shelf entries it
    /// filed included, which sit at the building rather than in the
    /// room.
    ///
    /// Without a lease the fence is **the run's write domain**, which is
    /// the building's own subtree plus whatever else its `BUILDING.md`
    /// declares. It used to be the room, on the belief that a room is
    /// the only place a run may write in the city itself - and the gate
    /// never agreed: `city::policy::write_domain` defaults to the whole
    /// building, and City Hall's residents reach every document under
    /// theirs. Everything a run wrote in between was staged by no fence,
    /// so it reached no `changes` answer and no `file_discarded` record
    /// could restore it. memory-SPEC section 8-18 already said the fence
    /// is the write domain; this is the code agreeing with it.
    ///
    /// # Errors
    /// Propagates a building whose declared prefixes its own rules
    /// refuse.
    pub(super) fn fence_scope(&self) -> Result<Vec<String>, AxError> {
        if self.lease.is_some() {
            return Ok(vec![self.building.addr().as_str().to_owned()]);
        }
        Ok(self
            .rules
            .write_domain()?
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect())
    }
}

/// The desks one dispatch lends out, and takes back when the drive ends.
///
/// Grouped because they are lent and taken back together: five handles
/// passed side by side are five chances to take four of them back. Four
/// of them settle in one order in `settle_desks`; `pr` settles after,
/// once the run has something to show for itself, and it belongs here
/// all the same - what makes them one value is the lending, not the
/// settling.
pub(super) struct Desks {
    pub(super) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    pub(super) goals: std::sync::Arc<std::sync::Mutex<collab::GoalDesk>>,
    pub(super) plan: std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>,
    pub(super) shelf: std::sync::Arc<std::sync::Mutex<collab::ArchiveDesk>>,
    pub(super) pr: std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
    /// Where the shared plan lives, so the claims that survive are
    /// written back to the file they were checked against.
    pub(super) plan_path: PathBuf,
    /// What was already in the room's queue when it was lent out, which
    /// is what `status` reports as waiting. Read before the queue goes
    /// to the desk, so it is counted here or not at all.
    waiting: u32,
}

/// What a run can be told about itself at the moment it starts.
///
/// Every field here is read from something. Eight of them used to be
/// constants — the mode was always `plan_goal`, the write domain was
/// the room rather than what the building granted, and the budget, the
/// context limit and the locks were zeros. City.md tells a model to call
/// `status` for exactly those, so a model that obeyed got a row of
/// noughts and learnt not to ask again.
///
/// `ctx_used` and `children` stay at their empty values, and both are
/// true: nothing has been read at dispatch, and this city cannot yet
/// make a child. `worktree_disk` is zero because measuring a tree costs
/// a walk of it, and a number nobody has asked for is not worth one.
pub(super) struct Situation<'a> {
    pub(super) addr: &'a Address,
    pub(super) who: &'a str,
    signals_pending: u32,
    mode: runtime::Mode,
    write_domain: &'a kernel::WriteDomain,
    worktree: &'a Path,
    trust: &'a kernel::Autonomy,
    context_tokens: u64,
    locks: Vec<String>,
    neighbours: u32,
}

pub(super) fn status_snapshot(situation: Situation<'_>) -> runtime::StatusSnapshot {
    runtime::StatusSnapshot {
        who: situation.who.to_owned(),
        addr: situation.addr.clone(),
        mode: situation.mode,
        ctx_used: kernel::Tokens::default(),
        ctx_limit: kernel::Tokens::new(situation.context_tokens),
        trust: autonomy_name(situation.trust),
        write_domain: situation
            .write_domain
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect::<Vec<String>>()
            .join(", "),
        locks: situation.locks,
        worktree_path: situation.worktree.display().to_string(),
        worktree_disk: kernel::ByteLen::default(),
        signals_pending: situation.signals_pending,
        now: None,
        provider_mode: runtime::ProviderMode::Normal,
        neighbours: situation.neighbours,
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
mod tests;
