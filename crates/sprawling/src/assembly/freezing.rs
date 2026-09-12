// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run is frozen with: its plan, and the handoff that resumes it.

use std::path::Path;

use kernel::Locator;
use kernel::{Address, AxCode, AxError, RunId};
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot, SegmentSource};
use runtime::run::RunPlan;

use super::{Assignment, Given, RunWorker, Site, Workbench, city_segment, held, name_of};

/// Bytes about to be frozen into one slot, and the documents they were
/// read from.
///
/// The two travel together everywhere: a segment whose sources were
/// computed separately would let the bytes and the account of them
/// drift, and the account is what a person reads to learn which of
/// their files their agent was actually given.
pub(super) struct Assembled {
    pub(super) bytes: Vec<u8>,
    pub(super) sources: Vec<SegmentSource>,
}

impl Assembled {
    /// Bytes this build carries rather than reads: no document on this
    /// disk holds them.
    pub(super) fn of_nothing(bytes: Vec<u8>) -> Assembled {
        Assembled {
            bytes,
            sources: Vec::new(),
        }
    }

    /// One document, whole.
    pub(super) fn of_one(addr: Address, bytes: Vec<u8>) -> Assembled {
        let kept = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        Assembled {
            sources: vec![SegmentSource::whole(addr, kept)],
            bytes,
        }
    }

    /// Appends one document's bytes after a blank line, and accounts it.
    ///
    /// The separator is written only between documents, so a segment
    /// that opens with a document does not open with a blank line.
    fn extend(&mut self, at: Address, bytes: &[u8]) {
        if !self.bytes.is_empty() {
            self.bytes.push(NEWLINE);
            self.bytes.push(NEWLINE);
        }
        self.bytes.extend_from_slice(bytes);
        self.sources.push(SegmentSource::whole(
            at,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }

    /// Freezes these bytes into one slot, keeping the account with
    /// them.
    fn freeze(self, slot: SegmentSlot) -> FrozenSegment {
        FrozenSegment::assembled(slot, self.bytes, self.sources)
    }
}

/// One line ending, named once. The prefix joins documents with it, and
/// a literal `10` at four call sites is four chances to mean something
/// else.
pub(super) const NEWLINE: u8 = 10;

/// The building slot: where this run stands, then the rules it stands
/// under.
///
/// `BUILDING.md` is here rather than left for the agent to open because
/// it is exactly as stable as the resident's own file — a person writes
/// it, no run may write it, and it does not move for the length of a
/// session. A rule an agent has to fetch before it can obey it is a rule
/// that gets obeyed one turn late, or not at all.
///
/// **`AGENTS.md` is here for that same sentence.** A building that was
/// adopted rather than raised arrives with conventions its authors wrote
/// for whoever works on that project, and a resident sent to fetch them
/// reads them after its first edit or never. Matched by exact name in
/// the building's own root: no parent's copy, no other spelling, nothing
/// resolved by search — an address a reader cannot predict from the rule
/// is an address nobody can check.
///
/// It follows the rules rather than leading them, and says so, because
/// the two can disagree: `BUILDING.md` is what this city enforces, while
/// the project's file was written for whatever harness its authors had.
/// A resident told to run a test suite by a building with no `exec` has
/// to know which of the two to believe.
/// # Errors
/// A path inside the city that this platform spells in a way the
/// address grammar refuses. The bytes would still be right, but the
/// account of where they came from would name a file nobody can open,
/// and a source row a reader cannot follow is worse than a refusal.
pub(super) fn building_segment(
    city_root: &Path,
    addr: &Address,
    building: &Address,
) -> Result<Assembled, AxError> {
    // The room's own address opens the slot and is not a document, so
    // it is accounted to nothing.
    let mut out = Assembled::of_nothing(addr.as_str().as_bytes().to_vec());
    let rules_at = city::building_path(city_root, building);
    if let Ok(rules) = std::fs::read(&rules_at) {
        out.extend(addressed(city_root, &rules_at)?, &rules);
    }
    // Written only when the file is: a heading over nothing would tell a
    // resident to follow conventions that do not exist.
    let conventions_at = city::agents_path(city_root, building);
    if let Ok(conventions) = std::fs::read(&conventions_at) {
        let mut block = format!(
            "## {}/{}\n\nHow work is done in this project, from the project itself. Where \
             this and the building's rules above disagree, the rules decide: they are what \
             the city enforces.\n\n",
            building.as_str(),
            city::AGENTS_FILE,
        )
        .into_bytes();
        block.extend_from_slice(&conventions);
        out.extend(addressed(city_root, &conventions_at)?, &block);
    }
    Ok(out)
}

/// The city's own spelling of a path inside it.
///
/// The path comes from `city`, which is the authority for where a
/// building's files sit; this only turns it back into the address a
/// reader opens it by, so nothing here repeats that layout.
///
/// # Errors
/// `E_INVALID_ARGS` for a path outside the city root or one the address
/// grammar refuses.
fn addressed(city_root: &Path, path: &Path) -> Result<Address, AxError> {
    let relative = path.strip_prefix(city_root).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "address a prefix source document",
            format!("{} is outside the city", path.display()),
        )
    })?;
    let spelled = relative
        .to_str()
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "address a prefix source document",
                format!("{} is not utf-8", relative.display()),
            )
        })?
        .replace('\\', "/");
    Address::parse(&spelled)
}

