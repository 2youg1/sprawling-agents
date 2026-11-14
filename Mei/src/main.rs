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
use mei::{agent, python, transport};

const MAX_TOKENS: u64 = 16000;
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
    let prompt = prompt()?;
    let model = optional_environment("HARNESS_MODEL")?.ok_or_else(|| {
        Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "read HARNESS_MODEL",
            "the variable is required and must not be empty",
            "set HARNESS_MODEL to magpie's provider/model spelling",
        ))
    })?;
    if model.is_empty() {
        return Err(Error::Diagnostic(Diagnostic::new(
            Code::Config,
            "read HARNESS_MODEL",
            "the variable must not be empty",
            "set HARNESS_MODEL to magpie's provider/model spelling",
        )));
    }
    let extra = optional_environment("HARNESS_EXTRA")?;
    let tools = optional_environment("MEI_TOOLS")?;
    let mut request = initial_request(&model, &prompt, extra.as_deref(), tools.as_deref())?;
    let base = environment_or("ANTHROPIC_BASE_URL", DEFAULT_BASE)?;
    let key = environment_or("ANTHROPIC_API_KEY", "magpie")?;
    let client = transport::Client::new(&base, key)?;
    let mut python = python::Py::spawn()?;
    let result = agent::run(
        &mut request,
        |request| client.post(request),
        |code| python.run(code),
        &mut std::io::stderr(),
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

fn prompt() -> Result<String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [flag, prompt] if flag == "-p" => Ok(prompt.clone()),
        [] if !std::io::stdin().is_terminal() => {
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
        _ => Err(Error::Diagnostic(Diagnostic::new(
            Code::Usage,
            "parse the command line",
            "expected `mei -p PROMPT` or a non-terminal stdin prompt",
            "use `mei -p PROMPT`; without -p, pipe input instead of leaving stdin on a terminal",
        ))),
    }
}

fn initial_request(
    model: &str,
    prompt: &str,
    extra: Option<&str>,
    server_tools: Option<&str>,
) -> Result<Value> {
    let mut request = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": SYSTEM,
        "tools": [python_tool()],
        "messages": [{"role": "user", "content": prompt}],
    });
    if let Some(extra) = extra {
        let extra: Value = serde_json::from_str(extra).map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "parse HARNESS_EXTRA",
                err.to_string(),
                "set HARNESS_EXTRA to one JSON object; messages are supplied by the prompt",
            ))
        })?;
        let fields = extra.as_object().ok_or_else(|| {
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
                "tools would replace the sole local Python tool",
                "put transparent server-side declarations in MEI_TOOLS, not HARNESS_EXTRA.tools",
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
        let target = request.as_object_mut().ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "construct the Messages request",
                "the base request is not an object",
                "keep the base request as a JSON object",
            ))
        })?;
        for (key, value) in fields {
            target.insert(key.clone(), value.clone());
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
            json!([{"role": "user", "content": prompt}]),
        );
    if let Some(server_tools) = server_tools {
        let additional: Value = serde_json::from_str(server_tools).map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "parse MEI_TOOLS",
                err.to_string(),
                "set MEI_TOOLS to a JSON array of transparent server-side tool declarations",
            ))
        })?;
        let additional = additional.as_array().ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Code::Config,
                "append MEI_TOOLS",
                "the value is not an array",
                "set MEI_TOOLS to a JSON array; the local Python declaration stays first",
            ))
        })?;
        let tools = request
            .get_mut("tools")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| {
                Error::Diagnostic(Diagnostic::new(
                    Code::Config,
                    "append MEI_TOOLS",
                    "tools is not an array",
                    "keep the local Python declaration first in tools",
                ))
            })?;
        tools.extend(additional.iter().cloned());
    }
    Ok(request)
}

fn python_tool() -> Value {
    json!({
        "name": "python",
        "description": "A persistent Python 3 interpreter. Variables survive across calls. Returns stdout+stderr. Output over 16000 chars is truncated and the full text is saved to a file named in the result. Each call has a 250 s best-effort timeout. Use subprocess for shell commands and print for expression results.",
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

    #[test]
    fn the_request_has_one_state_and_the_prompt_owns_messages() {
        let request = initial_request(
            "provider/model",
            "new prompt",
            Some(r#"{"system":"custom","messages":[{"role":"user","content":"old"}],"max_tokens":42}"#),
            Some(r#"[{"type":"web_search_20250305","name":"web_search","max_uses":2}]"#),
        ).unwrap();
        assert_eq!(request["model"], "provider/model");
        assert_eq!(request["system"], "custom");
        assert_eq!(request["max_tokens"], 42);
        assert_eq!(request["messages"][0]["content"], "new prompt");
        assert_eq!(request["tools"].as_array().unwrap().len(), 2);
        assert_eq!(request["tools"][0]["name"], "python");
        assert_eq!(request["tools"][1]["name"], "web_search");
    }

    #[test]
    fn forbidden_configuration_is_refused_before_any_side_effect() {
        assert!(initial_request("m", "p", Some("[]"), None).is_err());
        assert!(initial_request("m", "p", Some(r#"{"stream":true}"#), None).is_err());
        assert!(initial_request("m", "p", Some(r#"{"tools":[]}"#), None).is_err());
        assert!(initial_request("m", "p", None, Some("{}")).is_err());
    }
}
