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

pub(super) fn documentation_lines(file: &Path, rel: &str) -> Result<usize, XtaskError> {
    let interpreter = interpreter(file)?;
    parse_with(&interpreter, file, rel)
}

#[expect(
    clippy::disallowed_methods,
    reason = "doctor and the comments gate share uv's interpreter resolution"
)]
fn interpreter(file: &Path) -> Result<std::path::PathBuf, XtaskError> {
    let output = Command::new("uv")
        .args(["python", "find"])
        .output()
        .map_err(|source| {
            process_failure(
                file,
                Failure {
                    code: "python-resolver-unavailable",
                    action: "resolve the Python interpreter for the comments gate",
                    value: "uv python find",
                    error: &source.to_string(),
                },
            )
        })?;
    if !output.status.success() {
        return Err(process_failure(
            file,
            Failure {
                code: "python-resolution-failed",
                action: "resolve the Python interpreter for the comments gate",
                value: "uv python find",
                error: &String::from_utf8_lossy(&output.stderr),
            },
        ));
    }
    let path = String::from_utf8(output.stdout).map_err(|error| {
        process_failure(
            file,
            Failure {
                code: "python-path-not-utf8",
                action: "read the interpreter selected by uv",
                value: "uv python find standard output",
                error: &error.to_string(),
            },
        )
    })?;
    let path = path.trim();
    if path.is_empty() {
        return Err(process_failure(
            file,
            Failure {
                code: "python-path-empty",
                action: "read the interpreter selected by uv",
                value: "uv python find standard output",
                error: "the command succeeded without printing a path",
            },
        ));
    }
    Ok(std::path::PathBuf::from(path))
}

#[expect(
    clippy::disallowed_methods,
    reason = "Python's own AST identifies documentation strings"
)]
fn parse_with(interpreter: &Path, file: &Path, rel: &str) -> Result<usize, XtaskError> {
    let output = Command::new(interpreter)
        .arg("-c")
        .arg(DOCUMENTATION)
        .arg(file)
        .output()
        .map_err(|source| {
            process_failure(
                file,
                Failure {
                    code: "python-launch-failed",
                    action: "start Python to identify documentation strings",
                    value: &interpreter.display().to_string(),
                    error: &source.to_string(),
                },
            )
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

struct Failure<'a> {
    code: &'a str,
    action: &'a str,
    value: &'a str,
    error: &'a str,
}

fn process_failure(file: &Path, failure: Failure<'_>) -> XtaskError {
    let path = file.display().to_string();
    let fields = [
        ("code", failure.code),
        ("action", failure.action),
        ("subject", "comments gate Python AST"),
        ("path", &path),
        ("value", failure.value),
        (
            "rule",
            "the gate must use the interpreter selected by uv python find",
        ),
        (
            "repair",
            "Run just prereqs and follow its uv installation line if needed; run uv python install, confirm uv python find prints a path, then rerun cargo xtask gates comments.",
        ),
        ("error", failure.error),
    ];
    let mut msg = "```toml\n".to_owned();
    for (name, value) in fields {
        msg.push_str(&format!(
            "{name} = {}\n",
            toml::Value::String(value.to_owned())
        ));
    }
    msg.push_str("```");
    XtaskError::Doc { file: path, msg }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_missing_interpreter_names_the_file_and_the_uv_repair() {
        let root = crate::root::fixture::relocated("python-missing-interpreter");
        let rel = "tools/k/src/documented.py";
        crate::root::fixture::write(&root, rel, "value = 1\n");
        let missing = root.join("missing-python");
        let error = parse_with(&missing, &root.join(rel), rel).unwrap_err();
        let text = error.to_string();
        for expected in [
            "python-launch-failed",
            &root.join(rel).display().to_string(),
            "uv python install",
            "uv python find",
        ] {
            assert!(text.contains(expected), "{text}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

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
