-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::bundle

规定 `bundle`、`bundle::history`、`bundle::landing`、`bundle::manifest`、`bundle::export`、`bundle::files`、`bundle::fixture`（`crates/storage/src/` 下同名的文件）。导出与恢复；清单即完整性检验；落盘纪律只此一处。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-12 storage::bundle（形状 4 适配器＋形状 2 值类型）

```rust
pub struct Manifest { /* 私有；records、head、cas_objects、files、history（包数与引用数） */ }
impl Manifest {
    pub fn records(&self) -> u64;         pub fn head(&self) -> &str;   // 链头哈希
    pub fn cas_objects(&self) -> u64;     pub fn files(&self) -> u64;
}
pub struct Bundle;
impl Bundle {
    pub fn export(city_root: &Path, dest: &Path) -> Result<Manifest, StorageError>;
    pub fn restore(bundle: &Path, city_root: &Path) -> Result<Manifest, StorageError>;
    pub fn read_manifest(bundle: &Path) -> Result<Manifest, StorageError>;
}
pub const MANIFEST: &str = "MANIFEST.json";
pub fn open_restored(city_root: &Path, now: TimeMs) -> Result<PathBuf, StorageError>;  // 恢复后可继续写
// StorageError 增一臂：Bundle { op, detail }——I/O 正常但不是一座城（目的地已占、清单对不上、链有缺口）
// Vfs 内缝增 `list_dirs`：两个适配器同改；list 与 list_dirs 都是浅层，遍树用显式工作表（不递归，栈溢出接不住）
```

- **带走什么**：`ledger/`（唯一历史，必带）、`cas/`（Locator 指进去，不带就断链）、城里的产品文件（`City.md`、各楼的 `RULES.toml`／`Roadmap.md`／`URBANITE.md` 与房间内容）。**不带**：索引与任何派生视图（可弃，恢复后由 Ledger 重建，带了就是第二份历史）；凭证（**它从不在城里**，在宙主机金库——导出一份能拷走凭证的备份会把隐私保证一次性作废）。
- **为何是目录而非单文件**：单文件要么自造容器格式（多一个要养的格式），要么引 tar／zip 依赖。目录两者都不要，且任何备份工具都能再打包一层——压缩不是本模块的职责。
- **清单是完整性的依据**：`MANIFEST.json` 记下记录数、链头哈希、CAS 对象数与文件数；`restore` 恢复后重算并比对。不对即拒，而不是“恢复了但少了几条”——后者是历史失真。
- **四个数由 `Manifest::of(vfs, ledger_dir, cas_dir, files_root)` 一处算出**：导出量目的地、恢复量城本身、比较的两侧因此是同一种测量。
- **清单也数随行的历史：`history_packs` 与 `history_refs`**，由 `history::Carried::of(vfs, bundle)` 一处量出 bundle 的 `history/` 里有几个包、几条引用。导出端量目的地写进清单；恢复端在复制任何东西之前量 bundle 本身再比，不等即整次拒绝（`StorageError::Bundle { op: "restore" }`）——删掉包的 bundle 若照常恢复，文件与账本已落、引用却指向不存在的对象，全成或全不成就破了。清单没有这两个键时读作 0：v0.0.6 的导出不写 `history/`，于是一份被剥掉这两个键、却仍带 `history/` 的 bundle 照样被拒。被否：恢复后再从新仓库反查（那时城文件已经落下，拒绝只能留下半座城）。
  `walk` 返回 `Result`：不存在的目录算空（尚无 CAS 的城），读不动的目录停下并带路径上报——一个子目录静默贡献零个文件，正是一份短了的备份与它自己的清单相符的来路。
  `restore` 比四个字段而不是两个：丢了 CAS 对象的 bundle 链校全绿、Locator 全部指空，只有对象数说得出这件事。
