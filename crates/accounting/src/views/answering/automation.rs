// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two automation files a page reads (`crates/wire/Spec.lean` §8-62): read at
//! the moment of asking, through the same loaders the schedule tick and
//! the watch listener read them through.

use std::path::Path;

/// The schedule and the watch table as they stand, one unreadable file
/// leaving the other listed.
pub(crate) fn automation_answer(city_root: &Path) -> wire::Answer {
    let mut answer = wire::AutomationAnswer::default();
    match city::Schedule::load(city_root) {
        Ok(schedule) => answer.jobs = schedule.entries().iter().map(job_of).collect(),
        Err(refused) => answer
            .unreadable
            .push(unread(city::SCHEDULE_FILE, &refused)),
    }
    match city::Watch::load(city_root) {
        Ok(watch) => answer.sources = watch.sources().iter().map(source_of).collect(),
        Err(refused) => answer.unreadable.push(unread(city::WATCH_FILE, &refused)),
    }
    wire::Answer::Automation(Box::new(answer))
}

/// The line a page shows for a file that did not read: its name and the
/// refusal the next tick would give.
fn unread(file: &str, refused: &kernel::AxError) -> String {
    format!("{file}: {} ({})", refused.subject(), refused.recovery())
}

fn job_of(entry: &city::Entry) -> wire::ScheduledJob {
    wire::ScheduledJob {
        name: entry.name().to_owned(),
        addr: entry.addr().clone(),
        task: entry.task().to_owned(),
        goal: entry.goal().to_owned(),
        cadence: match entry.cadence() {
            city::Cadence::EveryMinutes(minutes) => wire::Cadence::EveryMinutes { minutes },
            city::Cadence::DailyAt(minute) => wire::Cadence::DailyAt { minute },
            city::Cadence::WeeklyAt(minute) => wire::Cadence::WeeklyAt { minute },
        },
    }
}

fn source_of(source: &city::Source) -> wire::WatchedSource {
    wire::WatchedSource {
        name: source.name().to_owned(),
        matches: source.matches().to_owned(),
        addr: source.addr().clone(),
        starts_work: source.starts_work(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    /// Both tables are listed as written, and a file that does not read
    /// is named with its reason while the other is still listed.
    #[test]
    fn the_automation_query_lists_both_tables_and_names_the_one_that_does_not_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            city::schedule_path(dir.path()),
            "[[job]]\nname = \"nightly\"\naddr = \"lab/room1\"\ntask = \"run the tests\"\ngoal = \"a report\"\ndaily = \"02:30\"\n",
        )
        .unwrap();
        std::fs::write(city::watch_path(dir.path()), "[[source]]\nname = \"ci\"\n").unwrap();

        let wire::Answer::Automation(answer) = automation_answer(dir.path()) else {
            panic!("answered something else")
        };
        let job = answer
            .jobs
            .first()
            .map(|job| (job.name.as_str(), job.cadence));
        assert_eq!(
            (job, answer.sources.len(), answer.unreadable.len()),
            (
                Some(("nightly", wire::Cadence::DailyAt { minute: 150 })),
                0,
                1
            )
        );
        assert!(
            answer
                .unreadable
                .first()
                .is_some_and(|line| line.starts_with("WATCH.toml")),
            "{:?}",
            answer.unreadable
        );
    }
}
