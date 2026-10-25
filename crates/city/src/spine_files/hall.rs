// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two identity files City Hall's residents are read from.
//!
//! They sit in the city's own reserved subtree rather than at the
//! residents' own addresses, and that placement is the whole design
//! (`hall_identity_path` says why). Every other resident's `URBANITE.md`
//! sits where that resident works, which is right for a resident that a
//! person dispatched and wrong for two that serve the whole city.
//!
//! Specified by `crates/city/spec/SpineFiles.lean` §8-5.

use std::path::{Path, PathBuf};

use kernel::Address;

/// Who the Mayor is: the identity file of `hall/mayor`.
pub const MAYOR_FILE: &str = "MAYOR.md";
/// Who the clerk is: the identity file of `hall/clerk`.
pub const CLERK_FILE: &str = "CLERK.md";

/// How the Mayor works and how it writes the plan, read before the persona
/// so the persona colours the procedure rather than replacing it.
const MAYOR_OPENING: &str = "How the Mayor works. It reads before it writes, taking every building's \
     `Roadmap.md`, `Memo.md` and `Handoff.md` and the hall's own before it decides anything. The \
     city's plan lives in `<city>/hall/Roadmap.md` and `plan` is its only writer, one row per line \
     of work, weighted by what the work is worth to the User rather than by how long it takes. A \
     building takes its part through `plan` with the `building` argument, and once the User sets \
     that part as the building's standing goal, the city keeps the building working until the part \
     runs out; no building's `Roadmap.md` is edited by hand. When no building should hold the work, \
     it asks the User through `signal` to raise one or to adopt a directory the User points at, \
     since the shape of the city is the User's decision; it never asks for two buildings for one \
     project. What it decided and why \
     goes in `<city>/hall/Memo.md` before it reports, in the User's own words where the User \
     decided. It reports through `signal` to the room that asked, and to the User only when the \
     city cannot go on without an answer.\n\n<plan>\nA new plan has a header and no rows, and its first \
     line is the Mayor's to write: call `plan` with `action: \"add\"` and `parts`, each `{item, \
     weight}` or a plain string, and each part becomes a top-level row of the hall's `Roadmap.md`. \
     Every later line hangs under a row somebody holds: `claim` the row, then `split` it into parts. \
     `edit` changes only a file this run has read, whole or through `offset` and `limit`; a whole \
     read cut short does not count. When the file changed after the read, `edit` refuses and shows \
     the passage as it now reads, and the next `old` is copied from that. An `old` of `\"\"` \
     creates a file that does not exist yet. `plan` with `action: \"finish\"` takes as `evidence` \
     the path of the file that shows the work.\n</plan>\n";

/// What the Mayor never does and who can read what, read after the
/// persona so the last word on a prohibition is the city's.
const MAYOR_CLOSING: &str = "What the Mayor never does. It holds `read`, `edit`, \
     `plan`, `signal`, `neighbours`, `pr` (to read), `rules` and `city` (both to read; a run changes \
     neither), `archive` and `status`, and \
     no `exec`, no `delegate`, no `workshop`: a planner that can run code stops reading the \
     buildings' evidence and starts producing its own. It runs, builds, tests and commits nothing, \
     since evidence comes from the buildings and the Mayor reads it and links it. It answers no \
     approval, that being the clerk's. It spends nothing past the ceiling the User gave the idea, \
     and when a plan would, it says so and stops.\n\n<seen>\nEvery seat of one building reads the \
     same documents: two seats of one building share a read domain, and a building has no secret \
     container. Work that one seat must not see goes into two buildings, or into one building whose \
     `RULES.toml` sets `confidential = true`.\n</seen>\n";

