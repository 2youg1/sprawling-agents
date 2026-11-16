// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Component, Path, PathBuf};

use syn::punctuated::Punctuated;

use crate::members::{self, Publish};
use crate::report::{Violation, XtaskError};
use crate::walk;

mod modules;

use modules::{Include, Target};

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let found = members::members(root)?;
    let mut judged: Vec<(String, usize, Violation)> = Vec::new();
    for member in found.iter().filter(|it| it.publish == Publish::Registry) {
        let package = normal(&root.join(&member.dir));
        let nested: Vec<PathBuf> = found
            .iter()
            .filter(|other| member.holds(&other.dir))
            .map(|other| normal(&root.join(&other.dir)))
            .collect();
        let reach = Reach {
            root,
            package: &package,
            nested: &nested,
        };
        for include in modules::includes(root, member)? {
            if let Some(violation) = reach.judge(&include) {
                judged.push((include.file, include.line, violation));
            }
        }
    }
    judged.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    Ok(judged
        .into_iter()
        .map(|(_, _, violation)| violation)
        .collect())
}

struct Reach<'a> {
    root: &'a Path,
    package: &'a Path,
    nested: &'a [PathBuf],
}

impl Reach<'_> {

    fn judge(&self, include: &Include) -> Option<Violation> {
        let location = format!("{}:{}", include.file, include.line);
        match &include.target {
            Target::BuildOutput => None,
            Target::Path(path) if self.carries(path) => None,
            Target::Path(path) => Some(packaged(
                location,
                format!(
                    "{} reads {}, which is outside {}",
                    include.written,
                    walk::rel(self.root, path),
                    walk::rel(self.root, self.package)
                ),
            )),
            Target::Unread(shown) => Some(packaged(
                location,
                format!("this gate cannot read which file {shown} names"),
            )),
        }
    }

    fn carries(&self, path: &Path) -> bool {
        path.starts_with(self.package) && !self.nested.iter().any(|dir| path.starts_with(dir))
    }
}

fn packaged(location: String, violation: String) -> Violation {
    Violation {
        gate: "packaged",
        location,
        rule: "a published package compiles only files it carries (Tools/xtask/Spec.lean §8-49)"
            .to_owned(),
        violation,
        alternative: "move the file into the package that owns it; or let the build script \
                      find it and write it into OUT_DIR, read as \
                      `concat!(env!(\"OUT_DIR\"), \"/<file>\")`; or have the package that \
                      carries it export a constant and read that"
            .to_owned(),
    }
}

fn normal(path: &Path) -> PathBuf {
    path.components().fold(PathBuf::new(), |mut at, part| {
        match part {
            Component::ParentDir => {
                at.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                at.push(part.as_os_str());
            }
        }
        at
    })
}

fn excludes_production(predicate: &syn::Meta) -> bool {
    match predicate {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) if list.path.is_ident("all") => {
            nested(list).is_some_and(|parts| parts.iter().any(excludes_production))
        }
        syn::Meta::List(list) if list.path.is_ident("any") => {
            nested(list).is_some_and(|parts| parts.iter().all(excludes_production))
        }
        syn::Meta::List(_) | syn::Meta::NameValue(_) => false,
    }
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| match attr.parse_args::<syn::Meta>() {
            Ok(predicate) => excludes_production(&predicate),

            Err(_unreadable) => false,
        })
}

fn nested(list: &syn::MetaList) -> Option<Vec<syn::Meta>> {
    match list.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated) {
        Ok(parts) => Some(parts.into_iter().collect()),
        Err(_unreadable) => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
