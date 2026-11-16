// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use super::drift::{Callers, Calling, Seen};
use super::*;

#[test]
fn a_run_is_offered_the_tools_the_worker_was_handed() {
    let offered = Arc::new(Mutex::new(Vec::new()));
    let dispatched = dispatch_one_run(
        Box::new(Listeners(Arc::clone(&offered))),
        Box::new(Scripted(LISTING.to_owned())),
    );

    assert!(
        offered
            .lock()
            .unwrap()
            .iter()
            .any(|name| *name == format!("{NAMESPACE} (1)")),
        "the model was never offered {OFFERED} under {NAMESPACE}: offered {:?}; the dispatch \
         answered {dispatched}",
        offered.lock().unwrap()
    );
}

/// A server of two hundred tools takes one line of the dormant index,
/// and `tool` `find` names the one a few words describe among its first
/// eight hits (`Sprawling/runtime/Spec.lean` §8-60, D110).
#[test]
fn two_hundred_tools_of_one_server_take_one_index_line_and_are_found() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let dispatched = dispatch_one_run(
        Box::new(Callers(Calling {
            seen: Arc::clone(&seen),
            tool: "tool",
            args: serde_json::json!({"op": "find", "query": "create issue"}),
        })),
        Box::new(Scripted(two_hundred_tools())),
    );

    let seen = seen.lock().unwrap();
    let lines: Vec<&String> = seen
        .index
        .iter()
        .filter(|name| name.starts_with(NAMESPACE))
        .collect();
    assert_eq!(
        lines,
        [&format!("{NAMESPACE} (200)")],
        "index {:?}; the dispatch answered {dispatched}",
        seen.index
    );
    let found = seen.read.first().map(String::as_str).unwrap_or_default();
    let hits: Vec<&str> = found
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert!(
        hits.len() <= 8
            && hits
                .iter()
                .any(|hit| hit.starts_with("- apps_create_issue:")),
        "find answered {found:?}; the dispatch answered {dispatched}"
    );
}

/// The `apps` rows of the building's capabilities answer.
pub(crate) fn apps_rows(city_root: &std::path::Path) -> Vec<wire::CatalogTool> {
    let answer = accounting::views::ask(
        city_root,
        &wire::Query::Catalog {
            building: Address::parse(LAB).unwrap(),
        },
    )
    .unwrap();
    let wire::Answer::Catalog(catalog) = answer else {
        panic!("the capabilities question answered {answer:?}");
    };
    catalog
        .tools
        .into_iter()
        .filter(|tool| tool.namespace == NAMESPACE)
        .collect()
}

/// The capabilities answer lists every tool of a two-hundred-tool server
/// the newest run was frozen with, and a namespace the building turns off
/// is gone from the next run's index and tool list while the answer
/// still lists it, off, so a person can turn it back on
/// (`Sprawling/wire/spec/Answer/Catalog.lean` §8-99, runtime D111).
#[test]
fn a_namespace_a_building_turns_off_leaves_the_next_run_and_stays_listed() {
    use kernel::config::{Exposure, ExposureFrom, ExposureTable};
    let offered = Arc::new(Mutex::new(Vec::new()));
    let (dir, mut worker) = lab_city(
        Box::new(Listeners(Arc::clone(&offered))),
        Box::new(Scripted(two_hundred_tools())),
    );
    let first = dispatch(&mut worker, "s1");
    let rows = apps_rows(dir.path());
    assert_eq!(rows.len(), 200, "the first dispatch answered {first}");
    assert!(
        rows.iter()
            .all(|row| row.exposure == Exposure::Dormant && row.from == ExposureFrom::Default),
        "{rows:?}"
    );
    assert!(
        offered
            .lock()
            .unwrap()
            .contains(&format!("{NAMESPACE} (200)")),
        "{:?}",
        offered.lock().unwrap()
    );

    offered.lock().unwrap().clear();
    worker
        .handle(wire::Command::ConfigureBuilding(wire::BuildingSettings {
            addr: Address::parse(LAB).unwrap(),
            sandbox: None,
            mcp: None,
            desktop: None,
            context_second_threshold: None,
            catalog: Some(ExposureTable {
                tools: std::collections::BTreeMap::new(),
                namespaces: [(NAMESPACE.to_owned(), Exposure::Off)].into(),
                skills: std::collections::BTreeMap::new(),
            }),
            mcp_references: None,
            skill_pins: None,
            reading_room: None,
            idem: idem(b"catalog"),
        }))
        .unwrap();
    let second = dispatch(&mut worker, "s2");

    let named: Vec<String> = offered.lock().unwrap().clone();
    assert!(
        !named.is_empty() && !named.iter().any(|name| name.starts_with(NAMESPACE)),
        "the next run was offered {named:?}; the second dispatch answered {second}"
    );
    let rows = apps_rows(dir.path());
    assert_eq!(rows.len(), 200);
    assert!(
        rows.iter()
            .all(|row| row.exposure == Exposure::Off && row.from == ExposureFrom::Building),
        "{rows:?}"
    );
}

