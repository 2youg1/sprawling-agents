// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Finished,

    Absent,

    NoRoom,

    Measuring,

    Listing,

    NoWindow,

    Context,

    Bitmap,

    Selecting,

    Drawing,

    Reading,

    ShortRows,

    Owner,

    Opening,

    Fetching,

    Locking,

    EmptyBlock,

    Allocating,

    Emptying,

    Handing,

    CpuSets,

    Affinity,

    Throttling,

    JobShare,

    JobWatch,
}

impl Step {
    pub const ALL: [Step; 25] = [
        Step::Finished,
        Step::Absent,
        Step::NoRoom,
        Step::Measuring,
        Step::Listing,
        Step::NoWindow,
        Step::Context,
        Step::Bitmap,
        Step::Selecting,
        Step::Drawing,
        Step::Reading,
        Step::ShortRows,
        Step::Owner,
        Step::Opening,
        Step::Fetching,
        Step::Locking,
        Step::EmptyBlock,
        Step::Allocating,
        Step::Emptying,
        Step::Handing,
        Step::CpuSets,
        Step::Affinity,
        Step::Throttling,
        Step::JobShare,
        Step::JobWatch,
    ];

    pub const fn number(self) -> u32 {
        match self {
            Step::Finished => 0,
            Step::Absent => 1,
            Step::NoRoom => 2,
            Step::Measuring => 3,
            Step::Listing => 4,
            Step::NoWindow => 5,
            Step::Context => 6,
            Step::Bitmap => 7,
            Step::Selecting => 8,
            Step::Drawing => 9,
            Step::Reading => 10,
            Step::ShortRows => 11,
            Step::Owner => 12,
            Step::Opening => 13,
            Step::Fetching => 14,
            Step::Locking => 15,
            Step::EmptyBlock => 16,
            Step::Allocating => 17,
            Step::Emptying => 18,
            Step::Handing => 19,
            Step::CpuSets => 20,
            Step::Affinity => 21,
            Step::Throttling => 22,
            Step::JobShare => 23,
            Step::JobWatch => 24,
        }
    }
}
