// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[cfg(test)]
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
  if [ -f create-barrier ]; then
    echo "$$" >create-pid
    ps -o ppid= -p "$$" >create-parent
    touch create-entered
    read -r release <create-fifo
  fi
  touch owned create-completed
  if [ -f fail-create ]; then exit 42; fi
  echo container-id ;;
container)
  case "$2" in
    inspect)
      running=false
      status=created
      if [ -f started ]; then status=exited; fi
      if [ -f running ]; then running=true; status=running; fi
      printf '[{"Config":{"User":"1000","WorkingDir":"/work"},"HostConfig":{"NanoCpus":1250000000,"Memory":67108864,"MemorySwap":67108864,"PidsLimit":64,"NetworkMode":"none","ReadonlyRootfs":true,"CapDrop":["ALL"],"SecurityOpt":["no-new-privileges"]},"Mounts":[{"Source":"%s","Destination":"/work","Type":"bind"}],"State":{"Status":"%s","Running":%s,"ExitCode":23}}]\n' "$(cat copy)" "$status" "$running" ;;
    ls) if [ -f owned ]; then cat name; fi ;;
  esac ;;
start)
  touch started running
  if [ -f background ]; then
    while [ -f running ]; do read -r unused <fifo; done
  fi
  rm -f running
  echo target-output
  exit 91 ;;
rm)
  if [ -f remove-retried ]; then touch remove-retried-again; fi
  if [ -f remove-attempted ]; then touch remove-retried; fi
  touch remove-attempted
  if [ ! -f owned ]; then echo absent >&2; exit 44; fi
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
        ExecTool::new(setup(work, None, None), backlog.clone())
            .unwrap()
            .with_container(
                limits(),
                ContainerRuntime::probe(ContainerEngine::Docker, self.program.clone()).unwrap(),
            )
    }

    fn guardian(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let executable = std::env::current_exe().unwrap();
        let executable = executable.to_str().unwrap().replace('\'', "'\\''");
        let exited = self.dir.path().join("guardian-exited");
        let exited = exited.to_str().unwrap().replace('\'', "'\\''");
        let harness = self.dir.path().join("guardian");
        std::fs::write(
            &harness,
            format!(r#"#!/bin/bash
set -eu -o pipefail
export SPRAWLING_GUARD_RECORD="$2"
'{executable}' --exact tools::exec::tests::container::guardian::guardian_process --ignored --nocapture --quiet | awk '/^ready$/ {{print; fflush()}}'
touch '{exited}'
"#),
        ).unwrap();
        std::fs::set_permissions(&harness, std::fs::Permissions::from_mode(0o700)).unwrap();
        harness
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
        .invoke(&call(line_of("/image-only/program", NO_ARGS)))
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
    assert!(tool.invoke(&call(line_of("/bin/true", NO_ARGS))).is_err());
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
    assert!(tool.invoke(&call(line_of("/bin/true", NO_ARGS))).is_err());
    let copy = PathBuf::from(std::fs::read_to_string(daemon.dir.path().join("copy")).unwrap());
    assert!(daemon.exists() && copy.is_dir());
    assert_eq!(backlog.standing(&tool.setup.domain).unwrap().len(), 1);
    assert!(backlog.harvest(tool.setup.run).is_err());
    std::fs::remove_file(daemon.dir.path().join("fail-remove")).unwrap();
    assert!(backlog.harvest(tool.setup.run).unwrap().is_empty());
    assert!(!daemon.exists() && !copy.exists());
}

mod guardian;
mod production;