pub(crate) fn apps_reference(city_root: &std::path::Path) -> kernel::McpReference {
    city::load_config(city_root, &Address::parse("lab/room1").unwrap())
        .unwrap()
        .mcp_references
        .into_iter()
        .find(|reference| reference.label.as_str() == NAMESPACE)
        .unwrap()
}

pub(crate) fn catalog_drifts(
    city_root: &std::path::Path,
) -> Vec<kernel::event::record::CatalogDrift> {
    storage::read_raw_lines_at(&kernel::layout::CityLayout::new(city_root).ledger())
        .unwrap()
        .iter()
        .map(|line| kernel::EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == kernel::EventKind::DigestInvalidated)
        .map(|record| {
            record
                .data()
                .read::<kernel::event::record::CatalogDrift>()
                .unwrap()
        })
        .collect()
}

pub(crate) fn assert_only_namespace_count(offered: &Arc<Mutex<Vec<String>>>, count: usize) {
    let names = offered.lock().unwrap();
    assert!(
        names
            .iter()
            .any(|name| name == &format!("{NAMESPACE} ({count})")),
        "expected {count} offered tools, saw {names:?}"
    );
}

#[test]
fn the_first_listing_migrates_legacy_pins_and_an_equal_listing_is_offered_again() {
    let (dir, mut worker, _listing, offered) = revision_city(LISTING);
    let first = dispatch(&mut worker, "s1");
    let reference = apps_reference(dir.path());
    let pinned = reference
        .tools
        .as_ref()
        .unwrap()
        .get(OFFERED)
        .copied()
        .unwrap();
    let expected = kernel::ToolDef {
        name: kernel::ToolName::parse(OFFERED).unwrap(),
        description: "answer with pong".to_owned(),
        input_schema: kernel::Payload::of(&serde_json::json!({"type": "object"})).unwrap(),
    }
    .digest()
    .unwrap();
    assert_eq!(pinned, expected, "the first dispatch answered {first}");
    assert_eq!(reference.definition, Some(apps_definition(dir.path())));
    let bytes = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas())
        .unwrap()
        .get(&pinned)
        .unwrap();
    assert_eq!(
        bytes,
        kernel::ToolDef {
            name: kernel::ToolName::parse(OFFERED).unwrap(),
            description: "answer with pong".to_owned(),
            input_schema: kernel::Payload::of(&serde_json::json!({"type": "object"})).unwrap(),
        }
        .pin_bytes()
        .unwrap()
    );

    offered.lock().unwrap().clear();
    let second = dispatch(&mut worker, "s2");
    assert_only_namespace_count(&offered, 1);
    assert!(catalog_drifts(dir.path()).is_empty(), "{first}; {second}");
}

