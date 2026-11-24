// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::{ARCHIVE_DIR, CityLayout};
use kernel::{Address, AxCode, AxError, SessionName};

const SUFFIX_LIMIT: u32 = 999;

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

    out.sort_by(|left: &Address, right: &Address| left.as_str().cmp(right.as_str()));
    Ok(out)
}

pub fn open(city_root: &Path, building: &Address, name: &SessionName) -> Result<Address, AxError> {
    for attempt in 1..=SUFFIX_LIMIT {
        let candidate = if attempt == 1 {
            format!("{}/{}", building.as_str(), name.as_str())
        } else {
            format!("{}/{}-{attempt}", building.as_str(), name.as_str())
        };
        let addr = Address::parse(&candidate)?;
        let dir = CityLayout::new(city_root).scope(&addr);

        match std::fs::create_dir_all(dir.parent().unwrap_or(city_root))
            .and_then(|()| std::fs::create_dir(&dir))
        {
            Ok(()) => {
                crate::gitignore::seal_room(&dir)?;

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

    #[test]
    fn a_buildings_reserved_subtree_is_not_in_the_way() {
        let dir = tempfile::tempdir().unwrap();
        let lab = Address::parse("lab").unwrap();
        std::fs::create_dir_all(dir.path().join("lab").join(kernel::RESERVED_PREFIX)).unwrap();
        let room = open(dir.path(), &lab, &named("notes")).unwrap();
        assert_eq!(room.as_str(), "lab/notes");
        assert!(!room.is_reserved());
    }

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
            .filter(|bytes| {
                bytes.as_slice() != format!("*\n!{}\n", crate::spine_files::HANDOFF_FILE).as_bytes()
            })
            .count();
        assert_eq!(
            (seen.len(), torn, refused),
            (ROOMS, 0, Vec::<String>::new()),
            "every room's seal is read whole"
        );
    }
}
