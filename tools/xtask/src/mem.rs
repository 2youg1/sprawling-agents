// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How heavy a served city is, in this platform's own counters
//! (tools/xtask/Spec.lean §8-30).
//!
//! **Three counters, because they answer three questions.** Private is
//! what the process holds that nobody shares, which is what one more
//! session costs a machine. Peak private is the highest that figure
//! reached, which is what a startup fold's spike cost. The working set
//! includes shared image pages, and the system trims it to almost
//! nothing while the process does nothing, so it is printed beside the
//! other two and never instead of them.
//!
//! **What is measured is named, never defaulted.** Without a pid this
//! raises a city of its own and serves it: a measurement that defaults
//! to the process running it measures the tool, and that is how the
//! register once recorded xtask's own working set as a city's.
//!
//! The report states no verdict. The budget and each scenario's reading
//! live in `tools/xtask/budgets.toml`, one row per scenario.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::XtaskError;

/// How long a fixture city sits idle, once it accepts, before it is
/// read: past the startup fold, so private is the steady figure while
/// peak private still carries the spike.
const SETTLE: Duration = Duration::from_secs(2);

/// One attempt to reach the fixture city, and the pause between two.
const POLL: Duration = Duration::from_millis(5);

/// How many attempts before a fixture city that never accepts is
/// reported: about 300 s, because a city with 400,000 records folds its
/// whole Ledger before it accepts, which is tens of seconds on a slow
/// disk, and a bound this instrument hits is a reading lost.
const ATTEMPTS: u32 = 30_000;

/// What a run of this command measures. There is no value for "this
/// process", so measuring the instrument itself by default has no
/// spelling.
enum Target {
    Pid(u32),
    Fixture(Fixture),
}

/// The city a fixture serves.
enum Fixture {
    /// Raised fresh in a temporary directory and removed afterwards.
    EmptyCity,
    /// A city somebody already has, served where it lies.
    City(PathBuf),
}

/// One counter's reading, with the platform's name for it.
struct Counter {
    counter: &'static str,
    bytes: u64,
}

/// The three counters, read at one moment from one process.
struct Reading {
    private: Counter,
    peak_private: Counter,
    working_set: Counter,
}

/// Measures what `args` names: a pid, a city, or a fresh empty city.
pub(crate) fn run(root: &Path, args: &[String]) -> Result<String, XtaskError> {
    match target(args)? {
        Target::Pid(pid) => Ok(render(&format!("process {pid}"), pid, &measure(pid)?)),
        Target::Fixture(fixture) => serve_and_measure(root, &fixture),
    }
}

fn target(args: &[String]) -> Result<Target, XtaskError> {
    match args {
        [] => Ok(Target::Fixture(Fixture::EmptyCity)),
        [flag, dir] if flag == "--city" => Ok(Target::Fixture(Fixture::City(PathBuf::from(dir)))),
        [pid] => pid.parse::<u32>().map(Target::Pid).map_err(|_| usage(pid)),
        _ => Err(usage(&args.join(" "))),
    }
}

fn usage(given: &str) -> XtaskError {
    XtaskError::Cmd {
        cmd: "mem".to_owned(),
        msg: format!("`{given}` names nothing to measure; give a pid, `--city <dir>`, or nothing"),
    }
}

fn render(what: &str, pid: u32, reading: &Reading) -> String {
    let line = |label: &str, counter: &Counter| {
        format!(
            "  {label:<13}{:>14} B   ({})",
            counter.bytes, counter.counter
        )
    };
    format!(
        "resident memory of {what}\n  pid          {pid:>14}\n{}\n{}\n{}",
        line("private", &reading.private),
        line("peak private", &reading.peak_private),
        line("working set", &reading.working_set),
    )
}

