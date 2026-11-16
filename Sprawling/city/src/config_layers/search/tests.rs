// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[search]` table read, resolved, refused and written back
//! (`crates/city/spec/ConfigLayers.lean` §8-4c and its model
//! `City.ConfigLayers.Search`).

use super::*;
use kernel::{AxCode, SecretRef};

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

fn lab() -> Address {
    Address::parse("lab").unwrap()
}

fn id(raw: &str) -> ServerLabel {
    ServerLabel::parse(raw).unwrap()
}

/// A keyed supplier the settings page could have written.
fn keyed(name: &str) -> SearchSupplier {
    SearchSupplier {
        id: id(name),
        url: format!("https://{name}.example/mcp"),
        remote: format!("{name}_web_search"),
        query_field: "q".to_owned(),
        objective_field: None,
        count_field: Some("count".to_owned()),
        accounts: vec![
            ProviderAccount {
                id: id("main"),
                reference: Some(SecretRef::parse(&format!("secret:search/{name}.main")).unwrap()),
                header: Some("x-api-key".to_owned()),
            },
            ProviderAccount {
                id: id("spare"),
                reference: Some(SecretRef::parse(&format!("secret:search/{name}.spare")).unwrap()),
                header: Some("x-api-key".to_owned()),
            },
        ],
    }
}

fn custom(selected: &str, suppliers: Vec<SearchSupplier>) -> SearchConfiguration {
    SearchConfiguration::Custom {
        selected: id(selected),
        suppliers,
    }
}

/// Writes `text` as the file of `layer` for `addr`, the way a person
/// edits one by hand.
fn handwrite(root: &Path, addr: &Address, layer: Layer, text: &str) {
    let file = super::super::path(root, addr, layer).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A city whose files were written before `[search]` existed still
/// reads, and a ladder that says nothing about search resolves to the
/// default supplier, the one `default_search_supplier` declares.
#[test]
fn an_old_config_without_search_reads_and_resolves_to_the_default_supplier() {
    let dir = tempfile::tempdir().unwrap();
    handwrite(
        dir.path(),
        &room(),
        Layer::City,
        "[model]\neffort = \"high\"\n\n[[mcp]]\nlabel = \"apps\"\ncommand = \"mcp-apps\"\n",
    );
    handwrite(
        dir.path(),
        &room(),
        Layer::Building,
        "[clock]\nstamp = \"hour\"\n",
    );

    let frozen = super::super::load(dir.path(), &room()).unwrap();
    assert_eq!(frozen.search, SearchConfiguration::Default);
    assert_eq!(settled_search(dir.path(), &room()).unwrap(), None);
    let reached = search_supplier(&frozen.search).unwrap().unwrap();
    assert_eq!(reached, default_search_supplier().unwrap());
    assert_eq!(reached.url, "https://mcp.exa.ai/mcp");
    assert_eq!(reached.remote, "web_search_exa");
}

/// `City.ConfigLayers.Search`, checked over every list of distinct ids
/// drawn from three names and every selection from four: `Custom`
/// reaches a supplier exactly when its selection is listed, that
/// supplier is the one named, and an unlisted selection is refused
/// rather than answered with the default or another listed supplier.
#[test]
fn a_custom_search_reaches_exactly_its_selection_and_never_falls_back() {
    let names = ["alpha", "beta", "gamma"];
    let mut lists: Vec<Vec<&str>> = vec![Vec::new()];
    for _ in 0..names.len() {
        let longer: Vec<Vec<&str>> = lists
            .iter()
            .flat_map(|list| {
                names
                    .iter()
                    .filter(|name| !list.contains(name))
                    .map(|name| [list.clone(), vec![*name]].concat())
                    .collect::<Vec<_>>()
            })
            .collect();
        lists.extend(longer);
    }
    for list in &lists {
        for selected in ["alpha", "beta", "gamma", "delta"] {
            let suppliers = list.iter().map(|name| keyed(name)).collect();
            let answer = search_supplier(&custom(selected, suppliers));
            match (list.contains(&selected), answer) {
                (true, Ok(Some(reached))) => assert_eq!(reached, keyed(selected)),
                (false, Err(refused)) => {
                    assert_eq!(
                        refused.code(),
                        &AxCode::ConfigInvalid,
                        "{list:?} {selected}"
                    );
                }
                (listed, other) => panic!("{list:?} selecting {selected} ({listed}): {other:?}"),
            }
        }
    }
    assert_eq!(search_supplier(&SearchConfiguration::Off).unwrap(), None);
}

/// What the settings page writes is what the city's rung reads back,
/// for each of the three arms, and a key a person wrote by hand in the
/// same file survives the write.
#[test]
fn a_written_search_choice_is_read_back_from_the_city_rung() {
    let dir = tempfile::tempdir().unwrap();
    handwrite(
        dir.path(),
        &room(),
        Layer::City,
        "[clock]\nstamp = \"hour\"\n",
    );
    for configuration in [
        SearchConfiguration::Default,
        custom("beta", vec![keyed("alpha"), keyed("beta")]),
        SearchConfiguration::Off,
    ] {
        write_search(dir.path(), &configuration).unwrap();
        assert_eq!(
            settled_search(dir.path(), &room()).unwrap(),
            Some((configuration.clone(), Layer::City))
        );
        assert_eq!(
            super::super::load(dir.path(), &room()).unwrap().search,
            configuration
        );
    }
    let text = std::fs::read_to_string(CityLayout::new(dir.path()).city_config()).unwrap();
    assert!(text.contains("stamp"), "{text}");
}

/// A building's `[search]` replaces the city's whole value and says it
/// came from the building; a room's is refused on the ladder with the
/// file named and the building or city named as where it belongs.
#[test]
fn a_building_search_overrides_the_city_and_a_room_search_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write_search(dir.path(), &custom("alpha", vec![keyed("alpha")])).unwrap();
    handwrite(
        dir.path(),
        &lab(),
        Layer::Building,
        "[search]\nchoice = \"off\"\n",
    );
    assert_eq!(
        settled_search(dir.path(), &room()).unwrap(),
        Some((SearchConfiguration::Off, Layer::Building))
    );
    assert_eq!(
        search_supplier(&super::super::load(dir.path(), &room()).unwrap().search).unwrap(),
        None
    );

    handwrite(
        dir.path(),
        &room(),
        Layer::Resident,
        "[search]\nchoice = \"default\"\n",
    );
    let refused = super::super::load(dir.path(), &room()).unwrap_err();
    assert_eq!(refused.code(), &AxCode::ConfigInvalid);
    assert!(
        refused.subject().contains(SEARCH_KEY),
        "{}",
        refused.subject()
    );
    assert!(
        refused.recovery().contains("building"),
        "{}",
        refused.recovery()
    );
}

