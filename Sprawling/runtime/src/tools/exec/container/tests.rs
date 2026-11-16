// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
    limits.image =
        kernel::ContainerImage::parse(&std::env::var("SPRAWLING_CONTAINER_TEST_IMAGE").unwrap())
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
    assert_eq!(
        first["Config"]["Entrypoint"],
        match engine {
            ContainerEngine::Docker => json!(["/bin/true"]),
            ContainerEngine::Podman => json!("/bin/true"),
        }
    );
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
        #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
