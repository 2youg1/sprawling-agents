-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::fault_fs

规定 `fault_fs`、`fault_fs::plan`、`fault_fs::fs`（`crates/storage/src/` 下同名的文件）。Vfs 的第二适配器：撕裂写、乱序持久化、rename 中断，断电点阵的驱动器。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-2 storage::fault_fs

```rust
#[cfg(any(test, feature = "fault"))]        // 测试与 citysim 的故障面两个消费者
#[derive(Clone)]                            // 句柄共享状态（Rc<RefCell>）：断电后以同一实例重开
pub struct FaultFs { /* files: BTreeMap<路径, FileState{durable, live, durable_entry}>、op 计数、FaultPlan */ }
pub struct FaultPlan { pub cut_at_op: Option<u64>, pub cut_on_write: Option<&'static str>, pub torn_tail: TornTail }
                                            // cut_on_write：首个字节含该串的 append 断电，一次即消费
pub enum TornTail { None, KeepBytes(u64) }  // 撕裂写：每文件未同步增量保留前 k 字节；全显式即全确定，不需种子

impl FaultFs {
    pub fn new(plan: FaultPlan) -> Self;
    /// Simulates power loss now: live falls back to durable (+ torn
    /// prefix); files whose dir entry was never synced vanish.
    pub fn power_cut(&self);
    pub fn op_count(&self) -> u64;
}
impl Vfs for FaultFs { /* 每 op 自增计数；append 先落 live 再判 cut（撕裂可咬本次写），其余 op 先判；命中即 power_cut 并报 io::Error，plan 消费后后续 op 照常（重开阶段） */ }
```

**模型三则**（比真实平台严格，故纪律跨平台成立）：①`sync_data` 前的字节不存活：durable/live 两平面，断电即 live 回落 durable，撕裂按 `TornTail` 多留未同步增量前缀；②新建文件在 `sync_dir` 前目录项不存活，断电即消失（含已 sync_data 者——比 POSIX 更严，使建段后必 sync_dir 的纪律跨平台成立）；③rename 自身原子——恒不出现半个目标文件。
**第二个旋钮 `cut_on_write`，与两个入口**：

```rust
impl JsonlLedger {
    #[cfg(any(test, feature = "fault"))]
    /// 收具体 FaultFs，故 Vfs 缝不出门（缝表不动，depmap 不动）
    pub fn open_faulty(fs: FaultFs, dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), StorageError>;
}
```

`open_faulty` 返回的就是生产用的同一个 `JsonlLedger`，于是 crate 之外的调用方跑的是真代码，只丢掉它点名的那一次写。

**为什么按内容而不只按序号**：`cut_at_op` 在本 crate 内部好用，因为操作序就在眼前。**在装配层它是一个注定碎掉的数字**：要正好落在 `roadmap_claimed` 那一次 append 上得数一个魔术数，而上游任何一处多读一个文件就全盘失效。`cut_on_write` 让调用方用自己的词汇点名那一行——**它仍然完全显式、完全确定**（本模块自述「Everything is explicit… there is no randomness」，这条不破它），并且直接表达要问的那件事：假如这一行没落下。取 `&'static str` 是为了让 `FaultPlan` 保持 `Copy`；点名一条账本行用的是字面量。

**断电点阵**：以 `cut_at_op` 扫描 1..=N 全部注入点各跑一遍「写入→断电→重开→断言」；断言两条：链恒可验，**已返回 Ok 的波恒存活**（append_all 耐久契约的机器面）。断电于 EventRecord 落账与断电于 CAS rename 在此点阵上断言；checkpoint 经 git2 落盘、不经 `Vfs`（8-15），不在点阵上。
-/
