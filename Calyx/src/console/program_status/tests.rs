// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Counts, Event, Held, Report, Rest, Shown, State, judged, step};
use base64::Engine as _;

const VECTORS: &str = "\
judged 0 0 Idle Resting/Idle
judged 0 0 Done Resting/Done
judged 0 0 Failed Resting/Failed
judged 1 0 Idle Working/1
judged 1 0 Done Working/1
judged 1 0 Failed Working/1
judged 2 0 Idle Working/2
judged 2 0 Done Working/2
judged 2 0 Failed Working/2
judged 0 1 Idle Blocked/1
judged 0 1 Done Blocked/1
judged 0 1 Failed Blocked/1
judged 1 1 Idle Blocked/1
judged 1 1 Done Blocked/1
judged 1 1 Failed Blocked/1
judged 0 2 Idle Blocked/2
judged 0 2 Done Blocked/2
judged 0 2 Failed Blocked/2
judged 3 2 Idle Blocked/2
judged 3 2 Done Blocked/2
judged 3 2 Failed Blocked/2
step Nothing Idle Ended/done Nothing Done -
step Nothing Idle Ended/limit Nothing Failed -
step Nothing Idle Ended/cancelled Nothing Idle -
step Nothing Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
step Nothing Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
step Nothing Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
step Nothing Idle Closed Cleared Idle -
step Nothing Done Ended/done Nothing Done -
step Nothing Done Ended/limit Nothing Failed -
step Nothing Done Ended/cancelled Nothing Idle -
step Nothing Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
step Nothing Done Counted/1/0 Reported/Working/1 Done Of/Working/1
step Nothing Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
step Nothing Done Closed Cleared Done -
step Nothing Failed Ended/done Nothing Done -
step Nothing Failed Ended/limit Nothing Failed -
step Nothing Failed Ended/cancelled Nothing Idle -
step Nothing Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
step Nothing Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
step Nothing Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
step Nothing Failed Closed Cleared Failed -
step Reported/Working/1 Idle Ended/done Reported/Working/1 Done -
step Reported/Working/1 Idle Ended/limit Reported/Working/1 Failed -
step Reported/Working/1 Idle Ended/cancelled Reported/Working/1 Idle -
step Reported/Working/1 Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
step Reported/Working/1 Idle Counted/1/0 Reported/Working/1 Idle -
step Reported/Working/1 Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
step Reported/Working/1 Idle Closed Cleared Idle Cleared
step Reported/Working/1 Done Ended/done Reported/Working/1 Done -
step Reported/Working/1 Done Ended/limit Reported/Working/1 Failed -
step Reported/Working/1 Done Ended/cancelled Reported/Working/1 Idle -
step Reported/Working/1 Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
step Reported/Working/1 Done Counted/1/0 Reported/Working/1 Done -
step Reported/Working/1 Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
step Reported/Working/1 Done Closed Cleared Done Cleared
step Reported/Working/1 Failed Ended/done Reported/Working/1 Done -
step Reported/Working/1 Failed Ended/limit Reported/Working/1 Failed -
step Reported/Working/1 Failed Ended/cancelled Reported/Working/1 Idle -
step Reported/Working/1 Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
step Reported/Working/1 Failed Counted/1/0 Reported/Working/1 Failed -
step Reported/Working/1 Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
step Reported/Working/1 Failed Closed Cleared Failed Cleared
step Reported/Blocked/1 Idle Ended/done Reported/Blocked/1 Done -
step Reported/Blocked/1 Idle Ended/limit Reported/Blocked/1 Failed -
step Reported/Blocked/1 Idle Ended/cancelled Reported/Blocked/1 Idle -
step Reported/Blocked/1 Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
step Reported/Blocked/1 Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
step Reported/Blocked/1 Idle Counted/0/1 Reported/Blocked/1 Idle -
step Reported/Blocked/1 Idle Closed Cleared Idle Cleared
step Reported/Blocked/1 Done Ended/done Reported/Blocked/1 Done -
step Reported/Blocked/1 Done Ended/limit Reported/Blocked/1 Failed -
step Reported/Blocked/1 Done Ended/cancelled Reported/Blocked/1 Idle -
step Reported/Blocked/1 Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
step Reported/Blocked/1 Done Counted/1/0 Reported/Working/1 Done Of/Working/1
step Reported/Blocked/1 Done Counted/0/1 Reported/Blocked/1 Done -
step Reported/Blocked/1 Done Closed Cleared Done Cleared
step Reported/Blocked/1 Failed Ended/done Reported/Blocked/1 Done -
step Reported/Blocked/1 Failed Ended/limit Reported/Blocked/1 Failed -
step Reported/Blocked/1 Failed Ended/cancelled Reported/Blocked/1 Idle -
step Reported/Blocked/1 Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
step Reported/Blocked/1 Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
step Reported/Blocked/1 Failed Counted/0/1 Reported/Blocked/1 Failed -
step Reported/Blocked/1 Failed Closed Cleared Failed Cleared
step Reported/Resting/Done Idle Ended/done Reported/Resting/Done Done -
step Reported/Resting/Done Idle Ended/limit Reported/Resting/Done Failed -
step Reported/Resting/Done Idle Ended/cancelled Reported/Resting/Done Idle -
step Reported/Resting/Done Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
step Reported/Resting/Done Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
step Reported/Resting/Done Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
step Reported/Resting/Done Idle Closed Cleared Idle Cleared
step Reported/Resting/Done Done Ended/done Reported/Resting/Done Done -
step Reported/Resting/Done Done Ended/limit Reported/Resting/Done Failed -
step Reported/Resting/Done Done Ended/cancelled Reported/Resting/Done Idle -
step Reported/Resting/Done Done Counted/0/0 Reported/Resting/Done Done -
step Reported/Resting/Done Done Counted/1/0 Reported/Working/1 Done Of/Working/1
step Reported/Resting/Done Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
step Reported/Resting/Done Done Closed Cleared Done Cleared
step Reported/Resting/Done Failed Ended/done Reported/Resting/Done Done -
step Reported/Resting/Done Failed Ended/limit Reported/Resting/Done Failed -
step Reported/Resting/Done Failed Ended/cancelled Reported/Resting/Done Idle -
step Reported/Resting/Done Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
step Reported/Resting/Done Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
step Reported/Resting/Done Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
step Reported/Resting/Done Failed Closed Cleared Failed Cleared
step Reported/Resting/Idle Idle Ended/done Reported/Resting/Idle Done -
step Reported/Resting/Idle Idle Ended/limit Reported/Resting/Idle Failed -
step Reported/Resting/Idle Idle Ended/cancelled Reported/Resting/Idle Idle -
step Reported/Resting/Idle Idle Counted/0/0 Reported/Resting/Idle Idle -
step Reported/Resting/Idle Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
step Reported/Resting/Idle Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
step Reported/Resting/Idle Idle Closed Cleared Idle Cleared
step Reported/Resting/Idle Done Ended/done Reported/Resting/Idle Done -
step Reported/Resting/Idle Done Ended/limit Reported/Resting/Idle Failed -
step Reported/Resting/Idle Done Ended/cancelled Reported/Resting/Idle Idle -
step Reported/Resting/Idle Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
step Reported/Resting/Idle Done Counted/1/0 Reported/Working/1 Done Of/Working/1
step Reported/Resting/Idle Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
step Reported/Resting/Idle Done Closed Cleared Done Cleared
step Reported/Resting/Idle Failed Ended/done Reported/Resting/Idle Done -
step Reported/Resting/Idle Failed Ended/limit Reported/Resting/Idle Failed -
step Reported/Resting/Idle Failed Ended/cancelled Reported/Resting/Idle Idle -
step Reported/Resting/Idle Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
step Reported/Resting/Idle Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
step Reported/Resting/Idle Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
step Reported/Resting/Idle Failed Closed Cleared Failed Cleared
step Cleared Idle Ended/done Cleared Idle -
step Cleared Idle Ended/limit Cleared Idle -
step Cleared Idle Ended/cancelled Cleared Idle -
step Cleared Idle Counted/0/0 Cleared Idle -
step Cleared Idle Counted/1/0 Cleared Idle -
step Cleared Idle Counted/0/1 Cleared Idle -
step Cleared Idle Closed Cleared Idle -
step Cleared Done Ended/done Cleared Done -
step Cleared Done Ended/limit Cleared Done -
step Cleared Done Ended/cancelled Cleared Done -
step Cleared Done Counted/0/0 Cleared Done -
step Cleared Done Counted/1/0 Cleared Done -
step Cleared Done Counted/0/1 Cleared Done -
step Cleared Done Closed Cleared Done -
step Cleared Failed Ended/done Cleared Failed -
step Cleared Failed Ended/limit Cleared Failed -
step Cleared Failed Ended/cancelled Cleared Failed -
step Cleared Failed Counted/0/0 Cleared Failed -
step Cleared Failed Counted/1/0 Cleared Failed -
step Cleared Failed Counted/0/1 Cleared Failed -
step Cleared Failed Closed Cleared Failed -
";

