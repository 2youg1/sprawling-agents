-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::market

规定 `market`（`crates/gateway/src/market.rs`）：钉版的模型目录快照。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-7 gateway::market（形状 6 数据面＋快照）

```rust
pub use kernel::event::record::InputKinds;              // 这个模型收得下什么：Text（默认）| TextImage；定义在 kernel，因为 model_selected 行带着它
pub struct ModelEntry { pub id: String, pub context_tokens: u64, pub input: InputKinds,
                        pub max_output_tokens: Option<Ceiling>,   // §7、§8-17
                        pub input_price: UsdMicros /* per 1M tokens */, pub output_price: UsdMicros,
                        pub cache_read_price: UsdMicros, pub cache_write_price: UsdMicros }
pub struct MarketSnapshot { /* version: u32、entries: BTreeMap<String, ModelEntry> —— 私有 */ }
impl MarketSnapshot {
    pub fn builtin() -> Result<MarketSnapshot, AxError>;                 // 内置钉版目录（数据面）
    pub fn from_entries(version: u32, entries: Vec<ModelEntry>) -> Result<MarketSnapshot, AxError>;
    pub fn lookup(&self, id: &str) -> Option<&ModelEntry>;  pub fn version(&self) -> u32;
}
```

- **`input` 默认 `Text`，宽容读。** `selected_payload` 经 `Payload::of(&ModelSelected)` 写 `input` 键，`read_choice` 经 `Payload::read::<ModelSelected>` 读，读不到就当 `Text`——旧 Ledger 里的 `model_selected` 没有这个键，而重放一份旧历史不应该报错；默认取「只收文字」而非「收图」，因为猜错方向的代价不同：猜小了是一句拒绝，猜大了是 provider 的 400。内置目录里收图的行（现为 `claude-sonnet`）标 `TextImage`，`local` 保持 `Text`。目录只是「收得下什么」那架梯子的一档，登记时谁答由 §8-37 判。
- 钉版回滚＝持前一快照即回滚（值语义，无 I/O）；快照落盘属 projection／config 面，本模块只管形与查询。价目恒整数微美元（判定路径禁浮点）。
- **`builtin()` 上抛 `from_entries` 的拒绝，不顶一份空目录。** 两行同 id 是编程错误，而把它顶成空目录的后果是：城里每一个模型都查不到价目行，拒词一个也不点名那张表。`Result` 让这件事在第一个调用点就说出来（`E_INVALID_ARGS`，主题为撞车的 id）。落选的是「rows 改 const 数组加一条去重测试」：那把不变量交给一条可以被删掉的测试，而类型能一直拿着它。
-/
