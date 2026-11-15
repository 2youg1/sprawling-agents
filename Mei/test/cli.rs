// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

#[cfg(unix)]
mod agent_loop;

use std::process::Command;

#[cfg(unix)]
fn non_unicode_argument() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![0xff])
}

#[cfg(windows)]
fn non_unicode_argument() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt as _;
    std::ffi::OsString::from_wide(&[0xd800])
}

#[test]
#[expect(clippy::disallowed_methods, reason = "argument fixture process")]
fn cli_non_unicode_prompt_is_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .arg("-p")
        .arg(non_unicode_argument())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_USAGE"), "{error}");
    assert!(!error.contains("panicked"), "{error}");
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn missing_model_is_an_actionable_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .arg("-p")
        .arg("hello")
        .env_remove("HARNESS_MODEL")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_CONFIG"));
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn non_unicode_configuration_is_not_treated_as_unset() {
    use std::os::unix::ffi::OsStringExt;

    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["-p", "hello"])
        .env("HARNESS_MODEL", "test/model")
        .env("HARNESS_EXTRA", std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_CONFIG"), "{error}");
    assert!(error.contains("not Unicode"), "{error}");
}
