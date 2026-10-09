// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city tells the terminal it runs in about its own state, in
//! the Program Status Protocol, OSC 7501
//! (`crates/sprawling/spec/Console/ProgramStatus.lean`, sprawling D77).
//!
//! Pure and total: [`judged`] reads two counts and how the last run
//! ended, and [`step`] says what the terminal's one writer reports next,
//! if anything. The writer feeds it and queues the [`Report`] it answers
//! as a crossterm command, so a Windows console without virtual-terminal
//! sequences takes the WinAPI arm, which writes nothing.
//!
//! The point a reader most often gets wrong: a report carries counts and
//! never a word from a record. [`State`] holds numbers and a [`Rest`],
//! so what a person typed, what a model replied, a path or a room
//! address cannot reach the terminal through it.

use std::fmt;

use base64::Engine as _;
use kernel::{Completion, EventKind, EventRecord};

/// The name a terminal shows beside the record, in the protocol's `app`
/// character set.
const APP: &str = "sprawling";

/// The variable that turns reporting off, for a terminal that shows an
/// unknown OSC instead of ignoring it.
const SWITCH: &str = "SPRAWLING_PROGRAM_STATUS";

/// What the variable says to turn reporting off, the word `SPRAWLING_OPEN`
/// uses for the same refusal.
const NEVER: &str = "never";

/// Whether this console reports at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reporting {
    On,
    Off,
}

impl Reporting {
    pub(crate) fn from_environment() -> Reporting {
        deciding(std::env::var(SWITCH).ok().as_deref())
    }
}

/// The rule itself, given what the environment said, so it is asserted
/// without a test setting a process-wide variable.
fn deciding(variable: Option<&str>) -> Reporting {
    match variable {
        Some(NEVER) => Reporting::Off,
        Some(_) | None => Reporting::On,
    }
}

/// What the city reports once no run is going and no request waits: how
/// the last run that ended, ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rest {
    Idle,
    Done,
    Failed,
}

impl Rest {
    /// The rest a `run_frozen` record leaves, when `record` is one, or
    /// why its payload could not be read, which the writer prints where a
    /// line can go and otherwise keeps the rest it had.
    ///
    /// `Completion::name` spells the three words: a run a person
    /// cancelled reports `idle`, as the protocol asks, a run its ceiling
    /// cut reports `error`, and the remaining word is `done`.
    pub(crate) fn after(record: &EventRecord) -> Result<Option<Rest>, kernel::AxError> {
        if record.kind() != EventKind::RunFrozen {
            return Ok(None);
        }
        let frozen = record.data().read::<kernel::event::record::RunFrozen>()?;
        Ok(Some(Rest::of_completion(&frozen.completion)))
    }

    fn of_completion(word: &str) -> Rest {
        if word == Completion::Cancelled.name() {
            Rest::Idle
        } else if word == Completion::Limit.name() {
            Rest::Failed
        } else {
            Rest::Done
        }
    }
}

/// The two numbers of a `MetricsAnswer` a state is judged from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) runs: u64,
    pub(crate) approvals: u64,
}

impl Counts {
    pub(crate) fn of(vitals: &wire::MetricsAnswer) -> Counts {
        Counts {
            runs: vitals.runs_active,
            approvals: vitals.approvals_waiting,
        }
    }
}

/// The state the city reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Resting(Rest),
    Working { runs: u64 },
    Blocked { approvals: u64 },
}

/// A request waiting outranks a run going, which outranks how the last
/// run ended (D77).
pub(crate) fn judged(counts: Counts, rest: Rest) -> State {
    if counts.approvals > 0 {
        State::Blocked {
            approvals: counts.approvals,
        }
    } else if counts.runs > 0 {
        State::Working { runs: counts.runs }
    } else {
        State::Resting(rest)
    }
}

/// What the city is told about, as the writer meets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    /// A run ended, and this is the rest it leaves.
    Ended(Rest),
    /// The city's counts were read.
    Counted(Counts),
    /// The console hands the terminal back.
    Closed,
}

/// Whether a record of this kind can move what the city reports, so the
/// writer reads the counts again.
pub(crate) fn moves(kind: EventKind) -> bool {
    kind == EventKind::RunStarted
        || kind == EventKind::RunFrozen
        || kind == EventKind::ApprovalRequested
        || kind == EventKind::ApprovalResolved
}

