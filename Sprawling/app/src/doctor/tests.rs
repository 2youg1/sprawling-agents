// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod browsers;
mod city;
mod consent;
mod faults;
mod host;
mod reading;

use std::collections::BTreeSet;
use std::sync::Mutex;

use accounting::Runnable;

use super::*;
use crate::serving::standing::Standing;

pub(super) struct ScriptedMachine {
    absent: BTreeSet<&'static str>,
    asked: Mutex<Vec<String>>,
    core: Standing,
    scanning: super::scanning::Scanning,
}

impl ScriptedMachine {
    pub(super) fn missing(absent: &[&'static str]) -> ScriptedMachine {
        ScriptedMachine {
            absent: absent.iter().copied().collect(),
            asked: Mutex::new(Vec::new()),
            core: Standing::Raised,
            scanning: super::scanning::Scanning::DoesNotApply,
        }
    }

    pub(super) fn standing(self, core: Standing) -> ScriptedMachine {
        ScriptedMachine { core, ..self }
    }

    pub(super) fn scanning(self, scanning: super::scanning::Scanning) -> ScriptedMachine {
        ScriptedMachine { scanning, ..self }
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

    fn core_standing(&self) -> Result<Standing, kernel::AxError> {
        Ok(self.core.clone())
    }

    fn scanning(&self, _city: &std::path::Path) -> super::scanning::Scanning {
        self.scanning.clone()
    }
}

impl accounting::Machine for ScriptedMachine {
    fn report(&self, city: &std::path::Path) -> wire::DoctorAnswer {
        super::answer(self, city)
    }

    fn install(&self, name: &str, _runnable: &Runnable) -> Result<(), kernel::AxError> {
        self.asked.lock().unwrap().push(name.to_owned());
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
            Detection::Listed { program, args, .. } => {
                assert!(!program.is_empty(), "{name} names no program");
                assert!(!args.is_empty(), "{name} asks its program for no listing");
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
            Detection::Font { files } => {
                assert!(!files.is_empty(), "{name} names no font file");
                for file in *files {
                    assert!(!file.is_empty(), "{name} names an empty font file");
                }
            }
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

#[test]
fn the_table_names_what_this_repository_actually_asks_for() {
    let named: BTreeSet<&str> = REQUIREMENTS.iter().map(|item| item.name).collect();
    for needed in [
        "gecko",
        "chromium",
        "webkit",
        "rustup",
        "rust",
        "rustfmt",
        "clippy",
        "just",
        "cargo-nextest",
        "cargo-deny",
        "cargo-mutants",
        "cargo-fuzz",
        "kani",
        "bun",
        "chromedriver",
        "msedgedriver",
        "git",
        "host-python",
        "shell",
        "ffmpeg",
        "sarasa-gothic",
        "maple-mono-nf-cn",
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
        use_tier.contains(&"host-python") && use_tier.contains(&"shell"),
        "what a run's tools reach for is a use-tier fact: {use_tier:?}"
    );
}

#[test]
fn the_python_variable_is_spelled_once() {
    let variable = REQUIREMENTS
        .iter()
        .find(|item| item.name == "host-python")
        .and_then(|item| match &item.detect {
            Detection::Interpreter { variable, .. } => Some(variable.windows),
            _ => None,
        })
        .expect("the table carries host Python as an Interpreter");
    assert_eq!(variable, crate::doctor::table::PYTHON_VARIABLE);
    let literal = format!("\"{variable}\"");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut spelled_in = Vec::new();
    let mut pending = vec![src];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
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

    let no_coverage = examine(&ScriptedMachine::missing(&["cargo-mutants"]));
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

#[test]
fn every_item_has_the_page_clause_in_both_languages() {
    let words: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../RefRain/src/lang.json"
    )))
    .unwrap();
    let unworded: Vec<&str> = REQUIREMENTS
        .iter()
        .map(|requirement| requirement.name)
        .filter(|name| {
            let entry = &words[format!("machine_enables_{}", name.replace('-', "_"))];
            ["en", "zh"]
                .iter()
                .any(|lang| entry[lang].as_str().is_none_or(str::is_empty))
        })
        .collect();
    assert_eq!(unworded, Vec::<&str>::new());
}

#[test]
fn the_prereqs_file_is_the_develop_tier_rendered() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/doctor/table/prereqs.tsv");
    let rendered = table::prereqs();
    let written = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        written == rendered,
        "{} is not what the table renders; write this into it:\n<<<\n{rendered}>>>",
        path.display()
    );
}

#[test]
fn the_shell_every_recipe_runs_in_is_a_required_develop_row() {
    let justfile = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../justfile"),
    )
    .unwrap();
    let shell = justfile
        .lines()
        .find_map(|line| line.strip_prefix("set shell := [\""))
        .and_then(|rest| rest.split('"').next())
        .unwrap();
    let answering: Vec<&str> = REQUIREMENTS
        .iter()
        .filter(|row| row.tier == Tier::Develop && row.need == Need::Required)
        .filter(|row| match &row.detect {
            Detection::Program { program, .. } | Detection::Listed { program, .. } => {
                *program == shell
            }
            Detection::Component { .. }
            | Detection::Interpreter { .. }
            | Detection::Built { .. }
            | Detection::Family(_)
            | Detection::Font { .. } => false,
        })
        .map(|row| row.name)
        .collect();
    assert!(
        !answering.is_empty(),
        "every recipe runs in `{shell}`, and no required develop row looks for it"
    );
}
