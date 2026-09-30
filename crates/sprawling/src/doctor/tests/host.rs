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
    };
    let said = format!("{:?}", engine.run(&job));
    assert!(
        !said.contains("this build carries no execution engine"),
        "the feature is on and the run still met the absent engine: {said}"
    );
}
