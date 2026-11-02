// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{Address, B3Hash, RunId, Seq};

use super::{PromptTools, ToolRead, tool_rows};

fn city_with_lab() -> (tempfile::TempDir, Address) {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let lab = Address::parse("lab").unwrap();
    city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();
    (dir, lab)
}

fn install(city_root: &Path, lab: &Address, name: &str, text: &str) -> B3Hash {
    let source = city_root.join("fixtures").join(format!("{name}.md"));
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, text).unwrap();
    let mut cas = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).unwrap();
    city::install_skill(
        city_root,
        &city::Slot::building(lab, "catalog-tests").unwrap(),
        &source,
        &mut |bytes| cas.put(bytes).map_err(storage::StorageError::into_ax),
    )
    .unwrap()
    .hash
}

fn admit(city_root: &Path, lab: &Address, names: &[&str]) {
    let path = city::rules_path(city_root, lab);
    let base = std::fs::read_to_string(&path).unwrap();
    let mut document: toml::Table = toml::from_str(&base).unwrap();
    document.insert(
        "reading_room".to_owned(),
        toml::Value::Array(
            names
                .iter()
                .map(|name| toml::Value::String((*name).to_owned()))
                .collect(),
        ),
    );
    let body = toml::to_string_pretty(&document).unwrap();
    city::write_rules_against(city_root, lab, &base, &body).unwrap();
}

fn save_pins(city_root: &Path, lab: &Address, name: &str, pin: B3Hash) {
    save_pins_result(city_root, lab, name, pin).unwrap();
}

/// The same save, with the refusal in hand: a red test asserts the code
/// and the untouched building beside it.
fn save_pins_result(
    city_root: &Path,
    lab: &Address,
    name: &str,
    pin: B3Hash,
) -> Result<(), kernel::AxError> {
    run_settings(
        city_root,
        wire::BuildingSettings {
            addr: lab.clone(),
            sandbox: None,
            mcp: None,
            desktop: None,
            context_second_threshold: None,
            catalog: None,
            mcp_references: None,
            skill_pins: Some(BTreeMap::from([(name.to_owned(), pin)])),
            reading_room: Some(vec![name.to_owned()]),
            idem: kernel::IdemKey::derive(
                &RunId::CITY,
                Seq::FIRST,
                format!("save-pins:{name}:{pin}").as_bytes(),
            ),
        },
    )
}

/// The pins face alone, without the reading-room face: the half one
/// save leaves when its second door never runs
/// (`crates/accounting/spec/Connectors.lean` D121).
fn pin_only_result(
    city_root: &Path,
    lab: &Address,
    name: &str,
    pin: B3Hash,
) -> Result<(), kernel::AxError> {
    run_settings(
        city_root,
        wire::BuildingSettings {
            addr: lab.clone(),
            sandbox: None,
            mcp: None,
            desktop: None,
            context_second_threshold: None,
            catalog: None,
            mcp_references: None,
            skill_pins: Some(BTreeMap::from([(name.to_owned(), pin)])),
            reading_room: None,
            idem: kernel::IdemKey::derive(
                &RunId::CITY,
                Seq::FIRST,
                format!("pin-only:{name}:{pin}").as_bytes(),
            ),
        },
    )
}

/// Runs one settings frame through a fresh worker: the construction
/// every save in this module shares.
fn run_settings(city_root: &Path, settings: wire::BuildingSettings) -> Result<(), kernel::AxError> {
    let mut worker = crate::worker::RunWorker::new(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker.handle(wire::Command::ConfigureBuilding(settings))
}

fn catalog(city_root: &Path, lab: &Address) -> wire::CatalogAnswer {
    let wire::Answer::Catalog(answer) = crate::views::ask(
        city_root,
        &wire::Query::Catalog {
            building: lab.clone(),
        },
    )
    .unwrap() else {
        panic!("Catalog answered another shape")
    };
    *answer
}

fn skill<'a>(answer: &'a wire::CatalogAnswer, name: &str) -> &'a wire::CatalogSkill {
    answer
        .skill_rows
        .iter()
        .find(|row| row.name == name)
        .unwrap()
}

