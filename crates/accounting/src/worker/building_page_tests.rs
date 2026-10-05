// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::views::building_page::read_building;
use crate::worker::fixture::*;

/// The rules a person may read on the building page are the rules
/// the city obeys, and the page reads them from a directory the
/// walk deliberately skips.
#[test]
fn a_building_page_still_shows_the_rules_that_govern_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    let lab = Address::parse("lab").unwrap();
    let plan = crate::plan_view::PlanView::default().of(dir.path(), &lab);
    let answer = read_building(dir.path(), &lab, plan).expect("a created building has a page");
    let rules = answer
        .docs
        .iter()
        .find(|doc| doc.name == city::RULES_FILE)
        .expect("the page lost the tab that says what this building may do");
    assert!(rules.text.contains("confidential"), "{}", rules.text);
    assert!(
        !answer.rooms.iter().any(|room| room.starts_with('.')),
        "a reserved subtree is not a room: {:?}",
        answer.rooms
    );
}

/// A plan nobody can open and a plan somebody wrote badly are two
/// different facts, and the page states the first.
///
/// Reading the file as empty runs it through `check_roadmap_shape`,
/// which finds no header row and answers `no six-column table
/// found`. That sends a person to edit a table when what they have
/// to fix is a file that will not open - the same misreport already
/// removed from the dispatch path, still standing on the page.
#[test]
fn a_building_page_says_the_plan_cannot_be_read_rather_than_that_it_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let lab = Address::parse("lab").unwrap();
    city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();
    // A directory where the plan belongs, so the read fails for a
    // reason that is not "it is not there yet".
    let plan = city::roadmap_path(dir.path(), &lab);
    let _ = std::fs::remove_file(&plan);
    std::fs::create_dir_all(&plan).unwrap();

    let plan = crate::plan_view::PlanView::default().of(dir.path(), &lab);
    let answer = read_building(dir.path(), &lab, plan).expect("the building is still a building");
    assert!(
        answer
            .problems
            .iter()
            .any(|problem| problem.contains(city::ROADMAP_FILE)),
        "the page has to name the file a person must fix: {:?}",
        answer.problems
    );
}

#[test]
fn a_city_where_the_building_is_gone_hears_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::write(
        city::watch_path(dir.path()),
        "[[source]]
name = \"github\"
matches = \"pr\"
addr = \"gone/room1\"
",
    )
    .unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("unused", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    // Not a refusal: nothing listening is a fact about the city, and
    // the person who wrote the table is the one who can act on it.
    worker
        .handle(wire::Command::Wake {
            source: "github".to_owned(),
            subject: "pr opened".to_owned(),
            body: "x".to_owned(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"wake"),
        })
        .unwrap();
    drop(provider);
}

/// `[sandbox]` and `[mcp]` resolve city -> building -> room and
/// nothing wrote either, so a person was governed by settings they
/// could not change without a text editor.
#[test]
fn a_building_can_be_told_what_its_runs_may_reach() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let before = city::load_config(dir.path(), &room).unwrap();
    assert!(!before.sandbox.shell, "the shell arm is off by default");

    worker
        .handle(wire::Command::ConfigureBuilding {
            addr: room.clone(),
            sandbox: Some(kernel::SandboxLimits {
                shell: true,
                interpreter: kernel::Interpreter::Pwsh,
                fuel: 4096,
                mounts: vec![Address::parse("lab/shared").unwrap()],
                env_passthrough: Vec::new(),
                trusted: Vec::new(),
                container: None,
            }),
            mcp: Some(vec![kernel::McpServer {
                label: kernel::ServerLabel::parse("docs").unwrap(),
                transport: kernel::McpTransport::Http {
                    url: "https://mcp.example/v1".to_owned(),
                    headers: Vec::new(),
                },
            }]),
            desktop: None,
            context_second_threshold: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"reach"),
        })
        .unwrap();

    // The ladder is the authority: what a run in the room resolves
    // to is what the building's own rung now says.
    let after = city::load_config(dir.path(), &room).unwrap();
    assert!(after.sandbox.shell);
    assert_eq!(after.sandbox.interpreter, kernel::Interpreter::Pwsh);
    assert_eq!(after.sandbox.fuel, 4096);
    assert_eq!(after.mcp.len(), 1);
    assert_eq!(after.mcp[0].label.as_str(), "docs");

    // And the page reads the building's own rung back, not the
    // resolved value, so saving twice does not copy the city's
    // settings down into the building.
    let lab = Address::parse("lab").unwrap();
    let plan = crate::plan_view::PlanView::default().of(dir.path(), &lab);
    let shown = read_building(dir.path(), &lab, plan).expect("the building page has an answer");
    assert_eq!(shown.mcp.len(), 1);
    assert!(shown.sandbox.is_some_and(|limits| limits.shell));
}

