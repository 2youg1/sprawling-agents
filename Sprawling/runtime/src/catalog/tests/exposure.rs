// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A table moves a tool between the three exposures, nearer statements
/// first and a tool's own key before its namespace's; `tool` and `read`
/// stay core whatever is written; an `Off` tool reaches no request, index
/// or call, and the recorded table still lists it (§8-60, D111).
#[test]
fn a_catalog_table_moves_tools_between_core_dormant_and_off() {
    use kernel::config::{CatalogExposure, Exposure, ExposureFrom, ExposureTable};
    let mut catalog = Catalog::new();
    catalog.set_mode(Mode::Work);
    catalog.set_exposure(CatalogExposure {
        city: ExposureTable {
            tools: [("plan".to_owned(), Exposure::Core)].into(),
            namespaces: [("apps".to_owned(), Exposure::Off)].into(),
            skills: std::collections::BTreeMap::new(),
        },
        building: ExposureTable {
            tools: [
                ("exec".to_owned(), Exposure::Dormant),
                ("apps_mail".to_owned(), Exposure::Dormant),
                ("read".to_owned(), Exposure::Off),
            ]
            .into(),
            namespaces: std::collections::BTreeMap::new(),
            skills: std::collections::BTreeMap::new(),
        },
    });
    for name in ["read", "tool", "exec", "plan", "apps_mail", "apps_ping"] {
        catalog.file(&meta(name)).unwrap();
    }

    let carried: Vec<String> = catalog
        .tool_defs()
        .iter()
        .map(|def| def.name.as_str().to_owned())
        .collect();
    assert_eq!(carried, ["plan", "read", "tool"]);
    assert!(
        catalog.render().contains("- apps (1)"),
        "{}",
        catalog.render()
    );
    assert!(
        catalog
            .resolve_run(&ToolCall {
                id: "toolu_1".to_owned(),
                name: ToolName::parse("tool").unwrap(),
                args: Payload::of(
                    &serde_json::json!({ "op": "run", "name": "apps_ping", "args": {} })
                )
                .unwrap(),
            })
            .is_err()
    );

    let rows: Vec<(String, Exposure, ExposureFrom)> = catalog
        .tool_table()
        .unwrap()
        .record
        .rows
        .into_iter()
        .map(|row| (row.name, row.exposure, row.from))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "apps_mail".to_owned(),
                Exposure::Dormant,
                ExposureFrom::Building
            ),
            ("apps_ping".to_owned(), Exposure::Off, ExposureFrom::City),
            ("exec".to_owned(), Exposure::Dormant, ExposureFrom::Building),
            ("plan".to_owned(), Exposure::Core, ExposureFrom::City),
            ("read".to_owned(), Exposure::Core, ExposureFrom::Fixed),
            ("tool".to_owned(), Exposure::Core, ExposureFrom::Fixed),
        ]
    );
}

/// The skills open with the line that says what a skill is: without it,
/// the `skill` namespace's sentence reached no model, because no tool is
/// filed under that namespace.
#[test]
fn the_skills_open_with_the_skill_namespace_s_sentence() {
    let mut catalog = Catalog::new();
    catalog.admit_skill(skill("review")).unwrap();
    catalog.admit_skill(skill("release")).unwrap();
    catalog.set_mode(Mode::Work);
    let block = dormant_block(&catalog.render());
    let lines: Vec<&str> = block.lines().skip(1).collect();
    assert_eq!(
        lines,
        vec![
            "- skill (2): the steps this building follows for a kind of work",
            "- skill release: release: the steps this building follows for it, with examples",
            "- skill review: review: the steps this building follows for it, with examples",
        ]
    );
}
