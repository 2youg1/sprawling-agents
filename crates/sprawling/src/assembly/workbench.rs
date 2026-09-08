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

use kernel::{Address, Model, RunId};
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
    /// Some when the building asks for review: the tree this run writes
    /// in, which goes back whether the run finished or failed.
    pub(super) lease: Option<memory::WorktreeLease>,
    pub(super) write_root: PathBuf,
    branch: Option<String>,
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
    pub(super) catalog: std::rc::Rc<std::cell::RefCell<runtime::Catalog>>,
    pub(super) bench: ToolBench,
    pub(super) delegates: std::rc::Rc<std::cell::RefCell<collab::DelegateDesk>>,
}

/// Who this run can reach: the residents beside it, and the sub-agents
/// under it.
///
/// Both are read by one answer - what `status` tells the model about the
/// city around it - so they arrive as one value rather than as two
/// parameters a caller could hand over half of.
pub(super) struct Reach<'a> {
    pub(super) seen: &'a city::Neighbourhood,
    pub(super) delegates: &'a std::rc::Rc<std::cell::RefCell<collab::DelegateDesk>>,
}

impl Site {
    /// What a checkpoint fence covers for this run.
    ///
    /// Under review the worktree is this run's alone, so everything that
    /// changed inside it is this run's to offer - the shelf entries it
    /// filed included, which sit at the building rather than in the
    /// room. Without a lease the fence stays on the room, which is the
    /// only place a run may write in the city itself.
    pub(super) fn fence_scope(&self, addr: &Address) -> String {
        if self.lease.is_some() {
            self.building.addr().as_str().to_owned()
        } else {
            addr.as_str().to_owned()
        }
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
    pub(super) signals: std::rc::Rc<std::cell::RefCell<collab::SignalDesk>>,
    pub(super) goals: std::rc::Rc<std::cell::RefCell<collab::GoalDesk>>,
    pub(super) plan: std::rc::Rc<std::cell::RefCell<collab::ClaimDesk>>,
    pub(super) shelf: std::rc::Rc<std::cell::RefCell<collab::ArchiveDesk>>,
    pub(super) pr: std::rc::Rc<std::cell::RefCell<collab::PrDesk>>,
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
    pub(super) budget: kernel::BudgetCap,
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
        budget_usd: situation.budget.usd,
        budget_tokens: situation.budget.tokens,
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
