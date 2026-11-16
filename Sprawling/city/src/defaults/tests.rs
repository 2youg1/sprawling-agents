// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

const V0_0_12_MAYOR: &str = include_str!("tests/v0.0.12-MAYOR.md");
const V0_0_12_CLERK: &str = include_str!("tests/v0.0.12-CLERK.md");

fn city_with(copies: &[(Shipped, &[u8])]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (doc, bytes) in copies {
        let path = doc.path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    dir
}

fn on_disk(dir: &tempfile::TempDir, doc: Shipped) -> Vec<u8> {
    std::fs::read(doc.path(dir.path())).unwrap()
}

#[test]
fn an_unedited_v0_0_12_copy_follows_the_default() {
    let dir = city_with(&[
        (Shipped::City, Shipped::City.text().as_bytes()),
        (Shipped::Mayor, V0_0_12_MAYOR.as_bytes()),
        (Shipped::Clerk, V0_0_12_CLERK.as_bytes()),
    ]);

    let edited = follow_defaults(dir.path()).unwrap();

    assert_eq!(edited, Vec::new());
    for doc in Shipped::ALL {
        assert_eq!(on_disk(&dir, doc), doc.text().as_bytes(), "{doc:?}");
    }
}

#[test]
fn an_edited_copy_is_kept_and_named() {
    let mine = format!("{V0_0_12_MAYOR}\nThe Mayor speaks like a harbour pilot.\n");
    let dir = city_with(&[(Shipped::Mayor, mine.as_bytes())]);

    let edited = follow_defaults(dir.path()).unwrap();

    assert_eq!(
        edited,
        vec![Edited {
            doc: Shipped::Mayor,
            path: Shipped::Mayor.path(dir.path()),
        }]
    );
    assert_eq!(on_disk(&dir, Shipped::Mayor), mine.as_bytes());
}

#[test]
fn an_identity_area_stays_while_the_body_follows_the_default() {
    let area = "+++\nmayor_name = \"Ada\"\n+++\n";
    let dir = city_with(&[(Shipped::Mayor, format!("{area}{V0_0_12_MAYOR}").as_bytes())]);

    follow_defaults(dir.path()).unwrap();

    assert_eq!(
        on_disk(&dir, Shipped::Mayor),
        format!("{area}{}", Shipped::Mayor.text()).as_bytes()
    );
}

#[test]
fn a_missing_copy_is_laid_down() {
    let dir = tempfile::tempdir().unwrap();

    follow_defaults(dir.path()).unwrap();

    for doc in Shipped::ALL {
        assert_eq!(on_disk(&dir, doc), doc.text().as_bytes(), "{doc:?}");
    }
}

#[test]
fn no_default_lists_itself_as_earlier() {
    for doc in Shipped::ALL {
        let current = B3Hash::digest(doc.text().as_bytes()).to_string();
        assert!(!doc.earlier().contains(&current.as_str()), "{doc:?}");
    }
}