fn candidate(city_root: &Path, lab: &Address, name: &str) -> B3Hash {
    let home = crate::home::Home::detect().unwrap();
    let cas = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).unwrap();
    city::Library::scan(city_root, Some(lab), home.path(), &|digest| {
        cas.get(digest).map_err(storage::StorageError::into_ax)
    })
    .unwrap()
    .all()
    .into_iter()
    .find(|holding| holding.name == name)
    .unwrap()
    .hash
}

#[test]
fn an_accepted_tool_removal_leaves_no_historical_ghost_row() {
    let dir = tempfile::tempdir().unwrap();
    let label = kernel::ServerLabel::parse("docs").unwrap();
    let drift = kernel::event::record::CatalogDrift {
        subject: kernel::event::record::CatalogSubject::Tool {
            label: label.clone(),
            name: "search".to_owned(),
        },
        pinned: Some(B3Hash::digest(b"old definition")),
        found: None,
    };
    let references = vec![kernel::McpReference {
        label,
        definition: None,
        tools: Some(BTreeMap::new()),
    }];
    let evidence = crate::views::usage::CatalogEvidence::default();
    let drifts = vec![(Seq::FIRST, drift)];
    let cas = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas()).unwrap();
    let rows = tool_rows(
        &PromptTools {
            hash: B3Hash::digest(b""),
            len: 0,
            index: 0,
            reading: 0,
            always: 0,
            dormant: 0,
            skills: 0,
            rows: Vec::new(),
        },
        ToolRead {
            stated: &kernel::config::CatalogExposure::default(),
            references: &references,
            evidence: &evidence,
            drifts: &drifts,
            cas: &cas,
            newest: None,
        },
    )
    .unwrap();
    assert_eq!(rows, Vec::new());
}

#[test]
fn an_internal_skill_compares_two_cas_documents_and_accept_moves_the_pin() {
    let (dir, lab) = city_with_lab();
    let old = install(dir.path(), &lab, "guide", "# Old guide\n\nold body\n");
    admit(dir.path(), &lab, &["guide"]);
    save_pins(dir.path(), &lab, "guide", old);
    let new = install(dir.path(), &lab, "guide", "# New guide\n\nnew body\n");

    let answer = catalog(dir.path(), &lab);
    let row = skill(&answer, "guide");
    assert_eq!(row.change, Some(wire::CatalogChange::Changed));
    assert_eq!(row.candidate, Some(new));
    assert_eq!(row.pinned, Some(old));
    assert_eq!(
        row.current_document.as_deref(),
        Some("# New guide\n\nnew body\n")
    );
    assert_eq!(
        row.pinned_document.as_deref(),
        Some("# Old guide\n\nold body\n")
    );
    assert_eq!(answer.changed, 1);
    assert_eq!(answer.reading_room, ["guide"]);

    save_pins(dir.path(), &lab, "guide", new);
    let answer = catalog(dir.path(), &lab);
    assert_eq!(skill(&answer, "guide").change, None);
    assert_eq!(answer.changed, 0);
}

#[test]
fn an_accepted_skill_removal_leaves_no_historical_ghost_row() {
    let (dir, lab) = city_with_lab();
    let old = install(dir.path(), &lab, "guide", "# Old guide\n");
    admit(dir.path(), &lab, &["guide"]);
    save_pins(dir.path(), &lab, "guide", old);
    let index = kernel::layout::CityLayout::new(dir.path())
        .building_skills(&lab)
        .join(kernel::layout::SHELF_INDEX);
    std::fs::write(&index, "version = 1\n").unwrap();
    run_settings(
        dir.path(),
        wire::BuildingSettings {
            addr: lab.clone(),
            sandbox: None,
            mcp: None,
            desktop: None,
            context_second_threshold: None,
            catalog: None,
            mcp_references: None,
            skill_pins: Some(BTreeMap::new()),
            reading_room: Some(vec!["guide".to_owned()]),
            idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, b"accept-skill-removal"),
        },
    )
    .unwrap();

    let answer = catalog(dir.path(), &lab);
    assert!(
        !answer.skill_rows.iter().any(|row| row.name == "guide"),
        "an accepted removal left a ghost row: {:?}",
        answer.skill_rows
    );
}

