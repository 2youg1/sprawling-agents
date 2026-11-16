// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// A build that says it carries an execution engine carries one.
///
/// `AbsentSandbox` refuses with `this build carries no execution
/// engine` and tells the reader to install a build with the `wasm`
/// feature. Until the feature and this selection existed there was
/// no such build: the absent engine was written into the dispatch
/// as a literal, so the sentence named an action nobody could take
/// and `runtime::WasmtimeSandbox` had no caller outside its own
/// tests.
#[cfg(feature = "sandbox")]
#[test]
fn a_build_with_the_engine_feature_carries_one() {
    let mut engine = crate::doctor::host::execution_engine()
        .expect("a build with the feature starts its engine");
    // A module that is not there: whatever this reports, it is the
    // engine reporting it rather than the absence of one.
    let job = runtime::SandboxJob {
        wasm: std::path::PathBuf::from("no-such-module.wasm"),
        argv: Vec::new(),
        env: Vec::new(),
        stdin: Vec::new(),
        mounts: Vec::new(),
        fuel: runtime::Fuel(1_000),
        memory: runtime::GuestMemory::Wasm32,
    };
    let said = format!("{:?}", engine.run(&job));
    assert!(
        !said.contains("this build carries no execution engine"),
        "the feature is on and the run still met the absent engine: {said}"
    );
}

/// Every bench this process hands an engine to shares one compiled guest
/// (`crates/runtime/spec/Sandbox.lean` D101): after one bench has run a
/// guest, a second bench runs it again although the file now holds bytes
/// that are not WebAssembly under the same length and modification time.
#[cfg(feature = "sandbox")]
#[test]
fn every_bench_shares_one_compiled_guest() {
    use std::io::Write;
    // `(module (func (export "_start")))`, assembled by hand.
    const EMPTY_START: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, b'_', b's', b't', b'a', b'r', b't', 0x00, 0x00,
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ];
    let tmp = tempfile::tempdir().unwrap();
    let wasm = tmp.path().join("start.wasm");
    std::fs::write(&wasm, EMPTY_START).unwrap();
    let job = runtime::SandboxJob {
        wasm: wasm.clone(),
        argv: vec!["prog".to_owned()],
        env: Vec::new(),
        stdin: Vec::new(),
        mounts: Vec::new(),
        fuel: runtime::Fuel(1_000),
        memory: runtime::GuestMemory::Wasm32,
    };
    let mut first = crate::doctor::host::execution_engine().unwrap();
    let mut second = crate::doctor::host::execution_engine().unwrap();
    assert_eq!(first.run(&job).unwrap().exit, runtime::SandboxExit::Success);

    let mut file = std::fs::File::options().write(true).open(&wasm).unwrap();
    let before = file.metadata().unwrap();
    file.write_all(&[0_u8; EMPTY_START.len()]).unwrap();
    file.set_modified(before.modified().unwrap()).unwrap();
    drop(file);

    let again = second.run(&job).map(|outcome| outcome.exit);
    assert_eq!(
        again,
        Ok(runtime::SandboxExit::Success),
        "the second bench read and compiled the guest again"
    );
}

/// A building that asks for PowerShell 7 is handed a `pwsh` only when
/// its version line reads 7 or later (`crates/runtime/Spec.lean`
/// §8-13-2 D30): a pwsh 6, or one that would not say, is refused at the
/// call instead.
#[test]
fn pwsh_is_handed_to_a_run_only_at_major_seven_or_later() {
    use crate::doctor::host::seven_or_later;
    use crate::doctor::{Presence, Version};
    let at = std::path::PathBuf::from("pwsh");
    let said = |text: &str| Presence::Present {
        at: at.clone(),
        version: Version::Said(text.to_owned()),
    };
    assert_eq!(seven_or_later(&said("PowerShell 7.4.6")), Some(at.clone()));
    assert_eq!(seven_or_later(&said("PowerShell 10.0.0")), Some(at.clone()));
    assert_eq!(seven_or_later(&said("PowerShell 6.2.7")), None);
    assert_eq!(
        seven_or_later(&Presence::Present {
            at: at.clone(),
            version: Version::Silent,
        }),
        None
    );
}
