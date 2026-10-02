// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two identity files City Hall's residents are read from.
//!
//! They sit in the city's own reserved subtree rather than at the
//! residents' own addresses, and that placement is the whole design: no
//! write domain reaches the reserved subtree, so the Mayor cannot edit
//! who the Mayor is and the clerk cannot edit what it answers by. Every
//! other resident's `URBANITE.md` sits where that resident works, which
//! is right for a resident that a person dispatched and wrong for two
//! that serve the whole city.
//!
//! Specified by `crates/city/spec/SpineFiles.lean` §8-5.

use std::path::{Path, PathBuf};

use kernel::{Address, AxError};

use super::{storage, write_new};

/// Who the Mayor is: the identity file of `hall/mayor`.
pub const MAYOR_FILE: &str = "MAYOR.md";
/// Who the clerk is: the identity file of `hall/clerk`.
pub const CLERK_FILE: &str = "CLERK.md";

/// What the Mayor must do for the city to keep its plan.
const MAYOR_DISCIPLINE: &str = "How the Mayor works. It reads before it writes, taking every building's \
     `Roadmap.md`, `Memo.md` and `Handoff.md` and the hall's own before it decides anything. The \
     city's plan lives in `<city>/hall/Roadmap.md` and `plan` is its only writer, one row per line \
     of work, weighted by what the work is worth to the User rather than by how long it takes. A \
     building takes its part through `plan` with the `building` argument, then `pursue` keeps that \
     building working until the part runs out; no building's `Roadmap.md` is edited by hand. A \
     building is raised through `city` when none should hold the work, and a directory the User \
     points at is adopted; two buildings are never raised for one project. What it decided and why \
     goes in `<city>/hall/Memo.md` before it reports, in the User's own words where the User \
     decided. It reports through `signal` to the room that asked, and to the User only when the \
     city cannot go on without an answer.\n\nWhat the Mayor never does. It holds `read`, `edit`, \
     `plan`, `signal`, `neighbours`, `pr` (to read), `rules`, `city`, `archive` and `status`, and \
     no `exec`, no `delegate`, no `workshop`: a planner that can run code stops reading the \
     buildings' evidence and starts producing its own. It runs, builds, tests and commits nothing, \
     since evidence comes from the buildings and the Mayor reads it and links it. It answers no \
     approval, that being the clerk's. It spends nothing past the ceiling the User gave the idea, \
     and when a plan would, it says so and stops.\n";

/// What the clerk answers by, which a persona may not soften.
const CLERK_DISCIPLINE: &str = "What the clerk holds: `read`, `neighbours`, `rules`, `status`, and the \
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
     reason.\n";

const MAYOR_TEMPLATE: &str = include_str!("../../templates/MAYOR.md");
const CLERK_TEMPLATE: &str = include_str!("../../templates/CLERK.md");

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
/// Compiled into this build and appended after the identity file rather
/// than written into it, for the reason the file sits in the reserved
/// subtree: a person shapes who the Mayor is and how it speaks, and
/// cannot remove what keeps the city running. A persona that turns the
/// planner into a character of its own leaves the tool list and the
/// prohibitions untouched. Same shape as `resident::EPHEMERAL_SEGMENT`.
#[must_use]
pub fn hall_discipline(addr: &Address) -> Option<&'static str> {
    Some(seat(addr)?.discipline())
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

    fn discipline(self) -> &'static str {
        match self {
            Seat::Mayor => MAYOR_DISCIPLINE,
            Seat::Clerk => CLERK_DISCIPLINE,
        }
    }
}

/// Lays down the two identity files a city raises City Hall with.
///
/// Nothing is overwritten: a person who has edited either file keeps
/// what they wrote, whatever is run against the city afterwards.
///
/// # Errors
/// Propagates a reserved subtree that cannot be created or written.
pub fn lay_out_hall_identities(city_root: &Path) -> Result<(), AxError> {
    let governed = city_root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).map_err(|err| storage(&governed, &err))?;
    write_new(&governed.join(MAYOR_FILE), MAYOR_TEMPLATE)?;
    write_new(&governed.join(CLERK_FILE), CLERK_TEMPLATE)?;
    Ok(())
}
