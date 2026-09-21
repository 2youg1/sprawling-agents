// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wire's words and the kernel's, translated one way each.

use kernel::EventRecord;
use kernel::{Address, AxCode, AxError};

/// What this agent is called: the last segment of its address, which is
/// the word a person typed into `call it` when they started the
/// session. Never the whole address — an agent addressed as its own
/// name reads more like somebody than like a path.
pub(super) fn name_of(addr: &Address) -> &str {
    addr.as_str().rsplit('/').next().unwrap_or(addr.as_str())
}

/// The discipline a wire frame names, in the word `runtime` evaluates.
///
/// Total, with no default: the wire used to carry free text that this
/// matched against four words and answered every other word with
/// planning, so a client that misspelled `experiment` got a planning
/// run and no refusal. The two sets now have the same five members,
/// and a mode added to either without the other is a compile error.
pub(super) fn mode_of(mode: channels::Mode) -> runtime::Mode {
    match mode {
        channels::Mode::PlanGoal => runtime::Mode::PlanGoal,
        channels::Mode::Up => runtime::Mode::Up,
        channels::Mode::Sc => runtime::Mode::Sc,
        channels::Mode::Ud => runtime::Mode::Ud,
        channels::Mode::Experiment => runtime::Mode::Experiment,
    }
}

/// Which governed document a wire frame names. Total: the two sets have
/// the same three members and neither owns the other, so the translation
/// is written once here rather than guessed at each call site.
pub(super) fn governed_of(which: channels::GovernedDocument) -> city::Governed {
    match which {
        channels::GovernedDocument::Mayor => city::Governed::Mayor,
        channels::GovernedDocument::Clerk => city::Governed::Clerk,
        channels::GovernedDocument::Preferences => city::Governed::Preferences,
    }
}

/// The answer to a verb this build spells on the wire and cannot perform.
///
/// One authority for the shape, because the six of them differ only in
/// which action failed, what it named, and what a person can do instead.
/// `WireMismatch` rather than `InvalidArgs`: the frame is well formed and
/// its arguments are sound, and what is absent is the executor.
///
/// A verb answered here must not appear as a control in the client. A
/// refusal is what the city owes a peer that asks anyway; it is not a
/// substitute for the button being gone.
pub(super) fn not_built(action: &'static str, subject: String, instead: &'static str) -> AxError {
    AxError::failure(AxCode::WireMismatch, action, subject).with_recovery(instead)
}

/// The building an address belongs to: its first segment.
pub(super) fn building_of(addr: &Address) -> Option<Address> {
    Address::parse(addr.as_str().split('/').next()?).ok()
}

/// Which plan node a `roadmap_*` record names.
pub(super) fn plan_node_of(record: &EventRecord) -> Option<kernel::NodeId> {
    kernel::NodeId::parse(record.data().as_map().get("node")?.as_str()?).ok()
}

/// Which scope a wire frame names, in the value the ledger records.
///
/// Total: the two sets have the same three members. How that scope is
/// spelled on a ledger line is [`kernel::event::Scope`]'s and is
/// written nowhere else, so a halt recorded by this city and a halt
/// read back by a restart cannot disagree about which scope it named.
pub(super) fn scope_of(scope: &channels::HaltScope) -> kernel::event::Scope {
    match scope {
        channels::HaltScope::City => kernel::event::Scope::City,
        channels::HaltScope::Building(addr) => kernel::event::Scope::Building(addr.clone()),
        channels::HaltScope::Workshop(addr) => kernel::event::Scope::Workshop(addr.clone()),
    }
}
