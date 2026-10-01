// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The monitor a served city offers its sessions, and the thread that
//! samples it (sprawling-SPEC.md 8-94, 8-96), apart from the listening
//! it is handed to.

use kernel::AxError;
use std::sync::Arc;

use crate::monitor::sampler::Gauges;

/// The monitor a session watches over the socket, and the thread that
/// samples it once a second (sprawling-SPEC.md 8-94, 8-96). Whether
/// anybody watches is the monitor's count; a session holds its
/// [`crate::monitor::Watch`] for as long as it watches.
///
/// # Errors
/// `StorageFatal` when the sampler's thread cannot be started.
pub(super) fn watched(
    city_root: &std::path::Path,
    gauges: Gauges<impl Fn() -> u64 + Send + 'static>,
) -> Result<wire::MonitorFeed, AxError> {
    let monitor = Arc::new(std::sync::Mutex::new(crate::monitor::Monitor::new()));
    let samples = tokio::sync::broadcast::channel(1).0;
    crate::monitor::sampler::spawn_sampler(
        Arc::downgrade(&monitor),
        samples.clone(),
        city_root.to_path_buf(),
        gauges,
    )?;
    Ok(wire::MonitorFeed {
        watch: Arc::new(move |watched| -> Box<dyn Send> {
            // The count is an atomic, so a poisoned lock guards no
            // half-written state and the watcher still counts.
            let held = monitor
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Box::new(held.watch(watched))
        }),
        samples,
    })
}
