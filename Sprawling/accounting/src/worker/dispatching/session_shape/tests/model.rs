// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_new_session_lets_the_room_use_the_model_chosen_since() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-local", "m-other"],
        vec![
            completion("done", None),
            completion("done", None),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();

    worker.handle(ask(&room, None, b"dispatch-1")).unwrap();
    choose(&mut worker, "m-other", b"select-other");
    let refused = worker.handle(ask(&room, None, b"dispatch-2")).unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid);

    worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"open"),
        })
        .unwrap();

    worker.handle(ask(&room, None, b"dispatch-3")).unwrap();
    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-other"),
        "the new session froze the model the registry answers with now"
    );
}

#[test]
fn a_model_chosen_after_a_session_opened_does_not_reach_it() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-local", "m-other"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();

    worker.handle(ask(&room, None, b"dispatch-1")).unwrap();
    choose(&mut worker, "m-other", b"select-other");

    let refused = worker.handle(ask(&room, None, b"dispatch-2")).unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid, "{refused}");
    assert!(
        refused.subject().contains("the model changed"),
        "the refusal names the field that moved: {refused}"
    );
    assert!(
        refused
            .recovery()
            .contains("send this task to another address"),
        "and the way out a person can take today: {refused}"
    );
    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-local"),
        "the session still records the model it opened with"
    );

    let elsewhere = Address::parse("lab/room2").unwrap();
    worker.handle(ask(&elsewhere, None, b"dispatch-3")).unwrap();
    assert_eq!(
        city::own_layer(dir.path(), &elsewhere).unwrap().model(),
        Some("m-other")
    );
}

#[test]
fn a_dispatch_that_names_a_registered_model_runs_on_it() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) =
        fake_openai(&["m-local", "m-other"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-other".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select-digest"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(ask_on(&room, "m-other", b"dispatch-1"))
        .unwrap();
    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-other"),
        "the room froze the model the dispatch named, not the main tag's"
    );

    let stranger = Address::parse("lab/room2").unwrap();
    let refused = worker
        .handle(ask_on(&stranger, "m-nobody", b"dispatch-2"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid, "{refused}");
    assert!(
        !dir.path().join("lab").join("room2").exists(),
        "a refused dispatch opened no room"
    );
}

#[test]
fn a_run_naming_no_model_runs_on_the_room_frozen_one() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let replies = vec![completion("done", None), completion("done", None)];
    let (base_url, _provider) = fake_openai(&["m-local", "m-other"], replies);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-other".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select-digest"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(ask_on(&room, "m-other", b"dispatch-1"))
        .unwrap();
    let second = worker.handle(ask(&room, None, b"dispatch-2"));
    assert!(second.is_ok(), "{second:?}");
}