/// Every value the reader refuses is refused by the write face before a
/// byte is written, so the city's file keeps the bytes it had.
#[test]
fn a_search_value_the_reader_refuses_is_not_written() {
    let dir = tempfile::tempdir().unwrap();
    write_search(dir.path(), &SearchConfiguration::Off).unwrap();
    let file = CityLayout::new(dir.path()).city_config();
    let kept = std::fs::read_to_string(&file).unwrap();

    let with = |change: &dyn Fn(&mut SearchSupplier)| {
        let mut supplier = keyed("alpha");
        change(&mut supplier);
        custom("alpha", vec![supplier])
    };
    let refused = [
        custom("beta", vec![keyed("alpha")]),
        custom("alpha", Vec::new()),
        custom("alpha", vec![keyed("alpha"), keyed("alpha")]),
        with(&|supplier| supplier.url = "https://alpha.example/mcp?api_key=plain".to_owned()),
        with(&|supplier| supplier.url = "https://user:pass@alpha.example/mcp".to_owned()),
        with(&|supplier| supplier.url = "ftp://alpha.example/mcp".to_owned()),
        with(&|supplier| supplier.remote = " ".to_owned()),
        with(&|supplier| supplier.query_field = String::new()),
        with(&|supplier| supplier.objective_field = Some(String::new())),
        with(&|supplier| supplier.count_field = Some("q".to_owned())),
        with(&|supplier| supplier.accounts.clear()),
        with(&|supplier| supplier.accounts[1].id = id("main")),
        with(&|supplier| supplier.accounts[0].header = None),
        with(&|supplier| {
            supplier.accounts[0].reference = None;
        }),
    ];
    for configuration in refused {
        let err = write_search(dir.path(), &configuration).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid, "{configuration:?}");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            kept,
            "{configuration:?}"
        );
    }
}

/// The reader refuses what the file syntax alone can get wrong: a key
/// written as plaintext where a vault reference belongs, `selected` or
/// `suppliers` beside an arm that takes neither, a `custom` choice that
/// selects nothing, a spelling outside the three, and a key it does not
/// read.
#[test]
fn a_handwritten_search_table_is_refused_where_it_is_read() {
    let supplier = "[[search.suppliers]]\nid = \"alpha\"\nurl = \"https://alpha.example/mcp\"\n\
                    remote = \"alpha_search\"\nquery = \"q\"\n";
    let account = "[[search.suppliers.accounts]]\nid = \"main\"\nheader = \"x-api-key\"\n";
    for text in [
        format!(
            "[search]\nchoice = \"custom\"\nselected = \"alpha\"\n{supplier}{account}reference = \"plain-key-value\"\n"
        ),
        "[search]\nchoice = \"default\"\nselected = \"alpha\"\n".to_owned(),
        format!("[search]\nchoice = \"off\"\n{supplier}"),
        format!("[search]\nchoice = \"custom\"\n{supplier}"),
        "[search]\nchoice = \"exa\"\n".to_owned(),
        "[search]\nchoice = \"default\"\nenabled = true\n".to_owned(),
    ] {
        let err = ConfigLayer::parse(&text).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid, "{text}");
        assert!(
            !err.subject().contains("plain-key-value"),
            "{}",
            err.subject()
        );
    }

    let anonymous = ConfigLayer::parse(&format!(
        "[search]\nchoice = \"custom\"\nselected = \"alpha\"\n{supplier}\
         [[search.suppliers.accounts]]\nid = \"anonymous\"\n"
    ))
    .unwrap();
    assert!(matches!(
        anonymous.search(),
        Some(SearchConfiguration::Custom { suppliers, .. })
            if suppliers[0].accounts[0].reference.is_none()
    ));
}
