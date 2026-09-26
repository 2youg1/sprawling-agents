// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a command line becomes a subcommand: the word a person typed,
//! the positional arguments behind it, and the screen a launch with no
//! command gets.
//!
//! Nothing here does the work. Every arm hands off to `city` or to
//! `data`, so this module holds the reading of arguments and the list of
//! commands, and the commands themselves stay where they belong.

use kernel::consts_policy::DEFAULT_AT;
use sprawling::firstrun;

use super::calling::call;
use super::city::{init, resume, serve, up, up_at, use_folder};
use super::data::{adopt, enrol, export, fork, install, replay, restore, status};
use super::grammar::{Arguments, Invocation, parse};
use super::verbs::{self, Verb};
use super::{CLIENT_COMPLETE, CLIENT_FILES};
use std::process::ExitCode;

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

pub(super) fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(Invocation::FirstScreen) => first_screen(),
        Ok(Invocation::Overview) => {
            println!("{}", verbs::overview());
            ExitCode::SUCCESS
        }
        Ok(Invocation::Help(verb)) => {
            if let Some(row) = verbs::row(verb) {
                println!("{}", verbs::help(row));
            }
            ExitCode::SUCCESS
        }
        // One implementation: the release a binary names cannot depend
        // on which word asked for it.
        Ok(Invocation::Version) => status(&args),
        Ok(Invocation::Run(verb, read)) => run(verb, &read, &args),
        Err(refused) => {
            eprintln!("sprawling: {refused}");
            ExitCode::from(2)
        }
    }
}

/// Hands a read command line to the verb that carries it out. `args` is
/// the raw line for the verbs that still read their own flags.
fn run(verb: Verb, read: &Arguments, args: &[String]) -> ExitCode {
    let nth = |n| read.positional(n);
    match verb {
        Verb::Status => status(args),
        Verb::Replay => replay(nth(1)),
        Verb::Whose => super::whose::verb(nth(1), nth(2)),
        Verb::Check => super::check::verb(nth(1)),
        Verb::View => super::view::verb(read),
        Verb::Init => init(read),
        Verb::Up => up(read, args),
        Verb::Install => install(args),
        Verb::Doctor => sprawling::doctor::verb(args),
        Verb::Call => call(args).into(),
        Verb::Dispatch => super::dispatch::verb(read),
        Verb::Enrol => enrol(read),
        Verb::Serve => serve(nth(1), nth(2), args),
        Verb::Export => export(nth(1), nth(2)),
        Verb::Restore => restore(nth(1), nth(2)),
        Verb::Resume => resume(nth(1)),
        Verb::Fork => fork(args),
        Verb::Adopt => adopt(nth(1), nth(2)),
    }
}

/// What a launch with no command gets. Most of those come from a file
/// manager, where the console closes the moment this returns - so every
/// path out of here holds the window until somebody has read it.
pub(super) fn first_screen() -> ExitCode {
    let city = default_city_location();
    let answered = firstrun::ask(&city, &mut std::io::stdin().lock(), &mut std::io::stdout());
    let code = match answered {
        Ok(firstrun::FirstScreen::Start(city)) => up_at(&city, DEFAULT_AT, &[]),
        Ok(firstrun::FirstScreen::Use(folder)) => use_folder(&folder),
        Ok(firstrun::FirstScreen::Quit) => {
            println!("{}", verbs::overview());
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
    drop(std::io::stdin().read_line(&mut ignored));
}

/// Where `up` and the first screen put a city nobody named.
pub(super) fn default_city_location() -> std::path::PathBuf {
    // A missing home is not a refusal here: the city then goes beside
    // the binary, which `firstrun::default_city` decides.
    let home = sprawling::home::Home::detect().ok();
    match std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        Some(beside) => {
            let takes_a_city = firstrun::writability(&beside);
            firstrun::default_city(&beside, home.as_ref(), takes_a_city)
        }
        // No directory to ask about is the working directory, which
        // this process is already running in.
        None => firstrun::default_city(
            std::path::Path::new("."),
            home.as_ref(),
            firstrun::BesideBinary::Writable,
        ),
    }
}

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
