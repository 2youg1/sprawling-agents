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

use sprawling::firstrun;

use super::city::{init, resume, serve, up, up_at, use_folder};
use super::data::{adopt, call, enrol, export, fork, install, replay, restore, status};
use super::{CLIENT_COMPLETE, CLIENT_FILES};
use std::process::ExitCode;

/// Every crate this binary is built from, `name version` per line -
/// the embedded half of the bill of materials (`xtask sbom` writes the
/// One line a person can read about what this binary carries.
pub(super) fn client_summary() -> String {
    if CLIENT_COMPLETE {
        let total: usize = CLIENT_FILES.iter().map(|f| f.gz.len()).sum();
        format!(
            "embedded, {} file(s), {total} gzipped byte(s)",
            CLIENT_FILES.len()
        )
    } else {
        "page shell only - run `just build-web`, then rebuild this binary".to_owned()
    }
}

/// The command list, in one place. The refusal of an unknown subcommand
/// and the first screen print the same text, so neither can fall behind
/// what the binary actually accepts.
pub(super) const COMMANDS: &str = "\
commands:
  up [dir] [addr]              raise a city here if needed, serve it, open the WebUI
  install [--uninstall]        put this binary on your PATH, or take it back off
  doctor [<city>] [--install]  what this machine has against what a city needs
                               (<city>: judge each building's bits; --install: offer
                               each missing item, one at a time; --explain <code>:
                               connect a refusal code to this machine)
  init <dir> [--adopt]         raise a city: writes the genesis record
                               (--adopt: every folder there becomes a building)
  serve <dir> [addr] [--open]  serve a city that already exists
                               (--console enters it, --no-console does not)
  resume <dir>                 after a restart: verify, close what was lost, report
  status [--deps]              this binary: version, client, what it is built from
  fork <dir> <run> <seq>       branch a lineage from one step of a run
  adopt <dir> <addr>           take an existing directory in as a building
  call <frame|-> [--at a]      send one wire frame, print every frame back
  enrol <realm>/<name>         read a credential from stdin, hand it to a city
  replay <ledger-dir>          verify a chain offline, read-only
  whose <city> <oid>           which run wrote a commit this city made
  export <city> <dest>         pack a whole city
  restore <bundle> <city>      unpack it on another machine";

pub(super) fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("status") => status(&args),
        Some("replay") => replay(named(&args, 1)),
        Some("whose") => super::whose::verb(named(&args, 1), named(&args, 2)),
        Some("init") => init(&args),
        Some("up") => up(&args),
        Some("install") => install(&args),
        Some("doctor") => sprawling::doctor::verb(&args),
        Some("call") => call(&args),
        Some("enrol" | "enroll") => enrol(&args),
        Some("serve") => serve(named(&args, 1), named(&args, 2), &args),
        Some("export") => export(named(&args, 1), named(&args, 2)),
        Some("restore") => restore(named(&args, 1), named(&args, 2)),
        Some("resume") => resume(named(&args, 1)),
        Some("fork") => fork(&args),
        Some("adopt") => adopt(named(&args, 1), named(&args, 2)),
        Some("help" | "--help" | "-h") => {
            println!("{COMMANDS}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            eprintln!("{COMMANDS}");
            ExitCode::from(2)
        }
        None => first_screen(),
    }
}

/// The nth positional argument: a word that is not a flag.
///
/// Without this, `sprawling init --help` raises a city in a directory
/// called `--help` - which is what the repository root of this project
/// held for a day. A flag is never a path, and the subcommands that take
/// a path all read it through here.
pub(super) fn named(args: &[String], nth: usize) -> Option<&String> {
    args.iter()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .nth(nth.saturating_sub(1))
}

/// What a launch with no command gets. Most of those come from a file
/// manager, where the console closes the moment this returns - so every
/// path out of here holds the window until somebody has read it.
pub(super) fn first_screen() -> ExitCode {
    let city = default_city_location();
    let answered = firstrun::ask(&city, &mut std::io::stdin().lock(), &mut std::io::stdout());
    let code = match answered {
        Ok(firstrun::FirstScreen::Start(city)) => up_at(&city, "127.0.0.1:8787", &[]),
        Ok(firstrun::FirstScreen::Use(folder)) => use_folder(&folder),
        Ok(firstrun::FirstScreen::Quit) => {
            println!("{COMMANDS}");
            ExitCode::from(2)
        }
        Err(err) => {
            eprintln!("could not read the answer: {err}");
            ExitCode::FAILURE
        }
    };
    hold();
    code
}

/// Keeps the window long enough to be read. A console opened by a file
/// manager closes with the process, which is how a refusal becomes an
/// unexplained flash. With nobody at the keyboard this returns at once.
fn hold() {
    println!("\n  Press Enter to close.");
    let mut ignored = String::new();
    // A failed read means there is nobody to wait for, which is the same
    // outcome as being waited for: this process is ending either way.
    let _ = std::io::stdin().read_line(&mut ignored);
}

/// Where `up` and the first screen put a city nobody named.
pub(super) fn default_city_location() -> std::path::PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from);
    match std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        Some(beside) => {
            let writable = firstrun::is_writable(&beside);
            firstrun::default_city(&beside, home.as_deref(), writable)
        }
        None => firstrun::default_city(std::path::Path::new("."), home.as_deref(), true),
    }
}

/// The one command that makes a city run: raise it when it is not there,
/// serve it, and open the WebUI once the port answers. The first screen
/// and the launcher in the release archive both arrive here, so the
/// sequence has exactly one definition and `init` and `serve` keep theirs.
/// The value following a `--flag`, if present.
pub(super) fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut pairs = args.windows(2);
    pairs.find_map(|pair| match pair {
        [name, value] if name == flag => Some(value.clone()),
        _ => None,
    })
}

/// The floor `--log <level>` asks for. Absent means the default floor;
/// `--log off` means nothing is written.
///
/// # Errors
/// Returns the word that is not a level, so the caller can name it.
pub(super) fn log_floor(args: &[String]) -> Result<Option<runtime::diagnostics::Level>, String> {
    let Some(asked) = flag_value(args, "--log") else {
        return Ok(Some(runtime::diagnostics::Level::DEFAULT));
    };
    if asked == "off" {
        return Ok(None);
    }
    runtime::diagnostics::Level::parse(&asked)
        .map(Some)
        .ok_or(asked)
}

pub(super) fn log_levels() -> String {
    let names: Vec<&str> = runtime::diagnostics::Level::ALL
        .into_iter()
        .map(|level| level.as_str())
        .collect();
    format!("use --log with one of {}, or off", names.join(", "))
}