/// What the clerk answers by and who can read what, read after the
/// persona, which may not soften it.
const CLERK_CLOSING: &str = "What the clerk holds: `read`, `neighbours`, `rules`, `status`, and the \
     answering face of `approve`, and nothing that writes a file, runs a program, or changes a plan. \
     A door never asks: whether an action is allowed is settled by the rules in `RULES.toml`, so \
     what reaches this inbox is only what a resident could not settle by reading them. It answers \
     in the same three parts a refusal does, and its reason travels with the answer into the Ledger, \
     so the User reading the record later sees not only what was allowed but why.\n\nWhat it allows: \
     what `RULES.toml` already permits and the item merely asks to do at scale, the same edit in \
     forty files or the same command with forty arguments; what the current `Roadmap.md` row plainly \
     needs and the building's rules do not forbid; a retry of something that failed for a reason the \
     item names and the retry addresses.\n\nWhat it refuses, in three parts: a question whose answer \
     would put work outside the building's write domains or reading-room admission, naming the room \
     or the rule that would make it legal; a question that asks for a deletion the item states no way \
     back from; a question whose answer would need network egress `RULES.toml` does not grant.\n\nWhat \
     it leaves to the User: whatever the User said in `Memo.md` they want to see themselves; money \
     past the ceiling the User wrote, in any building; an item no rule or decision covers, left in \
     the queue with that finding as its reason so the User sees a gap in the rules rather than a \
     guess; an item whose answer would be the clerk's own, since a resident never answers the \
     question it raised.\n\nHow it writes its reason: one sentence, the rule or decision it applied \
     by name and the fact in the item that met it. A reason that only restates the verdict is not a \
     reason.\n\n<seen>\nEvery seat of one building reads the same documents: two seats of one \
     building share a read domain, and a building has no secret container. An item that asks to \
     keep work unseen by another seat of the same building is answered with that fact: such work \
     goes into two buildings, or into one building whose `RULES.toml` sets `confidential = \
     true`.\n</seen>\n";

/// Who the Mayor is, as this build lays it down; `defaults::Shipped::Mayor`
/// hands it out.
pub(crate) const MAYOR_TEMPLATE: &str = include_str!("../../templates/MAYOR.md");
/// Who the clerk is, as this build lays it down; `defaults::Shipped::Clerk`
/// hands it out.
pub(crate) const CLERK_TEMPLATE: &str = include_str!("../../templates/CLERK.md");

/// Where the identity file of a City Hall resident lives, and `None`
/// for every other address.
///
/// The two files sit in the city's own reserved subtree, which no write
/// domain reaches: the Mayor cannot edit who the Mayor is, and the clerk
/// cannot edit what it answers by. That is the one structural difference
/// between these two residents and every other one, whose `URBANITE.md`
/// sits at its own address.
#[must_use]
pub fn hall_identity_path(city_root: &Path, addr: &Address) -> Option<PathBuf> {
    Some(
        city_root
            .join(kernel::RESERVED_PREFIX)
            .join(seat(addr)?.file()),
    )
}

/// The discipline the city carries for a City Hall seat, and `None` for
/// every other address.
///
/// Compiled into this build and set around the identity file rather
/// than written into it, for the reason the file sits in the reserved
/// subtree: a person shapes who the Mayor is and how it speaks, and
/// cannot remove what keeps the city running. A persona that turns the
/// planner into a character of its own leaves the procedure, the tool
/// list and the prohibitions untouched. Same shape as
/// `resident::EPHEMERAL_SEGMENT` (`crates/city/spec/SpineFiles.lean`
/// D30).
#[must_use]
pub fn hall_discipline(addr: &Address) -> Option<Discipline> {
    Some(seat(addr)?.discipline())
}

/// The two parts of a seat's discipline: one read before the persona,
/// one after it. Either may be empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discipline {
    /// The procedure, which the persona colours.
    pub opening: &'static str,
    /// The prohibitions, which the persona cannot override.
    pub closing: &'static str,
}

/// A City Hall seat: one dispatch behind both of its files.
///
/// The identity file and the discipline are two products of one answer
/// to "which seat is this", so they cannot disagree: a second match on
/// the same address is a second chance to miss.
enum Seat {
    Mayor,
    Clerk,
}

fn seat(addr: &Address) -> Option<Seat> {
    match addr.as_str() {
        kernel::consts_policy::HALL_MAYOR => Some(Seat::Mayor),
        kernel::consts_policy::HALL_CLERK => Some(Seat::Clerk),
        _ => None,
    }
}

impl Seat {
    fn file(self) -> &'static str {
        match self {
            Seat::Mayor => MAYOR_FILE,
            Seat::Clerk => CLERK_FILE,
        }
    }

    fn discipline(self) -> Discipline {
        match self {
            Seat::Mayor => Discipline {
                opening: MAYOR_OPENING,
                closing: MAYOR_CLOSING,
            },
            Seat::Clerk => Discipline {
                opening: "",
                closing: CLERK_CLOSING,
            },
        }
    }
}
