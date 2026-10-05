// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Public exec/Backlog daemon regressions (`crates/runtime/spec/Tools/Exec/Container.lean`).

use super::*;
use serde_json::json;

fn limits() -> kernel::ContainerLimits {
    serde_json::from_value(json!({"image":format!("sha256:{}", "a".repeat(64)),
        "user":1000,"cpu_millis":1250,"memory_bytes":67108864,"pids":64}))
    .unwrap()
}

#[cfg(unix)]
struct Daemon {
    dir: tempfile::TempDir,
    program: PathBuf,
}

#[cfg(unix)]
impl Daemon {
    fn open() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("daemon");
        let script = r##"#!/bin/sh
set -eu
cd "$(dirname "$0")"
case "$1" in
info) echo '{"OSType":"linux","CgroupVersion":"2","MemoryLimit":true,"CpuCfsQuota":true,"PidsLimit":true}' ;;
image) printf '[{"Id":"sha256:%s","Config":{"Volumes":null}}]\n' "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
create)
  shift
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --name) printf '%s' "$2" >name; shift ;;
      --mount) printf '%s' "$2" | sed 's/^type=bind,src=//;s/,dst=.*$//' >copy; shift ;;
    esac
    shift
  done
  touch owned
  if [ -f fail-create ]; then exit 42; fi
  echo container-id ;;
container)
  case "$2" in
    inspect)
      running=false
      if [ -f running ]; then running=true; fi
      printf '[{"Config":{"User":"1000","WorkingDir":"/work"},"HostConfig":{"NanoCpus":1250000000,"Memory":67108864,"MemorySwap":67108864,"PidsLimit":64,"NetworkMode":"none","ReadonlyRootfs":true,"CapDrop":["ALL"],"SecurityOpt":["no-new-privileges"]},"Mounts":[{"Source":"%s","Destination":"/work","Type":"bind"}],"State":{"Running":%s,"ExitCode":23}}]\n' "$(cat copy)" "$running" ;;
    ls) if [ -f owned ]; then cat name; fi ;;
  esac ;;
start)
  touch running
  if [ -f background ]; then
    while [ -f running ]; do read -r unused <fifo; done
  fi
  rm -f running
  echo target-output
  exit 91 ;;
rm)
  if [ -f fail-remove ]; then echo refused >&2; exit 43; fi
  rm -f owned running
  echo removed ;;
esac
"##;
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir, program }
    }

    fn tool(&self, work: &std::path::Path, backlog: &Backlog) -> ExecTool {
        ExecTool::new(
            setup(work, None, None),
            Box::new(EchoSandbox::new()),
            backlog.clone(),
        )
        .unwrap()
        .with_container(
            limits(),
            ContainerRuntime::probe(ContainerEngine::Docker, self.program.clone()).unwrap(),
        )
    }

    fn exists(&self) -> bool {
        self.dir.path().join("owned").exists()
    }
}

#[cfg(unix)]
#[test]
fn daemon_target_exit_is_not_the_attach_cli_exit() {
    let daemon = Daemon::open();
    let work = tempfile::tempdir().unwrap();
    let tool = daemon.tool(work.path(), &patient());
    let answer = tool
        .invoke(&call(
            json!({"program":{"path":"/image-only/program","args":[]}}),
        ))
        .unwrap();
    let result = serde_json::to_value(answer.result).unwrap();
    assert_eq!(
        (
            result["exit_code"].clone(),
            result["stdout"].as_str().unwrap().trim()
        ),
        (json!(23), "target-output")
    );
    assert!(!daemon.exists());
    assert!(
        !PathBuf::from(std::fs::read_to_string(daemon.dir.path().join("copy")).unwrap()).exists()
    );
}

