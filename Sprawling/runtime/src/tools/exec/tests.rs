// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::Address;

mod container;
#[cfg(target_os = "macos")]
#[path = "native_macos/production_tests.rs"]
mod native_macos;
mod shell;
mod yielding;

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "c1".to_owned(),
        name: ToolName::parse("exec").unwrap(),
        args: Payload::of(&args).unwrap(),
    }
}

fn call_at(mut args: Value, placement: &str) -> ToolCall {
    args["where"] = Value::String(placement.to_owned());
    call(args)
}

fn command(text: &str) -> Value {
    serde_json::json!({ "command": text })
}

fn python(code: &str) -> Value {
    serde_json::json!({ "python": code })
}

fn line_of<S: AsRef<str>>(path: impl AsRef<str>, args: impl IntoIterator<Item = S>) -> Value {
    let quoted = |word: &str| {
        if word.contains('"') {
            format!("'{word}'")
        } else {
            format!("\"{word}\"")
        }
    };
    let line = std::iter::once(quoted(path.as_ref()))
        .chain(args.into_iter().map(|arg| quoted(arg.as_ref())))
        .collect::<Vec<_>>()
        .join(" ");
    command(&line)
}

#[cfg(unix)]
const NO_ARGS: &[&str] = &[];

fn setup(workdir: &std::path::Path, python: Option<PathBuf>, shell: Option<PathBuf>) -> ExecSetup {
    ExecSetup {
        workdir: workdir.to_path_buf(),
        python,
        shell: shell.map_or(Shell::Absent, |program| Shell::Found {
            program,
            interpreter: kernel::Interpreter::System,
        }),
        env_passthrough: Vec::new(),
        domain: Address::parse("work").unwrap(),
        run: kernel::RunId::from_bytes([1; 16]),
        policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
    }
}

fn a_tool(workdir: &std::path::Path, python: Option<PathBuf>, shell: Option<PathBuf>) -> ExecTool {
    regression_boundary(ExecTool::new(setup(workdir, python, shell), patient()).unwrap())
}

fn regression_boundary(tool: ExecTool) -> ExecTool {
    #[cfg(windows)]
    if std::env::var("SPRAWLING_DISPOSABLE_NATIVE").as_deref() == Ok("1") {
        let tool = tool.confined(Confined::with_arm(
            Confinement::WindowsJobObject,
            Some(std::env::temp_dir()),
        ));
        assert!(tool.meta().disclosure.contains("windows_job_object"));
        eprintln!("NATIVE_REGRESSION_ARM=windows_job_object");
        return tool;
    }
    tool
}

fn patient() -> Backlog {
    Backlog::with_window(crate::backlog::PollBudget::new(6_000, 20))
}

fn a_name_this_machine_sets() -> (String, String) {
    let mut candidates: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for (name, value) in std::env::vars() {
        let admissible = !ENV_ALLOWLIST.contains(&name.as_str())
            && kernel::EnvVarName::parse(&name).is_ok()
            && !value.is_empty()
            && value.is_ascii()
            && !value.contains('%')
            && !value.contains('"');
        if admissible {
            candidates.insert(name, value);
        }
    }
    candidates
        .into_iter()
        .next()
        .expect("every supported host sets at least one ordinary variable")
}

fn reads_the_variable(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec!["/C".to_owned(), format!("echo %{name}%")],
        )
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), format!("printf %s \"${name}\"")],
        )
    }
}

#[test]
fn a_child_sees_the_names_its_building_declared_and_no_others() {
    let (name, value) = a_name_this_machine_sets();
    let (path, args) = reads_the_variable(&name);

    let chamber = tempfile::tempdir().unwrap();
    let declared = ExecSetup {
        env_passthrough: vec![kernel::EnvVarName::parse(&name).unwrap()],
        ..setup(chamber.path(), None, None)
    };
    let tool = regression_boundary(ExecTool::new(declared, patient()).unwrap());
    let outcome = tool
        .invoke(&call(line_of(path.clone(), args.clone())))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert!(
        result["stdout"].as_str().unwrap().contains(&value),
        "a declared name reaches the child: {result}"
    );
    let listed: Vec<String> = serde_json::from_value(result["env"].clone()).unwrap();
    assert!(
        listed.contains(&name),
        "the ledger says what it inherited: {result}"
    );

    let bare_chamber = tempfile::tempdir().unwrap();
    let bare = a_tool(bare_chamber.path(), None, None);
    let outcome = bare.invoke(&call(line_of(path, args))).unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert!(
        !result["stdout"].as_str().unwrap().contains(&value),
        "an undeclared name must not reach the child: {result}"
    );
    let listed: Vec<String> = serde_json::from_value(result["env"].clone()).unwrap();
    assert!(!listed.contains(&name), "{result}");
}

#[test]
fn a_missing_component_refuses_and_names_the_alternative() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), None, None);
    let err = match tool.invoke(&call(python("print(1)"))) {
        Err(err) => err,
        Ok(_) => panic!("a missing component must refuse"),
    };
    assert_eq!(*err.code(), AxCode::ToolUnavailable);
    assert!(
        err.recovery().contains("`command`"),
        "the refusal carries the alternative"
    );
}

