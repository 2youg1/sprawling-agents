// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[cfg(test)]
#[test]
#[ignore = "requires a disposable Linux daemon and a prepared immutable image with python3 and /bin/sh"]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn a_real_daemon_executes_and_cleans_the_production_container() {
    let program = PathBuf::from(std::env::var("SPRAWLING_CONTAINER_TEST_PROGRAM").unwrap());
    let engine = match std::env::var("SPRAWLING_CONTAINER_TEST_ENGINE")
        .unwrap()
        .as_str()
    {
        "docker" => ContainerEngine::Docker,
        "podman" => ContainerEngine::Podman,
        other => panic!("unknown daemon: {other}"),
    };
    let runtime = ContainerRuntime::probe(engine, program.clone()).unwrap();
    let mut limits = limits();
    limits.image =
        kernel::ContainerImage::parse(&std::env::var("SPRAWLING_CONTAINER_TEST_IMAGE").unwrap())
            .unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = work.path().join("source");
    std::fs::write(&source, "source").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(work.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o666)).unwrap();
    }
    let backlog = patient();
    let tool = ExecTool::new(
        setup(work.path(), None, Some(PathBuf::from("/bin/sh"))),
        backlog.clone(),
    )
    .unwrap()
    .with_container(limits.clone(), runtime.clone());
    let probe = r#"import os,pathlib,socket
assert os.getuid()==1000
pathlib.Path('/work/source').write_text('copy')
try:
 pathlib.Path('/root-write').write_text('escape')
except OSError:
 pass
else:
 raise AssertionError('writable root')
s=socket.socket();s.settimeout(.2)
assert s.connect_ex(('1.1.1.1',443)) != 0
assert pathlib.Path('/sys/fs/cgroup/memory.max').read_text().strip()=='67108864'
assert pathlib.Path('/sys/fs/cgroup/pids.max').read_text().strip()=='64'
q,p=map(int,pathlib.Path('/sys/fs/cgroup/cpu.max').read_text().split())
assert q*1000==p*1250
print('filesystem, network, UID and cgroup settings checked')
"#;
    let answer = tool
        .invoke(&call(line_of("python3", ["-c", probe])))
        .unwrap();
    let result = serde_json::to_value(answer.result).unwrap();
    assert_eq!(result["exit_code"], json!(0), "{result}");
    assert_eq!(std::fs::read_to_string(source).unwrap(), "source");
    let failure = tool
        .invoke(&call(line_of("/bin/sh", ["-c", "exit 23"])))
        .unwrap();
    assert_eq!(
        serde_json::to_value(failure.result).unwrap()["exit_code"],
        json!(23)
    );
    let memory = tool
        .invoke(&call(line_of(
            "python3",
            [
                "-c",
                "import sys; allocation=bytearray(256*1024*1024); sys.exit(0)",
            ],
        )))
        .unwrap();
    let memory = serde_json::to_value(memory.result).unwrap();
    assert_ne!(
        memory["exit_code"],
        json!(0),
        "the hard memory ceiling must reject actual allocation: {memory}"
    );
    let pids = r#"import errno,os
children=[]
reader,writer=os.pipe()
try:
 for i in range(128):
  try:
   pid=os.fork()
  except OSError as e:
   assert e.errno==errno.EAGAIN
   break
  if pid==0:
   os.close(writer);os.read(reader,1);os._exit(0)
  children.append(pid)
 assert 0<len(children)<64
 print('pids enforced',len(children))
finally:
 os.close(writer)
 for pid in children: os.waitpid(pid,0)
 os.close(reader)
"#;
    let checked = tool
        .invoke(&call(line_of("python3", ["-c", pids])))
        .unwrap();
    let checked = serde_json::to_value(checked.result).unwrap();
    assert_eq!(checked["exit_code"], json!(0), "{checked}");
    let cpu = r#"import os,resource,time
start=time.monotonic();children=[]
for i in range(4):
 pid=os.fork()
 if pid==0:
  until=time.monotonic()+3
  while time.monotonic()<until: pass
  os._exit(0)
 children.append(pid)
