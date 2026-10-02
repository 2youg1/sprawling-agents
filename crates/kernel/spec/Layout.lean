-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address

/-!
# kernel::layout

规定 `kernel::layout`（`crates/kernel/src/layout.rs`）：城内磁盘布局的唯一权威。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-56 `kernel::layout`：城内磁盘布局的唯一权威（形状 2 值）

```rust
pub const LEDGER_DIR: &str = "ledger";
pub const CAS_DIR: &str = "cas";
pub const SNAPSHOT_DIR: &str = "snapshot";
pub const LIBRARY_DIR: &str = "library";
pub const BUILDING_SHELF: &str = "skills";
pub const SKILL_FILE: &str = "SKILL.md";
pub const CONFIG_FILE: &str = "CONFIG.toml";
pub const FILTERS_FILE: &str = "FILTERS.toml";
pub const ARCHIVE_DIR: &str = "Archive";
pub const JOB_FILE: &str = "JOB.md";
pub const HANDOFF_FILE: &str = "Handoff.md";
pub const URBANITE_FILE: &str = "URBANITE.md";
pub const GUIDE_FILE: &str = "GUIDE.toml";                          // 上手指南的进度，城的保留子树下（`crates/wire/Spec.lean` §8-68）
pub const TRANSCRIPT_EXT: &str = "jsonl";                            // 一次 run 的对话记录：房间里的 `<run>.jsonl`
pub const RUN_ID_PATTERN: &str = "????????-????-????-????-????????????";   // `RunId` 显示形的 git 忽略模式，一个字符一个 `?`
pub const REMOTE_DIR: &str = "remote";                               // 远程门的状态，城的保留子树下（§8-76）
pub const DEVICES_FILE: &str = "devices.toml";                       // 配对过的设备表
pub const PLAYBACK_DIR: &str = "playback";                           // playback 导出件，城的保留子树下

pub struct CityLayout { /* root —— 私有 */ }
impl CityLayout {
    pub fn new(root: &Path) -> Self;
    pub fn root(&self) -> &Path;
    pub fn scope(&self, addr: &Address) -> PathBuf;             // root + 逐段
    pub fn ledger(&self) -> PathBuf;                            // root/.sprawling/ledger
    pub fn snapshot(&self) -> PathBuf;                          // root/.sprawling/snapshot
    pub fn cas(&self) -> PathBuf;                               // root/.sprawling/cas
    pub fn library(&self) -> PathBuf;                           // root/.sprawling/library
    pub fn config(&self, addr: &Address) -> PathBuf;            // <scope>/.sprawling/CONFIG.toml
    pub fn city_config(&self) -> PathBuf;                       // <root>/.sprawling/CONFIG.toml
    pub fn city_filters(&self) -> PathBuf;                      // <root>/.sprawling/FILTERS.toml
    pub fn building_skills(&self, addr: &Address) -> PathBuf;   // <scope>/.sprawling/skills
    pub fn filters(&self, addr: &Address) -> PathBuf;           // <scope>/.sprawling/FILTERS.toml
    pub fn archive(&self, building: &Address) -> PathBuf;       // <scope>/Archive
    pub fn job(&self, addr: &Address) -> PathBuf;               // <scope>/JOB.md
    pub fn handoff(&self, room: &Address) -> PathBuf;           // <scope>/Handoff.md
    pub fn urbanite(&self, addr: &Address) -> PathBuf;          // <scope>/URBANITE.md
    pub fn guide(&self) -> PathBuf;                             // root/.sprawling/GUIDE.toml
    pub fn devices(&self) -> PathBuf;                           // root/.sprawling/remote/devices.toml（§8-76）
    pub fn playback_exports(&self) -> PathBuf;                  // root/.sprawling/playback
    pub fn city_address(&self) -> Option<Address>;             // 城自己的名字：根目录名，能拼成地址时
    pub fn of_ledger(dir: &Path) -> Option<CityLayout>;         // ledger() 的逆：从账本目录取回城根
}
```

**五条口径：**