/// The desktop allowlist travels on the same frame as the rest of
/// what a building's runs may reach, and lands where no write domain
/// goes (`crates/city/Spec.lean` §8-26).
///
/// The bytes a person wrote are the bytes on disk: this side never
/// parses the file, because the connector that reads it at start-up
/// is the authority on its syntax and fails closed.
#[test]
fn the_desktop_allowlist_is_written_where_no_resident_reaches_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let lab = Address::parse("lab").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: lab.clone(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();

    let allowlist = "[[window]]\ntitle = \"kusanagi\"\n";
    worker
        .handle(wire::Command::ConfigureBuilding {
            addr: lab.clone(),
            sandbox: None,
            mcp: None,
            desktop: Some(allowlist.to_owned()),
            context_second_threshold: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"desktop"),
        })
        .unwrap();

    let at = city::desktop_scope_path(dir.path(), &lab);
    assert_eq!(
        std::fs::read_to_string(&at).expect("the allowlist is on disk"),
        allowlist,
        "the bytes a person wrote are the bytes the connector reads"
    );
    let inside = Address::parse(&format!(
        "lab/{}/{}",
        kernel::RESERVED_PREFIX,
        city::DESKTOP_SCOPE_FILE
    ))
    .expect("the path an address spells");
    assert!(
        inside.is_reserved(),
        "it lands inside the subtree no write domain reaches"
    );
}

/// A run does not change what governs it. The rules tool answers the
/// model with a refusal rather than a rewrite, and the building it was
/// asked about stands exactly as it did.
#[test]
fn a_run_that_asks_to_rewrite_its_own_rules_is_refused_and_told_where_to_go() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let proposal = serde_json::json!({
        "op": "propose",
        "text": "# lab\n\nconfidential: false\nreview: true\n\n## Write domain\n\n- lab\n",
    });
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with("drafting the rules", "rules", "tu_1", proposal.clone()),
            completion("waiting on a person", None),
            completion_with("drafting the rules", "rules", "tu_2", proposal),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let before = city::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
    assert!(!before.review(), "the template does not ask for review");

    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "this building's work needs checking before it lands".to_owned(),
            goal: "the rules say so, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    assert!(
        !city::load(dir.path(), &Address::parse("lab").unwrap())
            .unwrap()
            .review(),
        "a building rewrote its own rules without anybody being asked"
    );

    assert!(
        worker.governance.pending.is_empty(),
        "a rule change is not a question: it is refused, and a person edits the file"
    );
    let after = city::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
    assert!(!after.review(), "a run rewrote the rules it is judged by");
    assert!(
        city::rules_path(dir.path(), &Address::parse("lab").unwrap())
            .to_string_lossy()
            .contains(".sprawling"),
        "the rules live where no write domain reaches"
    );
}

#[test]
fn a_new_building_is_visible_in_the_city_view_with_a_denominator_of_zero() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();

    let views = crate::views::Views::new(dir.path());
    let wire::Answer::City(city) = views.prepare(&wire::Query::CityView).finish() else {
        panic!("CityView answers with a city");
    };
    let lab = city
        .buildings
        .iter()
        .find(|b| b.addr.as_str() == "lab")
        .expect("a building the city made is a building the city can see");
    assert!(lab.problems.is_empty());
    let kernel::Progress::Planned(planned) = lab.progress else {
        panic!("a building with a roadmap has a denominator");
    };
    assert_eq!(
        planned.ratio(),
        (0, 0),
        "a new building owes nothing yet, and owes it out of nothing"
    );
}

/// The page's write of a building's rules lands only over the text the
/// page read, and only once the city can read what it says: a stale base
/// and a body that does not evaluate both leave the file as it was and
/// book nothing, and the write that holds both books one line.
#[test]
fn a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let lab = Address::parse("lab").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: lab.clone(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let path = city::rules_path(dir.path(), &lab);
    let read = || std::fs::read_to_string(&path).unwrap();
    let opened = read();
    let moved = format!("{opened}# a note the Mayor added\n");
    std::fs::write(&path, &moved).unwrap();
    let mut put = |base: &str, body: &str, tag: &[u8]| {
        worker
            .handle(wire::Command::PutRules(wire::RulesWrite {
                building: lab.clone(),
                base: base.to_owned(),
                body: body.to_owned(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, tag),
            }))
            .map_err(|err| *err.code())
    };
    let booked = |dir: &std::path::Path| {
        let ledger = kernel::layout::CityLayout::new(dir).ledger();
        runtime::replay::verify_ledger_dir(&ledger)
            .unwrap()
            .raw_lines()
            .iter()
            .map(|line| kernel::EventRecord::parse_line(line).unwrap())
            .filter(|record| record.kind() == kernel::EventKind::RulesChanged)
            .count()
    };
    let before = booked(dir.path());

    let stale = put(
        &opened,
        &format!("{opened}# from a page left open\n"),
        b"stale",
    );
    let unread = put(&moved, "confidential = \"perhaps\"\n", b"unread");
    assert_eq!(
        (stale, unread, read(), booked(dir.path())),
        (
            Err(AxCode::VersionConflict),
            Err(AxCode::ConfigInvalid),
            moved.clone(),
            before
        )
    );

    let kept = format!("{moved}# and one from the page\n");
    put(&moved, &kept, b"kept").unwrap();
    assert_eq!(
        (read(), booked(dir.path())),
        (kept, before.saturating_add(1))
    );
}
