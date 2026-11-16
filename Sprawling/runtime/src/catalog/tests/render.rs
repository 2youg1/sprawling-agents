// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn render_is_deterministic_and_sorted() {
    let mut catalog = Catalog::new();
    catalog.file(&meta("zeta")).unwrap();
    catalog.file(&meta("alpha")).unwrap();
    catalog.file(&meta("zulu_mail")).unwrap();
    catalog.file(&meta("apps_mail")).unwrap();
    catalog.admit_skill(skill("review")).unwrap();
    catalog.set_mode(Mode::Work);
    let text = catalog.render();
    let block = dormant_block(&text);
    let apps = block.find("- apps (1):").unwrap();
    let sprawling = block.find("- sprawling (2):").unwrap();
    let zulu = block.find("- zulu (1):").unwrap();
    let review = block.find("- skill review:").unwrap();
    assert!(
        apps < sprawling && sprawling < zulu,
        "namespaces by name, a server's beside the city's own: {block}"
    );
    assert!(zulu < review, "namespaces, then skills");
    assert!(text.contains("- mode:work:"));
    assert_eq!(text, catalog.render(), "same content, same bytes");
    // One line says the city itself can be changed; the evidence a
    // change can be asked for and the reading order sit behind an
    // expansion nobody pays for until they ask.
    assert!(text.contains("- dev: when the work is to change"));
    assert!(!text.contains("held-out evidence"), "the detail is fetched");
    let Some(Expansion::Said { text: detail }) = catalog.expand("dev") else {
        panic!("the developer entry expands");
    };
    assert!(detail.contains("-SPEC.md"));
    assert!(detail.contains("held-out evidence"));
}

#[test]
fn duplicates_and_empty_disclosures_are_refused() {
    let mut catalog = Catalog::new();
    catalog.file(&meta("probe")).unwrap();
    assert!(catalog.file(&meta("probe")).is_err());
    let mut empty = meta("hollow");
    empty.disclosure = "  ".to_owned();
    assert!(catalog.file(&empty).is_err());
}

/// The mode's core travels as tools and nothing else does: an admitted
/// tool outside the core is a line of the index and not a definition.
#[test]
fn the_union_of_the_modes_cores_travels_as_tools_in_every_mode() {
    let mut catalog = Catalog::new();
    for name in ["read", "edit", "probe", "tool"] {
        catalog.file(&meta(name)).unwrap();
    }
    catalog.set_mode(Mode::Chat);
    let names = |catalog: &Catalog| -> Vec<String> {
        catalog
            .tool_defs()
            .iter()
            .map(|def| def.name.as_str().to_owned())
            .collect()
    };
    assert_eq!(names(&catalog), ["edit", "read", "tool"]);
    let text = catalog.render();
    assert!(!dormant_block(&text).contains("- edit:"));
    assert!(dormant_block(&text).contains("- sprawling (1):"));
    assert!(catalog.list(Some("sprawling")).contains("- probe:"));
    assert!(
        !dormant_block(&text).contains("- read:"),
        "a core tool is listed once"
    );
    catalog.set_mode(Mode::Work);
    assert_eq!(names(&catalog), ["edit", "read", "tool"]);
    assert!(matches!(
        catalog.expand("mode:work"),
        Some(Expansion::Said { .. })
    ));
}

