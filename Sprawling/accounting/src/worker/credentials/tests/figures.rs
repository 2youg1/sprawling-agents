// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the model table's placeholders say, against what a pick with
//! empty boxes registers (`crates/wire/spec/Answer/ModelFigures.lean`
//! §8-97, wire D58).

use super::endpoints::selected_lines;
use crate::worker::fixture::*;
use crate::worker::*;

/// One `SelectModel` that states no figure, as a form with empty boxes
/// sends it.
fn pick(worker: &mut RunWorker, model: &str, tag: kernel::ModelTag, stated: Option<u64>) {
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: model.to_owned(),
            tag,
            context_tokens: stated.and_then(kernel::Window::new),
            max_output_tokens: stated.and_then(kernel::Ceiling::new),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, model.as_bytes()),
        })
        .unwrap();
}

/// The answer's rows in the shape `selected_lines` reads a
/// `model_selected` line in, so the two can be compared whole.
fn as_selected(answer: &wire::Answer) -> Vec<serde_json::Value> {
    let wire::Answer::ModelFigures(answer) = answer else {
        return Vec::new();
    };
    answer
        .models
        .iter()
        .map(|line| {
            let (tokens, from) = match line.ceiling {
                Some(wire::CeilingLine::Sent { tokens, from }) => {
                    (serde_json::json!(tokens.get()), serde_json::json!(from))
                }
                Some(wire::CeilingLine::ProviderDefault) => {
                    (serde_json::Value::Null, serde_json::json!("provider"))
                }
                None => (serde_json::Value::Null, serde_json::Value::Null),
            };
            serde_json::json!({
                "context_tokens": line.window.map_or(0, |window| window.tokens.get()),
                "window_from": line.window.map(|window| window.from),
                "max_output_tokens": tokens,
                "ceiling_from": from,
            })
        })
        .collect()
}

/// The placeholder is what a pick with empty boxes registers: the
/// provider's list for a row that states its figures, what a person
/// stated at an earlier pick of the same model under the same tag, and
/// for an id no rung knows on a server on this machine an unknown
/// window and the provider's own default ceiling on the chat face. The
/// question carries the text the person typed, which the city
/// normalises as the attach did.
#[test]
fn the_placeholder_answers_what_a_pick_with_empty_boxes_registers() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let ids = ["m-stated", "m-plain", "claude-sonnet-4-5"];
    let (base_url, _provider) = fake_openai(&ids, Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    pick(&mut worker, "m-plain", kernel::ModelTag::Main, Some(50_000));
    let earlier = selected_lines(&report.ledger_dir).len();

    let views = crate::views::Views::rebuild(&kernel::layout::CityLayout::new(dir.path()).ledger())
        .unwrap();
    let asked = |id: &str, listed: Option<u64>, tag| wire::FiguresAsked {
        id: id.to_owned(),
        context_tokens: listed.and_then(kernel::Window::new),
        max_output_tokens: listed
            .map(|_| LISTED_CEILING)
            .and_then(kernel::Ceiling::new),
        tag: Some(tag),
    };
    let answer = views
        .prepare(&wire::Query::ModelFigures {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            base_url: format!(" {base_url}/ "),
            face: kernel::DialectKind::OpenAi,
            models: vec![
                asked("m-stated", Some(LISTED_WINDOW), kernel::ModelTag::Digest),
                asked("m-plain", None, kernel::ModelTag::Main),
                asked("claude-sonnet-4-5", None, kernel::ModelTag::Transcribe),
            ],
        })
        .finish();

    pick(&mut worker, "m-stated", kernel::ModelTag::Digest, None);
    pick(&mut worker, "m-plain", kernel::ModelTag::Main, None);
    pick(
        &mut worker,
        "claude-sonnet-4-5",
        kernel::ModelTag::Transcribe,
        None,
    );
    let registered = selected_lines(&report.ledger_dir).split_off(earlier);
    assert_eq!(as_selected(&answer), registered);
}
