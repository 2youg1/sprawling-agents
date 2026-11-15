# CI cache adapter

This directory is the thin I/O adapter between `cargo xtask ci` and GitHub's Actions cache. Rust owns action identity, validation, execution and reuse policy. The adapter only restores or saves the `verdict.json` whose complete content key Rust computed.

`@actions/cache` is pinned exactly in `package.json` and `bun.lock`. Restore uses an exact key with no prefix fallback. A cache failure returns control to Rust, which executes the action; a failed save never changes an already computed verdict.
