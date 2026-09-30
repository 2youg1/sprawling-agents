// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// The skeleton lands in the directory cargo metadata gives the lib.
#[test]
fn a_relocated_package_gets_its_spec_skeleton_in_its_own_directory() {
    let root = crate::root::fixture::relocated("spec");
    let made = super::run(&root, Some("k"));
    let landed = root.join("tools/k/Spec.lean").is_file();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(made.is_ok() && landed, "{made:?}");
}

/// The skeleton is Lean: the notice as Lean comments, then the seventeen
/// section comments `skills/sdd` lists, numbered in order.
#[test]
fn the_skeleton_opens_with_the_notice_and_numbers_the_seventeen_sections_in_order() {
    let text = super::skeleton("k");
    let head: Vec<String> = text.lines().take(4).map(str::to_owned).collect();
    let numbers: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("/-! ## "))
        .map(|rest| rest.split(' ').next().unwrap().to_owned())
        .collect();
    assert_eq!(
        (head, numbers),
        (
            crate::header::notice(crate::header::Leader::Lean).to_vec(),
            (1..=17).map(|n: u8| n.to_string()).collect::<Vec<_>>()
        )
    );
}
