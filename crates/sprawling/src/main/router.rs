// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

fn run(verb: Verb, read: &Arguments, args: &[String]) -> ExitCode {
    let nth = |n| read.positional(n);
    match verb {
        Verb::Mei => sprawling::console::mei::verb(
            read.value("--replay"),
            std::sync::Arc::new(|| accounting::Clock::now(&SystemClock)),
        ),
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
        Verb::Secrets => super::view::secrets(read),
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

fn hold() {
    println!("\n  Press Enter to close.");
    let mut ignored = String::new();
    drop(std::io::stdin().read_line(&mut ignored));
}

fn default_city_location() -> Result<std::path::PathBuf, kernel::AxError> {
    match accounting::person::city()? {
        Some(named) => Ok(named),
        None => Ok(accounting::home::Home::detect()?.default_city()),
    }
}

pub(super) fn city_to_open() -> Result<std::path::PathBuf, kernel::AxError> {
    if accounting::person::city()?.is_none()
        && let Some(stayed) = move_home_from_beside_the_binary()?
    {
        return Ok(stayed);
    }
    default_city_location()
}

fn move_home_from_beside_the_binary() -> Result<Option<std::path::PathBuf>, kernel::AxError> {
    let exe_dir = match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return Ok(None),
        },
        Err(_) => return Ok(None),
    };
    let to = accounting::home::Home::detect()?.default_city();
    match sprawling::beside_binary::bring_home(&exe_dir, &to)? {
        sprawling::beside_binary::Homing::NothingBeside => Ok(None),
        sprawling::beside_binary::Homing::Moved { from, to, relinked } => {
            println!("moved the city from {} to {}", from.display(), to.display());
            match relinked {
                Ok(count) => println!("{count} worktree(s) re-pointed at the new folder"),
                Err(error) => eprintln!(
                    "{}the spare worktree is checked out again when the city next needs one",
                    super::refusal::written(&error, super::refusal::Form::Human)
                ),
            }
            name_city(&to);
            Ok(None)
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
            name_city(&at);
            Ok(Some(at))
        }
    }
}

fn doctor_city() -> std::path::PathBuf {
    default_city_location().unwrap_or_else(|error| {
        eprint!(
            "{}",
            super::refusal::written(&error, super::refusal::Form::Human)
        );
        std::path::PathBuf::new()
    })
}

pub(super) fn name_city(city: &std::path::Path) {
    if let Err(error) = accounting::person::settle_city(city) {
        eprint!(
            "{}",
            super::refusal::written(&error, super::refusal::Form::Human)
        );
    }
}

pub(super) fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut pairs = args.windows(2);
    pairs.find_map(|pair| match pair {
        [name, value] if name == flag => Some(value.clone()),
        _ => None,
    })
}

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

pub(super) fn level_names() -> String {
    let names: Vec<&str> = runtime::diagnostics::Level::ALL
        .into_iter()
        .map(|level| level.as_str())
        .collect();
    format!("{}, or off", names.join(", "))
}
