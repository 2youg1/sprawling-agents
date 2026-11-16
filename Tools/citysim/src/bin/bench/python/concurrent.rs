// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Barrier, mpsc};
use std::time::Duration;

use runtime::{Sandbox as _, SandboxExit};
use sprawling::monitor::spread::micros;
use sprawling::monitor::tree::{Tree, TreeReading};

use super::{
    CONCURRENT_BURSTS, CONCURRENT_CALLERS, Identity, MEMORY_BEAT, Prepared, STATE_CLEAN,
    STATE_LEAKED, STATE_WRITTEN, Sample, WARM_SAMPLES, checked_output, fixture, raw_job,
    timed_outcome,
};
use crate::reading::MachineClass;

pub(crate) fn warm_calls(prepared: &mut Prepared) -> Result<Vec<Sample>, String> {
    check_state_isolation(prepared)?;
    (0..WARM_SAMPLES)
        .map(|index| {
            let measured = timed_outcome(&mut prepared.sandbox, &prepared.guest)?;
            checked_output(&measured.outcome)?;
            Ok(Sample {
                took: measured.took,
                round: index.saturating_add(1),
                metric: "latency",
            })
        })
        .collect()
}

pub(super) fn check_state_isolation(prepared: &mut Prepared) -> Result<(), String> {
    let write_script = format!("bench_state = 41\nprint('{STATE_WRITTEN}')\n");
    let write = raw_job(&prepared.guest, &write_script);
    let written = prepared
        .sandbox
        .run(&write)
        .map_err(|why| why.to_string())?;
    if written.exit != SandboxExit::Success
        || written.stdout != output_line(STATE_WRITTEN)
        || !written.stderr.is_empty()
    {
        return Err(fixture(
            "the state-writing CPython call did not complete as written",
        ));
    }
    let read_script =
        format!("print('{STATE_LEAKED}' if 'bench_state' in globals() else '{STATE_CLEAN}')\n");
    let read = raw_job(&prepared.guest, &read_script);
    let read = prepared.sandbox.run(&read).map_err(|why| why.to_string())?;
    if read.exit != SandboxExit::Success
        || read.stdout != output_line(STATE_CLEAN)
        || !read.stderr.is_empty()
    {
        return Err(fixture(
            "a second CPython interpreter observed its predecessor's global",
        ));
    }
    Ok(())
}

pub(super) fn output_line(marker: &str) -> Vec<u8> {
    format!("{marker}\n").into_bytes()
}

pub(crate) struct Call {
    pub(crate) latency: Duration,
    pub(crate) start_delay: Duration,
    pub(crate) round: usize,
    pub(crate) returned: std::time::Instant,
    pub(crate) outcome: runtime::SandboxOutcome,
}

pub(crate) struct Concurrent {
    pub(crate) calls: Vec<Call>,
    pub(crate) walls: Vec<Duration>,
    pub(crate) calls_per_s: u64,
}

pub(crate) struct Memory {
    pub(crate) before: TreeReading,
    pub(crate) peak_address_bytes: u64,
    pub(crate) peak_resident_bytes: u64,
    pub(crate) after: TreeReading,
    pub(crate) beats: u64,
}

/// `sysinfo` calls the same field virtual size on Unix but PrivateUsage on
/// Windows. The report names the measured counter rather than promising an
/// unavailable Windows address-space reservation reading.
pub(crate) enum VirtualCounter {
    Virtual,
    PrivateCommit,
}

impl VirtualCounter {
    pub(crate) fn current() -> VirtualCounter {
        if cfg!(windows) {
            VirtualCounter::PrivateCommit
        } else {
            VirtualCounter::Virtual
        }
    }

    pub(crate) fn names(self) -> [&'static str; 3] {
        match self {
            VirtualCounter::Virtual => ["before_virtual", "peak_virtual", "after_virtual"],
            VirtualCounter::PrivateCommit => [
                "before_private_commit",
                "peak_private_commit",
                "after_private_commit",
            ],
        }
    }
}

pub(crate) fn concurrent_calls(prepared: &Prepared) -> Result<Concurrent, String> {
    let mut calls = Vec::new();
    let mut walls = Vec::new();
    let mut wall_us = 0_u64;
    for round in 1..=CONCURRENT_BURSTS {
        let burst = concurrent_burst(prepared, round)?;
        wall_us = wall_us.saturating_add(micros(burst.1));
        walls.push(burst.1);
        calls.extend(burst.0);
    }
    let call_count = u64::try_from(calls.len()).unwrap_or(u64::MAX);
    let calls_per_s = call_count
        .saturating_mul(1_000_000)
        .checked_div(wall_us.max(1))
        .unwrap_or(0);
    Ok(Concurrent {
        calls,
        walls,
        calls_per_s,
    })
}

