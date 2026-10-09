// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn await_condition(mut condition: impl FnMut() -> bool) {
    for _ in 0..12_000 {
        if condition() {
            return;
        }
        std::thread::park_timeout(std::time::Duration::from_millis(5));
    }
    panic!("native lifecycle condition exhausted its poll budget");
}

fn gated_command(tool: &ExecTool, scratch: &std::path::Path) -> (Value, PathBuf, u32) {
    let result = invoke_python(tool,
        "import os,pathlib,time; pathlib.Path('ready').write_text(str(os.getpid()))
while not pathlib.Path('continue').exists(): time.sleep(0.01)
pathlib.Path('late').write_text('copy'); print('later-out'); print('later-err',file=__import__('sys').stderr); raise SystemExit(7)", &[]);
    assert_eq!(result["outcome"], "backgrounded", "{result}");
    let copies: Vec<_> = std::fs::read_dir(scratch)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(copies.len(), 1);
    let copy = copies[0].clone();
    await_condition(|| {
        std::fs::read_to_string(copy.join("ready")).is_ok_and(|text| text.parse::<u32>().is_ok())
    });
    let pid = std::fs::read_to_string(copy.join("ready"))
        .unwrap()
        .parse()
        .unwrap();
    (result["handle"].clone(), copy, pid)
}

#[test]
fn native_macos_background_keeps_its_copy_and_reports_one_terminal_result() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let (handle, copy, _) = gated_command(&tool, scratch.path());
    let owner = tool.setup.run;
    let stranger = kernel::RunId::from_bytes([2; 16]);
    assert!(backlog.harvest(stranger).unwrap().is_empty());
    assert!(backlog.harvest(owner).unwrap().is_empty());
    assert!(copy.is_dir());
    std::fs::write(copy.join("continue"), "go").unwrap();
    let mut finished = None;
    await_condition(|| {
        assert!(backlog.harvest(stranger).unwrap().is_empty());
        let result = invoke_python(&tool, "pass", &[]);
        finished = result["background"].as_array().and_then(|members| {
            members
                .iter()
                .find(|member| member["handle"] == handle)
                .cloned()
        });
        finished.is_some()
    });
    let finished = finished.unwrap();
    assert_eq!(
        (
            finished["exit_code"].as_i64(),
            finished["stdout"].as_str(),
            finished["stderr"].as_str()
        ),
        (
            Some(7),
            Some(
                "later-out
"
            ),
            Some(
                "later-err
"
            )
        ),
        "{finished}"
    );
    assert!(
        backlog
            .harvest(owner)
            .unwrap()
            .iter()
            .all(|member| member.id.to_string() != handle.as_str().unwrap())
    );
    assert!(!source.path().join("late").exists());
    drop(tool);
    await_condition(|| {
        backlog.harvest(stranger).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    });
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn native_macos_halt_reaps_the_owned_primary_before_tool_release() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let (handle, copy, pid) = gated_command(&tool, scratch.path());
    let domain = Address::parse("work").unwrap();
    assert_eq!(backlog.halt(Some(&domain)).unwrap(), 1);
    let mut finished = Vec::new();
    await_condition(|| {
        finished.extend(backlog.harvest(tool.setup.run).unwrap());
        backlog.standing(&domain).unwrap().is_empty()
    });
    assert_eq!(
        finished
            .into_iter()
            .map(|member| (member.id.to_string(), member.exit))
            .collect::<Vec<_>>(),
        vec![(
            handle.as_str().unwrap().to_owned(),
            crate::backlog::Exit::Signalled
        )]
    );
    assert!(backlog.harvest(tool.setup.run).unwrap().is_empty());
    assert_primary_reaped(pid);
    assert!(
        copy.is_dir(),
        "halt must not depend on tool release or copy removal"
    );
    drop(tool);
    assert!(!copy.exists());
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn native_macos_tool_release_reaps_the_owned_primary_and_copy() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let (_, copy, pid) = gated_command(&tool, scratch.path());
    drop(tool);
    await_condition(|| {
        assert!(
            backlog
                .harvest(kernel::RunId::from_bytes([2; 16]))
                .unwrap()
                .is_empty()
        );
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    });
    assert_primary_reaped(pid);
    assert!(!copy.exists());
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn assert_primary_reaped(pid: u32) {
    assert!(
        !Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .output()
            .unwrap()
            .status
            .success(),
        "owned primary still exists"
    );
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn native_macos_release_leaves_another_owners_background_command_alive() {
    let source = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let other_scratch = tempfile::tempdir().unwrap();
    let backlog = Backlog::with_window(crate::backlog::PollBudget::new(1, 1));
    let tool = native_tool(source.path(), scratch.path(), backlog.clone());
    let mut other_setup = setup(source.path(), None, Some(PathBuf::from("/bin/sh")));
    other_setup.run = kernel::RunId::from_bytes([2; 16]);
    let other = ExecTool::new(other_setup, Box::new(EchoSandbox::new()), backlog.clone())
        .unwrap()
        .confined(Confined::with_arm(
            Confinement::MacosSeatbelt {
                wrapper: PathBuf::from("/usr/bin/sandbox-exec"),
            },
            Some(other_scratch.path().to_path_buf()),
        ));
    gated_command(&tool, scratch.path());
    let (_, other_copy, other_pid) = gated_command(&other, other_scratch.path());
    drop(tool);
    await_condition(|| {
        backlog.harvest(kernel::RunId::from_bytes([3; 16])).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .len()
            == 1
    });
    assert!(
        Command::new("/bin/kill")
            .args(["-0", &other_pid.to_string()])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(other_copy.is_dir());
    drop(other);
    await_condition(|| {
        backlog.harvest(kernel::RunId::from_bytes([3; 16])).unwrap();
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    });
    assert!(!other_copy.exists());
}
