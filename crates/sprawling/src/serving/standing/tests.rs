// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use super::{
    BUSY_LIMIT, CorePriority, Standing, Valve, Verdict, raise_this_thread, serving_runtime,
};

#[cfg(windows)]
#[test]
fn a_raised_core_thread_stands_above_normal() {
    use thread_priority::{ThreadPriority, WinAPIThreadPriority, get_current_thread_priority};
    let read = std::thread::spawn(|| {
        let standing = raise_this_thread(CorePriority::Raised);
        (standing, get_current_thread_priority().unwrap())
    })
    .join()
    .unwrap();
    assert_eq!(
        read,
        (
            Standing::Raised,
            ThreadPriority::Os(WinAPIThreadPriority::AboveNormal.into())
        )
    );
}

#[test]
fn a_thread_busy_through_the_window_is_lowered() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let limit = BUSY_LIMIT.as_millis().try_into().unwrap();

    let mut half = Valve::new(BUSY_LIMIT, start);
    (0..20).for_each(|turn| half.record(at(turn * limit / 20), at(turn * limit / 20 + limit / 40)));
    half.record(at(limit), at(limit + 1));

    let mut full = Valve::new(BUSY_LIMIT, start);
    full.record(at(0), at(limit / 2));
    full.record(at(limit / 2), at(limit + 1));
    full.record(at(limit + 1), at(limit + 2));

    assert_eq!(
        (half.verdict(), full.verdict()),
        (Verdict::Keep, Verdict::Lower)
    );
}

#[cfg(windows)]
#[test]
fn a_socket_worker_stands_above_normal_once_it_has_woken() {
    use thread_priority::{ThreadPriority, WinAPIThreadPriority, get_current_thread_priority};
    let runtime = serving_runtime(CorePriority::Raised).unwrap();
    let read = runtime.block_on(async {
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            get_current_thread_priority().unwrap()
        })
        .await
        .unwrap()
    });
    assert_eq!(
        read,
        ThreadPriority::Os(WinAPIThreadPriority::AboveNormal.into())
    );
}
