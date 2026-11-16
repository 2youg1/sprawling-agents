// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::event::record::{CheckpointCommitted, CommitAttribution};
use kernel::{
    Address, AxError, EventKind, EventRecord, GitOid, RunId, Seq, SessionName, UsdMicros,
};

use super::prepared::{Prepared, unavailable, unavailable_because};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct CommitFacts {
    run: RunId,
    seq: Seq,
    at: kernel::TimeMs,
    actor: Address,
    chosen: storage::ModelChoice,

    previous: Option<wire::CommitAt>,

    b3: Option<kernel::B3Hash>,
}

impl CommitFacts {
    fn worked_under(&self, building: &Address) -> bool {
        let actor = self.actor.as_str();
        actor == building.as_str()
            || actor
                .strip_prefix(building.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    }

    pub(super) fn answer(
        &self,
        oid: GitOid,
        lineage: Vec<RunId>,
        spent: UsdMicros,
    ) -> wire::CommitAnswer {
        wire::CommitAnswer {
            spent,
            oid,
            run: self.run,
            actor: self.actor.clone(),
            model: self.chosen.id.clone(),
            effort: self.chosen.effort,
            seq: self.seq,
            at: self.at,
            session: session_of(&self.actor),
            lineage,
            previous: self.previous,
            parents: None,
            message: None,
            b3: self.b3,
        }
    }
}

impl super::holding::Views {
    pub(super) fn fold_commit(&mut self, record: &EventRecord) -> Result<(), AxError> {
        if let Some((oid, mut facts)) = commit_facts(record)? {
            facts.previous = match self.commits.get(&oid) {
                Some(first) => first.previous,
                None => self.last_commit.get(&facts.run).copied(),
            };
            self.last_commit.insert(
                facts.run,
                wire::CommitAt {
                    oid,
                    seq: facts.seq,
                },
            );
            self.commit_seqs.insert(facts.seq, oid);
            self.commits.insert(oid, facts);
        }
        Ok(())
    }

    pub(super) fn commits_answer(
        &self,
        building: Option<&Address>,
        before: Option<Seq>,
        limit: u32,
    ) -> Result<wire::CommitsAnswer, AxError> {
        let want = usize::try_from(limit.clamp(1, wire::HISTORY_MAX)).unwrap_or(usize::MAX);
        let mut rows = self
            .commit_seqs
            .range(..before.unwrap_or(Seq::new(u64::MAX)))
            .rev()
            .filter_map(|(_, oid)| self.commits.get(oid).map(|facts| (*oid, facts)))
            .filter(|(_, facts)| building.is_none_or(|at| facts.worked_under(at)))
            .take(want.saturating_add(1))
            .collect::<Vec<_>>();
        let more = rows.len() > want;
        rows.truncate(want);
        let mut page = Vec::with_capacity(rows.len());
        for (oid, facts) in rows {
            let spent = self.billed_to(facts.run)?;
            page.push(facts.answer(oid, self.lineage_of(facts.run), spent));
        }
        Ok(wire::CommitsAnswer {
            building: building.cloned(),
            before,
            commits: page,
            more,
        })
    }

    pub(super) fn fold_predecessor(&mut self, record: &EventRecord) {
        let Ok(started) = record.data().read::<kernel::event::record::RunStarted>() else {
            return;
        };
        if let Some(predecessor) = started.predecessor {
            self.predecessors.insert(record.run(), predecessor);
        }
    }

    pub(super) fn prepare_commit(&self, oid: GitOid) -> Prepared {
        let query = || format!("Commit({oid})");
        let Some(facts) = self.commits.get(&oid) else {
            return Prepared::Held(unavailable(query()));
        };
        match self.billed_to(facts.run) {
            Ok(spent) => {
                let commit = facts.answer(oid, self.lineage_of(facts.run), spent);
                Prepared::Commits(self.parents_ask(Settled::One(Box::new(commit))))
            }
            Err(stopped) => Prepared::Held(unavailable_because(query(), &stopped)),
        }
    }

