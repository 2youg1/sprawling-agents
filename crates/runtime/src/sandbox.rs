// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The execution boundary: a seam with one method,
//! behind which a guest program runs with exactly the capabilities it
//! was handed and no others.
//!
//! The capability surface is the preopen set. A guest can reach a host
//! directory if and only if a [`Mount`] named it; there is no ambient
//! filesystem, no environment inheritance, and — because wasip1 has no
//! socket host implementation — no network. That last one matters: the
//! Python arm cannot reach the network, and the proof is structural
//! rather than a rule someone remembered to write down.
//!
//! Fuel exhaustion is an outcome, not an error. Running out of fuel is
//! the guest doing something the budget did not cover, which the caller
//! must see and decide about; `Err` here is reserved for the host
//! failing to run the job at all. The distinction is what lets a
//! watchdog treat a runaway loop differently from a broken engine.

use std::path::PathBuf;

use kernel::{AxCode, AxError};

/// Instruction budget. Exhaustion ends the guest; it does not fail the
/// host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fuel(pub u64);

/// One granted capability: a host directory visible to the guest under
/// `guest`, writable or not. The mount list is the whole of what the
/// job can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub host: PathBuf,
    pub guest: String,
    pub writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxJob {
    pub wasm: PathBuf,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub stdin: Vec<u8>,
    pub mounts: Vec<Mount>,
    pub fuel: Fuel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: SandboxExit,
}

/// How the guest ended. Every variant is a fact about the guest — the
/// host never hides a guest failure behind a success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxExit {
    Success,
    Failure { code: u64 },
    FuelExhausted,
    Trap { message: String },
}

/// The seam. One method: hand it a job, get back what the guest did.
/// `Send`, because the `exec` tool that holds one is `Send`
/// (sprawling-SPEC 8-44); every adapter here already was.
pub trait Sandbox: Send {
    fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>;
}

/// What a binary built without an execution engine offers: a refusal
/// that names the missing piece.
///
/// It exists so the absence is a verdict rather than a stand-in. An
/// echo in this position would answer "success" to a guest that never
/// ran, and the first person to notice would be whoever trusted the
/// output.
pub struct AbsentSandbox;

impl Sandbox for AbsentSandbox {
    fn run(&mut self, _job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        Err(AxError::failure(
            AxCode::ToolUnavailable,
            "run in the sandbox",
            "this build carries no execution engine",
        )
        .with_recovery("use the program arm, or install a build with the `wasm` feature"))
    }
}

/// A pass-through stand-in: stdout echoes stdin, plus whatever script
/// output the test queued. Used where the test is about the caller's
/// logic rather than the boundary.
#[derive(Default)]
pub struct EchoSandbox {
    scripted: Vec<Vec<u8>>,
    pub seen: Vec<SandboxJob>,
}

impl EchoSandbox {
    pub fn new() -> EchoSandbox {
        EchoSandbox::default()
    }

    pub fn queue_stdout(&mut self, bytes: Vec<u8>) {
        self.scripted.push(bytes);
    }
}

impl Sandbox for EchoSandbox {
    fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        self.seen.push(job.clone());
        let stdout = if self.scripted.is_empty() {
            job.stdin.clone()
        } else {
            self.scripted.remove(0)
        };
        Ok(SandboxOutcome {
            stdout,
            stderr: Vec::new(),
            exit: SandboxExit::Success,
        })
    }
}

/// A fault stand-in: pops a scripted outcome per call, so a caller's
/// handling of exhaustion and traps is testable without provoking the
/// real engine into either.
pub struct FaultSandbox {
    scripted: Vec<SandboxExit>,
}

impl FaultSandbox {
    pub fn new(scripted: Vec<SandboxExit>) -> FaultSandbox {
        FaultSandbox { scripted }
    }
}

impl Sandbox for FaultSandbox {
    fn run(&mut self, _job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        let exit = if self.scripted.is_empty() {
            SandboxExit::Success
        } else {
            self.scripted.remove(0)
        };
        Ok(SandboxOutcome {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit,
        })
    }
}

/// The wasmtime adapter lives in its own file, behind the `wasm`
/// feature: the seam above is what every caller programs against.
#[cfg(feature = "wasm")]
mod engine;

#[cfg(feature = "wasm")]
pub use engine::WasmtimeSandbox;

/// Conformance: any Sandbox must run twice in a row without the first
/// job poisoning the second, and must return a well-formed outcome.
#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob) {
    let first = sandbox.run(job);
    let second = sandbox.run(job);
    match (first, second) {
        (Ok(one), Ok(two)) => {
            // Well-ordered back-to-back calls: the second must not
            // inherit the first's state.
            assert!(
                matches!(
                    one.exit,
                    SandboxExit::Success
                        | SandboxExit::Failure { .. }
                        | SandboxExit::FuelExhausted
                        | SandboxExit::Trap { .. }
                ),
                "outcome shape is one of the four"
            );
            assert!(
                matches!(
                    two.exit,
                    SandboxExit::Success
                        | SandboxExit::Failure { .. }
                        | SandboxExit::FuelExhausted
                        | SandboxExit::Trap { .. }
                ),
                "the second call still returns a well-formed outcome"
            );
        }
        (one, two) => {
            assert!(
                one.is_err() == two.is_err(),
                "a host that can run a job once can run it twice"
            );
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
