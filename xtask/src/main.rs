// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "A gate reads somebody else's vocabulary: `syn`'s syntax               tree and `serde_json`'s value are `#[non_exhaustive]`               upstream, so a wildcard over them is the correct arm               rather than an erased decision, and no arm this package               writes can be made exhaustive by changing this tree. The               other two relax at one remove, for reports written to a               stream nobody reads back. This is the only exception in the               tree, and `expect` makes it self-cleaning: the day no               site needs it, the build says so. It does not extend to a               gate matching on a kernel enum - that arm belongs in the               crate that owns the enum."
)]

//! Gate runner. One gate per module; `gates` runs them all in order.
//! Exit codes: 0 clean, 1 violations found, 2 usage or gate-internal failure.
//! A broken gate must fail loudly (code 2): silent passes are the worst
//! failure mode a gate can have (xtask-SPEC.md section 12).

mod apisync;
mod architecture;
mod artifact;
mod badge;
mod boundary;
mod budget;
mod bundle;
mod channel;
mod color;
mod depmap;
mod docnum;
mod gates;
mod guard;
mod header;
mod length;
mod lexicon;
mod mem;
mod modmap;
mod npm;
mod package;
mod platform;
mod proof;
mod release;
mod render;
mod report;
mod repro;
mod root;
mod sbom;
mod secret;
mod spec;
mod specalign;
// The session-slice path gate: only its writer names the path
// (memory-SPEC 8-24). Declared here because a module lives where the
// crate root says it does.
// The grid instrument (xtask-SPEC.md section 8-26). It is declared here
// because a module lives where the crate root says it does; it is not a
// subcommand, and `cargo xtask render --survey` is how a person reaches
// it, for the reason section 8-26 states.
mod vocabulary;
mod walk;
mod wire_ts;
mod wiring;
mod wording;

use std::path::Path;
use std::process::ExitCode;

use report::XtaskError;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let judged = std::env::current_dir()
        .map_err(|source| XtaskError::Io {
            path: ".".to_owned(),
            source,
        })
        .and_then(|cwd| root::judged(Path::new(env!("CARGO_MANIFEST_DIR")), &cwd));
    let root = match judged {
        Ok(root) => root,
        Err(err) => return report::internal_failure(&err),
    };
    match args.first().map(String::as_str) {
        // The roster documents quote, printed from the same array the
        // usage text renders from: a gate table in a document is checked
        // against this output rather than maintained beside it.
        Some("gates") if args.iter().any(|a| a == "--list") => {
            for gate in &gates::GATES {
                println!("{}", gate.name);
            }
            ExitCode::SUCCESS
        }
        Some("gates") => gates::run(&root, &gate_names(&args)),
        Some("color") => report::finish("color", color::check(&root)),
        Some("render") => report::finish("render", render::check(&root)),
        Some("budget") => match budget::report(&root) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("badge") if args.iter().any(|a| a == "--write") => match badge::write(&root) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("badge") => report::finish("badge", badge::check(&root)),
        Some("mem") => match mem::run(&root, args.get(1..).unwrap_or(&[])) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("package") => match package::run(
            &root,
            &package::ReleaseTarget::from_arg(value_arg(&args, "--target").as_deref()),
        ) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        // The npm channel is assembled out of a tag's own archives, so
        // it takes the tag and where they were downloaded to rather
        // than discovering either.
        Some("channel") => match channel::run(
            &root,
            &value_arg(&args, "--tag").unwrap_or_default(),
            Path::new(&value_arg(&args, "--assets").unwrap_or_default()),
            Path::new(&value_arg(&args, "--out").unwrap_or_default()),
        ) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("sbom") => match sbom::run(&root) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("repro") => match repro::run(&root, args.iter().any(|a| a == "--full")) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        // `--list` is the roster CI consumes; bare `proof` proves it.
        // Both read the same `#[kani::proof]` attributes, so the list a
        // person reads and the set a machine proves cannot disagree.
        Some("proof") if args.iter().any(|a| a == "--list") => match proof::list(&root) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("proof") => match proof::run(&root) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        // `--write` is the recovery the gate itself names, so it is the
        // same command with one flag rather than a second spelling.
        Some("docnum") if args.iter().any(|a| a == "--write") => match docnum::write(&root) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("docnum") => report::finish("docnum", docnum::check(&root)),
        Some("secret") => report::finish("secret", secret::check(&root)),
        Some("specalign") => report::finish("specalign", specalign::check(&root)),
        Some("wiring") => report::finish("wiring", wiring::check(&root)),
        Some("wire-ts") if args.iter().any(|a| a == "--write") => match wire_ts::write(&root) {
            Ok(message) => {
                print!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some("wire-ts") => report::finish("wire-ts", wire_ts::check(&root)),
        Some("wording") => report::finish("wording", wording::check(&root)),
        Some("apisync") if args.iter().any(|a| a == "--write") => match apisync::write(&root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => report::internal_failure(&err),
        },
        Some("apisync") => report::finish("apisync", apisync::check(&root)),
        Some("header") => report::finish("header", header::check(&root)),
        Some("lexicon") => report::finish("lexicon", lexicon::check(&root)),
        Some("length") => report::finish("length", length::check(&root)),
        Some("boundary") => report::finish("boundary", boundary::check(&root)),
        Some("artifact") => report::finish("artifact", artifact::check(&root)),
        Some("modmap") => report::finish("modmap", modmap::check(&root)),
        Some("npm") => report::finish("npm", npm::check(&root)),
        Some("depmap") => report::finish("depmap", depmap::check(&root)),
        Some("guard") => report::finish("guard", guard::check(&root)),
        Some("release") => report::finish("release", release::check(&root)),
        Some("spec") => match spec::run(&root, args.get(1).map(String::as_str)) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(err) => report::internal_failure(&err),
        },
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            usage();
            ExitCode::from(2)
        }
        None => {
            usage();
            ExitCode::from(2)
        }
    }
}