#[test]
fn without_a_shell_a_command_runs_its_program_and_refuses_shell_syntax() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), None, None);
    let line = if cfg!(windows) {
        r#"cmd /C "exit 3""#
    } else {
        r#"sh -c "exit 3""#
    };
    let ran = serde_json::to_value(tool.invoke(&call(command(line))).unwrap().result).unwrap();
    let Err(refused) = tool.invoke(&call(command("echo hi | sort"))) else {
        panic!("a pipe without a shell must refuse");
    };
    assert_eq!(
        (
            ran["exit_code"].clone(),
            ran["arm"].clone(),
            *refused.code(),
            refused.subject().to_owned(),
        ),
        (
            serde_json::json!(3),
            serde_json::json!("program"),
            AxCode::InvalidArgs,
            "`|` outside quotes".to_owned(),
        )
    );
}

#[test]
fn a_call_names_a_command_or_a_script_and_nothing_else() {
    let read = |args: Value| {
        parse_arm(args.as_object().unwrap()).map_err(|err| (*err.code(), err.subject().to_owned()))
    };
    assert_eq!(
        [
            read(command("cargo test")),
            read(python("print(1)")),
            read(serde_json::json!({ "arm": { "shell": { "text": "ls" } } })),
            read(serde_json::json!({ "command": "ls", "python": "print(1)" })),
            read(serde_json::json!({ "command": "  " })),
        ],
        [
            Ok(ExecArm::Shell {
                text: "cargo test".to_owned()
            }),
            Ok(ExecArm::Python {
                code: "print(1)".to_owned()
            }),
            Err((
                AxCode::InvalidArgs,
                "`arm` is how an older build spelled this call".to_owned()
            )),
            Err((
                AxCode::InvalidArgs,
                "both `command` and `python`".to_owned()
            )),
            Err((
                AxCode::InvalidArgs,
                "`command` is not a string with something in it".to_owned()
            )),
        ]
    );
}

#[test]
fn an_unrecognised_arm_is_refused_not_guessed() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), None, None);
    let err = match tool.invoke(&call(serde_json::json!({ "bash": "ls" }))) {
        Err(err) => err,
        Ok(_) => panic!("an unknown arm must refuse"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

#[test]
fn the_program_arm_runs_a_real_child_with_a_scrubbed_environment() {
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), None, None);

    let (path, args) = if cfg!(windows) {
        ("cmd", vec!["/C".to_owned(), "exit 3".to_owned()])
    } else {
        ("sh", vec!["-c".to_owned(), "exit 3".to_owned()])
    };
    let outcome = tool.invoke(&call(line_of(path, args))).unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["exit_code"], 3, "{result}");
    assert_eq!(result["arm"], "program");
}

fn writes_beside_itself(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec![
                "/C".to_owned(),
                format!("echo written> {name} && type {name}"),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                format!("echo written > {name}; cat {name}"),
            ],
        )
    }
}

#[test]
fn a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone() {
    let chamber = tempfile::tempdir().unwrap();
    std::fs::write(chamber.path().join("kept.txt"), b"the person's own\n").unwrap();
    let tool = a_tool(chamber.path(), None, None);
    let (path, args) = writes_beside_itself("made.txt");
    let outcome = tool.invoke(&call(line_of(path, args))).unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["exit_code"], 0, "{result}");
    assert!(
        result["stdout"].as_str().unwrap().contains("written"),
        "the command wrote where it ran: {result}"
    );
    assert!(
        !chamber.path().join("made.txt").exists(),
        "a sandboxed write must not land in the tree the person has"
    );
    assert_eq!(
        std::fs::read_to_string(chamber.path().join("kept.txt")).unwrap(),
        "the person's own\n",
        "the tree it read is the tree the person has"
    );

    let host = a_tool(chamber.path(), None, None);
    let (path, args) = writes_beside_itself("made.txt");
    host.invoke(&call_at(line_of(path, args), "host")).unwrap();
    assert!(
        chamber.path().join("made.txt").exists(),
        "the host placement is the one a person decides on, and it is the one that writes"
    );
}

#[test]
fn the_python_arm_runs_on_the_host_interpreter_when_asked() {
    let Some(interpreter) = host_python() else {
        eprintln!("skip: no Python interpreter on PATH or SPRAWLING_PYTHON");
        return;
    };
    let chamber = tempfile::tempdir().unwrap();
    let tool = a_tool(chamber.path(), Some(interpreter), None);
    let outcome = tool.invoke(&call_at(python("print(7)"), "host")).unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(result["exit_code"], 0, "{result}");
    assert!(result["stdout"].as_str().unwrap().contains('7'), "{result}");
    assert_eq!(result["arm"], "python");
}

fn host_python() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("SPRAWLING_PYTHON")
        && !named.is_empty()
    {
        let path = PathBuf::from(&named);
        if path.is_file() {
            return Some(path);
        }
        let search = std::env::var_os("PATH").unwrap_or_default();
        return std::env::split_paths(&search).find_map(|dir| {
            let candidate = dir.join(&named);
            candidate.is_file().then_some(candidate)
        });
    }
    let search = std::env::var_os("PATH").unwrap_or_default();
    let names: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe", "python", "python3"]
    } else {
        &["python3", "python"]
    };
    names.iter().find_map(|name| {
        std::env::split_paths(&search).find_map(|dir| {
            let candidate = dir.join(name);
            candidate.is_file().then_some(candidate)
        })
    })
}
