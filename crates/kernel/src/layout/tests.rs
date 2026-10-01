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
    assert_eq!(layout().playback_exports(), governed.join(PLAYBACK_DIR));
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

/// The ignore pattern and the display of a run id are two spellings of
/// one shape, and a building keeps transcripts out of git only while
/// they agree character for character.
#[test]
fn the_run_id_pattern_matches_every_run_id_the_city_displays() {
    for bytes in [[0u8; 16], [0xffu8; 16]] {
        let shown = crate::RunId::from_bytes(bytes).to_string();
        assert_eq!(shown.len(), RUN_ID_PATTERN.len(), "{shown}");
        for (glyph, wanted) in shown.chars().zip(RUN_ID_PATTERN.chars()) {
            assert!(wanted == '?' || glyph == wanted, "{shown}");
            assert_eq!(glyph == '-', wanted == '-', "{shown}");
        }
    }
}

#[test]
fn the_device_table_sits_in_the_remote_gate_s_own_directory_of_the_reserved_subtree() {
    let devices = layout().devices();
    assert_eq!(
        devices,
        Path::new("/city")
            .join(RESERVED_PREFIX)
            .join(REMOTE_DIR)
            .join(DEVICES_FILE)
    );
    let spelled = devices.to_string_lossy().replace('\\', "/");
    assert!(
        addr(spelled.trim_start_matches("/city/")).is_reserved(),
        "{spelled} is reachable by a write domain"
    );
}
