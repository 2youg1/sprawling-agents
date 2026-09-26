// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person reads off the report: four status words, two sections,
//! the level the core's threads get, colour a terminal may refuse, and a
//! version column no vendor banner can push off the screen
//! (sprawling-SPEC.md sections 8-59 and 8-40).

use super::{ScriptedMachine, finding_for};
use crate::doctor::paint::{Counted, Ink, Part, Status, count, row, summary};
use crate::doctor::screen::{Asked, run};
use crate::doctor::{Fault, Finding, Presence, REQUIREMENTS, Tier, Version, examine};
use crate::serving::standing::{Held, Standing};

/// Four status words, one per state, so a person reads the column
/// rather than the sentence - and an item nobody is waiting for is
/// never read as an item standing in the way.
#[test]
fn one_row_per_item_carries_one_of_four_status_words() {
    let present = finding_for("git", &[]);
    assert_eq!(Status::of(&present), Status::Present);
    let line = row(&present, Ink::Plain);
    assert!(line.contains("present"), "{line}");
    assert!(line.contains("9.9.9 at /bin/git"), "{line}");

    let absent = finding_for("git", &["git"]);
    assert_eq!(Status::of(&absent), Status::Missing);
    assert!(
        row(&absent, Ink::Plain)
            .trim()
            .ends_with("not on the search path")
    );

    let optional = finding_for("ffmpeg", &["ffmpeg"]);
    assert_eq!(Status::of(&optional), Status::Optional);
    let line = row(&optional, Ink::Plain);
    assert!(line.contains("optional"), "{line}");
    assert!(line.contains("it enables"), "{line}");

    let broken = Finding {
        requirement: REQUIREMENTS
            .iter()
            .find(|item| item.name == "git")
            .expect("the table carries git"),
        presence: Presence::Broken {
            at: std::path::PathBuf::from("/bin/git"),
            fault: Fault::WillNotStart("permission denied".to_owned()),
        },
    };
    assert_eq!(Status::of(&broken), Status::Broken);
    assert!(row(&broken, Ink::Plain).contains("broken"));
}

/// Colour is an offer a terminal may refuse, and either refusal is
/// enough: the environment's and the command line's.
#[test]
fn no_color_is_honoured_from_the_environment_and_from_the_flag() {
    let present = finding_for("git", &[]);
    assert!(row(&present, Ink::Colour).contains('\u{1b}'));
    assert!(!row(&present, Ink::Plain).contains('\u{1b}'));
    assert_eq!(
        Ink::Colour.or_plain(Some(std::ffi::OsString::new())),
        Ink::Plain,
        "NO_COLOR is a request whatever it is set to, including nothing"
    );
    let words: Vec<String> = ["doctor", "--no-color"]
        .iter()
        .map(|word| (*word).to_owned())
        .collect();
    assert_eq!(crate::doctor::screen::asked(&words, None).ink, Ink::Plain);
}

/// A version column a vendor's banner cannot push off the screen.
#[test]
fn a_version_is_the_number_out_of_whatever_the_tool_printed() {
    let said = |text: &str| Version::Said(text.to_owned()).number();
    assert_eq!(
        said("ffmpeg version N-125649-g8d3942 Copyright (c) 2000-2026 the FFmpeg developers"),
        "N-125649-g8d"
    );
    assert_eq!(said("Mozilla Firefox 133.0.3"), "133.0.3");
    assert_eq!(said("git version 2.43.0"), "2.43.0");
    assert_eq!(said("cargo-nextest-cargo-nextest 0.9.78"), "0.9.78");
    assert_eq!(
        Version::Silent.number(),
        "no version",
        "a version this machine could not read is a word in the column rather than a blank"
    );
}

/// The two sections a person reads: what stands between them and a
/// running city, and what each other item would add.
#[test]
fn the_report_is_grouped_into_required_and_recommended() {
    let findings = examine(&ScriptedMachine::missing(&[]));
    let required: Vec<&str> = findings
        .iter()
        .filter(|found| Part::of(found.requirement) == Part::Required)
        .map(|found| found.requirement.name)
        .collect();
    assert_eq!(
        required,
        vec!["gecko", "webkit", "chromedriver", "msedgedriver"],
        "required is the use tier's own items; everything else is a recommendation"
    );
    assert!(
        findings
            .iter()
            .filter(|found| found.requirement.tier == Tier::Develop)
            .all(|found| Part::of(found.requirement) == Part::Recommended),
        "a person who only wants to run a city is not told they are short a fuzzer"
    );
}

