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
use sprawling::assembly::SystemClock;
use sprawling::firstrun;

use super::calling::call;
use super::city::{Entrance, init, resume, serve, up, up_at, use_folder};
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
            if let Some(result) = runtime::tools::run_container_guard(&args) {
                return match result {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        eprint!(
                            "{}",
                            super::refusal::written(&error, super::refusal::Form::Human)
                        );
                        ExitCode::FAILURE
                    }
                };
            }
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
        Verb::PrivacyElevatedWrite => {
            match sprawling::privacy::cli::elevated_write(nth(1).map(String::as_str)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprint!(
                        "{}",
                        super::refusal::written(&error, super::refusal::Form::Human)
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Verb::PrivacyStatus => answered(sprawling::privacy::cli::status(), args),
        Verb::PrivacyInspect => answered(
            sprawling::privacy::cli::inspect(SystemClock, nth(1).map(String::as_str)),
            args,
        ),
        Verb::PrivacyApply => answered(
            sprawling::privacy::cli::apply(
                SystemClock,
                nth(1).map(String::as_str),
                nth(2).map(String::as_str),
            ),
            args,
        ),
        Verb::PrivacyRestore => answered(
            sprawling::privacy::cli::restore(
                SystemClock,
                nth(1).map(String::as_str),
                nth(2).map(String::as_str),
            ),
            args,
        ),
        Verb::PrivacyRestoreAll => answered(
            sprawling::privacy::cli::restore_all(SystemClock, |line| println!("{line}"))
                .map(|()| String::new()),
            args,
        ),
        Verb::PrivacyReconcile => answered(
            sprawling::privacy::cli::reconcile(SystemClock, nth(1).map(String::as_str)),
            args,
        ),
        Verb::Replay => replay(nth(1)),
        Verb::Whose => super::whose::verb(read),
        Verb::Check => super::check::verb(nth(1)),
        Verb::View => super::view::verb(read),
        Verb::PlaybackExport => super::playback::export(read),
        Verb::PlaybackCheck => super::playback::check(read),
        Verb::Init => init(read),
        Verb::Up => up(read, args),
        Verb::Install => install(args),
        Verb::Doctor => sprawling::doctor::verb(args, doctor_city),
        Verb::Desktop => super::desktop::verb(nth(1).map(String::as_str)),
        Verb::Call => call(args).into(),
        Verb::Dispatch => super::dispatch::verb(read),
        Verb::Gauge => super::gauge::verb(read),
        Verb::Enrol => enrol(read),
        Verb::Serve => serve(nth(1), nth(2), args),
        Verb::Export => export(nth(1), nth(2)),
        Verb::Restore => restore(nth(1), nth(2)),
        Verb::Resume => resume(nth(1)),
        Verb::Fork => fork(args),
        Verb::Adopt => adopt(nth(1), nth(2)),
    }
}

/// Prints a privacy verb's answer, or its refusal in the form the line
/// asked for.
fn answered(result: Result<String, kernel::AxError>, args: &[String]) -> ExitCode {
    match result {
        Ok(answer) => {
            if !answer.is_empty() {
                println!("{answer}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprint!(
                "{}",
                super::refusal::written(&error, super::refusal::Form::of(args))
            );
            ExitCode::FAILURE
        }
    }
}

/// What a launch with no command gets. Most of those come from a file
/// manager, where the console closes the moment this returns - so every
/// path out of here holds the window until somebody has read it.
pub(super) fn first_screen() -> ExitCode {
    let city = match city_to_open() {
        Ok(city) => city,
        Err(error) => {
            eprint!(
                "{}",
                super::refusal::written(&error, super::refusal::Form::Human)
            );
            hold();
            return ExitCode::FAILURE;
        }
    };
    // A city already there is opened rather than asked about: the
    // question exists so that genesis has somebody behind it.
    if city::has_history(&city).is_ok_and(|history| history == city::History::Present) {
        let code = up_at(&city, DEFAULT_AT, &[], Entrance::Bare);
        hold();
        return code;
    }
    let answered = firstrun::ask(&city, &mut std::io::stdin().lock(), &mut std::io::stdout());
    let code = match answered {
        Ok(firstrun::FirstScreen::Start(city)) => {
            name_city(&city);
            up_at(&city, DEFAULT_AT, &[], Entrance::Bare)
        }
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

/// Where `up` and the first screen put a city nobody named on the
/// command line: the one the person's file names, else the home
/// directory's (`crates/sprawling/spec/Firstrun.lean` §8-8).
///
/// # Errors
/// The person's file cannot be read or names a relative path, or there
/// is no home directory: a city is never placed beside the binary,
/// because the next release unpacks into a directory without it.
fn default_city_location() -> Result<std::path::PathBuf, kernel::AxError> {
    match accounting::person::city()? {
        Some(named) => Ok(named),
        None => Ok(accounting::home::Home::detect()?.default_city()),
    }
}

/// The city a launch that names none opens: [`default_city_location`],
/// after a city an earlier release kept beside the binary has been
/// moved there, which happens only while the person's file names no
/// city (`crates/sprawling/spec/Firstrun.lean` §8-8).
///
/// # Errors
/// Everything [`default_city_location`] refuses, and a move the old
/// folder or the new one refuses.
pub(super) fn city_to_open() -> Result<std::path::PathBuf, kernel::AxError> {
    if accounting::person::city()?.is_none() {
        move_home_from_beside_the_binary()?;
    }
    default_city_location()
}

/// Moves the city an earlier release kept beside this binary to the
/// home directory's default, or leaves it where it is when the home
/// directory is on another volume; says which, and names the city's
/// folder in the person's file so the move is never looked for again.
fn move_home_from_beside_the_binary() -> Result<(), kernel::AxError> {
    let exe_dir = match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return Ok(()),
        },
        // A binary that cannot find itself has no folder beside it to
        // look in, which is the same answer as a folder with no city.
        Err(_) => return Ok(()),
    };
    let to = accounting::home::Home::detect()?.default_city();
    match sprawling::beside_binary::bring_home(&exe_dir, &to)? {
        sprawling::beside_binary::Homing::NothingBeside => Ok(()),
        sprawling::beside_binary::Homing::Moved { from, to, relinked } => {
            println!("moved the city from {} to {}", from.display(), to.display());
            match relinked {
                Ok(count) => println!("{count} worktree(s) re-pointed at the new folder"),
                Err(error) => eprintln!(
                    "{}the spare worktree is checked out again when the city next needs one",
                    super::refusal::written(&error, super::refusal::Form::Human)
                ),
            }
            // The city has moved whatever the person's file says next: a
            // failed write is told, and the next launch finds the city at
            // the default, where it now is.
            name_city(&to);
            Ok(())
        }
        sprawling::beside_binary::Homing::OnAnotherVolume { at, home } => {
            println!(
                "the city stays at {}: {} is on another volume, and this release does not copy a city across volumes",
                at.display(),
                home.display()
            );
            println!(
                "to move it yourself, stop the city, move that folder whole, then change the city line in the person's config.toml"
            );
            // Naming the old place is what makes this the only time it is
            // said: the next launch finds a city in the person's file and
            // looks beside the binary no more.
            name_city(&at);
            Ok(())
        }
    }
}

/// Where the doctor scans when the line names no city. When `up` would
/// refuse, the refusal is shown and the scan judges an empty path,
/// which reports that no city is there.
fn doctor_city() -> std::path::PathBuf {
    default_city_location().unwrap_or_else(|error| {
        eprint!(
            "{}",
            super::refusal::written(&error, super::refusal::Form::Human)
        );
        std::path::PathBuf::new()
    })
}

/// Names the city the person chose on the first screen in their file,
/// so the next launch with no command opens it. A file that refuses
/// the write is reported and the city still starts: the person is
/// asked again next time, which is what happened before they chose.
pub(super) fn name_city(city: &std::path::Path) {
    if let Err(error) = accounting::person::settle_city(city) {
        eprint!(
            "{}",
            super::refusal::written(&error, super::refusal::Form::Human)
        );
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
    format!("use --log with one of {}", level_names())
}

/// The words `--log` takes, from the one list of levels: what its help
/// shows and what its refusal names.
pub(super) fn level_names() -> String {
    let names: Vec<&str> = runtime::diagnostics::Level::ALL
        .into_iter()
        .map(|level| level.as_str())
        .collect();
    format!("{}, or off", names.join(", "))
}
