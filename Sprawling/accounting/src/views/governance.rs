// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Scope;
use kernel::event::record::{
    Admittance, AutonomyChanged, CityHalted, GoverningDocument, RulesChanged,
};
use kernel::{Address, AxError, EventKind, Payload, RunId};

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockedJob {
    pub addr: Address,
    pub run: RunId,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Governance {
    pub pending: std::collections::BTreeMap<String, kernel::ApprovalItem>,
    pub autonomy: kernel::Autonomy,
    pub granted: Vec<kernel::ClusterKey>,

    pub halted: std::collections::BTreeSet<Scope>,

    pub rules: std::collections::BTreeMap<(Scope, GoverningDocument), kernel::B3Hash>,

    pub origins: std::collections::BTreeMap<String, BlockedJob>,

    pub proposals: super::proposals::Proposals,
}

impl Governance {
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

                if ruled.verdict == kernel::Ruling::Allow {
                    self.granted.push(ruled.cluster);
                }
            }
            EventKind::AutonomyChanged => {
                self.autonomy = payload.read::<AutonomyChanged>()?.autonomy;
            }

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
