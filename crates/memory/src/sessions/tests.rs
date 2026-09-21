// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
use std::collections::BTreeMap;

use kernel::layout::CityLayout;
use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};

use super::*;
use crate::JsonlLedger;

fn draft(where_at: Option<&str>, kind: EventKind) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(1),
        who: "city".to_owned(),
        addr: where_at.map(|raw| Address::parse(raw).unwrap()),
        kind,
        data: Payload::empty(),
        ig: false,
    }
}

fn city(root: &Path) -> PathBuf {
    CityLayout::new(root).ledger()
}

fn slice_of(root: &Path, room: &str) -> PathBuf {
    CityLayout::new(root).session_slice(&Address::parse(room).unwrap())
}

/// Every file under the sessions of `webapp`, by relative path.
fn slices(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let sessions = slice_of(root, "webapp/room")
        .parent()
        .unwrap()
        .to_path_buf();
    let mut out = BTreeMap::new();
    if !sessions.exists() {
        return out;
    }
    let mut stack = vec![sessions.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(&sessions)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, std::fs::read(&path).unwrap());
        }
    }
    out
}

/// Two rooms' records and one the city wrote for itself, all through
/// the running Ledger, so the live projection is what is under test.
fn two_rooms_and_one_city_record(ledger_dir: &Path) {
    let (mut ledger, _) = JsonlLedger::open(ledger_dir, TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![
            draft(Some("webapp/api-rewrite"), EventKind::RunStarted),
            draft(Some("webapp/backend/db-migration"), EventKind::ToolCalled),
            draft(None, EventKind::CityInitialized),
        ])
        .unwrap();
    ledger
        .append_all(vec![draft(
            Some("webapp/api-rewrite"),
            EventKind::RunFrozen,
        )])
        .unwrap();
}

/// The sentence this module exists to keep: deleting the whole
/// `sessions/` directory has no consequence, and replaying the
/// Ledger lays the same bytes back down.
#[test]
fn deleting_the_sessions_directory_and_replaying_the_ledger_restores_the_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let ledger_dir = city(tmp.path());
    two_rooms_and_one_city_record(&ledger_dir);
    let before = slices(tmp.path());
    let mut names: Vec<&String> = before.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec!["api-rewrite.jsonl", "backend/db-migration.jsonl"],
        "one file per room, and none for a city record"
    );

    std::fs::remove_dir_all(slice_of(tmp.path(), "webapp/room").parent().unwrap()).unwrap();

    let mut rebuilt = Sessions::for_ledger(&ledger_dir).unwrap();
    for line in crate::read_raw_lines_at(&ledger_dir).unwrap() {
        rebuilt
            .absorb(&EventRecord::parse_line(&line).unwrap())
            .unwrap();
    }
    assert_eq!(
        slices(tmp.path()),
        before,
        "a deleted projection is laid down again byte for byte"
    );
}

/// A file this build did not write is not migrated: it is laid down
/// again from the Ledger, in full.
#[test]
fn a_slice_of_another_shape_is_laid_down_again() {
    let tmp = tempfile::tempdir().unwrap();
    let ledger_dir = city(tmp.path());
    two_rooms_and_one_city_record(&ledger_dir);
    let (mut ledger, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![draft(
            Some("webapp/api-rewrite"),
            EventKind::ModelCalled,
        )])
        .unwrap();
    drop(ledger);

    let expected = crate::read_raw_lines_at(&ledger_dir)
        .unwrap()
        .into_iter()
        .filter(|line| {
            EventRecord::parse_line(line).is_ok_and(|record| {
                record
                    .addr()
                    .is_some_and(|addr| addr.as_str() == "webapp/api-rewrite")
            })
        })
        .count();
    let path = slice_of(tmp.path(), "webapp/api-rewrite");
    let text = std::fs::read_to_string(&path).unwrap();
    let body: Vec<&str> = text.lines().skip(1).collect();
    std::fs::write(&path, format!("older-format 0\n{}\n", body.join("\n"))).unwrap();

    let mut rebuilt = Sessions::for_ledger(&ledger_dir).unwrap();
    let line = crate::read_raw_lines_at(&ledger_dir)
        .unwrap()
        .pop()
        .unwrap();
    rebuilt
        .absorb(&EventRecord::parse_line(&line).unwrap())
        .unwrap();
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(
        after.starts_with(&format!("{SLICE_MAGIC} 0\n")),
        "the rebuilt slice carries this build's header"
    );
    assert_eq!(
        after.lines().count(),
        expected.saturating_add(1),
        "every record the Ledger holds for the address is back"
    );
}

