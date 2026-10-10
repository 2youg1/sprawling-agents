// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This file is the one definition of the words a scene is written in.
// The library declares it as a module, and `build.rs` includes it to
// hold `zig/part.zig` to the same names and numbers on every build, so
// the Zig reader cannot drift from the Rust writer without the build
// refusing (`crates/console_ffi/Spec.lean` D2).

/// What one tagged piece of a scene is: the kind of frame, a line of
/// the transcript, or a row of the live region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// A frame on the main screen: transcript lines, then the live region.
    Inline,
    /// A frame on the alternate screen: the quiet host.
    Quiet,
    Banner,
    You,
    Head,
    Reasoning,
    Tool,
    Reply,
    Note,
    Ended,
    Resolved,
    Waiting,
    Working,
    Calling,
    Asking,
    Composer,
    Menu,
}

impl Part {
    pub const ALL: [Part; 17] = [
        Part::Inline,
        Part::Quiet,
        Part::Banner,
        Part::You,
        Part::Head,
        Part::Reasoning,
        Part::Tool,
        Part::Reply,
        Part::Note,
        Part::Ended,
        Part::Resolved,
        Part::Waiting,
        Part::Working,
        Part::Calling,
        Part::Asking,
        Part::Composer,
        Part::Menu,
    ];

    /// The byte that tags this part in a scene.
    pub const fn number(self) -> u8 {
        match self {
            Part::Inline => 1,
            Part::Quiet => 2,
            Part::Banner => 10,
            Part::You => 11,
            Part::Head => 12,
            Part::Reasoning => 13,
            Part::Tool => 14,
            Part::Reply => 15,
            Part::Note => 16,
            Part::Ended => 17,
            Part::Resolved => 18,
            Part::Waiting => 30,
            Part::Working => 31,
            Part::Calling => 32,
            Part::Asking => 33,
            Part::Composer => 34,
            Part::Menu => 35,
        }
    }
}

/// How a tool call answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Answered,
    Failed,
}

impl Outcome {
    pub const ALL: [Outcome; 2] = [Outcome::Answered, Outcome::Failed];

    pub const fn number(self) -> u8 {
        match self {
            Outcome::Answered => 0,
            Outcome::Failed => 1,
        }
    }
}

/// How a run ended, in the three words `kernel::Completion` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Done,
    Limit,
    Cancelled,
}

impl Ending {
    pub const ALL: [Ending; 3] = [Ending::Done, Ending::Limit, Ending::Cancelled];

    pub const fn number(self) -> u8 {
        match self {
            Ending::Done => 0,
            Ending::Limit => 1,
            Ending::Cancelled => 2,
        }
    }
}

/// How a waiting request was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approved,
    Denied,
}

impl Verdict {
    pub const ALL: [Verdict; 2] = [Verdict::Approved, Verdict::Denied];

    pub const fn number(self) -> u8 {
        match self {
            Verdict::Approved => 0,
            Verdict::Denied => 1,
        }
    }
}

/// What one call into the leaf came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The frame was written.
    Drawn,
    /// The frame is longer than the buffer lent; a longer one is needed.
    Full,
    /// The scene could not be read: a defect in the writer, never input
    /// a person typed.
    Malformed,
}

impl Status {
    pub const ALL: [Status; 3] = [Status::Drawn, Status::Full, Status::Malformed];

    pub const fn number(self) -> u32 {
        match self {
            Status::Drawn => 0,
            Status::Full => 1,
            Status::Malformed => 2,
        }
    }
}
