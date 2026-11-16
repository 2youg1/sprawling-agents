// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_session_keeps_the_shape_it_froze_and_refuses_a_different_effort() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-thinks"],
        vec![
            completion("done", None),
            completion("done", None),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-thinks").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let seen: Prefixes = Arc::new(Mutex::new(Vec::new()));
    watch(&mut worker, &seen);

    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-1"))
        .unwrap();
    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-2"))
        .unwrap();

    let frozen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(frozen.len(), 2, "one assembled prompt per dispatch");
    assert_eq!(
        frozen[0], frozen[1],
        "the second run in this session froze the same four segments"
    );
    assert_eq!(
        bytes_of(dir.path(), &frozen[0]),
        bytes_of(dir.path(), &frozen[1]),
        "and the bytes behind them are equal, slot by slot"
    );
    assert_eq!(
        frozen[0]
            .iter()
            .map(|(slot, _, _)| slot.as_str())
            .collect::<Vec<_>>(),
        ["city", "building", "resident", "run"],
        "the four slots, in the order the prefix concatenates them"
    );

    let refused = worker
        .handle(ask(&room, Some(kernel::Effort::High), b"dispatch-3"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid, "{refused}");
    assert!(
        refused.subject().contains("the effort changed"),
        "the refusal names the field that moved: {refused}"
    );
    assert!(
        refused
            .recovery()
            .contains("send this task to another address"),
        "and the way out a person can take today: {refused}"
    );
    assert_eq!(
        city::settled_effort(dir.path(), &room)
            .unwrap()
            .map(|(effort, _)| effort),
        Some(kernel::Effort::Low),
        "the refused dispatch left the session's effort where it was"
    );
    assert_eq!(
        seen.lock().unwrap_or_else(PoisonError::into_inner).len(),
        2,
        "a refused dispatch assembled no prompt"
    );

    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-4"))
        .unwrap();
    let after = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(after.len(), 3);
    assert_eq!(
        after[2], frozen[0],
        "the session runs on the prefix it froze"
    );
}

#[test]
fn generation_defaults_reach_new_sessions_without_moving_an_existing_one() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    set_temperature(&mut worker, "0.2");
    worker.handle(ask(&room, None, b"dispatch-1")).unwrap();
    assert_eq!(
        city::own_layer(dir.path(), &room)
            .unwrap()
            .generation()
            .and_then(|generation| generation.settings.temperature.as_deref()),
        Some("0.2")
    );

    set_temperature(&mut worker, "0.8");
    let refused = worker.handle(ask(&room, None, b"dispatch-2")).unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid);
    assert!(refused.subject().contains("sampling settings"));

    let elsewhere = Address::parse("lab/room2").unwrap();
    worker.handle(ask(&elsewhere, None, b"dispatch-3")).unwrap();
    let temperatures = provider
        .bodies()
        .into_iter()
        .filter(|body| !body.is_empty())
        .map(|body| {
            serde_json::from_str::<serde_json::Value>(&body)
                .unwrap()
                .get("temperature")
                .and_then(serde_json::Value::as_f64)
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(temperatures, vec![0.2, 0.8]);
}