fn apps_definition(city_root: &std::path::Path) -> kernel::B3Hash {
    let config = city::load_config(city_root, &Address::parse("lab/room1").unwrap()).unwrap();
    let server = config
        .mcp
        .iter()
        .find(|server| server.label.as_str() == NAMESPACE)
        .unwrap();
    kernel::config::definition_digest(server).unwrap()
}

#[test]
fn a_changed_tool_is_withheld_and_its_drift_is_recorded_only_once() {
    let initial = listing(&[("ping", "old ping"), ("pong", "unchanged pong")]);
    let (dir, mut worker, listing, offered) = revision_city(&initial);
    dispatch(&mut worker, "s1");
    *listing.lock().unwrap() = Some(self::listing(&[
        ("ping", "new ping"),
        ("pong", "unchanged pong"),
    ]));

    offered.lock().unwrap().clear();
    let second = dispatch(&mut worker, "s2");
    assert_only_namespace_count(&offered, 1);
    let drifts = catalog_drifts(dir.path());
    assert_eq!(drifts.len(), 1, "the second dispatch answered {second}");
    assert!(matches!(
        &drifts[0],
        kernel::event::record::CatalogDrift {
            subject: kernel::event::record::CatalogSubject::Tool { name, .. },
            pinned: Some(_),
            found: Some(_),
        } if name == OFFERED
    ));
    let rows = apps_rows(dir.path());
    let changed = rows.iter().find(|row| row.name == OFFERED).unwrap();
    assert_eq!(changed.change, Some(wire::CatalogChange::Changed));
    assert_eq!(changed.current.as_ref().unwrap().description, "new ping");
    assert_eq!(changed.pinned.as_ref().unwrap().description, "old ping");
    let unchanged = rows.iter().find(|row| row.name == "apps_pong").unwrap();
    assert_eq!(unchanged.change, None);

    offered.lock().unwrap().clear();
    dispatch(&mut worker, "s3");
    assert_only_namespace_count(&offered, 1);
    assert_eq!(catalog_drifts(dir.path()), drifts);
}

#[test]
fn an_added_tool_is_withheld_until_it_is_admitted() {
    let (dir, mut worker, listing, offered) = revision_city(LISTING);
    dispatch(&mut worker, "s1");
    *listing.lock().unwrap() = Some(self::listing(&[
        ("ping", "answer with pong"),
        ("new", "not admitted"),
    ]));

    offered.lock().unwrap().clear();
    dispatch(&mut worker, "s2");
    assert_only_namespace_count(&offered, 1);
    let drifts = catalog_drifts(dir.path());
    assert_eq!(drifts.len(), 1);
    assert!(matches!(
        &drifts[0],
        kernel::event::record::CatalogDrift {
            subject: kernel::event::record::CatalogSubject::Tool { name, .. },
            pinned: None,
            found: Some(_),
        } if name == "apps_new"
    ));
    let rows = apps_rows(dir.path());
    let added = rows.iter().find(|row| row.name == "apps_new").unwrap();
    assert_eq!(added.change, Some(wire::CatalogChange::Added));
    assert_eq!(added.current.as_ref().unwrap().description, "not admitted");
    assert_eq!(added.pinned, None);

    let mut reference = apps_reference(dir.path());
    reference.tools.as_mut().unwrap().insert(
        "apps_new".to_owned(),
        added.current.as_ref().unwrap().digest,
    );
    worker
        .handle(wire::Command::ConfigureBuilding(wire::BuildingSettings {
            addr: Address::parse(LAB).unwrap(),
            sandbox: None,
            mcp: None,
            desktop: None,
            context_second_threshold: None,
            catalog: None,
            mcp_references: Some(vec![reference]),
            skill_pins: None,
            reading_room: None,
            idem: idem(b"accept-added-tool"),
        }))
        .unwrap();
    offered.lock().unwrap().clear();
    dispatch(&mut worker, "s3");
    assert_only_namespace_count(&offered, 2);
    assert!(apps_rows(dir.path()).iter().all(|row| row.change.is_none()));
}
