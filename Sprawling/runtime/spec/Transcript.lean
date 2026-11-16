-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::transcript

规定 `transcript`（`crates/runtime/src/` 下同名的文件）。一个 run 实际看到了什么，写在它房间旁边。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::transcript` 旁的测试守住。
-/

/-!
### 8-32 runtime::transcript（形状 2 值类型）


> 旧对话得到一个地址，于是 §8-30 的 `search` 从「方便」变成「必要前提」——一个不能检索的地址比没有地址更糟。

**它是什么**：一跑冻结时，把**模型实际看到的消息**——工具调用与其结果、sieve 产出的压缩形、指回被搁置部分的 rest 指针——写成 `<room>/<run-id>.jsonl`，一行一条 `ChatMessage`（`kernel::model::wire` 的 serde 形，原样，所以 `search` 找到的行号就是消息序号）。frozen prefix 不在其中：它是每次请求都相同的那一半，账本的 `prompt_assembled` 已经持有它的哈希。

**它不是账本**：账本住 `.sprawling/`，`read` 对那里的每一条路径都拒绝；而且账本是**全城一条链**——把它交给一个 resident 就是把一栋 confidential 楼的事件也交出去。transcript 不加密，谁读得到它由读界答：它住在 run 的房间里，`read` 与 `search` 对它的路径和对房间里任何文件一样先过 `chosen_path::admit`，所以本楼的 resident 读得到，非机密楼的 transcript 他楼也读得到，而机密楼的 transcript 楼外读不到（`crates/city/Spec.lean` §8-2）。

**三步定序，不可颠倒**：①凭据扫描（`redact::redact` 逐消息走一遍，与 `model_returned` 入账本同一把扫描器）；②钉入 CAS（`storage::Cas::put`，内容寻址，同一份 transcript 写两次是一次）；③实体化（写到房间，只读位）。先扫后钉：一个钉进 CAS 的密钥永远删不掉。

```rust
pub struct Transcript { run: RunId, lines: Vec<String>, redacted: u32 }   // 私有字段，一处构造
impl Transcript {
    pub fn of(run: RunId, conversation: &Conversation) -> Result<Transcript, AxError>;  // 逐消息序列化、扫描
    pub fn address(room: &Address, run: RunId) -> Result<Address, AxError>; // `<room>/<run-id>.jsonl`，纯函数：路径在跑之前就已知
    pub fn materialise(&self, cas: &mut Cas, city_root: &Path, room: &Address) -> Result<TranscriptRecord, AxError>;
    pub fn lines(&self) -> &[String];  pub fn redacted(&self) -> u32;
}
pub struct TranscriptRecord { pub address: Address, pub original: Locator, pub redacted: u32 }
impl Run<Frozen> { pub fn transcript(&self) -> Result<Transcript, AxError>; pub fn plan(&self) -> &RunPlan; }
```

`Frozen` 状态因此保留 `Window`：冻结后唯一还需要它的读者就是这一处。地址是纯函数，所以 `freeze_plan` 在 drive 之前就能把它写进 Handoff 的 `context` 段——`handoff_written` 载荷携一行 `transcript at <room>/<run-id>.jsonl`，一条测试钉住这一行的存在。实体化发生在 drive 归来之后的 `conclude`（装配层持 CAS），失败记 diagnostics 而不让一次已冻结的跑变成 `Err`：冻结已在账本上，transcript 是它的副本。

**接线**：`RunPlan::predecessor` 为 `Some` 时，run segment 增一行 `Predecessor transcript: <address>`——继任者从 prefix 就知道去哪里 `search`。
-/
