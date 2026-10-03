-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::status

规定 `status`（`crates/storage/src/` 下同名的文件）。还没被检查点收走的改动：分支、它与上游的差距、与某次提交不同的文件。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-22 `storage::status`：还没被检查点收走的那些改动，以及仓库此刻站在哪

`between` 比的是调用方已经握着的两个点，答不出仓库自己站在哪：哪个分支被检出、它有没有上游、它跑出上游多远——这三件是人在问「哪些文件动了」之前先问的。

```rust
pub struct Drift { pub ahead: u64, pub behind: u64 }
pub struct WorkingStatus { pub branch: Option<String>, pub drift: Option<Drift>,
                           pub files: Vec<FileChange> }
pub fn working_status(city_root: &Path, scope: Option<&str>, base: Option<GitOid>)
    -> Result<WorkingStatus, StorageError>;
```

**四条口径：**

1. **`base` 由调用方点名，因为检查点不动 HEAD。** `checkpoint::wave_pre` 把提交挂在 `refs/sprawling/` 之下，HEAD 停在基提交或上一次落地处；照 HEAD 比会把这座城跑过的每一次 wave 都报成「未提交」。`None` 退回 HEAD，那是一座还没立过检查点的城所拥有的全部。
2. **未跟踪文件照样成行。** 一个 agent 写下又从未入暂存的文件，恰恰是人要找的那个;只列已跟踪改动的清单会把一个新模块报成什么都没发生。为此 `changes::collect` 的 `Untracked` 归入 `How::Added`——工作树有而没有任何提交有的文件，就是这次加出来的。
3. **`scope` 是一条 pathspec 而不是事后过滤。** 楼页问的是它自己那些文件，让 git 在走差异时就收窄，比走完全城再筛一遍少一趟盘。
4. **`drift` 整个可缺席。** 没有上游、上游被删、以及处在游离头上，对读者而言是同一件可做的事（没有可比的对象），而与「和上游齐平」不是一回事。
-/
