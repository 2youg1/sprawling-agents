-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::ablation

规定 `citysim::ablation`（`tools/citysim/src/ablation.rs`）与它的语料 `citysim::ablation::capabilities`（`tools/citysim/src/ablation/capabilities.rs`），一件只在测试构型里编译的仪器（D21）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-8-4，别处引作 `tools/citysim/Spec.lean §8-8-4`。

本分部只有节注释：切段规则、三值的代价与排序由 `ablation::tests` 守着（§16）；被量的文档是 `crates/city/templates/City.md`，它的每一句由人写，模型不重述它。
-/

/-!
#### 8-8-4 ablation（`ablation.rs` 形状 1 判定，`ablation/capabilities.rs` 形状 6 数据；仅测试构型）

`crates/city/templates/City.md` 是每个居民读到的第一份文本，它每多一段就向每一次 prefix 收一次租。这把尺把文档按段切开，逐段拿掉，量一个居民因此做不了什么。入口是一条 `#[ignore]` 测试，只在有人点名时跑：

```
cargo nextest run -p citysim --run-ignored all -E 'test(city_md)' --no-capture
```

可测量的替身是**能力与凭据**：一条 `Capability` 是居民必须能做的一件事，它的 `cue` 是文中授予这件事的那句逐字短语。

```rust
pub(crate) struct Capability { pub(crate) name: &'static str, pub(crate) cue: &'static str }
pub(crate) struct Passage { pub(crate) index: u32, pub(crate) opening: String, pub(crate) removed: ByteLen }
pub(crate) enum Cost { Untouched, Restated { also_said: Vec<&'static str> }, Sole { lost: Vec<&'static str> } }
pub(crate) struct Charge { pub(crate) passage: Passage, pub(crate) cost: Cost }
pub(crate) struct Ablation { /* 私有：passages、corpus */ }
impl Ablation {
    pub(crate) fn new(document: &str, corpus: &'static [Capability]) -> Result<Ablation, AxError>;
    pub(crate) fn charges(&self) -> Vec<Charge>;
    pub(crate) fn costliest_first(&self) -> Vec<Charge>;
}
```

- **三值而非布尔**：`Restated` 让人看得见文档在哪里重复自己。
- **语料自己不能给自己打分**：`new` 在整份文档里找不到某条 cue 即拒（`E_INVALID_ARGS`，recovery 指向语料）。
- **切段规则**：空行切块，以 `- ` 开头的块并入上一段。
- **`costliest_first` 先比失去的能力数（降），平手比被删字节数（升），末位比 `index`**：排序两次必须一样。
-/
