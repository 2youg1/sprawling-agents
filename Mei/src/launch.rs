// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::{Value, json};

use crate::agent;
use crate::doctor;
use crate::error::{Code, Diagnostic, Error, Result};
use crate::python;

pub const MAX_TOKENS: u64 = 256_000;
pub const DEFAULT_EFFORT: &str = "high";
pub const SYSTEM: &str = include_str!("MEI.md");
pub const DEFAULT_BASE: &str = "http://127.0.0.1:3425";

pub struct Task<'a> {
    pub model: &'a str,
    pub prompt: &'a str,
    pub history: Option<Vec<Value>>,
    pub guide: Option<&'a str>,
}

pub fn current_unix_seconds() -> Result<u64> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the assembly samples wall time once and passes it into doctor policy"
    )]
    {
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
}

pub fn optional_environment(name: &str) -> Result<Option<String>> {
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

pub fn environment_or(name: &str, default: &str) -> Result<String> {
    Ok(optional_environment(name)?.unwrap_or_else(|| default.to_owned()))
}

pub fn parse_extra(extra: Option<&str>) -> Result<Option<serde_json::Map<String, Value>>> {
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

pub fn python_tool(environment: &doctor::Environment) -> Value {
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

pub fn request_settings(
    model: &str,
    extra: Option<serde_json::Map<String, Value>>,
    environment: &doctor::Environment,
) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::from_iter([
        ("model".to_owned(), Value::from(model)),
        ("max_tokens".to_owned(), Value::from(MAX_TOKENS)),
        ("thinking".to_owned(), json!({"type": "adaptive"})),
        (
            "output_config".to_owned(),
            json!({"effort": DEFAULT_EFFORT}),
        ),
        ("system".to_owned(), Value::from(SYSTEM)),
        (
            "tools".to_owned(),
            Value::Array(vec![python_tool(environment)]),
        ),
    ]);
    fields.extend(extra.into_iter().flatten());
    fields.remove("messages");
    fields
}

pub fn initial_request(
    task: Task<'_>,
    extra: Option<serde_json::Map<String, Value>>,
    environment: &doctor::Environment,
) -> Result<Value> {
    let mut fields = request_settings(task.model, extra, environment);
    let opening = match &task.history {
        Some(_) => Value::from(task.prompt),
        None => crate::workspace::opening_content(task.guide, task.prompt),
    };
    let mut messages = task.history.unwrap_or_default();
    messages.push(json!({"role": "user", "content": opening}));
    fields.insert("messages".to_owned(), Value::Array(messages));
    Ok(Value::Object(fields))
}

pub fn resolve_model(
    client: &mut crate::transport::Client,
    now: u64,
) -> Result<(String, Option<crate::catalog::Selection>)> {
    match optional_environment("HARNESS_MODEL")? {
        Some(model) => {
            if model.is_empty() {
                return Err(Error::Diagnostic(Diagnostic::new(
                    Code::Config,
                    "read HARNESS_MODEL",
                    "the variable must not be empty",
                    "set HARNESS_MODEL to magpie's provider/model spelling, or unset it to use Magpie's preferred route",
                )));
            }
            Ok((model, None))
        }
        None => {
            let selection = client.discover(now)?;
            Ok((selection.model.clone(), Some(selection)))
        }
    }
}

pub fn bind_python_to_cwd(
    python: &mut Option<crate::python::Py>,
    bound: &mut Option<std::path::PathBuf>,
    cwd: &std::path::Path,
    environment: &doctor::Environment,
) -> Result<()> {
    if bound.as_deref() != Some(cwd) {
        if let Some(mut previous) = python.take() {
            previous.kill_and_wait()?;
        }
        crate::acp::enter_cwd(cwd)?;
        *bound = Some(cwd.to_path_buf());
    }
    if python.is_none() {
        *python = Some(crate::python::Py::prepare_in(environment)?);
    }
    Ok(())
}
