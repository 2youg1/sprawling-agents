// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask spec <lib>`: create a crate's `Spec.lean` with the MPL
//! notice and the seventeen numbered section comments `skills/sdd` lists
//! (xtask-SPEC.md section 8-41), in the directory of the package whose lib
//! (or, for a tool without one, whose package) goes by that name.
//! Creation only: an existing `Spec.lean` is never overwritten, because
//! everything past the skeleton is a specification somebody wrote.
//!
//! The gate of the same name judges what the tree holds and never
//! writes (xtask-SPEC.md section 8-42): the command makes a
//! specification, the gate holds every specification to its rules, and
//! neither does the other's job.

use std::path::Path;

use crate::header::{Leader, notice};
use crate::members;
use crate::report::{Violation, XtaskError};

mod effective;
mod source;

/// The seventeen responsibilities of `skills/sdd`, in its order. What each
/// section holds is that skill's to say, so the skeleton carries titles only.
const SECTIONS: [&str; 17] = [
    "需求分解",
    "验收标准",
    "假设与歧义",
    "现状分析",
    "权威信源",
    "命名统一",
    "模块边界",
    "接口先行",
    "工作流程",
    "实现逻辑",
    "边界枚举",
    "错误处理",
    "依赖选型",
    "硬编码声明",
    "影响面",
    "测试与约束",
    "文档关系",
];

/// The `spec` gate: one effective specification per package, no prose
/// naming a SPEC the tree lacks, the import discipline, no `sorry`,
/// `admit` or `axiom`, and every path a specification cites on disk.
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let _ = root;
    Ok(Vec::new())
}

pub(crate) fn run(root: &Path, lib: Option<&str>) -> Result<String, XtaskError> {
    let lib = lib.ok_or_else(|| XtaskError::Doc {
        file: "spec".to_owned(),
        msg: "usage: cargo xtask spec <lib>".to_owned(),
    })?;
    let found = members::members(root)?;
    let path = root
        .join(&members::find(&found, lib)?.dir)
        .join("Spec.lean");
    if path.exists() {
        return Ok(format!(
            "already exists, left untouched: {}",
            path.to_string_lossy()
        ));
    }
    std::fs::write(&path, skeleton(lib)).map_err(|source| XtaskError::Io {
        path: path.to_string_lossy().into_owned(),
        source,
    })?;
    Ok(format!("created {}", path.to_string_lossy()))
}

fn skeleton(lib: &str) -> String {
    let head: String = notice(Leader::Lean)
        .iter()
        .map(|row| format!("{row}\n"))
        .collect();
    let sections: String = SECTIONS
        .iter()
        .zip(1_u8..)
        .map(|(title, number)| format!("\n/-! ## {number} {title}\n-/\n"))
        .collect();
    format!(
        "{head}\n/-! # {lib} 的规格\n\n\
         `{lib}` 的规格入口；分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。\n-/\n{sections}"
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
