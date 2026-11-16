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
mod tests;
