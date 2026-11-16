// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A9's structural half: there is no way to interrupt inside a phase —
// interrupts are transition parameters, and no cancel method exists.

fn main() {
    let mut now = || -> Result<kernel::TimeMs, kernel::AxError> { Ok(kernel::TimeMs::new(0)) };
    let mut lines = runtime::turn::HeldLines::open(kernel::RunId::CITY, "who".to_owned());
    let turn = runtime::turn::Turn::begin(&mut lines, kernel::TimeMs::new(0), &mut now);
    turn.cancel();
}
