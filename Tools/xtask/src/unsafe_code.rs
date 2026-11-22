// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use proc_macro2::{TokenStream, TokenTree};

use crate::report::{Violation, XtaskError};
use crate::walk;

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, None)
}

pub(crate) fn check_scoped(
    root: &Path,
    scope: &crate::source::Scope,
) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, Some(scope))
}

fn check_with(
    root: &Path,
    scope: Option<&crate::source::Scope>,
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for file in crate::source::unsafe_files_in(root, scope)? {
        let rel = walk::rel(root, &file);
        let text = walk::read_text(&file)?;
        let lines = unsafe_lines(&text, &rel)?;
        if let Some(first) = lines.first().copied() {
            violations.push(Violation {
                gate: "unsafe",
                location: format!("{rel}:{first}"),
                rule: "Rust source contains no unsafe token".to_owned(),
                violation: format!("{} unsafe token(s)", lines.len()),
                alternative: "use a safe interface or redesign the boundary; do not relocate or suppress unsafe Rust".to_owned(),
            });
        }
    }
    Ok(violations)
}

fn unsafe_lines(text: &str, rel: &str) -> Result<Vec<usize>, XtaskError> {
    let stream = text.parse::<TokenStream>().map_err(|err| XtaskError::Doc {
        file: rel.to_owned(),
        msg: format!("Rust tokenization failed: {err}"),
    })?;
    let mut lines = Vec::new();
    collect(stream, &mut lines);
    Ok(lines)
}

fn collect(stream: TokenStream, lines: &mut Vec<usize>) {
    for token in stream {
        match token {
            TokenTree::Ident(ident) if ident == "unsafe" => {
                lines.push(ident.span().start().line);
            }
            TokenTree::Group(group) => collect(group.stream(), lines),
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn real_tokens_inside_macros_count_but_lookalikes_do_not() {
        let source = r#"const TEXT: &str = "unsafe";
// unsafe in prose
#[expect(unsafe_code)]
macro_rules! demo {
    () => { unsafe { call() } };
}
unsafe extern "C" { fn call(); }
"#;
        assert_eq!(unsafe_lines(source, "fixture.rs").unwrap(), [5, 7]);
    }

    #[test]
    fn the_gate_rejects_a_fixture_and_its_scope_is_explicit() {
        let root = crate::root::fixture::relocated("unsafe-gate");
        crate::root::fixture::write(&root, "tools/k/src/a.rs", "pub fn clear() {}\n");
        crate::root::fixture::write(&root, "tools/k/src/b.rs", "pub fn old() { unsafe {} }\n");
        assert!(check(&root).unwrap().len() == 1);
        let scope = crate::source::Scope::new(&["tools/k/src/a.rs".to_owned()]).unwrap();
        assert!(check_scoped(&root, &scope).unwrap().is_empty());
        crate::root::fixture::write(
            &root,
            "tools/k/src/a.rs",
            "pub fn blocked() { unsafe {} }\n",
        );
        assert_eq!(check_scoped(&root, &scope).unwrap().len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
