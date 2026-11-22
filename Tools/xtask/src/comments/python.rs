// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::Command;

use crate::report::XtaskError;

const DOCUMENTATION: &str = r#"
import ast, json, pathlib, sys
path = pathlib.Path(sys.argv[1])
try:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
except (SyntaxError, UnicodeError) as err:
    print(err, file=sys.stderr)
    sys.exit(2)
found = []
for node in ast.walk(tree):
    if not isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
        continue
    if not node.body:
        continue
    first = node.body[0]
    if (isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)):
        found.append([first.lineno, first.end_lineno])
print(json.dumps(found))
"#;

#[expect(
    clippy::disallowed_methods,
    reason = "Python's own AST identifies documentation strings"
)]
pub(super) fn documentation_lines(file: &Path, rel: &str) -> Result<usize, XtaskError> {
    let output = Command::new("python")
        .arg("-c")
        .arg(DOCUMENTATION)
        .arg(file)
        .output()
        .map_err(|source| XtaskError::Io {
            path: "python -c ast".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(XtaskError::Doc {
            file: rel.to_owned(),
            msg: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let ranges: Vec<[usize; 2]> =
        serde_json::from_slice(&output.stdout).map_err(|err| XtaskError::Doc {
            file: rel.to_owned(),
            msg: format!("Python AST returned invalid line ranges: {err}"),
        })?;
    ranges
        .into_iter()
        .try_fold(0_usize, |total, [start, end]| {
            end.checked_sub(start)?.checked_add(1)?.checked_add(total)
        })
        .ok_or_else(|| XtaskError::Doc {
            file: rel.to_owned(),
            msg: "Python AST returned an inverted or overflowing line range".to_owned(),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn documentation_strings_count_but_assigned_and_returned_strings_do_not() {
        let root = crate::root::fixture::relocated("python-documentation");
        let rel = "tools/k/src/documented.py";
        let source = r#""""module
span
"""
value = """data"""
class C:
    """class"""
    def f(self):
        """function"""
        return """data"""
"#;
        crate::root::fixture::write(&root, rel, source);
        assert_eq!(documentation_lines(&root.join(rel), rel).unwrap(), 5);
        assert_eq!(super::super::check(&root).unwrap().len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