/// Serves the fixture city on loopback, waits for it to accept, lets it
/// settle, reads it, and stops it - whichever of those fails.
#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn serve_and_measure(root: &Path, fixture: &Fixture) -> Result<String, XtaskError> {
    let binary = binary_path(root, &ReleaseTarget::Host).ok_or_else(|| XtaskError::Cmd {
        cmd: "mem".to_owned(),
        msg: "no release binary under target/release; run `just mem`, which builds it first"
            .to_owned(),
    })?;
    let scratch = std::env::temp_dir().join(format!("sprawl-mem-{}", std::process::id()));
    let (city, what) = match fixture {
        Fixture::EmptyCity => {
            let city = scratch.join("city");
            raise(&binary, &city)?;
            (city, "a served empty city, idle".to_owned())
        }
        Fixture::City(dir) => (dir.clone(), format!("{} served, idle", dir.display())),
    };
    let port = free_port()?;
    let mut child = Command::new(&binary)
        .arg("serve")
        .arg(&city)
        .arg(format!("127.0.0.1:{port}"))
        .args(["--no-console", "--no-open"])
        .env("SPRAWLING_OPEN", "never")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| io(&binary, source))?;
    let pid = child.id();
    let read = accepting(&mut child, port).and_then(|()| {
        std::thread::sleep(SETTLE);
        measure(pid)
    });
    let stopped = child
        .kill()
        .and_then(|()| child.wait())
        .map_err(|source| io(&binary, source));
    let cleared = match fixture {
        Fixture::EmptyCity => {
            std::fs::remove_dir_all(&scratch).map_err(|source| io(&scratch, source))
        }
        Fixture::City(_) => Ok(()),
    };
    let reading = read?;
    stopped?;
    cleared?;
    Ok(render(&what, pid, &reading))
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn raise(binary: &Path, city: &Path) -> Result<(), XtaskError> {
    let out = Command::new(binary)
        .arg("init")
        .arg(city)
        .output()
        .map_err(|source| io(binary, source))?;
    if out.status.success() {
        return Ok(());
    }
    Err(XtaskError::Cmd {
        cmd: format!("{} init {}", binary.display(), city.display()),
        msg: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
    })
}

/// A loopback port nobody holds right now, borrowed and handed back.
fn free_port() -> Result<u16, XtaskError> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .map_err(|source| io(Path::new("127.0.0.1:0"), source))
}

/// Polls until the served city accepts a connection, refusing one that
/// exits first or never accepts.
fn accepting(child: &mut Child, port: u16) -> Result<(), XtaskError> {
    let at = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    for _ in 0..ATTEMPTS {
        if std::net::TcpStream::connect_timeout(&at, POLL).is_ok() {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|source| io(Path::new("serve"), source))?
        {
            return Err(never_accepted(&format!("it exited first, {status}")));
        }
        std::thread::sleep(POLL);
    }
    Err(never_accepted("it did not accept within about 300 s"))
}

fn never_accepted(why: &str) -> XtaskError {
    XtaskError::Cmd {
        cmd: "sprawling serve".to_owned(),
        msg: format!(
            "the fixture city never accepted a connection: {why}; run the same `serve` by hand to see what it says"
        ),
    }
}

fn io(path: &Path, source: std::io::Error) -> XtaskError {
    XtaskError::Io {
        path: path.display().to_string(),
        source,
    }
}

#[cfg(target_os = "linux")]
fn measure(pid: u32) -> Result<Reading, XtaskError> {
    let read = |file: &str| {
        let path = format!("/proc/{pid}/{file}");
        std::fs::read_to_string(&path).map_err(|source| XtaskError::Io { path, source })
    };
    let rollup = read("smaps_rollup")?;
    let status = read("status")?;
    let private = kib(&rollup, "Private_Clean:")?.saturating_add(kib(&rollup, "Private_Dirty:")?);
    Ok(Reading {
        private: Counter {
            counter: "linux smaps_rollup Private_Clean + Private_Dirty",
            bytes: private,
        },
        peak_private: Counter {
            counter: "linux status VmHWM: peak resident, shared pages included",
            bytes: kib(&status, "VmHWM:")?,
        },
        working_set: Counter {
            counter: "linux status VmRSS",
            bytes: kib(&status, "VmRSS:")?,
        },
    })
}

