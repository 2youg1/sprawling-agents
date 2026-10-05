// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::process::Command;

use super::wrap;

#[test]
fn native_macos_path_and_target_arguments_cannot_become_profile_source() {
    let copy = tempfile::tempdir().unwrap();
    let mut target = Command::new("target-program");
    target.args(["-p", "(allow default)", "quotes\" and \\ slashes"]);
    let wrapper = std::path::Path::new("sandbox-exec");
    let actual = wrap(wrapper, copy.path(), &target).unwrap();
    let args: Vec<_> = actual.get_args().map(OsString::from).collect();
    assert_eq!(actual.get_program(), wrapper);
    assert_eq!(args.first(), Some(&OsString::from("-p")));
    assert!(
        args.get(1)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("(version 1)\n(deny default)")
    );
    assert_eq!(args.get(2), Some(&OsString::from("-D")));
    assert_eq!(
        args.get(3),
        Some(&OsString::from(format!(
            "WORKDIR={}",
            copy.path().canonicalize().unwrap().display()
        )))
    );
    assert_eq!(
        &args[4..],
        &[
            OsString::from("target-program"),
            OsString::from("-p"),
            OsString::from("(allow default)"),
            OsString::from("quotes\" and \\ slashes")
        ]
    );
}

#[test]
fn native_macos_bad_copy_or_empty_program_refuses_before_command_construction() {
    let copy = tempfile::tempdir().unwrap();
    let wrapper = std::path::Path::new("sandbox-exec");
    for (path, program) in [
        (copy.path().join("missing"), "target"),
        (copy.path().to_path_buf(), ""),
    ] {
        let error = wrap(wrapper, &path, &Command::new(program)).unwrap_err();
        assert_eq!(*error.code(), kernel::AxCode::SandboxDenied);
    }
    assert_eq!(
        *super::probe(&copy.path().join("missing-wrapper"), copy.path())
            .unwrap_err()
            .code(),
        kernel::AxCode::SandboxDenied
    );
}

proptest::proptest! {
    #[test]
    fn native_macos_every_argument_sequence_preserves_target_argv(arguments in proptest::collection::vec("[^\x00]{0,100}", 0..20)) {
        let copy = tempfile::tempdir().unwrap();
        let mut target = Command::new("target");
        target.args(&arguments).env_clear().env("LANG", "C");
        let wrapped = wrap(std::path::Path::new("sandbox-exec"), copy.path(), &target).unwrap();
        let actual: Vec<_> = wrapped.get_args().skip(5).map(OsString::from).collect();
        let expected: Vec<_> = arguments.into_iter().map(OsString::from).collect();
        proptest::prop_assert_eq!(actual, expected);
        proptest::prop_assert_eq!(wrapped.get_envs().collect::<Vec<_>>(), target.get_envs().collect::<Vec<_>>());
    }
}
