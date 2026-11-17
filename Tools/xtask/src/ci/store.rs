// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Success-only verdict storage. The local disk and GitHub Actions cache
//! adapters share one validation rule (xtask `spec/Ci.lean`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::report::XtaskError;

use super::content::{Environment, Key};
use super::graph::Reuse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Verdict {
    Green,
    Refused,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Cached {
    pub(crate) key: String,
    pub(crate) verdict: Verdict,
}

/// Lean `may_reuse`: a green result under the same complete content key.
pub(crate) fn may_reuse(policy: Reuse, cached: &Cached, current: &Key) -> bool {
    policy == Reuse::SuccessOnly
        && cached.verdict == Verdict::Green
        && cached.key == current.as_str()
}

#[derive(Debug)]
pub(crate) struct RemoteStore {
    adapter: PathBuf,
    work: PathBuf,
}

#[derive(Debug)]
pub(crate) enum Store {
    Local(PathBuf),
    Remote(RemoteStore),
}

impl Store {
    #[cfg(test)]
    pub(crate) fn local(path: PathBuf) -> Self {
        Self::Local(path)
    }

    pub(crate) fn for_root(root: &Path, local_override: Option<PathBuf>) -> Self {
        if let Some(path) = local_override {
            return Self::Local(path);
        }
        match std::env::var_os("SPRAWLING_CI_CACHE_ADAPTER") {
            Some(adapter) if !adapter.is_empty() => Self::Remote(RemoteStore {
                adapter: PathBuf::from(adapter),
                work: root.join("target/ci/remote"),
            }),
            _ => Self::Local(root.join("target/ci/verdicts")),
        }
    }

    pub(crate) fn restore(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
        policy: Reuse,
    ) -> Result<Option<Cached>, XtaskError> {
        if policy == Reuse::Never {
            return Ok(None);
        }
        let cached = match self {
            Self::Local(root) => read_local(&local_path(root, action, key))?,
            Self::Remote(remote) => remote.restore(action, key, environment)?,
        };
        Ok(cached.filter(|cached| may_reuse(policy, cached, key)))
    }

    /// Publish a green verdict. There is deliberately no save operation for
    /// refused or broken outcomes.
    pub(crate) fn save_green(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
    ) -> Result<(), XtaskError> {
        let cached = Cached {
            key: key.as_str().to_owned(),
            verdict: Verdict::Green,
        };
        match self {
            Self::Local(root) => write_local(&local_path(root, action, key), &cached),
            Self::Remote(remote) => remote.save(action, key, environment, &cached),
        }
    }
}

fn local_path(root: &Path, action: &str, key: &Key) -> PathBuf {
    root.join(action).join(format!("{}.json", key.as_str()))
}

fn read_local(path: &Path) -> Result<Option<Cached>, XtaskError> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) => {
            return Err(XtaskError::Io {
                path: path.display().to_string(),
                source,
            });
        }
    };
    Ok(serde_json::from_slice(&bytes).ok())
}

fn write_local(path: &Path, cached: &Cached) -> Result<(), XtaskError> {
    let parent = path.parent().ok_or_else(|| XtaskError::Doc {
        file: path.display().to_string(),
        msg: "the verdict path has no parent directory (verdict-root)".to_owned(),
    })?;
    std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
        path: parent.display().to_string(),
        source,
    })?;
    if read_local(path)?.is_some() {
        return Ok(());
    }
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".{}-{sequence}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(cached).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: format!("serialize a green verdict: {err} (verdict-serialize)"),
    })?;
    std::fs::write(&temporary, bytes).map_err(|source| XtaskError::Io {
        path: temporary.display().to_string(),
        source,
    })?;
    match std::fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_already_exists) if path.is_file() => {
            std::fs::remove_file(&temporary).map_err(|source| XtaskError::Io {
                path: temporary.display().to_string(),
                source,
            })?;
            Ok(())
        }
        Err(source) => Err(XtaskError::Io {
            path: path.display().to_string(),
            source,
        }),
    }
}

fn remote_dir(work: &Path, action: &str, key: &Key) -> PathBuf {
    work.join(action).join(key.as_str())
}

pub(crate) fn remote_key(action: &str, key: &Key, environment: &Environment) -> String {
    format!("ci-action-{action}-{}", key.cache_key(environment))
}

