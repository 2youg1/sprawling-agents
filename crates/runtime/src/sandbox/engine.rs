// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wasmtime-backed execution boundary: the only adapter that runs a
//! real guest, the one compiled guest every clone of it shares, the
//! ceiling a guest's memory grows under, and the rules by which its
//! ending becomes a [`SandboxExit`](super::SandboxExit)
//! (`crates/runtime/spec/Sandbox.lean` §8-13, D101, D102).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use super::{Fuel, GuestMemory, Sandbox, SandboxExit, SandboxJob, SandboxOutcome};
use crate::backlog::Ceiling;
use kernel::{AxCode, AxError};

/// The real boundary: wasmtime running wasip1 guests.
///
/// A handle: every clone shares one engine and one compiled guest, so a
/// process that hands a clone to each bench compiles its guest once
/// (D101). Compiling holds the slot, so two benches that ask at once
/// compile once between them rather than twice side by side.
#[derive(Clone)]
pub struct WasmtimeSandbox {
    shared: Arc<Shared>,
}

struct Shared {
    engine: wasmtime::Engine,
    compiled: Mutex<Option<Compiled>>,
}

/// The guest last compiled, and the file it was compiled from.
struct Compiled {
    identity: GuestIdentity,
    module: wasmtime::Module,
}

/// What a guest file is recognised by without reading it. A reinstall
/// that replaces the component in place changes the length or the
/// modification time, so it is compiled afresh; a replacement that keeps
/// both is not noticed until the process restarts (D101).
#[derive(Debug, PartialEq, Eq)]
struct GuestIdentity {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

/// Every refusal the host side of the sandbox raises, with the one
/// sentence that tells its reader what to do next.
fn host_error(op: &'static str, detail: String, recovery: &'static str) -> AxError {
    AxError::failure(AxCode::SandboxDenied, op, detail).with_recovery(recovery)
}

/// The refusal for a guest file that cannot be read, whether its
/// metadata or its bytes.
fn unreadable(wasm: &Path, err: &std::io::Error) -> AxError {
    host_error(
        "read guest module",
        format!("{}: {err}", wasm.display()),
        "run `sprawling doctor`: the Python arm's guest module is missing \
         from this installation or cannot be read",
    )
}

impl WasmtimeSandbox {
    pub fn new() -> Result<WasmtimeSandbox, AxError> {
        let mut config = wasmtime::Config::new();
        // Fuel is the only instruction budget: metering must be on before
        // any guest is compiled, or the store's fuel is never consumed.
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|err| {
            host_error(
                "configure sandbox engine",
                err.to_string(),
                "reinstall sprawling: this build's execution engine refuses \
                     the fuel metering every guest here is run under",
            )
        })?;
        Ok(WasmtimeSandbox {
            shared: Arc::new(Shared {
                engine,
                compiled: Mutex::new(None),
            }),
        })
    }

    /// The compiled guest at `wasm`: the shared one when the file is the
    /// one it was compiled from, otherwise compiled now and kept in its
    /// place.
    fn module_for(&self, wasm: &Path) -> Result<wasmtime::Module, AxError> {
        let identity = GuestIdentity::of(wasm)?;
        let mut slot = self
            .shared
            .compiled
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(held) = slot.as_ref().filter(|held| held.identity == identity) {
            return Ok(held.module.clone());
        }
        let module = compile(&self.shared.engine, wasm)?;
        *slot = Some(Compiled {
            identity,
            module: module.clone(),
        });
        Ok(module)
    }
}

impl GuestIdentity {
    fn of(wasm: &Path) -> Result<GuestIdentity, AxError> {
        let metadata = std::fs::metadata(wasm).map_err(|err| unreadable(wasm, &err))?;
        Ok(GuestIdentity {
            path: wasm.to_path_buf(),
            len: metadata.len(),
            modified: metadata.modified().map_err(|err| unreadable(wasm, &err))?,
        })
    }
}

/// Reads and compiles a guest: the step D101 keeps to once per process.
fn compile(engine: &wasmtime::Engine, wasm: &Path) -> Result<wasmtime::Module, AxError> {
    let module_bytes = std::fs::read(wasm).map_err(|err| unreadable(wasm, &err))?;
    wasmtime::Module::new(engine, &module_bytes).map_err(|err| {
        host_error(
            "compile guest module",
            err.to_string(),
            "reinstall sprawling: the guest module shipped with this build is \
             not the WebAssembly this engine reads",
        )
    })
}

