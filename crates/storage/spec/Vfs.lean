-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::vfs

规定 `vfs`（`crates/storage/src/` 下同名的文件）。内缝 `trait Vfs`：本 crate 触碰文件系统的唯一一张脸，以及谁在缝外。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-15 storage::vfs（形状 3 端口）

```rust
pub(crate) trait Vfs {                      // 内缝：不出对外接口，不升真缝
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()>;
    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>>;     // 排序后返回：遍历确定性
    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>>; // 同为浅层：遍树用显式工作表
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn size(&self, path: &Path) -> io::Result<u64>;             // 段长；index 比长度而不抬字节
    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>>;  // 定位读；短答＝文件到头
    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()>;
    fn sync_data(&mut self, path: &Path) -> io::Result<()>;
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn sync_dir(&mut self, dir: &Path) -> io::Result<()>;       // Windows no-op（§3-3）
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()>;  // 落盘抄权限（8-12 的 landing::Bits）
    fn exists(&self, path: &Path) -> bool;                      // cas 去重与重开容忍需要
}
```

- **`read_at` 只报事实，不判越界**：短答表示文件在那里结束，「要的范围超出了对象」这句话归提出范围的模块（§8-3）。端口若自己拒绝，Locator 文法就有了第二个权威。
- **`FaultFs` 另记 `bytes_read`**：op 计数答「碰了几次盘」，字节计数答「抬回来多少」，而范围读与整读的唯一区别就是后者。`FaultFs::bytes_read()` 是范围读不整读这条断言的观测面。
- **端口的符合性套件就是断电点阵**（§8-2）：两个适配器对同一组语义负责，`fault_fs` 存在本身就是这条缝的存在证明（§10 的设计 A）。
- **仍然不升真缝**：`pub(crate)`，`JsonlLedger` 把它藏在 `Box<dyn Vfs>` 后面，公开签名里一次不出现（否则 E0445）。
- **谁在缝外，以及为什么**：不经 `Vfs` 的读写属下列几类，每一类的理由写在它下面。
  - `index::reader::OpenSegment`——按 seq 取单行时持住段句柄。`Vfs::read_at` 每次调用开一次文件，
    按 seq 逐行取时，每行重开一次文件的代价远大于顺着持住的句柄读。
    它只读不写，而缝要建模的是崩溃语义，对一次定位读无话可说。
  - `worktree`——树由 git2 建、由 git2 prune，落盘不经本 crate；`release` 只解 git 的锁，不删目录；开城的 `sweep_abandoned` 用 `std::fs` 删城自己造的树目录（§8-9）。
    缝拦不住 git2，声称拦得住才是第二个权威。释放顺序与自愈见 §8-9。
  - `checkpoint`、`changes`、`hunks`、`status`、`blob`——同样经 git2 提交、检出与读对象。
  - `chain_audit`——逐段流式读整条链，只读不写（§8-30）。
  - `jsonl::ledger::WriterLock`——文件锁是操作系统对打开句柄的事实，Vfs 的崩溃语义模型对它无话可说（§8-1）。
  - `alias`——重解析点与链接计数是 `std::fs` 自己的事实（`file_type().is_symlink()` 对 symlink 与 junction 同真，Unix `MetadataExt::nlink` 判硬链接），Vfs 的崩溃语义模型对它们无话可说；它只问元数据、不读不写字节。
  - **`index` 不在例外之列**：`LedgerIndex` 持 `Box<dyn Vfs>`，段列举、段长、整段读（建表）与尾部增量读（刷新）全部经缝，
    而它没有一步是写；按 seq 取行走 `LineReader`，那正是上一条的例外。
-/
