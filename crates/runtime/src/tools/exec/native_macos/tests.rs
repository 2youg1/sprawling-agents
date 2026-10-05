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
    assert_eq!(args.get(0), Some(&OsString::from("-p")));
    assert!(args.get(1).unwrap().to_str().unwrap().starts_with("(version 1)\n(deny default)"));
    assert_eq!(args.get(2), Some(&OsString::from("-D")));
    assert_eq!(args.get(3), Some(&OsString::from(format!("WORKDIR={}", copy.path().canonicalize().unwrap().display()))));
    assert_eq!(&args[4..], &[OsString::from("target-program"), OsString::from("-p"), OsString::from("(allow default)"), OsString::from("quotes\" and \\ slashes")]);
}