/// What the terminal holds for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shown {
    Nothing,
    Reported(State),
    Cleared,
}

/// What the writer holds between events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) rest: Rest,
    pub(crate) shown: Shown,
}

impl Held {
    pub(crate) const OPENING: Held = Held {
        rest: Rest::Idle,
        shown: Shown::Nothing,
    };
}

/// One step: what the writer holds next, and the report it writes, if
/// any. Nothing is written twice in a row, and nothing after the clear.
pub(crate) fn step(held: Held, event: Event) -> (Held, Option<Report>) {
    match (held.shown, event) {
        (Shown::Cleared, _) => (held, None),
        (_, Event::Ended(rest)) => (Held { rest, ..held }, None),
        (shown, Event::Counted(counts)) => {
            let state = judged(counts, held.rest);
            if shown == Shown::Reported(state) {
                (held, None)
            } else {
                (
                    Held {
                        shown: Shown::Reported(state),
                        ..held
                    },
                    Some(Report::Of(state)),
                )
            }
        }
        (Shown::Nothing, Event::Closed) => (
            Held {
                shown: Shown::Cleared,
                ..held
            },
            None,
        ),
        (Shown::Reported(_), Event::Closed) => (
            Held {
                shown: Shown::Cleared,
                ..held
            },
            Some(Report::Cleared),
        ),
    }
}

/// One report, as the writer queues it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Report {
    Of(State),
    Cleared,
}

impl Report {
    /// The protocol's `state` word, and the `kind` a blocked state adds.
    fn pairs(self) -> (&'static str, Option<&'static str>) {
        match self {
            Report::Of(State::Resting(Rest::Idle)) => ("idle", None),
            Report::Of(State::Resting(Rest::Done)) => ("done", None),
            Report::Of(State::Resting(Rest::Failed)) => ("error", None),
            Report::Of(State::Working { .. }) => ("working", None),
            Report::Of(State::Blocked { .. }) => ("blocked", Some("permission")),
            Report::Cleared => ("clear", None),
        }
    }

    /// The one line a person may read beside the state: counts, or how
    /// the last run ended, and nothing a record says.
    fn message(self) -> Option<String> {
        match self {
            Report::Of(State::Working { runs: 1 }) => Some("1 run is going".to_owned()),
            Report::Of(State::Working { runs }) => Some(format!("{runs} runs are going")),
            Report::Of(State::Blocked { approvals: 1 }) => {
                Some("1 request waits for an answer".to_owned())
            }
            Report::Of(State::Blocked { approvals }) => {
                Some(format!("{approvals} requests wait for an answer"))
            }
            Report::Of(State::Resting(Rest::Done)) => Some("every run has finished".to_owned()),
            Report::Of(State::Resting(Rest::Failed)) => {
                Some("a run stopped at its limit".to_owned())
            }
            Report::Of(State::Resting(Rest::Idle)) | Report::Cleared => None,
        }
    }
}

impl crossterm::Command for Report {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        let (state, kind) = self.pairs();
        write!(f, "\x1b]7501;state={state}")?;
        if let Report::Of(_) = self {
            write!(f, ":app={APP}")?;
        }
        if let Some(kind) = kind {
            write!(f, ":kind={kind}")?;
        }
        if let Some(message) = self.message() {
            let encoded = base64::engine::general_purpose::STANDARD.encode(message);
            write!(f, ":msg={encoded}")?;
        }
        f.write_str("\x1b\\")
    }

    /// A console that takes no virtual-terminal sequences has nowhere
    /// to keep a program's state, so there is nothing to call.
    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::{Counts, Event, Held, Report, Rest, Shown, State, judged, step};
    use base64::Engine as _;

    /// `crates/sprawling/spec/Console/ProgramStatus.lean` section 5, as
    /// its `#eval` prints it.
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

    /// Every judgement and every step the model prints. A row the
    /// implementation answers differently is named in the failure.
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

    /// What the protocol refuses in one report, checked the way a
    /// terminal checks it: the frame, the size, the key and value
    /// character sets, the `app` name, and `msg` as base64 of UTF-8
    /// text with no control character.
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

    /// Every report the city can write is one a terminal keeps, from the
    /// smallest counts to the largest.
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
}
