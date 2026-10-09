// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One engine process from start to finish: what it wrote to stdout,
//! where its stderr went, how long it may take, and what happens to the
//! helper processes it leaves behind (tools/xtask/Spec.lean §8-44).
//!
//! A Chromium-family engine starts helper processes that inherit its
//! stdout and may outlive it. Reading stdout to its end, as
//! `Command::output` does, waits for every one of them, and on a macOS
//! runner that wait never ended; so the browser's own exit ends the wait
//! for the dump, and what follows its exit is the caller's choice
//! ([`AfterExit`]).

use std::io::Read as _;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::report::XtaskError;

/// How often the engine is asked whether it has exited.
const POLL: Duration = Duration::from_millis(100);

/// How long the dump may keep arriving after the engine has exited,
/// under [`AfterExit::Drain`]. The engine writes the whole dump before
/// it exits, so this only drains a pipe.
const DRAIN: Duration = Duration::from_secs(2);

/// How long a killed tree is given to close the pipe it held.
const AFTER_KILL: Duration = Duration::from_secs(5);

/// How one engine process is run.
pub(crate) struct Launch<'a> {
    /// How a failure names the command.
    pub(crate) cmd: &'a str,
    pub(crate) stderr: Stderr<'a>,
    /// How long the engine may run before its process tree is killed.
    pub(crate) patience: Duration,
    pub(crate) after_exit: AfterExit,
}

/// Where the engine's stderr goes.
pub(crate) enum Stderr<'a> {
    Discarded,
    /// A file created (or truncated) for this launch alone.
    KeptIn(&'a Path),
}

/// What the run waits for once the engine itself has exited.
#[derive(Clone, Copy)]
pub(crate) enum AfterExit {
    /// Read what arrives until the pipe is quiet for [`DRAIN`], and leave
    /// any helper that still holds it running.
    Drain,
    /// Read until every process holding stdout has closed it, so the
    /// next launch starts with no helper of this one alive; past the
    /// bound the tree is killed where the platform can still find it,
    /// and [`Tree::Stray`] says so.
    AwaitTree(Duration),
}

/// What one launch wrote to stdout, and whether its process tree let go.
#[derive(Debug)]
pub(crate) struct Ran {
    pub(crate) stdout: Vec<u8>,
    pub(crate) tree: Tree,
}

/// Whether every process of a launch closed its output before the run
/// returned.
#[derive(Debug)]
pub(crate) enum Tree {
    Released,
    /// A helper still held it; why, in words a report can carry.
    Stray(String),
}