/// What a successor is told about the run it replaces: the room they
/// share, and the predecessor's id when there is one. The two travel
/// together because the transcript's address is made of both.
pub(super) struct Predecessor<'a> {
    pub(super) room: &'a Address,
    pub(super) run: Option<RunId>,
}

/// The run slot: what the last run in this room left behind, where its
/// conversation is, then what this one was asked for.
///
/// In that order, because the brief is what the agent acts on and the
/// last thing in a prompt is the thing that is read. A handoff that is
/// still its blank form contributes nothing and is left out. The
/// transcript line is one address, never the transcript itself: a
/// successor searches it for what it needs rather than rereading
/// everything its predecessor saw.
/// # Errors
/// Propagates a handoff that exists and cannot be read: a prefix that
/// left it out would tell the next session there was none.
pub(super) fn run_segment(
    city_root: &Path,
    brief: &city::RunBrief,
    before: Predecessor<'_>,
) -> Result<Vec<u8>, AxError> {
    let mut out = Vec::new();
    if let Some(handoff) = city::handoff(city_root, before.room)? {
        out.extend_from_slice(handoff.as_bytes());
        out.push(NEWLINE);
        out.push(NEWLINE);
    }
    if let Some(run) = before.run {
        let transcript = runtime::Transcript::address(before.room, run)?;
        out.extend_from_slice(
            format!("Predecessor transcript: {}\n\n", transcript.as_str()).as_bytes(),
        );
    }
    out.extend_from_slice(brief.segment_text().as_bytes());
    Ok(out)
}

pub(super) fn task_line(plan: &RunPlan) -> String {
    format!("{} at {}", plan.task, plan.addr.as_str())
}

impl RunWorker {
    /// Puts one desk's effects on the ledger, and only then makes the
    /// change they announce.
    ///
    /// This is the one door for all five of them. The change arrives as
    /// the return value of `Landing::record`, which appends first, so
    /// there is no expression in this file that reaches the city before
    /// the history - and the match below is exhaustive, so a new kind of
    /// change has to say here what it is.
    ///
    /// # Errors
    /// Propagates the first line the ledger refuses, in which case
    /// nothing changes; then whatever delivering, writing the plan or
    /// filing an entry reports.
    /// Freezes what this run is: the plan it drives on, and the handoff
    /// that says what to read to pick it up again.
    ///
    /// One phase because the two are one decision. The prefix is
    /// assembled for this plan and frozen with it, the handoff quotes
    /// the plan's own task line, and the job locator ends up in both -
    /// pinned in the store, so what a resumed run reads is the bytes the
    /// run segment carried rather than a file somebody edited since.
    ///
    /// # Errors
    /// Propagates a city or run segment that will not read, a norm on
    /// the must-read list that will not open, a store that will not take
    /// the bytes, and a handoff the runtime refuses.
    /// Puts every segment of a frozen prefix into the store, so the
    /// hashes `prompt_assembled` records can be read back as text.
    ///
    /// All four, not the two that happen to be pinned elsewhere. A hash
    /// with nothing behind it is a reference a person cannot follow,
    /// and "what was this agent told" is answerable only if every
    /// segment is there. Content addressing pays for it: one building's
    /// rules are one object however many runs are frozen under them.
    ///
    /// # Errors
    /// Propagates a store that will not take the bytes.
    fn intern_prefix(&mut self, prefix: &FrozenPrefix) -> Result<(), AxError> {
        for segment in prefix.segments() {
            self.cas
                .put(segment.bytes())
                .map_err(memory::MemoryError::into_ax)?;
        }
        Ok(())
    }

