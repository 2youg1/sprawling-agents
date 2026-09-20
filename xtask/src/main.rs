// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    clippy::fn_params_excessive_bools,
    reason = "A gate reads somebody else's vocabulary: `syn`'s syntax               tree and `serde_json`'s value are `#[non_exhaustive]`               upstream, so a wildcard over them is the correct arm               rather than an erased decision, and no arm this package               writes can be made exhaustive by changing this tree. The               other three relax at one remove, for reports written to a               stream nobody reads back and for two independent switches               of one command line. This is the only exception in the               tree, and `expect` makes it self-cleaning: the day no               site needs it, the build says so. It does not extend to a               gate matching on a kernel enum - that arm belongs in the               crate that owns the enum."
)]

//! Gate runner. One gate per module; `gates` runs them all in order.
//! Exit codes: 0 clean, 1 violations found, 2 usage or gate-internal failure.
//! A broken gate must fail loudly (code 2): silent passes are the worst
//! failure mode a gate can have (xtask-SPEC.md section 12).

mod apisync;
mod artifact;
mod badge;
mod boundary;
mod budget;
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
mod proof;
mod release;
mod render;
mod report;
mod repro;
mod sbom;
mod secret;
mod spec;
mod specalign;
mod vocabulary;
mod walk;
mod wire_ts;
mod wiring;
mod wording;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use report::XtaskError;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = match repo_root() {
        Ok(root) => root,
        Err(err) => return report::internal_failure(&err),
    };
    let range = value_arg(&args, "--range");
    match args.first().map(String::as_str) {
        // The roster documents quote, printed from the same array the
        // usage text renders from: a gate table in a document is checked
        // against this output rather than maintained beside it.
        Some("gates") if args.iter().any(|a| a == "--list") => {
            for gate in GATES {
                println!("{gate}");
            }
            ExitCode::SUCCESS
        }
        Some("gates") => gates::run(&root, range.as_deref()),
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
        Some("mem") => match mem::run(&root, args.get(1).map(String::as_str)) {
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
        Some("apisync") => report::finish("apisync", apisync::check(&root, range.as_deref())),
        Some("header") => report::finish("header", header::check(&root)),
        Some("lexicon") => report::finish("lexicon", lexicon::check(&root)),
        Some("length") => report::finish("length", length::check(&root)),
        Some("boundary") => report::finish("boundary", boundary::check(&root)),
        Some("artifact") => report::finish("artifact", artifact::check(&root)),
        Some("modmap") => report::finish("modmap", modmap::check(&root)),
        Some("npm") => report::finish("npm", npm::check(&root)),
        Some("depmap") => report::finish("depmap", depmap::check(&root)),
        Some("guard") => report::finish("guard", guard::check(&root, range.as_deref())),
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

/// The repo root is the parent of the xtask manifest directory.
fn repo_root() -> Result<PathBuf, XtaskError> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.parent() {
        Some(parent) => Ok(parent.to_path_buf()),
        None => Err(XtaskError::Doc {
            file: "CARGO_MANIFEST_DIR".to_owned(),
            msg: "xtask manifest directory has no parent".to_owned(),
        }),
    }
}

/// The value of one named flag anywhere after the subcommand: `--range`
/// for `guard`, `--target` for `package`. One reader, so two flags cannot
/// end up with two spellings of what "the value after it" means.
fn value_arg(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == flag {
            return it.next().cloned();
        }
    }
    None
}

/// Every gate, as the subcommand that runs it alone, in the order
/// `gates` runs them. Typed by [`gates::COUNT`], so a gate added to the
/// gate table stops this file compiling until usage names it — which is
/// how a runnable gate missing from usage is caught before a reader is.
const GATES: [&str; gates::COUNT] = [
    "header",
    "lexicon",
    "modmap",
    "length",
    "boundary",
    "artifact",
    "depmap",
    "npm",
    "secret",
    "color",
    "wording",
    "render",
    "wiring",
    "wire-ts",
    "docnum",
    "proof",
    "budget",
    "specalign",
    "apisync",
    "release",
    "guard",
];

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
        call: "gates [--range a..b]",
        gives: "every gate in order; --range bounds the commits `apisync` and `guard` judge",
    },
    Tool {
        call: "gates --list",
        gives: "the gate roster, one name per line",
    },
    Tool {
        call: "<gate> [--range a..b]",
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
        call: "mem [pid]",
        gives: "resident memory of this process or of that pid",
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
    eprintln!("gates: {}", GATES.join(" "));
    eprintln!();
    for tool in &TOOLS {
        eprintln!("  cargo xtask {:<52} {}", tool.call, tool.gives);
    }
}
