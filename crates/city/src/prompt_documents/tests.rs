// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn addr(spelled: &str) -> Address {
    Address::parse(spelled).unwrap()
}

/// The list reads back every address it spells, and nothing else in a
/// reserved subtree is on it: a building's rules and configuration keep
/// their own doors.
#[test]
fn prompt_documents_are_one_list_that_reads_back_what_it_spells() {
    let lab = addr("lab");
    let listed = [
        PromptDocument::Shipped(Shipped::City),
        PromptDocument::Shipped(Shipped::Mayor),
        PromptDocument::Shipped(Shipped::Clerk),
        PromptDocument::BuildingCity {
            building: lab.clone(),
        },
        PromptDocument::ModelNote {
            building: None,
            provider: "local".to_owned(),
            model: "m-1".to_owned(),
        },
        PromptDocument::ModelNote {
            building: Some(lab),
            provider: "local".to_owned(),
            model: "m-1".to_owned(),
        },
    ];
    for doc in &listed {
        assert_eq!(PromptDocument::of(&doc.at().unwrap()).as_ref(), Some(doc));
    }
    for other in [
        ".sprawling/PREFERENCES.md",
        "lab/.sprawling/RULES.toml",
        "lab/.sprawling/CONFIG.toml",
        "lab/City.md",
        ".sprawling/models/local/m-1.txt",
    ] {
        assert_eq!(PromptDocument::of(&addr(other)), None, "{other}");
    }
}

/// A building's own `City.md` starts as the city's words, goes again on
/// `Inherit`, and each step is refused once the file moved since the page
/// read it.
#[test]
fn prompt_documents_own_copies_the_city_and_inherit_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(CITY_FILE), "the city's words\n").unwrap();
    let doc = PromptDocument::BuildingCity {
        building: addr("lab"),
    };
    let empty = B3Hash::digest(b"");

    let owned = put_prompt(dir.path(), &doc, empty, PromptStep::Own).unwrap();
    let path = doc.path(dir.path()).unwrap();
    assert_eq!(
        (owned, std::fs::read_to_string(&path).unwrap()),
        (
            PromptWritten {
                bytes: 17,
                removed: false
            },
            "the city's words\n".to_owned()
        )
    );

    let stale = put_prompt(dir.path(), &doc, empty, PromptStep::Inherit).unwrap_err();
    assert_eq!(stale.code(), &AxCode::VersionConflict);

    let base = B3Hash::digest(b"the city's words\n");
    let inherited = put_prompt(dir.path(), &doc, base, PromptStep::Inherit).unwrap();
    assert_eq!(
        (inherited.removed, path.exists()),
        (true, false),
        "the building's copy is gone, so the city's governs again"
    );
}

/// `Restore` writes the default back and keeps the names written above
/// the Mayor's persona; a step a document does not take is refused with
/// the ones it does.
#[test]
fn prompt_documents_restore_the_default_and_refuse_a_step_out_of_place() {
    let dir = tempfile::tempdir().unwrap();
    let mayor = PromptDocument::Shipped(Shipped::Mayor);
    let path = mayor.path(dir.path()).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "my own persona\n").unwrap();

    put_prompt(
        dir.path(),
        &mayor,
        B3Hash::digest(b"my own persona\n"),
        PromptStep::Restore,
    )
    .unwrap();
    assert_eq!(
        crate::defaults::standing(Shipped::Mayor, Some(&std::fs::read(&path).unwrap())),
        crate::defaults::CopyStanding::Current
    );

    let refused =
        put_prompt(dir.path(), &mayor, B3Hash::digest(b""), PromptStep::Inherit).unwrap_err();
    assert_eq!(refused.code(), &AxCode::InvalidArgs);
}

/// The notes a layer keeps are listed by provider and model, and a file
/// that is not a note is not one of them.
#[test]
fn prompt_documents_list_the_notes_one_layer_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    for (building, provider, model) in [
        (None, "local", "m-2"),
        (None, "local", "m-1"),
        (Some(&lab), "remote", "big"),
    ] {
        let path = dir
            .path()
            .join(model_note_at(building, provider, model).unwrap().as_str());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "note").unwrap();
    }
    std::fs::write(
        dir.path().join(".sprawling/models/local/readme.txt"),
        "not a note",
    )
    .unwrap();

    assert_eq!(
        (
            model_notes(dir.path(), None).unwrap(),
            model_notes(dir.path(), Some(&lab)).unwrap()
        ),
        (
            vec![
                ("local".to_owned(), "m-1".to_owned()),
                ("local".to_owned(), "m-2".to_owned())
            ],
            vec![("remote".to_owned(), "big".to_owned())]
        )
    );
}
