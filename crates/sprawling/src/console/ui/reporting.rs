// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How the terminal's one writer keeps the city's program status
//! (`crates/sprawling/spec/Console/ProgramStatus.lean`, sprawling D77):
//! the records that move it, the counts read again, and the report the
//! model's step answers queued on the screen. A console told
//! `SPRAWLING_PROGRAM_STATUS=never` asks nothing and writes nothing.

use super::super::program_status::{self, Counts, Reporting, Rest};
use super::Ui;

impl Ui {
    /// A record that can move the city's state: the rest a run left is
    /// kept, then the counts are read again (`program_status`).
    pub(super) fn heard(&mut self, record: &kernel::EventRecord) {
        if self.reporting == Reporting::Off || !program_status::moves(record.kind()) {
            return;
        }
        match Rest::after(record) {
            Ok(Some(rest)) => self.status(program_status::Event::Ended(rest)),
            Ok(None) => {}
            Err(err) => self.line(&format!(
                "  the terminal's status keeps how the last run ended: a finished run could not be read ({err})"
            )),
        }
        self.recount();
    }

    /// The city's counts read again and judged; a city too busy to
    /// answer is asked again at the next record that moves its state.
    pub(super) fn recount(&mut self) {
        if self.reporting == Reporting::Off {
            return;
        }
        if let Some(vitals) = super::super::cli::metrics(&self.inside) {
            self.status(program_status::Event::Counted(Counts::of(&vitals)));
        }
    }

    /// One event of the program status, and the report it writes, if any.
    pub(super) fn status(&mut self, event: program_status::Event) {
        if self.reporting == Reporting::Off {
            return;
        }
        let (held, report) = program_status::step(self.held, event);
        self.held = held;
        if let Some(report) = report {
            let written = self.screen.report(report);
            self.drawn(written);
        }
    }
}
