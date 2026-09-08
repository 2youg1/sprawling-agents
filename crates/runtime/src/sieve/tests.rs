// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One golden per built-in filter, the two properties every input is
//! held to, and the account each stage leaves behind.

use kernel::ExecArm;
use memory::Cas;
use proptest::prelude::*;

use crate::offload::OffloadSite;
use crate::sieve::{
    CommandKey, FilterTable, PassReason, SieveHistory, SieveInput, Sieved, Stage, StageOutcome,
    sieve,
};

struct World {
    _dir: tempfile::TempDir,
    cas: Cas,
    env: std::path::PathBuf,
    history: SieveHistory,
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let cas = Cas::open(&dir.path().join("cas")).unwrap();
    let env = dir.path().join("env");
    std::fs::create_dir_all(&env).unwrap();
    World {
        _dir: dir,
        cas,
        env,
        history: SieveHistory::default(),
    }
}

fn run(world: &mut World, key: &CommandKey, code: Option<i64>, text: &str) -> Sieved {
    let mut tee = OffloadSite {
        cas: &mut world.cas,
        environment: &world.env,
    };
    let input = SieveInput {
        key,
        exit_code: code,
        text,
    };
    sieve(input, &FilterTable::builtin(), &mut tee, &mut world.history).unwrap()
}

fn cut(sieved: Sieved) -> crate::sieve::SieveRecord {
    match sieved {
        Sieved::Cut(record) => record,
        Sieved::Passed { reason, .. } => panic!("expected a cut, got a pass: {reason:?}"),
    }
}

