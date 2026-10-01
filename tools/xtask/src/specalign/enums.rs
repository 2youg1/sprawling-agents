// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The code side of `specalign`'s roster assertion: every enum the
//! kernel compiles, by name, with its variant names, read with `syn`
//! from `crates/kernel/src/**`. `tables` holds every `inductive` of the
//! kernel's specification that shares a name with one of these to
//! exactly its variants, in both directions.
//!
//! **A roster somebody ports match arms from has to be the real one.**
//! When a specification spelled the fourth `SecretCharset` variant
//! `Base36Lower` and the kernel compiled `UpperBase36`, every arm written
//! from it failed to compile, and nothing in the build said which side
//! was wrong.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

/// Where the kernel's compiled enums are read from.
const KERNEL_SRC: &str = "crates/kernel/src";

/// The extension of the files parsed on the code side.
const RUST: [&str; 1] = ["rs"];

/// Every enum the kernel compiles, by name, with its variant names.
/// Enums inside a `#[cfg(test)]` module are left out: a fixture enum is
/// not part of the interface the specification settles.
pub(super) fn compiled(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, XtaskError> {
    let mut out = BTreeMap::new();
    for file in walk::files_with_ext(&root.join(KERNEL_SRC), &RUST)? {
        let text = walk::read_text(&file)?;
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: walk::rel(root, &file),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        collect(&parsed.items, &mut out);
    }
    Ok(out)
}

fn collect(items: &[syn::Item], out: &mut BTreeMap<String, BTreeSet<String>>) {
    for item in items {
        if let syn::Item::Enum(declared) = item {
            let names = declared.variants.iter().map(|v| v.ident.to_string());
            out.insert(declared.ident.to_string(), names.collect());
        } else if let syn::Item::Mod(module) = item
            && let Some((_, inner)) = &module.content
            && !test_only(&module.attrs)
        {
            collect(inner, out);
        }
    }
}

/// True when this item is compiled for tests only.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        syn::Meta::List(list) => {
            list.path.is_ident("cfg") && list.tokens.to_string().contains("test")
        }
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => false,
    })
}
