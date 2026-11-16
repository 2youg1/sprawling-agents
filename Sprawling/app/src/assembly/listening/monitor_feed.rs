// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use std::sync::Arc;

use crate::monitor::sampler::Gauges;

pub(super) fn watched(
    city_root: &std::path::Path,
    gauges: Gauges<impl Fn() -> u64 + Send + 'static>,
) -> Result<wire::MonitorFeed, AxError> {
    let monitor = Arc::new(std::sync::Mutex::new(crate::monitor::Monitor::new()));
    let samples = tokio::sync::broadcast::channel(1).0;
    let beat = crate::monitor::sampler::spawn_sampler(
        Arc::downgrade(&monitor),
        samples.clone(),
        city_root.to_path_buf(),
        gauges,
    )?;
    Ok(wire::MonitorFeed {
        beat: Arc::new(move |chosen| beat.set(chosen)),
        watch: Arc::new(move |watched| -> Box<dyn Send> {
            let held = monitor
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Box::new(held.watch(watched))
        }),
        samples,
    })
}
