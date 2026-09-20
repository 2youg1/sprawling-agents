// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wasmtime-backed execution boundary: the only adapter that runs a
//! real guest, and the rules by which its ending becomes a
//! [`SandboxExit`](super::SandboxExit).

use super::{Fuel, Sandbox, SandboxExit, SandboxJob, SandboxOutcome};
use kernel::{AxCode, AxError};

/// The real boundary: wasmtime running wasip1 guests.
pub struct WasmtimeSandbox {
    engine: wasmtime::Engine,
}

/// Every refusal the host side of the sandbox raises, with the one
/// sentence that tells its reader what to do next.
fn host_error(op: &'static str, detail: String, recovery: &'static str) -> AxError {
    AxError::failure(AxCode::SandboxDenied, op, detail).with_recovery(recovery)
}

impl WasmtimeSandbox {
    pub fn new() -> Result<WasmtimeSandbox, AxError> {
        let mut config = wasmtime::Config::new();
        // Fuel is the only budget: metering must be on before any
        // guest is compiled, or the store's fuel is never consumed.
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|err| {
            host_error(
                "configure sandbox engine",
                err.to_string(),
                "reinstall sprawling: this build's execution engine refuses \
                     the fuel metering every guest here is run under",
            )
        })?;
        Ok(WasmtimeSandbox { engine })
    }
}

struct HostState {
    wasi: wasmtime_wasi::p1::WasiP1Ctx,
}

impl Sandbox for WasmtimeSandbox {
    fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        let module_bytes = std::fs::read(&job.wasm).map_err(|err| {
            host_error(
                "read guest module",
                format!("{}: {err}", job.wasm.display()),
                "run `sprawling doctor`: the Python arm's guest module is missing \
                 from this installation or cannot be read",
            )
        })?;
        let module = wasmtime::Module::new(&self.engine, &module_bytes).map_err(|err| {
            host_error(
                "compile guest module",
                err.to_string(),
                "reinstall sprawling: the guest module shipped with this build is \
                 not the WebAssembly this engine reads",
            )
        })?;

        let stdout = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(1 << 20);
        let stderr = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(1 << 20);
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        builder
            .stdin(wasmtime_wasi::p2::pipe::MemoryInputPipe::new(
                job.stdin.clone(),
            ))
            .stdout(stdout.clone())
            .stderr(stderr.clone())
            .args(&job.argv);
        for (key, value) in &job.env {
            builder.env(key, value);
        }
        // The capability surface, stated one mount at a time. There
        // is no ambient access to add to it.
        for mount in &job.mounts {
            let perms = if mount.writable {
                wasmtime_wasi::FsPerms::ReadWrite
            } else {
                wasmtime_wasi::FsPerms::ReadOnly
            };
            builder
                .preopened_dir(&mount.host, &mount.guest, perms)
                .map_err(|err| {
                    host_error(
                        "grant mount",
                        format!("{}: {err}", mount.host.display()),
                        "check that the directory exists and this process may read \
                         it; a guest reaches only the directories mounted for it",
                    )
                })?;
        }
        let state = HostState {
            wasi: builder.build_p1(),
        };
        let mut store = wasmtime::Store::new(&self.engine, state);
        let Fuel(fuel) = job.fuel;
        store.set_fuel(fuel).map_err(|err| {
            host_error(
                "set fuel",
                err.to_string(),
                "report this against runtime::sandbox: this engine was built \
                     with fuel metering on and refused a fuel budget anyway",
            )
        })?;

        let mut linker: wasmtime::Linker<HostState> = wasmtime::Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut HostState| &mut state.wasi)
            .map_err(|err| {
                host_error(
                    "link wasi",
                    err.to_string(),
                    "report this against runtime::sandbox: the wasip1 host functions \
                 are linked once into an empty linker and cannot collide",
                )
            })?;

        let instance = linker.instantiate(&mut store, &module).map_err(|err| {
            host_error(
                "instantiate guest",
                err.to_string(),
                "reinstall sprawling: the guest module asks this host for an \
                     import it does not provide, so the two are different builds",
            )
        })?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|err| {
                host_error(
                    "find guest entry",
                    err.to_string(),
                    "reinstall sprawling: a wasip1 guest exports `_start`, and \
                     this module does not",
                )
            })?;

        let result = start.call(&mut store, ());
        let exit = classify(result, &store)?;
        // The pipes are only recoverable once every other handle is
        // gone; the store holds one until here.
        drop(store);
        let stdout = stdout.try_into_inner().ok_or_else(|| {
            host_error(
                "collect guest stdout",
                "the output pipe was still shared after the guest ended".to_owned(),
                "report this against runtime::sandbox: the store holding the \
                 second handle to the output pipe is dropped one line above",
            )
        })?;
        let stderr = stderr.try_into_inner().ok_or_else(|| {
            host_error(
                "collect guest stderr",
                "the error pipe was still shared after the guest ended".to_owned(),
                "report this against runtime::sandbox: the store holding the \
                 second handle to the error pipe is dropped one line above",
            )
        })?;
        Ok(SandboxOutcome {
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
            exit,
        })
    }
}

/// Turns the engine's outcome into ours. Fuel exhaustion and guest
/// exits are guest facts; a host malfunction — an engine that never
/// metered the fuel it was asked to meter — is an `Err`, because
/// reporting "not exhausted" without having measured would be a
/// guess dressed as a fact.
fn classify(
    result: Result<(), wasmtime::Error>,
    store: &wasmtime::Store<HostState>,
) -> Result<SandboxExit, AxError> {
    let Err(err) = result else {
        return Ok(SandboxExit::Success);
    };
    if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
        if exit.0 == 0 {
            return Ok(SandboxExit::Success);
        }
        // WASI pins exit codes to [0, 126); a negative one would mean
        // the guest bypassed proc_exit's own check, which is a broken
        // engine rather than a program that merely failed.
        let code = u64::try_from(exit.0).map_err(|_| {
            host_error(
                "read guest exit code",
                format!("exit code {} is outside the WASI range", exit.0),
                "reinstall sprawling: WASI pins exit codes to 0 through 125, and \
                 this engine returned one outside that range",
            )
        })?;
        return Ok(SandboxExit::Failure { code });
    }
    if let Some(trap) = err.downcast_ref::<wasmtime::Trap>()
        && *trap == wasmtime::Trap::OutOfFuel
    {
        return Ok(SandboxExit::FuelExhausted);
    }
    // Some builds surface exhaustion only through the remaining
    // budget, so the budget is consulted before this is called a
    // trap. An engine that cannot report its fuel was never metering.
    let remaining = store.get_fuel().map_err(|err| {
        host_error(
            "read remaining fuel",
            format!("the engine reported no fuel metering: {err}"),
            "reinstall sprawling: without fuel metering nothing bounds a guest, so \
             this build refuses to run one rather than guess",
        )
    })?;
    if remaining == 0 {
        return Ok(SandboxExit::FuelExhausted);
    }
    Ok(SandboxExit::Trap {
        message: err.to_string(),
    })
}