/// The closing count agrees with the verdict above it: a family of
/// browsers is one thing to have, so a machine with one of them reads
/// `1 / 1` beside `ready to use` rather than `1 / 4`.
#[test]
fn a_family_of_browsers_counts_once_in_the_summary() {
    let only_zen = examine(&ScriptedMachine::missing(&[
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        count(&only_zen, Part::Required),
        Counted { here: 1, wanted: 1 }
    );
    assert!(
        summary(&only_zen, Ink::Plain)
            .iter()
            .any(|line| line.contains("next: sprawling up")),
        "a person who has what a city needs is told the command that starts one"
    );

    let no_engine = examine(&ScriptedMachine::missing(&[
        "gecko",
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        count(&no_engine, Part::Required),
        Counted { here: 0, wanted: 1 }
    );
    assert!(
        summary(&no_engine, Ink::Plain)
            .iter()
            .any(|line| line.contains("doctor --install")),
        "a person who is short of one is told the command that offers it"
    );
}

/// The heading is on the screen before the first item is asked about, so
/// a person waiting on a slow tool is never looking at an empty terminal.
#[test]
fn the_heading_is_written_before_any_item_is_asked() {
    use crate::doctor::{Absence, Machine, Requirement};
    use accounting::Runnable;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Watched {
        written: Arc<AtomicBool>,
        asked_in_silence: AtomicBool,
    }
    impl Machine for Watched {
        fn look(&self, _requirement: &Requirement) -> Presence {
            if !self.written.load(Ordering::SeqCst) {
                self.asked_in_silence.store(true, Ordering::SeqCst);
            }
            Presence::Absent(Absence::NotOnSearchPath)
        }
        fn core_standing(&self) -> Result<crate::serving::standing::Standing, kernel::AxError> {
            Ok(crate::serving::standing::Standing::Raised)
        }
    }
    impl accounting::Machine for Watched {
        fn report(&self) -> channels::DoctorAnswer {
            crate::doctor::answer(self)
        }
        fn install(&self, _name: &str, _runnable: &Runnable) -> Result<(), kernel::AxError> {
            Ok(())
        }
    }
    struct Screen(Arc<AtomicBool>);
    impl std::io::Write for Screen {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.store(true, Ordering::SeqCst);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let written = Arc::new(AtomicBool::new(false));
    let machine = Watched {
        written: Arc::clone(&written),
        asked_in_silence: AtomicBool::new(false),
    };
    let asked = crate::doctor::screen::asked(&[], None);
    crate::doctor::screen::run(
        &asked,
        &machine,
        &mut std::io::empty(),
        &mut Screen(written),
    )
    .unwrap();
    assert!(
        !machine.asked_in_silence.load(Ordering::SeqCst),
        "an item was asked about while the screen was still empty"
    );
}

/// The first row is on the screen once its own probe answers, while the
/// other probes are still out: every other probe here waits for that row
/// to appear, and gives up after a second when it never does.
#[test]
fn the_first_row_is_written_while_the_other_items_are_still_asked() {
    use crate::doctor::{Absence, Machine, Requirement};
    use accounting::Runnable;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    struct Waiting {
        first: &'static str,
        screen: Arc<Mutex<String>>,
        waited_in_vain: AtomicBool,
    }
    impl Waiting {
        fn first_row_shown(&self) -> bool {
            self.screen
                .lock()
                .unwrap()
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{} ", self.first)))
        }
    }
    impl Machine for Waiting {
        #[allow(
            clippy::disallowed_methods,
            clippy::arithmetic_side_effects,
            reason = "test code: how long a probe waits for the first row is read off the wall clock"
        )]
        fn look(&self, requirement: &Requirement) -> Presence {
            if requirement.name != self.first {
                let giving_up = Instant::now() + Duration::from_secs(1);
                while !self.first_row_shown() {
                    if Instant::now() > giving_up {
                        self.waited_in_vain.store(true, Ordering::SeqCst);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            Presence::Absent(Absence::NotOnSearchPath)
        }
        fn core_standing(&self) -> Result<Standing, kernel::AxError> {
            Ok(Standing::Raised)
        }
    }
    impl accounting::Machine for Waiting {
        fn report(&self) -> channels::DoctorAnswer {
            crate::doctor::answer(self)
        }
        fn install(&self, _name: &str, _runnable: &Runnable) -> Result<(), kernel::AxError> {
            Ok(())
        }
    }
    struct Screen(Arc<Mutex<String>>);
    impl std::io::Write for Screen {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(bytes));
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let first = REQUIREMENTS
        .iter()
        .find(|requirement| Part::of(requirement) == Part::Required)
        .unwrap()
        .name;
    let screen = Arc::new(Mutex::new(String::new()));
    let machine = Waiting {
        first,
        screen: Arc::clone(&screen),
        waited_in_vain: AtomicBool::new(false),
    };
    run(
        &crate::doctor::screen::asked(&[], None),
        &machine,
        &mut std::io::empty(),
        &mut Screen(screen),
    )
    .unwrap();
    assert!(
        !machine.waited_in_vain.load(Ordering::SeqCst),
        "the row for {first} waited until every item had answered"
    );
}

/// A Unix machine without `CAP_SYS_NICE` refuses the raise; the doctor
/// says the core stands at normal and why, rather than nothing.
#[test]
fn the_doctor_says_where_the_platform_lets_the_core_stand() {
    let machine = ScriptedMachine::missing(&[]).standing(Standing::Normal(Held::Refused(
        "Operation not permitted".to_owned(),
    )));
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
    let shown = String::from_utf8(screen).unwrap();
    assert!(
        shown.contains(
            "  priority - where the core's threads stand\n\n    core threads    normal, the platform refused: Operation not permitted\n\n"
        ),
        "{shown}"
    );
}
