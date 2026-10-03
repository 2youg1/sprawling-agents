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

/// The work an answered item was holding up: the room that raised it
/// and the run that was working there. What that run was sent to do is
/// read back from its `run_started` when the item is answered, so no
/// run's task stays in this process after it froze
/// (`crates/sprawling/spec/Accounting/Worker.lean` §8-25).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockedJob {
    pub addr: Address,
    pub run: RunId,
}

/// Who may answer, what is waiting, what has already been allowed, and
/// what each waiting item is holding up.
///
/// One fold, two readers. `Standing::fold` shows it every line of a
/// history it did not write; `RunWorker::govern` shows it every line the
/// running city writes. These were once two implementations that
/// happened to agree, and `set_admission` and `answer_approval` each
/// held a third by writing a field directly.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    /// What each waiting item is holding up, by approval id. Pruned when
    /// the item is answered, because an answered item holds nothing up.
    pub origins: std::collections::BTreeMap<String, BlockedJob>,
    /// The proposal cards waiting on a person, and the ones handled
    /// (`crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22, accounting D34).
    pub proposals: super::proposals::Proposals,
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
            origins: std::collections::BTreeMap::new(),
            proposals: super::proposals::Proposals::default(),
        }
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
    /// (`crates/sprawling/Spec.lean` §8-74).
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
            EventKind::ApprovalRequested => {
                let item = payload.read::<kernel::ApprovalItem>()?;
                // What this item is holding up: the room from this
                // record's envelope and the run that raised it. The work
                // itself is that run's `run_started`, read back when the
                // item is answered.
                if let Some(addr) = addr {
                    self.origins.insert(
                        item.id.as_str().to_owned(),
                        BlockedJob {
                            addr: addr.clone(),
                            run,
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
            EventKind::ProposalOffered => self.proposals.offered(run, payload)?,
            EventKind::ProposalDecided => self.proposals.decided(payload)?,
            EventKind::ProposalWithdrawn => self.proposals.withdrawn(payload)?,
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
