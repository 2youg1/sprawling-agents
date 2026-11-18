// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The assembly point: arguments, environment, stdin/stdout and shutdown
//! (`Mei/Spec.lean` §8-§10).

use std::io::{IsTerminal, Read, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

use mei::error::{Code, Diagnostic, Error, Result};
use mei::{agent, doctor, observation, python, resume, transport, workspace};

const MAX_TOKENS: u64 = 65536;
const DEFAULT_EFFORT: &str = "high";
const SYSTEM: &str = include_str!("MEI.md");
const DEFAULT_BASE: &str = "http://127.0.0.1:3425";

fn main() -> ExitCode {
    match run() {
        Ok(answer) => {
            let mut out = std::io::stdout();
            if let Err(err) = out.write_all(answer.as_bytes()).and_then(|()| out.flush()) {
                eprintln!("write the final answer: {err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String> {
    let arguments = arguments()?;
    let prompt = prompt(arguments.prompt)?;
    let history = arguments.resume.as_deref().map(resume::read).transpose()?;
    let configured_model = optional_environment("HARNESS_MODEL")?;
    let now = current_unix_seconds()?;
    let base = environment_or("ANTHROPIC_BASE_URL", DEFAULT_BASE)?;
    let key = environment_or("ANTHROPIC_API_KEY", "magpie")?;
    let mut client = transport::Client::new(&base, key)?;
    let (model, discovered) = match configured_model {
        Some(model) => {
            if model.is_empty() {
                return Err(Error::Diagnostic(Diagnostic::new(
                    Code::Config,
                    "read HARNESS_MODEL",
                    "the variable must not be empty",
                    "set HARNESS_MODEL to magpie's provider/model spelling, or unset it to use Magpie's preferred route",
                )));
            }
            (model, None)
        }
        None => {
            let selection = client.discover(now)?;
            (selection.model.clone(), Some(selection))
        }
    };
    let extra = parse_extra(optional_environment("HARNESS_EXTRA")?.as_deref())?;
    let guide = match history {
        Some(_) => None,
        None => workspace::read_new_session()?,
    };
    let task = Task {
        model: &model,
        prompt: &prompt,
        history,
        guide: guide.as_deref(),
    };
    let environment = doctor::ensure(now)?;
    let mut request = initial_request(task, extra, &environment)?;
    let sent = request
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Config,
                    "read the model the request carries",
                    "HARNESS_EXTRA replaced `model` with a value that is not a string",
                    "keep HARNESS_EXTRA's `model` a string, or omit it to use the selected model",
                )
                .with_path("request.model"),
            )
        })?;
    let (context_tokens, efforts) = match &discovered {
        Some(selection) if selection.model == sent => {
            (selection.context_tokens, selection.efforts.clone())
        }
        _ => (None, Vec::new()),
    };
    let effort = request
        .pointer("/output_config/effort")
        .and_then(Value::as_str)
        .unwrap_or("default");
    let mut trace = std::io::stderr();
    observation::write(
        &mut trace,
        &json!({
            "type": "mei.obs",
            "kind": "model",
            "model": sent,
            "context_tokens": context_tokens,
            "efforts": efforts,
            "effort": effort,
            "effort_source": "requested",
        }),
        "trace the selected Mei model",
    )?;
    let mut python = python::Py::spawn_in(&environment)?;
    let result = agent::run(
        &mut request,
        |request, pulse| client.post(request, pulse),
        |code| python.run(code),
        &mut trace,
    );
    match result {
        Ok(answer) => {
            python.close()?;
            Ok(answer)
        }
        Err(error) => {
            if let Err(cleanup) = python.kill_and_wait() {
                eprintln!("{cleanup}");
            }
            Err(error)
        }
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "the assembly samples wall time once and passes it into doctor policy"
)]
fn current_unix_seconds() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Doctor,
                "read time for Python support",
                err.to_string(),
                "keep the system clock at or after the Unix epoch",
            ))
        })
}

