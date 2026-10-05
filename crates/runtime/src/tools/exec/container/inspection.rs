// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Daemon policy verification (`crates/runtime/spec/Tools/Exec/Container.lean`).

use kernel::{AxError, ContainerLimits};
use serde_json::{Value, json};

use super::{ContainerEngine, WORKDIR, denied};

pub(super) fn image(bytes: &[u8], limits: &ContainerLimits) -> Result<(), AxError> {
    let value = first(bytes)?;
    let volumes = value.pointer("/Config/Volumes");
    if value.get("Id").and_then(Value::as_str) != Some(limits.image.as_str())
        || !matches!(volumes, Some(Value::Null))
            && !volumes
                .is_some_and(|value| value.as_object().is_some_and(serde_json::Map::is_empty))
    {
        return Err(denied(
            "inspect the container image",
            "the image identity is unconfirmed or it declares writable volumes",
        ));
    }
    Ok(())
}

pub(super) fn stopped(
    bytes: &[u8],
    limits: &ContainerLimits,
    engine: ContainerEngine,
    copy: &std::path::Path,
) -> Result<(), AxError> {
    let value = first(bytes)?;
    let host = value
        .get("HostConfig")
        .ok_or_else(|| denied("inspect container limits", "missing HostConfig"))?;
    let config = value
        .get("Config")
        .ok_or_else(|| denied("inspect container limits", "missing Config"))?;
    let cpu = match engine {
        ContainerEngine::Docker => {
            host.get("NanoCpus").and_then(Value::as_u64)
                == u64::from(limits.cpu_millis.get()).checked_mul(1_000_000)
        }
        ContainerEngine::Podman => host
            .get("CpuQuota")
            .and_then(Value::as_u64)
            .zip(host.get("CpuPeriod").and_then(Value::as_u64))
            .is_some_and(|(quota, period)| {
                period > 0
                    && quota.checked_mul(u64::from(super::MILLICPU_PER_CPU))
                        == period.checked_mul(u64::from(limits.cpu_millis.get()))
            }),
    };
    let mounts = value.get("Mounts").and_then(Value::as_array);
    let mount = mounts.and_then(|mounts| mounts.first());
    let matches_copy = copy.to_str().is_some_and(|copy| {
        mount
            .and_then(|mount| mount.get("Source"))
            .and_then(Value::as_str)
            == Some(copy)
    });
    let caps = host.get("CapDrop").and_then(Value::as_array);
    let security = host.get("SecurityOpt").and_then(Value::as_array);
    let caps_dropped = caps.is_some_and(|caps| {
        caps.iter().any(|cap| {
            cap.as_str()
                .is_some_and(|cap| cap.eq_ignore_ascii_case("ALL"))
        })
    }) || matches!(engine, ContainerEngine::Podman)
        && value
            .get("EffectiveCaps")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
    if !cpu
        || host.get("Memory") != Some(&json!(limits.memory_bytes.get()))
        || host.get("MemorySwap") != Some(&json!(limits.memory_bytes.get()))
        || host.get("PidsLimit") != Some(&json!(limits.pids.get()))
        || host.get("NetworkMode").and_then(Value::as_str) != Some("none")
        || host.get("ReadonlyRootfs").and_then(Value::as_bool) != Some(true)
        || config.get("User").and_then(Value::as_str) != Some(&limits.user.to_string())
        || config.get("WorkingDir").and_then(Value::as_str) != Some(WORKDIR)
        || value.pointer("/State/Running").and_then(Value::as_bool) != Some(false)
        || mounts.is_none_or(|mounts| mounts.len() != 1)
        || !matches_copy
        || mount
            .and_then(|mount| mount.get("Destination"))
            .and_then(Value::as_str)
            != Some(WORKDIR)
        || mount
            .and_then(|mount| mount.get("Type"))
            .and_then(Value::as_str)
            != Some("bind")
        || !caps_dropped
        || !security.is_some_and(|security| {
            security.iter().any(|option| {
                matches!(
                    option.as_str(),
                    Some("no-new-privileges" | "no-new-privileges:true")
                )
            })
        })
    {
        return Err(denied(
            "inspect container limits",
            "the stopped container does not retain every requested boundary",
        ));
    }
    Ok(())
}

pub(super) fn state(bytes: &[u8]) -> Result<(bool, i32), AxError> {
    let value = first(bytes)?;
    let running = value
        .pointer("/State/Running")
        .and_then(Value::as_bool)
        .ok_or_else(|| denied("read the container state", "missing running state"))?;
    let code = value
        .pointer("/State/ExitCode")
        .and_then(Value::as_i64)
        .ok_or_else(|| denied("read the container result", "missing target exit code"))?;
    let code =
        i32::try_from(code).map_err(|err| denied("read the container result", err.to_string()))?;
    Ok((running, code))
}

fn first(bytes: &[u8]) -> Result<Value, AxError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|err| denied("inspect the container", err.to_string()))?;
    value
        .as_array()
        .filter(|values| values.len() == 1)
        .and_then(|values| values.first())
        .cloned()
        .ok_or_else(|| {
            denied(
                "inspect the container",
                "expected exactly one identified object",
            )
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_created_container_with_zero_exit_has_no_target_result() {
        let bytes = br#"[{"State":{"Status":"created","Running":false,"ExitCode":0}}]"#;
        assert!(
            super::state(bytes).is_err(),
            "a target that the daemon never started cannot be a successful exit zero"
        );
    }
}
