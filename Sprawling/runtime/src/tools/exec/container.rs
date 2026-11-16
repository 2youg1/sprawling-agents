// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::{AxCode, AxError, ContainerLimits};
use serde_json::Value;

mod cleanup;
mod guardian;
pub use guardian::run_container_guard;
mod control;
mod inspection;
mod lifetime;
pub(crate) use lifetime::ContainerLease;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContainerEngine {
    Docker,
    Podman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRuntime {
    program: PathBuf,
    engine: ContainerEngine,
    guardian: Option<PathBuf>,
}

pub struct ContainerLaunch<'a> {
    pub name: &'a str,
    pub copy: &'a Path,
    pub command: &'a Command,
}

const WORKDIR: &str = "/work";
const MILLICPU_PER_CPU: u32 = 1000;

impl ContainerRuntime {
    #[must_use]
    pub fn guarded(mut self, harness: PathBuf) -> Self {
        self.guardian = Some(harness);
        self
    }

    pub fn probe(engine: ContainerEngine, program: PathBuf) -> Result<Self, AxError> {
        let bytes = control::checked(&mut Self::info_command(engine, &program))?;
        Self::admit(engine, program, &bytes)
    }

    fn command(&self) -> Command {
        child::command(&self.program)
    }

    #[must_use]
    pub fn info_command(engine: ContainerEngine, program: &Path) -> Command {
        let mut command = child::command(program);
        command.arg("info").arg("--format");
        match engine {
            ContainerEngine::Docker => command.arg("{{json .}}"),
            ContainerEngine::Podman => command.arg("json"),
        };
        command
    }

    pub fn admit(engine: ContainerEngine, program: PathBuf, info: &[u8]) -> Result<Self, AxError> {
        let info: Value = serde_json::from_slice(info)
            .map_err(|err| denied("read container daemon capabilities", err.to_string()))?;
        let supported = match engine {
            ContainerEngine::Docker => {
                info.get("OSType").and_then(Value::as_str) == Some("linux")
                    && info.get("CgroupVersion").and_then(Value::as_str) == Some("2")
                    && info.get("MemoryLimit").and_then(Value::as_bool) == Some(true)
                    && info.get("CpuCfsQuota").and_then(Value::as_bool) == Some(true)
                    && info.get("PidsLimit").and_then(Value::as_bool) == Some(true)
            }
            ContainerEngine::Podman => {
                let host = info.get("host");
                host.and_then(|host| host.get("os")).and_then(Value::as_str) == Some("linux")
                    && host
                        .and_then(|host| host.get("cgroupVersion"))
                        .and_then(Value::as_str)
                        == Some("v2")
                    && host
                        .and_then(|host| host.get("cgroupControllers"))
                        .and_then(Value::as_array)
                        .is_some_and(|controllers| {
                            ["cpu", "memory", "pids"].iter().all(|needed| {
                                controllers
                                    .iter()
                                    .any(|controller| controller.as_str() == Some(needed))
                            })
                        })
            }
        };
        if !supported {
            return Err(denied(
                "admit the container daemon",
                "the daemon does not report Linux cgroup v2 with CPU, memory and process controllers",
            ));
        }
        Ok(Self {
            program,
            engine,
            guardian: None,
        })
    }

    pub fn create_command(
        &self,
        limits: &ContainerLimits,
        launch: &ContainerLaunch<'_>,
    ) -> Result<Command, AxError> {
        if launch.name.is_empty()
            || !launch
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(denied(
                "name the container",
                "use an owned alphanumeric container name",
            ));
        }
        let copy = launch
            .copy
            .to_str()
            .filter(|copy| {
                !copy.is_empty()
                    && !copy.contains([',', '\n', '\r', '\0'])
                    && launch.copy.is_absolute()
            })
            .ok_or_else(|| {
                denied(
                    "mount the sandbox copy",
                    "the copy path cannot be represented as one absolute bind mount",
                )
            })?;
        if launch.command.get_program().is_empty() {
            return Err(denied(
                "start the container program",
                "the program is empty",
            ));
        }
        let cpu = limits
            .cpu_millis
            .get()
            .checked_div(MILLICPU_PER_CPU)
            .zip(limits.cpu_millis.get().checked_rem(MILLICPU_PER_CPU))
            .ok_or_else(|| {
                denied(
                    "translate the container CPU limit",
                    "millicpu conversion failed",
                )
            })?;
        let mut command = child::command(&self.program);
        command
            .args(["create", "--pull", "never", "--name"])
            .arg(launch.name)
            .args(["--network", "none", "--read-only", "--cap-drop", "ALL"])
            .args(["--security-opt", "no-new-privileges", "--user"])
            .arg(limits.user.to_string())
            .arg("--cpus")
            .arg(format!("{}.{:03}", cpu.0, cpu.1))
            .arg("--memory")
            .arg(limits.memory_bytes.to_string())
            .arg("--memory-swap")
            .arg(limits.memory_bytes.to_string())
            .arg("--pids-limit")
            .arg(limits.pids.to_string())
            .arg("--mount")
            .arg(format!("type=bind,src={copy},dst={WORKDIR}"))
            .arg("--workdir")
            .arg(WORKDIR)
            .arg("--entrypoint");
        match self.engine {
            ContainerEngine::Docker => command.arg(launch.command.get_program()),
            ContainerEngine::Podman => {
                let program = launch.command.get_program().to_str().ok_or_else(|| {
                    denied(
                        "encode the Podman entrypoint",
                        "a non-UTF-8 program cannot be represented without changing its identity",
                    )
                })?;
                command.arg(
                    serde_json::to_string(&[program]).map_err(|source| {
                        denied("encode the Podman entrypoint", source.to_string())
                    })?,
                )
            }
        };
        for (name, value) in launch.command.get_envs() {
            let value = value.ok_or_else(|| {
                denied(
                    "pass the container environment",
                    "an explicitly removed variable cannot be translated",
                )
            })?;
            let (Some(name), Some(value)) = (name.to_str(), value.to_str()) else {
                return Err(denied(
                    "pass the container environment",
                    "a non-UTF-8 variable cannot be translated",
                ));
            };
            if name.is_empty() || name.contains(['=', '\0']) || value.contains('\0') {
                return Err(denied(
                    "pass the container environment",
                    "an environment assignment has an invalid name or value",
                ));
            }
            command.arg("--env").arg(format!("{name}={value}"));
        }
        command
            .arg(limits.image.as_str())
            .args(launch.command.get_args());
        Ok(command)
    }
}

fn denied(action: &str, subject: impl std::fmt::Display) -> AxError {
    AxError::failure(AxCode::SandboxDenied, action, subject.to_string()).with_recovery(
        "make the Linux container daemon accessible with cgroup v2 CPU, memory and process controllers, or explicitly choose an existing sandbox arm",
    )
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