impl RemoteStore {
    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    fn restore(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
    ) -> Result<Option<Cached>, XtaskError> {
        let dir = remote_dir(&self.work, action, key);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|source| XtaskError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        std::fs::create_dir_all(&dir).map_err(|source| XtaskError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let output = Command::new("bun")
            .arg(&self.adapter)
            .arg("restore")
            .arg(remote_key(action, key, environment))
            .arg(&dir)
            .output()
            .map_err(|source| XtaskError::Io {
                path: self.adapter.display().to_string(),
                source,
            })?;
        if !output.status.success() {
            return Err(XtaskError::Cmd {
                cmd: "ci-cache restore".to_owned(),
                msg: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if String::from_utf8_lossy(&output.stdout).lines().last() == Some("hit") {
            read_local(&dir.join("verdict.json"))
        } else {
            Ok(None)
        }
    }

    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    fn save(
        &self,
        action: &str,
        key: &Key,
        environment: &Environment,
        cached: &Cached,
    ) -> Result<(), XtaskError> {
        let dir = remote_dir(&self.work, action, key);
        write_local(&dir.join("verdict.json"), cached)?;
        let output = Command::new("bun")
            .arg(&self.adapter)
            .arg("save")
            .arg(remote_key(action, key, environment))
            .arg(&dir)
            .output()
            .map_err(|source| XtaskError::Io {
                path: self.adapter.display().to_string(),
                source,
            })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(XtaskError::Cmd {
                cmd: "ci-cache save".to_owned(),
                msg: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn key() -> Key {
        Key::named("same-content".to_owned())
    }

    #[test]
    fn only_the_same_green_content_is_reused() {
        let green = Cached {
            key: "same-content".to_owned(),
            verdict: Verdict::Green,
        };
        assert!(may_reuse(Reuse::SuccessOnly, &green, &key()));
        assert!(!may_reuse(Reuse::Never, &green, &key()));
        assert!(!may_reuse(
            Reuse::SuccessOnly,
            &green,
            &Key::named("changed".to_owned())
        ));
        for verdict in [Verdict::Refused, Verdict::Broken] {
            assert!(!may_reuse(
                Reuse::SuccessOnly,
                &Cached {
                    key: green.key.clone(),
                    verdict
                },
                &key()
            ));
        }
    }

    #[test]
    fn local_round_trip_and_corruption_fail_closed() {
        let root = std::env::temp_dir().join(format!("xtask-ci-store-{}", std::process::id()));
        let store = Store::local(root.clone());
        let environment = Environment {
            os: "test".to_owned(),
            arch: "test".to_owned(),
        };
        store.save_green("gate", &key(), &environment).unwrap();
        assert!(
            store
                .restore("gate", &key(), &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_some()
        );
        assert!(
            store
                .restore(
                    "gate",
                    &Key::named("other".to_owned()),
                    &environment,
                    Reuse::SuccessOnly
                )
                .unwrap()
                .is_none()
        );
        let record = local_path(&root, "gate", &key());
        std::fs::write(&record, "not json").unwrap();
        assert!(
            store
                .restore("gate", &key(), &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_none()
        );
        store.save_green("gate", &key(), &environment).unwrap();
        assert!(
            store
                .restore("gate", &key(), &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_some(),
            "publishing over a corrupted entry must heal it"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_publication_leaves_one_valid_green_result() {
        let root = std::env::temp_dir().join(format!("xtask-ci-store-race-{}", std::process::id()));
        let store = Store::local(root.clone());
        let environment = Environment {
            os: "test".to_owned(),
            arch: "test".to_owned(),
        };
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| store.save_green("gate", &key(), &environment).unwrap());
            }
        });
        assert!(
            store
                .restore("gate", &key(), &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_some()
        );
        let files = std::fs::read_dir(root.join("gate")).unwrap().count();
        assert_eq!(files, 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remote_cache_keys_include_environment() {
        let environment = Environment {
            os: "windows".to_owned(),
            arch: "x86_64".to_owned(),
        };
        let other = Environment {
            os: "linux".to_owned(),
            arch: "x86_64".to_owned(),
        };
        let keys = BTreeSet::from([key().cache_key(&environment), key().cache_key(&other)]);
        assert_eq!(keys.len(), 2);
        assert_ne!(
            remote_key("one", &key(), &environment),
            remote_key("two", &key(), &environment)
        );
    }

    #[test]
    fn the_remote_adapter_round_trip_preserves_each_content_generation() {
        let root =
            std::env::temp_dir().join(format!("xtask-ci-remote-store-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let adapter = root.join("adapter.ts");
        let store = serde_json::to_string(&root.join("objects")).unwrap();
        std::fs::write(
            &adapter,
            format!(
                r#"import {{ join }} from "node:path";
const [operation, key, directory] = process.argv.slice(2);
const object = join({store}, `${{key}}.json`);
if (operation === "save") {{
  await Bun.write(object, await Bun.file(join(directory, "verdict.json")).text());
}} else if (await Bun.file(object).exists()) {{
  await Bun.write(join(directory, "verdict.json"), await Bun.file(object).text());
  console.log("hit");
}} else {{
  console.log("miss");
}}
"#
            ),
        )
        .unwrap();
        std::fs::create_dir_all(root.join("objects")).unwrap();
        let store = Store::Remote(RemoteStore {
            adapter,
            work: root.join("work"),
        });
        let environment = Environment {
            os: "test".to_owned(),
            arch: "test".to_owned(),
        };
        for key in [key(), Key::named("next-generation".to_owned())] {
            store.save_green("gate", &key, &environment).unwrap();
            assert!(
                store
                    .restore("gate", &key, &environment, Reuse::SuccessOnly)
                    .unwrap()
                    .is_some()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
