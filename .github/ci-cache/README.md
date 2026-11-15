# CI cache adapter

This directory is the thin I/O adapter between `cargo xtask ci` and GitHub's Actions cache. Rust owns action identity, validation, execution and reuse policy. The adapter only restores or saves the `verdict.json` whose complete content key Rust computed.

`@actions/cache` is pinned exactly in `package.json` and `bun.lock`. Restore uses an exact key with no prefix fallback. A cache failure returns control to Rust, which executes the action; a failed save never changes an already computed verdict.

The local Node24 action starts `just verify-sources` in the original workspace and inherits the runner's cache runtime environment. Only installation of this optional SDK may fail without failing verification. The adapter and SDK stay in this stable directory; source checks use private ordinary-file copies without installed dependencies.

Six audited pure source gates can reuse a same-key green result on Linux, where `/proc/self/exe` identifies the running verifier. Other gates, external commands and platforms execute each time. Each non-reusable action gets its own copy, so a tool cannot change the inputs of a reusable gate. A warm-run claim requires an exact `hit <gate>` verdict on a fresh runner; dependency-cache hits are separate.
