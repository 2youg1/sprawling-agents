// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Resident-to-Resident protocols: Inbox, Workshop, fan-in, PR flow,
//! arbitration, triage.

mod arbiter;
mod archive_tool;
mod citation;
mod claim_effect;
mod claim_tool;
mod delegate_tool;
mod fanin;
mod goal_tool;
mod handback;
mod inbox;
mod pr;
mod pr_tool;
mod reply_wait;
mod signal_desk;
mod signal_tool;
mod steer;
mod triage;
mod workshop;
mod workshop_tool;

pub use arbiter::{Level, arbitrate, conflict_payload};
pub use archive_tool::{ARCHIVE_KINDS, ArchiveDesk, ArchiveEffect, ArchiveTool, Filer, Held};
pub use citation::{Citation, Reading};
pub use claim_effect::{ClaimEffect, still_true};
pub use claim_tool::{Booking, ClaimDesk, ClaimTool};
pub use delegate_tool::{DelegateDesk, DelegateTool, Delegated};
pub use fanin::{Artifact, Claim, FanIn, Joined, PrivateQuestion};
pub use goal_tool::{GoalBooking, GoalDesk, GoalTool, conflict_refusal};
pub use handback::Handback;
pub use inbox::{Inbox, Mailslot, SenderState, Signal};
pub use pr::{Open, Pr, Verified};
pub use pr_tool::{MergedRequest, OpenRequest, PrDesk, PrEffect, PrTool, RejectedRequest};
pub use reply_wait::WaitTurn;
pub use signal_desk::{Post, RoomMail, SignalDesk, SignalEffect};
pub use signal_tool::SignalTool;
pub use steer::{AgentSteer, Steer};
pub use triage::{Arrival, Landing, Reflex, Rule, Triage};
pub use workshop::{LaidOut, NodeContract, NodeId, Underway, Workshop};
pub use workshop_tool::{WorkshopDesk, WorkshopTool};
