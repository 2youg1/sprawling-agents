-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::guide

规定 `crates/accounting/src/guide.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`accounting::guide` 旁还没有测试。
-/

/-!
### 8-18-2 上手指南的进度（`accounting::guide`，形状 4 适配器）

```rust
// accounting::guide
pub fn read(city_root: &Path) -> Result<wire::GuideProgress, AxError>;
pub fn put(city_root: &Path, progress: &wire::GuideProgress) -> Result<(), AxError>;
```

- **一份记录，一种文法。** 文件 `CityLayout::guide`（`<城>/.sprawling/GUIDE.toml`）就是 `wire::GuideProgress` 的 TOML 序列化，与 `person` 的 `[ui]` 是 `PreferencesAnswer` 的序列化同一条理（§8-8）：本模块只读与写，什么是一步、什么是标记由线上的类型说。
- **读。** 文件不在即 `GuideProgress::default()`；读不出或解析不了答 `E_CONFIG_INVALID`（文件与原因）或 `E_STORAGE_FATAL`（读不了），视图把两者答成 `Unavailable`，不拿缺省值冒充。
- **写。** 经 `city::edit_document` 整份替换，读者在写的途中只会读到写之前或写之后的那一份；保留子树不存在时先建它。`Command::PutGuide` 由 `worker::commanding::routing` 直接交给 `put`，不写账本行（`crates/wire/Spec.lean` §8-68）。
- 验收：`worker::commanding::tests::guide` 的 `the_guide_keeps_its_progress_across_a_reopen`。
-/
