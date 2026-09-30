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

use crate::assembly::*;

thread_local! {
    /// Every address the scripted file manager was handed, in order.
    static HANDED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// A file manager that opens nothing and remembers what it was asked
/// to show.
fn scripted(_city: &Path, at: &Address) -> Result<(), AxError> {
    HANDED.with(|handed| handed.borrow_mut().push(at.as_str().to_owned()));
    Ok(())
}

/// The address names nothing on disk, so a worker that still starts the
/// host's file manager itself refuses it with `PathNotFound`, and the
/// file manager it was handed is never asked.
#[test]
fn a_reveal_reaches_the_file_manager_the_worker_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    crate::assembly::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::assembly::fixture::hands(),
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
