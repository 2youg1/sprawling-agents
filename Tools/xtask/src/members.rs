// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::report::XtaskError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Product,

    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Member {
    pub(crate) package: String,

    pub(crate) lib: Option<String>,

    pub(crate) dir: String,
    pub(crate) role: Role,

    pub(crate) depends_on: BTreeSet<String>,

    pub(crate) publish: Publish,

    pub(crate) roots: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Publish {
    Registry,

    Never,
}

impl Member {
    pub(crate) fn name(&self) -> &str {
        self.lib.as_deref().unwrap_or(&self.package)
    }

    pub(crate) fn holds(&self, rel: &str) -> bool {
        rel.strip_prefix(self.dir.as_str())
            .is_some_and(|tail| tail.starts_with('/'))
    }

    pub(crate) fn in_product_graph(&self) -> bool {
        self.role == Role::Product
    }
}

const COMPILED: [&str; 8] = [
    "lib",
    "rlib",
    "dylib",
    "cdylib",
    "staticlib",
    "proc-macro",
    "bin",
    "custom-build",
];

const METADATA: [&str; 5] = [
    "metadata",
    "--format-version",
    "1",
    "--no-deps",
    "--offline",
];

const USAGE: &str = "usage: cargo xtask members --owning [<path>...] | --dir <package>";

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
pub(crate) fn members(root: &Path) -> Result<Vec<Member>, XtaskError> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .args(METADATA)
        .current_dir(root)
        .output()
        .map_err(|source| XtaskError::Io {
            path: "cargo metadata".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd: "cargo metadata".to_owned(),
            msg: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(|err| XtaskError::Cmd {
            cmd: "cargo metadata".to_owned(),
            msg: err.to_string(),
        })?;
    read(&metadata)
}

pub(crate) fn product(root: &Path) -> Result<Vec<Member>, XtaskError> {
    Ok(members(root)?
        .into_iter()
        .filter(|member| member.role == Role::Product)
        .collect())
}

pub(crate) fn find<'a>(found: &'a [Member], name: &str) -> Result<&'a Member, XtaskError> {
    found
        .iter()
        .find(|member| member.package == name || member.lib.as_deref() == Some(name))
        .ok_or_else(|| XtaskError::UnknownPackage {
            name: name.to_owned(),
            known: found
                .iter()
                .map(|member| member.package.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        })
}

pub(crate) fn owner<'a>(found: &'a [Member], rel: &str) -> Option<&'a Member> {
    found
        .iter()
        .filter(|member| member.holds(rel))
        .max_by_key(|member| member.dir.len())
}

pub(crate) fn run(root: &Path, args: &[String]) -> Result<String, XtaskError> {
    match args.split_first() {
        Some((flag, paths)) if flag == "--owning" => {
            let given = if paths.is_empty() {
                standard_input()?
            } else {
                paths.to_vec()
            };
            Ok(owning(&members(root)?, &given)
                .into_iter()
                .map(|package| format!("{package}\n"))
                .collect())
        }
        Some((flag, [package])) if flag == "--dir" => {
            let found = members(root)?;
            Ok(format!("{}\n", find(&found, package)?.dir))
        }
        Some(_) | None => Err(XtaskError::Doc {
            file: "members".to_owned(),
            msg: USAGE.to_owned(),
        }),
    }
}

fn owning<'a>(found: &'a [Member], paths: &[String]) -> BTreeSet<&'a str> {
    paths
        .iter()
        .map(|path| path.trim().replace('\\', "/"))
        .filter(|path| !path.ends_with(".md") && !path.ends_with(".lean"))
        .filter_map(|path| owner(found, &path))
        .map(|member| member.package.as_str())
        .collect()
}

fn standard_input() -> Result<Vec<String>, XtaskError> {
    std::io::stdin()
        .lines()
        .map(|line| {
            line.map_err(|source| XtaskError::Io {
                path: "standard input".to_owned(),
                source,
            })
        })
        .collect()
}

fn read(metadata: &Value) -> Result<Vec<Member>, XtaskError> {
    let root = Path::new(text(metadata, "workspace_root")?);
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("a `packages` array"))?;
    let mut found = packages
        .iter()
        .map(|package| workspace_member(root, package))
        .collect::<Result<Vec<_>, _>>()?;
    found.sort_by_key(|member| member.dir.clone());
    Ok(found)
}

fn workspace_member(root: &Path, package: &Value) -> Result<Member, XtaskError> {
    let name = text(package, "name")?;
    let manifest = Path::new(text(package, "manifest_path")?);
    let dir = manifest
        .parent()
        .ok_or_else(|| missing("a manifest path with a parent directory"))?;
    Ok(Member {
        package: name.to_owned(),
        lib: lib_of(package)?,
        dir: relative(root, dir, name)?,
        role: role_of(package, name)?,
        depends_on: dependencies(package)?
            .iter()
            .filter(|dependency| dependency.get("kind").and_then(Value::as_str) != Some("dev"))
            .filter_map(|dependency| dependency.get("name").and_then(Value::as_str))
            .map(str::to_owned)
            .collect(),
        publish: publish_of(package),
        roots: roots_of(root, package, name)?,
    })
}

fn publish_of(package: &Value) -> Publish {
    let refused = package
        .get("publish")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    if refused {
        Publish::Never
    } else {
        Publish::Registry
    }
}

fn roots_of(root: &Path, package: &Value, name: &str) -> Result<Vec<String>, XtaskError> {
    targets(package)?
        .iter()
        .filter(|target| {
            target
                .get("kind")
                .and_then(Value::as_array)
                .is_some_and(|kinds| {
                    kinds
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|kind| COMPILED.contains(&kind))
                })
        })
        .map(|target| relative(root, Path::new(text(target, "src_path")?), name))
        .collect()
}

fn targets(package: &Value) -> Result<&Vec<Value>, XtaskError> {
    package
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("a `targets` array for every package"))
}

fn relative(root: &Path, dir: &Path, package: &str) -> Result<String, XtaskError> {
    let tail = dir
        .strip_prefix(root)
        .map_err(|_| XtaskError::OutsideCheckout {
            package: package.to_owned(),
            dir: dir.display().to_string(),
            root: root.display().to_string(),
        })?;
    Ok(tail
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn lib_of(package: &Value) -> Result<Option<String>, XtaskError> {
    Ok(targets(package)?
        .iter()
        .find(|target| {
            target
                .get("kind")
                .and_then(Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("lib")))
        })
        .and_then(|target| target.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned))
}

fn role_of(package: &Value, name: &str) -> Result<Role, XtaskError> {
    match package.pointer("/metadata/sprawling/role") {
        None => Ok(Role::Product),
        Some(Value::String(role)) if role == "tool" => Ok(Role::Tool),
        Some(other) => Err(XtaskError::UnknownRole {
            package: name.to_owned(),
            role: other.to_string(),
        }),
    }
}

fn dependencies(package: &Value) -> Result<&Vec<Value>, XtaskError> {
    package
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("a `dependencies` array for every package"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, XtaskError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| missing(&format!("a `{key}` string")))
}

fn missing(what: &str) -> XtaskError {
    XtaskError::Doc {
        file: "cargo metadata".to_owned(),
        msg: format!("the output has no {what}"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