/// One `<name> <n> kB` line of a `/proc` file, in bytes.
#[cfg(target_os = "linux")]
fn kib(text: &str, name: &str) -> Result<u64, XtaskError> {
    text.lines()
        .find_map(|line| line.strip_prefix(name))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.saturating_mul(1024))
        .ok_or_else(|| XtaskError::Doc {
            file: "/proc".to_owned(),
            msg: format!("no readable `{name}` line; the kernel may be too old for smaps_rollup"),
        })
}

#[cfg(target_os = "macos")]
fn measure(pid: u32) -> Result<Reading, XtaskError> {
    let out = capture("ps", &["-o", "rss=", "-p", &pid.to_string()])?;
    let bytes = parse(&out, "ps")?.saturating_mul(1024);
    let counter = "macos ps rss (conservative: shared pages counted in full; \
                   private and peak need a private API)";
    Ok(Reading {
        private: Counter { counter, bytes },
        peak_private: Counter { counter, bytes },
        working_set: Counter { counter, bytes },
    })
}

#[cfg(target_os = "windows")]
fn measure(pid: u32) -> Result<Reading, XtaskError> {
    let script = format!(
        "$p = Get-Process -Id {pid}; \
         \"$($p.PrivateMemorySize64) $($p.PeakPagedMemorySize64) $($p.WorkingSet64)\""
    );
    let out = capture(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
    )?;
    let values = out
        .split_whitespace()
        .map(|value| parse(value, "powershell"))
        .collect::<Result<Vec<u64>, XtaskError>>()?;
    let [private, peak_private, working_set] = values[..] else {
        return Err(XtaskError::Doc {
            file: "powershell".to_owned(),
            msg: format!("three counters were asked for and this came back: `{out}`"),
        });
    };
    Ok(Reading {
        private: Counter {
            counter: "windows PrivateMemorySize64: private commit",
            bytes: private,
        },
        peak_private: Counter {
            counter: "windows PeakPagedMemorySize64: peak commit",
            bytes: peak_private,
        },
        working_set: Counter {
            counter: "windows WorkingSet64: shared image pages included",
            bytes: working_set,
        },
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn measure(_pid: u32) -> Result<Reading, XtaskError> {
    Err(XtaskError::Doc {
        file: "mem".to_owned(),
        msg: "no memory counter is defined for this platform; \
              add one here rather than reporting a number of unknown meaning"
            .to_owned(),
    })
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse(value: &str, program: &str) -> Result<u64, XtaskError> {
    value.trim().parse::<u64>().map_err(|_| XtaskError::Doc {
        file: program.to_owned(),
        msg: format!("unreadable counter: `{value}`"),
    })
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn capture(program: &str, args: &[&str]) -> Result<String, XtaskError> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| io(Path::new(program), source))?;
    if !out.status.success() {
        return Err(XtaskError::Doc {
            file: program.to_owned(),
            msg: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The three counters answer three questions, so a report that
    /// prints one number cannot say which of them it answered: private
    /// is what an extra session costs, peak private is what a startup
    /// spike cost, and the working set carries shared image pages the
    /// system trims at will.
    #[test]
    fn a_reading_names_private_peak_private_and_the_working_set_apart() {
        let pid = std::process::id().to_string();
        let text = run(Path::new("."), &[pid]).unwrap();
        for counter in ["private", "peak private", "working set"] {
            assert!(
                text.lines()
                    .any(|line| line.trim_start().starts_with(counter)),
                "the report has a `{counter}` line: {text}"
            );
        }
    }

    /// A peak is never below the value it is the peak of; a reader that
    /// swapped two counters would say so here first.
    #[test]
    fn peak_private_is_never_below_private() {
        let reading = measure(std::process::id()).unwrap();
        assert!(reading.private.bytes > 0, "a live process holds something");
        assert!(reading.peak_private.bytes >= reading.private.bytes);
    }
}