    pub(super) fn prepare_commits(
        &self,
        building: Option<&Address>,
        before: Option<Seq>,
        limit: u32,
    ) -> Prepared {
        match self.commits_answer(building, before, limit) {
            Ok(page) => Prepared::Commits(self.parents_ask(Settled::Page(page))),
            Err(stopped) => Prepared::Held(unavailable_because(
                format!("Commits({before:?})"),
                &stopped,
            )),
        }
    }

    fn parents_ask(&self, settled: Settled) -> CommitsAsk {
        CommitsAsk {
            city_root: self.city_root.clone(),
            settled,
        }
    }

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

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a few kinds name a commit; the rest of the event vocabulary does not"
)]
pub(crate) fn commit_facts(record: &EventRecord) -> Result<Option<(GitOid, CommitFacts)>, AxError> {
    let data = record.data();
    let (oid, by) = match record.kind() {
        EventKind::CheckpointCommitted => match data.read::<CheckpointCommitted>()? {
            CheckpointCommitted::JobPinned { .. } => return Ok(None),
            CheckpointCommitted::Committed(commit) => (commit.oid, commit.by),
        },

        EventKind::PrMerged => {
            let by = data.read::<CommitAttribution>()?;
            let Some(oid) = data
                .as_map()
                .get("commit")
                .and_then(serde_json::Value::as_str)
                .and_then(GitOid::parse)
            else {
                return Ok(None);
            };
            (oid, by)
        }
        _ => return Ok(None),
    };
    let Some(actor) = record.addr().cloned() else {
        return Ok(None);
    };
    Ok(Some((
        oid,
        CommitFacts {
            run: record.run(),
            seq: record.seq(),
            at: record.t(),
            actor,
            chosen: storage::ModelChoice {
                id: by.model,
                effort: by.effort,
            },
            previous: None,

            b3: match record.canonical_line() {
                Ok(line) => Some(kernel::B3Hash::digest(&line)),
                Err(_unserialisable) => None,
            },
        },
    )))
}

pub struct CommitsAsk {
    city_root: PathBuf,
    settled: Settled,
}

enum Settled {
    One(Box<wire::CommitAnswer>),
    Page(wire::CommitsAnswer),
}

impl CommitsAsk {
    pub(super) fn read(self) -> wire::Answer {
        let CommitsAsk { city_root, settled } = self;
        match settled {
            Settled::One(mut commit) => {
                give_git_facts(&city_root, std::slice::from_mut(&mut *commit));
                wire::Answer::Commit(commit)
            }
            Settled::Page(mut page) => {
                give_git_facts(&city_root, &mut page.commits);
                wire::Answer::Commits(page)
            }
        }
    }
}

fn give_git_facts(city_root: &std::path::Path, commits: &mut [wire::CommitAnswer]) {
    give_parents(city_root, commits);
    give_messages(city_root, commits);
}

fn give_messages(city_root: &std::path::Path, commits: &mut [wire::CommitAnswer]) {
    let Ok(repository) = git2::Repository::open(city_root) else {
        return;
    };
    for commit in commits {
        commit.message = match git2::Oid::from_str(&commit.oid.to_string())
            .and_then(|oid| repository.find_commit(oid))
        {
            Ok(found) => match found.message() {
                Ok(said) => Some(said.to_owned()),
                Err(_not_utf8) => None,
            },
            Err(_not_read) => None,
        };
    }
}

