// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Container daemon admission and direct create argv
//! (`crates/runtime/spec/Tools/Exec/Container.lean`). A successful admission
//! is a daemon capability statement; it is not an isolation measurement.
//! The caller must inspect the image and created container before start,
//! and hand daemon cancellation to the existing backlog with the CLI child.

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

/// The CLI grammar and info schema to use for one daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerEngine {
    Docker,
    Podman,
}

/// One daemon whose reported controllers passed admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRuntime {
    program: PathBuf,
    engine: ContainerEngine,
    guardian: Option<PathBuf>,
}

/// The values that must remain paired for one container create.
pub struct ContainerLaunch<'a> {
    pub name: &'a str,
    pub copy: &'a Path,
    pub command: &'a Command,
}

/// The sole translation from the host copy to the container filesystem.
const WORKDIR: &str = "/work";
const MILLICPU_PER_CPU: u32 = 1000;

impl ContainerRuntime {
    /// Binds the harness whose private protocol retains cleanup after parent termination.
    #[must_use]
    pub fn guarded(mut self, harness: PathBuf) -> Self {
        self.guardian = Some(harness);
        self
    }

    /// Contacts the daemon under the bounded control-request budget.
    ///
    /// # Errors
    /// Refuses unavailable, unsupported, or unreadable daemon capabilities.
    pub fn probe(engine: ContainerEngine, program: PathBuf) -> Result<Self, AxError> {
        let bytes = control::checked(&mut Self::info_command(engine, &program))?;
        Self::admit(engine, program, &bytes)
    }

    fn command(&self) -> Command {
        Command::new(&self.program)
    }

    /// Builds the daemon question for the host's bounded probe runner.
    /// Unlike `--version`, `info` contacts the server and tests access.
    #[must_use]
    pub fn info_command(engine: ContainerEngine, program: &Path) -> Command {
        let mut command = Command::new(program);
        command.arg("info").arg("--format");
        match engine {
            ContainerEngine::Docker => command.arg("{{json .}}"),
            ContainerEngine::Podman => command.arg("json"),
        };
        command
    }

    /// Admits reported Linux cgroup v2 CPU, memory and process limits.
    /// The host must obtain these bytes from a successful bounded info call.
    ///
    /// # Errors
    /// `E_SANDBOX_DENIED` for missing, unknown or unsupported evidence.
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

    /// Builds a stopped container. No shell interprets the target argv.
    /// The source path must name the already synced copy, never the source tree.
    ///
    /// # Errors
    /// `E_SANDBOX_DENIED` when mount or environment translation is ambiguous.
    /// No process starts and no image is pulled by this method.
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
        let mut command = Command::new(&self.program);
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
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn docker_info() -> Value {
        json!({"OSType":"linux", "CgroupVersion":"2", "MemoryLimit":true,
            "CpuCfsQuota":true, "PidsLimit":true})
    }

    fn admit(engine: ContainerEngine, info: &Value) -> Result<ContainerRuntime, AxError> {
        ContainerRuntime::admit(
            engine,
            PathBuf::from("container-cli"),
            &serde_json::to_vec(info).unwrap(),
        )
    }

    fn limits() -> ContainerLimits {
        serde_json::from_value(json!({"image": format!("sha256:{}", "a".repeat(64)),
            "user":1000, "cpu_millis":1250, "memory_bytes":67108864, "pids":64}))
        .unwrap()
    }

    #[test]
    fn a_cli_without_complete_daemon_evidence_is_refused() {
        for engine in [ContainerEngine::Docker, ContainerEngine::Podman] {
            let err = admit(engine, &json!({"version":"99"})).unwrap_err();
            assert_eq!(err.code(), &AxCode::SandboxDenied);
            assert!(!err.recovery().is_empty());
        }
        for field in [
            "OSType",
            "CgroupVersion",
            "MemoryLimit",
            "CpuCfsQuota",
            "PidsLimit",
        ] {
            let mut missing = docker_info();
            missing.as_object_mut().unwrap().remove(field);
            assert!(admit(ContainerEngine::Docker, &missing).is_err(), "{field}");
        }
        let mut unsupported = docker_info();
        unsupported["OSType"] = json!("windows");
        assert!(admit(ContainerEngine::Docker, &unsupported).is_err());
        unsupported["OSType"] = json!("linux");
        unsupported["MemoryLimit"] = json!(false);
        assert!(admit(ContainerEngine::Docker, &unsupported).is_err());
        assert!(admit(ContainerEngine::Docker, &docker_info()).is_ok());
    }

