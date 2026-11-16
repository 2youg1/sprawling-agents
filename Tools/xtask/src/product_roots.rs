// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Product-root path table in `architecture.toml` (xtask D45).
//!
//! The one reader of `[[product_root]]` and `[[legacy_path]]`. Later
//! moves derive package directories from [`ProductRoot::path`]; the
//! residue gate refuses a legacy path once its status is `retired`;
//! `release` still treats every legacy path name as a citation anchor
//! so historical CHANGELOG paths keep a home in the tree's vocabulary.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::modmap;
use crate::report::XtaskError;
use crate::walk;

/// Every product root the foundation slice names, in declaration order.
const REQUIRED: [&str; 5] = ["Sprawling", "Tools", "Calyx", "RefRain", "Mei"];

/// Whether the checkout already holds the root, or a later move will.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layout {
    Current,
    Planned,
}

/// Whether a legacy path still belongs in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LegacyStatus {
    Live,
    Retired,
}

/// One product root row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ProductRoot {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) layout: Layout,
    #[serde(default)]
    pub(crate) transition: Option<String>,
}

/// One path the residue gate may refuse once retired.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct LegacyPath {
    pub(crate) path: String,
    pub(crate) status: LegacyStatus,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    product_root: Vec<ProductRoot>,
    #[serde(default)]
    legacy_path: Vec<LegacyPath>,
}

/// The table loaded from [`modmap::MAP`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Table {
    pub(crate) roots: Vec<ProductRoot>,
    pub(crate) legacy: Vec<LegacyPath>,
}

impl Table {
    /// The root directory for `name`, when the table holds that product.
    ///
    /// F2 joins this with a package leaf (`format!("{root}/{package}")`)
    /// so a move rewrites from one table rather than a second map.
    #[must_use]
    pub(crate) fn path_of(&self, name: &str) -> Option<&str> {
        self.roots
            .iter()
            .find(|root| root.name == name)
            .map(|root| root.path.as_str())
    }

    /// Legacy paths whose status is `retired`.
    #[must_use]
    pub(crate) fn retired(&self) -> Vec<&LegacyPath> {
        self.legacy
            .iter()
            .filter(|row| row.status == LegacyStatus::Retired)
            .collect()
    }
}

/// Load and validate the product-root table.
///
/// # Errors
/// When the file is missing or unreadable, when a row fails the closed
/// field set, when a required product is absent, or when two rows share
/// a `path` or a `name`.
pub(crate) fn load(root: &Path) -> Result<Table, XtaskError> {
    let file = modmap::MAP;
    let text = walk::read_text(&root.join(file))?;
    let parsed: File = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: file.to_owned(),
        msg: format!("the product-root table does not parse: {err}"),
    })?;
    let table = Table {
        roots: parsed.product_root,
        legacy: parsed.legacy_path,
    };
    validate(&table)?;
    Ok(table)
}

fn validate(table: &Table) -> Result<(), XtaskError> {
    let file = modmap::MAP;
    let names: BTreeSet<&str> = table.roots.iter().map(|root| root.name.as_str()).collect();
    for required in REQUIRED {
        if !names.contains(required) {
            return Err(XtaskError::Doc {
                file: file.to_owned(),
                msg: format!(
                    "product-root table lacks `{required}`; the closed set is {}",
                    REQUIRED.join(", ")
                ),
            });
        }
    }
    if names.len() != table.roots.len() {
        return Err(XtaskError::Doc {
            file: file.to_owned(),
            msg: "product-root table repeats a `name`".to_owned(),
        });
    }
    let paths: BTreeSet<&str> = table.roots.iter().map(|root| root.path.as_str()).collect();
    if paths.len() != table.roots.len() {
        return Err(XtaskError::Doc {
            file: file.to_owned(),
            msg: "product-root table repeats a `path`".to_owned(),
        });
    }
    for root in &table.roots {
        refuse_one_segment(file, "product root path", &root.path)?;
        if let Some(transition) = &root.transition {
            refuse_one_segment(file, "product root transition", transition)?;
        }
    }
    let legacy_paths: BTreeSet<&str> = table.legacy.iter().map(|row| row.path.as_str()).collect();
    if legacy_paths.len() != table.legacy.len() {
        return Err(XtaskError::Doc {
            file: file.to_owned(),
            msg: "legacy_path table repeats a `path`".to_owned(),
        });
    }
    for row in &table.legacy {
        refuse_one_segment(file, "legacy path", &row.path)?;
    }
    Ok(())
}

