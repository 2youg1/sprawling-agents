// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[cfg(unix)]
#[test]
#[ignore = "private subprocess fixture selected only by the guardian regression"]
fn guardian_process() {
    let record = std::env::var("SPRAWLING_GUARD_RECORD").unwrap();
    crate::tools::run_container_guard(&["__container-guardian".into(), record])
        .unwrap()
        .unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "private subprocess fixture selected only by the guardian regression"]
fn guardian_parent_process() {
    let program = PathBuf::from(std::env::var("SPRAWLING_GUARD_DAEMON").unwrap());
    let harness = PathBuf::from(std::env::var("SPRAWLING_GUARD_HARNESS").unwrap());
    let work = PathBuf::from(std::env::var("SPRAWLING_GUARD_WORK").unwrap());
    let runtime = ContainerRuntime::probe(ContainerEngine::Docker, program)
        .unwrap()
        .guarded(harness);
    let tool = ExecTool::new(
        setup(&work, None, None),
        Box::new(EchoSandbox::new()),
        patient(),
    )
    .unwrap()
    .with_container(limits(), runtime);
    tool.invoke(&call(line_of("/bin/true", NO_ARGS))).unwrap();
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn guardian_eof_waits_for_late_create_and_reaps_its_client() {
    use std::process::{Command, Stdio};

    for lost_response in [false, true] {
        let daemon = Daemon::open();
        if lost_response {
            std::fs::write(daemon.dir.path().join("fail-create"), "").unwrap();
        }

        let work = tempfile::tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let harness = daemon.guardian();
        std::fs::write(daemon.dir.path().join("create-barrier"), "").unwrap();
        assert!(
            Command::new("mkfifo")
                .arg(daemon.dir.path().join("create-fifo"))
                .status()
                .unwrap()
                .success()
        );
        let exited = daemon.dir.path().join("guardian-exited");
        let mut parent = Command::new(&executable)
            .args([
                "--exact",
                "tools::exec::tests::container::guardian_parent_process",
                "--ignored",
                "--nocapture",
            ])
            .env("SPRAWLING_GUARD_DAEMON", &daemon.program)
            .env("SPRAWLING_GUARD_HARNESS", &harness)
            .env("SPRAWLING_GUARD_WORK", work.path())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        wait_for_fixture(&daemon.dir.path().join("create-entered"));
        let creator = std::fs::read_to_string(daemon.dir.path().join("create-parent"))
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();
        let parent_id = parent.id();
        parent.kill().unwrap();
        parent.wait().unwrap();
        if creator == parent_id {
            // The parent-owned client can outlive a guardian that accepted absence.
            wait_for_fixture(&exited);
        }
        let absent = Command::new(&daemon.program)
            .args(["container", "ls", "--all", "--format", "{{.Names}}"])
            .output()
            .unwrap();
        assert!(absent.status.success() && absent.stdout.is_empty());
        std::fs::write(daemon.dir.path().join("create-fifo"), "release\n").unwrap();
        wait_for_fixture(&daemon.dir.path().join("create-completed"));
        wait_for_fixture(&exited);
        let copy = PathBuf::from(std::fs::read_to_string(daemon.dir.path().join("copy")).unwrap());
        let released = !daemon.exists() && !copy.exists();
        if daemon.exists() {
            assert!(
                Command::new(&daemon.program)
                    .args(["rm", "--force", "--volumes", "fixture"])
                    .status()
                    .unwrap()
                    .success()
            );
        }
        assert!(
            released,
            "EOF plus an absent inventory cannot release ownership before late create"
        );
        let pid = std::fs::read_to_string(daemon.dir.path().join("create-pid")).unwrap();
        assert!(
            !Command::new("kill")
                .args(["-0", pid.trim()])
                .status()
                .unwrap()
                .success(),
            "guardian must reap the create client before it exits"
        );
    }
}

#[cfg(unix)]
fn wait_for_fixture(path: &std::path::Path) {
    for _ in 0..1000 {
        if path.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("fixture condition did not arrive: {}", path.display());
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn guardian_timeout_retains_the_member_until_harvest() {
    use std::process::Command;
    use std::time::Duration;

    for lost_response in [false, true] {
        let daemon = Daemon::open();
        let work = tempfile::tempdir().unwrap();
        let backlog = patient();
        let runtime = ContainerRuntime::probe(ContainerEngine::Docker, daemon.program.clone())
            .unwrap()
            .guarded(daemon.guardian());
        let tool = ExecTool::new(
            setup(work.path(), None, None),
            Box::new(EchoSandbox::new()),
            backlog.clone(),
        )
        .unwrap()
        .with_container(limits(), runtime);
        let scope = tool.setup.domain.clone();
        let run = tool.setup.run;
        if lost_response {
            std::fs::write(daemon.dir.path().join("fail-create"), "").unwrap();
            std::fs::write(daemon.dir.path().join("fail-remove"), "").unwrap();
        }
        std::fs::write(daemon.dir.path().join("create-barrier"), "").unwrap();
        assert!(
            Command::new("mkfifo")
                .arg(daemon.dir.path().join("create-fifo"))
                .status()
                .unwrap()
                .success()
        );
        std::thread::scope(|threads| {
            let (answer, response) = std::sync::mpsc::channel();
            let invocation = &tool;
            threads.spawn(move || {
                answer
                    .send(
                        invocation
                            .invoke(&call(line_of("/bin/true", NO_ARGS)))
                            .map(|_| ()),
                    )
                    .unwrap();
            });
            wait_for_fixture(&daemon.dir.path().join("create-entered"));
            // The parent budget counts polls; the fixture also allows for OS scheduling.
            let returned = response.recv_timeout(Duration::from_secs(30));
            // Release even a regressed unbounded parent before joining the scoped thread.
            if returned.is_err() {
                std::fs::write(daemon.dir.path().join("create-fifo"), "release\n").unwrap();
            }
            assert_eq!(
                returned.unwrap().unwrap_err().code(),
                &kernel::AxCode::SandboxDenied
            );
            let copy =
                PathBuf::from(std::fs::read_to_string(daemon.dir.path().join("copy")).unwrap());
            assert!(copy.is_dir());
            assert_eq!(backlog.standing(&scope).unwrap().len(), 1);
            assert!(backlog.harvest(run).is_err());
            assert!(!daemon.dir.path().join("guardian-exited").exists());
            let absent = Command::new(&daemon.program)
                .args(["container", "ls", "--all", "--format", "{{.Names}}"])
                .output()
                .unwrap();
            assert!(absent.status.success() && absent.stdout.is_empty());
            std::fs::write(daemon.dir.path().join("create-fifo"), "release\n").unwrap();
            wait_for_fixture(&daemon.dir.path().join("create-completed"));
            if lost_response {
                wait_for_fixture(&daemon.dir.path().join("remove-attempted"));
                std::fs::remove_file(daemon.dir.path().join("owned")).unwrap();
                wait_for_fixture(&daemon.dir.path().join("remove-retried"));
                wait_for_fixture(&daemon.dir.path().join("remove-retried-again"));
                assert!(backlog.harvest(run).is_err());
                assert!(copy.is_dir());
                assert_eq!(backlog.standing(&scope).unwrap().len(), 1);
                assert!(!daemon.dir.path().join("guardian-exited").exists());
                std::fs::write(daemon.dir.path().join("owned"), "").unwrap();
                std::fs::remove_file(daemon.dir.path().join("fail-remove")).unwrap();
            }
            wait_for_fixture(&daemon.dir.path().join("guardian-exited"));
            for _ in 0..1000 {
                if backlog.harvest(run).is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(backlog.standing(&scope).unwrap().is_empty());
            assert!(!daemon.exists() && !copy.exists());
            let pid = std::fs::read_to_string(daemon.dir.path().join("create-pid")).unwrap();
            assert!(
                !Command::new("kill")
                    .args(["-0", pid.trim()])
                    .status()
                    .unwrap()
                    .success(),
                "create client must be reaped before harvest forgets its owner"
            );
        });
    }
}