/// The rest path differs per machine; the golden holds everything else.
fn stable(text: &str) -> String {
    text.lines()
        .map(|line| match line.find(", rest at ") {
            Some(at) => format!("{}, rest at <rest>]", line.get(..at).unwrap_or("")),
            None => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cargo_key() -> CommandKey {
    CommandKey::of(&ExecArm::Program {
        path: "/usr/bin/cargo".to_owned(),
        args: vec!["build".to_owned(), "--release".to_owned()],
    })
}

fn cargo_build_output() -> String {
    let mut out = String::new();
    for n in 0..120 {
        out.push_str(&format!(
            "\x1b[1m\x1b[32m   Compiling\x1b[0m dep{n} v0.{n}.1\n"
        ));
    }
    for n in 0..6 {
        out.push_str(&format!(
            "warning: unused variable: `x{n}`\n --> src/lib{n}.rs:{}:9\n  |\n{} |     let x{n} = 1;\n  |         ^^ help: if this is intentional, prefix it with an underscore\n  |\n",
            n + 10,
            n + 10
        ));
    }
    out.push_str("error[E0308]: mismatched types\n --> src/main.rs:12:5\n  |\n12 |     1\n  |     ^ expected `&str`, found integer\n\n");
    out.push_str("error: could not compile `app` (bin \"app\") due to 1 previous error; 6 warnings emitted\n");
    out.push_str("see https://doc.rust-lang.org/error_codes/E0308.html\n");
    out.push_str("\n\n\n\n");
    for n in 0..40 {
        out.push_str(&format!("    Finished step {n} in 0.{n}s\n"));
    }
    out
}

#[test]
fn cargo_build_is_reduced_to_its_diagnostics() {
    let mut world = world();
    let record = cut(run(
        &mut world,
        &cargo_key(),
        Some(101),
        &cargo_build_output(),
    ));
    assert_eq!(record.filter, "cargo");
    assert!(record.text.contains("error[E0308]"));
    assert!(record.text.contains("src/main.rs:12:5"));
    assert!(!record.text.contains("Compiling"), "{}", record.text);
    assert!(!record.text.contains("\x1b["));
    assert!(
        record
            .text
            .contains("https://doc.rust-lang.org/error_codes/E0308.html")
    );
    insta::assert_snapshot!(stable(&record.text));
}

fn git_key() -> CommandKey {
    CommandKey::of(&ExecArm::Shell {
        text: "git push origin main".to_owned(),
    })
}

fn git_push_output() -> String {
    let mut out = String::new();
    for n in 0..200 {
        out.push_str(&format!("remote: Compressing objects: {n}% ({n}/200)\r"));
    }
    out.push('\n');
    for n in 0..200 {
        out.push_str(&format!(
            "Writing objects: {n}% ({n}/200), 1.{n} MiB | 4.00 MiB/s\n"
        ));
    }
    out.push_str("remote: error: GH006: Protected branch update failed for refs/heads/main.\n");
    out.push_str("remote: error: Required status check \"ci\" is expected.\n");
    out.push_str("To github.com:2youg1/sprawling.git\n");
    out.push_str(" ! [remote rejected] main -> main (protected branch hook declined)\n");
    out.push_str("error: failed to push some refs to 'github.com:2youg1/sprawling.git'\n");
    out
}

#[test]
fn git_push_keeps_the_rejection_and_drops_the_progress() {
    let mut world = world();
    let record = cut(run(&mut world, &git_key(), Some(1), &git_push_output()));
    assert_eq!(record.filter, "git");
    assert!(record.text.contains("[remote rejected]"));
    assert!(record.text.contains("GH006"));
    assert!(
        !record.text.contains("Compressing objects"),
        "{}",
        record.text
    );
    insta::assert_snapshot!(stable(&record.text));
}

fn listing_key() -> CommandKey {
    CommandKey::of(&ExecArm::Program {
        path: "ls".to_owned(),
        args: vec!["-R".to_owned()],
    })
}

fn listing_output() -> String {
    let mut out = String::new();
    for n in 0..400 {
        out.push_str(&format!(
            "-rw-r--r-- 1 user user {} Jan 1 00:00 file_{n:03}.txt\n",
            n * 17
        ));
    }
    out.push_str("total 400\n");
    out
}

#[test]
fn a_command_nobody_wrote_a_filter_for_takes_the_generic_path() {
    let mut world = world();
    let record = cut(run(&mut world, &listing_key(), Some(0), &listing_output()));
    assert_eq!(record.filter, "generic");
    assert!(record.text.contains("total 400"));
    assert!(record.text.contains("similar"), "{}", record.text);
    insta::assert_snapshot!(stable(&record.text));
}

#[test]
fn below_the_floor_nothing_is_touched_and_nothing_is_stored() {
    let mut world = world();
    let small = "   Compiling a v0.1.0\n".repeat(20);
    match run(&mut world, &cargo_key(), Some(0), &small) {
        Sieved::Passed { text, reason } => {
            assert_eq!(text, small);
            assert_eq!(reason, PassReason::BelowFloor);
        }
        Sieved::Cut(record) => panic!("cut below the floor: {}", record.text),
    }
    assert!(std::fs::read_dir(&world.env).unwrap().next().is_none());
}

#[test]
fn the_second_identical_run_shows_only_what_changed() {
    let mut world = world();
    let first = cut(run(&mut world, &listing_key(), Some(0), &listing_output()));
    let mut second_text = listing_output();
    second_text.push_str("-rw-r--r-- 1 user user 9 Jan 1 00:00 brand_new.md\n");
    let second = cut(run(&mut world, &listing_key(), Some(0), &second_text));
    assert!(second.text.contains("brand_new.md"), "{}", second.text);
    assert!(second.text.contains("[unchanged:"), "{}", second.text);
    assert!(first.text.contains("file_000"), "{}", first.text);
    assert!(!second.text.contains("file_000"), "{}", second.text);
    let diffed = second
        .stages
        .iter()
        .find(|report| report.stage == Stage::DiffPrevious)
        .unwrap();
    assert!(matches!(diffed.outcome, StageOutcome::Applied { .. }));
    let first_diff = first
        .stages
        .iter()
        .find(|report| report.stage == Stage::DiffPrevious)
        .unwrap();
    assert!(matches!(
        first_diff.outcome,
        StageOutcome::Unavailable { .. }
    ));
}

#[test]
fn every_filtered_result_says_where_the_original_is() {
    let mut world = world();
    let record = cut(run(
        &mut world,
        &cargo_key(),
        Some(101),
        &cargo_build_output(),
    ));
    assert!(record.rest_path.exists());
    assert_eq!(
        std::fs::read_to_string(&record.rest_path).unwrap(),
        cargo_build_output()
    );
    assert!(record.text.ends_with(']'));
    assert!(record.text.contains("[sieve: "));
    assert!(record.text.contains("filter=cargo"));
    let payload = serde_json::to_value(record.payload().unwrap()).unwrap();
    assert_eq!(payload["original"], record.original.to_string());
    assert_eq!(payload["substitute_len"], record.bytes_out);
    assert!(payload["stages"].as_array().unwrap().len() == 7);
}

#[test]
fn a_clean_build_says_so_rather_than_saying_nothing() {
    let mut world = world();
    let noise = "   Compiling dep v0.1.0\n".repeat(120);
    let record = cut(run(&mut world, &cargo_key(), Some(0), &noise));
    assert!(
        record.text.starts_with("cargo build: clean, exit 0"),
        "{}",
        record.text
    );
}

#[test]
fn a_building_table_replaces_the_city_table_whole() {
    let city = "[[filter]]\nid = \"a\"\ncommand = \"a\"\n[[filter]]\nid = \"b\"\ncommand = \"b\"\n";
    let building = "[[filter]]\nid = \"c\"\ncommand = \"c\"\n";
    let table = FilterTable::resolve(Some(city), Some(building)).unwrap();
    let key = CommandKey::of(&ExecArm::Program {
        path: "a".to_owned(),
        args: Vec::new(),
    });
    assert_eq!(
        table.lookup(&key).id,
        "generic",
        "a lower layer overrides whole"
    );
    assert!(FilterTable::parse("[[filter]]\nid = 1\n").is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn the_output_is_never_larger_than_the_input(
        lines in prop::collection::vec("[ -~\u{1b}字]{0,120}", 0..400),
        code in prop::option::of(-1i64..200),
        which in 0usize..3,
    ) {
        let text = lines.join("\n");
        let key = match which {
            0 => cargo_key(),
            1 => git_key(),
            _ => listing_key(),
        };
        let mut world = world();
        let out = match run(&mut world, &key, code, &text) {
            Sieved::Passed { text, .. } => text,
            Sieved::Cut(record) => {
                prop_assert!(record.text.len() <= text.len());
                prop_assert_eq!(record.bytes_in, text.len() as u64);
                record.text
            }
        };
        prop_assert!(out.len() <= text.len(), "{} > {}", out.len(), text.len());
        prop_assert!(text.is_empty() || !out.is_empty(), "invariant 2");
        prop_assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    }
}
