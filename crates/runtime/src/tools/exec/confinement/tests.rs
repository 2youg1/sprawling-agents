// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the arm table promises a reader, and what the copy does to a
//! real tree. Every machine-shaped input is stated rather than sampled,
//! so a test judges a machine instead of borrowing one.

use std::path::PathBuf;

use kernel::AxCode;
use serde_json::{Map, Value};

use super::*;

#[test]
fn a_placement_is_read_as_one_of_two_words_and_absence_is_the_sandbox() {
    let mut args = Map::new();
    assert_eq!(parse_placement(&args).unwrap(), Placement::Sandbox);
    args.insert("where".to_owned(), Value::String("host".to_owned()));
    assert_eq!(parse_placement(&args).unwrap(), Placement::Host);
    args.insert("where".to_owned(), Value::String("host-please".to_owned()));
    let err = parse_placement(&args).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(err.recovery().contains("sandbox"), "{}", err.recovery());
}

/// The arm a machine is given is a function of what it has, so a test
/// states a machine instead of borrowing one.
#[test]
fn the_sandbox_arm_a_machine_gets_is_chosen_from_what_it_has() {
    let scratch = Some(std::env::temp_dir());
    assert_eq!(
        Confinement::choose(&Offerings {
            namespace_tool: None,
            scratch: scratch.clone(),
        }),
        if cfg!(windows) {
            Confinement::WindowsJobObject
        } else {
            Confinement::CopiedTree
        },
        "a Windows build has its native leaf; other platforms need a wrapper"
    );
    let wrapper = PathBuf::from("/usr/bin/bwrap");
    assert_eq!(
        Confinement::choose(&Offerings {
            namespace_tool: Some(wrapper.clone()),
            scratch: scratch.clone(),
        }),
        Confinement::LinuxNamespaces { wrapper },
        "a wrapper is the better arm, and it is chosen rather than preferred"
    );
    assert_eq!(
        Confinement::choose(&Offerings {
            namespace_tool: Some(PathBuf::from("/usr/bin/bwrap")),
            scratch: None,
        }),
        Confinement::Unavailable {
            missing: Missing::ScratchDirectory
        },
        "no scratch root means no copy, which means no arm at all"
    );
}

/// What each arm promises is stated by the arm, and the statement names
/// what it does not hold. This is the sentence that stands in front of
/// an agent; a missing 'not' here is how a command that needed a closed
/// network gets run in an open one.
#[test]
fn every_sandbox_arm_states_what_it_does_not_hold() {
    let floor = Confinement::CopiedTree.statement();
    assert!(floor.contains("copied_tree"), "{floor}");
    assert!(
        floor.contains("writes through the working directory land on a copy"),
        "{floor}"
    );
    assert!(floor.contains("network isolation"), "{floor}");
    assert!(floor.contains("process-tree containment"), "{floor}");

    let namespaced = Confinement::LinuxNamespaces {
        wrapper: PathBuf::from("/usr/bin/bwrap"),
    }
    .statement();
    assert!(namespaced.contains("the network is closed"), "{namespaced}");
    assert!(
        !namespaced.contains("network isolation"),
        "an arm that closes the network cannot also admit it does not: {namespaced}"
    );

    let absent = Confinement::Unavailable {
        missing: Missing::ScratchDirectory,
    }
    .statement();
    assert!(absent.contains("a writable scratch directory"), "{absent}");
    assert!(
        Confinement::Unavailable {
            missing: Missing::ScratchDirectory
        }
        .assurances()
        .of(Guarantee::Filesystem)
            == Kept::No,
        "an arm that cannot place a command keeps nothing"
    );
}

/// A walk that follows links has to end. The depth is the one thing
/// bounding it, and past it the copy refuses by name rather than
/// running until the process dies.
#[test]
fn a_sandbox_refuses_a_tree_deeper_than_its_walk_can_end() {
    let source = tempfile::tempdir().unwrap();
    let mut path = source.path().to_path_buf();
    for level in 0..70u32 {
        path = path.join(format!("level-{level}"));
    }
    std::fs::create_dir_all(&path).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let mut confined =
        Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
    let err = match confined.place(std::process::Command::new("cmd"), source.path()) {
        Err(err) => err,
        Ok(_) => panic!("a tree with no bottom must be refused"),
    };
    assert_eq!(*err.code(), AxCode::SandboxDenied);
    assert!(err.subject().contains("nests deeper"), "{err}");
}

/// A tree of `files` files of sixteen bytes under `src/`.
fn source_tree(files: u64) -> tempfile::TempDir {
    let source = tempfile::tempdir().unwrap();
    std::fs::create_dir(source.path().join("src")).unwrap();
    for i in 0..files {
        std::fs::write(
            source.path().join("src").join(format!("file-{i:04}.txt")),
            b"sixteen bytes ..",
        )
        .unwrap();
    }
    source
}

