// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The subcommands that move a city's data rather than stand a city up:
//! `enrol`, `install`, `export`, `restore`, `fork`, `adopt`,
//! `replay` and `status`.
//!
//! Each function here owns only the part a person sees — the usage
//! line, the sentence printed about what happened, and the exit code —
//! while the work itself stays in `storage`, `assembly`, `install` and
//! `wire_client`. That is why every one of them takes the arguments
//! `router` already read and returns an `ExitCode` instead of a value:
//! for an agent driving this binary, the exit code is the result.
//!
//! Exit codes are the vocabulary `exit::Exit` defines. A refusal
//! carrying an `AxError` is printed by `city::report`, so its failure
//! line and its recovery line have one spelling across the whole binary.
//!
//! The point a reader most often gets wrong: `fork`, `adopt`, `replay`
//! and `export` read the city directory directly and never speak to a
//! serving process, while `enrol` (like `calling::call`) speaks only over
//! the wire and never touches the directory — the two routes to the same
//! city share nothing.

use super::city::report;
use super::grammar::Arguments;
use super::router::{client_summary, log_floor, log_levels};
use super::version::{check, cut};
use super::{DEPENDENCIES, install, wire_client};
use kernel::consts_policy::DEFAULT_AT;
use sprawling::{assembly, serving};
use std::process::ExitCode;

/// Hands a credential to a city on this machine, reading it from stdin.
///
/// Never from `argv`: a key on a command line is in the process table,
/// in shell history, and in the log of whatever started this process.
pub(super) fn enrol(read: &Arguments) -> ExitCode {
    let Some(reference) = read.positional(1) else {
        return ExitCode::from(2);
    };
    let Some((realm, name)) = wire_client::split_reference(reference) else {
        eprintln!("not a credential reference: {reference}");
        eprintln!("recovery: give it as <realm>/<name>, for example modelscope/api");
        return ExitCode::from(2);
    };
    let value = match std::io::read_to_string(std::io::stdin()) {
        Ok(text) => text.trim().to_owned(),
        Err(err) => {
            eprintln!("could not read the credential from stdin: {err}");
            return ExitCode::FAILURE;
        }
    };
    if value.is_empty() {
        eprintln!("nothing arrived on stdin");
        eprintln!(
            "recovery: pipe the value in, for example: cat key.txt | sprawling enrol {reference}"
        );
        return ExitCode::from(2);
    }
    let at = read.value("--at").unwrap_or(DEFAULT_AT);
    match wire_client::enrol(at, realm, name, &value) {
        Ok(reference) => {
            println!("{reference}");
            // Accepted, not yet stored: the route answers before the
            // worker has taken it (wire-SPEC.md section 8).
            eprintln!("accepted; the city stores it as soon as its worker is free");
            ExitCode::SUCCESS
        }
        Err(err) => report(err),
    }
}

/// Makes `sprawling` a word this machine's shells resolve, or unmakes
/// it. Nothing here needs administrator rights, because nothing outside
/// the person's own profile is touched.
pub(super) fn install(args: &[String]) -> ExitCode {
    let direction = if args.iter().any(|a| a == "--uninstall") {
        install::Direction::Uninstall
    } else {
        install::Direction::Install
    };
    let done = match install::install(direction) {
        Ok(done) => done,
        Err(err) => return report(err),
    };
    println!();
    match direction {
        install::Direction::Uninstall => println!("  removed {}", done.binary.display()),
        install::Direction::Install => println!("  installed {}", done.binary.display()),
    }
    match done.path {
        install::PathOutcome::Unchanged => println!("  PATH already said what it needed to say"),
        install::PathOutcome::Rewritten => {
            println!("  PATH rewritten for this user account");
            println!();
            println!("  Open a NEW shell window - a running one keeps the PATH it started with.");
        }
        install::PathOutcome::SelfService(line) => {
            println!("  that directory is not on PATH; add this line where you keep such lines:");
            println!();
            println!("      {line}");
        }
    }
    if let Some(notice) = done.notice {
        println!("  note: {notice}");
    }
    println!();
    ExitCode::SUCCESS
}

/// Writes a bundle: the history, the objects it points at, and the work.
/// Credentials are not in it, because they are not in the city.
pub(super) fn export(city: Option<&String>, dest: Option<&String>) -> ExitCode {
    let (Some(city), Some(dest)) = (city, dest) else {
        eprintln!("usage: sprawling export <city-dir> <bundle-dir>");
        return ExitCode::from(2);
    };
    match storage::Bundle::export(std::path::Path::new(city), std::path::Path::new(dest)) {
        Ok(manifest) => {
            println!(
                "exported {} record(s), {} object(s), {} file(s) to {dest}",
                manifest.records(),
                manifest.cas_objects(),
                manifest.files()
            );
            println!("chain head: {}", manifest.head());
            ExitCode::SUCCESS
        }
        Err(err) => report(err.into_ax()),
    }
}

