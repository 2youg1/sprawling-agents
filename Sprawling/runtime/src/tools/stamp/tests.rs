// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The check derived from `crates/runtime/spec/Tools/Edit.lean` §8-64-2:
//! the model's steps replayed on [`ReadStamps`] and [`admission`], with
//! the theorem `an_admitted_edit_follows_its_runs_last_look` asserted at
//! every edit that lands.

use proptest::prelude::*;
use std::collections::BTreeMap;

use super::{Admission, Intent, ReadStamps, Stamp, admission};

const RUNS: usize = 3;
const PATHS: usize = 3;

/// One step of the model: `read r p`, `write p`, `edit r p creating unique`.
#[derive(Debug, Clone)]
enum Step {
    Read(usize, usize),
    Write(usize),
    Edit(usize, usize, Intent, bool),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0..RUNS, 0..PATHS).prop_map(|(r, p)| Step::Read(r, p)),
        (0..PATHS).prop_map(Step::Write),
        (
            0..RUNS,
            0..PATHS,
            prop_oneof![Just(Intent::Replace), Just(Intent::Create)],
            any::<bool>()
        )
            .prop_map(|(r, p, intent, unique)| Step::Edit(r, p, intent, unique)),
    ]
}

/// The model's `fresh`: every landing mints a stamp no earlier one had,
/// which is what renaming a staging file over the name does to identity.
fn minted(n: u64) -> Stamp {
    Stamp {
        len: n,
        #[cfg(unix)]
        written: (i64::try_from(n).unwrap(), 0),
        #[cfg(windows)]
        written: Some(n),
        volume: 1,
        file: n,
    }
}

fn address(p: usize) -> kernel::Address {
    kernel::Address::parse(&format!("work/f{p}.md")).unwrap()
}

proptest! {
    #[test]
    fn an_admitted_edit_follows_its_runs_last_look(trace in proptest::collection::vec(step(), 0..60)) {
        let runs: Vec<ReadStamps> = (0..RUNS).map(|_| ReadStamps::default()).collect();
        let mut content: BTreeMap<usize, Stamp> = BTreeMap::new();
        let mut fresh = 0u64;
        // The ghost the theorem speaks of: whether anybody else landed on
        // `p` since run `r` last observed it.
        let mut written_since_look = [[false; PATHS]; RUNS];
        for step in trace {
            match step {
                Step::Read(r, p) => {
                    let now = content.get(&p).cloned();
                    if now.is_some() {
                        written_since_look[r][p] = false;
                    }
                    runs[r].look(&address(p), now.clone(), now);
                }
                Step::Write(p) => {
                    fresh += 1;
                    content.insert(p, minted(fresh));
                    for seen in &mut written_since_look {
                        seen[p] = true;
                    }
                }
                Step::Edit(r, p, intent, unique) => {
                    let current = content.get(&p).cloned();
                    let seen = runs[r].last(&address(p));
                    match admission(seen.as_ref(), current.as_ref(), intent) {
                        Admission::Lands => {
                            if intent == Intent::Replace {
                                prop_assert!(
                                    !written_since_look[r][p],
                                    "run {r} landed on path {p} over a write it never saw"
                                );
                            }
                            if intent == Intent::Create || unique {
                                fresh += 1;
                                content.insert(p, minted(fresh));
                                for (other, seen) in written_since_look.iter_mut().enumerate() {
                                    seen[p] = other != r;
                                }
                                runs[r].look(&address(p), Some(minted(fresh)), Some(minted(fresh)));
                            }
                        }
                        Admission::Moved => {
                            // The refusal shows the passage as it now reads,
                            // and that counts as a look.
                            written_since_look[r][p] = false;
                            runs[r].look(&address(p), current.clone(), current);
                        }
                        Admission::Unread => prop_assert!(seen.is_none()),
                        Admission::Absent => prop_assert!(current.is_none()),
                        Admission::Present => prop_assert!(current.is_some()),
                    }
                }
            }
        }
    }
}

#[test]
fn a_look_between_two_different_stamps_records_nothing() {
    let run = ReadStamps::default();
    run.look(&address(0), Some(minted(1)), Some(minted(2)));
    assert_eq!(run.last(&address(0)), None);
}

#[test]
fn a_rewrite_of_the_same_length_is_a_different_stamp() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.txt");
    std::fs::write(&path, "one\n").unwrap();
    let before = Stamp::of(&path).unwrap().unwrap();
    let staged = dir.path().join("a.txt.staged");
    std::fs::write(&staged, "two\n").unwrap();
    std::fs::rename(&staged, &path).unwrap();
    let after = Stamp::of(&path).unwrap().unwrap();
    assert_ne!(before, after);
    assert_eq!(Stamp::of(&dir.path().join("absent")).unwrap(), None);
}