fn rest(word: &str) -> Rest {
    match word {
        "Idle" => Rest::Idle,
        "Done" => Rest::Done,
        "Failed" => Rest::Failed,
        other => panic!("no rest {other}"),
    }
}

fn number(word: &str) -> u64 {
    word.parse().unwrap()
}

fn state(word: &str) -> State {
    match word.split('/').collect::<Vec<_>>().as_slice() {
        ["Resting", r] => State::Resting(rest(r)),
        ["Working", n] => State::Working { runs: number(n) },
        ["Blocked", n] => State::Blocked {
            approvals: number(n),
        },
        other => panic!("no state {other:?}"),
    }
}

fn shown(word: &str) -> Shown {
    match word.split_once('/') {
        None if word == "Nothing" => Shown::Nothing,
        None if word == "Cleared" => Shown::Cleared,
        Some(("Reported", s)) => Shown::Reported(state(s)),
        other => panic!("no shown {other:?}"),
    }
}

fn event(word: &str) -> Event {
    match word.split('/').collect::<Vec<_>>().as_slice() {
        ["Ended", completion] => Event::Ended(Rest::of_completion(completion)),
        ["Counted", runs, approvals] => Event::Counted(Counts {
            runs: number(runs),
            approvals: number(approvals),
        }),
        ["Closed"] => Event::Closed,
        other => panic!("no event {other:?}"),
    }
}

