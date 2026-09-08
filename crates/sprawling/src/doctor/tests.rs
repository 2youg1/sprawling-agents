// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::screen::{Asked, run};
use super::table::WASM_BINDGEN_VERSION;
use super::*;

/// A machine that answers from a script and installs nothing.
///
/// It is the second implementation of `Machine`, which is what makes
/// that trait a seam rather than a hypothetical one: a verdict is judged
/// here without the machine this test runs on being asked anything.
struct ScriptedMachine {
    absent: BTreeSet<&'static str>,
    asked: RefCell<Vec<String>>,
}

impl ScriptedMachine {
    fn missing(absent: &[&'static str]) -> ScriptedMachine {
        ScriptedMachine {
            absent: absent.iter().copied().collect(),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl Machine for ScriptedMachine {
    fn look(&self, requirement: &Requirement) -> Presence {
        if self.absent.contains(requirement.name) {
            Presence::Absent
        } else {
            Presence::Present("9.9.9".to_owned())
        }
    }

    fn install(&self, name: &str, _recipe: &Recipe) -> Result<(), kernel::AxError> {
        self.asked.borrow_mut().push(name.to_owned());
        Ok(())
    }
}

fn finding_for(name: &str, absent: &[&'static str]) -> Finding {
    let machine = ScriptedMachine::missing(absent);
    examine(&machine)
        .into_iter()
        .find(|finding| finding.requirement.name == name)
        .expect("the table carries this item")
}

/// Every row answers both questions a person has: how do I find out
/// whether I have it, and what would getting it cost me here.
///
/// A row with no detection method cannot be checked, and a platform
/// column with no recipe leaves a person on that platform holding an
/// item and no next move - which is what `Manual` says out loud.
#[test]
fn every_row_is_detectable_and_per_platform_installable_or_manual() {
    assert!(
        !REQUIREMENTS.is_empty(),
        "a doctor with an empty table checks nothing"
    );
    for requirement in REQUIREMENTS {
        let name = requirement.name;
        assert!(!name.is_empty(), "an item with no name cannot be reported");
        assert!(
            !requirement.enables.is_empty(),
            "{name} does not say what it enables"
        );
        match &requirement.detect {
            Detection::Program {
                program,
                version_arg,
                ..
            } => {
                assert!(!program.is_empty(), "{name} names no program");
                assert!(
                    version_arg.starts_with('-'),
                    "{name} asks its version with {version_arg}"
                );
            }
            Detection::Environment { variable } => {
                assert!(!variable.is_empty(), "{name} names no environment variable");
            }
        }
        for platform in Platform::ALL {
            let spelled = requirement.recipe.at(platform).spelled();
            assert!(
                !spelled.is_empty(),
                "{name} says nothing on {}",
                platform.as_str()
            );
        }
    }
}

/// The table is the authority on what this repository needs, so the
/// items its own documents name are in it - including the two that are
/// not programs on a path: the pinned `wasm-bindgen` version, and the
/// component the exec tool's python arm reads from an environment
/// variable.
#[test]
fn the_table_names_what_this_repository_actually_asks_for() {
    let named: BTreeSet<&str> = REQUIREMENTS.iter().map(|item| item.name).collect();
    for needed in [
        "firefox",
        "rustup",
        "just",
        "cargo-nextest",
        "wasm-bindgen",
        "bun",
        "chromedriver",
        "git",
        "python-wasi",
    ] {
        assert!(named.contains(needed), "the table does not carry {needed}");
    }
    let use_tier: Vec<&str> = REQUIREMENTS
        .iter()
        .filter(|item| item.tier == Tier::Use)
        .map(|item| item.name)
        .collect();
    assert_eq!(
        use_tier,
        vec!["firefox"],
        "the use tier is what a person needs to run a city, and nothing else"
    );
    let pinned = REQUIREMENTS
        .iter()
        .find(|item| item.name == "wasm-bindgen")
        .expect("the table carries wasm-bindgen");
    let spelled = pinned.recipe.at(Platform::Linux).spelled();
    assert!(
        spelled.contains(WASM_BINDGEN_VERSION),
        "the CLI version must equal the crate version: {spelled}"
    );
}

/// The version the workspace manifest pins. Read from the manifest at
/// compile time would be a second parser; this assertion is what keeps
/// the two equal.
#[test]
fn the_pinned_wasm_bindgen_version_is_the_one_the_workspace_carries() {
    let manifest = include_str!("../../../../Cargo.toml");
    let pinned = manifest
        .lines()
        .find(|line| line.starts_with("wasm-bindgen = "))
        .expect("the workspace pins wasm-bindgen");
    assert!(
        pinned.contains(WASM_BINDGEN_VERSION),
        "the table says {WASM_BINDGEN_VERSION}, the workspace says {pinned}"
    );
}

/// The variable this table watches is the one the exec tool reads.
///
/// `PYTHON_WASM_ENV` is private to `assembly::mcp`, so the table spells
/// the name a second time. This assertion is what keeps the second
/// spelling from drifting away from the first one.
#[test]
fn the_python_component_is_looked_for_where_the_exec_tool_reads_it() {
    let assembly = include_str!("../assembly/mcp.rs");
    let variable = REQUIREMENTS
        .iter()
        .find(|item| item.name == "python-wasi")
        .map(|item| match &item.detect {
            Detection::Environment { variable } => *variable,
            Detection::Program { program, .. } => program,
        })
        .expect("the table carries the python component");
    assert!(
        assembly.contains(&format!("PYTHON_WASM_ENV: &str = \"{variable}\"")),
        "the exec tool does not read {variable}"
    );
}

/// A tier is answered on its own, and an optional item absent never
/// stands between a person and a tier.
#[test]
fn a_verdict_counts_the_required_items_of_its_own_tier_only() {
    let all_here = examine(&ScriptedMachine::missing(&[]));
    assert_eq!(verdict(&all_here, Tier::Use), Verdict::Ready);
    assert_eq!(verdict(&all_here, Tier::Develop), Verdict::Ready);

    let no_browser = examine(&ScriptedMachine::missing(&["firefox"]));
    assert_eq!(
        verdict(&no_browser, Tier::Use),
        Verdict::Missing(vec!["firefox"])
    );
    assert_eq!(
        verdict(&no_browser, Tier::Develop),
        Verdict::Ready,
        "a browser is not what stands between a person and this code"
    );

    let no_driver = examine(&ScriptedMachine::missing(&["chromedriver"]));
    assert_eq!(
        verdict(&no_driver, Tier::Develop),
        Verdict::Ready,
        "an optional item is reported, not counted"
    );
    assert_eq!(
        verdict_line(Tier::Develop, &Verdict::Missing(vec!["just"])),
        "  not ready to develop: missing just"
    );
}

/// Three shapes of line, so absent and optional-absent are never read as
/// the same fact.
#[test]
fn one_line_per_item_says_present_absent_or_optional_absent() {
    let present = finding_for("git", &[]);
    assert!(finding_line(&present).contains("present 9.9.9"));

    let absent = finding_for("git", &["git"]);
    assert!(finding_line(&absent).trim().ends_with("absent"));

    let optional = finding_for("chromedriver", &["chromedriver"]);
    let line = finding_line(&optional);
    assert!(line.contains("optional-absent"), "{line}");
    assert!(line.contains("it enables"), "{line}");
}

/// Consent is asked item by item, and a no installs nothing.
///
/// The command is on the screen before the question, so nobody agrees to
/// a command they were not shown.
#[test]
fn nothing_is_installed_without_a_yes_to_that_one_item() {
    let Some(platform) = Platform::current() else {
        return;
    };
    let machine = ScriptedMachine::missing(&["just"]);
    let mut refused = std::io::Cursor::new(b"n\n".to_vec());
    let mut screen: Vec<u8> = Vec::new();
    let ready = run(
        &Asked { install: true },
        &machine,
        &mut refused,
        &mut screen,
    )
    .unwrap();
    assert!(!ready, "a missing required item is not ready");
    let shown = String::from_utf8(screen).unwrap();
    let command = REQUIREMENTS
        .iter()
        .find(|item| item.name == "just")
        .expect("the table carries just")
        .recipe
        .at(platform)
        .spelled();
    assert!(
        shown.contains(&command),
        "the command was not shown: {shown}"
    );
    assert!(
        machine.asked.borrow().is_empty(),
        "a no installed something"
    );

    let machine = ScriptedMachine::missing(&["just", "git"]);
    let mut agreed = std::io::Cursor::new(b"y\nn\n".to_vec());
    let mut screen: Vec<u8> = Vec::new();
    run(&Asked { install: true }, &machine, &mut agreed, &mut screen).unwrap();
    assert_eq!(
        machine.asked.borrow().as_slice(),
        ["just".to_owned()],
        "exactly the item that was answered yes is installed"
    );
}

/// Checking is what the default does, and it changes nothing.
#[test]
fn the_default_checks_and_installs_nothing() {
    let machine = ScriptedMachine::missing(&["just", "firefox"]);
    let mut nobody = std::io::Cursor::new(Vec::new());
    let mut screen: Vec<u8> = Vec::new();
    run(
        &Asked { install: false },
        &machine,
        &mut nobody,
        &mut screen,
    )
    .unwrap();
    assert!(machine.asked.borrow().is_empty());
    let shown = String::from_utf8(screen).unwrap();
    assert!(
        shown.contains("not ready to use: missing firefox"),
        "{shown}"
    );
    assert!(
        !shown.contains("[y/N]"),
        "the default asks nothing: {shown}"
    );
}

/// A program is looked for under the names this platform gives it.
///
/// On Windows the extensionless file comes last: a directory holding
/// both `bun` and `bun.cmd` holds one file this operating system can
/// start and one it cannot, and taking them in the wrong order reports
/// an installed tool as a silent one.
#[test]
fn a_program_is_looked_for_under_every_name_this_platform_gives_it() {
    let names = super::probe::names_of("bun");
    assert_eq!(names.last().map(String::as_str), Some("bun"));
    if cfg!(target_os = "windows") {
        assert_eq!(names.first().map(String::as_str), Some("bun.exe"));
        assert!(names.iter().any(|name| name == "bun.cmd"));
    } else {
        assert_eq!(names.len(), 1);
    }
}
