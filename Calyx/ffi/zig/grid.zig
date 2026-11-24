// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const paint = @import("paint.zig");

const gutter_from = 60;

const reading = 88;

pub const Grid = struct {
    timed: bool,
    mark: usize,
    words: usize,
    limit: usize,
    wide: bool,

    pub fn of(p: *const paint.Painter) Grid {
        const timed = p.limit + 1 >= gutter_from;
        const mark: usize = if (timed) 10 else 0;
        return .{ .timed = timed, .mark = mark, .words = mark + 2, .limit = p.limit, .wide = p.mei };
    }

    pub fn measure(self: Grid) usize {
        const room = self.limit -| self.words;
        return if (self.wide) room else @min(room, reading);
    }

    pub fn figures(self: Grid) usize {
        return if (self.wide) self.limit else @min(self.limit, self.words + reading) -| 8;
    }
};
