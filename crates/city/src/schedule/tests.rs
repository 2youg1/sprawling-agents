// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

const JOB: &str = "[[job]]\nname = \"sweep\"\naddr = \"lab/room1\"\n\
                   task = \"sweep the roadmap\"\ngoal = \"every row has a status\"\n";

fn at(minutes: u64) -> TimeMs {
    TimeMs::new(minutes.saturating_mul(MINUTE_MS))
}

#[test]
fn a_city_with_no_schedule_has_no_scheduled_work() {
    let dir = tempfile::tempdir().unwrap();
    let schedule = Schedule::load(dir.path()).unwrap();
    assert!(schedule.entries().is_empty());
    assert!(schedule.due(at(0), at(100_000)).is_empty());
}

#[test]
fn an_entry_fires_once_per_period_and_only_inside_the_window() {
    let schedule = Schedule::parse(&format!("{JOB}every = \"15m\"\n")).unwrap();
    assert_eq!(schedule.entries()[0].cadence(), Cadence::EveryMinutes(15));

    // 14:59 to 15:01 crosses one firing.
    assert_eq!(schedule.due(at(14), at(15)).len(), 1);
    // Inside one period, nothing fires twice.
    assert!(schedule.due(at(15), at(16)).is_empty());
    assert!(schedule.due(at(16), at(29)).is_empty());
    assert_eq!(schedule.due(at(29), at(30)).len(), 1);
}

#[test]
fn a_missed_firing_owes_one_run_rather_than_one_per_period_slept_through() {
    let schedule = Schedule::parse(&format!("{JOB}every = \"1h\"\n")).unwrap();
    // Eight hours of downtime: the city owes one run, not eight.
    let due = schedule.due(at(0), at(480));
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].name(), "sweep");
    assert_eq!(due[0].addr().as_str(), "lab/room1");
}

#[test]
fn daily_and_weekly_are_counted_in_utc_from_the_epoch() {
    let daily = Schedule::parse(&format!("{JOB}daily = \"09:00\"\n")).unwrap();
    assert_eq!(daily.entries()[0].cadence(), Cadence::DailyAt(540));
    // Day one, 08:59 to 09:00.
    assert_eq!(
        daily
            .due(at(DAY_MINUTES + 539), at(DAY_MINUTES + 540))
            .len(),
        1
    );
    assert!(
        daily
            .due(at(DAY_MINUTES + 540), at(DAY_MINUTES + 541))
            .is_empty()
    );

    // 1970-01-01 was a Thursday, so the first Monday is day four.
    let weekly = Schedule::parse(&format!("{JOB}weekly = \"mon 09:00\"\n")).unwrap();
    assert_eq!(weekly.entries()[0].cadence(), Cadence::WeeklyAt(540));
    let first_monday = 4 * DAY_MINUTES + 540;
    assert_eq!(weekly.due(at(first_monday - 1), at(first_monday)).len(), 1);
    assert!(
        weekly
            .due(at(first_monday), at(first_monday + WEEK_MINUTES - 1))
            .is_empty()
    );
    assert_eq!(
        weekly
            .due(at(first_monday), at(first_monday + WEEK_MINUTES))
            .len(),
        1
    );
}

#[test]
fn a_job_that_states_two_cadences_is_refused_rather_than_ranked() {
    let err = Schedule::parse(&format!("{JOB}every = \"15m\"\ndaily = \"09:00\"\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("two cadences"));
}

#[test]
fn a_cadence_this_version_cannot_spell_says_what_it_can() {
    for cadence in [
        "every = \"fortnightly\"\n",
        "every = \"0m\"\n",
        "daily = \"9am\"\n",
        "daily = \"25:00\"\n",
        "weekly = \"someday 09:00\"\n",
    ] {
        let err = Schedule::parse(&format!("{JOB}{cadence}")).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(err.recovery().contains("day-of-month"));
    }
}

#[test]
fn a_job_with_no_goal_does_not_parse_at_all() {
    let err = Schedule::parse(
        "[[job]]\nname = \"sweep\"\naddr = \"lab\"\ntask = \"sweep\"\nevery = \"15m\"\n",
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.subject().contains("goal"),
        "a dispatch without a stop condition is one that does not stop: {}",
        err.subject()
    );
}