/// The fixture building: the work core, a dozen dormant tools shaped like
/// MCP tools, and four skills. The three tiers pinned at once, and the
/// bytes the request pays read off before and after.
#[test]
fn a_fixture_buildings_tiers_cost_what_the_lock_allows() {
    let mut catalog = Catalog::new();
    let core = ["edit", "exec", "read", "search", "status", "tool"];
    for name in core {
        catalog.file(&sized(name)).unwrap();
    }
    let dormant: Vec<String> = (0..12).map(|n| format!("apps_service_{n:02}")).collect();
    for name in &dormant {
        catalog.file(&sized(name)).unwrap();
    }
    for name in ["review", "release", "triage", "migrate"] {
        catalog.admit_skill(skill(name)).unwrap();
    }
    catalog.set_mode(Mode::Work);

    let text = catalog.render();
    let block = dormant_block(&text);
    let defs = catalog.tool_defs();
    let tools_bytes = serde_json::to_string(&defs).unwrap().len();
    assert!(
        block.len() <= DORMANT_INDEX_CEILING,
        "{} bytes",
        block.len()
    );
    assert_eq!(
        defs.len(),
        core.len(),
        "the core and nothing else is a definition"
    );
    assert!(
        block.contains("- apps (12): service_00, service_01"),
        "{block}"
    );
    for name in &dormant {
        assert!(
            catalog.list(Some("apps")).contains(&format!("- {name}:")),
            "{name} is listed in its namespace"
        );
    }

    // What the same building cost when every admitted tool travelled as
    // a definition and every skill was a line of its own.
    let mut every = Catalog::new();
    for name in core
        .iter()
        .copied()
        .chain(dormant.iter().map(String::as_str))
    {
        every.file(&sized(name)).unwrap();
    }
    let all_defs: Vec<ToolDef> = every
        .tools
        .values()
        .map(|filed| filed.def.clone())
        .collect();
    let before_tools = serde_json::to_string(&all_defs).unwrap().len();
    let before_skills: usize = ["review", "release", "triage", "migrate"]
        .iter()
        .map(|name| format!("- skill {name}: {}\n", skill(name).disclosure).len())
        .sum();
    println!(
        "fixture: before tools {before_tools} B + skill lines {before_skills} B; \
         after tools {tools_bytes} B + dormant index {} B",
        block.len()
    );
    assert!(tools_bytes + block.len() < before_tools + before_skills);

    // Tier one: a capability the building did not admit is in no list,
    // no index and no answer.
    let unadmitted = "mail_send";
    assert!(!text.contains(unadmitted));
    assert!(catalog.find(unadmitted).starts_with("Nothing admitted"));
    assert!(catalog.detail(unadmitted).is_err());
    assert!(
        catalog
            .resolve_run(&call(serde_json::json!({
                "op": "run", "name": unadmitted, "args": {}
            })))
            .is_err()
    );
}

/// However much a building admits, the index stays under its ceiling, and
/// the entries it cannot hold are counted rather than dropped silently.
#[test]
fn the_index_stays_under_its_ceiling_and_counts_what_it_leaves_out() {
    let mut catalog = Catalog::new();
    for n in 0..200 {
        catalog.file(&sized(&format!("apps{n:03}_tool"))).unwrap();
    }
    catalog.set_mode(Mode::Work);
    let text = catalog.render();
    let block = dormant_block(&text);
    assert!(
        block.len() <= DORMANT_INDEX_CEILING,
        "{} bytes",
        block.len()
    );
    let last = block.lines().last().unwrap();
    assert!(last.starts_with('+') && last.ends_with(" more"), "{last}");
    let listed = block.lines().filter(|line| line.starts_with("- ")).count();
    let left: usize = last
        .trim_start_matches('+')
        .trim_end_matches(" more")
        .parse()
        .unwrap();
    assert_eq!(listed + left, 200);
}

/// A hint is cut on a character boundary, so a disclosure in any script
/// leaves the index readable text.
#[test]
fn a_hint_is_cut_on_a_character_boundary() {
    let long = "检索这栋楼收到的邮件并按发件人归类，每一类给出最近的三封与它们的摘要";
    assert!(long.len() > dormant::HINT_MAX_BYTES);
    let hint = dormant::hint(long);
    assert!(hint.len() <= dormant::HINT_MAX_BYTES);
    assert!(long.starts_with(hint));
    assert_eq!(dormant::hint("Read a file. Then more."), "Read a file");
}

/// The sentence the city introduces each of its own namespaces by fits
/// the index's hint whole: a model reads it in the index and in `tool`
/// `list`, and one cut in the middle of a word tells it less.
#[test]
fn the_city_s_own_namespaces_are_introduced_by_a_whole_sentence() {
    let cut: Vec<(&str, String)> = kernel::ReservedNamespace::ALL
        .iter()
        .map(|own| {
            let about = namespace::Namespace::Own(*own).about(std::iter::empty());
            (own.as_str(), about)
        })
        .filter(|(_, about)| dormant::hint(about) != about.as_str())
        .collect();
    assert_eq!(cut, Vec::<(&str, String)>::new());
}
