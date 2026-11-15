// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The columns every line of the main screen stands on. A line that
// happened at a moment writes its time of day in the gutter; the mark
// that says what kind of line it is stands on the mark column; the words
// start on the text column, and so does every line they wrap onto, so
// the transcript reads as one left edge with times beside it. Below the
// width that leaves room for the gutter the times go and the marks move
// to the edge.

const paint = @import("paint.zig");

/// Below this many columns the time gutter is not drawn.
const gutter_from = 60;

/// The widest a paragraph of reply runs, about the WebUI's reading
/// column, however wide the window.
const reading = 88;

pub const Grid = struct {
    /// Whether times are drawn in the gutter.
    timed: bool,
    mark: usize,
    words: usize,
    /// The last column a line may reach.
    limit: usize,

    pub fn of(p: *const paint.Painter) Grid {
        const timed = p.limit + 1 >= gutter_from;
        const mark: usize = if (timed) 10 else 0;
        return .{ .timed = timed, .mark = mark, .words = mark + 2, .limit = p.limit };
    }

    /// Columns a paragraph of words may take.
    pub fn measure(self: Grid) usize {
        return @min(self.limit -| self.words, reading);
    }

    /// Where the figures of a tool line end: before the room the word
    /// `failed` needs, and no further right than a reading column.
    pub fn figures(self: Grid) usize {
        return @min(self.limit, self.words + reading) -| 8;
    }
};