#[cfg(unix)]
#[test]
fn a_lost_create_response_still_removes_the_owned_identity() {
    let daemon = Daemon::open();
    std::fs::write(daemon.dir.path().join("fail-create"), "").unwrap();
    let work = tempfile::tempdir().unwrap();
    let backlog = patient();
    let tool = daemon.tool(work.path(), &backlog);
    assert!(
        tool.invoke(&call(json!({"program":{"path":"/bin/true","args":[]}})))
            .is_err()
    );
    assert!(
        !daemon.exists(),
        "a rejected create response cannot abandon a daemon resource"
    );
    assert!(backlog.standing(&tool.setup.domain).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn a_refused_cleanup_keeps_the_identity_and_copy_until_retry() {
    let daemon = Daemon::open();
    std::fs::write(daemon.dir.path().join("fail-create"), "").unwrap();
    std::fs::write(daemon.dir.path().join("fail-remove"), "").unwrap();
    let work = tempfile::tempdir().unwrap();
    let backlog = patient();
    let tool = daemon.tool(work.path(), &backlog);
    assert!(
        tool.invoke(&call(json!({"program":{"path":"/bin/true","args":[]}})))
            .is_err()
    );
    let copy = PathBuf::from(std::fs::read_to_string(daemon.dir.path().join("copy")).unwrap());
    assert!(daemon.exists() && copy.is_dir());
    assert_eq!(backlog.standing(&tool.setup.domain).unwrap().len(), 1);
    assert!(backlog.harvest(tool.setup.run).is_err());
    std::fs::remove_file(daemon.dir.path().join("fail-remove")).unwrap();
    assert!(backlog.harvest(tool.setup.run).unwrap().is_empty());
    assert!(!daemon.exists() && !copy.exists());
}

#[test]
#[ignore = "requires a disposable Linux daemon and a prepared immutable image with python3 and /bin/sh"]
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
        Box::new(EchoSandbox::new()),
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
print('five axes checked')
"#;
    let answer = tool
        .invoke(&call(
            json!({"program":{"path":"python3","args":["-c",probe]}}),
        ))
        .unwrap();
    let result = serde_json::to_value(answer.result).unwrap();
    assert_eq!(result["exit_code"], json!(0), "{result}");
    assert_eq!(std::fs::read_to_string(source).unwrap(), "source");
    let failure = tool
        .invoke(&call(json!({"shell":{"text":"exit 23"}})))
        .unwrap();
    assert_eq!(
        serde_json::to_value(failure.result).unwrap()["exit_code"],
        json!(23)
    );
    let memory = tool
        .invoke(&call(json!({"program":{"path":"python3","args":["-c",
        "import sys; allocation=bytearray(256*1024*1024); sys.exit(0)"]}})))
        .unwrap();
    let memory = serde_json::to_value(memory.result).unwrap();
    assert_ne!(
        memory["exit_code"],
        json!(0),
        "the hard memory ceiling must reject actual allocation: {memory}"
    );
    let pids = r#"import errno,os,signal
children=[]
try:
 for i in range(128):
  try:
   pid=os.fork()
  except OSError as e:
   assert e.errno==errno.EAGAIN
   break
  if pid==0:
   signal.pause();os._exit(0)
  children.append(pid)
 assert 0<len(children)<64
 print('pids enforced',len(children))
finally:
 for pid in children: os.kill(pid,signal.SIGKILL)
 for pid in children: os.waitpid(pid,0)
"#;
    let checked = tool
        .invoke(&call(
            json!({"program":{"path":"python3","args":["-c",pids]}}),
        ))
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
    let checked = tool
        .invoke(&call(
            json!({"program":{"path":"python3","args":["-c",cpu]}}),
        ))
        .unwrap();
    let checked = serde_json::to_value(checked.result).unwrap();
    assert_eq!(checked["exit_code"], json!(0), "{checked}");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let network = format!(
        "import socket; s=socket.socket(); s.settimeout(.2); assert s.connect_ex(('127.0.0.1',{port})) != 0"
    );
    let checked = tool
        .invoke(&call(
            json!({"program":{"path":"python3","args":["-c",network]}}),
        ))
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
    for halt in [true, false] {
        let short = Backlog::with_window(crate::PollBudget::new(1, 1));
        let owner = setup(work.path(), None, None);
        let run = owner.run;
        let scope = owner.domain.clone();
        let background = ExecTool::new(owner, Box::new(EchoSandbox::new()), short.clone())
            .unwrap()
            .with_container(limits.clone(), runtime.clone());
        let started = background.invoke(&call(json!({"program":{"path":"python3","args":["-c",
            "import subprocess,time; subprocess.Popen(['python3','-c','import time; time.sleep(600)']); time.sleep(600)"]}}))).unwrap();
        assert_eq!(
            serde_json::to_value(started.result).unwrap()["outcome"],
            json!("backgrounded")
        );
        assert_eq!(short.standing(&scope).unwrap().len(), 1);
        if halt {
            short.halt(Some(&scope)).unwrap();
        }
        drop(background);
        short.harvest(run).unwrap();
        assert!(short.standing(&scope).unwrap().is_empty());
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