- **清单的四个数全部读自导出结果，因而不能自证**：一次把半座城丢掉的拷贝与它自己的清单完全相符。故 `export` **另取源侧的五个数**——账本文件数、CAS 对象数、城内文件数、账本记录数、链头——与目的地逐项比，任一项不等即拒。源侧的数从 `copy_tree`／`copy_city_files` 的返回值来。
- **链验在 restore 内**：恢复完即走一遍 Ledger 开启与链校（jsonl 已有的那一道）。交给调用方去验等于把一个必须成立的性质变成约定。
- **文件顺序确定**：遍历走 `Vfs::list`（已排序），清单用 BTreeMap；同一座城导两次，`MANIFEST.json` 逐字节相同。
- **什么算城里的文件：`travels` 一个家（形状 1 判定）。** 导出端与恢复端问同一个谓词：根层 `.sprawling`（其内容经 ledger／cas 两张门各走各的）与任何深度的 `.git`（受保护元数据，写下即提权）不算城文件；楼自己的 `.sprawling`（`RULES.toml` 等治理字节）**随行**，它是城的一部分。名字引自 kernel 的名单（12.3 定规），不重拼字符串。两侧的策略不同且必须不同：导出端**跳过**（选择带走什么，源城里的簿记本来就不走），恢复端**拒绝**（全成或全拒——一份夹带 `.git` 的 bundle 是伪造品，恢复它就是在落 hooks）。
- **git 历史随行（`bundle::history`，形状 4 适配器；git2）。** 城根若是 git 仓库，`export` 用 `git2::PackBuilder` 把每条引用所达的全部对象（沿 revwalk 的提交与树，外加每个引用目标的递归闭包，覆盖附注标签）打成 `history/history.pack`，把引用写成 `history/refs`（每行 `<oid> <name>` 或 `ref:<target> <name>`，`HEAD` 在内）；城根不是仓库时 bundle 没有 `history/`，恢复端也不建仓库。`restore` 在复制任何东西之前拒绝已有 `.git` 的城根（`StorageError::Bundle { op: "restore" }`），复制后 `git2::Repository::init` 一个新仓库，经 `Odb::packwriter` 写入包（libgit2 边写边建索引并校验每个对象），再逐条立引用、按 `HEAD` 读出树写入索引，使 `git status` 只报导出时未提交的改动。**`hooks/` 与 `config` 永不复制**：bundle 里只有包与引用两样，仓库的配置与钩子来自新 init，于是一份伪造的 bundle 没有落钩子的路。引用名只收 `HEAD` 与 `refs/` 之下、且过 `git2::Reference::is_valid_name` 的名字，其余整次拒绝。被否：直接复制 `.git` 目录（会带上 hooks 与 config，正是 8-12 要拒的东西）；`git bundle` 子进程（引外部二进制，且 bundle 格式的解析不在本进程的校验之内）。
- **v0.0.6 的 bundle 导入其仓库而不是整体拒绝。** v0.0.6 的导出把城根的 `.git` 整个复制进 `city/.git`，清单的 `files` 也数了它的文件。恢复端见到 `city/.git` 是目录、而 bundle 没有 `history/` 时，把它当作一份旧式历史：以 `git2::Repository::open_bare` 只读打开，按导出端同一段打包逻辑（同一个引用名准入）得到包与引用，再走同一个 `History::land`；`city/.git` 下的文件既不拒也不复制，清单的 `files` 减去它们的个数再比。`hooks/` 与 `config` 依旧只来自新 init。仍整次拒绝：`history/` 与 `city/.git` 同在（v0.0.7 的导出从不写后者，二者同在即伪造）、`city/.git` 不是能打开的仓库、`city/.git/objects/info/alternates` 或 `city/.git/commondir` 存在（`open_bare` 会顺着前者把 bundle 之外任一仓库的对象读进包，顺着后者把 `objects`、`refs` 与 `packed-refs` 整个换成它所写路径下的那一份，伪造的 bundle 借此把 bundle 之外的对象与引用带进新城；v0.0.6 的导出从不写二者，因为城根的仓库既不借用别处的对象库，也不是别处仓库的 worktree），以及任何更深处的 `.git`（那是别的仓库，导入它不在本契约内）。被否：继续整体拒绝（v0.0.6 用户的备份因此恢复不了，而其中的对象与引用正是本节已有的导入路径能安全接住的）。
- **别名永不落盘（8-25）。** `walk` 见到任一链接拒绝整次操作；bundle 的每一次落盘只走 `bundle::landing::land`（形状 4 适配器）：它按值收下 `alias::WriteTarget`，让清过的目标被消费而不是查完即丢。bundle 的目标由 `WriteTarget::within(op, root, path)` 清出：`root` 是人给的根（导出时是 bundle 目录，恢复时是城根），只查 `path` 与它和 `root` 之间的每一级目录，不查 `root` 本身及其上——城根放在哪里是人选的，`/home` 指向 `var/home`、macOS 的 `$TMPDIR` 经 `/var` 这样的链接不是 run 能造出来的写路径。不在 `root` 之下的路径退回到查到文件系统根，所以界只会放宽人选的那一段。被否：沿用 `WriteTarget::at` 查到文件系统根（城只要放在链接下面，导出与恢复就整次失败）。runtime 的 edit 工具同一条理由：它以城根为界调 `within`，因为 macOS 的临时目录都在 `/var` 这个链接之下，查到文件系统根的写门在那里拒绝每一次编辑（`crates/runtime/src/tools/edit/tests.rs` 里放在链接下面的城那一条）。字节先写同目录的暂存文件 `.<name>.part`（这个拼写只在 `landing` 定义一处；导出不把合这个拼写的名字当城文件带走，因为恢复中途崩溃会把暂存文件留在城根下）、`sync_data`，再把原权限抄到暂存文件上，最后 `rename` 覆盖并 `sync_dir`。原权限按 `landing::Bits` 取：复制城文件时取源文件的（可执行位、只读位随文件走，导出再恢复后不丢），写清单与历史时取被覆盖文件的（没有就用新建默认值）。`rename` 换的是目录项，所以判定之后才出现的硬链接也写不穿（硬链接本身在 `WriteTarget` 被拒，8-25），且不存在「名已删、字节未落」的丢文件窗口。权限在 `sync_data` 之后才抄：只读位一旦落上，Windows 不再允许以写句柄打开该文件。被否：写前移除该名再建（失败即丢文件，且按 umask 新建，丢可执行位）；就地截断重写（写穿硬链接，崩溃留半个文件）。链接臂是拒绝不是跳过——对照组的 “skipped N files” 就是宽容部分还原。悬空链接在列举中不可见（`is_file`／`is_dir` 走不到它），但落盘必须先过 `WriteTarget`，故恒拒仍然成立。
-/

/-!
### 8-21 `bundle` 的城夹具住一处（`storage::bundle::fixture`）

`city_with(records, root)`——立一座有 N 条记录的城、写两个文件、开一次 CAS——在 `bundle/export.rs`、`bundle/files.rs`、`bundle/manifest.rs` 的测试模块里**逐字节重复三遍**。三份拷贝就是三个「一座城长什么样」的权威：改其中一份，另外两份的断言仍在对着旧形状作证。

**一个夹具一处**：`crates/storage/src/bundle/fixture.rs`，`#[cfg(test)]` 编译，由 `bundle.rs` 以 `#[cfg(test)] mod fixture;` 挂上，三个测试模块 `use super::super::fixture::city_with;`。形状 4 适配器（它造的是被测代码之外的一个真实环境）。**不放进 `bundle.rs` 自身**：索引文件不持逻辑，而夹具是逻辑。
-/