1. **一个事实一个家，而这里的事实是「哪一类文件落在哪」。** 每个目录名与文件名在这里声明一次，每条路径在这里拼一次；调用点各自拼路径时，改名一个目录要靠 grep 找齐，漏掉的那一处会安静地读一个空目录。
2. **形状 2 值，不碰磁盘。** `CityLayout` 只回答某个文件*会在*哪里；创建、读取、拒绝归拥有 I/O 的那一层。路径是数据而不是效应，故它住在 kernel，与它所依赖的地址文法同处一地（ARCHITECTURE.md 第 1 段）。
3. **`RESERVED_PREFIX` 仍住在 `kernel::address`，本模块引用它。** 它是地址文法的一部分——`is_reserved` 是写域与读路径共用的谓词（8-2、8-55）——而不是一条布局规定。布局这一侧只决定「什么落在保留子树里」：治理一个 scope 的文件（`CONFIG.toml`、`FILTERS.toml`、`skills`）落在该 scope 的 `.sprawling/` 下，于是没有任何写域够得到它们；居民自己写的文件（`JOB.md`、`Handoff.md`、`URBANITE.md`、`Archive/`）落在明处。这条摆放规则由单元测试逐个方法核对，而不是靠注释重申。
4. **逐段 push 而不是整串 join。** 一个地址在 Windows 与在 Linux 必须落成同一个目录树；整串 join 把 `/` 交给平台去解释，逐段 push 不给它这个机会。
5. **一个落点一个方法，不是便利方法。** 少一个落点，就有一处调用点继续自己拼，于是本模块不再是唯一权威。后续新增一类文件时，先在此加方法与常量，再写调用点。
6. **session 切片的路径不在此处。** 切片是账本的可弃投影，只有 `storage::sessions` 一个写者、没有读者；它的目录名与路径推导是该模块的私有项（`crates/storage/Spec.lean` §8-24），于是「别处点名这条路」在编译期就写不出来，不必再靠文本扫描去拦。
7. **`of_ledger` 是 `ledger` 的逆，为「只拿到账本目录」的写者而存在。** 账本的写者手里只有它打开的那一个目录，而切片落在城根之下，故城根必须能从这一个输入反推回来；逆运算住在具名常量所在的同一模块里，任何调用点都不许用 `parent().parent()` 重新拼一遍。不是 `ledger()` 形状的目录不是城（夹具、bundle 的校验台、直接打开的存储），回答 `None`。
-/

/-!
### 8-76 远程门的设备表在城的保留子树里（`kernel::layout`，形状 2 值）

```rust
pub const REMOTE_DIR: &str = "remote";
pub const DEVICES_FILE: &str = "devices.toml";
impl CityLayout {
    pub fn devices(&self) -> PathBuf;   // root/.sprawling/remote/devices.toml
}
```

- **它治理的是谁能从外面够到这座城**，所以按 §8-56 第 3 条落在城的保留子树里：没有任何写域够得到它，一个 agent 改不了哪台设备配对过、各有什么权限。表里每台设备一行：id、人给的名字、权限与公钥；表的格式、读写与跨重启的持久化归远程门的装配层（sprawling-SPEC 8-139），本模块只回答它在哪。
- **`remote/` 一个目录，而不是保留子树根下一个文件**：远程门落盘的状态都归这一个目录，于是撤掉远程门在盘上留下的一切就是删掉一个目录，不必逐个认文件。城密钥不在这里，它进 vault。
- 第一个读者是远程门的装配；在它之前，这个路径没有调用方，由 `layout` 的单元测试核它的摆放。

**playback 的导出件也在城的保留子树里**：`PLAYBACK_DIR` 与 `CityLayout::playback_exports()`（`root/.sprawling/playback`）是居民经城工具 `playback` 导出的报告落下的地方，每栋楼一个子目录。放在这里，是因为没有写域够得到它、城根的 `.gitignore` 把它挡在历史之外、清扫 worktree 也不碰它；目录下的命名、落盘与寿命归 `accounting::playback`（`crates/accounting/Spec.lean` §8-13、sprawling-SPEC.md 8-132），本模块只回答它在哪。
-/

namespace Kernel.Layout

open Kernel.Address

/-! ### 模型

一条路径是它的段，从城根之上的某处数起：`CityLayout` 逐段 push，所以在 Windows 与 Linux 上落成同一棵树（口径 4）。目录名与文件名是参数 `Names`：它们的值只住 `crates/kernel/src/layout.rs` 与 `kernel::address`（`RESERVED_PREFIX`），这里只说摆放规则，名字换了，下面的每一条陈述一字不变。 -/

/-- 布局用到的名字。`reserved` 是 `RESERVED_PREFIX`，其余是 `layout` 里同名的常量。 -/
structure Names where
  reserved : String
  ledger : String
  snapshot : String
  cas : String
  library : String
  skills : String
  config : String
  filters : String
  archive : String
  job : String
  handoff : String
  urbanite : String
  guide : String
  remote : String
  devices : String
  playback : String

/-- 一座城的根。 -/
structure CityLayout where
  root : List String
  deriving DecidableEq, Repr

/-- 一个 scope 的目录：城根加地址的每一段。 -/
def scope (city : CityLayout) (addr : Address) : List String :=
  city.root ++ addr.segments

/-- 城自己的保留子树。 -/
def governed_root (names : Names) (city : CityLayout) : List String :=
  city.root ++ [names.reserved]

