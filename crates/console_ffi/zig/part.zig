// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The Zig spelling of `crates/console_ffi/src/part.rs`, which is the
// definition. `build.rs` compares this file with that one on every build
// and refuses when they differ, printing the text this file must hold.

pub const Part = enum(u8) {
    Inline = 1,
    Quiet = 2,
    Banner = 10,
    You = 11,
    Head = 12,
    Reasoning = 13,
    Tool = 14,
    Reply = 15,
    Note = 16,
    Ended = 17,
    Resolved = 18,
    Waiting = 30,
    Working = 31,
    Calling = 32,
    Asking = 33,
    Composer = 34,
    Menu = 35,
};

pub const Outcome = enum(u8) {
    Answered = 0,
    Failed = 1,
};

pub const Ending = enum(u8) {
    Done = 0,
    Limit = 1,
    Cancelled = 2,
};

pub const Verdict = enum(u8) {
    Approved = 0,
    Denied = 1,
};

pub const Status = enum(u32) {
    Drawn = 0,
    Full = 1,
    Malformed = 2,
};
