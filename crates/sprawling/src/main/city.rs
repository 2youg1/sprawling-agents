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
//! `assembly::listen`, and print the banner only once that has taken the
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

#[path = "city/banner.rs"]
mod banner;
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

use banner::{Reader, print_banner, print_pairing};
pub(super) use opening::Entrance;
use opening::{Interactive, Open, opening};

pub(super) fn up(read: &Arguments, args: &[String]) -> ExitCode {
    let city = read
        .positional(1)
        .map_or_else(default_city_location, std::path::PathBuf::from);
    let addr = read.positional(2).map_or(DEFAULT_AT, String::as_str);
    up_at(&city, addr, args, Entrance::Up)
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
    let history = match city::has_history(folder) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == city::History::Present {
        println!("{} is already a city; opening it", folder.display());
        return serve_city(folder, DEFAULT_AT, &[], Entrance::Bare);
    }
    match assembly::form_city(folder, accounting::worker::Adopt::EveryFolder) {
        Ok(report) => {
            report_standing(&report);
            serve_city(folder, DEFAULT_AT, &[], Entrance::Bare)
        }
        Err(err) => report(err),
    }
}

/// What forming a city found, and what it did about it. Printed rather
/// than assumed, because the person is watching their own work being
/// taken in.
pub(super) fn report_standing(report: &accounting::worker::InitReport) {
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
                "found {} folder(s) and {loose} other item(s); no file in them was moved or rewritten",
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
/// then serve it on the face its entrance opens. The first screen and
/// the launcher in the release archive both arrive here, so the sequence
/// has exactly one definition and `init` and `serve` keep theirs.
pub(super) fn up_at(
    city: &std::path::Path,
    raw: &str,
    args: &[String],
    entrance: Entrance,
) -> ExitCode {
    let history = match city::has_history(city) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == city::History::Absent {
        match assembly::init_city(city) {
            Ok(raised) => println!(
                "city raised at {} (genesis seq {})",
                raised.ledger_dir.display(),
                raised.genesis.seq().value()
            ),
            Err(err) => return report(err),
        }
    }
    serve_city(city, raw, args, entrance)
}

/// The genesis write: a city is born when city_initialized becomes line
/// zero of its ledger (walkthrough step 1).
pub(super) fn init(read: &Arguments) -> ExitCode {
    let Some(dir) = read.positional(1) else {
        return ExitCode::from(2);
    };
    let adopt = if read.has("--adopt") {
        accounting::worker::Adopt::EveryFolder
    } else {
        accounting::worker::Adopt::Nothing
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
    serve_city(std::path::Path::new(dir), raw, args, Entrance::Serve)
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
    entrance: Entrance,
) -> ExitCode {
    let open = opening(args, entrance.opens());
    // A directory with no history is not a city, and saying so beats the
    // storage layer's report that it could not list a ledger directory -
    // which is true, unhelpful, and names a path nobody chose.
    let history = match city::has_history(city) {
        Ok(history) => history,
        Err(err) => return report(err),
    };
    if history == city::History::Absent {
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
    // (`crates/sprawling/spec/Supervising.lean` §8-109).
    // The face the terminal opens on is the entrance's; `--no-console`
    // is the way out for a supervisor that wants the old blocking shape.
    let surface = entrance.surface(args, Interactive::here());
    let wanted = surface.is_some();
    let owns_the_terminal = matches!(
        surface,
        Some(console::Surface::Cli | console::Surface::QuietHost)
    );
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
        Some(dir) => wire::ClientAssets::Disk(std::path::PathBuf::from(dir)),
        None => wire::ClientAssets::Embedded(CLIENT_FILES),
    };
    if let wire::ClientAssets::Embedded(_) = &client
        && !CLIENT_COMPLETE
    {
        eprintln!(
            "warning: this binary carries the page shell only; the browser will get an empty \
             page. Run `just build-web`, rebuild, or pass --web-dir with the sprawling \
             package's {CLIENT_BUNDLE_DIR} directory"
        );
    }
    // A city ended by force could not remove its redirect pages; their
    // open codes have expired, so the next start does it. Said, not
    // fatal: an expired code opens nothing.
    if let Err(unswept) =
        accounting::Clock::now(&assembly::SystemClock).and_then(firstrun::sweep_expired_redirects)
    {
        eprintln!("{}: {}", unswept.action(), unswept.recovery());
    }
    // The key is settled before anything binds. A configured token is
    // adopted; an address that reaches past this machine and has none
    // gets one minted for this serve alone. Read once here and never
    // stored - `serve` is handed a digest.
    let keyed = match serving::key_for(bind, std::env::var(child::PAIRING_TOKEN).ok()) {
        Ok(keyed) => keyed,
        Err(err) => return report(err),
    };
    let token = keyed.code().to_owned();
    // The socket's workers stand above the commands the city dispatches
    // (`crates/sprawling/spec/Serving/Standing.lean` §8-93).
    let core = serving::setting_telling_a_refusal();
    let runtime = match serving::serving_runtime(core) {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("could not start the async runtime: {err}");
            return ExitCode::FAILURE;
        }
    };
    let client_line = match &client {
        wire::ClientAssets::Disk(dir) => {
            format!("read per request from {}", dir.display())
        }
        wire::ClientAssets::Embedded(_) => client_summary(),
    };
    // The one sink a diagnostic line leaves this process through: the
    // terminal, and the page that has the log lens open. Made before
    // the `Diagnostics` because the sink is what writes into it.
    let journal = serving::Journal::new(std::sync::Arc::new(assembly::SystemClock));
    let journal = if owns_the_terminal {
        journal.lens_only()
    } else {
        journal
    };
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
    let (vault, vault_notice) = serving::open_vault();
    // The port and the writer are both taken before a word is printed:
    // a banner saying "running" over a port another process holds was a
    // claim the city could not keep (`crates/sprawling/spec/Assembly/Listening.lean` §8-88).
    let listening = match runtime.block_on(assembly::listen(serving::Serving {
        city_root: city.to_path_buf(),
        addr: bind,
        token,
        client,
        vault,
        vault_notice,
        log,
        journal,
        core,
    })) {
        Ok(listening) => listening,
        Err(err) => return report(err),
    };
    // Read from the listener, not from `bind`: a city asked for port 0
    // listens on the port the operating system gave (wire D16).
    let at = listening.local_addr();
    let url = listening.origins().url().to_owned();
    let console = surface.map(|surface| console::Terminal {
        url: url.clone(),
        token: keyed.shown().map(str::to_owned),
        city: city.display().to_string(),
        client: client_line.clone(),
        bind: at,
        surface,
    });
    // The banner is for a city with no face of its own: a harness reads
    // the URL from it. The CLI prints one header line, the quiet host
    // its two lines (`crates/sprawling/spec/Firstrun.lean` §8-8).
    if !owns_the_terminal {
        print_banner(city, &url, &client_line, &keyed);
        if let Some(level) = floor {
            println!("log: {level}");
        }
        print_pairing(if wanted {
            Reader::LineConsole
        } else {
            Reader::Log
        });
    }
    match open {
        Open::Browser => firstrun::open_when_ready(at, url, listening.door().clone()),
        Open::Nothing => {}
    }
    match runtime.block_on(listening.serve(console)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report(err),
    }
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
    let outcome = accounting::worker::RunWorker::new(
        std::path::Path::new(dir),
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(vault),
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