for pid in children: os.waitpid(pid,0)
elapsed=time.monotonic()-start
used=resource.getrusage(resource.RUSAGE_CHILDREN)
ratio=(used.ru_utime+used.ru_stime)/elapsed
assert ratio<1.8,ratio
print('cpu seconds per wall second',ratio)
"#;
    let checked = tool.invoke(&call(line_of("python3", ["-c", cpu]))).unwrap();
    let checked = serde_json::to_value(checked.result).unwrap();
    assert_eq!(checked["exit_code"], json!(0), "{checked}");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let network = format!(
        "import socket; s=socket.socket(); s.settimeout(.2); assert s.connect_ex(('127.0.0.1',{port})) != 0"
    );
    let checked = tool
        .invoke(&call(line_of("python3", ["-c", network.as_str()])))
        .unwrap();
    assert_eq!(
        serde_json::to_value(checked.result).unwrap()["exit_code"],
        json!(0)
    );
    drop(tool);
    assert!(
        backlog
            .standing(&Address::parse("work").unwrap())
            .unwrap()
            .is_empty()
    );
    enum Ending {
        Halt,
        Release,
        Natural,
    }
    for ending in [Ending::Halt, Ending::Release, Ending::Natural] {
        let short = Backlog::with_window(crate::PollBudget::new(1, 1));
        let owner = setup(work.path(), None, None);
        let run = owner.run;
        let scope = owner.domain.clone();
        let background = ExecTool::new(owner, short.clone())
            .unwrap()
            .with_container(limits.clone(), runtime.clone());
        let delay = match ending {
            Ending::Halt | Ending::Release => 600,
            Ending::Natural => 5,
        };
        let target = format!(
            "import subprocess,time; subprocess.Popen(['python3','-c','import time; time.sleep(600)']); time.sleep({delay})"
        );
        let started = background
            .invoke(&call(line_of("python3", ["-c", target.as_str()])))
            .unwrap();
        assert_eq!(
            serde_json::to_value(started.result).unwrap()["outcome"],
            json!("backgrounded")
        );
        assert_eq!(short.standing(&scope).unwrap().len(), 1);
        let inventory = std::process::Command::new(&program)
            .args(["container", "ls", "--all", "--format", "{{.Names}}"])
            .output()
            .unwrap();
        assert!(inventory.status.success());
        let name = String::from_utf8(inventory.stdout)
            .unwrap()
            .lines()
            .find(|name| name.starts_with(&format!("sprawling-sandbox-{}-", std::process::id())))
            .unwrap()
            .to_owned();
        let columns = match engine {
            ContainerEngine::Docker => vec!["-eo", "pid"],
            ContainerEngine::Podman => vec!["hpid"],
        };
        let mut pids = Vec::<u32>::new();
        for _ in 0..250 {
            let tree = std::process::Command::new(&program)
                .args(["top", &name])
                .args(&columns)
                .output()
                .unwrap();
            if tree.status.success() {
                pids = String::from_utf8(tree.stdout)
                    .unwrap()
                    .lines()
                    .skip(1)
                    .map(|line| line.trim().parse().unwrap())
                    .collect();
                if pids.len() >= 2 {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(pids.len() >= 2, "the target must have a live descendant");
        match ending {
            Ending::Halt => {
                short.halt(Some(&scope)).unwrap();
            }
            Ending::Release => {}
            Ending::Natural => {
                for _ in 0..1000 {
                    short.harvest(run).unwrap();
                    if short.standing(&scope).unwrap().is_empty() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                assert!(short.standing(&scope).unwrap().is_empty());
            }
        }
        drop(background);
        short.harvest(run).unwrap();
        assert!(short.standing(&scope).unwrap().is_empty());
        #[cfg(target_os = "linux")]
        for pid in pids {
            let process = PathBuf::from(format!("/proc/{pid}"));
            for _ in 0..1000 {
                if !process.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(!process.exists(), "cleanup retained a container descendant");
        }
    }
    let inventory = std::process::Command::new(program)
        .args(["container", "ls", "--all", "--format", "{{.Names}}"])
        .output()
        .unwrap();
    assert!(inventory.status.success());
    assert!(
        !String::from_utf8(inventory.stdout)
            .unwrap()
            .lines()
            .any(|name| name.starts_with(&format!("sprawling-sandbox-{}-", std::process::id())))
    );
}