/// A slice left behind by a stopped process catches up from the
/// Ledger before the running one adds to it.
#[test]
fn a_slice_that_missed_the_tail_catches_up_before_it_is_appended_to() {
    let tmp = tempfile::tempdir().unwrap();
    let ledger_dir = city(tmp.path());
    let (mut ledger, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![draft(
            Some("webapp/api-rewrite"),
            EventKind::RunStarted,
        )])
        .unwrap();
    ledger
        .append_all(vec![draft(
            Some("webapp/api-rewrite"),
            EventKind::ToolCalled,
        )])
        .unwrap();
    drop(ledger);

    let path = slice_of(tmp.path(), "webapp/api-rewrite");
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    // The last line never reached the file: the shape of a power cut
    // between the Ledger's barrier and the projection's append.
    std::fs::write(&path, format!("{}\n", lines[..lines.len() - 1].join("\n"))).unwrap();

    let (mut reopened, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    reopened
        .append_all(vec![draft(
            Some("webapp/api-rewrite"),
            EventKind::RunFrozen,
        )])
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text.lines().count(),
        4,
        "the missed line and the new one are both there"
    );
}

/// A projection that cannot be written is reported, and the history
/// still takes the record.
#[test]
fn a_refused_projection_never_fails_the_ledger() {
    let tmp = tempfile::tempdir().unwrap();
    let ledger_dir = city(tmp.path());
    let slice = slice_of(tmp.path(), "webapp/room");
    let blocked = slice.parent().unwrap();
    std::fs::create_dir_all(blocked.parent().unwrap()).unwrap();
    std::fs::write(blocked, b"a file where the directory belongs").unwrap();

    let (mut ledger, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    let written = ledger.append_all(vec![draft(Some("webapp/room"), EventKind::RunStarted)]);
    assert!(written.is_ok(), "the Ledger's contract holds anyway");

    let line = crate::read_raw_lines_at(&ledger_dir).unwrap().remove(0);
    let record = EventRecord::parse_line(&line).unwrap();
    let mut sessions = Sessions::for_ledger(&ledger_dir).unwrap();
    let refused = sessions.absorb(&record);
    assert!(
        refused.is_err(),
        "a projection that could not be written says so"
    );
    assert!(blocked.is_file(), "the blocked path is left as it was");
}

/// The city's own record is filed nowhere. Its address names the city
/// rather than a building inside it, and a sessions directory named
/// after the city would stand inside the city root forever.
#[test]
fn the_city_s_own_record_gets_no_slice() {
    let tmp = tempfile::tempdir().unwrap();
    let ledger_dir = city(tmp.path());
    let name = tmp
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a temporary directory has a name")
        .to_owned();
    let (mut ledger, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![
            draft(Some(&name), EventKind::CityInitialized),
            draft(Some("webapp/room"), EventKind::RunStarted),
        ])
        .unwrap();
    drop(ledger);
    let filed = slices(tmp.path());
    assert_eq!(
        filed.keys().collect::<Vec<_>>(),
        vec![&"room.jsonl".to_owned()],
        "the room's slice is the only one laid down"
    );
    assert!(
        !tmp.path().join(&name).exists(),
        "no directory named after the city stands inside it"
    );
}
