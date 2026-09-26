// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The subcommands that raise a city and serve it: `up`, `use_folder`,
//! `init`, `serve` and `resume`.
//!
//! [`serve_city`] is the single definition of what serving means, and
//! every route into a running city passes through it: refuse a
//! directory that holds no history, settle the pairing key before
//! anything binds, choose where the client bundle comes from, build the
//! diagnostics sink, attach the console, hand one `serving::Serving` to
//! `serving::listen`, and print the banner only once that has taken the
//! port and the city's writer. `up`, the first screen and `serve`
//! differ only in what they do before they arrive there and in whether
//! they open a browser, so none of them re-derives the sequence.
//!
//! [`report`] is where an `AxError` becomes an exit: the failure line,
//! then its recovery line, then `ExitCode::FAILURE`. The verbs in `data`
//! print their refusals through this same function, so the whole binary
//! refuses in one voice.
//!
//! The point a reader most often gets wrong: `up` raises a city that is
//! not there and `serve` refuses one. That difference is deliberate, so
//! that a mistyped path becomes a refusal rather than an empty city at a
//! location nobody looked at.

#[path = "city/opening.rs"]
mod opening;

use super::exit::Exit;
use super::grammar::Arguments;
use super::refusal::{Form, written};
use super::router::{client_summary, default_city_location, flag_value, log_floor, log_levels};
use super::{CLIENT_BUNDLE_DIR, CLIENT_COMPLETE, CLIENT_FILES};
use kernel::consts_policy::DEFAULT_AT;
use sprawling::{assembly, console, firstrun, serving};
use std::process::ExitCode;

use opening::{Open, opening};

pub(super) fn up(read: &Arguments, args: &[String]) -> ExitCode {
    let city = read
        .positional(1)
        .map_or_else(default_city_location, std::path::PathBuf::from);
    let addr = read.positional(2).map_or(DEFAULT_AT, String::as_str);
    up_at(&city, addr, args)
}

/// A folder the person already works in becomes a city around their
/// work.
///
/// The folder has to be there. A path that is not a directory is
/// reported and nothing is created: the alternative is making a city out
/// of a typo, at a location nobody looked at.
pub(super) fn use_folder(folder: &std::path::Path) -> ExitCode {
    if !folder.is_dir() {
        eprintln!("{} is not a folder on this machine", folder.display());
        eprintln!(
            "recovery: paste the path of a folder you already work in, or press Enter to start a new city"
        );
        return ExitCode::FAILURE;
    }
    let history = match assembly::has_history(folder) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == assembly::History::Present {
        println!("{} is already a city; opening it", folder.display());
        return serve_city(folder, DEFAULT_AT, &[], opening(&[], Open::Browser));
    }
    match assembly::form_city(folder, assembly::Adopt::EveryFolder) {
        Ok(report) => {
            report_standing(&report);
            serve_city(folder, DEFAULT_AT, &[], opening(&[], Open::Browser))
        }
        Err(err) => report(err),
    }
}

/// What forming a city found, and what it did about it. Printed rather
/// than assumed, because the person is watching their own work being
/// taken in.
pub(super) fn report_standing(report: &assembly::InitReport) {
    println!(
        "city raised: ledger at {} (genesis seq {})",
        report.ledger_dir.display(),
        report.genesis.seq().value()
    );
    match &report.standing {
        city::Standing::Empty => println!("the folder was empty; nothing was there to touch"),
        city::Standing::AlreadyACity => println!("the folder was already a city"),
        city::Standing::Work { adoptable, loose } => {
            println!(
                "found {} folder(s) and {loose} other item(s); nothing in them was read, moved or rewritten",
                adoptable.len()
            );
        }
    }
    for addr in &report.adopted {
        println!(
            "  {} is now a building - edit its rules on its page",
            addr.as_str()
        );
    }
}

/// The one command that makes a city run: raise it when it is not there,
/// serve it, and open the WebUI once the port answers. The first screen
/// and the launcher in the release archive both arrive here, so the
/// sequence has exactly one definition and `init` and `serve` keep theirs.
pub(super) fn up_at(city: &std::path::Path, raw: &str, args: &[String]) -> ExitCode {
    let history = match assembly::has_history(city) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == assembly::History::Absent {
        match assembly::init_city(city) {
            Ok(raised) => println!(
                "city raised at {} (genesis seq {})",
                raised.ledger_dir.display(),
                raised.genesis.seq().value()
            ),
            Err(err) => return report(err),
        }
    }
    serve_city(city, raw, args, opening(args, Open::Browser))
}

