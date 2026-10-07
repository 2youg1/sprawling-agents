// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which room a named session works in, and how a new one comes into
//! being (`crates/city/spec/Room.lean` §8-13).
//!
//! A room is where one session keeps its files, which is what
//! `ARCHITECTURE.md` section 6 says of `JOB.md`. This module opens a
//! room for a session that does not name one, so two dispatches a person
//! types by hand land in two addresses and the second does not write
//! over the first one's work.

use std::path::Path;

use kernel::layout::{ARCHIVE_DIR, CityLayout};
use kernel::{Address, AxCode, AxError, SessionName};

/// How many suffixed names are tried before a person is asked to pick
/// another word. High enough that nobody meets it by working, low
/// enough that a wrong loop stops rather than fills a disk.
const SUFFIX_LIMIT: u32 = 999;

/// The rooms this building has, in address order.
///
/// One authority for what counts as a room, read by the building page
/// and by the city-level readers alike. A room is a direct subdirectory whose name an
/// address can hold, which is exactly how rooms come into being -
/// [`open`] and delegation both create one level down. Dot directories
/// are not rooms (that is what keeps the reserved subtree out), and
/// neither is the archive, which is where a building keeps what it
/// remembers rather than somebody to talk to.
///
/// A building with no directory yet has no rooms; that is an answer
/// rather than a failure.
///
/// # Errors
/// Propagates a directory that exists and cannot be read. A caller that
/// would rather show what it could read says so at its own call site.
pub fn all(city_root: &Path, building: &Address) -> Result<Vec<Address>, AxError> {
    let root = CityLayout::new(city_root).scope(building);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let unreadable = |err: &std::io::Error| {
        AxError::failure(
            AxCode::StorageFatal,
            "list the rooms of a building",
            format!("{}: {err}", root.display()),
        )
        .with_recovery("check the building directory is readable")
    };
    let entries = std::fs::read_dir(&root).map_err(|err| unreadable(&err))?;
    let mut out = Vec::new();
    for entry in entries {
        // An entry this process cannot stat is reported rather than
        // skipped: a room dropped from the list is a room a person is
        // told does not exist.
        let entry = entry.map_err(|err| unreadable(&err))?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == ARCHIVE_DIR {
            continue;
        }
        if let Ok(addr) = Address::parse(&format!("{}/{name}", building.as_str())) {
            out.push(addr);
        }
    }
    // Two machines reading one directory must answer the same thing;
    // `read_dir` order is the filesystem's, not ours.
    out.sort_by(|left: &Address, right: &Address| left.as_str().cmp(right.as_str()));
    Ok(out)
}

/// Opens a room under `building` for a session called `name`.
///
/// A name that is already taken is suffixed rather than reused:
/// `refactor`, then `refactor-2`. Reusing it would put two sessions that
/// share nothing but a common word into one set of files, which is the
/// defect this module exists to remove. Continuing an earlier session is
/// dispatching to the room it already has, and the interface offers that
/// as a choice rather than as a coincidence of spelling.
///
/// # Errors
/// Refuses a name that cannot follow the building as one address
/// segment, and a building whose rooms are all taken up to the suffix
/// limit. Propagates a directory that cannot be created.
pub fn open(city_root: &Path, building: &Address, name: &SessionName) -> Result<Address, AxError> {
    for attempt in 1..=SUFFIX_LIMIT {
        let candidate = if attempt == 1 {
            format!("{}/{}", building.as_str(), name.as_str())
        } else {
            format!("{}/{}-{attempt}", building.as_str(), name.as_str())
        };
        let addr = Address::parse(&candidate)?;
        let dir = CityLayout::new(city_root).scope(&addr);
        // `create_dir` rather than exists-then-create: the question and
        // the answer are one operation, so two dispatches in the same
        // millisecond cannot both be told the name was free.
        match std::fs::create_dir_all(dir.parent().unwrap_or(city_root))
            .and_then(|()| std::fs::create_dir(&dir))
        {
            // What one session works on stays on this machine, and the
            // room says so itself: at the moment a building is raised
            // no room exists to be named, so the building's own file
            // cannot state this rule (`crates/city/spec/Gitignore.lean` §8-21).
            Ok(()) => {
                crate::gitignore::seal_room(&dir)?;
                // The handoff is this room's: what one session leaves
                // for its own next run, never for the building.
                crate::spine_files::lay_out_handoff(&dir, &addr)?;
                return Ok(addr);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(AxError::failure(
                    AxCode::StorageFatal,
                    "open a room for this session",
                    format!("{}: {err}", dir.display()),
                )
                .with_recovery("check the city directory is writable"));
            }
        }
    }
    Err(AxError::failure(
        AxCode::InvalidArgs,
        "open a room for this session",
        name.as_str().to_owned(),
    )
    .with_recovery(
        "that name and its first 999 suffixes are taken; give this session another word",
    ))
}

