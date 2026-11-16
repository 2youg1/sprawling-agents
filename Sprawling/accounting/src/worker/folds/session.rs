// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{Address, AxError, EventKind, Origin, RunId};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::worker) struct SessionOrigins {
    pending: BTreeMap<Address, Origin>,

    last_run: BTreeMap<Address, RunId>,

    carried: BTreeMap<Address, RunId>,

    policies: BTreeMap<Address, kernel::RunPolicy>,
}

impl SessionOrigins {
    pub(in crate::worker) fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        data: &kernel::Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::SessionOpened => {
                let Some(addr) = addr else {
                    return Ok(());
                };
                let opened = data.read::<kernel::event::record::SessionOpened>()?;
                match opened.from {
                    Some(origin) => {
                        self.pending.insert(addr.clone(), origin);
                    }
                    None => {
                        self.pending.remove(addr);
                    }
                }
                self.policies.remove(addr);
                match self.last_run.get(addr).filter(|_| opened.carried) {
                    Some(last) => {
                        self.carried.insert(addr.clone(), *last);
                    }
                    None => {
                        self.carried.remove(addr);
                    }
                }
                Ok(())
            }
            EventKind::RunStarted => {
                if let Some(addr) = addr {
                    self.started(addr, run);
                }
                Ok(())
            }
            EventKind::RunForked => {
                if let Some(addr) = addr {
                    self.spent(addr);
                }
                Ok(())
            }
            EventKind::RunPolicyChanged => {
                if let Some(addr) = addr {
                    let changed = data.read::<kernel::event::record::RunPolicyChanged>()?;
                    self.policies.insert(addr.clone(), changed.policy);
                }
                Ok(())
            }

            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::PromptAssembled
            | EventKind::PromptShapeCompared
            | EventKind::ModelCalled
            | EventKind::ModelReturned
            | EventKind::ToolCalled
            | EventKind::ToolResult
            | EventKind::ResultOffloaded
            | EventKind::GateChecked
            | EventKind::GateDenied
            | EventKind::CheckpointCommitted
            | EventKind::HandoffWritten
            | EventKind::SteerReceived
            | EventKind::CancelReceived
            | EventKind::WatchdogFired
            | EventKind::BudgetLimit
            | EventKind::RunFrozen
            | EventKind::LogTruncated
            | EventKind::SignalEnqueued
            | EventKind::SignalLanded
            | EventKind::SignalConsumed
            | EventKind::DraftHeld
            | EventKind::DraftResolved
            | EventKind::GoalRegistered
            | EventKind::GoalConflict
            | EventKind::ArbitrationVerdict
            | EventKind::RepairStarted
            | EventKind::RepairReused
            | EventKind::WorktreeOpened
            | EventKind::PrOpened
            | EventKind::PrMerged
            | EventKind::PrRejected
            | EventKind::RoadmapClaimed
            | EventKind::RoadmapFinished
            | EventKind::RoadmapReleased
            | EventKind::RoadmapSplit
            | EventKind::RoadmapBlocked
            | EventKind::PursuitChanged
            | EventKind::ApprovalRequested
            | EventKind::ApprovalResolved
            | EventKind::PolicyCreated
            | EventKind::PolicyRevoked
            | EventKind::TaintPromoted
            | EventKind::CrossBuildingTransfer
            | EventKind::CityHalted
            | EventKind::BackpressureShed
            | EventKind::DigestInvalidated
            | EventKind::EndpointAttached
            | EventKind::EndpointProbed
            | EventKind::EndpointLost
            | EventKind::ModelSelected
            | EventKind::ProviderDegraded
            | EventKind::LoginStarted
            | EventKind::EvalRun
            | EventKind::AssetArchived
            | EventKind::CredentialLent
            | EventKind::SecretCaptured
            | EventKind::SecretEgressBlocked
            | EventKind::FileDiscarded
            | EventKind::DiscardRestored
            | EventKind::AutonomyChanged
            | EventKind::WentBack
            | EventKind::FileRestored
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack
            | EventKind::CacheRenewed
            | EventKind::PrefixHeld
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted
            | EventKind::DocumentWritten
            | EventKind::ProposalOffered
            | EventKind::ProposalDecided
            | EventKind::ProposalWithdrawn
            | EventKind::SessionNamed
            | EventKind::SkillAudited
            | EventKind::SkillShelved
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded => Ok(()),
        }
    }

    pub(in crate::worker) fn get(&self, addr: &Address) -> Option<Origin> {
        self.pending.get(addr).copied()
    }

    pub(in crate::worker) fn policy(&self, addr: &Address) -> Option<kernel::RunPolicy> {
        self.policies.get(addr).copied()
    }

    pub(in crate::worker) fn carried_from(&self, addr: &Address) -> Option<RunId> {
        self.carried.get(addr).copied()
    }

    pub(in crate::worker) fn started(&mut self, addr: &Address, run: RunId) {
        self.carried.remove(addr);
        self.last_run.insert(addr.clone(), run);
    }

    pub(in crate::worker) fn spent(&mut self, addr: &Address) {
        self.pending.remove(addr);
    }
}
