// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use wire::{LayerDocument, LayerState, PrefixSlot, PromptKind, PromptLayer};

use super::*;
use crate::views::holding::Views;

fn at(spelled: &str) -> Address {
    Address::parse(spelled).unwrap()
}

fn put(root: &Path, spelled: &str, text: &str) {
    let path = root.join(spelled);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn ask(root: &Path, addr: &str) -> Vec<LayerDocument> {
    let wire::Answer::Layers(answer) =
        Views::new(root).answer(&wire::Query::Layers { addr: at(addr) })
    else {
        panic!("Layers answers with layers");
    };
    answer.documents
}

/// One document as the answer gives it; `text` is what is on disk, and
/// none for a document that is absent or withheld.
fn document(
    (spelled, text): (&str, Option<&str>),
    (slot, kind, layer): (PrefixSlot, PromptKind, PromptLayer),
    state: LayerState,
) -> LayerDocument {
    LayerDocument {
        at: at(spelled),
        version: B3Hash::digest(text.unwrap_or_default().as_bytes()),
        slot,
        kind,
        layer,
        state,
        shipped: None,
    }
}

fn note(model: &str) -> PromptKind {
    PromptKind::ModelNote {
        provider: "local".to_owned(),
        model: model.to_owned(),
    }
}

/// Every prompt document of a room, in the order the prefix sends them:
/// the city's `City.md`, the two building conventions, the room's
/// identity and its own conventions, then the model notes. A directory
/// named `AGENTS.md` is withheld with its reason rather than refusing
/// the room; the building's note for a model replaces the city's, and
/// where the building keeps none its place is listed absent.
#[test]
fn the_layers_of_a_room_are_listed_in_send_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, "City.md", "city\n");
    put(root, "AGENTS.md", "workspace\n");
    std::fs::create_dir_all(root.join("lab/AGENTS.md")).unwrap();
    put(root, "lab/room/AGENTS.md", "room\n");
    put(root, ".sprawling/models/local/m.md", "note\n");
    put(root, "lab/.sprawling/models/local/m.md", "lab note\n");
    put(root, ".sprawling/models/local/n.md", "n\n");

    let mut city = document(
        ("City.md", Some("city\n")),
        (PrefixSlot::City, PromptKind::City, PromptLayer::City),
        LayerState::Sent { bytes: 5 },
    );
    city.shipped = Some(wire::ShippedDefault {
        version: B3Hash::digest(city::Shipped::City.text().as_bytes()),
        standing: wire::DefaultStanding::Edited,
    });
    assert_eq!(
        ask(root, "lab/room"),
        vec![
            city,
            document(
                ("lab/.sprawling/City.md", None),
                (PrefixSlot::City, PromptKind::City, PromptLayer::Building),
                LayerState::Absent,
            ),
            document(
                ("AGENTS.md", Some("workspace\n")),
                (
                    PrefixSlot::Building,
                    PromptKind::Conventions,
                    PromptLayer::City
                ),
                LayerState::Sent { bytes: 10 },
            ),
            document(
                ("lab/AGENTS.md", None),
                (
                    PrefixSlot::Building,
                    PromptKind::Conventions,
                    PromptLayer::Building
                ),
                LayerState::Withheld {
                    why: "AGENTS.md is a directory, not a file".to_owned()
                },
            ),
            document(
                ("lab/room/URBANITE.md", None),
                (
                    PrefixSlot::Resident,
                    PromptKind::Identity,
                    PromptLayer::Room
                ),
                LayerState::Absent,
            ),
            document(
                ("lab/room/AGENTS.md", Some("room\n")),
                (
                    PrefixSlot::Resident,
                    PromptKind::Conventions,
                    PromptLayer::Room
                ),
                LayerState::Sent { bytes: 5 },
            ),
            document(
                (".sprawling/models/local/m.md", Some("note\n")),
                (PrefixSlot::Run, note("m"), PromptLayer::City),
                LayerState::Replaced {
                    by: at("lab/.sprawling/models/local/m.md")
                },
            ),
            document(
                ("lab/.sprawling/models/local/m.md", Some("lab note\n")),
                (PrefixSlot::Run, note("m"), PromptLayer::Building),
                LayerState::Sent { bytes: 9 },
            ),
            document(
                (".sprawling/models/local/n.md", Some("n\n")),
                (PrefixSlot::Run, note("n"), PromptLayer::City),
                LayerState::Sent { bytes: 2 },
            ),
            document(
                ("lab/.sprawling/models/local/n.md", None),
                (PrefixSlot::Run, note("n"), PromptLayer::Building),
                LayerState::Absent,
            ),
        ]
    );
}

/// A link a run could make in its room, leading out of the city, is not
/// followed: the page reads why it is withheld, which is also why the
/// freeze leaves it out.
#[cfg(unix)]
#[test]
fn a_convention_linked_out_of_the_city_is_withheld() {
    let outside = tempfile::tempdir().unwrap();
    put(outside.path(), "secret.txt", "not the project's\n");
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        dir.path().join("lab/AGENTS.md"),
    )
    .unwrap();

    let building = ask(dir.path(), "lab")
        .into_iter()
        .find(|doc| doc.at == at("lab/AGENTS.md"))
        .unwrap();
    assert!(
        matches!(building.state, LayerState::Withheld { .. }),
        "{building:?}"
    );
}