/// Makes the room a dispatch is about to write in when nothing is there
/// yet, sealed like one [`open`] made (`crates/city/spec/Gitignore.lean` §8-21).
///
/// A directory that is already there is left as it is: either the city
/// opened it and sealed it then, or it is one of the project's own, where
/// a `*` would hide every new file from the project's history. The city's
/// files in such a directory are kept out of git by name instead. A
/// building's own address is not a room and is never sealed.
///
/// # Errors
/// Propagates an address outside every building and a directory that
/// cannot be created or sealed.
pub fn claim(city_root: &Path, room: &Address) -> Result<(), AxError> {
    if crate::building::Building::of(room)?.addr() == room {
        return Ok(());
    }
    let dir = CityLayout::new(city_root).scope(room);
    match std::fs::create_dir_all(dir.parent().unwrap_or(city_root))
        .and_then(|()| std::fs::create_dir(&dir))
    {
        Ok(()) => crate::gitignore::seal_room(&dir),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "make the room a dispatch works in",
            format!("{}: {err}", dir.display()),
        )
        .with_recovery("check the city directory is writable")),
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
mod tests {
    use super::open;
    use kernel::{Address, SessionName};

    fn named(raw: &str) -> SessionName {
        SessionName::parse(raw).unwrap()
    }

    #[test]
    fn a_session_gets_the_room_it_is_named_after() {
        let dir = tempfile::tempdir().unwrap();
        let lab = Address::parse("lab").unwrap();
        let first = open(dir.path(), &lab, &named("refactor")).unwrap();
        assert_eq!(first.as_str(), "lab/refactor");
        assert!(dir.path().join("lab").join("refactor").is_dir());
    }

    /// Two sessions that happen to share a word do not share a room:
    /// that is the overwriting this module was written to stop.
    #[test]
    fn the_same_word_twice_is_two_rooms() {
        let dir = tempfile::tempdir().unwrap();
        let lab = Address::parse("lab").unwrap();
        let first = open(dir.path(), &lab, &named("test")).unwrap();
        let second = open(dir.path(), &lab, &named("test")).unwrap();
        assert_eq!(first.as_str(), "lab/test");
        assert_eq!(second.as_str(), "lab/test-2");
        assert_ne!(first, second);
    }

    /// A room opened beside a directory that is not a room of ours -
    /// the building's own reserved subtree - does not collide with it.
    #[test]
    fn a_buildings_reserved_subtree_is_not_in_the_way() {
        let dir = tempfile::tempdir().unwrap();
        let lab = Address::parse("lab").unwrap();
        std::fs::create_dir_all(dir.path().join("lab").join(kernel::RESERVED_PREFIX)).unwrap();
        let room = open(dir.path(), &lab, &named("notes")).unwrap();
        assert_eq!(room.as_str(), "lab/notes");
        assert!(!room.is_reserved());
    }

    /// A reader that looks at a room while it is claimed finds no seal or
    /// the whole one. A checkpoint walking the building is such a reader:
    /// a seal it found empty and then read grown refused the wave as a file
    /// that changed under it (city D24). The watcher polls each room's
    /// seal before that room is claimed, so every claim happens under its
    /// eye.
    #[test]
    fn a_claimed_rooms_seal_is_never_seen_half_written() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        const ROOMS: usize = 100;
        let dir = tempfile::tempdir().unwrap();
        let rooms: Vec<Address> = (0..ROOMS)
            .map(|n| Address::parse(&format!("lab/r{n}")).unwrap())
            .collect();
        let watching = AtomicUsize::new(0);
        let claimed = AtomicBool::new(false);
        let (seen, refused) = std::thread::scope(|scope| {
            let watcher = scope.spawn(|| {
                let mut seen = Vec::new();
                for (n, room) in rooms.iter().enumerate() {
                    let seal = dir.path().join(room.as_str()).join(".gitignore");
                    watching.store(n, Ordering::SeqCst);
                    loop {
                        if let Ok(bytes) = std::fs::read(&seal) {
                            seen.push(bytes);
                            break;
                        }
                        if claimed.load(Ordering::SeqCst) && !seal.exists() {
                            break;
                        }
                        std::hint::spin_loop();
                    }
                }
                seen
            });
            let mut refused = Vec::new();
            for (n, room) in rooms.iter().enumerate() {
                while watching.load(Ordering::SeqCst) < n {
                    std::hint::spin_loop();
                }
                if let Err(err) = super::claim(dir.path(), room) {
                    refused.push(err.to_string());
                }
            }
            claimed.store(true, Ordering::SeqCst);
            (watcher.join().unwrap(), refused)
        });
        let torn = seen
            .iter()
            .filter(|bytes| bytes.as_slice() != b"*\n")
            .count();
        assert_eq!(
            (seen.len(), torn, refused),
            (ROOMS, 0, Vec::<String>::new()),
            "every room's seal is read whole"
        );
    }
}