/-- 一个 scope 自己的保留子树：治理它的文件住这里。 -/
def governed (names : Names) (city : CityLayout) (addr : Address) : List String :=
  scope city addr ++ [names.reserved]

def ledger (names : Names) (city : CityLayout) : List String :=
  governed_root names city ++ [names.ledger]

def config (names : Names) (city : CityLayout) (addr : Address) : List String :=
  governed names city addr ++ [names.config]

def filters (names : Names) (city : CityLayout) (addr : Address) : List String :=
  governed names city addr ++ [names.filters]

def building_skills (names : Names) (city : CityLayout) (addr : Address) : List String :=
  governed names city addr ++ [names.skills]

def job (names : Names) (city : CityLayout) (addr : Address) : List String :=
  scope city addr ++ [names.job]

def handoff (names : Names) (city : CityLayout) (room : Address) : List String :=
  scope city room ++ [names.handoff]

def urbanite (names : Names) (city : CityLayout) (addr : Address) : List String :=
  scope city addr ++ [names.urbanite]

def archive (names : Names) (city : CityLayout) (building : Address) : List String :=
  scope city building ++ [names.archive]

def devices (names : Names) (city : CityLayout) : List String :=
  governed_root names city ++ [names.remote, names.devices]

/-- `CityLayout::of_ledger`：`ledger` 的逆。不是「某个非空的根、其下保留前缀、再下账本目录」这个形状的目录不是城（夹具、bundle 的校验台、直接打开的存储），答 `none`。 -/
def of_ledger (names : Names) (dir : List String) : Option CityLayout :=
  match dir.reverse with
  | last :: parent :: above =>
    if last = names.ledger ∧ parent = names.reserved ∧ above ≠ [] then
      some ⟨above.reverse⟩
    else none
  | _ => none

/-- **`of_ledger` 从账本目录取回它的城。** 账本的写者手里只有它打开的那一个目录，城根必须能从这一个输入反推回来。 -/
theorem of_ledger_inverts_ledger (names : Names) (city : CityLayout) (rooted : city.root ≠ []) :
    of_ledger names (ledger names city) = some city := by
  simp [of_ledger, ledger, governed_root, rooted]

/-- **`of_ledger` 只认账本目录。** 它答出一座城，那座城的账本恰是给它的目录。 -/
theorem of_ledger_answers_only_a_ledger (names : Names) (dir : List String) (city : CityLayout)
    (found : of_ledger names dir = some city) : ledger names city = dir := by
  simp only [of_ledger] at found
  split at found
  · rename_i last parent above shape
    split at found
    · rename_i held
      cases found
      obtain ⟨isLedger, isReserved, _⟩ := held
      have back : dir = (last :: parent :: above).reverse := by
        rw [← shape, List.reverse_reverse]
      simp [ledger, governed_root, back, isLedger, isReserved]
    · cases found
  · cases found

/-- 一个 scope 里一个文件的地址形状：scope 的段再加几段。它就是写域判这个文件时看到的地址。 -/
def relative (addr : Address) (below : List String) : Address :=
  ⟨addr.segments ++ below⟩

/-- 路径与地址的对应：scope 里的文件就是城根加上它的相对地址。 -/
theorem config_is_the_root_and_its_address (names : Names) (city : CityLayout) (addr : Address) :
    config names city addr = city.root ++ (relative addr [names.reserved, names.config]).segments := by
  simp [config, governed, scope, relative]

/-- **治理一个 scope 的文件落在它的保留子树里，没有任何写域够得到它们。** `CONFIG.toml`、`FILTERS.toml` 与楼的 `skills` 都在 `.sprawling` 之下，所以它们的地址是保留的，`WriteDomain::admits` 恒答 `Outside`（`crates/kernel/spec/WriteDomain.lean`）。 -/
theorem governing_files_are_reserved (names : Names) (protected_name : String → Bool)
    (protects : protected_name names.reserved = true) (addr : Address) (leaf : String) :
    is_reserved protected_name (relative addr [names.reserved, leaf]) = true := by
  simp [is_reserved, relative, List.any_append, protects]

/-- **居民自己写的文件落在明处。** 一个不保留的 scope 里，`JOB.md`、`Handoff.md`、`URBANITE.md` 与 `Archive` 的地址不保留，只要它们的名字本身不是受保护的名字。 -/
theorem resident_files_are_in_the_open (protected_name : String → Bool) (addr : Address)
    (open_scope : is_reserved protected_name addr = false) (leaf : String)
    (plain : protected_name leaf = false) :
    is_reserved protected_name (relative addr [leaf]) = false := by
  simp only [is_reserved, relative] at *
  simp [List.any_append, open_scope, plain]

end Kernel.Layout
