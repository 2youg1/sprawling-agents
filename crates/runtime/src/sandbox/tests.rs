// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn job() -> SandboxJob {
    SandboxJob {
        wasm: PathBuf::from("unused.wasm"),
        argv: vec![],
        env: vec![],
        stdin: b"hello".to_vec(),
        mounts: vec![],
        fuel: Fuel(1000),
    }
}

#[test]
fn the_echo_stand_in_passes_stdin_through_and_records_the_job() {
    let mut sandbox = EchoSandbox::new();
    let outcome = sandbox.run(&job()).unwrap();
    assert_eq!(outcome.stdout, b"hello");
    assert_eq!(outcome.exit, SandboxExit::Success);
    assert_eq!(sandbox.seen.len(), 1);
    assert_eq!(sandbox.seen[0].fuel, Fuel(1000));
}

#[test]
fn the_fault_stand_in_delivers_its_script_in_order() {
    let mut sandbox = FaultSandbox::new(vec![
        SandboxExit::FuelExhausted,
        SandboxExit::Trap {
            message: "unreachable".to_owned(),
        },
        SandboxExit::Failure { code: 2 },
    ]);
    assert_eq!(
        sandbox.run(&job()).unwrap().exit,
        SandboxExit::FuelExhausted
    );
    assert!(matches!(
        sandbox.run(&job()).unwrap().exit,
        SandboxExit::Trap { .. }
    ));
    assert_eq!(
        sandbox.run(&job()).unwrap().exit,
        SandboxExit::Failure { code: 2 }
    );
    // Script spent: a stand-in must not invent further failures.
    assert_eq!(sandbox.run(&job()).unwrap().exit, SandboxExit::Success);
}
