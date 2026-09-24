const std = @import("std");

// The Zig performance kernel: byte-level leaves for `crates/mem`, built as
// one static library (`libmem.a` / `mem.lib`) and linked by that crate's
// build script. No domain model lives here (mem-SPEC.md section 1).
pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const lib = b.addLibrary(.{
        .name = "mem",
        .linkage = .static,
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/lib.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    b.installArtifact(lib);

    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/lib.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "run the kernel's tests (also: --fuzz for the Smith target)");
    test_step.dependOn(&run_tests.step);
}