/// The value of one named flag anywhere after the subcommand: `--target`
/// for `package`, `--tag`, `--assets` and `--out` for `release`. One
/// reader, so two flags cannot end up with two spellings of what "the
/// value after it" means.
fn value_arg(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == flag {
            return it.next().cloned();
        }
    }
    None
}

/// The gate names typed after `gates`: every argument that is neither a
/// flag nor the value `--range` takes. No gate reads history, so a
/// `--range` a CI job still passes is skipped rather than read as a gate
/// name.
fn gate_names(args: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        if arg == "--range" {
            it.next();
        } else if !arg.starts_with("--") {
            names.push(arg.clone());
        }
    }
    names
}

/// One command this tool answers beyond running a gate: what a person
/// types after `cargo xtask`, and what they get for it.
struct Tool {
    /// The subcommand and the arguments it takes, as they are typed.
    call: &'static str,
    /// What it produces, in one clause.
    gives: &'static str,
}

/// Everything `cargo xtask` answers that is not a gate, plus the flags
/// that change what a gate does. The dispatcher above and this array are
/// read together, so a command that grows a flag is printed with it.
const TOOLS: [Tool; 12] = [
    Tool {
        call: "gates [<gate>...]",
        gives: "every gate, or only the named ones",
    },
    Tool {
        call: "gates --list",
        gives: "the gate roster, one name per line",
    },
    Tool {
        call: "<gate>",
        gives: "one gate on its own",
    },
    Tool {
        call: "<gate> --write",
        gives: "the recovery that gate names, applied: apisync, docnum, wire-ts, badge",
    },
    Tool {
        call: "proof --list",
        gives: "the kani harness roster, read from the `#[kani::proof]` attributes",
    },
    Tool {
        call: "spec <crate>",
        gives: "a SPEC skeleton for that crate",
    },
    Tool {
        call: "mem [<pid> | --city <dir>]",
        gives: "private, peak private and working set of that pid, or of a city served idle (a fresh empty one by default)",
    },
    Tool {
        call: "sbom",
        gives: "the CycloneDX bill of materials",
    },
    Tool {
        call: "package [--target <triple>]",
        gives: "the release archive for this machine or for that triple",
    },
    Tool {
        call: "repro [--full]",
        gives: "two builds of one tree compared byte for byte",
    },
    Tool {
        call: "channel --tag <tag> --assets <dir> --out <dir>",
        gives: "the npm channel, assembled from that tag's archives",
    },
    Tool {
        call: "budget",
        gives: "every budget, what it costs today, and what is gated",
    },
];

fn usage() {
    eprintln!("usage: cargo xtask <subcommand> [flags]");
    eprintln!();
    eprintln!("gates: {}", gates::GATES.map(|gate| gate.name).join(" "));
    eprintln!();
    for tool in &TOOLS {
        eprintln!("  cargo xtask {:<52} {}", tool.call, tool.gives);
    }
}