    #[test]
    fn rootless_podman_needs_each_delegated_controller() {
        for controllers in [
            vec!["cpu", "memory"],
            vec!["memory", "pids"],
            vec!["cpu", "pids"],
            vec![],
        ] {
            assert!(
                admit(
                    ContainerEngine::Podman,
                    &json!({"host":{"os":"linux",
                "cgroupVersion":"v2", "cgroupControllers":controllers}})
                )
                .is_err()
            );
        }
        assert!(
            admit(
                ContainerEngine::Podman,
                &json!({"host":{"os":"linux",
            "cgroupVersion":"v2", "cgroupControllers":["cpu","memory","pids"]}})
            )
            .is_ok()
        );
    }

    #[test]
    fn container_entrypoint_keeps_exactly_the_requested_program() {
        let dir = tempfile::tempdir().unwrap();
        for engine in [ContainerEngine::Docker, ContainerEngine::Podman] {
            let info = match engine {
                ContainerEngine::Docker => docker_info(),
                ContainerEngine::Podman => json!({"host":{"os":"linux",
                    "cgroupVersion":"v2", "cgroupControllers":["cpu","memory","pids"]}}),
            };
            let runtime = admit(engine, &info).unwrap();
            for program in [
                "[\"/bin/sh\",\"-c\"]",
                "[]",
                "null",
                "/bin/with\"quote",
                "/bin/true",
            ] {
                let target = Command::new(program);
                let command = runtime
                    .create_command(
                        &limits(),
                        &ContainerLaunch {
                            name: "owned-entrypoint",
                            copy: dir.path(),
                            command: &target,
                        },
                    )
                    .unwrap();
                let entrypoint = command
                    .get_args()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .find(|pair| pair[0] == "--entrypoint")
                    .unwrap()[1]
                    .to_str()
                    .unwrap()
                    .to_owned();
                let decoded = match engine {
                    ContainerEngine::Docker => vec![entrypoint],
                    ContainerEngine::Podman => serde_json::from_str::<Vec<String>>(&entrypoint)
                        .unwrap_or_else(|_| vec![entrypoint]),
                };
                assert_eq!(decoded, vec![program.to_owned()], "{engine:?}: {program}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn podman_non_utf8_program_refuses_before_create() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let runtime = admit(
            ContainerEngine::Podman,
            &json!({"host":{"os":"linux", "cgroupVersion":"v2",
                "cgroupControllers":["cpu","memory","pids"]}}),
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut program = b"/bin/program-".to_vec();
        program.push(0xff);
        let target = Command::new(OsString::from_vec(program));
        let err = runtime
            .create_command(
                &limits(),
                &ContainerLaunch {
                    name: "owned-non-utf8",
                    copy: dir.path(),
                    command: &target,
                },
            )
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::SandboxDenied);
        assert!(!err.recovery().is_empty());
    }

    #[test]
    fn ambiguous_mount_and_removed_environment_are_refused_before_create() {
        let runtime = admit(ContainerEngine::Docker, &docker_info()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut target = Command::new("sh");
        for path in [dir.path().join("copy,readonly"), PathBuf::from("relative")] {
            assert!(
                runtime
                    .create_command(
                        &limits(),
                        &ContainerLaunch {
                            name: "owned-copy",
                            copy: &path,
                            command: &target,
                        }
                    )
                    .is_err()
            );
        }
        target.env_remove("LANG");
        assert!(
            runtime
                .create_command(
                    &limits(),
                    &ContainerLaunch {
                        name: "owned-copy",
                        copy: dir.path(),
                        command: &target,
                    }
                )
                .is_err()
        );
    }

    #[test]
    #[ignore = "requires a Linux Docker/Podman daemon and an explicitly prepared local image ID"]
    fn a_real_daemon_retains_the_requested_limits_on_a_stopped_container() {
        let engine = std::env::var("SPRAWLING_CONTAINER_TEST_ENGINE").unwrap();
        let engine = match engine.as_str() {
            "docker" => ContainerEngine::Docker,
            "podman" => ContainerEngine::Podman,
            other => panic!("unsupported test engine: {other}"),
        };
        let program = PathBuf::from(std::env::var("SPRAWLING_CONTAINER_TEST_PROGRAM").unwrap());
        let info = ContainerRuntime::info_command(engine, &program)
            .output()
            .unwrap();
        assert!(info.status.success(), "daemon info failed");
        let runtime = ContainerRuntime::admit(engine, program.clone(), &info.stdout).unwrap();
        let mut limits = limits();
        limits.image = kernel::ContainerImage::parse(
            &std::env::var("SPRAWLING_CONTAINER_TEST_IMAGE").unwrap(),
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let name = format!(
            "sprawling-create-{}-{}",
            std::process::id(),
            dir.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('.', "")
        );
        let target = Command::new("/bin/true");
        let mut create = runtime
            .create_command(
                &limits,
                &ContainerLaunch {
                    name: &name,
                    copy: dir.path(),
                    command: &target,
                },
            )
            .unwrap();
        let observed = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let created = create.output()?;
            if !created.status.success() {
                return Err("the daemon refused create".into());
            }
            let inspected = Command::new(&program).args(["inspect", &name]).output()?;
            if !inspected.status.success() {
                return Err("the daemon refused inspect".into());
            }
            Ok(serde_json::from_slice(&inspected.stdout)?)
        })();
        // Remove by the owned name before checking any daemon response.
        let cleanup = Command::new(&program)
            .args(["rm", "--force", "--volumes", &name])
            .output()
            .unwrap();
        assert!(
            cleanup.status.success(),
            "the created container could not be removed"
        );
        let observed = observed.unwrap();
        let first = observed.as_array().unwrap().first().unwrap();
        match engine {
            ContainerEngine::Docker => assert_eq!(
                first["HostConfig"]["NanoCpus"],
                json!(
                    u64::from(limits.cpu_millis.get())
                        .checked_mul(1_000_000)
                        .unwrap()
                ),
            ),
            ContainerEngine::Podman => {
                let quota = first["HostConfig"]["CpuQuota"].as_u64().unwrap();
                let period = first["HostConfig"]["CpuPeriod"].as_u64().unwrap();
                assert!(period > 0);
                assert_eq!(
                    quota.checked_mul(u64::from(MILLICPU_PER_CPU)).unwrap(),
                    period
                        .checked_mul(u64::from(limits.cpu_millis.get()))
                        .unwrap()
                );
            }
        }
        assert_eq!(
            first["Mounts"].as_array().unwrap().len(),
            1,
            "the prepared image must not declare anonymous volumes"
        );
        assert_eq!(first["State"]["Running"], json!(false));
        assert_eq!(first["Config"]["User"], json!(limits.user.to_string()));
        assert_eq!(first["HostConfig"]["NetworkMode"], json!("none"));
        assert_eq!(
            first["HostConfig"]["Memory"],
            json!(limits.memory_bytes.get())
        );
        assert_eq!(
            first["HostConfig"]["MemorySwap"],
            json!(limits.memory_bytes.get())
        );
        assert_eq!(first["HostConfig"]["PidsLimit"], json!(limits.pids.get()));
        assert_eq!(first["Config"]["Entrypoint"], json!(["/bin/true"]));
        assert_eq!(first["Config"]["WorkingDir"], json!(WORKDIR));
    }

    proptest! {
        #[test]
        fn every_missing_docker_control_refuses(memory in any::<bool>(), cpu in any::<bool>(), pids in any::<bool>()) {
            let mut info = docker_info();
            info["MemoryLimit"] = json!(memory);
            info["CpuCfsQuota"] = json!(cpu);
            info["PidsLimit"] = json!(pids);
            prop_assert_eq!(admit(ContainerEngine::Docker, &info).is_ok(), memory && cpu && pids);
        }
        #[test]
        fn target_arguments_cannot_become_container_options(args in prop::collection::vec("[^\x00]{0,64}", 0..12)) {
            let runtime = admit(ContainerEngine::Docker, &docker_info()).unwrap();
            let dir = tempfile::tempdir().unwrap();
            let limits = limits();
            let mut target = Command::new("/bin/printf");
            target.args(&args).env("LANG", "C");
            let command = runtime.create_command(&limits, &ContainerLaunch {
                name:"owned-copy", copy:dir.path(), command:&target,
            }).unwrap();
            let actual: Vec<_> = command.get_args().map(|arg| arg.to_str().unwrap()).collect();
            let image = actual.iter().position(|arg| *arg == limits.image.as_str()).unwrap();
            prop_assert_eq!(actual.get(image.saturating_add(1)..).unwrap(), args.iter().map(String::as_str).collect::<Vec<_>>());
            prop_assert_eq!(command.get_program(), "container-cli");
            prop_assert!(actual.windows(2).any(|pair| pair == ["--pull", "never"]));
            prop_assert!(actual.windows(2).any(|pair| pair == ["--entrypoint", "/bin/printf"]));
            prop_assert!(actual.windows(2).any(|pair| pair == ["--cpus", "1.250"]));
        }
    }
}