struct HostState {
    wasi: wasmtime_wasi::p1::WasiP1Ctx,
    limiter: Limiter,
}

/// The store's limiter (D102): wasmtime's own `StoreLimits` decides every
/// growth, and this layer only remembers whether it refused one for
/// passing the run's ceiling. `StoreLimits` also refuses a growth past
/// the maximum the module itself declares, and that is not the run's
/// ceiling.
struct Limiter {
    limits: wasmtime::StoreLimits,
    ceiling: GuestMemory,
    hit: bool,
}

impl Limiter {
    fn new(ceiling: GuestMemory) -> Limiter {
        let limits = match ceiling {
            GuestMemory::Wasm32 => wasmtime::StoreLimitsBuilder::new(),
            // A ceiling past this platform's address space bounds nothing
            // the address space does not already bound.
            GuestMemory::Ceiling(limit) => wasmtime::StoreLimitsBuilder::new()
                .memory_size(usize::try_from(limit.get()).unwrap_or(usize::MAX)),
        }
        .build();
        Limiter {
            limits,
            ceiling,
            hit: false,
        }
    }

    /// The run's ceiling, when a growth was refused at it.
    fn verdict(&self) -> Option<Ceiling> {
        match (self.ceiling, self.hit) {
            (GuestMemory::Ceiling(limit), true) => Some(Ceiling::Hit { limit }),
            (GuestMemory::Ceiling(_), false) | (GuestMemory::Wasm32, _) => None,
        }
    }

    /// Whether growing to `desired` bytes was refused for the run's
    /// ceiling alone: past it, and within the maximum the module itself
    /// declares, which would refuse the growth with no ceiling at all.
    fn past_ceiling(&self, desired: usize, maximum: Option<usize>) -> bool {
        let within_module = maximum.is_none_or(|maximum| desired <= maximum);
        match self.ceiling {
            GuestMemory::Ceiling(limit) => {
                within_module
                    && u64::try_from(desired).map_or(true, |desired| desired > limit.get())
            }
            GuestMemory::Wasm32 => false,
        }
    }
}

impl wasmtime::ResourceLimiter for Limiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let granted = self.limits.memory_growing(current, desired, maximum)?;
        self.hit |= !granted && self.past_ceiling(desired, maximum);
        Ok(granted)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.limits.table_growing(current, desired, maximum)
    }

    fn instances(&self) -> usize {
        self.limits.instances()
    }

    fn tables(&self) -> usize {
        self.limits.tables()
    }

    fn memories(&self) -> usize {
        self.limits.memories()
    }
}

impl Sandbox for WasmtimeSandbox {
    fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError> {
        let module = self.module_for(&job.wasm)?;

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
            limiter: Limiter::new(job.memory),
        };
        let mut store = wasmtime::Store::new(&self.shared.engine, state);
        store.limiter(|state| &mut state.limiter);
        let Fuel(fuel) = job.fuel;
        store.set_fuel(fuel).map_err(|err| {
            host_error(
                "set fuel",
                err.to_string(),
                "report this against runtime::sandbox: this engine was built \
                     with fuel metering on and refused a fuel budget anyway",
            )
        })?;

        let mut linker: wasmtime::Linker<HostState> = wasmtime::Linker::new(&self.shared.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut HostState| &mut state.wasi)
            .map_err(|err| {
                host_error(
                    "link wasi",
                    err.to_string(),
                    "report this against runtime::sandbox: the wasip1 host functions \
                 are linked once into an empty linker and cannot collide",
                )
            })?;

        let instance = match linker.instantiate(&mut store, &module) {
            Ok(instance) => instance,
            // The guest's initial memory alone passes the run's ceiling:
            // a fact about this guest under this ceiling, told as the
            // ceiling it hit rather than as a broken build (D102).
            Err(err) if store.data().limiter.verdict().is_some() => {
                return Ok(SandboxOutcome {
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    exit: SandboxExit::Trap {
                        message: err.to_string(),
                    },
                    ceiling: store.data().limiter.verdict(),
                });
            }
            Err(err) => {
                return Err(host_error(
                    "instantiate guest",
                    err.to_string(),
                    "reinstall sprawling: the guest module asks this host for an \
                     import it does not provide, so the two are different builds",
                ));
            }
        };
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
        let ceiling = store.data().limiter.verdict();
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
            ceiling,
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
