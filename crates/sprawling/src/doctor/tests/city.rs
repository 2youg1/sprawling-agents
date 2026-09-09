// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A building named for what it asks and this machine lacks, rules that
//! will not read, and a code explained (sprawling-SPEC.md section 8-48).

use kernel::Address;

use super::ScriptedMachine;
use crate::doctor::explain::{Explanation, explain};
use crate::doctor::needs::{Bits, Capability, lack_line, lacks};
use crate::doctor::screen::{Asked, asked, run};
use crate::doctor::visit::{Visited, visit};
use crate::doctor::{Platform, examine};

fn lab() -> Address {
    Address::parse("lab").unwrap()
}

/// A building that wrote `browser: true` on a machine with neither
/// engine is named, with both roads it tried and why each is closed.
#[test]
fn a_building_that_asks_for_a_browser_is_named_when_this_machine_has_none() {
    let findings = examine(&ScriptedMachine::missing(&["firefox", "chromedriver"]));
    let bits = Bits {
        browser: true,
        desktop: false,
        shell: false,
    };
    let lacking = lacks(&lab(), &bits, &findings);
    assert_eq!(lacking.len(), 1);
    assert_eq!(lacking[0].capability, Capability::Browser);
    let line = lack_line(&lacking[0]);
    assert!(line.contains("lab"), "{line}");
    assert!(line.contains("browser: true"), "{line}");
    assert!(line.contains("firefox"), "{line}");
    assert!(line.contains("chromedriver"), "{line}");
    assert!(line.contains("not on the search path"), "{line}");

    let one_road = examine(&ScriptedMachine::missing(&["firefox"]));
    assert!(
        lacks(&lab(), &bits, &one_road).is_empty(),
        "chromedriver alone is a road, so the building lacks nothing"
    );
    let nothing_asked = Bits {
        browser: false,
        desktop: false,
        shell: false,
    };
    assert!(lacks(&lab(), &nothing_asked, &findings).is_empty());
}

/// Every capability bit is judged, and each names the items it tries.
#[test]
fn every_capability_names_at_least_one_item_the_table_carries() {
    let named: std::collections::BTreeSet<&str> = crate::doctor::REQUIREMENTS
        .iter()
        .map(|item| item.name)
        .collect();
    for capability in Capability::ALL {
        let tried = capability.any_of();
        assert!(!tried.is_empty(), "{capability:?} tries nothing");
        for item in tried {
            assert!(
                named.contains(item),
                "{capability:?} tries {item}, not a row"
            );
        }
    }
}

/// A city is walked building by building; one whose rules will not
/// read is a line about that building, and never a building skipped.
#[test]
fn a_building_whose_rules_will_not_read_is_reported_not_skipped() {
    let dir = tempfile::tempdir().unwrap();
    crate::assembly::init_city(dir.path()).unwrap();
    city::create_building(dir.path(), &lab(), city::BuildingTemplate::Minimal).unwrap();
    city::write_rules(dir.path(), &lab(), "confidential: false\nbrowser: true\n").unwrap();
    let mill = Address::parse("mill").unwrap();
    city::create_building(dir.path(), &mill, city::BuildingTemplate::Minimal).unwrap();
    std::fs::write(
        city::building_path(dir.path(), &mill),
        "confidential: true\nbrowser: true\n",
    )
    .unwrap();

    let visited = visit(dir.path()).unwrap();
    let lab_bits = visited.iter().find_map(|seen| match seen {
        Visited::Bits { building, bits } if *building == lab() => Some(bits.clone()),
        _ => None,
    });
    assert_eq!(
        lab_bits,
        Some(Bits {
            browser: true,
            desktop: false,
            shell: false,
        })
    );
    assert!(
        visited.iter().any(|seen| matches!(
            seen,
            Visited::Unreadable { building, .. } if *building == mill
        )),
        "a confidential building asking for a browser is rules that will not read"
    );
    assert!(
        visit(&dir.path().join("no-such-city")).is_err(),
        "a directory that is not a city is an error, not an empty report"
    );
}

