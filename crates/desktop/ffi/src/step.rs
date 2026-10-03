// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This file is the one definition of the leaf's step vocabulary. The
// library declares it as a module, and `build.rs` includes it to hold
// `zig/step.zig` to the same names and numbers on every build, so the
// Zig spelling cannot drift without the build refusing
// (`crates/desktop/Spec.lean` D12).

/// Where one call into the Zig leaf ended: finished, or the part of the
/// operation that stopped it.
///
/// A step is what lets the caller say *what* this machine refused to do
/// in a caller's words; the Win32 error code beside it says why, in the
/// machine's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The operation did what it was asked.
    Finished,
    /// The operation finished and found nothing: no text on the clipboard.
    Absent,
    /// The caller's buffer is shorter than the answer; the count beside
    /// the step says how long the answer is.
    NoRoom,
    /// A size the caller gave does not describe the buffer it lent, or a
    /// size overflows the type it is counted in.
    Measuring,
    /// `EnumWindows` refused.
    Listing,
    /// No window was named, and a null window means the whole screen.
    NoWindow,
    /// `GetDC` gave no drawing context for the window.
    Context,
    /// No compatible memory context or bitmap of this size.
    Bitmap,
    /// The bitmap could not be selected into the memory context.
    Selecting,
    /// `PrintWindow` refused to draw the window.
    Drawing,
    /// `GetDIBits` read no rows back.
    Reading,
    /// `GetDIBits` read back fewer rows than the window has.
    ShortRows,
    /// The message-only window that owns the clipboard was not made.
    Owner,
    /// Another program holds the clipboard.
    Opening,
    /// The clipboard reported text and then gave no block for it.
    Fetching,
    /// The clipboard's block would not lock.
    Locking,
    /// The clipboard's block reported a size of nothing.
    EmptyBlock,
    /// No global block for the new text.
    Allocating,
    /// The clipboard would not be emptied before writing.
    Emptying,
    /// The clipboard was emptied and refused the new block.
    Handing,
    /// `GetSystemCpuSetInformation` refused.
    CpuSets,
    /// `GetThreadGroupAffinity` refused.
    Affinity,
    /// `SetProcessInformation` refused the power-throttling state.
    Throttling,
    /// A job refused its CPU weight, or the reading or writing of its
    /// memory limit; with no reason of the machine's, no job was named.
    JobShare,
}

impl Step {
    /// Every step, in the order of its number.
    pub const ALL: [Step; 24] = [
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
    ];

    /// The number this step crosses the boundary as.
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
        }
    }
}