fn refuse_one_segment(file: &str, kind: &str, value: &str) -> Result<(), XtaskError> {
    use std::path::{Component, Path};
    let mut parts = Path::new(value).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(name)), None) if name.to_str() == Some(value) => Ok(()),
        _ => Err(XtaskError::Doc {
            file: file.to_owned(),
            msg: format!("{kind} `{value}` must be exactly one ordinary path segment"),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::root::fixture;
    use crate::root::this_checkout;

    fn write_table(body: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-product-roots-{}-{}",
            std::process::id(),
            body.len()
        ));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        fixture::write(&root, "Cargo.lock", "");
        fixture::write(&root, modmap::MAP, body);
        root
    }

    const MINIMAL: &str = r#"
module = []
[[product_root]]
name = "Sprawling"
path = "Sprawling"
layout = "planned"
[[product_root]]
name = "Tools"
path = "Tools"
layout = "planned"
[[product_root]]
name = "Calyx"
path = "Calyx"
layout = "current"
[[product_root]]
name = "RefRain"
path = "RefRain"
layout = "current"
[[product_root]]
name = "Mei"
path = "Mei"
layout = "current"
[[legacy_path]]
path = "crates"
status = "retired"
[[legacy_path]]
path = "client"
status = "retired"
"#;

    #[test]
    fn the_checkout_table_names_every_required_root_and_retires_crates() {
        let table = load(this_checkout()).unwrap();
        assert_eq!(table.path_of("Sprawling"), Some("Sprawling"));
        assert_eq!(
            table
                .path_of("Sprawling")
                .map(|root| format!("{root}/kernel"))
                .as_deref(),
            Some("Sprawling/kernel")
        );
        assert_eq!(table.path_of("Tools"), Some("Tools"));
        assert_eq!(table.path_of("Calyx"), Some("Calyx"));
        assert_eq!(table.path_of("RefRain"), Some("RefRain"));
        assert_eq!(table.path_of("Mei"), Some("Mei"));
        let sprawling = table
            .roots
            .iter()
            .find(|root| root.name == "Sprawling")
            .unwrap();
        assert_eq!(sprawling.layout, Layout::Current);
        let tools = table
            .roots
            .iter()
            .find(|root| root.name == "Tools")
            .unwrap();
        assert_eq!(tools.layout, Layout::Current);
        let refrain = table
            .roots
            .iter()
            .find(|root| root.name == "RefRain")
            .unwrap();
        assert_eq!(refrain.layout, Layout::Current);
        assert_eq!(refrain.transition, None);
        assert_eq!(
            table
                .retired()
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["crates", "client"]
        );
        assert_eq!(table.legacy.len(), 2);
    }

    #[test]
    fn a_missing_required_root_is_refused() {
        let body = MINIMAL.replace(
            "[[product_root]]\nname = \"Mei\"\npath = \"Mei\"\nlayout = \"current\"\n",
            "",
        );
        let root = write_table(&body);
        let err = load(&root).unwrap_err().to_string();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(err.contains("Mei"), "{err}");
    }

    #[test]
    fn a_later_move_joins_path_of_with_the_package_leaf() {
        let root = write_table(MINIMAL);
        let table = load(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            table
                .path_of("Tools")
                .map(|root| format!("{root}/xtask"))
                .as_deref(),
            Some("Tools/xtask")
        );
    }

    #[test]
    fn dot_and_dotdot_are_refused_as_root_segments() {
        for bad in [".", ".."] {
            let body = MINIMAL.replace(
                "path = \"Calyx\"\nlayout = \"current\"",
                &format!("path = \"{bad}\"\nlayout = \"current\""),
            );
            let root = write_table(&body);
            let err = load(&root).unwrap_err().to_string();
            std::fs::remove_dir_all(&root).unwrap();
            assert!(err.contains(bad), "{err}");
        }
    }
}
