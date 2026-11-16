// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[path = "bench_startup/actions.rs"]
mod actions;
#[path = "bench_startup/samples.rs"]
mod samples;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kernel::{AxCode, AxError};
use sprawling::monitor::spread::SUSPICIOUS_TIMES;

use actions::Action;
use actions::history::History;
use samples::{Share, Tier};

enum Part {
    Everything,
    FirstByte,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a bench harness is a measuring Main; time is its subject, not its input"
)]
fn stamp() -> std::time::Instant {
    std::time::Instant::now()
}

fn main() -> ExitCode {
    let machine = format!(
        "{}-{}, {} core(s)",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::thread::available_parallelism().map_or(0, std::num::NonZero::get)
    );
    println!("bench_startup on {machine}");
    println!(
        "machine class: the reference class - one ordinary machine rather than a benchmark \
         host. This harness can see neither the disk nor the memory, so the register row that \
         records these readings states them."
    );
    println!(
        "{} samples per action; p50/p95/p99 nearest-rank over every sample; \
         suspicious = over {SUSPICIOUS_TIMES} x p50, flagged and never removed\n",
        actions::SAMPLES
    );
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("recovery: {}", err.recovery());
            ExitCode::FAILURE
        }
    }
}

fn executable_name() -> String {
    format!("sprawling{}", std::env::consts::EXE_SUFFIX)
}

fn run() -> Result<(), AxError> {
    let binary = shipped_binary()?;
    match std::env::args().nth(1).as_deref() {
        None => Ok(Part::Everything),
        Some("first-byte") => Ok(Part::FirstByte),
        Some(other) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "choose which readings to take",
            other.to_owned(),
        )
        .with_recovery("give nothing for every reading, or `first-byte` for that one alone")),
    }
    .and_then(|part| match part {
        Part::Everything => four_actions(&binary),
        Part::FirstByte => Ok(()),
    })?;
    first_bytes(&binary)
}

fn first_bytes(binary: &Path) -> Result<(), AxError> {
    let Some(cities) = binary
        .parent()
        .and_then(Path::parent)
        .map(|target| target.join("bench-cities"))
    else {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "find the build directory",
            binary.display().to_string(),
        )
        .with_recovery("run just bench-startup, which builds the binary it measures first"));
    };
    std::fs::create_dir_all(&cities).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "keep the fixture cities",
            format!("{}: {err}", cities.display()),
        )
        .with_recovery("free space beside the build directory and run again")
    })?;
    println!(
        "
first byte: `serve` spawned to the first byte of GET /, fixture cities in {}",
        cities.display()
    );
    println!(
        "two readings compare only when their fixture digests are equal; each serve's \
         standard error is kept beside its city, and its `opened the city in` line is the \
         product's own split of the same opening (`Sprawling/app/Spec.lean` §8-121)"
    );
    println!(
        "{:<8} {:>8} {:>10} {:>10} {:>10}   {:<16}   serve log",
        "city", "samples", "floor ms", "p50 ms", "peak ms", "fixture"
    );
    for (name, history, samples) in [
        ("empty", History::Empty, 20),
        ("l100k", History::Runs(2_000), 5),
        ("l400k", History::Runs(8_000), 3),
    ] {
        let city = actions::history::fixture_city(&cities, name, history)?;
        let fixture = citysim::ledger_digest(&kernel::layout::CityLayout::new(&city).ledger())?;
        let log = cities.join(format!("{name}.serve.log"));
        let taken = actions::first_byte::first_byte(binary, &city, samples, &log)?;
        println!(
            "{name:<8} {samples:>8} {:>10.3} {:>10.3} {:>10.3}   {:<16}   {}",
            ms(taken.floor()),
            ms(taken.p(Share::P50)),
            ms(taken.peak()),
            citysim::fixture_label(&fixture),
            log.display()
        );
    }
    Ok(())
}

fn four_actions(binary: &Path) -> Result<(), AxError> {
    let scratch = scratch_dir()?;
    let archive = actions::archive::fixture(&scratch, binary)?;
    let archive_bytes = std::fs::metadata(&archive)
        .map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "weigh the archive",
                format!("{}: {err}", archive.display()),
            )
            .with_recovery(
                "the scratch directory should hold the fixture for the length of the run",
            )
        })?
        .len();
    let install = actions::install(&scratch, &archive)?;
    let startup = actions::startup(binary)?;
    let raising = actions::raise_city(&scratch)?;

    let session_city = scratch.join("session-city");
    let session = actions::open_session(&session_city)?;
    let rows: [(&str, &Action); 4] = [
        ("install", &install),
        ("startup", &startup),
        ("raise_city", &raising),
        ("open_session", &session),
    ];
    report(&rows, archive_bytes);
    match std::fs::remove_dir_all(&scratch) {
        Ok(()) => {}
        Err(err) => println!("scratch left behind at {}: {err}", scratch.display()),
    }
    Ok(())
}