/// `crates/runtime/spec/Tools/Exec.lean` §8-13-2 and runtime D10: the second command of a tool gets the
/// copy the first one used, brought back to the working directory, so
/// what it writes is what the first command and the person changed, not
/// the whole tree again. Judged at two sizes: a copy made afresh would
/// create as many files as the tree holds.
#[test]
fn a_sandbox_copy_is_synced_rather_than_made_again() {
    for files in [32, 64] {
        let scratch = tempfile::tempdir().unwrap();
        let source = source_tree(files);
        let mut confined =
            Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
        let (command, first) = confined
            .place(std::process::Command::new("cmd"), source.path())
            .unwrap();
        let copy = command.get_current_dir().unwrap().to_path_buf();
        assert_eq!(
            first.work(),
            storage::FileWork {
                created: files,
                rewritten: 0,
                removed: 0,
                walked: files + 1,
            },
            "the first command's copy at {files} files"
        );
        confined.settled(first);
        assert!(
            copy.is_dir(),
            "the copy is kept for the tool's next command"
        );

        std::fs::write(copy.join("src").join("file-0000.txt"), b"a command wrote").unwrap();
        std::fs::write(copy.join("src").join("stray.txt"), b"a command left").unwrap();
        std::fs::write(
            source.path().join("src").join("file-0001.txt"),
            b"sixteen bytes !!",
        )
        .unwrap();
        let (command, second) = confined
            .place(std::process::Command::new("cmd"), source.path())
            .unwrap();
        assert_eq!(command.get_current_dir(), Some(copy.as_path()));
        assert_eq!(
            second.work(),
            storage::FileWork {
                created: 0,
                rewritten: 2,
                removed: 1,
                walked: 2 * (files + 1) + 1,
            },
            "the second command's copy at {files} files"
        );
        confined.settled(second);

        let (_, third) = confined
            .place(std::process::Command::new("cmd"), source.path())
            .unwrap();
        assert_eq!(
            (
                third.work(),
                std::fs::read(copy.join("src").join("file-0000.txt")).unwrap(),
                std::fs::read(copy.join("src").join("file-0001.txt")).unwrap(),
                copy.join("src").join("stray.txt").exists(),
            ),
            (
                storage::FileWork {
                    walked: 2 * (files + 1),
                    ..storage::FileWork::default()
                },
                b"sixteen bytes ..".to_vec(),
                b"sixteen bytes !!".to_vec(),
                false,
            ),
            "an unchanged tree writes nothing, and the copy holds the working directory at {files} files"
        );
    }
}

/// The copy of a command that ended is kept for the tool's next command,
/// a second copy exists only while two commands hold one each, and every
/// copy goes when the tool does: a scratch root that grew one tree per
/// command would be a leak nothing reports.
#[test]
fn a_sandbox_copy_goes_with_the_tool() {
    let scratch = tempfile::tempdir().unwrap();
    let source = source_tree(2);
    let copies = || std::fs::read_dir(scratch.path()).unwrap().count();
    let mut confined =
        Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
    let (_, first) = confined
        .place(std::process::Command::new("cmd"), source.path())
        .unwrap();
    let (_, second) = confined
        .place(std::process::Command::new("cmd"), source.path())
        .unwrap();
    assert_eq!(copies(), 2, "two commands at once, two copies");
    confined.settled(first);
    confined.settled(second);
    assert_eq!(copies(), 1, "one copy is kept for the next command");
    drop(confined);
    assert_eq!(copies(), 0, "the copies go with the tool");
}

/// A read-only file in the working directory is copied read-only, and a
/// sync that has to replace it still can: the copy's entry is removed
/// and made again, and the removal is not stopped by the flag.
#[test]
#[allow(
    clippy::permissions_set_readonly_false,
    reason = "the working directory's file is changed between two commands"
)]
fn a_read_only_file_is_synced_like_any_other() {
    let scratch = tempfile::tempdir().unwrap();
    let source = source_tree(1);
    let file = source.path().join("src").join("file-0000.txt");
    let set_read_only = |read_only: bool| {
        let mut bits = std::fs::metadata(&file).unwrap().permissions();
        bits.set_readonly(read_only);
        std::fs::set_permissions(&file, bits).unwrap();
    };
    set_read_only(true);
    let mut confined =
        Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
    let (command, first) = confined
        .place(std::process::Command::new("cmd"), source.path())
        .unwrap();
    let copy = command
        .get_current_dir()
        .unwrap()
        .join("src")
        .join("file-0000.txt");
    confined.settled(first);
    set_read_only(false);
    std::fs::write(&file, b"sixteen bytes !!").unwrap();
    set_read_only(true);

    let (_, second) = confined
        .place(std::process::Command::new("cmd"), source.path())
        .unwrap();
    set_read_only(false);
    assert_eq!(
        (second.work().rewritten, std::fs::read(&copy).unwrap()),
        (1, b"sixteen bytes !!".to_vec())
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_namespaced_command_requires_a_user_namespace() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let mut confined = Confined::with_arm(
        Confinement::LinuxNamespaces {
            wrapper: PathBuf::from("/bin/true"),
        },
        Some(scratch.path().to_path_buf()),
    );
    let (command, placed) = confined
        .place(std::process::Command::new("/bin/true"), source.path())
        .unwrap();
    confined.settled(placed);
    let args: Vec<_> = command.get_args().collect();
    assert!(
        args.windows(2)
            .any(|pair| { pair == ["--unshare-all", "--unshare-user"] })
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_namespace_setup_failure_refuses_before_copying() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    for wrapper in [
        PathBuf::from("/bin/false"),
        source.path().join("missing-wrapper"),
    ] {
        let mut confined = Confined::with_arm(
            Confinement::LinuxNamespaces { wrapper },
            Some(scratch.path().to_path_buf()),
        );
        let err = match confined.place(std::process::Command::new("/bin/true"), source.path()) {
            Err(err) => err,
            Ok(_) => panic!("a failed namespace setup must refuse the placement"),
        };
        assert_eq!(*err.code(), AxCode::SandboxDenied);
        assert!(
            err.subject().contains("unprivileged user namespace"),
            "{err}"
        );
        assert!(err.recovery().contains("where: host"), "{err}");
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
    }
}
