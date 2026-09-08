// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which run wrote a commit, folded from the records that announced it.
//!
//! **Why the ledger and not git.** Every commit this city makes carries
//! five git trailers naming the run, the resident, the model, the effort
//! and the city. Those trailers are a projection for a reader outside
//! the city, and reading them back to answer this question would make
//! the projection the authority. A city exported and restored on another
//! machine, with no `.git` beside it, still answers from here.
//!
//! **Two kinds of record make a commit, and one impostor looks like
//! one.** A tool wave's fence and a base commit arrive as
//! `checkpoint_committed` under the key `oid`; the merge a review lands
//! arrives as `pr_merged` under the key `commit`. The line that opens a
//! dispatch is also a `checkpoint_committed`, but it pins a job rather
//! than a commit and names no oid at all — so what is asked of a record
//! is whether it names a commit, never which kind it is.

use kernel::{Address, EventKind, EventRecord, GitOid, RunId, Seq, SessionName};

/// What one commit's own record says about the run that made it.
///
/// Private fields with one production point: every field is read off a
/// record, so a set of facts about a commit nobody made cannot be
/// assembled here a field at a time.
pub(super) struct CommitFacts {
    run: RunId,
    seq: Seq,
    actor: Address,
    chosen: memory::ModelChoice,
}

impl CommitFacts {
    /// What a page or a person is handed. The lineage is this run
    /// first, then each run it replaced, back to the first.
    pub(super) fn answer(&self, oid: GitOid, lineage: Vec<RunId>) -> channels::CommitAnswer {
        channels::CommitAnswer {
            oid,
            run: self.run,
            actor: self.actor.clone(),
            model: self.chosen.id.clone(),
            effort: self.chosen.effort,
            seq: self.seq,
            session: session_of(&self.actor),
            lineage,
        }
    }
}

impl super::holding::Views {
    /// Files the commit one record announced, when it announced one.
    ///
    /// Both kinds of record are asked the same question - does this one
    /// name a commit - because the line that opens a dispatch is a
    /// `checkpoint_committed` that fences nothing.
    pub(super) fn fold_commit(&mut self, record: &EventRecord) {
        if let Some((oid, facts)) = commit_facts(record) {
            self.commits.insert(oid, facts);
        }
    }

    /// Files which run a `run_started` says this one replaced, when it
    /// says so.
    pub(super) fn fold_predecessor(&mut self, record: &EventRecord) {
        if let Some(predecessor) = memory::predecessor_of(record.data().as_map()) {
            self.predecessors.insert(record.run(), predecessor);
        }
    }

    /// Which run wrote this commit, and the runs that run succeeded.
    ///
    /// A commit this city never wrote is `Unavailable`, for the reason
    /// `Changes` gives: "I did not write it" and "it changed nothing"
    /// are different answers, and a reader acts differently on each.
    pub(super) fn commit_answer(&self, oid: GitOid) -> channels::Answer {
        match self.commits.get(&oid) {
            Some(facts) => channels::Answer::Commit(facts.answer(oid, self.lineage_of(facts.run))),
            None => channels::Answer::Unavailable {
                query: format!("Commit({oid})"),
            },
        }
    }

    /// The run and every predecessor behind it, nearest first. A chain
    /// that loops - which no ledger this city wrote can hold - stops at
    /// the first repeat rather than walking forever.
    fn lineage_of(&self, run: RunId) -> Vec<RunId> {
        let mut chain = vec![run];
        let mut at = run;
        while let Some(before) = self.predecessors.get(&at) {
            if chain.contains(before) {
                break;
            }
            chain.push(*before);
            at = *before;
        }
        chain
    }
}

/// The commit one record announced, if it announced one.
///
/// `None` for every record that names no commit — including the job pin
/// that opens a dispatch, which is a `checkpoint_committed` carrying a
/// job locator and no oid — for an oid this build cannot parse, and for
/// a record naming no address, which is a commit with no actor to
/// attribute it to. A projection skips what it cannot read rather than
/// inventing a row.
pub(super) fn commit_facts(record: &EventRecord) -> Option<(GitOid, CommitFacts)> {
    let key = match record.kind() {
        EventKind::CheckpointCommitted => "oid",
        EventKind::PrMerged => "commit",
        _ => return None,
    };
    let map = record.data().as_map();
    let oid = GitOid::parse(map.get(key)?.as_str()?)?;
    Some((
        oid,
        CommitFacts {
            run: record.run(),
            seq: record.seq(),
            actor: record.addr()?.clone(),
            chosen: memory::model_choice_of(map),
        },
    ))
}

/// What a person called this line of work: the room the actor worked in.
///
/// The room is the last segment of the address, because that is how
/// `city::open_room` put it there. A run dispatched at a building's own
/// address has no room, and therefore no session — which is a different
/// fact from a session nobody named.
fn session_of(actor: &Address) -> Option<SessionName> {
    let (_, room) = actor.as_str().rsplit_once('/')?;
    SessionName::parse(room).ok()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::{B3Hash, EventDraft, Payload, TimeMs};

    fn record(kind: EventKind, data: serde_json::Map<String, serde_json::Value>) -> EventRecord {
        let draft = EventDraft {
            run: RunId::from_bytes([4u8; 16]),
            t: TimeMs::new(7),
            who: "resident".to_owned(),
            addr: Some(Address::parse("lab/room1").unwrap()),
            kind,
            data: Payload::new(data).unwrap(),
            ig: false,
        };
        EventRecord::from_draft(draft, Seq::new(9), B3Hash::digest(b""))
    }

    #[test]
    fn the_line_that_pins_a_job_is_not_a_commit() {
        // The first thing a dispatch writes is a `checkpoint_committed`
        // naming the job it pinned. It fences nothing, so a fold that
        // asked only for the kind would file a commit that does not
        // exist.
        let mut pin = serde_json::Map::new();
        pin.insert(
            "job".to_owned(),
            serde_json::Value::String("cas:b3-00".to_owned()),
        );
        assert!(commit_facts(&record(EventKind::CheckpointCommitted, pin)).is_none());
    }

    #[test]
    fn a_merge_names_its_commit_under_its_own_key() {
        let oid = "cd".repeat(20);
        let mut merged = serde_json::Map::new();
        merged.insert("commit".to_owned(), serde_json::Value::String(oid.clone()));
        let (found, facts) = commit_facts(&record(EventKind::PrMerged, merged)).unwrap();
        assert_eq!(found.to_string(), oid);
        // A record written before the city put the model on the ledger
        // says nothing about it, and the answer says nothing back.
        let said = facts.answer(found, vec![facts.run]);
        assert_eq!(said.model, String::new());
        assert_eq!(said.lineage.len(), 1, "a first run is its own lineage");
        assert_eq!(said.effort, None);
        assert_eq!(said.session.unwrap().as_str(), "room1");
    }

    #[test]
    fn a_run_at_a_buildings_own_address_has_no_session() {
        assert!(session_of(&Address::parse("lab").unwrap()).is_none());
    }
}
