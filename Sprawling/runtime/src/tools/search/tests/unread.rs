// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_walk_does_not_follow_a_link_into_a_closed_building() {
    let dir = city();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("secret.md"), "the ledger of the vault\n").unwrap();
    super::super::chosen_path::make_link(&dir.path().join("lab").join("to-vault"), &vault);
    super::super::chosen_path::make_link(&dir.path().join("lab").join("loop"), dir.path());
    let only_lab: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str().starts_with("vault") {
            ReadVerdict::Confidential
        } else {
            ReadVerdict::Open
        }
    });
    let tool = SearchTool::new(dir.path(), only_lab).unwrap();

    for asked in [None, Some("lab")] {
        let mut args = vec![
            ("pattern", Value::String("ledger".to_owned())),
            ("output_mode", Value::String("content".to_owned())),
        ];
        args.extend(asked.map(|path| ("path", Value::String(path.to_owned()))));
        let outcome = tool.invoke(&call(&args)).unwrap();
        let paths: Vec<&str> = outcome.result.as_map()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| hit["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            ["lab/Notes.md", "lab/room1/Memo.md"],
            "from {asked:?}"
        );
    }
}

#[test]
fn a_link_that_does_not_resolve_is_counted_as_unread() {
    let dir = city();
    let knot = dir.path().join("lab").join("knot");
    super::super::chosen_path::make_link(&knot, &knot);
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&pattern("the ledger")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["unreadable"], 1, "{map:?}");
    let unread = map["unread"].as_array().unwrap();
    assert_eq!(unread[0]["path"], "lab/knot");
    assert!(
        unread[0]["why"]
            .as_str()
            .unwrap()
            .contains("E_STORAGE_FATAL"),
        "{}",
        unread[0]["why"]
    );
}

#[test]
fn a_building_whose_rules_do_not_read_is_named_among_the_unread() {
    let dir = city();
    std::fs::create_dir_all(dir.path().join("broken").join("room1")).unwrap();
    let bound: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str().starts_with("broken") {
            ReadVerdict::RulesUnreadable(
                AxError::failure(
                    kernel::AxCode::InvalidArgs,
                    "read the rules",
                    "broken/RULES.toml",
                )
                .with_recovery("fix the rules"),
            )
        } else {
            ReadVerdict::Open
        }
    });
    let tool = SearchTool::new(dir.path(), bound).unwrap();

    let whole = tool.invoke(&pattern("the ledger")).unwrap();
    let map = whole.result.as_map();
    assert_eq!(map["count"], 2, "the two hits in lab");
    assert_eq!(map["unreadable"], 1, "{:?}", map["unread"]);
    assert_eq!(map["unread"][0]["path"], "broken");
}

#[test]
fn the_unread_list_follows_the_walk_order() {
    let dir = city();
    for building in ["broken", "mill"] {
        std::fs::create_dir_all(dir.path().join(building).join("room1")).unwrap();
    }
    std::fs::write(
        dir.path().join("lab").join("Big.md"),
        "the ledger\n".repeat(100_000),
    )
    .unwrap();
    let bound: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str() == "lab" {
            ReadVerdict::Open
        } else {
            ReadVerdict::RulesUnreadable(
                AxError::failure(kernel::AxCode::InvalidArgs, "read the rules", addr.as_str())
                    .with_recovery("fix the rules"),
            )
        }
    });
    let tool = SearchTool::new(dir.path(), bound).unwrap();

    let whole = tool.invoke(&pattern("the ledger")).unwrap();
    let paths: Vec<&Value> = whole.result.as_map()["unread"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| &entry["path"])
        .collect();
    assert_eq!(paths, ["broken", "lab/Big.md", "mill"]);
}

#[test]
fn a_name_that_is_no_address_is_named_among_the_unread() {
    let named = match descent::admissible("lab", "a:b", &|_: &Address| ReadVerdict::Open) {
        descent::Entry::Unread { child, why } => Some((child, why)),
        descent::Entry::Descend(_) | descent::Entry::Passed => None,
    };
    let reason = Address::parse("lab/a:b").unwrap_err().recovery().to_owned();
    let why = format!("its name is not an address: {reason}");
    assert_eq!(named, Some(("lab/a:b".to_owned(), why)));
}
