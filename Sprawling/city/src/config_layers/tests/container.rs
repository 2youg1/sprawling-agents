// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_container_declaration_reaches_the_frozen_sandbox_whole() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let image = format!("sha256:{}", "a".repeat(64));
    write(
        &path(dir.path(), &room, Layer::Building).unwrap(),
        &format!(
            "[sandbox]
shell = false
fuel = 4000
[sandbox.container]
image = '{image}'
user = 1000
cpu_millis = 1250
memory_bytes = 67108864
pids = 64
"
        ),
    );
    let frozen = load(dir.path(), &room);
    assert!(
        frozen.is_ok(),
        "an explicit container must reach the frozen exec configuration: {frozen:?}"
    );
    assert_eq!(
        frozen.unwrap().sandbox,
        SandboxLimits {
            fuel: 4000,
            container: Some(kernel::ContainerLimits {
                image: kernel::ContainerImage::parse(&image).unwrap(),
                user: std::num::NonZeroU32::new(1000).unwrap(),
                cpu_millis: std::num::NonZeroU32::new(1250).unwrap(),
                memory_bytes: std::num::NonZeroU64::new(67_108_864).unwrap(),
                pids: std::num::NonZeroU32::new(64).unwrap(),
            }),
            ..SandboxLimits::default()
        }
    );
}
