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
        Confinement::CopiedTree,
        "the floor arm is what a machine with no wrapper gets"
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
    let confined = Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
    let err = match confined.place(std::process::Command::new("cmd"), source.path()) {
        Err(err) => err,
        Ok(_) => panic!("a tree with no bottom must be refused"),
    };
    assert_eq!(*err.code(), AxCode::SandboxDenied);
    assert!(err.subject().contains("nests deeper"), "{err}");
}

/// The copy is removed when the command that ran in it has ended: a
/// scratch root that grew one tree per command would be a leak nothing
/// reports.
#[test]
fn a_settled_sandbox_command_leaves_no_copy_behind() {
    let scratch = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("a.txt"), b"one\n").unwrap();
    let confined = Confined::with_arm(Confinement::CopiedTree, Some(scratch.path().to_path_buf()));
    let (_, placed) = confined
        .place(std::process::Command::new("cmd"), source.path())
        .unwrap();
    let copies = std::fs::read_dir(scratch.path()).unwrap().count();
    assert_eq!(copies, 1, "one command, one copy");
    confined.settled(placed);
    assert_eq!(
        std::fs::read_dir(scratch.path()).unwrap().count(),
        0,
        "the copy goes when the wait ends"
    );
}
