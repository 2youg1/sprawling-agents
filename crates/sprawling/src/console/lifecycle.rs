// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which face a served process shows, and what each event does on each
//! face (`crates/sprawling/spec/Console/Lifecycle.lean`, §8-11).
//!
//! Pure and total: [`step`] reads no clock, no terminal and no desk. The
//! shell that feeds it events and carries out what a step means is
//! `bin::assembly::listening::lifetime`; the first cause a step gives is
//! the one `CommandDesk::close` keeps, which is the model's "the first
//! cause stands".
//!
//! The point a reader most often gets wrong: a typed Ctrl+C is not an
//! event here. In raw mode it is a key, and the console answers it; only
//! the signal form reaches this table, and on an interactive face it
//! changes nothing.

use accounting::worker::ClosedBy;
use wire::CloseMode;

/// What the process is showing, from opening to gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Face {
    /// The ledger folds and the port is taken; there is no CLI yet.
    Opening,
    /// The main screen, raw mode, the session inline.
    Cli,
    /// The alternate screen: the address, the pairing code, one
    /// transient line.
    QuietHost,
    /// No terminal is the city's: a pipe, a service, a harness.
    Headless,
    /// The desk is closed; the runs under way land or are stopped.
    Stopping { mode: CloseMode, deadline: Deadline },
    /// The process ends, with or without its handoff.
    Gone(Handoff),
}

/// Whether a close has a time limit. Only a lost terminal arms one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Deadline {
    Unarmed,
    Armed,
}

/// Whether the process wrote its handoff before it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handoff {
    Written,
    Skipped,
}

/// The face opening ends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Cli,
    QuietHost,
    Headless,
}

/// Who typed `/quit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asker {
    Console,
    Page,
}

/// What reached the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    Ready(Surface),
    /// `/web` in the CLI; in the quiet host, Enter opening the page again.
    Web,
    /// Esc in the quiet host.
    Back,
    Quit(Asker, CloseMode),
    /// `SIGINT`, or `CTRL_C_EVENT`, as a signal rather than a key.
    InterruptSignal,
    /// Windows' Ctrl+Break, which is always a signal.
    BreakSignal,
    /// `SIGTERM`.
    #[cfg_attr(
        all(windows, not(test)),
        expect(
            dead_code,
            reason = "Windows has no SIGTERM; the variant stays so the model's table is one table on every platform"
        )
    )]
    Terminate,
    /// The window closed, the line hung up, or reading the terminal failed.
    TerminalLost,
    /// `wire::serve` returned an error.
    Failed,
    /// The worker wrote its handoff and ended.
    Landed,
    /// The armed time limit passed.
    Deadline,
}

/// Why the city is closing, given by the step that enters `Stopping`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    Closed(ClosedBy),
    Failed,
}

impl Face {
    /// Whether the keyboard is the city's on this face.
    fn interactive(self) -> bool {
        match self {
            Face::Cli | Face::QuietHost => true,
            Face::Opening | Face::Headless | Face::Stopping { .. } | Face::Gone(_) => false,
        }
    }
}

/// One step: the next face, and the cause this step gives, which only
/// the step entering `Stopping` does.
pub(crate) fn step(face: Face, event: Event) -> (Face, Option<Cause>) {
    match face {
        Face::Stopping { mode, deadline } => (stopping(mode, deadline, event), None),
        Face::Gone(handoff) => (Face::Gone(handoff), None),
        Face::Opening | Face::Cli | Face::QuietHost | Face::Headless => serving(face, event),
    }
}

/// A face that still serves meets a request to close: which mode, with
/// or without a time limit, and why.
fn asked(face: Face, event: Event) -> Option<(CloseMode, Deadline, Cause)> {
    let unarmed = |mode, by| Some((mode, Deadline::Unarmed, Cause::Closed(by)));
    match event {
        Event::Quit(Asker::Console, mode) => unarmed(mode, ClosedBy::Console),
        Event::Quit(Asker::Page, mode) => unarmed(mode, ClosedBy::Page),
        // On an interactive face the keyboard is the city's, so this
        // signal came from some other process and is not a request.
        Event::InterruptSignal if face.interactive() => None,
        Event::InterruptSignal => unarmed(CloseMode::Drain, ClosedBy::InterruptSignal),
        Event::BreakSignal => unarmed(CloseMode::Drain, ClosedBy::BreakSignal),
        Event::Terminate => unarmed(CloseMode::Interrupt, ClosedBy::Terminate),
        Event::TerminalLost if face.interactive() => Some((
            CloseMode::Interrupt,
            Deadline::Armed,
            Cause::Closed(ClosedBy::TerminalLost),
        )),
        Event::TerminalLost => None,
        Event::Failed => Some((CloseMode::Drain, Deadline::Unarmed, Cause::Failed)),
        Event::Ready(_) | Event::Web | Event::Back | Event::Landed | Event::Deadline => None,
    }
}

