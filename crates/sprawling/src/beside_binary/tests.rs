// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A folder that `city::has_history` reads as a city: a ledger
/// directory with a segment in it.
fn lay_city(root: &Path) {
    let ledger = kernel::layout::CityLayout::new(root).ledger();
    std::fs::create_dir_all(&ledger).unwrap();
    std::fs::write(ledger.join("ledger-00000000000000000000.jsonl"), b"{}\n").unwrap();
}

#[test]
fn a_city_beside_the_binary_moves_whole_and_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = dir.path().join("unpacked");
    let from = unpacked.join(accounting::home::CITY_DIR);
    lay_city(&from);
    std::fs::write(from.join("notes.md"), b"mine").unwrap();
    let to = dir.path().join("home").join("sprawling").join("city");

    assert_eq!(
        bring_home(&unpacked, &to).unwrap(),
        Homing::Moved {
            from: from.clone(),
            to: to.clone(),
            relinked: Ok(0)
        }
    );
    assert!(!from.exists());
    assert_eq!(std::fs::read(to.join("notes.md")).unwrap(), b"mine");
    assert_eq!(city::has_history(&to).unwrap(), city::History::Present);
    assert_eq!(
        bring_home(&unpacked, &to).unwrap(),
        Homing::NothingBeside,
        "the move happens once"
    );
}

#[test]
fn a_running_city_and_an_occupied_destination_are_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = dir.path().join("unpacked");
    let from = unpacked.join(accounting::home::CITY_DIR);
    lay_city(&from);
    let to = dir.path().join("home").join("city");

    let held = storage::LedgerHold::take(&kernel::layout::CityLayout::new(&from).ledger()).unwrap();
    assert_eq!(
        bring_home(&unpacked, &to).unwrap_err().code(),
        &AxCode::LedgerHeld
    );
    drop(held);
    assert!(from.exists() && !to.exists());

    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("theirs.txt"), b"x").unwrap();
    assert_eq!(
        bring_home(&unpacked, &to).unwrap_err().code(),
        &AxCode::InvalidArgs
    );
    assert!(from.exists());
    assert_eq!(std::fs::read(to.join("theirs.txt")).unwrap(), b"x");
}

#[test]
fn a_city_on_another_volume_than_home_is_kept_where_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("unpacked").join(accounting::home::CITY_DIR);
    lay_city(&from);
    let to = dir.path().join("home").join("sprawling").join("city");

    let across = std::io::Error::from(std::io::ErrorKind::CrossesDevices);
    assert!(
        after_rename(Err(across), from.clone(), &to).is_ok(),
        "a city on another volume is opened where it is, not refused"
    );
    assert_eq!(city::has_history(&from).unwrap(), city::History::Present);
}
