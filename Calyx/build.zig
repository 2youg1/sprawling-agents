// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.option(
        std.builtin.Optimize,
        "optimize",
        "Optimization mode for Calyx Zig libraries",
    ) orelse .safe;
    const pic = b.option(bool, "pic", "Emit position-independent code") orelse true;

    const leaf_logic = b.createModule(.{
        .root_source_file = b.path("ffi/zig/leaf.zig"),
        .target = target,
        .optimize = optimize,
        .pic = pic,
    });
    const leaf_c_mod = b.createModule(.{
        .root_source_file = b.path("ffi/zig/leaf_c.zig"),
        .target = target,
        .optimize = optimize,
        .pic = pic,
    });
    const leaf = b.addLibrary(.{
        .name = "sprawling_console_leaf",
        .linkage = .static,
        .root_module = leaf_c_mod,
    });
    b.getInstallStep().dependOn(&b.addInstallArtifact(leaf, .{}).step);

    const shell_imports = [_]std.Build.Module.Import{
        .{ .name = "leaf", .module = leaf_logic },
    };
    const shell_mod = b.createModule(.{
        .root_source_file = b.path("shell/root.zig"),
        .target = target,
        .optimize = optimize,
        .pic = pic,
        .imports = &shell_imports,
        .link_libc = true,
    });
    const shell = b.addLibrary(.{
        .name = "sprawling_calyx_shell",
        .linkage = .static,
        .root_module = shell_mod,
    });
    // Rust links this archive; without compiler_rt, large Zig frames miss __zig_probe_stack.
    shell.bundle_compiler_rt = true;
    b.getInstallStep().dependOn(&b.addInstallArtifact(shell, .{}).step);

    const calyx_mod = b.createModule(.{
        .root_source_file = b.path("shell/main.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &shell_imports,
        .link_libc = true,
    });
    const calyx = b.addExecutable(.{
        .name = "calyx",
        .root_module = calyx_mod,
    });
    b.installArtifact(calyx);

    const leaf_tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("ffi/zig/leaf.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    const shell_tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("shell/root.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &shell_imports,
            .link_libc = true,
        }),
    });
    const test_step = b.step("test", "Run Calyx Zig leaf and shell tests");
    test_step.dependOn(&b.addRunArtifact(leaf_tests).step);
    test_step.dependOn(&b.addRunArtifact(shell_tests).step);
}