fn optional_environment(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(Error::Diagnostic(Diagnostic::new(
            Code::Config,
            format!("read {name}"),
            "the environment value is not Unicode",
            format!("set {name} to valid UTF-8 or leave it unset"),
        ))),
    }
}

fn environment_or(name: &str, default: &str) -> Result<String> {
    Ok(optional_environment(name)?.unwrap_or_else(|| default.to_owned()))
}

struct Arguments {
    prompt: Option<String>,
    resume: Option<std::path::PathBuf>,
}

fn arguments() -> Result<Arguments> {
    let mut prompt = None;
    let mut resume = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| usage(format!("{} requires a value", flag.to_string_lossy())))?;
        match flag.to_str() {
            Some("-p") if prompt.is_none() => {
                let value = value.into_string().map_err(|value| {
                    usage(format!(
                        "-p prompt must be valid UTF-8, got `{}`",
                        value.to_string_lossy()
                    ))
                })?;
                prompt = Some(value);
            }
            Some("--resume") if resume.is_none() => resume = Some(value.into()),
            Some("-p" | "--resume") => {
                return Err(usage(format!("duplicate {}", flag.to_string_lossy())));
            }
            _ => {
                return Err(usage(format!(
                    "unknown argument `{}`",
                    flag.to_string_lossy()
                )));
            }
        }
    }
    Ok(Arguments { prompt, resume })
}

fn usage(subject: String) -> Error {
    Diagnostic::new(
        Code::Usage,
        "parse the command line",
        subject,
        "use `mei [-p PROMPT] [--resume JSONL]`; without -p, pipe a complete prompt on stdin",
    )
    .into()
}

fn prompt(argument: Option<String>) -> Result<String> {
    if let Some(prompt) = argument {
        return Ok(prompt);
    }
    if std::io::stdin().is_terminal() {
        return Err(usage(
            "without -p, stdin must be a complete non-terminal prompt".to_owned(),
        ));
    }
    let mut prompt = String::new();
    std::io::stdin()
        .read_to_string(&mut prompt)
        .map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Usage,
                "read the prompt from stdin",
                err.to_string(),
                "use `mei -p PROMPT` or pipe a complete prompt on stdin",
            ))
        })?;
    Ok(prompt)
}

struct Task<'a> {
    model: &'a str,
    prompt: &'a str,
    history: Option<Vec<Value>>,
    guide: Option<&'a str>,
}

fn initial_request(
    task: Task,
    extra: Option<serde_json::Map<String, Value>>,
    environment: &doctor::Environment,
) -> Result<Value> {
    let Task {
        model,
        prompt,
        history,
        guide,
    } = task;
    let opening = match history {
        Some(_) => Value::String(prompt.to_owned()),
        None => workspace::opening_content(guide, prompt),
    };
    let mut request = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "thinking": {"type": "adaptive"},
        "output_config": {"effort": DEFAULT_EFFORT},
        "system": SYSTEM,
        "tools": [python_tool(environment)],
        "messages": [{"role": "user", "content": prompt}],
    });
    if let Some(fields) = extra {
        let target = request.as_object_mut().ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "construct the Messages request",
                "the base request is not an object",
                "keep the base request as a JSON object",
            ))
        })?;
        for (key, value) in fields {
            target.insert(key, value);
        }
    }
    request
        .as_object_mut()
        .ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "construct the Messages request",
                "the base request is not an object",
                "keep the base request as a JSON object",
            ))
        })?
        .insert(
            "messages".to_owned(),
            Value::Array(
                history
                    .unwrap_or_default()
                    .into_iter()
                    .chain([json!({"role": "user", "content": opening})])
                    .collect(),
            ),
        );
    Ok(request)
}

