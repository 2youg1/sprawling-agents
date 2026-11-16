// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, TimeMs};
use serde::Deserialize;

pub const SCHEDULE_FILE: &str = "SCHEDULE.toml";

const MINUTE_MS: u64 = 60_000;
const DAY_MINUTES: u64 = 1_440;
const WEEK_MINUTES: u64 = 10_080;

const EPOCH_WEEKDAY_OFFSET: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    EveryMinutes(u64),

    DailyAt(u64),

    WeeklyAt(u64),
}

impl Cadence {
    fn last_firing(self, now_minutes: u64) -> u64 {
        match self {
            Cadence::EveryMinutes(period) => {
                let period = period.max(1);
                now_minutes.saturating_sub(now_minutes.checked_rem(period).unwrap_or(0))
            }
            Cadence::DailyAt(minute_of_day) => {
                let day =
                    now_minutes.saturating_sub(now_minutes.checked_rem(DAY_MINUTES).unwrap_or(0));
                let today = day.saturating_add(minute_of_day);
                if today <= now_minutes {
                    today
                } else {
                    today.saturating_sub(DAY_MINUTES)
                }
            }
            Cadence::WeeklyAt(minute_of_week) => {
                let week_start =
                    now_minutes.saturating_add(EPOCH_WEEKDAY_OFFSET.saturating_mul(DAY_MINUTES));
                let offset = week_start.checked_rem(WEEK_MINUTES).unwrap_or(0);
                let this_week = now_minutes
                    .saturating_sub(offset)
                    .saturating_add(minute_of_week);
                if this_week <= now_minutes {
                    this_week
                } else {
                    this_week.saturating_sub(WEEK_MINUTES)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    name: String,
    addr: Address,
    task: String,
    goal: String,
    cadence: Cadence,
}

impl Entry {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    #[must_use]
    pub fn task(&self) -> &str {
        &self.task
    }

    #[must_use]
    pub fn goal(&self) -> &str {
        &self.goal
    }

    #[must_use]
    pub fn cadence(&self) -> Cadence {
        self.cadence
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    entries: Vec<Entry>,
}

impl Schedule {
    pub fn parse(text: &str) -> Result<Schedule, AxError> {
        let file: ScheduleFile = toml::from_str(text).map_err(|err| refuse(err.to_string()))?;
        let mut entries = Vec::new();
        for row in file.job {
            entries.push(Entry {
                addr: Address::parse(&row.addr)?,
                cadence: read_cadence(&row)?,
                name: row.name,
                task: row.task,
                goal: row.goal,
            });
        }
        Ok(Schedule { entries })
    }

    pub fn load(city_root: &Path) -> Result<Schedule, AxError> {
        let path = schedule_path(city_root);
        match std::fs::read_to_string(&path) {
            Ok(text) => Schedule::parse(&text),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Schedule::default()),
            Err(err) => Err(AxError::failure(
                AxCode::StorageFatal,
                "read the city schedule",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("fix the file's permissions; a schedule that exists is read")),
        }
    }

    #[must_use]
    pub fn due(&self, after: TimeMs, now: TimeMs) -> Vec<&Entry> {
        let after_minutes = after.value().checked_div(MINUTE_MS).unwrap_or(0);
        let now_minutes = now.value().checked_div(MINUTE_MS).unwrap_or(0);
        self.entries
            .iter()
            .filter(|entry| {
                let fired = entry.cadence.last_firing(now_minutes);
                fired > after_minutes && fired <= now_minutes
            })
            .collect()
    }

    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[must_use]
    pub fn due_after(&self, after: TimeMs, now: TimeMs) -> Vec<(Address, String, String)> {
        self.due(after, now)
            .into_iter()
            .map(|entry| {
                (
                    entry.addr().clone(),
                    entry.task().to_owned(),
                    entry.goal().to_owned(),
                )
            })
            .collect()
    }
}

#[must_use]
pub fn schedule_path(city_root: &Path) -> PathBuf {
    city_root.join(SCHEDULE_FILE)
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScheduleFile {
    #[serde(default)]
    job: Vec<JobRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobRow {
    name: String,
    addr: String,
    task: String,
    goal: String,
    #[serde(default)]
    every: Option<String>,
    #[serde(default)]
    daily: Option<String>,
    #[serde(default)]
    weekly: Option<String>,
}

fn read_cadence(row: &JobRow) -> Result<Cadence, AxError> {
    match (&row.every, &row.daily, &row.weekly) {
        (Some(every), None, None) => read_every(every),
        (None, Some(daily), None) => read_clock(daily).map(Cadence::DailyAt),
        (None, None, Some(weekly)) => read_weekly(weekly),
        (None, None, None) => Err(refuse(format!("{}: no cadence", row.name))),
        _ => Err(refuse(format!(
            "{}: two cadences on one job; it can only run on one of them",
            row.name
        ))),
    }
}

fn read_every(raw: &str) -> Result<Cadence, AxError> {
    let (value, scale) = match raw.strip_suffix('m') {
        Some(value) => (value, 1),
        None => match raw.strip_suffix('h') {
            Some(value) => (value, 60),
            None => return Err(refuse(format!("`every = {raw:?}` is not `<n>m` or `<n>h`"))),
        },
    };
    let count: u64 = value
        .trim()
        .parse()
        .map_err(|_| refuse(format!("`every = {raw:?}` has no number in it")))?;
    if count == 0 {
        return Err(refuse("`every` is at least one minute".to_owned()));
    }
    Ok(Cadence::EveryMinutes(count.saturating_mul(scale)))
}

fn read_clock(raw: &str) -> Result<u64, AxError> {
    let (hours, minutes) = raw
        .trim()
        .split_once(':')
        .ok_or_else(|| refuse(format!("{raw:?} is not `HH:MM`")))?;
    let hours: u64 = hours
        .parse()
        .map_err(|_| refuse(format!("{raw:?} has no hour in it")))?;
    let minutes: u64 = minutes
        .parse()
        .map_err(|_| refuse(format!("{raw:?} has no minute in it")))?;
    if hours > 23 || minutes > 59 {
        return Err(refuse(format!("{raw:?} is not a time of day")));
    }
    Ok(hours.saturating_mul(60).saturating_add(minutes))
}

fn read_weekly(raw: &str) -> Result<Cadence, AxError> {
    let (day, clock) = raw
        .trim()
        .split_once(' ')
        .ok_or_else(|| refuse(format!("`weekly = {raw:?}` is not `<day> HH:MM`")))?;
    let index: u64 = match day.to_ascii_lowercase().as_str() {
        "mon" => 0,
        "tue" => 1,
        "wed" => 2,
        "thu" => 3,
        "fri" => 4,
        "sat" => 5,
        "sun" => 6,
        other => return Err(refuse(format!("{other:?} is not mon..sun"))),
    };
    let minute_of_day = read_clock(clock)?;
    Ok(Cadence::WeeklyAt(
        index
            .saturating_mul(DAY_MINUTES)
            .saturating_add(minute_of_day),
    ))
}

fn refuse(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read the city schedule", subject).with_recovery(
        "each job takes name, addr, task, goal and exactly one of `every = \"30m\"`, \
         `daily = \"09:00\"`, `weekly = \"mon 09:00\"`; times are UTC, and calendar shapes \
         such as day-of-month wait for a calendar the city does not have yet",
    )
}

#[cfg(test)]
mod tests;