/// The page accepted an external skill whose shelf rewrote it before the
/// save. The pinned version was never admitted to the store, so the save
/// refuses before anything lands: a pin that names bytes no read can
/// answer would take the catalog panel and the building's runs down
/// (`crates/accounting/spec/Connectors.lean` D119).
#[test]
fn a_pin_for_a_rewritten_external_skill_is_refused_before_anything_lands() {
    let (dir, lab) = city_with_lab();
    let outside = dir.path().join("outside-skills");
    let package = outside.join("roam");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("SKILL.md"), "# Outside one\n").unwrap();
    std::fs::write(
        dir.path()
            .join(kernel::RESERVED_PREFIX)
            .join(city::CONFIG_FILE),
        format!(
            "[skills]\nshelves = [{}]\n",
            serde_json::to_string(&outside.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    admit(dir.path(), &lab, &["roam"]);
    // What the page's answer showed; the shelf then moves before the save.
    let stale = candidate(dir.path(), &lab, "roam");
    std::fs::write(package.join("SKILL.md"), "# Outside two\n").unwrap();

    let refused = save_pins_result(dir.path(), &lab, "roam", stale).unwrap_err();

    assert_eq!(refused.code(), &kernel::AxCode::VersionConflict);
    assert!(
        refused.recovery().contains("catalog"),
        "the recovery names where the person settles the change: {}",
        refused.recovery()
    );
    let file = kernel::layout::CityLayout::new(dir.path()).config(&lab);
    assert!(
        !file.exists(),
        "a refused save left {} behind",
        file.display()
    );
}

/// One save lands its CONFIG.toml face and then its RULES.toml face; the
/// half where pins landed and the admission did not must still read —
/// the skill shows pinned and withheld — and the next save converges
/// (`crates/accounting/spec/Connectors.lean` D121).
#[test]
fn pins_without_a_reading_room_entry_read_as_pinned_and_withheld() {
    let (dir, lab) = city_with_lab();
    let digest = install(dir.path(), &lab, "guide", "# Guide\n");
    pin_only_result(dir.path(), &lab, "guide", digest).unwrap();

    let answer = catalog(dir.path(), &lab);
    assert_eq!(answer.skill_pins.get("guide"), Some(&digest));
    let row = skill(&answer, "guide");
    assert_eq!(row.pinned, Some(digest));
    assert_eq!(row.change, None);
    assert_eq!(
        row.exposure,
        kernel::config::Exposure::Off,
        "a pinned skill the reading room does not admit is withheld"
    );

    save_pins(dir.path(), &lab, "guide", digest);
    let answer = catalog(dir.path(), &lab);
    assert_eq!(
        skill(&answer, "guide").exposure,
        kernel::config::Exposure::Dormant,
        "the next save admits the pinned skill and converges"
    );
}

#[test]
fn an_external_accept_copies_the_candidate_and_keep_leaves_the_old_pin() {
    let (dir, lab) = city_with_lab();
    let outside = dir.path().join("outside-skills");
    let package = outside.join("roam");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("SKILL.md"), "# Outside one\n").unwrap();
    std::fs::write(
        dir.path()
            .join(kernel::RESERVED_PREFIX)
            .join(city::CONFIG_FILE),
        format!(
            "[skills]\nshelves = [{}]\n",
            serde_json::to_string(&outside.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    admit(dir.path(), &lab, &["roam"]);
    let old = candidate(dir.path(), &lab, "roam");
    save_pins(dir.path(), &lab, "roam", old);

    std::fs::write(package.join("SKILL.md"), "# Outside two\n").unwrap();
    let new = candidate(dir.path(), &lab, "roam");
    let answer = catalog(dir.path(), &lab);
    let row = skill(&answer, "roam");
    assert_eq!(row.change, Some(wire::CatalogChange::Changed));
    assert_eq!(row.current_document.as_deref(), Some("# Outside two\n"));
    assert_eq!(row.pinned_document.as_deref(), Some("# Outside one\n"));

    save_pins(dir.path(), &lab, "roam", old);
    assert_eq!(catalog(dir.path(), &lab).skill_pins.get("roam"), Some(&old));

    save_pins(dir.path(), &lab, "roam", new);
    let answer = catalog(dir.path(), &lab);
    assert_eq!(skill(&answer, "roam").change, None);
    let cas = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas()).unwrap();
    assert_eq!(
        city::skill_document(
            &|digest| cas.get(digest).map_err(storage::StorageError::into_ax),
            &new,
            "roam",
        )
        .unwrap(),
        "# Outside two\n"
    );
}