    pub(super) fn freeze_plan(
        &mut self,
        site: &Site,
        workbench: &Workbench,
        at: &Assignment,
        given: Given,
    ) -> Result<(RunPlan, runtime::handoff::Handoff), AxError> {
        let addr = &at.addr;
        let Given {
            brief,
            task,
            goal,
            job,
        } = given;
        let tools = held(&workbench.catalog, "read the catalog")?.tool_defs();
        // The catalog is part of the resident segment, not a fifth slot:
        // what a resident may reach is as much a standing fact about it
        // as who it is, and both are frozen for the whole run so the
        // prefix stays cacheable across the run's life. Assembled here
        // rather than earlier because the catalog does not exist until
        // the tools, the reading room and the mode are known.
        // The name a person typed when they started this session, which
        // is the last segment of the address they started it at. It
        // opens the resident slot rather than the city one: the city
        // segment is identical for every agent in the city and is
        // cached as such, and a name in it would make one copy per
        // agent of the largest stable block in the prompt.
        let mut resident = format!("Your name: {}\n\n", name_of(addr)).into_bytes();
        resident.extend_from_slice(&site.identity.segment_bytes());
        resident.push(NEWLINE);
        resident.extend_from_slice(
            held(&workbench.catalog, "read the catalog")?
                .render()
                .as_bytes(),
        );
        let prefix = FrozenPrefix::assemble(
            city_segment(&self.city_root)?.freeze(SegmentSlot::City),
            building_segment(&self.city_root, addr, site.building.addr())?
                .freeze(SegmentSlot::Building),
            Assembled::of_nothing(resident).freeze(SegmentSlot::Resident),
            Assembled::of_nothing(run_segment(
                &self.city_root,
                &brief,
                Predecessor {
                    room: addr,
                    run: at.predecessor(),
                },
            )?)
            .freeze(SegmentSlot::Run),
        )?;
        self.intern_prefix(&prefix)?;

        let plan = RunPlan {
            run: site.run_id,
            who: site.who.clone(),
            addr: addr.clone(),
            task,
            goal,
            // The city decided this when it laid the brief down; the
            // window and the run segment read the one decision.
            opening: match &brief {
                city::RunBrief::Job { .. } => runtime::Opening::FromJob,
                city::RunBrief::Principal => runtime::Opening::WithPerson,
            },
            job: job.clone(),
            parent: at.parent,
            predecessor: at.predecessor(),
            shape: runtime::turn::CallShape {
                model: site.model.id.clone(),
                // The model's own ceiling, not a number chosen here.
                // With thinking enabled this covers reasoning and answer
                // together, so a hand-picked value truncates runs for a
                // reason that appears nowhere in the account.
                max_tokens: site.model.max_output_tokens,
                // Stated in CONFIG.toml, resolved down the three-layer
                // ladder, and frozen with the run.
                effort: site.config.effort,
                // The window the reminder measures against: the book's
                // figure, which is what `status` already reports.
                context_tokens: site.model.context_tokens,
            },
            prefix,
            policy: site.rules.policy().clone(),
            tools,
            // From the catalog rather than from a second scan of the
            // shelves: the catalog already decided what this run can
            // reach, and reading the shelf again would answer that
            // question a second time at a different instant.
            skills: held(&workbench.catalog, "read the catalog")?.skill_pins(),
        };

        // The norms are filled by the machine: their addresses are known
        // when the building is laid out, and a model asked to recite the
        // list from memory gets one entry wrong eventually.
        let mut must_read = Vec::new();
        for norm in city::norms(&self.city_root, addr)? {
            let bytes = std::fs::read(&norm).map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "read a norm document for the must-read list",
                    format!("{}: {err}", norm.display()),
                )
                .with_recovery("fix the file's permissions, or remove it from the building")
            })?;
            let hash = self.cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
            must_read.push(Locator::parse(&format!("cas:b3-{hash}"))?);
        }
        must_read.push(job);
        // The address is a pure function of the room and the run, so
        // the handoff can name the transcript before a turn is taken.
        // It names the room's file and never the ledger: the ledger is
        // one chain for the whole city, under a subtree `read` refuses.
        let transcript = runtime::Transcript::address(addr, site.run_id)?;
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            task_line(&plan),
            "see the city roadmap".to_owned(),
            format!(
                "dispatched from the control surface; transcript at {}",
                transcript.as_str()
            ),
            "resume from the job locator".to_owned(),
        )?;
        Ok((plan, handoff))
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
