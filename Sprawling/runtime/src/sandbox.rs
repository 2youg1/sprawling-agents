// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::num::NonZeroU64;
use std::path::PathBuf;

use kernel::{AxCode, AxError};

use crate::backlog::{Ceiling, Shares};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fuel(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub host: PathBuf,
    pub guest: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemory {
    Wasm32,

    Ceiling(NonZeroU64),
}

impl GuestMemory {
    #[must_use]
    pub fn of_shares(shares: Shares) -> GuestMemory {
        match shares {
            Shares::CpuAndMemory { limit } => GuestMemory::Ceiling(limit),
            Shares::Unset | Shares::Cpu => GuestMemory::Wasm32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxJob {
    pub wasm: PathBuf,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub stdin: Vec<u8>,
    pub mounts: Vec<Mount>,
    pub fuel: Fuel,
    pub memory: GuestMemory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: SandboxExit,

    pub ceiling: Option<Ceiling>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxExit {
    Success,
    Failure { code: u64 },
    FuelExhausted,
    Trap { message: String },
}

pub trait Sandbox: Send {
    fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>;
}

pub struct AbsentSandbox;

impl Sandbox for AbsentSandbox {
    fn run(&mut self, _job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        Err(AxError::failure(
            AxCode::ToolUnavailable,
            "run in the sandbox",
            "this build carries no execution engine",
        )
        .with_recovery("pass the work as a `command`, or install a build with the `wasm` feature"))
    }
}

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
            ceiling: None,
        })
    }
}

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
            ceiling: None,
        })
    }
}

#[cfg(feature = "wasm")]
mod engine;

#[cfg(feature = "wasm")]
pub use engine::WasmtimeSandbox;

#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob) {
    let first = sandbox.run(job);
    let second = sandbox.run(job);
    match (first, second) {
        (Ok(one), Ok(two)) => {
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