fn report(rows: &[(&str, &Action)], archive_bytes: u64) {
    println!(
        "{:<12} {:>9} {:>9} {:>9} {:>9} {:>9}   suspicious",
        "action", "p50 ms", "p95 ms", "p99 ms", "floor ms", "peak ms"
    );
    for (name, action) in rows {
        let total = &action.total;
        println!(
            "{name:<12} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3}   {} of {}",
            ms(total.p(Share::P50)),
            ms(total.p(Share::P95)),
            ms(total.p(Share::P99)),
            ms(total.floor()),
            ms(total.peak()),
            total.suspicious(),
            actions::SAMPLES
        );
    }
    println!("\nthree-piece (second tier at p99 <= 1 ms | limit reading | dominant sub-step)");
    for (name, action) in rows {
        let tier = match action.total.tier() {
            Tier::Within => "Within",
            Tier::Outside => "Outside",
        };
        let steps: Vec<String> = action
            .steps
            .iter()
            .map(|(label, step)| format!("{label} {:.3}", ms(step.p(Share::P50))))
            .collect();
        let dominant = match actions::dominant(&action.steps) {
            Some(label) => format!("{label} of [{}]", steps.join(", ")),
            None => "no sub-step split: attribute with the counts below".to_owned(),
        };
        println!(
            "{name:<12} {tier} | floor {:.3} ms | {dominant}",
            ms(action.total.floor())
        );
    }
    println!("\nsub-metrics (each counted, per sample)");
    for (name, action) in rows {
        let counts = &action.per_sample;
        println!(
            "  {name:<12} processes {}, files {}, barriers {}",
            counts.processes, counts.files, counts.barriers
        );
    }
    println!(
        "  a barrier is one ledger append; the floor for one is the `durability_barrier` \
         row of `just bench`, never re-measured here"
    );
    println!(
        "  archive digest   install only: SHA-256 over {archive_bytes} bytes, which is the \
         check `install.sh` and `install.ps1` perform before they unpack; its cost is the \
         `digest` sub-step of the three-piece line above"
    );
    println!(
        "  verify hashing   0 ms: release signing is not wired, so no signature is verified. Recorded as zero and \
         named here; the sub-step stays when signing lands"
    );
    let marks: Vec<String> = rows
        .iter()
        .map(|(name, action)| {
            let mut at = Vec::new();
            for index in 0..actions::SAMPLES {
                if action.total.kind_at(index) == samples::SampleKind::Suspicious {
                    at.push(format!("#{index}"));
                }
            }
            format!("{name} {} at {}", action.total.suspicious(), at.join(","))
        })
        .collect();
    println!(
        "  interference    suspicious samples (over {SUSPICIOUS_TIMES} x p50): {}",
        marks.join(" | ")
    );
}

fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn scratch_dir() -> Result<PathBuf, AxError> {
    let dir = std::env::temp_dir().join(format!("sprawl-bench-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "create the scratch directory",
            format!("{}: {err}", dir.display()),
        )
        .with_recovery("free space on the temporary drive and run again")
    })?;
    Ok(dir)
}

fn shipped_binary() -> Result<PathBuf, AxError> {
    let here = std::env::current_exe().map_err(|err| {
        AxError::failure(
            AxCode::PathNotFound,
            "find this executable",
            err.to_string(),
        )
        .with_recovery("run just bench-startup from a shell that can see its own binary")
    })?;
    let Some(parent) = here.parent() else {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "find the build profile directory",
            here.display().to_string(),
        )
        .with_recovery("run just bench-startup, which builds the binary it measures first"));
    };
    let binary = parent.join(executable_name());
    let there = binary.try_exists().map_err(|err| {
        AxError::failure(
            AxCode::PathNotFound,
            "find the shipped binary",
            format!("{}: {err}", binary.display()),
        )
        .with_recovery("run just bench-startup, which builds the binary it measures first")
    })?;
    if !there {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "find the shipped binary",
            binary.display().to_string(),
        )
        .with_recovery("run just bench-startup, which builds the binary it measures first"));
    }
    Ok(binary)
}
