// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod browsers;
mod city;
mod faults;
mod reading;

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::paint::Ink;
use super::screen::{Asked, run};
use super::*;

/// A machine that answers from a script and installs nothing.
///
/// It is the second implementation of `Machine`, which is what makes
/// that trait a seam rather than a hypothetical one: a verdict is judged
/// here without the machine this test runs on being asked anything.
pub(super) struct ScriptedMachine {
    absent: BTreeSet<&'static str>,
    asked: RefCell<Vec<String>>,
}

impl ScriptedMachine {
    pub(super) fn missing(absent: &[&'static str]) -> ScriptedMachine {
        ScriptedMachine {
            absent: absent.iter().copied().collect(),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl Machine for ScriptedMachine {
    fn look(&self, requirement: &Requirement) -> Presence {
        if self.absent.contains(requirement.name) {
            Presence::Absent(Absence::NotOnSearchPath)
        } else {
            Presence::Present {
                at: std::path::PathBuf::from(format!("/bin/{}", requirement.name)),
                version: Version::Said("9.9.9".to_owned()),
            }
        }
    }

    fn install(&self, name: &str, _recipe: &Recipe) -> Result<(), kernel::AxError> {
        self.asked.borrow_mut().push(name.to_owned());
        Ok(())
    }
}

pub(super) fn finding_for(name: &str, absent: &[&'static str]) -> Finding {
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
            Detection::Component { variable, file } => {
                assert!(!variable.is_empty(), "{name} names no environment variable");
                assert!(!file.is_empty(), "{name} names no component file");
            }
            Detection::Interpreter { variable, fallback } => {
                for platform in Platform::ALL {
                    assert!(
                        !variable.at(platform).is_empty(),
                        "{name} names no variable"
                    );
                    assert!(
                        !fallback.at(platform).is_empty(),
                        "{name} names no fallback"
                    );
                }
            }
            Detection::Built { .. } => {}
            Detection::Family(family) => {
                assert!(
                    !family.members().is_empty(),
                    "{name} is a family with no members"
                );
                for member in family.members() {
                    assert!(
                        member.homepage.starts_with("https://"),
                        "{} names no site",
                        member.name
                    );
                }
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
/// items its own documents name are in it - including the one that is
/// not a program on a path: the component the exec tool's python arm
/// reads from an environment variable.
#[test]
fn the_table_names_what_this_repository_actually_asks_for() {
    let named: BTreeSet<&str> = REQUIREMENTS.iter().map(|item| item.name).collect();
    for needed in [
        "gecko",
        "chromium",
        "webkit",
        "rustup",
        "rustfmt",
        "clippy",
        "just",
        "cargo-nextest",
        "cargo-deny",
        "cargo-audit",
        "cargo-mutants",
        "cargo-fuzz",
        "cargo-llvm-cov",
        "kani",
        "bun",
        "chromedriver",
        "msedgedriver",
        "git",
        "python-wasi",
        "sandbox-engine",
        "shell",
        "sprawling-desktop",
        "ffmpeg",
    ] {
        assert!(named.contains(needed), "the table does not carry {needed}");
    }
    let use_tier: Vec<&str> = REQUIREMENTS
        .iter()
        .filter(|item| item.tier == Tier::Use)
        .map(|item| item.name)
        .collect();
    let use_required: Vec<&str> = REQUIREMENTS
        .iter()
        .filter(|item| {
            item.tier == Tier::Use && matches!(item.need, Need::Required | Need::OneOf(_))
        })
        .map(|item| item.name)
        .collect();
    assert_eq!(
        use_required,
        vec!["gecko", "webkit", "chromedriver", "msedgedriver"],
        "a browser engine is what the use tier asks for, by family rather than by brand"
    );
    assert!(
        use_tier.contains(&"python-wasi") && use_tier.contains(&"shell"),
        "what a run's tools reach for is a use-tier fact: {use_tier:?}"
    );
}

/// The variable the python component is looked for under is spelled
/// once in this crate, in the doctor's table, and the exec tool asks
/// the doctor rather than reading it a second time.
///
/// Wave A left two spellings and a test pinning them equal; two pinned
/// spellings are still two authorities. Every source file under this
/// crate is read here, so a third spelling anywhere turns this red.
#[test]
fn the_python_variable_is_spelled_once() {
    let variable = REQUIREMENTS
        .iter()
        .find(|item| item.name == "python-wasi")
        .and_then(|item| match &item.detect {
            Detection::Component { variable, .. } => Some(*variable),
            _ => None,
        })
        .expect("the table carries the python component as a Component");
    let literal = format!("\"{variable}\"");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut spelled_in = Vec::new();
    let mut pending = vec![src];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && std::fs::read_to_string(&path).unwrap().contains(&literal)
            {
                spelled_in.push(path);
            }
        }
    }
    assert_eq!(
        spelled_in.len(),
        1,
        "{variable} is spelled in {spelled_in:?}; the table is its one home"
    );
}

/// A tier is answered on its own, and an optional item absent never
/// stands between a person and a tier.
#[test]
fn a_verdict_counts_the_required_items_of_its_own_tier_only() {
    let all_here = examine(&ScriptedMachine::missing(&[]));
    assert_eq!(verdict(&all_here, Tier::Use), Verdict::Ready);
    assert_eq!(verdict(&all_here, Tier::Develop), Verdict::Ready);

    let no_engine = examine(&ScriptedMachine::missing(&[
        "gecko",
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        verdict(&no_engine, Tier::Use),
        Verdict::Missing(vec!["a browser engine"]),
        "four ways in that are all closed is one thing missing, not four"
    );
    assert_eq!(
        verdict(&no_engine, Tier::Develop),
        Verdict::Ready,
        "a browser is not what stands between a person and this code"
    );

    let only_zen = examine(&ScriptedMachine::missing(&[
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        verdict(&only_zen, Tier::Use),
        Verdict::Ready,
        "one Gecko browser is a browser engine; no brand is asked for by name"
    );

    let only_edge = examine(&ScriptedMachine::missing(&[
        "gecko",
        "webkit",
        "chromedriver",
    ]));
    assert_eq!(
        verdict(&only_edge, Tier::Use),
        Verdict::Ready,
        "Edge with its own driver is a way in, and Windows ships both"
    );

    let no_coverage = examine(&ScriptedMachine::missing(&["cargo-llvm-cov"]));
    assert_eq!(
        verdict(&no_coverage, Tier::Develop),
        Verdict::Ready,
        "an optional item is reported, not counted"
    );
    assert_eq!(
        verdict_line(Tier::Develop, &Verdict::Missing(vec!["just"])),
        "  not ready to develop: missing just"
    );
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
        &Asked {
            install: true,
            city: None,
            explain: None,
            ink: Ink::Plain,
        },
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
    run(
        &Asked {
            install: true,
            city: None,
            explain: None,
            ink: Ink::Plain,
        },
        &machine,
        &mut agreed,
        &mut screen,
    )
    .unwrap();
    assert_eq!(
        machine.asked.borrow().as_slice(),
        ["just".to_owned()],
        "exactly the item that was answered yes is installed"
    );
}

/// Checking is what the default does, and it changes nothing.
#[test]
fn the_default_checks_and_installs_nothing() {
    let machine =
        ScriptedMachine::missing(&["just", "gecko", "webkit", "chromedriver", "msedgedriver"]);
    let mut nobody = std::io::Cursor::new(Vec::new());
    let mut screen: Vec<u8> = Vec::new();
    run(
        &Asked {
            install: false,
            city: None,
            explain: None,
            ink: Ink::Plain,
        },
        &machine,
        &mut nobody,
        &mut screen,
    )
    .unwrap();
    assert!(machine.asked.borrow().is_empty());
    let shown = String::from_utf8(screen).unwrap();
    assert!(
        shown.contains("not ready to use: missing a browser engine"),
        "{shown}"
    );
    assert!(
        !shown.contains("[y/N]"),
        "the default asks nothing: {shown}"
    );
}