/// Runs `command` and returns what it wrote to stdout.
///
/// # Errors
///
/// `XtaskError::Cmd` naming `launch.cmd` when the engine cannot start,
/// or when it has not exited within `launch.patience`; in the second
/// case its whole process tree has been killed first, and the message
/// says whether the tree let go of the pipe.
pub(crate) fn run(mut command: Command, launch: &Launch) -> Result<Ran, XtaskError> {
    let failed = |msg: String| XtaskError::Cmd {
        cmd: launch.cmd.to_owned(),
        msg,
    };
    let stderr = match launch.stderr {
        Stderr::Discarded => Stdio::null(),
        Stderr::KeptIn(path) => std::fs::File::create(path)
            .map(Stdio::from)
            .map_err(|source| XtaskError::Io {
                path: path.display().to_string(),
                source,
            })?,
    };
    in_own_group(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(stderr)
        .spawn()
        .map_err(|err| failed(err.to_string()))?;
    let arrived = read_in_background(&mut child).map_err(failed)?;
    let mut out = Vec::new();
    if !exits_within(&mut child, launch.patience).map_err(|err| failed(err.to_string()))? {
        let killed = kill_tree(&mut child);
        let released = drained(&arrived, &mut out, AFTER_KILL);
        return Err(failed(format!(
            "the engine did not finish within {} s, so its process tree was killed{}{}; \
             run the same command by hand to see where it stops",
            launch.patience.as_secs(),
            killed
                .err()
                .map_or_else(String::new, |why| format!(" ({why})")),
            if released {
                ""
            } else {
                ", and a process of it still held its output"
            },
        )));
    }
    match launch.after_exit {
        AfterExit::Drain => {
            while let Ok(bytes) = arrived.recv_timeout(DRAIN) {
                out.extend_from_slice(&bytes);
            }
        }
        AfterExit::AwaitTree(bound) => {
            if !drained(&arrived, &mut out, bound) {
                return Ok(Ran {
                    stdout: out,
                    tree: stray(&mut child, &arrived, bound),
                });
            }
        }
    }
    Ok(Ran {
        stdout: out,
        tree: Tree::Released,
    })
}

/// What happened to a tree that still held the output `bound` after its
/// leader exited: killed where its process group still finds it (macOS,
/// Linux), left running where nothing can (Windows, where `taskkill /T`
/// needs the leader alive). Either way the launch itself succeeded, and
/// a caller that gave it a profile of its own is not disturbed by it.
fn stray(child: &mut Child, arrived: &Receiver<Vec<u8>>, bound: Duration) -> Tree {
    let mut late = Vec::new();
    match kill_tree(child) {
        Ok(()) if drained(arrived, &mut late, AFTER_KILL) => Tree::Released,
        Ok(()) => Tree::Stray(format!(
            "a process of it held the output {} s after it exited and outlived the kill",
            bound.as_secs()
        )),
        Err(why) => Tree::Stray(format!(
            "a process of it held the output {} s after it exited and could not be killed: {why}",
            bound.as_secs()
        )),
    }
}

/// Puts the engine at the head of a process group of its own, so its
/// helpers can be killed with it on macOS and Linux.
fn in_own_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _unchanged = command;
    }
}

/// The child's stdout, read chunk by chunk on a thread of its own; the
/// channel closes when every process holding the pipe has closed it.
fn read_in_background(child: &mut Child) -> Result<Receiver<Vec<u8>>, String> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "the engine was started without a stdout to read".to_owned())?;
    let (send, arrived) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 64 * 1024];
        while let Ok(read) = stdout.read(&mut chunk) {
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                break;
            };
            if send.send(bytes.to_vec()).is_err() {
                break;
            }
        }
    });
    Ok(arrived)
}

/// Whether the child exited within `patience`, counted in polls so the
/// wait samples no clock.
fn exits_within(child: &mut Child, patience: Duration) -> std::io::Result<bool> {
    for _ in 0..polls_in(patience) {
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        std::thread::sleep(POLL);
    }
    Ok(child.try_wait()?.is_some())
}

/// Appends what arrives until the pipe closes, and says whether it did
/// within `bound`; only the quiet polls count toward the bound.
fn drained(arrived: &Receiver<Vec<u8>>, out: &mut Vec<u8>, bound: Duration) -> bool {
    let mut quiet: u128 = 0;
    while quiet < polls_in(bound) {
        match arrived.recv_timeout(POLL) {
            Ok(bytes) => out.extend_from_slice(&bytes),
            Err(RecvTimeoutError::Disconnected) => return true,
            Err(RecvTimeoutError::Timeout) => quiet = quiet.saturating_add(1),
        }
    }
    false
}

/// How many [`POLL`]s fit in `span`.
fn polls_in(span: Duration) -> u128 {
    span.as_millis().checked_div(POLL.as_millis()).unwrap_or(0)
}

/// Kills the child and every process it started, then reaps the child.
///
/// Windows walks the tree from the child with `taskkill /T`, which finds
/// the helpers only while the child is alive; macOS and Linux signal the
/// process group [`in_own_group`] made, which outlives its leader.
fn kill_tree(child: &mut Child) -> Result<(), String> {
    let pid = child.id().to_string();
    let mut tree = tree_kill_command(&pid);
    let status = tree
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let leader = child.kill().or_else(|err| match child.try_wait() {
        Ok(Some(_)) => Ok(()),
        Ok(None) | Err(_) => Err(err),
    });
    child
        .wait()
        .map_err(|err| format!("reaping it failed: {err}"))?;
    match (status, leader) {
        (Ok(status), Ok(())) if status.success() => Ok(()),
        (Ok(status), Ok(())) => Err(format!("the tree kill exited with {status}")),
        (Err(err), _) => Err(format!("the tree kill did not start: {err}")),
        (Ok(_), Err(err)) => Err(format!("killing the engine failed: {err}")),
    }
}

