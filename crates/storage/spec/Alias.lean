-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::alias

规定 `alias`（`crates/storage/src/` 下同名的文件）。别名族（junction／symlink／硬链接）与每扇写门收的已清空写目标，以及它的两种落盘。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-25 `storage::alias`：别名族与被清空的写目标（形状 2 值）

```rust
pub enum AliasKind { Link, HardLink }   // Link＝symlink 与 junction（Win32 重解析点族）
pub struct WriteTarget(PathBuf);        // 字段私有；构造点是 at 与 within
impl WriteTarget {
    pub fn at(op: &'static str, path: &Path) -> Result<WriteTarget, StorageError>;   // 查目标与每一级父目录
    pub fn within(op: &'static str, root: &Path, path: &Path) -> Result<WriteTarget, StorageError>;  // 只查到 root 为止；bundle 与 runtime 的 edit 工具用它
    pub fn as_path(&self) -> &Path;
}
pub(crate) fn kind_at(path: &Path) -> Result<Option<AliasKind>, StorageError>;   // 叶级分类，walk 用
```

- **别名族全不穿透（junction／symlink／硬链接）。** junction 与 symlink 都是重解析点、`file_type().is_symlink()` 对两者同真，故合为 `Link`；硬链接是链接计数大于 1 的普通文件（Unix 读 `nlink`，Windows 经句柄读 `number_of_links`，§3.5）。三者在每一扇门**字面拒绝**。判定与落盘之间的替换窗口由落盘纪律罩住：`bundle::landing::land` 把同目录的暂存文件 `rename` 覆盖该名，换的是目录项，其它名字保有旧字节。「写前移除该名再建」同样换目录项，但移除之后写入失败即丢文件、新建即丢权限位，8-12 否决它，runtime 的 edit 也已改走同一个落盘函数（8-32）。**被否：跳过并报数**（对照组 “skipped N files”）——部分落盘破坏全成/全拒，且一行计数无法让重放方复现跳过了哪几个。
- **`WriteTarget` 是形状 2 值：不变量在唯一构造点，字段私有。** 「未经检查的写目标拼不出来」由 trybuild 编译失败反例钉住（`tests/ui/`）。它的证明范围是「检查那一刻这个名与它的父级都不是链接」；检查与落盘之间的替换窗口属效果面，故两个采样点（walk 与写入）都过同一判定。硬链接在两个平台上都在判定内被拒。
- **消费面是三个写域加一个工具写面**：checkpoint 暂存回调（8-8）、bundle 的 `landing::land`（8-12；`worktree::back::restore_file` 也经它落盘，8-27）、worktree 放置，加上 `runtime::tools::edit` 的物理写入（运行的写域）——经链接写保留路径在每一扇门恒拒。
- **proptest 族「别名永不落盘」**：对别名种类 × 目标（受保护／普通）× 落点（名上／父目录）的组合，凡该平台造得出的别名（junction 无需特权即可创建；symlink 需特权；硬链接随处可造）：链接臂与硬链接臂写入都被拒；各臂同一断言——目标字节不变、别名带不出新字节；该平台造不出的退化为断言「放置失败时盘上无任何变化」，各臂同性质。
- **Windows 上读计数的代价与三种情形。** 判定每遇到一个普通文件多开一次句柄。读数（Windows x86_64 笔记本级 CPU，NTFS，Defender 实时防护开，debug 构建，2,000 个文件取三轮）：只读元数据约 34 µs/文件，加开句柄读计数约 66 µs/文件。三种情形的答案：有第二个名字的文件答 `HardLink`，只读文件与被别的句柄以共享模式 0 独占打开的文件都答不是别名（`alias::tests` 的 `a_second_name_is_a_hard_link_and_nothing_else_is`）。
-/

/-!
### 8-32 `alias::WriteTarget` 的两种落盘：替换与新建（形状 4 适配器）

```rust
impl WriteTarget {
    pub fn replace(self, bytes: &[u8]) -> Result<(), StorageError>;   // 暂存文件再 rename，取被替换文件的权限
    pub fn create(self, bytes: &[u8]) -> Result<(), StorageError>;    // 名字不存在才建；已有即 NameTaken
}
pub enum StorageError { /* …既有臂… */ NameTaken { path: PathBuf } }   // → E_VERSION_CONFLICT
```

- **落盘纪律只有一处，runtime 借用它。** `replace` 就是 `bundle::landing::land`（`Bits::OfReplaced`）：字节先写同目录的暂存文件、`sync_data`、抄被替换文件的权限、`rename` 覆盖、`sync_dir`（8-12）。runtime 的 edit 改写一个已有文件时经它落盘，写前移除该名再建的做法随之删去：移除之后写入失败不再丢文件，可执行位也不再丢。
- **`create` 由文件系统原子地占名。** 以「仅当不存在才建」（`create_new`）打开目标：名字上已有任何东西——普通文件、悬空链接、别的调用刚建成的文件——都答 `NameTaken`，什么都不写；占到之后写入字节、`sync_data`，再 `sync_dir` 父目录。两次竞争的新建只有一次占得到名字，这是 `WriteLimit::Create` 的「竞争创建只成一次」在盘上的依据（kernel-SPEC §8-78）。占到名字之后写入失败，删掉自己刚建的文件再报原来的失败；删也失败时报删的失败，因为那时盘上留着半个文件，这才是调用方要处理的状态。
- **两者都消费清过的目标**：只有经 `WriteTarget::at`／`within` 清过别名的名字才落得了盘（8-25），所以经链接新建或改写一个旧文件在两条路上都不通。
- `NameTaken`→`E_VERSION_CONFLICT`：不可定义掉——名字有没有被占是落盘那一刻文件系统的事实。恢复：读那个文件、对着它的版本改，或换一个名字。
- 验收：`alias` 测试 `two_racing_creates_admit_one`（八个线程同时新建同一个名字，恰一个 `Ok`，其余 `NameTaken`，盘上是那一个的字节）；runtime 的 `an_existing_file_is_unchanged_under_create_by_edit_exec_and_link`。
-/

/-! D13 Windows 上硬链接按链接计数字面拒绝，计数经 `winapi-util` 的句柄读取

同一个硬链接在两个平台上得到同一个答案：`E_OUTSIDE_WRITE_DOMAIN`，恢复是换成普通文件后重试。被否：Windows 上维持不报拒、只靠写入落新 entry——同一条规则在 Unix 拒、在 Windows 静默拆开，拆开的那一臂还要靠每一扇写门都记得换目录项，edit 曾经的「写前移除该名再建」正是这样来的，今天它经 `WriteTarget::replace` 落盘（8-32）。代价是每个被判定的普通文件多开一次句柄，读数在 8-25。**重开参数**：Windows `std` 出现稳定的 `number_of_links`。
-/

/-! D15 runtime 的 edit 经本 crate 落盘，新建由文件系统原子地占名（8-32）

落盘纪律只有一处：替换就是 `bundle::landing::land`，新建是 `create_new` 加同样的刷盘。理由：edit 自己的「写前移除该名再建」有丢文件与丢权限两个窗口（8-12 已否），而「仅新建」要的「竞争创建只成一次」只有在写的那一刻由文件系统判才成立；先查 `exists` 再写，两次调用都会查到「不存在」。被否：①edit 留在 runtime 里自己写，只把移除换成暂存再 `rename`——同一条纪律的第二份抄本；②新建也走暂存再 `rename`——`rename` 会覆盖竞争者刚建成的文件，占名就不再是原子的。**重开参数**：出现要在新建时保留别处权限位的调用方（今天新建取新建默认值）。
-/
