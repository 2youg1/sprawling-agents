// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! CLI entry. Subcommands land with their stages and are refused honestly
//! until then — a refusal that names what is missing beats a stub that
//! pretends (sprawling-SPEC.md). Live now: status, replay, init, serve,
//! export, restore, resume, fork.

// The city harness is the library half of this package (`src/lib.rs`);
// these two are the binary's own. `install` puts this executable where a
// shell will find it, and `wire_client` talks to a served city from a
// terminal - both are about the command line rather than about a city.

use super::city::{DEFAULT_AT, report};
use super::router::{client_summary, flag_value, log_floor, log_levels};
use super::{DEPENDENCIES, install, wire_client};
use sprawling::{assembly, serving};
use std::process::ExitCode;

/// Sends one frame over the wire and prints everything that comes back.
///
/// Exits 1 when the city refused something, so an agent driving this
/// learns the outcome from the exit code rather than by parsing JSON.
pub(super) fn call(args: &[String]) -> ExitCode {
    let Some(frame) = args.get(1).filter(|a| !a.starts_with("--")) else {
        eprintln!(
            "usage: sprawling call <frame-json|-> [--at host:port] [--token T] [--quiet-ms N]"
        );
        eprintln!("commands: {}", channels::COMMAND_NAMES.join(", "));
        eprintln!("queries:  {}", channels::QUERY_NAMES.join(", "));
        return ExitCode::from(2);
    };
    // `-` reads the frame from stdin, which is how a frame too long for
    // one command line, or one a script generated, gets in.
    let frame = if frame == "-" {
        match std::io::read_to_string(std::io::stdin()) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("could not read the frame from stdin: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        frame.clone()
    };
    let at = flag_value(args, "--at").unwrap_or_else(|| DEFAULT_AT.to_owned());
    let token = flag_value(args, "--token");
    let quiet = match flag_value(args, "--quiet-ms") {
        None => 2_000,
        Some(raw) => match raw.parse::<u64>() {
            Ok(ms) => ms,
            Err(_) => {
                eprintln!("not a number of milliseconds: {raw}");
                return ExitCode::from(2);
            }
        },
    };
    match wire_client::call(
        &at,
        &frame,
        token.as_deref(),
        std::time::Duration::from_millis(quiet),
    ) {
        Ok(heard) => {
            eprintln!("{} frame(s), {} refusal(s)", heard.frames, heard.refusals);
            if heard.refusals > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => report(err),
    }
}

/// Hands a credential to a city on this machine, reading it from stdin.
///
/// Never from `argv`: a key on a command line is in the process table,
/// in shell history, and in the log of whatever started this process.
pub(super) fn enrol(args: &[String]) -> ExitCode {
    let Some(reference) = args.get(1).filter(|a| !a.starts_with("--")) else {
        eprintln!("usage: sprawling enrol <realm>/<name> [--at host:port]");
        eprintln!("the value is read from stdin, never from the command line");
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
    let at = flag_value(args, "--at").unwrap_or_else(|| DEFAULT_AT.to_owned());
    match wire_client::enrol(&at, realm, name, &value) {
        Ok(reference) => {
            println!("{reference}");
            // Accepted, not yet stored: the route answers before the
            // worker has taken it (channels-SPEC.md section 8).
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
    let uninstall = args.iter().any(|a| a == "--uninstall");
    let done = match install::install(uninstall) {
        Ok(done) => done,
        Err(err) => return report(err),
    };
    println!();
    if uninstall {
        println!("  removed {}", done.binary.display());
    } else {
        println!("  installed {}", done.binary.display());
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
    match memory::Bundle::export(std::path::Path::new(city), std::path::Path::new(dest)) {
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
    match memory::Bundle::restore(std::path::Path::new(bundle), std::path::Path::new(city)) {
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
    let outcome = assembly::RunWorker::new(
        std::path::Path::new(dir),
        vault,
        runtime::diagnostics::Diagnostics::off(),
    )
    .and_then(|mut worker| worker.fork(run, kernel::Seq::new(seq), addr));
    match outcome {
        Ok(new_run) => {
            println!("forked as {new_run}");
            println!("dispatch into the address when ready; the lineage is recorded");
            ExitCode::SUCCESS
        }
        Err(err) => report(err),
    }
}

/// Adopts an existing directory under the city as a building: BUILDING.md
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
            println!("edit {}/BUILDING.md to shape them", addr.as_str());
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
    let segments = memory::ledger_segments_at(dir).map_err(memory::MemoryError::into_ax)?;
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
    let verified = runtime::replay::verify_ledger_dir(dir)?;
    Ok(format!(
        "chain verified: {} line(s), tail seq {}",
        verified.raw_lines().len(),
        verified
            .tail_seq()
            .map_or_else(|| "none".to_string(), |s| s.value().to_string())
    ))
}

pub(super) fn status(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "--deps") {
        print!("{DEPENDENCIES}");
        return ExitCode::SUCCESS;
    }
    println!("sprawling {} (pre-alpha)", env!("CARGO_PKG_VERSION"));
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
    ExitCode::SUCCESS
}
