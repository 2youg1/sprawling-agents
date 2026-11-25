// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{IsTerminal, Read, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

use mei::acp::{self, Outcome, StopReason, Turn};
use mei::error::{Code, Diagnostic, Error, Result};
use mei::launch::{self, Task};
use mei::{agent, doctor, observation, python, resume, steer, transport, workspace};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let arguments = arguments()?;
    if arguments.acp {
        return run_acp();
    }
    if arguments.describe {
        return describe_model();
    }
    let answer = run_headless(arguments)?;
    let mut out = std::io::stdout();
    out.write_all(answer.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Usage,
                "write the final answer",
                err.to_string(),
                "keep stdout open for the headless answer",
            ))
        })
}

struct Arguments {
    acp: bool,
    describe: bool,
    prompt: Option<String>,
    resume: Option<std::path::PathBuf>,
}

fn arguments() -> Result<Arguments> {
    let mut prompt = None;
    let mut resume = None;
    let mut acp = false;
    let mut describe = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--acp") if !acp => {
                acp = true;
            }
            Some("--acp") => return Err(usage("duplicate --acp".to_owned())),
            Some("--describe") if !describe => describe = true,
            Some("--describe") => return Err(usage("duplicate --describe".to_owned())),
            Some("-p") if prompt.is_none() => {
                let value = args
                    .next()
                    .ok_or_else(|| usage("-p requires a value".to_owned()))?;
                let value = value.into_string().map_err(|value| {
                    usage(format!(
                        "-p prompt must be valid UTF-8, got `{}`",
                        value.to_string_lossy()
                    ))
                })?;
                prompt = Some(value);
            }
            Some("--resume") if resume.is_none() => {
                let value = args
                    .next()
                    .ok_or_else(|| usage("--resume requires a value".to_owned()))?;
                resume = Some(value.into());
            }
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
    if acp && (prompt.is_some() || resume.is_some()) {
        return Err(usage(
            "--acp cannot be combined with -p or --resume".to_owned(),
        ));
    }
    if describe && (acp || prompt.is_some() || resume.is_some()) {
        return Err(usage("--describe must be used by itself".to_owned()));
    }
    Ok(Arguments {
        acp,
        describe,
        prompt,
        resume,
    })
}

fn usage(subject: String) -> Error {
    Diagnostic::new(
        Code::Usage,
        "parse the command line",
        subject,
        "use `mei [-p PROMPT] [--resume JSONL]` or `mei --acp`; without -p, pipe a complete prompt on stdin",
    )
    .into()
}

fn interactive_face_hint(subject: String) -> Error {
    Diagnostic::new(
        Code::Usage,
        "open the interactive face",
        subject,
        "this `mei` is the headless core only; install `@sprawling/mei` with Bun \
         (`bun install --global @sprawling/mei@latest`) or run `calyx` from a source \
         build that includes sprawling-calyx",
    )
    .into()
}

