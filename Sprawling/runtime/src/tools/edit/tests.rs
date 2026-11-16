// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::PolicyCell;
use kernel::{Address, Mode, RunPolicy};

fn tool(root: &Path) -> EditTool {
    tool_sharing(root, PathLocks::default())
}

/// A run's edit tool with a table of its own, holding `locks` - the
/// city's one table - so two of these are two runs in one city.
fn tool_sharing(root: &Path, locks: PathLocks) -> EditTool {
    let work = Address::parse("work").unwrap();
    let domain = kernel::WriteDomain::new(vec![work.clone()]).unwrap();
    std::fs::create_dir_all(root.join("work")).unwrap();
    EditTool::new(root, work, domain, desk(locks)).unwrap()
}

fn desk(locks: PathLocks) -> EditDesk {
    EditDesk {
        policy: PolicyCell::new(RunPolicy::of(Mode::Work)).reader(),
        seen: ReadStamps::default(),
        locks,
    }
}

/// What a whole read of `rel` records: the stamp `read` takes off its
/// handle (`tools::read::tests::looks` drives the read itself).
fn look(tool: &EditTool, root: &Path, rel: &str) {
    let stamp = Stamp::of(&root.join(rel)).unwrap();
    tool.desk
        .seen
        .look(&Address::parse(rel).unwrap(), stamp.clone(), stamp);
}

fn call(path: &str, old: &str, new: &str) -> ToolCall {
    edit_call(serde_json::json!({ "file_path": path, "old_string": old, "new_string": new }))
}

fn edit_call(args: Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::of(&args).unwrap(),
    }
}

mod landing;
mod links;
mod passages;
