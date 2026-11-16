// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::reading::{MachineClass, row_line, spread_of};
use kernel::{AxCode, AxError};
use sprawling::monitor::spread::micros;

use super::concurrent::{VirtualCounter, keep_both};
use super::{Arm, Identity, Measurement, Memory, Observation, Report, Sample};

pub(crate) fn record(
    row: &str,
    samples: Vec<Sample>,
    identity: Identity,
    beside: &str,
) -> Result<Measurement, String> {
    let mut lines: Vec<String> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "python_sample row={row} index={} took_us={} {}",
                index.saturating_add(1),
                micros(sample.took),
                identity.labels()
            )
        })
        .collect();
    let conditions = format!(
        " machine_class={} os={} arch={} {}{beside}",
        MachineClass::General.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        identity.labels()
    );
    let observations = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| Observation {
            row: row.to_owned(),
            index: index.saturating_add(1),
            round: sample.round,
            metric: sample.metric,
            value: micros(sample.took),
            unit: "us",
            identity,
        })
        .collect();
    let durations = samples.iter().map(|sample| sample.took).collect();
    lines.push(row_line(row, &spread_of(durations)?, &conditions));
    Ok(Measurement {
        lines,
        observations,
    })
}

impl Report {
    pub(crate) fn add(&mut self, measurement: Measurement) {
        self.lines.extend(measurement.lines);
        self.observations.extend(measurement.observations);
    }

    pub(crate) fn new(arm: Arm) -> Report {
        Report {
            lines: Vec::new(),
            observations: Vec::new(),
            arm,
        }
    }

    /// Writes raw observations, not copied summaries, as one CSV artifact.
    ///
    /// # Errors
    /// Refuses an existing result and reports creation, write or publication failure
    /// without presenting the partial file as a completed measurement.
    pub(crate) fn write_csv(&self) -> Result<PathBuf, String> {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        let path = root.join(format!(
            "python-bench-{}-{}.csv",
            self.arm.as_str(),
            std::process::id()
        ));
        self.write_csv_at(&path)?;
        Ok(path)
    }

    pub(crate) fn write_csv_at(&self, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| fixture("the CSV output path has no parent directory"))?;
        std::fs::create_dir_all(parent).map_err(|why| csv_storage(path, why))?;
        let part = path.with_extension("csv.part");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|why| csv_storage(&part, why))?;
        let written = file
            .write_all(CSV_HEADER.as_bytes())
            .and_then(|()| {
                for observation in &self.observations {
                    observation.write_csv(&mut file)?;
                }
                Ok(())
            })
            .and_then(|()| file.sync_all());
        drop(file);
        // A same-directory hard link publishes atomically and fails when the
        // target already exists, unlike a replacement-style rename.
        let published = publish_csv(written, &part, path);
        let cleanup = std::fs::remove_file(&part)
            .map_err(|why| csv_cleanup_error(path, &part, published.is_ok(), why));
        keep_both(published, cleanup)?;
        Ok(())
    }
}

const CSV_SCHEMA_VERSION: u8 = 1;
pub(crate) const CSV_HEADER: &str = "schema_version,build_blake3,workload,observation,round,cohort,arm,metric,value,unit,guest_blake3,script_blake3,machine_class,os,arch\n";

impl Observation {
    fn write_csv(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            out,
            "{CSV_SCHEMA_VERSION},{},{},{},{},1,{},{},{},{},{},{},{},{},{}",
            self.identity.build,
            self.row,
            self.index,
            self.round,
            self.identity.arm.as_str(),
            self.metric,
            self.value,
            self.unit,
            self.identity.guest,
            self.identity.script,
            MachineClass::General.as_str(),
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}

pub(crate) fn round_observations(identity: Identity, walls: &[Duration]) -> Vec<Observation> {
    walls
        .iter()
        .enumerate()
        .map(|(index, wall)| Observation {
            row: "python_concurrent_round".to_owned(),
            index: index.saturating_add(1),
            round: index.saturating_add(1),
            metric: "wall",
            value: micros(*wall),
            unit: "us",
            identity,
        })
        .collect()
}

pub(crate) fn memory_observations(identity: Identity, memory: &Memory) -> Vec<Observation> {
    let [before_name, peak_name, after_name] = VirtualCounter::current().names();
    [
        (before_name, memory.before.private_bytes),
        (peak_name, memory.peak_address_bytes),
        (after_name, memory.after.private_bytes),
        ("before_resident", memory.before.working_set_bytes),
        ("peak_resident", memory.peak_resident_bytes),
        ("after_resident", memory.after.working_set_bytes),
    ]
    .into_iter()
    .map(|(metric, value)| Observation {
        row: "python_memory".to_owned(),
        index: 1,
        round: 1,
        metric,
        value,
        unit: "bytes",
        identity,
    })
    .chain(std::iter::once(Observation {
        row: "python_memory".to_owned(),
        index: 1,
        round: 1,
        metric: "beats",
        value: memory.beats,
        unit: "count",
        identity,
    }))
    .collect()
}

/// Associates write/sync errors with the partial file and link errors with the result.
pub(crate) fn publish_csv(
    written: std::io::Result<()>,
    part: &Path,
    path: &Path,
) -> Result<(), String> {
    written.map_err(|why| csv_storage(part, why))?;
    std::fs::hard_link(part, path).map_err(|why| csv_storage(path, why))
}

/// Preserves both paths when publication succeeded before cleanup failed.
pub(crate) fn csv_cleanup_error(
    path: &Path,
    part: &Path,
    published: bool,
    why: std::io::Error,
) -> String {
    if !published {
        return csv_storage(part, why);
    }
    error_with_recovery(
        AxError::failure(
            AxCode::StorageFatal,
            "remove the temporary CPython-WASI CSV after publication",
            format!(
                "CSV published at {}; temporary file {} remains: {why}",
                path.display(),
                part.display()
            ),
        )
        .with_recovery(format!(
            "delete only the residual .part file {}; keep the published CSV at {}",
            part.display(),
            path.display()
        )),
    )
}

/// Keeps the existing error display and its actionable recovery together.
pub(super) fn error_with_recovery(error: AxError) -> String {
    format!("{error}; recovery: {}", error.recovery())
}

pub(super) fn csv_storage(path: &Path, detail: impl std::fmt::Display) -> String {
    error_with_recovery(
        AxError::failure(
            AxCode::StorageFatal,
            "write raw CPython-WASI measurements",
            format!("{}: {detail}", path.display()),
        )
        .with_recovery(
            "choose a writable target directory and rerun; the partial CSV is not a result",
        ),
    )
}

pub(crate) fn prerequisite(detail: String) -> String {
    error_with_recovery(
        AxError::failure(
            AxCode::ToolUnavailable,
            "measure the real CPython-WASI baseline",
            detail,
        )
        .with_recovery(
            "configure a real CPython-WASI guest through the external guest door documented by \
         `sprawling doctor`, then rerun; a WAT correctness fixture is not a timing substitute",
        ),
    )
}

pub(crate) fn fixture(detail: impl Into<String>) -> String {
    let error = AxError::failure(
        AxCode::InvalidArgs,
        "measure the registered CPython-WASI scenario",
        detail.into(),
    )
    .with_recovery("check the selected guest and the fixed script; no partial report is emitted");
    error_with_recovery(error)
}