fn report(word: &str) -> Option<Report> {
    match word.split_once('/') {
        None if word == "-" => None,
        None if word == "Cleared" => Some(Report::Cleared),
        Some(("Of", s)) => Some(Report::Of(state(s))),
        other => panic!("no report {other:?}"),
    }
}

#[test]
fn every_row_is_the_models() {
    let differing: Vec<&str> = VECTORS
        .lines()
        .filter(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            match words.as_slice() {
                ["judged", runs, approvals, r, s] => {
                    let counts = Counts {
                        runs: number(runs),
                        approvals: number(approvals),
                    };
                    judged(counts, rest(r)) != state(s)
                }
                ["step", from, r, on, to, r_to, wrote] => {
                    let held = Held {
                        rest: rest(r),
                        shown: shown(from),
                    };
                    let wanted = (
                        Held {
                            rest: rest(r_to),
                            shown: shown(to),
                        },
                        report(wrote),
                    );
                    step(held, event(on)) != wanted
                }
                other => panic!("a vector row has a known shape: {other:?}"),
            }
        })
        .collect();
    assert_eq!(
        (VECTORS.lines().count(), differing),
        (147, Vec::<&str>::new())
    );
}

fn rendered(report: Report) -> String {
    let mut out = String::new();
    crossterm::Command::write_ansi(&report, &mut out).unwrap();
    out
}

fn refusals(sequence: &str) -> Vec<String> {
    let mut found = Vec::new();
    if sequence.len() > 4096 {
        found.push("longer than 4096 bytes".to_owned());
    }
    let Some(body) = sequence
        .strip_prefix("\x1b]7501;")
        .and_then(|rest| rest.strip_suffix("\x1b\\"))
    else {
        return vec![format!("not framed as OSC 7501 ... ST: {sequence:?}")];
    };
    let value_byte = |c: char| c.is_ascii_alphanumeric() || "_.,+/=-".contains(c);
    for pair in body.split(':') {
        let Some((key, value)) = pair.split_once('=') else {
            found.push(format!("a pair with no '=': {pair}"));
            continue;
        };
        if key.is_empty() || key.len() > 16 || !key.chars().all(|c| c.is_ascii_lowercase()) {
            found.push(format!("a key outside [a-z]{{1,16}}: {key}"));
        }
        if !value.chars().all(value_byte) {
            found.push(format!("a value outside the value set: {value}"));
        }
        match key {
            "app" if value.is_empty() || value.len() > 32 => {
                found.push(format!("an app name outside 1 to 32 bytes: {value}"));
            }
            "msg" => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(value)
                    .map_err(|err| err.to_string())
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|err| err.to_string()));
                match decoded {
                    Ok(text) if text.len() <= 2048 && !text.chars().any(char::is_control) => {}
                    Ok(text) => found.push(format!("a msg the protocol refuses: {text:?}")),
                    Err(err) => found.push(format!("a msg that does not decode: {err}")),
                }
            }
            _ => {}
        }
    }
    if !body.starts_with("state=") {
        found.push(format!("no state first: {body}"));
    }
    found
}

#[test]
fn every_report_is_one_a_terminal_keeps() {
    let states = [1, 2, u64::MAX]
        .into_iter()
        .flat_map(|n| [State::Working { runs: n }, State::Blocked { approvals: n }]);
    let reports: Vec<Report> = [Rest::Idle, Rest::Done, Rest::Failed]
        .into_iter()
        .map(State::Resting)
        .chain(states)
        .map(Report::Of)
        .chain([Report::Cleared])
        .collect();
    let refused: Vec<(Report, Vec<String>)> = reports
        .into_iter()
        .map(|report| (report, refusals(&rendered(report))))
        .filter(|(_, found)| !found.is_empty())
        .collect();
    assert_eq!(refused, Vec::new());
    assert_eq!(rendered(Report::Cleared), "\x1b]7501;state=clear\x1b\\");
}
