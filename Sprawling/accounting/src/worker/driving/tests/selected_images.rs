// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::flight::history;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn dispatch_and_steer_send_only_each_selected_png_to_a_multimodal_provider() {
    use base64::Engine as _;
    use std::sync::{Arc, Mutex, mpsc};
    const WITHIN: std::time::Duration = std::time::Duration::from_secs(45);
    const FIRST: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 2,
        0, 0, 0, 123, 64, 232, 221, 0, 0, 0, 15, 73, 68, 65, 84, 120, 156, 99, 120, 214, 163, 200,
        245, 225, 56, 0, 11, 112, 3, 85, 70, 67, 57, 100, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
        130,
    ];
    const SECOND: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 2,
        0, 0, 0, 123, 64, 232, 221, 0, 0, 0, 15, 73, 68, 65, 84, 120, 156, 99, 144, 211, 48, 117,
        59, 209, 8, 0, 5, 58, 2, 11, 33, 21, 49, 125, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let first = base64::engine::general_purpose::STANDARD.encode(FIRST);
    let second = base64::engine::general_purpose::STANDARD.encode(SECOND);
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let (arrived_tx, arrived) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let held = Mutex::new((Some(arrived_tx), released));
    let pace: Pace = Arc::new(move |request: &str| {
        if request.starts_with("POST ") {
            let mut held = held.lock().unwrap();
            if let Some(arrived) = held.0.take() {
                arrived.send(()).unwrap();
                held.1.recv_timeout(WITHIN).unwrap();
            }
        }
    });
    let initial = completion_with("looking", "status", "tu_0", serde_json::json!({}));
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![initial, completion("done", None)],
        pace,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let key = |words: &[u8]| kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, words);
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: Some(kernel::InputKinds::TextImage),
            idem: key(b"choose image model"),
        })
        .unwrap();
    let desk = Arc::new(crate::worker::CommandDesk::default());
    let observing = Arc::clone(&desk);
    worker.observe(Box::new(move |record| observing.observe(record)));
    let asking = Arc::clone(&desk);
    let store = worker.attachment_store();
    worker.serve(only_interrupts(Arc::new(move |run| {
        asking.interrupt_for(run, Some(&store))
    })));
    let attending = {
        let desk = Arc::clone(&desk);
        std::thread::spawn(move || crate::worker::attend::attend(&mut worker, &desk))
    };
    desk.post(
        wire::Command::Dispatch {
            addr: Address::parse("lab/east").unwrap(),
            task: "Discuss the highlighted equation".to_owned(),
            goal: "Explain the equation".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: key(b"dispatch crop"),
            session: None,
            resume: None,
            images: vec![first.clone()],
            effort: None,
            model: None,
        },
        wire::Reply::nowhere(),
    );
    arrived.recv_timeout(WITHIN).unwrap();
    let started = history(&report.ledger_dir)
        .into_iter()
        .find(|line| line["kind"] == "run_started")
        .unwrap();
    let run = RunId::parse(started["run"].as_str().unwrap()).unwrap();
    let request = key(b"steer crop");
    let (receipt, heard) = mpsc::channel();
    let command = || wire::Command::Steer {
        run,
        text: "Compare with the second highlighted equation".to_owned(),
        images: vec![second.clone()],
        idem: request,
    };
    let reply = wire::Reply::outcomes(move |outcome| match receipt.send(outcome) {
        Ok(()) => wire::Delivered::ToThePeer,
        Err(_) => wire::Delivered::PeerGone,
    });
    desk.post(command(), reply.clone());
    release.send(()).unwrap();
    assert!(
        matches!(heard.recv_timeout(WITHIN).unwrap(), wire::ReplyOutcome::Accepted(key) if key == request)
    );
    desk.post(command(), reply);
    assert!(
        matches!(heard.recv_timeout(WITHIN).unwrap(), wire::ReplyOutcome::Accepted(key) if key == request)
    );
    desk.close(by_hand());
    attending.join().unwrap();

    let bodies = provider.bodies();
    let dispatch_image = bodies
        .iter()
        .position(|body| body.contains(&first))
        .expect("the dispatch crop reached the provider");
    assert!(!bodies[dispatch_image].contains(&second));
    assert!(
        bodies
            .iter()
            .skip(dispatch_image + 1)
            .any(|body| body.contains(&second))
    );
    let steered = history(&report.ledger_dir)
        .into_iter()
        .filter(|line| line["kind"] == "steer_received")
        .count();
    assert_eq!(steered, 1);
}
