-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::digest

规定 `digest`（`crates/runtime/src/` 下同名的文件）。一份长文档从外面看是什么样，只摘一次。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-16 runtime::digest（形状 1 判定＋形状 2 值类型）


```rust
pub struct StructureNode { pub level: u8, pub title: String, pub offset: ByteLen, pub span: ByteLen }
pub struct Digest { /* source、origin、structure、prose —— 私有 */ }
impl Digest {
    pub fn structural(source: B3Hash, origin: Option<Locator>, text: &str) -> Digest;
    pub fn with_prose(self, prose: String) -> Digest;      // 消费并返回：带 prose 的是另一个值
    pub fn is_suspect(&self) -> bool;  pub fn window_header(&self) -> String;
}
pub fn structure_of(text: &str) -> Vec<StructureNode>;      // 纯、全函数

pub struct Breaker { /* limit、consecutive */ }
pub enum BreakerVerdict { Attempt, Open { after: u32 } }
pub enum DigestOutcome { Cached(Digest), Fresh(Digest), Structural { digest: Digest, reason: AxError } }
pub fn digest_once(text, origin, breaker, cached: &mut dyn FnMut(&B3Hash) -> Result<Option<Digest>, AxError>,
                   write_prose: &mut dyn FnMut(&str) -> Result<String, AxError>) -> Result<DigestOutcome, AxError>;
```

- **结构是读出来的，prose 是写出来的**：前者机械可复现，与原文恒不冲突；后者恒带 `suspect`，**没有清除该标记的方法**——摘要不会因为被读两遍就不再是摘要。
- **`window_header` 把可疑说在读者看得见的地方**，并给出原文位置：与原文冲突时以原文为准，这条要能被执行而不只是被相信。
- **一个内容哈希一生只摘一次**：顺序即全部策略——先哈希、再问缓存、再读结构、最后才花一次模型调用；熔断打开时连那一次也省掉。
- **熔断按次数不按时间**：判定路径里放墙钟会毁掉重放，而「连续三次失败」是调用方可复现的事实。一次成功即复位：间歇性的 provider 与坏掉的 provider 是两种情况。
- **失败不升级为错误**：`Structural{digest, reason}` 仍带完整结构树——摘要失败的文档仍然是一份有标题的文档。
- **模型调用归调用方**：本模块决定问什么、信什么，`bin::assembly` 持有 provider（同 `runtime::run` 的钩子形状）。
-/

/-!
### 8-24 runtime::digest 目录化


| 文件 | 管什么 |
|---|---|
| `digest.rs` | 摘要管线本身：`StructureNode`／`Digest`／`Breaker`／`BreakerVerdict`／`DigestOutcome`，纯函数 `structure_of` 与 `close_deeper`，以及唯一入口 `digest_once`。`digest_once` 带 `argument_count` 豁免，故留在原路径 |
| `digest/tests.rs` | 摘要对读者的四个承诺：标题树跳过代码围栏、模型写下的散文永远 suspect、同一内容哈希一生只摘要一次、熔断器计次开合 |
-/
