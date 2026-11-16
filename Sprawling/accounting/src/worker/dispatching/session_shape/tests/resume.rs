// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_new_session_checks_allowed_and_inherited_effort_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(&["m-thinks", "m-other"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-thinks").unwrap();
    lay_rules(
        dir.path(),
        "lab",
        &ordinary_rules(
            r#"
[workspace_models]
allowed = [{ endpoint = "house", model = "m-thinks" }]
max_effort = "low"
[workspace_models.recommendations.easy]
endpoint = "house"
model = "m-thinks"
effort = "low"
[workspace_models.recommendations.medium]
endpoint = "house"
model = "m-thinks"
effort = "low"
[workspace_models.recommendations.hard]
endpoint = "house"
model = "m-thinks"
effort = "low"
"#,
        ),
    );
    // An old room's frozen model must not exempt a newly opened room.
    let old = Address::parse("lab/old").unwrap();
    city::write_session(
        dir.path(),
        &old,
        &city::SessionShape::new("m-thinks", None, None),
    )
    .unwrap();
    choose(&mut worker, "m-other", b"other");
    let mut request = ask_on(&old, "m-other", b"new-disallowed");
    if let wire::Command::Dispatch { session, .. } = &mut request {
        *session = Some(kernel::SessionName::parse("new-room").unwrap());
    }
    assert_eq!(
        worker.handle(request).unwrap_err().code(),
        &kernel::AxCode::ConfigInvalid
    );
    assert!(
        city::own_layer(dir.path(), &Address::parse("lab/new-room").unwrap())
            .unwrap()
            .model()
            .is_none()
    );
    choose(&mut worker, "m-thinks", b"allowed");
    std::fs::write(
        city::config_path(dir.path(), &old, city::Layer::Building).unwrap(),
        "[model]\neffort = \"high\"\n",
    )
    .unwrap();
    let fresh = Address::parse("lab/fresh").unwrap();
    assert_eq!(
        worker
            .handle(ask_on(&fresh, "m-thinks", b"too-high"))
            .unwrap_err()
            .code(),
        &kernel::AxCode::ConfigInvalid
    );
    assert!(
        city::own_layer(dir.path(), &fresh)
            .unwrap()
            .model()
            .is_none()
    );
}

#[test]
fn a_resumed_session_keeps_its_endpoint_when_model_ids_collide() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker.handle(ask(&room, None, b"first")).unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"keep-house"),
        })
        .unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("other").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach-other"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("other").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select-other"),
        })
        .unwrap();
    let selected = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&selected);
    worker.observe(Box::new(move |record: &EventRecord| {
        if record.kind() == EventKind::RunStarted {
            observed.lock().unwrap().push(
                record
                    .data()
                    .read::<kernel::event::record::RunStarted>()
                    .unwrap()
                    .routing
                    .unwrap()
                    .model,
            );
        }
    }));
    worker.handle(ask(&room, None, b"resume")).unwrap();
    assert_eq!(
        *selected.lock().unwrap(),
        vec![kernel::ModelRef {
            endpoint: "house".to_owned(),
            model: "m-local".to_owned()
        }]
    );
}
