// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the backlog's scratch directories are held to: one name per
//! member of one backlog of one process, and never a name two of them
//! could both produce.

use super::Backlog;

/// Two backlogs opened separately must not name one directory.
///
/// Judged on the name rather than by racing two commands, because
/// the defect is not a race: both backlogs mint id 1 and the old
/// name was built from the id alone, so the collision is certain and
/// only the damage was timing-dependent. A city ran a command that
/// exited zero and produced nothing, because the other backlog's
/// `collect` had deleted the directory this one was writing into.
#[test]
fn two_backlogs_of_one_process_name_different_directories() {
    let mine = Backlog::new();
    let theirs = Backlog::new();
    let first = mine.mint().unwrap();
    assert_eq!(
        first,
        theirs.mint().unwrap(),
        "the ids collide by design; the directories are what must not"
    );
    assert_ne!(mine.scratch.dir(first), theirs.scratch.dir(first));
}

/// A clone is another handle onto one backlog, so it keeps naming
/// that backlog's directories rather than opening its own.
#[test]
fn a_clone_shares_the_directories_of_the_backlog_it_came_from() {
    let mine = Backlog::new();
    let id = mine.mint().unwrap();
    assert_eq!(mine.clone().scratch.dir(id), mine.scratch.dir(id));
}

/// One backlog's members never share a directory either.
#[test]
fn two_members_of_one_backlog_name_different_directories() {
    let mine = Backlog::new();
    assert_ne!(
        mine.scratch.dir(mine.mint().unwrap()),
        mine.scratch.dir(mine.mint().unwrap())
    );
}