/// Memory is read in a separate burst so the process-table walk never
/// disturbs the latency and throughput samples.
pub(crate) fn measure_memory(prepared: &Prepared) -> Result<Memory, String> {
    let mut tree = Tree::open(std::process::id());
    let before = tree
        .read(Duration::ZERO)
        .ok_or_else(|| fixture("the memory reader could not find this bench process"))?;
    let before_at = crate::stamp();
    let (stop, stopped) = mpsc::channel();
    let monitor = std::thread::spawn(move || monitor_memory(tree, stopped, before_at));
    let measured: Result<(), String> = (|| {
        for round in 1..=CONCURRENT_BURSTS {
            concurrent_burst(prepared, round)?;
        }
        Ok(())
    })();
    let sent = stop.send(()).map_err(|why| why.to_string());
    let monitored = monitor
        .join()
        .map_err(|_| fixture("the memory monitor panicked"))
        .and_then(|result| result);
    let cleanup = keep_both(sent, monitored).map(|((), monitored)| monitored);
    let ((), (mut tree, beats, last_read)) = keep_both(measured, cleanup)?;
    let now = crate::stamp();
    let remaining = MEMORY_BEAT.saturating_sub(now.saturating_duration_since(last_read));
    if !remaining.is_zero() {
        std::thread::sleep(remaining);
    }
    let after_at = crate::stamp();
    let after = tree
        .read(after_at.saturating_duration_since(last_read))
        .ok_or_else(|| fixture("the memory reader lost this bench process after the burst"))?;
    let seen = tree
        .seen()
        .ok_or_else(|| fixture("the memory reader kept no readings"))?;
    Ok(Memory {
        before,
        peak_address_bytes: seen.peak_private_bytes,
        peak_resident_bytes: seen.peak_working_set_bytes,
        after,
        beats: beats.saturating_add(2),
    })
}

pub(super) fn concurrent_burst(
    prepared: &Prepared,
    round: usize,
) -> Result<(Vec<Call>, Duration), String> {
    let barrier = Barrier::new(CONCURRENT_CALLERS.saturating_add(1));
    let spawned_at = crate::stamp();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(CONCURRENT_CALLERS);
        for _ in 0..CONCURRENT_CALLERS {
            let barrier = &barrier;
            let mut sandbox = prepared.sandbox.clone();
            let guest = &prepared.guest;
            handles.push(scope.spawn(move || {
                barrier.wait();
                let start_delay = spawned_at.elapsed();
                let measured = timed_outcome(&mut sandbox, guest)?;
                Ok::<Call, String>(Call {
                    latency: measured.took,
                    start_delay,
                    round,
                    returned: measured.returned,
                    outcome: measured.outcome,
                })
            }));
        }
        let started = crate::stamp();
        barrier.wait();
        let joined: Vec<_> = handles.into_iter().map(|handle| handle.join()).collect();
        let mut calls = Vec::with_capacity(CONCURRENT_CALLERS);
        for result in joined {
            calls.push(result.map_err(|_| fixture("a concurrent CPython caller panicked"))??);
        }
        let wall = calls
            .iter()
            .map(|call| call.returned)
            .max()
            .map(|returned| returned.saturating_duration_since(started))
            .ok_or_else(|| fixture("a concurrent burst contained no calls"))?;
        for call in &calls {
            checked_output(&call.outcome)?;
        }
        Ok((calls, wall))
    })
}

pub(super) fn monitor_memory(
    mut tree: Tree,
    stopped: mpsc::Receiver<()>,
    mut previous: std::time::Instant,
) -> Result<(Tree, u64, std::time::Instant), String> {
    let mut beats = 0_u64;
    loop {
        match stopped.recv_timeout(MEMORY_BEAT) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Ok((tree, beats, previous));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now = crate::stamp();
                if tree.read(now.duration_since(previous)).is_none() {
                    return Err(fixture("the memory monitor lost this bench process"));
                }
                previous = now;
                beats = beats.saturating_add(1);
            }
        }
    }
}

/// Neither a measurement failure nor a cleanup failure erases the other.
pub(crate) fn keep_both<T, U>(
    work: Result<T, String>,
    cleanup: Result<U, String>,
) -> Result<(T, U), String> {
    match (work, cleanup) {
        (Ok(work), Ok(cleanup)) => Ok((work, cleanup)),
        (Err(work), Ok(_)) => Err(work),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(work), Err(cleanup)) => Err(format!("{work}; {cleanup}")),
    }
}

pub(crate) fn memory_line(identity: Identity, memory: &Memory) -> Result<String, String> {
    if memory.before.private_bytes == 0
        || memory.before.working_set_bytes == 0
        || memory.after.private_bytes == 0
        || memory.after.working_set_bytes == 0
    {
        return Err(fixture(
            "a platform memory counter answered zero for a live process",
        ));
    }
    let [before_name, peak_name, after_name] = VirtualCounter::current().names();
    Ok(format!(
        "python_memory machine_class={} os={} arch={} {} beats={} {before_name}_bytes={} {peak_name}_bytes={} {after_name}_bytes={} before_resident_bytes={} peak_resident_bytes={} after_resident_bytes={}",
        MachineClass::General.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        identity.labels(),
        memory.beats,
        memory.before.private_bytes,
        memory.peak_address_bytes,
        memory.after.private_bytes,
        memory.before.working_set_bytes,
        memory.peak_resident_bytes,
        memory.after.working_set_bytes
    ))
}