fn give_parents(city_root: &std::path::Path, commits: &mut [wire::CommitAnswer]) {
    let oids: Vec<GitOid> = commits.iter().map(|commit| commit.oid).collect();
    let Ok(parents) = storage::parents_of(city_root, &oids) else {
        return;
    };
    for (commit, stated) in commits.iter_mut().zip(parents) {
        commit.parents = stated;
    }
}

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
        record_at(kind, data, 9, "lab/room1")
    }

    fn record_at(
        kind: EventKind,
        data: serde_json::Map<String, serde_json::Value>,
        seq: u64,
        addr: &str,
    ) -> EventRecord {
        record_by(kind, data, (seq, addr), RunId::from_bytes([4u8; 16]))
    }

    fn record_by(
        kind: EventKind,
        data: serde_json::Map<String, serde_json::Value>,
        (seq, addr): (u64, &str),
        run: RunId,
    ) -> EventRecord {
        let draft = EventDraft {
            run,
            t: TimeMs::new(7),
            who: "resident".to_owned(),
            addr: Some(Address::parse(addr).unwrap()),
            kind,
            data: Payload::new(data).unwrap(),
            ig: false,
        };
        EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
    }

    fn checkpointed(seq: u64, addr: &str) -> EventRecord {
        let mut data = serde_json::Map::new();
        data.insert(
            "oid".to_owned(),
            serde_json::Value::String(format!("{seq:02x}").repeat(20)),
        );
        record_at(EventKind::CheckpointCommitted, data, seq, addr)
    }

    fn checkpoint_by(seq: u64, oid: &str, run: u8) -> EventRecord {
        let mut data = serde_json::Map::new();
        data.insert("oid".to_owned(), serde_json::Value::String(oid.to_owned()));
        record_by(
            EventKind::CheckpointCommitted,
            data,
            (seq, "lab/room1"),
            RunId::from_bytes([run; 16]),
        )
    }

    fn oid_at(seq: u64) -> String {
        format!("{seq:02x}").repeat(20)
    }

    fn previous_of(views: &super::super::holding::Views, seq: u64) -> Option<wire::CommitAt> {
        views
            .commits_answer(None, None, 20)
            .unwrap()
            .commits
            .into_iter()
            .find(|commit| commit.seq == Seq::new(seq))
            .unwrap()
            .previous
    }

    #[test]
    fn a_commit_carries_the_b3_of_the_line_that_announced_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = super::super::holding::Views::new(dir.path());
        let line = checkpoint_by(3, &oid_at(3), 1);
        views.fold_commit(&line).unwrap();
        let page = views.commits_answer(None, None, 20).unwrap();
        let b3s: Vec<Option<B3Hash>> = page.commits.iter().map(|commit| commit.b3).collect();
        assert_eq!(
            b3s,
            vec![Some(B3Hash::digest(&line.canonical_line().unwrap()))]
        );
    }

    #[test]
    fn a_commit_names_the_one_its_run_announced_before_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = super::super::holding::Views::new(dir.path());
        for (seq, run) in [(3, 1), (5, 2), (8, 1)] {
            views
                .fold_commit(&checkpoint_by(seq, &oid_at(seq), run))
                .unwrap();
        }
        let first = wire::CommitAt {
            oid: GitOid::parse(&oid_at(3)).unwrap(),
            seq: Seq::new(3),
        };
        assert_eq!(
            (
                previous_of(&views, 8),
                previous_of(&views, 5),
                previous_of(&views, 3)
            ),
            (Some(first), None, None),
            "the other run's commit in between is not this run's previous one"
        );
    }

    #[test]
    fn a_page_of_commits_carries_each_ones_parents_from_git() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        let root = repo
            .commit(Some("HEAD"), &signature, &signature, "root", &tree, &[])
            .unwrap();
        let parent = repo.find_commit(root).unwrap();
        let child = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "child",
                &tree,
                &[&parent],
            )
            .unwrap();
        let mut views = super::super::holding::Views::new(dir.path());
        for (seq, oid) in [
            (3, root.to_string()),
            (5, child.to_string()),
            (8, oid_at(8)),
        ] {
            views.fold_commit(&checkpoint_by(seq, &oid, 1)).unwrap();
        }
        let asked = wire::Query::Commits {
            building: None,
            before: None,
            limit: 20,
        };
        let wire::Answer::Commits(page) = views.prepare(&asked).finish() else {
            panic!("a page of commits");
        };
        let parents: Vec<(u64, Option<Vec<String>>)> = page
            .commits
            .iter()
            .map(|commit| {
                let named = commit
                    .parents
                    .as_ref()
                    .map(|oids| oids.iter().map(ToString::to_string).collect());
                (commit.seq.value(), named)
            })
            .collect();
        assert_eq!(
            parents,
            vec![
                (8, None),
                (5, Some(vec![root.to_string()])),
                (3, Some(Vec::new())),
            ],
            "a commit the repository does not hold has no parents to read; a root has none"
        );
    }

    #[test]
    fn a_page_of_commits_carries_each_ones_message_from_git() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        let said = "checkpoint: wave 2

