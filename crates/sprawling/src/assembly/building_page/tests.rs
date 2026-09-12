// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// The rules a person may read on the building page are the rules
/// the city obeys, and the page reads them from a directory the
/// walk deliberately skips.
#[test]
fn a_building_page_still_shows_the_rules_that_govern_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
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
        .find(|doc| doc.name == city::BUILDING_FILE)
        .expect("the page lost the tab that says what this building may do");
    assert!(rules.text.contains("confidential"), "{}", rules.text);
    assert!(
        !answer.rooms.iter().any(|room| room.starts_with('.')),
        "a reserved subtree is not a room: {:?}",
        answer.rooms
    );
}

/// A plan nobody can open and a plan somebody wrote badly are two
/// different facts, and the page used to state the second one.
///
/// Reading the file as empty runs it through `check_roadmap_shape`,
/// which finds no header row and answers `no six-column table
/// found`. That sends a person to edit a table when what they have
/// to fix is a file that will not open - the same misreport already
/// removed from the dispatch path, still standing on the page.
#[test]
fn a_building_page_says_the_plan_cannot_be_read_rather_than_that_it_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
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
    init_city(dir.path()).unwrap();
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
        .handle(channels::Command::Wake {
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
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let before = city::load_config(dir.path(), &room).unwrap();
    assert!(!before.sandbox.shell, "the shell arm is off by default");

    worker
        .handle(channels::Command::ConfigureBuilding {
            addr: room.clone(),
            sandbox: Some(kernel::SandboxLimits {
                shell: true,
                fuel: 4096,
                mounts: vec![Address::parse("lab/shared").unwrap()],
                env_passthrough: Vec::new(),
            }),
            mcp: Some(vec![kernel::McpServer {
                label: kernel::ServerLabel::parse("docs").unwrap(),
                transport: kernel::McpTransport::Http {
                    url: "https://mcp.example/v1".to_owned(),
                    headers: Vec::new(),
                },
            }]),
            desktop: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"reach"),
        })
        .unwrap();

    // The ladder is the authority: what a run in the room resolves
    // to is what the building's own rung now says.
    let after = city::load_config(dir.path(), &room).unwrap();
    assert!(after.sandbox.shell);
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
/// goes (city-SPEC.md 8-26).
///
/// The bytes a person wrote are the bytes on disk: this side never
/// parses the file, because the connector that reads it at start-up
/// is the authority on its syntax and fails closed.
#[test]
fn the_desktop_allowlist_is_written_where_no_resident_reaches_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let lab = Address::parse("lab").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: lab.clone(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();

    let allowlist = "[[window]]\ntitle = \"kusanagi\"\n";
    worker
        .handle(channels::Command::ConfigureBuilding {
            addr: lab.clone(),
            sandbox: None,
            mcp: None,
            desktop: Some(allowlist.to_owned()),
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

/// A building's rules are a governance document, and asking a
/// person to type one by hand is the wrong door. An agent drafts
/// them; the person is shown the proposal and allows it; the file
/// lands in the reserved subtree that no write domain reaches.
#[test]
fn a_building_can_be_asked_to_rewrite_its_own_rules_and_the_person_decides() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
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
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let before = city::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
    assert!(!before.review(), "the template does not ask for review");

    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "this building's work needs checking before it lands".to_owned(),
            goal: "the rules say so, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    assert!(
        !city::load(dir.path(), &Address::parse("lab").unwrap())
            .unwrap()
            .review(),
        "a building rewrote its own rules without anybody being asked"
    );

    let waiting = worker
        .governance
        .pending
        .values()
        .next()
        .cloned()
        .expect("the person was never asked");
    assert_eq!(waiting.cluster_key.class, kernel::ApprovalClass::Governance);
    assert!(
        waiting.action_desc.contains("review: true"),
        "the person is shown what they are allowing: {}",
        waiting.action_desc
    );
    allow_the_one_pending_item(&mut worker);

    let after = city::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
    assert!(after.review(), "the allowed proposal never landed");
    assert!(
        city::building_path(dir.path(), &Address::parse("lab").unwrap())
            .to_string_lossy()
            .contains(".sprawling"),
        "the rules live where no write domain reaches"
    );
}