/// `--explain` connects a refusal code to the items on this machine
/// that can raise it: one line per item, each ending in the next move
/// on this platform. A code decided inside the city is said to be so,
/// and a code nobody knows is said to be unknown.
#[test]
fn explain_connects_a_refusal_code_to_what_this_machine_has() {
    let findings = examine(&ScriptedMachine::missing(&["python-wasi", "shell"]));
    let Explanation::Lines(lines) = explain("E_TOOL_UNAVAILABLE", &findings, Platform::current())
    else {
        panic!("E_TOOL_UNAVAILABLE is about this machine");
    };
    let python = lines
        .iter()
        .find(|line| line.contains("python-wasi"))
        .expect("the python component is one of the items");
    assert!(python.contains("absent"), "{python}");
    assert!(
        python.contains("->"),
        "the line ends in what to do next: {python}"
    );
    assert!(
        python.contains("components/python-wasi") || python.contains("manual"),
        "{python}"
    );
    let engine = lines
        .iter()
        .find(|line| line.contains("sandbox-engine"))
        .expect("the engine is one of the items");
    assert!(
        engine.ends_with("nothing to do"),
        "a present item has no next move: {engine}"
    );

    let Explanation::Lines(browser) =
        explain("E_BROWSER_UNAVAILABLE", &findings, Platform::current())
    else {
        panic!("E_BROWSER_UNAVAILABLE is about this machine");
    };
    assert!(browser.iter().any(|line| line.contains("firefox")));

    assert!(matches!(
        explain("E_GATE_DENIED", &findings, Platform::current()),
        Explanation::NotAboutThisMachine(kernel::AxCode::GateDenied)
    ));
    assert!(matches!(
        explain("E_BOGUS", &findings, Platform::current()),
        Explanation::NoSuchCode(_)
    ));
}

/// Checking is what `doctor` does; touching the machine takes the flag,
/// the one bare word is the city, and `--explain` takes the next word.
///
/// A verb that installed by default would act on a person who only
/// asked what they had, which is the one thing this verb must not do.
#[test]
fn the_line_is_read_as_flag_city_and_code() {
    let words =
        |raw: &[&str]| -> Vec<String> { raw.iter().map(|word| (*word).to_owned()).collect() };
    let plain = asked(&words(&["doctor"]));
    assert!(!plain.install && plain.city.is_none() && plain.explain.is_none());
    assert!(asked(&words(&["doctor", "--install"])).install);
    assert!(
        !asked(&words(&["doctor", "--installed"])).install,
        "a flag that only looks like --install is not --install"
    );
    let with_city = asked(&words(&["doctor", "C:/cities/one", "--install"]));
    assert_eq!(
        with_city.city.as_deref(),
        Some(std::path::Path::new("C:/cities/one"))
    );
    assert!(with_city.install);
    let explained = asked(&words(&["doctor", "--explain", "E_TOOL_UNAVAILABLE"]));
    assert_eq!(explained.explain.as_deref(), Some("E_TOOL_UNAVAILABLE"));
    assert!(
        explained.city.is_none(),
        "the code after --explain is not a city"
    );
}

/// `sprawling doctor <city>` names the building on the screen, and the
/// verdict is not ready while a building lacks what it asked for.
#[test]
fn doctor_with_a_city_names_the_building_on_the_screen() {
    let dir = tempfile::tempdir().unwrap();
    crate::assembly::init_city(dir.path()).unwrap();
    city::create_building(dir.path(), &lab(), city::BuildingTemplate::Minimal).unwrap();
    city::write_rules(dir.path(), &lab(), "confidential: false\nbrowser: true\n").unwrap();

    let machine = ScriptedMachine::missing(&["firefox", "chromedriver"]);
    let mut nobody = std::io::Cursor::new(Vec::new());
    let mut screen: Vec<u8> = Vec::new();
    let asked = Asked {
        install: false,
        city: Some(dir.path().to_path_buf()),
        explain: None,
    };
    let ready = run(&asked, &machine, &mut nobody, &mut screen).unwrap();
    assert!(!ready);
    let shown = String::from_utf8(screen).unwrap();
    assert!(shown.contains("lab: browser: true"), "{shown}");
    assert!(shown.contains("no firefox"), "{shown}");

    let machine = ScriptedMachine::missing(&[]);
    let mut screen: Vec<u8> = Vec::new();
    let ready = run(&asked, &machine, &mut nobody, &mut screen).unwrap();
    assert!(ready, "{}", String::from_utf8(screen).unwrap());
}

/// `--explain` is an explanation, so it exits with success whatever
/// the machine lacks, and prints one line per item.
#[test]
fn explain_on_the_screen_is_one_line_per_item_and_never_a_failure() {
    let machine = ScriptedMachine::missing(&["python-wasi"]);
    let mut nobody = std::io::Cursor::new(Vec::new());
    let mut screen: Vec<u8> = Vec::new();
    let asked = Asked {
        install: false,
        city: None,
        explain: Some("E_TOOL_UNAVAILABLE".to_owned()),
    };
    let ready = run(&asked, &machine, &mut nobody, &mut screen).unwrap();
    assert!(ready);
    let shown = String::from_utf8(screen).unwrap();
    assert!(shown.contains("python-wasi"), "{shown}");
    assert!(shown.contains("E_TOOL_UNAVAILABLE"), "{shown}");
}