Sprawling-Actor: lab/room1
";
        let made = repo
            .commit(Some("HEAD"), &signature, &signature, said, &tree, &[])
            .unwrap();
        let mut views = super::super::holding::Views::new(dir.path());
        views
            .fold_commit(&checkpoint_by(3, &made.to_string(), 1))
            .unwrap();
        views.fold_commit(&checkpoint_by(5, &oid_at(5), 1)).unwrap();
        let asked = wire::Query::Commits {
            building: None,
            before: None,
            limit: 20,
        };
        let wire::Answer::Commits(page) = views.prepare(&asked).finish() else {
            panic!("a page of commits");
        };
        let messages: Vec<(u64, Option<String>)> = page
            .commits
            .into_iter()
            .map(|commit| (commit.seq.value(), commit.message))
            .collect();
        assert_eq!(messages, vec![(5, None), (3, Some(said.to_owned()))]);
    }

    fn addr(text: &str) -> Address {
        Address::parse(text).unwrap()
    }

    #[test]
    fn a_buildings_commits_are_listed_newest_first_and_paged_by_seq() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = super::super::holding::Views::new(dir.path());
        for record in [
            checkpointed(3, "lab/room1"),
            checkpointed(5, "hall/mayor"),
            checkpointed(8, "lab"),
        ] {
            views.fold_commit(&record).unwrap();
        }

        let lab = views.commits_answer(Some(&addr("lab")), None, 20).unwrap();
        let seqs: Vec<u64> = lab.commits.iter().map(|c| c.seq.value()).collect();
        assert_eq!(
            seqs,
            vec![8, 3],
            "the building root and its room, newest first"
        );
        assert!(!lab.more);
        assert_eq!(lab.building, Some(addr("lab")));

        let first = views.commits_answer(Some(&addr("lab")), None, 1).unwrap();
        assert_eq!(first.commits.len(), 1);
        assert!(first.more, "one page held back is a page");
        let next = views
            .commits_answer(Some(&addr("lab")), Some(Seq::new(8)), 1)
            .unwrap();
        assert_eq!(next.commits.first().map(|c| c.seq.value()), Some(3));
        assert!(!next.more);
        assert_eq!(next.before, Some(Seq::new(8)));

        let hall = views.commits_answer(Some(&addr("hall")), None, 20).unwrap();
        assert_eq!(hall.commits.len(), 1);
        assert_eq!(hall.commits.first().unwrap().actor.as_str(), "hall/mayor");

        assert!(
            views
                .commits_answer(Some(&addr("la")), None, 20)
                .unwrap()
                .commits
                .is_empty()
        );

        let all = views.commits_answer(None, None, 20).unwrap();
        assert_eq!(all.commits.len(), 3);
    }

    #[test]
    fn the_line_that_pins_a_job_is_not_a_commit() {
        let mut pin = serde_json::Map::new();
        pin.insert(
            "job".to_owned(),
            serde_json::Value::String(
                "file:hall/mayor@0123456789abcdef0123456789abcdef01234567".to_owned(),
            ),
        );
        assert!(
            commit_facts(&record(EventKind::CheckpointCommitted, pin))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_merge_names_its_commit_under_its_own_key() {
        let oid = "cd".repeat(20);
        let mut merged = serde_json::Map::new();
        merged.insert("commit".to_owned(), serde_json::Value::String(oid.clone()));
        let (found, facts) = commit_facts(&record(EventKind::PrMerged, merged))
            .unwrap()
            .unwrap();
        assert_eq!(found.to_string(), oid);

        let said = facts.answer(found, vec![facts.run], UsdMicros::new(0));
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
