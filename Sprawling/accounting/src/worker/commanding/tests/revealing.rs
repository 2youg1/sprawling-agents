// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

use std::cell::RefCell;

use crate::worker::*;

thread_local! {

    static HANDED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn scripted(_city: &Path, at: &Address) -> Result<(), AxError> {
    HANDED.with(|handed| handed.borrow_mut().push(at.as_str().to_owned()));
    Ok(())
}

#[test]
fn a_reveal_reaches_the_file_manager_the_worker_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker.reveal_with(scripted);

    let told = worker
        .handle(wire::Command::Reveal {
            at: Address::parse("hall/nothing.md").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"reveal"),
        })
        .map_err(|refusal| *refusal.code());

    assert_eq!(
        (told, HANDED.with(|handed| handed.borrow().clone())),
        (Ok(()), vec!["hall/nothing.md".to_owned()])
    );
}
