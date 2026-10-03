-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::checkpoint::provenance

规定 `checkpoint::provenance`（`crates/storage/src/` 下同名的文件）。一次提交出自哪个 session：五条 git trailer，与它在账本一侧的形状。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-17 storage::checkpoint::provenance（形状 2 值）

```rust
/// 一次运行选定的模型，两者恒同行：模型 id 与它被要求的思考档位。
pub struct ModelChoice { pub id: String, pub effort: Option<kernel::Effort> }

/// 一次提交出自谁：唯一构造点在 `Provenance::new`，字段私有。
pub struct Provenance { /* run、actor、model、effort、city —— 私有 */ }
impl Provenance {
    pub fn new(run: RunId, actor: Address, city: B3Hash, chosen: ModelChoice) -> Provenance;
    /// 城的身份＝创世行的链哈希，从账本首段的第一行读出（只读一行）。
    pub fn city_of(ledger_dir: &Path) -> Result<B3Hash, StorageError>;
    /// git trailers 块，顺序与拼写恒为下列五行，有前任时加第六行，末尾带换行。
    pub fn succeeding(self, predecessor: RunId) -> Provenance;   // 写前任的唯一入口
    pub fn predecessor(&self) -> Option<RunId>;
    pub fn trailers(&self) -> String;
    pub fn actor(&self) -> &Address;
    pub fn run(&self) -> RunId;
}
```

```
Sprawling-Run: <run>
Sprawling-Actor: <actor>
Sprawling-Model: <model>
Sprawling-Effort: <effort or none>
Sprawling-City: <hex>
Sprawling-Predecessor: <run-id>      （只在有前任时）
```

前任经消费式的 `succeeding` 写入（`new` 已到四参数上限，而前任与 run 并不总是同行），`attribution()` 同时多出 `predecessor` 键（8-18）；缺席即 `None`——一条缺席的世系比一条编出来的好。没有前任的提交，trailers 只有上面五行。

- **每一个由城作出的提交都带上作出它的会话。** 提交署名为
  `<actor> <<actor>@<city 前 12 位 hex>.sprawling>`，
  正文为 `<subject>\n\n<trailers>\n`。一个人 `git log` 一眼看得出这一行出自哪个居民、
  哪次运行、哪个模型；`git interpret-trailers --parse` 读得出结构。
- **被否的另一条路：把这些事实塞进 subject。** subject 是给人读的一行，塞五个字段就没人读它了；
  trailers 是 git 自己就有的机制（`interpret-trailers`），复用它比发明一种前缀语法更省。
- **账本仍是权威。** trailers 是**给城外读者的投影**，不是第二个事实来源：谁做了什么由 Ledger
  回答，两边靠 oid 对上（`checkpoint_committed.oid` 与 `file_discarded.restoration`）。
  trailers 与账本不一致时以账本为准，trailers 是要修的那一侧。
- **effort 的字面来自 serde 的名字**（`kernel::Effort` 的 `snake_case`），所以「档位怎么拼」在这个仓库里只有一处权威；缺档位写 `none`。
  模型 id 未知时写空串——写一个假的 id 比写空更糟。
- **`city_of` 只读第一行**：整本账本可以有几十兆，而创世行是第一段文件的第一行；它与 open 的版本探测共用 `jsonl::first_line`，所以「第一行是什么」只有一处权威——以 `\n` 结尾的第一行，撕裂的首行不算。

### 8-18 trailers 的账本一侧

```rust
impl Provenance {
    /// 这次提交出自谁，按每一条指名提交的记录都携的形状给出。
    /// run 与 actor 已经是记录自己的身份（`EventRecord::run` 与 `addr`），
    /// 在载荷里重复它们等于给同一个事实立第二个权威。
    pub fn attribution(&self) -> kernel::event::record::CommitAttribution;
}

/// 没人点过的档位怎么记：记成 `Effort::None`。
/// `Option` 划出的那条线（由提供方决定 / 要它别想）从未上过 trailer 或账本，
/// 两边一向都写 `none`；这个坍缩全仓只在这里发生一次。
pub fn recorded_effort(effort: Option<kernel::Effort>) -> Effort;

/// 档位怎么拼的唯一权威（取自 `kernel::Effort` 的 serde 名）。
pub fn effort_word(effort: kernel::Effort) -> String;
```

- **键的权威在 kernel。** `model` / `effort` / `predecessor` 三个键由
  `kernel::event::record::CommitAttribution` 拼写，本 crate 不另持键名常量：
  两个 crate 各手写一次同一个键，就是两个会各说各话的权威。
- **`checkpoint_committed` 与 `pr_merged` 携同一份归属。** 8-17 写着「trailers 是投影，
  账本是权威」，而这两个事实否则只存在于 git 提交上；带上它们，`sprawling whose`
  才能只读账本作答（`accounting::views`，`crates/sprawling/Spec.lean` §8-167）。两条记录 flatten 同一个结构，
  于是「一次提交出自谁」不会在两个 kind 上长成两种说法。
- **缺键的记录读得回**：不带这两项的 `checkpoint_committed` 没有这两个键，
  `CommitAttribution` 的 `#[serde(default)]` 对它答空 id 与 `None`——
  **投影说不知道，好过投影猜一个**。
- **被否的另一条路：让 `sprawling whose` 去读 git trailers。** 那是把投影当成权威，正是 8-17
  明确拒绝的方向；而且一座导出后在别处恢复、`.git` 并不在身边的城将答不出自己的历史。
-/
