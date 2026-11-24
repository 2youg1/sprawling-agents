// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub fn build(b: *std.Build) void {
    const boundary = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("zig/boundary.zig"),
            .target = b.standardTargetOptions(.{}),
            .optimize = .safe,
        }),
    });
    b.step("fuzz", "Exercise the desktop buffer rules").dependOn(&b.addRunArtifact(boundary).step);
}
