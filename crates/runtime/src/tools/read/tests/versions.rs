// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The version `read` prints is the one a model copies back: `edit`
/// takes it as `base_version`, its conflict names the file's version in
/// the same form, and `plan finish` takes it as evidence (kernel
/// `spec/Locator.lean` D27).
#[test]
fn the_version_read_prints_is_the_one_edit_and_plan_finish_take() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work").join("a.txt"), "one\n").unwrap();
    let (reader, _catalog) = tool(dir.path());
    let read_version = |path: &str| -> String {
        let outcome = reader.invoke(&call(path)).unwrap();
        let version = outcome.result.as_map().get("version").cloned();
        assert!(
            matches!(version, Some(Value::String(_))),
            "read answered no version for {path}"
        );
        version.and_then(|v| v.as_str().map(str::to_owned)).unwrap()
    };
    let seen = read_version("work/a.txt");
    kernel::Locator::parse(&seen).unwrap();

    let work = kernel::Address::parse("work").unwrap();
    let editor = crate::tools::EditTool::new(
        dir.path(),
        work.clone(),
        kernel::WriteDomain::new(vec![work]).unwrap(),
        crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
    )
    .unwrap();
    let edit = |base: &str| {
        let mut args = Map::new();
        for (key, value) in [
            ("path", "work/a.txt"),
            ("base_version", base),
            ("old", "one"),
            ("new", "two"),
        ] {
            args.insert(key.to_owned(), Value::String(value.to_owned()));
        }
        editor.invoke(&ToolCall {
            id: "call-2".to_owned(),
            name: ToolName::parse("edit").unwrap(),
            args: Payload::new(args).unwrap(),
        })
    };
    let landed = edit(&seen).unwrap();
    assert_eq!(
        landed.result.as_map()["new_version"].as_str().unwrap(),
        read_version("work/a.txt")
    );
    let conflict = edit(&seen).unwrap_err();
    assert_eq!(
        conflict.subject(),
        format!(
            "work/a.txt is at {}, not {seen}",
            read_version("work/a.txt")
        )
    );
}
