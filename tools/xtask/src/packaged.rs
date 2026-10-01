// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Packaged gate: a package that can be published compiles only files it
//! carries (tools/xtask/Spec.lean §8-49).
//!
//! **A `.crate` is one package directory.** `cargo package` archives what
//! a package's directory holds, and the packaging check and every
//! `cargo install` compile that archive unpacked somewhere else. An
//! `include_str!` that reaches `../../docs/` compiles in this checkout
//! and fails there, and nobody sees it fail until a release is cut or a
//! stranger installs. This gate sees it on every `just check`.
//!
//! **What is judged is what cargo compiles.** The walk starts at the
//! targets cargo's metadata names and follows `mod` declarations the way
//! the compiler does, leaving out whatever a `cfg` keeps out of every
//! build that is not a test build (section 12-16). A file name says
//! nothing here: this tree loads tests through `#[path]`, through
//! `*_tests.rs` and through `mod tests;`, and the module tree is the one
//! answer that does not guess.

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

/// Where one published package ends: its own directory, minus the
/// directories of the packages nested in it, which cargo leaves out of
/// its archive because each carries a `Cargo.toml` of its own.
struct Reach<'a> {
    root: &'a Path,
    package: &'a Path,
    nested: &'a [PathBuf],
}

impl Reach<'_> {
    /// One call, judged: `None` when it reads a file this package
    /// carries or its build script's output.
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
        rule: "a published package compiles only files it carries (tools/xtask/Spec.lean §8-49)"
            .to_owned(),
        violation,
        alternative: "move the file into the package that owns it; or let the build script \
                      find it and write it into OUT_DIR, read as \
                      `concat!(env!(\"OUT_DIR\"), \"/<file>\")`; or have the package that \
                      carries it export a constant and read that"
            .to_owned(),
    }
}

/// `path` with `.` and `..` resolved by spelling alone, the way the
/// compiler joins an `include_str!` argument onto a directory before it
/// opens anything.
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

/// Whether a `cfg` predicate cannot hold while `test` is false, which
/// keeps its item out of every build a published package makes.
///
/// `not(…)` and every other predicate count as able to hold: the error
/// in that direction is a finding somebody reads, and the other one is
/// a published package that fails to build.
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

/// Whether an item's own `cfg` attributes keep it out of every build
/// that is not a test build.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| match attr.parse_args::<syn::Meta>() {
            Ok(predicate) => excludes_production(&predicate),
            // A predicate this gate cannot read may hold, so its item is
            // judged.
            Err(_unreadable) => false,
        })
}

/// The predicates inside `all(…)` or `any(…)`; `None` when they do not
/// read as predicates, which its caller takes as "may hold".
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
