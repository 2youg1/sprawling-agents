-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::resident

规定 `resident`（`crates/storage/src/resident.rs`）。进程里只放工作集：一份按字节计预算、能从盘上读回的缓存。本文件是 `crates/storage/Spec.lean` 的一个分部；下面一节保留它在 storage 规格里的标签 §8-40，别处引作 `crates/storage/Spec.lean §8-40`。

这一分部只写接口的形状。缓存必须守住的性质（常驻字节从不超过预算、冻结的 run 不留任何东西、每次读答出盘上的字节）只住在一个模型里：`crates/sprawling/spec/Serving/Memory.lean` §8-173，那里证明它们，并给出按项数计的被否设计的反例（sprawling D42）。本模块旁的 `proptest`（`crates/storage/src/resident/tests.rs`）从那三条性质导出。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `storage::resident::tests` 守住。
-/

/-!
### 8-40 storage::resident（形状 7）

```rust
/// 读回缓存的总预算：视图与旁索引的 12 MiB 加 12 MiB 的读回（Memory.lean 的预算表）。
pub const RESIDENT_TOTAL_BYTES: usize = 24 * 1024 * 1024;

pub struct Resident<R, K> { /* 按放入次序排的项，每项一个 run、一个地址、Arc<[u8]> 字节；冻结的 run 集 */ }
impl<R: Ord + Clone, K: Ord + Clone> Resident<R, K> {
    pub fn new(budget_bytes: usize) -> Self;
    /// 读盘（`load`）并放进缓存，再从最旧的一项丢起，直到常驻字节不超过预算。
    /// 冻结的 run 不放；一项自己超过预算时不留。
    pub fn insert<E>(&mut self, owner: &R, key: &K, load: impl FnOnce(&K) -> Result<Vec<u8>, E>) -> Result<(), E>;
    /// 命中答缓存里的字节；不命中读盘、照 `insert` 放进去，答读到的字节。
    pub fn read<E>(&mut self, owner: &R, key: &K, load: impl FnOnce(&K) -> Result<Vec<u8>, E>) -> Result<Arc<[u8]>, E>;
    /// 丢掉最旧的一项（操作系统要内存时，或节拍逐出）。
    pub fn evict_oldest(&mut self);
    /// 丢掉这个 run 的每一项，此后不再为它放任何东西。
    pub fn freeze(&mut self, owner: &R);
    pub fn resident_bytes(&self) -> usize;
}
```

失败只有一种来源：`load` 读盘失败。缓存原样交出 `load` 的错误，状态不变（没读到的字节不会被放进去），所以错误类型是调用者的，本模块不定义错误码。

`read` 命中时不改次序：模型的修剪是先进先出（Memory.lean 的 `trim` 从最旧的一项丢起，`read` 命中不动缓存），Rust 照它；重新放入同一个地址才把它挪到最新。

三个平台相同：缓存是纯 Rust 的内存结构，不调平台。盘上的字节由调用者的 `load` 用定位读取得（Memory.lean「定位读」一节）。
-/