fn prompt(argument: Option<String>) -> Result<String> {
    if let Some(prompt) = argument {
        return Ok(prompt);
    }
    if std::io::stdin().is_terminal() {
        return Err(interactive_face_hint(
            "TTY `mei` without `-p` is not an interactive shell; the Calyx face is a separate binary"
                .to_owned(),
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

fn run_headless(arguments: Arguments) -> Result<String> {
    let prompt = prompt(arguments.prompt)?;
    let history = arguments.resume.as_deref().map(resume::read).transpose()?;
    let now = launch::current_unix_seconds()?;
    let base = launch::environment_or("ANTHROPIC_BASE_URL", launch::DEFAULT_BASE)?;
    let key = launch::environment_or("ANTHROPIC_API_KEY", "magpie")?;
    let mut client = transport::Client::new(&base, key)?;
    let (model, discovered) = launch::resolve_model(&mut client, now)?;
    let extra = launch::parse_extra(launch::optional_environment("HARNESS_EXTRA")?.as_deref())?;
    let guide = match history {
        Some(_) => None,
        None => workspace::read_new_session()?,
    };
    if guide.is_some() {
        observation::write(
            &mut std::io::stderr(),
            &json!({"type": "mei.obs", "kind": "workspace", "agents": true}),
            "trace loaded workspace instructions",
        )?;
    }
    let environment = doctor::ensure(now)?;
    let mut request = launch::initial_request(
        Task {
            model: &model,
            prompt: &prompt,
            history,
            guide: guide.as_deref(),
        },
        extra,
        &environment,
    )?;
    trace_model(&request, discovered.as_ref())?;
    let mut python = python::Py::spawn_in(&environment)?;
    let mut intake = match launch::optional_environment("MEI_STEER_PATH")? {
        Some(path) => Some(steer::Intake::open(path)?),
        None => None,
    };
    let mut live = agent::Live {
        client: &mut client,
        python: &mut python,
        intake: &mut intake,
        cancelled: None,
    };
    let mut trace = std::io::stderr();
    let result = agent::drive(&mut request, &mut live, &mut trace);
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

fn run_acp() -> Result<()> {
    let now = launch::current_unix_seconds()?;
    let base = launch::environment_or("ANTHROPIC_BASE_URL", launch::DEFAULT_BASE)?;
    let key = launch::environment_or("ANTHROPIC_API_KEY", "magpie")?;
    let mut client = transport::Client::new(&base, key)?;
    let (model, discovered) = launch::resolve_model(&mut client, now)?;
    let extra = launch::parse_extra(launch::optional_environment("HARNESS_EXTRA")?.as_deref())?;
    let environment = doctor::ensure(now)?;
    let mut python: Option<python::Py> = None;
    let mut request: Option<Value> = None;
    let mut guide_pending = true;
    let mut opened_cwd = None;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let result = acp::serve(stdin, stdout, |turn: Turn| {
        if !turn.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            launch::bind_python_to_cwd(&mut python, &mut opened_cwd, &turn.cwd, &environment)?;
        }
        if turn.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(Outcome {
                stop: StopReason::Cancelled,
                text: String::new(),
            });
        }
        let mut trace = std::io::stderr();
        if turn.first || request.is_none() {
            let guide = if guide_pending {
                workspace::read_new_session()?
            } else {
                None
            };
            guide_pending = false;
            let built = launch::initial_request(
                Task {
                    model: &model,
                    prompt: &turn.prompt,
                    history: None,
                    guide: guide.as_deref(),
                },
                extra.clone(),
                &environment,
            )?;
            request = Some(built);
            if let Some(body) = request.as_ref() {
                trace_model(body, discovered.as_ref())?;
            }
        } else {
            let body = request.as_mut().ok_or_else(|| {
                Error::Diagnostic(Diagnostic::new(
                    Code::Protocol,
                    "continue an ACP session",
                    "no Messages request exists for this session",
                    "open a new ACP session with session/new before another prompt",
                ))
            })?;
            let messages = body
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| {
                    Error::Diagnostic(Diagnostic::new(
                        Code::Protocol,
                        "append an ACP prompt",
                        "the Messages request has no messages array",
                        "restart `mei --acp` and open a new session",
                    ))
                })?;
            messages.push(json!({"role": "user", "content": turn.prompt}));
        }
        let body = request.as_mut().ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Code::Protocol,
                "run an ACP turn",
                "the Messages request is missing",
                "restart `mei --acp` and open a new session",
            ))
        })?;
        let run_result = {
            let interpreter = python.as_mut().ok_or_else(|| {
                Error::Diagnostic(Diagnostic::new(
                    Code::Protocol,
                    "run an ACP turn",
                    "no interpreter is bound to the session cwd",
                    "restart `mei --acp` and open a new session",
                ))
            })?;
            let mut intake = None;
            let mut live = agent::Live {
                client: &mut client,
                python: interpreter,
                intake: &mut intake,
                cancelled: Some(turn.cancelled.as_ref()),
            };
            agent::drive(body, &mut live, &mut trace)
        };
        let answer = match run_result {
            Ok(answer) => answer,
            Err(error) => {
                request = None;
                if let Some(mut dead) = python.take()
                    && let Err(cleanup) = dead.close()
                {
                    eprintln!("{cleanup}");
                }
                return Err(error);
            }
        };
        let stop = if turn.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            StopReason::Cancelled
        } else {
            StopReason::EndTurn
        };
        Ok(Outcome { stop, text: answer })
    });
    match result {
        Ok(()) => {
            if let Some(mut python) = python {
                python.close()?;
            }
            Ok(())
        }
        Err(error) => {
            if let Some(mut python) = python
                && let Err(cleanup) = python.close()
            {
                eprintln!("{cleanup}");
            }
            Err(error)
        }
    }
}

fn describe_model() -> Result<()> {
    let extra = launch::parse_extra(launch::optional_environment("HARNESS_EXTRA")?.as_deref())?;
    let model = launch::optional_environment("HARNESS_MODEL")?;
    let fields = extra.as_ref();
    let requested = fields
        .and_then(|fields| fields.get("model"))
        .and_then(Value::as_str)
        .or(model.as_deref());
    let base = launch::environment_or("ANTHROPIC_BASE_URL", launch::DEFAULT_BASE)?;
    let key = launch::environment_or("ANTHROPIC_API_KEY", "magpie")?;
    let catalog = transport::Client::new(&base, key)?.describe(requested)?;
    let output = fields
        .and_then(|fields| fields.get("output_config"))
        .cloned()
        .unwrap_or_else(|| json!({"effort": launch::DEFAULT_EFFORT}));
    let request = json!({"model": catalog.model, "output_config": output});
    let model = model_observation(&request, Some(&catalog))?;
    observation::write(
        &mut std::io::stdout(),
        &model,
        "describe the requested Mei model",
    )
}

fn trace_model(request: &Value, discovered: Option<&mei::catalog::Selection>) -> Result<()> {
    let model = model_observation(request, discovered)?;
    observation::write(
        &mut std::io::stderr(),
        &model,
        "trace the selected Mei model",
    )
}

fn model_observation(
    request: &Value,
    discovered: Option<&mei::catalog::Selection>,
) -> Result<Value> {
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
    let (context_tokens, efforts) = match discovered {
        Some(selection) if selection.model == sent => {
            (selection.context_tokens, selection.efforts.clone())
        }
        _ => (None, Vec::new()),
    };
    let effort = request
        .pointer("/output_config/effort")
        .and_then(Value::as_str)
        .unwrap_or("default");
    Ok(json!({
            "type": "mei.obs",
            "kind": "model",
            "model": sent,
            "context_tokens": context_tokens,
            "efforts": efforts,
            "effort": effort,
            "effort_source": "requested",
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
#[path = "main_tests.rs"]
mod tests;
