// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::mpsc::SyncSender;

use crossterm::event::KeyEventKind;

use super::Show;

pub(super) fn read_keys(to: SyncSender<Show>) {
    std::thread::spawn(move || {
        loop {
            let show = match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(key)) if key.kind != KeyEventKind::Release => {
                    Show::Key(key)
                }
                Ok(crossterm::event::Event::Paste(pasted)) => Show::Paste(pasted),
                Ok(crossterm::event::Event::Resize(..)) => Show::Resized,
                Ok(_) => continue,
                Err(_) => Show::Lost,
            };
            let lost = matches!(show, Show::Lost);
            if to.send(show).is_err() || lost {
                return;
            }
        }
    });
}

pub(super) fn read_records(
    to: SyncSender<Show>,
    mut watching: tokio::sync::broadcast::Receiver<wire::Committed>,
) {
    use tokio::sync::broadcast::error::RecvError;
    std::thread::spawn(move || {
        loop {
            let show = match watching.blocking_recv() {
                Ok(committed) => Show::Record(committed),
                Err(RecvError::Lagged(missed)) => Show::Line(format!(
                    "  {missed} records were not shown here; the WebUI has them"
                )),
                Err(RecvError::Closed) => return,
            };
            drop(to.try_send(show));
        }
    });
}
