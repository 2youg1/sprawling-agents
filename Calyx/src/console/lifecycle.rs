// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use accounting::worker::ClosedBy;
use wire::CloseMode;

pub const INTERRUPT_GRACE: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Opening,
    Cli,
    QuietHost,
    Headless,
    Stopping { mode: CloseMode, sinks: Sinks },
    Gone(Handoff),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sinks {
    Held,
    Cut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handoff {
    Written,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Cli,
    QuietHost,
    Headless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asker {
    Console,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Ready(Surface),
    Web,
    Back,
    Quit(Asker, CloseMode),
    InterruptSignal,
    BreakSignal,
    Terminate,
    TerminalLost,
    Failed,
    Landed,
    Deadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Closed(ClosedBy),
    Failed,
}

impl Face {
    pub fn armed(self) -> bool {
        match self {
            Face::Stopping {
                mode: CloseMode::Interrupt,
                ..
            } => true,
            Face::Stopping {
                mode: CloseMode::Drain,
                ..
            }
            | Face::Opening
            | Face::Cli
            | Face::QuietHost
            | Face::Headless
            | Face::Gone(_) => false,
        }
    }

    fn interactive(self) -> bool {
        match self {
            Face::Cli | Face::QuietHost => true,
            Face::Opening | Face::Headless | Face::Stopping { .. } | Face::Gone(_) => false,
        }
    }
}

pub fn step(face: Face, event: Event) -> (Face, Option<Cause>) {
    match face {
        Face::Stopping { mode, sinks } => (stopping(mode, sinks, event), None),
        Face::Gone(handoff) => (Face::Gone(handoff), None),
        Face::Opening | Face::Cli | Face::QuietHost | Face::Headless => serving(face, event),
    }
}

fn asked(face: Face, event: Event) -> Option<(CloseMode, Sinks, Cause)> {
    let held = |mode, by| Some((mode, Sinks::Held, Cause::Closed(by)));
    match event {
        Event::Quit(Asker::Console, mode) => held(mode, ClosedBy::Console),
        Event::Quit(Asker::Page, mode) => held(mode, ClosedBy::Page),
        Event::InterruptSignal if face.interactive() => None,
        Event::InterruptSignal => held(CloseMode::Drain, ClosedBy::InterruptSignal),
        Event::BreakSignal => held(CloseMode::Drain, ClosedBy::BreakSignal),
        Event::Terminate => held(CloseMode::Interrupt, ClosedBy::Terminate),
        Event::TerminalLost if face.interactive() => Some((
            CloseMode::Interrupt,
            Sinks::Cut,
            Cause::Closed(ClosedBy::TerminalLost),
        )),
        Event::TerminalLost => None,
        Event::Failed => Some((CloseMode::Drain, Sinks::Held, Cause::Failed)),
        Event::Ready(_) | Event::Web | Event::Back | Event::Landed | Event::Deadline => None,
    }
}

fn serving(face: Face, event: Event) -> (Face, Option<Cause>) {
    if let Some((mode, sinks, cause)) = asked(face, event) {
        return (Face::Stopping { mode, sinks }, Some(cause));
    }
    let next = match (face, event) {
        (Face::Opening, Event::Ready(Surface::Cli)) | (Face::QuietHost, Event::Back) => Face::Cli,
        (Face::Opening, Event::Ready(Surface::QuietHost)) | (Face::Cli, Event::Web) => {
            Face::QuietHost
        }
        (Face::Opening, Event::Ready(Surface::Headless)) => Face::Headless,
        (other, _) => other,
    };
    (next, None)
}

fn again(mode: CloseMode, sinks: Sinks) -> Face {
    match mode {
        CloseMode::Drain => Face::Stopping {
            mode: CloseMode::Interrupt,
            sinks,
        },
        CloseMode::Interrupt => Face::Gone(Handoff::Skipped),
    }
}

fn stopping(mode: CloseMode, sinks: Sinks, event: Event) -> Face {
    match event {
        Event::Quit(..) | Event::InterruptSignal | Event::BreakSignal => again(mode, sinks),
        Event::Terminate => Face::Gone(Handoff::Skipped),
        Event::TerminalLost => Face::Stopping {
            mode: CloseMode::Interrupt,
            sinks: Sinks::Cut,
        },
        Event::Landed => Face::Gone(Handoff::Written),
        Event::Deadline => match mode {
            CloseMode::Interrupt => Face::Gone(Handoff::Skipped),
            CloseMode::Drain => Face::Stopping { mode, sinks },
        },
        Event::Ready(_) | Event::Web | Event::Back | Event::Failed => {
            Face::Stopping { mode, sinks }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{Asker, Cause, Event, Face, Handoff, Sinks, Surface, step};
    use accounting::worker::ClosedBy;
    use wire::CloseMode;

    const VECTORS: &str = "\
Opening Ready/Cli Cli -
Opening Ready/QuietHost QuietHost -
Opening Ready/Headless Headless -
Opening Web Opening -
Opening Back Opening -
Opening Quit/Console/Drain Stopping/Drain/Held Console
Opening Quit/Console/Interrupt Stopping/Interrupt/Held Console
Opening Quit/Page/Drain Stopping/Drain/Held Page
Opening Quit/Page/Interrupt Stopping/Interrupt/Held Page
Opening InterruptSignal Stopping/Drain/Held InterruptSignal
Opening BreakSignal Stopping/Drain/Held BreakSignal
Opening Terminate Stopping/Interrupt/Held Terminate
Opening TerminalLost Opening -
Opening Failed Stopping/Drain/Held Failed
Opening Landed Opening -
Opening Deadline Opening -
Cli Ready/Cli Cli -
Cli Ready/QuietHost Cli -
Cli Ready/Headless Cli -
Cli Web QuietHost -
Cli Back Cli -
Cli Quit/Console/Drain Stopping/Drain/Held Console
Cli Quit/Console/Interrupt Stopping/Interrupt/Held Console
Cli Quit/Page/Drain Stopping/Drain/Held Page
Cli Quit/Page/Interrupt Stopping/Interrupt/Held Page
Cli InterruptSignal Cli -
Cli BreakSignal Stopping/Drain/Held BreakSignal
Cli Terminate Stopping/Interrupt/Held Terminate
Cli TerminalLost Stopping/Interrupt/Cut TerminalLost
Cli Failed Stopping/Drain/Held Failed
Cli Landed Cli -
Cli Deadline Cli -
QuietHost Ready/Cli QuietHost -
QuietHost Ready/QuietHost QuietHost -
QuietHost Ready/Headless QuietHost -
QuietHost Web QuietHost -
QuietHost Back Cli -
QuietHost Quit/Console/Drain Stopping/Drain/Held Console
QuietHost Quit/Console/Interrupt Stopping/Interrupt/Held Console
QuietHost Quit/Page/Drain Stopping/Drain/Held Page
QuietHost Quit/Page/Interrupt Stopping/Interrupt/Held Page
QuietHost InterruptSignal QuietHost -
QuietHost BreakSignal Stopping/Drain/Held BreakSignal
QuietHost Terminate Stopping/Interrupt/Held Terminate
QuietHost TerminalLost Stopping/Interrupt/Cut TerminalLost
QuietHost Failed Stopping/Drain/Held Failed
QuietHost Landed QuietHost -
QuietHost Deadline QuietHost -
Headless Ready/Cli Headless -
Headless Ready/QuietHost Headless -
Headless Ready/Headless Headless -
Headless Web Headless -
Headless Back Headless -
Headless Quit/Console/Drain Stopping/Drain/Held Console
Headless Quit/Console/Interrupt Stopping/Interrupt/Held Console
Headless Quit/Page/Drain Stopping/Drain/Held Page
Headless Quit/Page/Interrupt Stopping/Interrupt/Held Page
Headless InterruptSignal Stopping/Drain/Held InterruptSignal
Headless BreakSignal Stopping/Drain/Held BreakSignal
Headless Terminate Stopping/Interrupt/Held Terminate
Headless TerminalLost Headless -
Headless Failed Stopping/Drain/Held Failed
Headless Landed Headless -
Headless Deadline Headless -
Stopping/Drain/Held Ready/Cli Stopping/Drain/Held -
Stopping/Drain/Held Ready/QuietHost Stopping/Drain/Held -
Stopping/Drain/Held Ready/Headless Stopping/Drain/Held -
Stopping/Drain/Held Web Stopping/Drain/Held -
Stopping/Drain/Held Back Stopping/Drain/Held -
Stopping/Drain/Held Quit/Console/Drain Stopping/Interrupt/Held -
Stopping/Drain/Held Quit/Console/Interrupt Stopping/Interrupt/Held -
Stopping/Drain/Held Quit/Page/Drain Stopping/Interrupt/Held -
Stopping/Drain/Held Quit/Page/Interrupt Stopping/Interrupt/Held -
Stopping/Drain/Held InterruptSignal Stopping/Interrupt/Held -
Stopping/Drain/Held BreakSignal Stopping/Interrupt/Held -
Stopping/Drain/Held Terminate Gone/Skipped -
Stopping/Drain/Held TerminalLost Stopping/Interrupt/Cut -
Stopping/Drain/Held Failed Stopping/Drain/Held -
Stopping/Drain/Held Landed Gone/Written -
Stopping/Drain/Held Deadline Stopping/Drain/Held -
Stopping/Drain/Cut Ready/Cli Stopping/Drain/Cut -
Stopping/Drain/Cut Ready/QuietHost Stopping/Drain/Cut -
Stopping/Drain/Cut Ready/Headless Stopping/Drain/Cut -
Stopping/Drain/Cut Web Stopping/Drain/Cut -
Stopping/Drain/Cut Back Stopping/Drain/Cut -
Stopping/Drain/Cut Quit/Console/Drain Stopping/Interrupt/Cut -
Stopping/Drain/Cut Quit/Console/Interrupt Stopping/Interrupt/Cut -
Stopping/Drain/Cut Quit/Page/Drain Stopping/Interrupt/Cut -
Stopping/Drain/Cut Quit/Page/Interrupt Stopping/Interrupt/Cut -
Stopping/Drain/Cut InterruptSignal Stopping/Interrupt/Cut -
Stopping/Drain/Cut BreakSignal Stopping/Interrupt/Cut -
Stopping/Drain/Cut Terminate Gone/Skipped -
Stopping/Drain/Cut TerminalLost Stopping/Interrupt/Cut -
Stopping/Drain/Cut Failed Stopping/Drain/Cut -
Stopping/Drain/Cut Landed Gone/Written -
Stopping/Drain/Cut Deadline Stopping/Drain/Cut -
Stopping/Interrupt/Held Ready/Cli Stopping/Interrupt/Held -
Stopping/Interrupt/Held Ready/QuietHost Stopping/Interrupt/Held -
Stopping/Interrupt/Held Ready/Headless Stopping/Interrupt/Held -
Stopping/Interrupt/Held Web Stopping/Interrupt/Held -
Stopping/Interrupt/Held Back Stopping/Interrupt/Held -
Stopping/Interrupt/Held Quit/Console/Drain Gone/Skipped -
Stopping/Interrupt/Held Quit/Console/Interrupt Gone/Skipped -
Stopping/Interrupt/Held Quit/Page/Drain Gone/Skipped -
Stopping/Interrupt/Held Quit/Page/Interrupt Gone/Skipped -
Stopping/Interrupt/Held InterruptSignal Gone/Skipped -
Stopping/Interrupt/Held BreakSignal Gone/Skipped -
Stopping/Interrupt/Held Terminate Gone/Skipped -
Stopping/Interrupt/Held TerminalLost Stopping/Interrupt/Cut -
Stopping/Interrupt/Held Failed Stopping/Interrupt/Held -
Stopping/Interrupt/Held Landed Gone/Written -
Stopping/Interrupt/Held Deadline Gone/Skipped -
Stopping/Interrupt/Cut Ready/Cli Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Ready/QuietHost Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Ready/Headless Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Web Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Back Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Quit/Console/Drain Gone/Skipped -
Stopping/Interrupt/Cut Quit/Console/Interrupt Gone/Skipped -
Stopping/Interrupt/Cut Quit/Page/Drain Gone/Skipped -
Stopping/Interrupt/Cut Quit/Page/Interrupt Gone/Skipped -
Stopping/Interrupt/Cut InterruptSignal Gone/Skipped -
Stopping/Interrupt/Cut BreakSignal Gone/Skipped -
Stopping/Interrupt/Cut Terminate Gone/Skipped -
Stopping/Interrupt/Cut TerminalLost Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Failed Stopping/Interrupt/Cut -
Stopping/Interrupt/Cut Landed Gone/Written -
Stopping/Interrupt/Cut Deadline Gone/Skipped -
Gone/Written Ready/Cli Gone/Written -
Gone/Written Ready/QuietHost Gone/Written -
Gone/Written Ready/Headless Gone/Written -
Gone/Written Web Gone/Written -
Gone/Written Back Gone/Written -
Gone/Written Quit/Console/Drain Gone/Written -
Gone/Written Quit/Console/Interrupt Gone/Written -
Gone/Written Quit/Page/Drain Gone/Written -
Gone/Written Quit/Page/Interrupt Gone/Written -
Gone/Written InterruptSignal Gone/Written -
Gone/Written BreakSignal Gone/Written -
Gone/Written Terminate Gone/Written -
Gone/Written TerminalLost Gone/Written -
Gone/Written Failed Gone/Written -
Gone/Written Landed Gone/Written -
Gone/Written Deadline Gone/Written -
Gone/Skipped Ready/Cli Gone/Skipped -
Gone/Skipped Ready/QuietHost Gone/Skipped -
Gone/Skipped Ready/Headless Gone/Skipped -
Gone/Skipped Web Gone/Skipped -
Gone/Skipped Back Gone/Skipped -
Gone/Skipped Quit/Console/Drain Gone/Skipped -
Gone/Skipped Quit/Console/Interrupt Gone/Skipped -
Gone/Skipped Quit/Page/Drain Gone/Skipped -
Gone/Skipped Quit/Page/Interrupt Gone/Skipped -
Gone/Skipped InterruptSignal Gone/Skipped -
Gone/Skipped BreakSignal Gone/Skipped -
Gone/Skipped Terminate Gone/Skipped -
Gone/Skipped TerminalLost Gone/Skipped -
Gone/Skipped Failed Gone/Skipped -
Gone/Skipped Landed Gone/Skipped -
Gone/Skipped Deadline Gone/Skipped -
";

    fn mode(word: &str) -> CloseMode {
        match word {
            "Drain" => CloseMode::Drain,
            "Interrupt" => CloseMode::Interrupt,
            other => panic!("no mode {other}"),
        }
    }

    fn face(word: &str) -> Face {
        let parts: Vec<&str> = word.split('/').collect();
        match parts.as_slice() {
            ["Opening"] => Face::Opening,
            ["Cli"] => Face::Cli,
            ["QuietHost"] => Face::QuietHost,
            ["Headless"] => Face::Headless,
            ["Stopping", m, k] => Face::Stopping {
                mode: mode(m),
                sinks: match *k {
                    "Held" => Sinks::Held,
                    "Cut" => Sinks::Cut,
                    other => panic!("no sinks {other}"),
                },
            },
            ["Gone", "Written"] => Face::Gone(Handoff::Written),
            ["Gone", "Skipped"] => Face::Gone(Handoff::Skipped),
            other => panic!("no face {other:?}"),
        }
    }

    fn event(word: &str) -> Event {
        let parts: Vec<&str> = word.split('/').collect();
        match parts.as_slice() {
            ["Ready", "Cli"] => Event::Ready(Surface::Cli),
            ["Ready", "QuietHost"] => Event::Ready(Surface::QuietHost),
            ["Ready", "Headless"] => Event::Ready(Surface::Headless),
            ["Web"] => Event::Web,
            ["Back"] => Event::Back,
            ["Quit", "Console", m] => Event::Quit(Asker::Console, mode(m)),
            ["Quit", "Page", m] => Event::Quit(Asker::Page, mode(m)),
            ["InterruptSignal"] => Event::InterruptSignal,
            ["BreakSignal"] => Event::BreakSignal,
            ["Terminate"] => Event::Terminate,
            ["TerminalLost"] => Event::TerminalLost,
            ["Failed"] => Event::Failed,
            ["Landed"] => Event::Landed,
            ["Deadline"] => Event::Deadline,
            other => panic!("no event {other:?}"),
        }
    }

    fn cause(word: &str) -> Option<Cause> {
        match word {
            "-" => None,
            "Console" => Some(Cause::Closed(ClosedBy::Console)),
            "Page" => Some(Cause::Closed(ClosedBy::Page)),
            "InterruptSignal" => Some(Cause::Closed(ClosedBy::InterruptSignal)),
            "BreakSignal" => Some(Cause::Closed(ClosedBy::BreakSignal)),
            "Terminate" => Some(Cause::Closed(ClosedBy::Terminate)),
            "TerminalLost" => Some(Cause::Closed(ClosedBy::TerminalLost)),
            "Failed" => Some(Cause::Failed),
            other => panic!("no cause {other}"),
        }
    }

    #[test]
    fn every_transition_is_the_models() {
        type Stepped = (Face, Option<Cause>);
        let rows: Vec<(&str, Stepped, Stepped)> = VECTORS
            .lines()
            .map(|line| {
                let words: Vec<&str> = line.split(' ').collect();
                let [from, on, to, why] = words.as_slice() else {
                    panic!("a vector has four words: {line}");
                };
                (line, step(face(from), event(on)), (face(to), cause(why)))
            })
            .collect();
        let differing: Vec<&str> = rows
            .iter()
            .filter(|(_, got, wanted)| got != wanted)
            .map(|(line, _, _)| *line)
            .collect();
        assert_eq!((rows.len(), differing), (160, Vec::<&str>::new()));
    }
}