/// The genesis write: a city is born when city_initialized becomes line
/// zero of its ledger (walkthrough step 1).
pub(super) fn init(read: &Arguments) -> ExitCode {
    let Some(dir) = read.positional(1) else {
        return ExitCode::from(2);
    };
    let adopt = if read.has("--adopt") {
        assembly::Adopt::EveryFolder
    } else {
        assembly::Adopt::Nothing
    };
    match assembly::form_city(std::path::Path::new(dir), adopt) {
        Ok(report) => {
            report_standing(&report);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            eprintln!("recovery: {}", err.recovery());
            ExitCode::FAILURE
        }
    }
}

pub(super) fn report(err: kernel::AxError) -> ExitCode {
    eprint!("{}", written(&err, Form::Human));
    Exit::Refused.into()
}

/// Binds the control surface. Loopback unless an address says otherwise,
/// and an address beyond this machine needs `SPRAWLING_PAIRING_TOKEN` -
/// refused at startup, not at connect time.
pub(super) fn serve(dir: Option<&String>, addr: Option<&String>, args: &[String]) -> ExitCode {
    let Some(dir) = dir else {
        return ExitCode::from(2);
    };
    let raw = addr.map_or(DEFAULT_AT, String::as_str);
    let open = opening(args, Open::Nothing);
    serve_city(std::path::Path::new(dir), raw, args, open)
}

/// The child `serve` of a supervised run, from the decisions made above.
fn child(args: &[String], open: Open, wanted: bool) -> sprawling::supervising::Child {
    use sprawling::supervising::{Child, Console, Window};
    Child {
        forwarded: super::verbs::forwarded(args),
        first: match open {
            Open::Browser => Window::Open,
            Open::Nothing => Window::Leave,
        },
        console: if wanted {
            Console::Enter
        } else {
            Console::Skip
        },
    }
}

pub(super) fn serve_city(
    city: &std::path::Path,
    raw: &str,
    args: &[String],
    open: Open,
) -> ExitCode {
    // A directory with no history is not a city, and saying so beats the
    // storage layer's report that it could not list a ledger directory -
    // which is true, unhelpful, and names a path nobody chose.
    let history = match assembly::has_history(city) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == assembly::History::Absent {
        eprintln!("no city at {}", city.display());
        eprintln!(
            "recovery: `sprawling up {0}` raises one and serves it",
            city.display()
        );
        eprintln!(
            "          `sprawling init {0}` raises one and stops",
            city.display()
        );
        return ExitCode::from(2);
    }
    let Ok(bind) = raw.parse() else {
        eprintln!("not a socket address: {raw}");
        eprintln!("recovery: give host:port, for example {DEFAULT_AT}");
        return ExitCode::from(2);
    };
    // After the refusals a restart could not cure, so a mistyped line
    // is refused once here rather than spending the crash budget
    // (sprawling-SPEC.md 8-90).
    // The terminal this city runs in becomes its console when `up`
    // started it, or when `serve` was asked. `--no-console` is the way
    // out for a supervisor that wants the old blocking shape.
    let wanted = (open == Open::Browser || args.iter().any(|a| a == "--console"))
        && !args.iter().any(|a| a == "--no-console");
    if args.iter().any(|a| a == "--supervise") {
        return match sprawling::supervising::supervise(city, raw, &child(args, open, wanted)) {
            Ok(sprawling::supervising::Ended::Chosen) => ExitCode::SUCCESS,
            Ok(sprawling::supervising::Ended::Degraded) => ExitCode::FAILURE,
            Err(err) => report(err),
        };
    }
    // The client source: embedded by default; a directory for the
    // development loop, read per request so an edit shows on refresh.
    let client = match flag_value(args, "--web-dir") {
        Some(dir) => channels::ClientAssets::Disk(std::path::PathBuf::from(dir)),
        None => channels::ClientAssets::Embedded(CLIENT_FILES),
    };
    if let channels::ClientAssets::Embedded(_) = &client
        && !CLIENT_COMPLETE
    {
        eprintln!(
            "warning: this binary carries the page shell only; the browser will get an empty \
             page. Run `just build-web`, rebuild, or pass --web-dir {CLIENT_BUNDLE_DIR}"
        );
    }
    // The key is settled before anything binds. A configured token is
    // adopted; an address that reaches past this machine and has none
    // gets one minted for this serve alone. Read once here and never
    // stored - `serve` is handed a digest.
    let keyed = match serving::key_for(bind, std::env::var("SPRAWLING_PAIRING_TOKEN").ok()) {
        Ok(keyed) => keyed,
        Err(err) => return report(err),
    };
    let token = keyed.code().map(str::to_owned);
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("could not start the async runtime: {err}");
            return ExitCode::FAILURE;
        }
    };
    let client_line = match &client {
        channels::ClientAssets::Disk(dir) => {
            format!("read per request from {}", dir.display())
        }
        channels::ClientAssets::Embedded(_) => client_summary(),
    };
    // The one sink a diagnostic line leaves this process through: the
    // terminal, and the page that has the log lens open. Made before
    // the `Diagnostics` because the sink is what writes into it.
    let journal = serving::Journal::new();
    let floor = match log_floor(args) {
        Ok(floor) => floor,
        Err(unknown) => {
            eprintln!("not a log level: {unknown}");
            eprintln!("recovery: {}", log_levels());
            return ExitCode::from(2);
        }
    };
    let log = match floor {
        Some(level) => runtime::diagnostics::Diagnostics::new(level, journal.sink()),
        None => runtime::diagnostics::Diagnostics::off(),
    };
    let console = wanted.then(|| console::Terminal {
        url: firstrun::local_url(bind),
        token: token.clone(),
        // The three facts the banner below prints. `/serving`
        // reprints them on demand, because the event stream scrolls
        // them away within seconds of a city getting busy.
        city: city.display().to_string(),
        client: client_line.clone(),
        bind,
    });
    let (vault, vault_notice) = serving::open_vault();
    // The port and the writer are both taken before a word is printed:
    // a banner saying "running" over a port another process holds was a
    // claim the city could not keep (sprawling-SPEC.md 8-88).
    let listening = match runtime.block_on(serving::listen(serving::Serving {
        city_root: city.to_path_buf(),
        addr: bind,
        token,
        client,
        vault,
        vault_notice,
        log,
        journal,
        console,
    })) {
        Ok(listening) => listening,
        Err(err) => return report(err),
    };
    let url = firstrun::local_url(bind);
    print_banner(city, &url, &client_line, &keyed);
    if let Some(level) = floor {
        println!("log: {level}");
    }
    if wanted {
        println!("  This terminal is the console. `/help` lists what it takes,");
        println!("  and `/serving` says where this city listens and what is running in it.");
        println!();
    }
    match open {
        Open::Browser => firstrun::open_when_ready(bind, url),
        Open::Nothing => {}
    }
    match runtime.block_on(listening.serve()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report(err),
    }
}