fn parse_extra(extra: Option<&str>) -> Result<Option<serde_json::Map<String, Value>>> {
    let Some(extra) = extra else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(extra).map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "parse HARNESS_EXTRA",
            err.to_string(),
            "set HARNESS_EXTRA to one JSON object; messages are supplied by the prompt",
        ))
    })?;
    let fields = value.as_object().ok_or_else(|| {
        Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "merge HARNESS_EXTRA",
            "the value is not an object",
            "set HARNESS_EXTRA to one JSON object",
        ))
    })?;
    if fields.contains_key("tools") {
        return Err(Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "merge HARNESS_EXTRA",
            "tools would replace the sole local python tool",
            "keep tools out of HARNESS_EXTRA; Mei declares only python",
        )));
    }
    if fields.get("stream") == Some(&Value::Bool(true)) {
        return Err(Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "merge HARNESS_EXTRA",
            "stream=true requires SSE, which Mei does not implement",
            "remove stream or set it to false; Mei waits for complete HTTP responses",
        )));
    }
    Ok(Some(fields.clone()))
}

fn python_tool(environment: &doctor::Environment) -> Value {
    json!({
        "name": agent::TOOL_NAME,
        "description": python::tool_description(environment),
        "input_schema": {
            "type": "object",
            "properties": {"code": {"type": "string"}},
            "required": ["code"]
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    fn environment() -> doctor::Environment {
        doctor::Environment::new(
            doctor::Program::new("3.12.13", std::path::PathBuf::from("python")),
            doctor::Program::new("0.12.3", std::path::PathBuf::from("uv")),
        )
    }

    fn request(task: Task, extra: Option<&str>) -> Value {
        initial_request(task, parse_extra(extra).unwrap(), &environment()).unwrap()
    }

    #[test]
    fn the_request_has_one_state_and_the_prompt_owns_messages() {
        let request = request(
            Task {
                model: "provider/model",
                prompt: "new prompt",
                history: None,
                guide: None,
            },
            Some(
                r#"{"system":"custom","messages":[{"role":"user","content":"old"}],"max_tokens":42}"#,
            ),
        );
        assert_eq!(request["model"], "provider/model");
        assert_eq!(request["system"], "custom");
        assert_eq!(request["max_tokens"], 42);
        assert_eq!(request["messages"][0]["content"], "new prompt");
        assert_eq!(request["tools"].as_array().unwrap().len(), 1);
        assert_eq!(request["tools"][0]["name"], "python");
        assert_eq!(
            request["tools"][0]["description"],
            python::tool_description(&environment())
        );
        assert!(request["tools"][0].get("strict").is_none());
    }

    #[test]
    fn resumed_messages_precede_the_new_prompt() {
        let request = request(
            Task {
                model: "provider/model",
                prompt: "new",
                history: Some(vec![
                    json!({"role": "user", "content": "old"}),
                    json!({"role": "assistant", "content": "answer"}),
                ]),
                guide: Some("must-not-be-injected"),
            },
            None,
        );
        assert_eq!(
            request["messages"],
            json!([
                {"role": "user", "content": "old"},
                {"role": "assistant", "content": "answer"},
                {"role": "user", "content": "new"}
            ])
        );
    }

    #[test]
    fn readmes_match_the_documented_request_fixture() {
        let tool = python_tool(&environment());
        let pretty = serde_json::to_string_pretty(&tool).unwrap();
        let tool_bytes = serde_json::to_vec(&tool).unwrap().len();
        let total = SYSTEM.len().checked_add(tool_bytes).unwrap();
        for readme in [
            include_str!("../README.md"),
            include_str!("../README.zh-CN.md"),
        ] {
            assert!(readme.contains(SYSTEM));
            assert!(readme.contains(&pretty));
            for bytes in [SYSTEM.len(), tool_bytes, total] {
                assert!(readme.contains(&format!("| {bytes} |")));
            }
        }
    }

    #[test]
    fn forbidden_configuration_is_refused_before_any_side_effect() {
        assert!(parse_extra(Some("[]")).is_err());
        assert!(parse_extra(Some(r#"{"stream":true}"#)).is_err());
        assert!(parse_extra(Some(r#"{"tools":[]}"#)).is_err());
    }
}