/// Reads a bundle back into an empty directory. The chain is walked and
/// compared against the manifest before this reports success, so a short
/// copy is refused here rather than discovered later.
pub(super) fn restore(bundle: Option<&String>, city: Option<&String>) -> ExitCode {
    let (Some(bundle), Some(city)) = (bundle, city) else {
        eprintln!("usage: sprawling restore <bundle-dir> <city-dir>");
        return ExitCode::from(2);
    };
    match storage::Bundle::restore(std::path::Path::new(bundle), std::path::Path::new(city)) {
        Ok(manifest) => {
            println!(
                "restored {} record(s) into {city}; chain head {}",
                manifest.records(),
                manifest.head()
            );
            ExitCode::SUCCESS
        }
        Err(err) => report(err.into_ax()),
    }
}
/// Records a fork: a new run identity branched from an event node. The
/// lineage is the record; dispatching into it is the person's next move.
pub(super) fn fork(args: &[String]) -> ExitCode {
    let (Some(dir), Some(run_raw), Some(seq_raw)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: sprawling fork <city-dir> <run-id> <at-seq> [addr]");
        return ExitCode::from(2);
    };
    let run = match kernel::RunId::parse(run_raw) {
        Ok(run) => run,
        Err(err) => return report(err),
    };
    let Ok(seq) = seq_raw.parse::<u64>() else {
        eprintln!("not a sequence number: {seq_raw}");
        return ExitCode::from(2);
    };
    let addr = match args.get(4).map(|raw| kernel::Address::parse(raw)) {
        None => None,
        Some(Ok(addr)) => Some(addr),
        Some(Err(err)) => return report(err),
    };
    let (vault, _notice) = serving::open_vault();
    // Which room the branch lands in: the address the person named, or
    // the one the mother ran in.
    let Some(room) = addr else {
        eprintln!("usage: sprawling fork <city-dir> <run> <seq> <addr>");
        eprintln!("a branch opens a session in a room; name the room it opens in");
        return ExitCode::from(2);
    };
    let origin = kernel::Origin {
        run,
        at_seq: kernel::Seq::new(seq),
    };
    let outcome = assembly::RunWorker::new(
        std::path::Path::new(dir),
        vault,
        runtime::diagnostics::Diagnostics::off(),
    )
    .and_then(|mut worker| {
        worker.handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: Some(origin),
            idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"fork"),
        })
    });
    match outcome {
        Ok(()) => {
            println!("{} now branches from {run} at seq {seq}", room.as_str());
            println!("the next dispatch into it inherits that conversation");
            ExitCode::SUCCESS
        }
        Err(err) => report(err),
    }
}

/// Adopts an existing directory under the city as a building: RULES.toml
/// and the missing spine files are laid, nothing found is overwritten.
pub(super) fn adopt(dir: Option<&String>, addr: Option<&String>) -> ExitCode {
    let (Some(dir), Some(addr_raw)) = (dir, addr) else {
        eprintln!("usage: sprawling adopt <city-dir> <addr>");
        eprintln!("move or clone the directory under the city first, then adopt it");
        return ExitCode::from(2);
    };
    let addr = match kernel::Address::parse(addr_raw) {
        Ok(addr) => addr,
        Err(err) => return report(err),
    };
    let (vault, _notice) = serving::open_vault();
    let outcome = assembly::RunWorker::new(
        std::path::Path::new(dir),
        vault,
        runtime::diagnostics::Diagnostics::off(),
    )
    .and_then(|mut worker| worker.adopt_building(addr.clone()));
    match outcome {
        Ok(()) => {
            println!(
                "adopted {} - its files are untouched, its rules are new",
                addr.as_str()
            );
            println!("edit {}/RULES.toml to shape them", addr.as_str());
            ExitCode::SUCCESS
        }
        Err(err) => report(err),
    }
}

/// Offline chain verification (A2); strictly read-only.
pub(super) fn replay(dir: Option<&String>) -> ExitCode {
    let Some(dir) = dir else {
        eprintln!("usage: sprawling replay <ledger-dir>");
        return ExitCode::from(2);
    };
    match verified_chain(std::path::Path::new(dir)) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            eprintln!("recovery: {}", err.recovery());
            ExitCode::FAILURE
        }
    }
}

/// The line `replay` prints when the chain holds.
///
/// The path came from a person, so "there is no ledger here" is a thing
/// that happens, and it must not read as "the chain verified": a scripted
/// integrity check would otherwise score a mistyped argument as a pass.
/// An empty ledger is a different fact and still verifies, which is why
/// the question asked is whether a segment exists rather than whether a
/// line does (sprawling-SPEC section 12).
///
/// # Errors
/// `E_PATH_NOT_FOUND` when the directory holds no ledger segment, and
/// whatever chain verification says about a ledger that is there.
pub(super) fn verified_chain(dir: &std::path::Path) -> Result<String, kernel::AxError> {
    let segments = storage::ledger_segments_at(dir).map_err(storage::StorageError::into_ax)?;
    if segments.is_empty() {
        return Err(kernel::AxError::failure(
            kernel::AxCode::PathNotFound,
            "verify ledger",
            dir.display().to_string(),
        )
        .with_recovery(
            "no ledger segment here; point at the ledger directory itself \
             rather than at the city that contains it",
        ));
    }
    // Seqs run from 0 without a gap on a whole chain, so the tail seq is
    // the line count less one.
    match storage::audit_chain(dir).map_err(storage::StorageError::into_ax)? {
        storage::ChainAudit::Whole { lines } => Ok(format!(
            "chain verified: {lines} line(s), tail seq {}",
            lines
                .checked_sub(1)
                .map_or_else(|| "none".to_owned(), |tail| tail.to_string())
        )),
        storage::ChainAudit::Broken(reason) => Err(reason),
    }
}

pub(super) fn status(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "--deps") {
        print!("{DEPENDENCIES}");
        return ExitCode::SUCCESS;
    }
    println!(
        "sprawling {} (pre-alpha){}",
        env!("CARGO_PKG_VERSION"),
        cut()
    );
    println!("client: {}", client_summary());
    println!(
        "built from {} crate(s); list them with status --deps",
        DEPENDENCIES.lines().count()
    );
    match log_floor(args) {
        Ok(Some(level)) => println!("log: {level} and everything a wider audience reads"),
        Ok(None) => println!("log: off"),
        Err(unknown) => {
            eprintln!("not a log level: {unknown}");
            eprintln!("recovery: {}", log_levels());
            return ExitCode::from(2);
        }
    }
    if args.iter().any(|a| a == "--check") {
        return check();
    }
    ExitCode::SUCCESS
}
