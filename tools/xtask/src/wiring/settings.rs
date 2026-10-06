// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Coverage decisions for settable fields (`tools/xtask/Spec.lean`).

use crate::report::{Violation, XtaskError};
use crate::walk;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut found = Vec::new();
    for (source, spec) in [
        ("crates/wire/src/preference", "crates/wire"),
        ("crates/city/src/config_layers", "crates/city"),
        ("crates/kernel/src/config/container", "crates/kernel"),
    ] {
        let mut files = vec![root.join(format!("{source}.rs"))];
        if root.join(source).is_dir() {
            files.extend(walk::files_with_ext(&root.join(source), &["rs"])?);
        }
        let mut records = String::new();
        for file in walk::files_with_ext(&root.join(spec), &["lean"])? {
            records.push_str(&walk::read_text(&file)?);
            records.push(char::from(10));
        }
        for file in files {
            let text = walk::read_text(&file)?;
            for field in fields(&text, &walk::rel(root, &file))? {
                if !covered(root, &records, &field) {
                    found.push(Violation {
                        gate: "wiring",
                        location: walk::rel(root, &file),
                        rule: "every settable field has a settings control or a recorded reason".to_owned(),
                        violation: format!("{field} has no valid coverage record in {spec}"),
                        alternative: format!("add `settings-control {field} client/src/path` or `settings-reason {field} <reason>` to its owning SPEC"),
                    });
                }
            }
        }
    }
    Ok(found)
}

fn fields(text: &str, location: &str) -> Result<BTreeSet<String>, XtaskError> {
    let parsed = syn::parse_file(text).map_err(|err| XtaskError::Doc {
        file: location.to_owned(),
        msg: format!("read configuration declarations: {err}"),
    })?;
    let mut found = BTreeSet::new();
    for item in parsed.items {
        match item {
            syn::Item::Struct(item) if deserialized(&item.attrs) => {
                named(&mut found, &item.ident.to_string(), &item.fields);
            }
            syn::Item::Enum(item) if deserialized(&item.attrs) => {
                for variant in item.variants {
                    named(
                        &mut found,
                        &format!("{}.{}", item.ident, variant.ident),
                        &variant.fields,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(found)
}

fn deserialized(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        let mut yes = false;
        let parsed = attr.parse_nested_meta(|meta| {
            yes |= meta.path.is_ident("Deserialize");
            Ok(())
        });
        parsed.is_ok() && yes
    })
}

fn named(found: &mut BTreeSet<String>, owner: &str, fields: &syn::Fields) {
    for field in fields {
        if let Some(name) = &field.ident {
            found.insert(format!("{owner}.{name}"));
        }
    }
}

fn covered(root: &Path, records: &str, field: &str) -> bool {
    records.lines().any(|line| {
        let mut cells = line.trim().splitn(3, ' ');
        let kind = cells.next();
        if cells.next() != Some(field) {
            return false;
        }
        let Some(detail) = cells.next().filter(|detail| !detail.trim().is_empty()) else {
            return false;
        };
        match kind {
            Some("settings-control") => {
                detail.starts_with("client/src/") && root.join(detail).is_file()
            }
            Some("settings-reason") => true,
            _ => false,
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_new_field_requires_its_own_coverage_decision() {
        let fixture = crate::root::fixture::relocated("settings-fields");
        let text = "#[derive(Deserialize)] struct Config { known: u64, added: Option<u64> }";
        let fields = fields(text, "fixture.rs").unwrap();
        let records = "settings-reason Config.known resolved by a session control";
        let missing: Vec<_> = fields
            .iter()
            .filter(|field| !covered(&fixture, records, field))
            .cloned()
            .collect();
        assert_eq!(missing, vec!["Config.added"]);
        assert!(covered(
            &fixture,
            "settings-reason Config.added explicitly edited in raw config",
            "Config.added"
        ));
        assert!(!covered(
            &fixture,
            "settings-control Config.added client/src/missing.svelte",
            "Config.added"
        ));
        assert!(!covered(
            &fixture,
            "settings-reason Config.added ",
            "Config.added"
        ));
        std::fs::remove_dir_all(fixture).unwrap();
    }
}
