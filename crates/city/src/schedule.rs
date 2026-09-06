// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Work that starts by itself.
//!
//! Two rules shape this module. Time is a parameter: `due` is asked
//! "between these two instants, what should have started", and it never
//! reads a clock, so a replay of the same window produces the same
//! dispatches. And a window returns each entry at most once, however
//! long the window is: an hourly job missed for eight hours is one run
//! owed, not eight. How wide the window is on a fresh start is the
//! caller's decision, not this module's.
//!
//! Cadences are counted from the epoch in whole minutes, which is why
//! they are stated in UTC and why the calendar shapes cron usually
//! carries - day of month, month - are refused rather than approximated.
//! Those need a calendar, a calendar needs an authority, and the city
//! does not have one yet. Concern time zones are a rendering matter and
//! live with the clock stamp.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, TimeMs};
use serde::Deserialize;

/// The city's schedule, at the city root.
pub const SCHEDULE_FILE: &str = "SCHEDULE.toml";

const MINUTE_MS: u64 = 60_000;
const DAY_MINUTES: u64 = 1_440;
const WEEK_MINUTES: u64 = 10_080;
/// 1970-01-01 was a Thursday; weekday 0 is Monday.
const EPOCH_WEEKDAY_OFFSET: u64 = 3;

/// How often an entry runs. Exhaustive: a fourth cadence is a change
/// here, not a string somebody smuggles through the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// Every `n` minutes, counted from the epoch.
    EveryMinutes(u64),
    /// Once a day at this minute of the day, UTC.
    DailyAt(u64),
    /// Once a week, at this minute of the week, UTC.
    WeeklyAt(u64),
}

impl Cadence {
    /// The last instant at or before `now` at which this cadence fired.
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

/// One scheduled job.
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

    /// What counts as done. Required, like every other dispatch's goal.
    #[must_use]
    pub fn goal(&self) -> &str {
        &self.goal
    }

    #[must_use]
    pub fn cadence(&self) -> Cadence {
        self.cadence
    }
}

/// The city's scheduled work.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    entries: Vec<Entry>,
}

impl Schedule {
    /// Reads a schedule file.
    ///
    /// # Errors
    /// Refuses a file that does not parse, a key this version does not
    /// read, an address that is not one, and a cadence it cannot spell.
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

    /// Reads the city's schedule. A city with no schedule file has no
    /// scheduled work, which is the ordinary case.
    ///
    /// # Errors
    /// Propagates an unreadable file and a file that does not parse.
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

    /// What should have started in `(after, now]`.
    ///
    /// An entry whose firing was slept through appears once, however
    /// long the sleep was: the city owes one morning run after a night
    /// off, not one per hour it was down.
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

    /// What should have started in `(after, now]`, as owned dispatches.
    ///
    /// The same window as [`Schedule::due`]; the triple is what a
    /// dispatch is made of, so the caller keeps only the loop. `due`
    /// stays because it is this module's own public face and removing
    /// it would break its readers.
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

/// Where the schedule lives.
#[must_use]
pub fn schedule_path(city_root: &Path) -> PathBuf {
    city_root.join(SCHEDULE_FILE)
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleFile {
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

/// One refusal shape, so the recovery line is written once.
fn refuse(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read the city schedule", subject).with_recovery(
        "each job takes name, addr, task, goal and exactly one of `every = \"30m\"`, \
         `daily = \"09:00\"`, `weekly = \"mon 09:00\"`; times are UTC, and calendar shapes \
         such as day-of-month wait for a calendar the city does not have yet",
    )
}

#[cfg(test)]
mod tests;