/// What a person reads once the city listens: where it is, where to
/// open it, what client it serves, and the key when one was minted.
fn print_banner(city: &std::path::Path, url: &str, client_line: &str, keyed: &serving::Keyed) {
    println!();
    println!("  sprawling is running.");
    println!();
    println!("    city     {}", city.display());
    println!("    WebUI    {url}");
    println!("    client   {client_line}");
    println!();
    match keyed {
        serving::Keyed::NothingToPresent => {}
        serving::Keyed::Adopted(_) => {
            println!("    key      the one you configured; this city will ask for it");
            println!();
        }
        // Shown here and nowhere else, for as long as this process
        // lives. Nothing writes it down, so a person who loses it stops
        // and starts the city again rather than looking for a file.
        serving::Keyed::Minted(code) => {
            println!("    key      {code}");
            println!();
            println!("  This address reaches past this machine, so the city minted a key.");
            println!("  It is shown once, kept nowhere, and replaced the next time you start.");
            println!("  Open:    {}/?token={code}", url.trim_end_matches('/'));
            println!();
        }
    }
    println!("  Open the WebUI in a browser. Ctrl-C stops the city.");
    println!();
}

/// The startup scan: verify the chain, close every tool call whose
/// outcome a process death left unknown, and say what still waits on a
/// person. `serve` continues approved work; this is the offline half.
pub(super) fn resume(dir: Option<&String>) -> ExitCode {
    let Some(dir) = dir else {
        eprintln!("usage: sprawling resume <city-dir>");
        return ExitCode::from(2);
    };
    let (vault, _notice) = serving::open_vault();
    let outcome = assembly::RunWorker::new(
        std::path::Path::new(dir),
        vault,
        runtime::diagnostics::Diagnostics::off(),
    )
    .and_then(|mut worker| worker.startup_scan());
    match outcome {
        Ok(report) => {
            println!("{}", report.summary());
            if report.waiting_approvals > 0 {
                println!("answer them in the interface: sprawling serve {dir}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => report(err),
    }
}
