-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::version

规定 `kernel::version`（`crates/kernel/src/version.rs`）：一个主体的版本，从 `FIRST` 起只向前走。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部是说明文档，不是形式规格：`Version` 是一个带溢出检查的计数，没有判定与状态机。
-/

/-!
### 8-5 kernel::version

```rust
pub struct Version(u64);                               // serde 透明为 u64
impl Version { pub const FIRST: Version;               // 1；首个可见版本
               pub fn next(self) -> Result<Version, AxError>; }   // 溢出时 E_INVALID_ARGS
```

信号记下它发往的房间的版本（`room_version`，`crates/kernel/src/event/record/collaboration.rs`）。被改的文件不靠版本守：edit 由这个 run 自己读到的戳准入（`crates/runtime/Spec.lean` §8-64），所以这里没有新鲜度判定。
-/