fn serving(face: Face, event: Event) -> (Face, Option<Cause>) {
    if let Some((mode, deadline, cause)) = asked(face, event) {
        return (Face::Stopping { mode, deadline }, Some(cause));
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

/// A second explicit stop while closing: a drain becomes an interrupt,
/// and an interrupt ends the process without its handoff.
fn again(mode: CloseMode, deadline: Deadline) -> Face {
    match mode {
        CloseMode::Drain => Face::Stopping {
            mode: CloseMode::Interrupt,
            deadline,
        },
        CloseMode::Interrupt => Face::Gone(Handoff::Skipped),
    }
}

fn stopping(mode: CloseMode, deadline: Deadline, event: Event) -> Face {
    match event {
        Event::Quit(..) | Event::InterruptSignal | Event::BreakSignal => again(mode, deadline),
        // The time limit belongs to whoever sent it, so a second one ends
        // the process.
        Event::Terminate => Face::Gone(Handoff::Skipped),
        // Not an escalation: the terminal is gone and the platform ends
        // the process within seconds, so the runs are stopped and the
        // limit is armed.
        Event::TerminalLost => Face::Stopping {
            mode: CloseMode::Interrupt,
            deadline: Deadline::Armed,
        },
        Event::Landed => Face::Gone(Handoff::Written),
        Event::Deadline => match deadline {
            Deadline::Armed => Face::Gone(Handoff::Skipped),
            Deadline::Unarmed => Face::Stopping { mode, deadline },
        },
        Event::Ready(_) | Event::Web | Event::Back | Event::Failed => {
            Face::Stopping { mode, deadline }
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
    use super::{Asker, Cause, Deadline, Event, Face, Handoff, Surface, step};
    use accounting::worker::ClosedBy;
    use wire::CloseMode;

    /// `crates/sprawling/spec/Console/Lifecycle.lean` section 5, as its
    /// `#eval` prints it: face, event, next face, cause.
    const VECTORS: &str = "\
Opening Ready/Cli Cli -
Opening Ready/QuietHost QuietHost -
Opening Ready/Headless Headless -
Opening Web Opening -
Opening Back Opening -
Opening Quit/Console/Drain Stopping/Drain/Unarmed Console
Opening Quit/Console/Interrupt Stopping/Interrupt/Unarmed Console
Opening Quit/Page/Drain Stopping/Drain/Unarmed Page
Opening Quit/Page/Interrupt Stopping/Interrupt/Unarmed Page
Opening InterruptSignal Stopping/Drain/Unarmed InterruptSignal
Opening BreakSignal Stopping/Drain/Unarmed BreakSignal
Opening Terminate Stopping/Interrupt/Unarmed Terminate
Opening TerminalLost Opening -
Opening Failed Stopping/Drain/Unarmed Failed
Opening Landed Opening -
Opening Deadline Opening -
Cli Ready/Cli Cli -
Cli Ready/QuietHost Cli -
Cli Ready/Headless Cli -
Cli Web QuietHost -
Cli Back Cli -
Cli Quit/Console/Drain Stopping/Drain/Unarmed Console
Cli Quit/Console/Interrupt Stopping/Interrupt/Unarmed Console
Cli Quit/Page/Drain Stopping/Drain/Unarmed Page
Cli Quit/Page/Interrupt Stopping/Interrupt/Unarmed Page
Cli InterruptSignal Cli -
Cli BreakSignal Stopping/Drain/Unarmed BreakSignal
Cli Terminate Stopping/Interrupt/Unarmed Terminate
Cli TerminalLost Stopping/Interrupt/Armed TerminalLost
Cli Failed Stopping/Drain/Unarmed Failed
Cli Landed Cli -
Cli Deadline Cli -
QuietHost Ready/Cli QuietHost -
QuietHost Ready/QuietHost QuietHost -
QuietHost Ready/Headless QuietHost -
QuietHost Web QuietHost -
QuietHost Back Cli -
QuietHost Quit/Console/Drain Stopping/Drain/Unarmed Console
QuietHost Quit/Console/Interrupt Stopping/Interrupt/Unarmed Console
QuietHost Quit/Page/Drain Stopping/Drain/Unarmed Page
QuietHost Quit/Page/Interrupt Stopping/Interrupt/Unarmed Page
QuietHost InterruptSignal QuietHost -
QuietHost BreakSignal Stopping/Drain/Unarmed BreakSignal
QuietHost Terminate Stopping/Interrupt/Unarmed Terminate
QuietHost TerminalLost Stopping/Interrupt/Armed TerminalLost
QuietHost Failed Stopping/Drain/Unarmed Failed
QuietHost Landed QuietHost -
QuietHost Deadline QuietHost -
Headless Ready/Cli Headless -
Headless Ready/QuietHost Headless -
Headless Ready/Headless Headless -
Headless Web Headless -
Headless Back Headless -
Headless Quit/Console/Drain Stopping/Drain/Unarmed Console
Headless Quit/Console/Interrupt Stopping/Interrupt/Unarmed Console
Headless Quit/Page/Drain Stopping/Drain/Unarmed Page
Headless Quit/Page/Interrupt Stopping/Interrupt/Unarmed Page
Headless InterruptSignal Stopping/Drain/Unarmed InterruptSignal
Headless BreakSignal Stopping/Drain/Unarmed BreakSignal
Headless Terminate Stopping/Interrupt/Unarmed Terminate
Headless TerminalLost Headless -
Headless Failed Stopping/Drain/Unarmed Failed
Headless Landed Headless -
Headless Deadline Headless -
Stopping/Drain/Unarmed Ready/Cli Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Ready/QuietHost Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Ready/Headless Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Web Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Back Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Quit/Console/Drain Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed Quit/Console/Interrupt Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed Quit/Page/Drain Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed Quit/Page/Interrupt Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed InterruptSignal Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed BreakSignal Stopping/Interrupt/Unarmed -
Stopping/Drain/Unarmed Terminate Gone/Skipped -
Stopping/Drain/Unarmed TerminalLost Stopping/Interrupt/Armed -
Stopping/Drain/Unarmed Failed Stopping/Drain/Unarmed -
Stopping/Drain/Unarmed Landed Gone/Written -
Stopping/Drain/Unarmed Deadline Stopping/Drain/Unarmed -
Stopping/Drain/Armed Ready/Cli Stopping/Drain/Armed -
Stopping/Drain/Armed Ready/QuietHost Stopping/Drain/Armed -
Stopping/Drain/Armed Ready/Headless Stopping/Drain/Armed -
Stopping/Drain/Armed Web Stopping/Drain/Armed -
Stopping/Drain/Armed Back Stopping/Drain/Armed -
Stopping/Drain/Armed Quit/Console/Drain Stopping/Interrupt/Armed -
Stopping/Drain/Armed Quit/Console/Interrupt Stopping/Interrupt/Armed -
Stopping/Drain/Armed Quit/Page/Drain Stopping/Interrupt/Armed -
Stopping/Drain/Armed Quit/Page/Interrupt Stopping/Interrupt/Armed -
Stopping/Drain/Armed InterruptSignal Stopping/Interrupt/Armed -
Stopping/Drain/Armed BreakSignal Stopping/Interrupt/Armed -
Stopping/Drain/Armed Terminate Gone/Skipped -
Stopping/Drain/Armed TerminalLost Stopping/Interrupt/Armed -
Stopping/Drain/Armed Failed Stopping/Drain/Armed -
Stopping/Drain/Armed Landed Gone/Written -
Stopping/Drain/Armed Deadline Gone/Skipped -
Stopping/Interrupt/Unarmed Ready/Cli Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Ready/QuietHost Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Ready/Headless Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Web Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Back Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Quit/Console/Drain Gone/Skipped -
Stopping/Interrupt/Unarmed Quit/Console/Interrupt Gone/Skipped -
Stopping/Interrupt/Unarmed Quit/Page/Drain Gone/Skipped -
Stopping/Interrupt/Unarmed Quit/Page/Interrupt Gone/Skipped -
Stopping/Interrupt/Unarmed InterruptSignal Gone/Skipped -
Stopping/Interrupt/Unarmed BreakSignal Gone/Skipped -
Stopping/Interrupt/Unarmed Terminate Gone/Skipped -
Stopping/Interrupt/Unarmed TerminalLost Stopping/Interrupt/Armed -
Stopping/Interrupt/Unarmed Failed Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Unarmed Landed Gone/Written -
Stopping/Interrupt/Unarmed Deadline Stopping/Interrupt/Unarmed -
Stopping/Interrupt/Armed Ready/Cli Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Ready/QuietHost Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Ready/Headless Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Web Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Back Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Quit/Console/Drain Gone/Skipped -
Stopping/Interrupt/Armed Quit/Console/Interrupt Gone/Skipped -
Stopping/Interrupt/Armed Quit/Page/Drain Gone/Skipped -
Stopping/Interrupt/Armed Quit/Page/Interrupt Gone/Skipped -
Stopping/Interrupt/Armed InterruptSignal Gone/Skipped -
Stopping/Interrupt/Armed BreakSignal Gone/Skipped -
Stopping/Interrupt/Armed Terminate Gone/Skipped -
Stopping/Interrupt/Armed TerminalLost Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Failed Stopping/Interrupt/Armed -
Stopping/Interrupt/Armed Landed Gone/Written -
Stopping/Interrupt/Armed Deadline Gone/Skipped -
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
            ["Stopping", m, d] => Face::Stopping {
                mode: mode(m),
                deadline: match *d {
                    "Unarmed" => Deadline::Unarmed,
                    "Armed" => Deadline::Armed,
                    other => panic!("no deadline {other}"),
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

    /// Every face and every event, as the model steps them. A step that
    /// leaves the model turns this red at the row that differs.
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
