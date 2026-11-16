// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::header::{Leader, notice};
use crate::members;
use crate::report::{Violation, XtaskError};

mod decisions;
mod effective;
mod ratchet;
mod rustpath;
mod source;
mod theorems;

const CHECKER: &str = "Tools/adversary";

fn under_checker(rel: &str) -> Option<&str> {
    if let Some(rest) = rel.strip_prefix(CHECKER) {
        return match rest.strip_prefix('/') {
            Some(inside) => Some(inside),
            None if rest.is_empty() => Some(""),
            None => None,
        };
    }
    let prefix = CHECKER.as_bytes();
    let bytes = rel.as_bytes();
    if bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix) {
        let rest = &rel[prefix.len()..];
        return match rest.strip_prefix('/') {
            Some(inside) => Some(inside),
            None if rest.is_empty() => Some(""),
            None => None,
        };
    }
    None
}

fn checker_rel(rel: &str) -> std::borrow::Cow<'_, str> {
    match under_checker(rel) {
        Some("") => std::borrow::Cow::Borrowed(CHECKER),
        Some(inside) => std::borrow::Cow::Owned(format!("{CHECKER}/{inside}")),
        None => std::borrow::Cow::Borrowed(rel),
    }
}

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

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    effective::one_per_package(root, &mut violations)?;
    effective::no_dangling_names(root, &mut violations)?;
    source::check(root, &mut violations)?;
    ratchet::check(root, &mut violations)?;
    decisions::check(root, &mut violations)?;
    Ok(violations)
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
