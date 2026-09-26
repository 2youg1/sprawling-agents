// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The layout's own grammar: one path per kind of file, the
//! reserved subtree's shape, and the city's name read from its root.

use super::*;

fn layout() -> CityLayout {
    CityLayout::new(Path::new("/city"))
}

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

#[test]
fn a_nested_address_becomes_one_directory_per_segment() {
    let expected = Path::new("/city").join("lab").join("refactor");
    assert_eq!(layout().scope(&addr("lab/refactor")), expected);
}

#[test]
fn the_city_wide_stores_sit_under_the_city_s_reserved_subtree() {
    let governed = Path::new("/city").join(RESERVED_PREFIX);
    assert_eq!(layout().ledger(), governed.join(LEDGER_DIR));
    assert_eq!(layout().cas(), governed.join(CAS_DIR));
    assert_eq!(layout().library(), governed.join(LIBRARY_DIR));
}

#[test]
fn what_governs_a_scope_is_unreachable_by_any_write_domain() {
    let building = addr("lab");
    let governing = [
        layout().config(&building),
        layout().filters(&building),
        layout().building_skills(&building),
    ];
    for path in governing {
        let spelled = path.to_string_lossy().replace('\\', "/");
        let as_address = spelled.trim_start_matches("/city/").to_owned();
        assert!(
            addr(&as_address).is_reserved(),
            "{spelled} is not in a reserved subtree"
        );
    }
}

#[test]
fn what_residents_write_stays_where_residents_can_reach_it() {
    let room = addr("lab/refactor");
    let open = [
        layout().job(&room),
        layout().handoff(&room),
        layout().urbanite(&room),
        layout().archive(&addr("lab")),
    ];
    for path in open {
        let spelled = path.to_string_lossy().replace('\\', "/");
        assert!(
            !spelled.contains(RESERVED_PREFIX),
            "{spelled} is governing rather than written"
        );
    }
}

#[test]
fn a_configuration_layer_is_named_by_the_scope_it_sits_in() {
    assert_eq!(
        layout().config(&addr("lab")),
        Path::new("/city")
            .join("lab")
            .join(RESERVED_PREFIX)
            .join(CONFIG_FILE)
    );
}

#[test]
fn a_session_slice_lives_under_the_sessions_of_its_first_segment() {
    let sessions = Path::new("/city")
        .join("webapp")
        .join(RESERVED_PREFIX)
        .join(SESSIONS_DIR);
    assert_eq!(
        layout().session_slice(&addr("webapp/api-rewrite")),
        sessions.join("api-rewrite.jsonl")
    );
    // A room one level below a room keeps its own path: the nesting
    // of the address is the nesting of the files.
    assert_eq!(
        layout().session_slice(&addr("webapp/backend/db-migration")),
        sessions.join("backend").join("db-migration.jsonl")
    );
    // A run that named no session works at the building's address.
    assert_eq!(
        layout().session_slice(&addr("webapp")),
        sessions.join("webapp.jsonl")
    );
}

#[test]
fn a_session_slice_is_out_of_every_write_domain() {
    let path = layout().session_slice(&addr("lab/room1"));
    let spelled = path.to_string_lossy().replace('\\', "/");
    let as_address = spelled.trim_start_matches("/city/").to_owned();
    assert!(
        addr(&as_address).is_reserved(),
        "{spelled} is not in a reserved subtree"
    );
}

#[test]
fn a_fence_can_tell_a_session_slice_from_a_promise() {
    let held = [
        "lab/.sprawling/sessions/room1.jsonl",
        "lab/room1/.sprawling/sessions/room1.jsonl",
        "lab/.SPRAWLING/sessions/x",
    ];
    for spelled in held {
        assert!(
            CityLayout::is_session_projection(Path::new(spelled)),
            "{spelled} is a session slice"
        );
    }
    let kept = [
        "lab/.sprawling/CONFIG.toml",
        "lab/.sprawling/skills/one/SKILL.md",
        "lab/room1/JOB.md",
        "sessions/room1.jsonl",
        "lab/room1/sessions/notes.md",
        "lab/.sprawling/library/sessions.md",
    ];
    for spelled in kept {
        assert!(
            !CityLayout::is_session_projection(Path::new(spelled)),
            "{spelled} is a person's or a promise, not a slice"
        );
    }
}

#[test]
fn a_city_s_name_is_the_directory_it_lives_in() {
    assert_eq!(
        CityLayout::new(Path::new("/city")).city_address(),
        Some(addr("city"))
    );
    // A directory name that is not an address is no name at all
    // rather than a guessed one, and the root itself has none.
    assert_eq!(
        CityLayout::new(Path::new("/city/bad:name")).city_address(),
        None
    );
    assert_eq!(CityLayout::new(Path::new("/")).city_address(), None);
}

#[test]
fn a_ledger_directory_alone_yields_the_city_it_belongs_to() {
    let ledger = Path::new("/city").join(RESERVED_PREFIX).join(LEDGER_DIR);
    assert_eq!(
        CityLayout::of_ledger(&ledger),
        Some(CityLayout::new(Path::new("/city")))
    );
    // A store opened directly is not a city's ledger: a fixture's
    // directory, a bundle's scratch space, a relative path.
    for not_a_city in [
        Path::new("/store/ledger"),
        Path::new("/store/.sprawling"),
        Path::new(".sprawling/ledger"),
    ] {
        assert_eq!(CityLayout::of_ledger(not_a_city), None, "{not_a_city:?}");
    }
}