#[cfg(windows)]
#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn tree_kill_command(pid: &str) -> Command {
    let mut command = Command::new("taskkill");
    command.args(["/T", "/F", "/PID", pid]);
    command
}

#[cfg(not(windows))]
#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn tree_kill_command(pid: &str) -> Command {
    let mut command = Command::new("kill");
    command.args(["-KILL", "--", &format!("-{pid}")]);
    command
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::zombie_processes,
    clippy::disallowed_methods,
    reason = "test code: fixture processes are killed or exit on their own, and the stall test times itself"
)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// The environment variable that turns this test binary into a
    /// fixture engine, and what that engine does.
    const FIXTURE: &str = "XTASK_LAUNCH_FIXTURE";
    const FIXTURE_TEST: &str = "render::engine::launch::tests::fixture";

    /// Not a test: the body a fixture engine runs when the launch tests
    /// start this binary with [`FIXTURE`] set.
    #[test]
    #[ignore = "a fixture process the launch tests start"]
    fn fixture() {
        match std::env::var(FIXTURE).as_deref() {
            Ok("says") => eprintln!("the engine said this"),
            Ok("stalls") => {
                let _helper = fixture_engine("hangs").spawn().unwrap();
                std::thread::sleep(Duration::from_secs(600));
            }
            Ok("hangs") => std::thread::sleep(Duration::from_secs(600)),
            Ok("leaves") => {
                fixture_engine("late").spawn().unwrap();
            }
            Ok("late") => {
                std::thread::sleep(Duration::from_secs(3));
                println!("written after the engine exited");
            }
            _ => {}
        }
    }

    fn fixture_engine(kind: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([FIXTURE_TEST, "--exact", "--ignored", "--nocapture"])
            .env(FIXTURE, kind);
        command
    }

    fn launch<'a>(stderr: Stderr<'a>, patience: u64, after_exit: AfterExit) -> Launch<'a> {
        Launch {
            cmd: "fixture",
            stderr,
            patience: Duration::from_secs(patience),
            after_exit,
        }
    }

    #[test]
    fn the_engine_stderr_lands_in_the_file_kept_for_it() {
        let file = std::env::temp_dir().join(format!("xtask-launch-{}.log", std::process::id()));
        run(
            fixture_engine("says"),
            &launch(Stderr::KeptIn(&file), 60, AfterExit::Drain),
        )
        .unwrap();
        let said = std::fs::read_to_string(&file).unwrap();
        std::fs::remove_file(&file).unwrap();
        assert!(said.contains("the engine said this"), "{said}");
    }

    #[test]
    fn a_stalled_engine_is_killed_with_its_helpers() {
        let started = Instant::now();
        let err = run(
            fixture_engine("stalls"),
            &launch(Stderr::Discarded, 3, AfterExit::Drain),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("process tree was killed"), "{err}");
        assert!(!err.contains("still held its output"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(30), "{err}");
    }

    #[test]
    fn awaiting_the_tree_reads_what_a_helper_writes_after_the_engine_exits() {
        let out = run(
            fixture_engine("leaves"),
            &launch(
                Stderr::Discarded,
                60,
                AfterExit::AwaitTree(Duration::from_secs(20)),
            ),
        )
        .unwrap();
        assert!(matches!(out.tree, Tree::Released), "{:?}", out.tree);
        let out = String::from_utf8_lossy(&out.stdout);
        assert!(out.contains("written after the engine exited"), "{out}");
    }
}
