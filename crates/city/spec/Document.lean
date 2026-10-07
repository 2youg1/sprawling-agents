-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::document

规定 `document`（`crates/city/src/` 下同名的文件）。一份文档整个换上去，或者旧的留着；读出此刻的字节、判、整份换上。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-27 city::document：一份文档整个换上去，或者旧的留着（形状 4 adapter）

```rust
pub(crate) fn replace(path: &Path, body: &[u8]) -> Result<(), AxError>;
pub fn edit<T>(path: &Path, act: impl FnOnce(&Held<'_>) -> Result<T, AxError>)
    -> Result<T, AxError>;                        // 门面上是 city::edit_document
pub fn edit_against(path: &Path, base: &[u8], body: &[u8]) -> Result<(), AxError>;
pub struct Held<'a> { /* 私有 */ }
impl Held<'_> { pub fn replace(&self, body: &[u8]) -> Result<(), AxError>; }
pub(crate) enum TreeEntry<'a> { Directory(&'a str), File(&'a str, &'a [u8]) }
pub(crate) fn place_tree(target: &Path, entries: &[TreeEntry<'_>]) -> Result<(), AxError>;
```

- **一棵树同样要么整棵、要么没有**：`place_tree` 把一个此刻不存在的目录整棵放上去（§8-28 的整包安装是它唯一的调用者）——每一项在目标旁一个点开头的暂存目录里写好并 `sync_all`，暂存目录里的每一层目录（嵌套的子目录和暂存根）在 macOS 与 Linux 上各自 `sync_all`（Windows 上不做，理由见下文父目录那一段），换到位之后再刷目标所在的目录——只刷根时，换入可能先于某个子目录的目录项落盘，崩溃后读者会看到半个包——再一次 `rename` 把整个目录换到位；读者看到的是没有这个目录或完整的它。它与 `edit` 取同一张按路径的锁表里目标那一把锁，所以同一进程里两次放置同一棵树一个接一个地判「已在」。目标已在即拒（`E_STORAGE_FATAL`），判在暂存之前、判的是路径本身（一个链接也算「已在」），所以被拒时目标旁什么也没暂存：这扇门只放置，不覆盖一棵树——标准库的 `rename` 在 unix 与 Windows 上都会把一个空目录静默换掉，覆盖非空目录则不是一次操作。上次被杀的写者留下的暂存目录从未换到位、没有读者见过，先清掉再写。暂存目录里的文件不再各自经一次暂存改名：整棵树在换到位之前对谁都不可见，逐个文件的改名只多花系统调用。

**`edit` 与 `Held` 对外开放，`replace` 不**：人层 `<home>/.sprawling/config.toml` 不在任何一座城里，却与一份 `CONFIG.toml` 同性质——有人手工编辑它，有解析器把它读回来，同一条命令流写它。它要的正是本模块那两条性质，而**再写一份「要么整份要么不动」就是给 B-49 立第二个权威**，两份实现里迟早有一份漏掉 `sync_all` 或漏掉锁。开放的是读-改-写那扇门（`edit` 与它给出的 `Held`），不是整份覆写那条捷径：`replace` 留在 crate 内，因为城外唯一的调用方做的是读-改-写，而一个能整份覆写的外部调用方就能不读就写。

**城里写下的每一份文件都有人拿解析器读回来**：配置层、楼的规则、一次会话的 JOB.md。就地截断再流式写入，中间有一段时间盘上既不是旧版也不是新版；断电后那段时间不会结束，于是那间房、那栋楼乃至整座城的每一次派活都失败，直到有人手工改那份文件。两条性质把这扇窗关上，而两条都只写在本模块：

- **要么整份，要么不动**：字节先落到目标同目录的暂存文件，`sync_all` 把它交到设备上，再由一次 `rename` 把它放到位。覆盖式 `rename` 在本项目支持的每一种文件系统上是一个操作，所以读者拿到的是旧版或新版，没有第三种。
- **同一刻只有一个写者**：一份文档的读-改-写在本进程内互斥。`Held` 只能从 `edit` 里拿到，于是「改写期间锁是持有的」由类型成立，而不是由每个调用点记得成立。
- **`edit_against` 是有第二个写者那份文档的门**：四份 spine 文档（以及楼的规则），人手在编辑器里与城的命令流同时在写，而本进程的锁对编辑器毫无约束。写者有权替换的只是它起手时读到的那份正文（`base`），文件已经变了就拒而不覆写，报 `E_VERSION_CONFLICT`；`base` 为空即「这份文档本不该存在」，同一条规则读在它的起点。「文件被人动过就拒」这条规则全城只在这里判定，`put_spine` 经它写。

**暂存文件名固定为 `.<文件名>.staging`**：写者在 flush 与 rename 之间被杀会留下它，固定名让下一次写复用同一个位置，而不是攒出一目录谁也说不清归属的碎片。这个拼法只住 `kernel::layout::document_staging_name`（`crates/kernel/spec/Layout.lean` §8-56），本模块与检查点的暂存过滤都问它。点前缀不足以把它挡在扫描之外：检查点的 `add_all` 照样暂存点开头的文件，所以那里按名字拒它（storage D30）。

**父目录的 `sync_all` 只在 unix 上做**（macOS 与 Linux）：目录项住在目录里，光刷文件不够；Windows 没有可供进程打开的目录句柄，也不需要——`rename` 调到的 `MoveFileEx`（`REPLACE_EXISTING`）由文件系统自己记日志。

**锁是本进程的**：一个人在编辑器里改同一份文件不受它约束，而在本仓库现有依赖下也无法约束（跨进程文件锁需要新依赖，`crates/city/Cargo.toml` 只认 `kernel` + std + `toml`/`serde`）。挡住那个人的是原子替换：他的编辑器永远读不到半份文档。

**`create_new` 那一族不归本模块**：`spine_files::write_new`、`building::create` 要的是「独占地认领一个名字」，而 `OpenOptions::create_new` 已经把认领与拒绝合成一个操作。把它们改道本模块只会让一条已经成立的规则多一个家。`gitignore::seal_room` 不在这一族：它封的目录刚由调用方独占建出，封条没有名字要认领，而它要的是整张出现，所以经 `replace` 落盘（city D24）。

**错误面**：`E_STORAGE_FATAL`，主题是失败的那条路径与操作系统的原话，恢复语一句——把目录改成可写、确认磁盘有空间，然后重存。八个写面共用这一句。经 `edit_against` 另有一个码：`E_VERSION_CONFLICT`，含义是「文件不再是你起手时的那份」，恢复语是重读再发。它与 `runtime::tools::edit`、`library::install` 报同一件事的码相同，客户端已有它的词条，故不是新开的一种失败。

**关门条件**：断电模拟——任意时刻杀进程，`CONFIG.toml` 要么是旧版要么是新版。逼近它的是四条测试：一个读者在另一线程反复替换 512 KiB 文档时每次都读到完整的旧版或新版；被杀的写者留下的暂存文件既不是那份文档、也不挡下一次写；两个线程各二百次读-改-写之后计数是四百；一份文档把它上面的目录一并带来。
-/

/-!
### 8-40 city::document 的第三扇门：读出此刻的字节、判、整份换上（`revise`，形状 4 adapter）

```rust
pub fn revise<T>(path: &Path, act: impl FnOnce(&Held<'_>, &[u8]) -> Result<T, AxError>)
    -> Result<T, AxError>;                        // 门面上是 city::revise_document
```

- **一次保存要的是「此刻的字节」，不是「我起手时的正文」。** `edit_against` 拿调用方给的整份正文与盘上的比较；页面对任意一份文档的保存（`crates/wire/Spec.lean` §8-72）带的是 32 字节的版本摘要与几段编辑，判它的是 `documents::save`，它要读的是盘上此刻的全部字节。`revise` 在 `edit` 的锁里把这份文件读出来交给 `act`，`act` 判定、经 `Held::replace` 整份换上，锁在 `act` 返回之前一直持有，所以两个从同一版出发的保存只有先到的那个落下，第二个读到的已经是第一个的字节。
- **没有文件读作空字节**，与 `edit_against` 同一条规则；`edit_against` 就是 `revise` 加一次逐字节比较，「读不到就是空」这条读法只写在 `revise` 一处。别的读错（目录、无权限）是 `E_STORAGE_FATAL`，与本模块其余的写面同一句恢复语。
- **本 crate 不依赖 `documents`。** 判定由调用方交进来：`accounting::worker::commanding::saving` 交的是 `documents::save` 与 `documents::decide`（`crates/accounting/Spec.lean` §8-22）。这扇门只拥有锁、读与整份换上——就是 §8-27 的两条性质。
- **当前状态：修改提案由 run 经工作台的 `proposal` 工具提出与收回**（`crates/accounting/Spec.lean` §8-30）：工具读城里那份文件此刻的一版（documents D35），把一行 `proposal_offered` 记在这次 run 名下，收回时记 `proposal_withdrawn`；run 不改那份文档，接受时由人的决定经 `revise` 落下。run 一边提案一边直接改同一份文档时不由 runtime 的写门判，守边界的是版本：直接的写造出新版本，人决定那张卡时以 `E_VERSION_CONFLICT` 拒（documents D36；citysim `tests/proposal_baseline.rs`）。
- 验收：`document::tests::a_revision_reads_the_bytes_on_disk_and_holds_the_lock_while_it_decides`——两个线程各从同一份文件出发做两百次读-判-换，计数是四百；没有文件时交给 `act` 的是空字节。
-/

/-! D17 读-判-换的门把此刻的字节交给判定，判定留在调用方

**决定**：`city::document` 多一扇门 `revise`：在一份文档的锁里读出它此刻的全部字节，交给调用方的判定，判定经 `Held::replace` 整份换上（§8-40）。`edit_against` 改由它实现。本 crate 不依赖 `documents`。

**理由**：页面的保存与修改提案的决定都是「读此刻、判、换上」，判定是 `documents` 的，锁与整份换上是本模块的，两件事各有一个家。`edit` 已经开放了锁与 `Held`，但让调用方自己读文件，就会出现第二处「读不到就是空」的读法；把读放进门里，`edit_against` 与保存读的是同一次读。

**被否**：①`city` 依赖 `documents`、门面上给一个 `save_document(path, baseline, edits)`：多一条 crate 边，而本 crate 不需要知道编辑长什么样；②`accounting` 在 `edit` 里自己读文件：理由见上；③另开一把锁：一份文档两把锁，`PutSpine` 与 `PutRange` 写同一份 `Roadmap.md` 时互相看不见。

**重开参数**：出现一个要在锁里读别的东西（例如同目录的另一份文件）的写者时，重议门交出的是字节还是 `Held` 加一个读的方法。
-/

/-!
## 模型：读、判、换在一把锁里，所以同一版出发的两个写者只落一个

一份文档在盘上要么不在、要么是一串字节。`revise` 在锁里读出此刻的字节（不在读作空）交给调用方的判定，判定说换成什么或不换；锁在判定返回之前一直持有，所以本进程里的写者一个接一个地走完读—判—换，一串写者的结果就是按它们拿到锁的先后依次执行。暂存文件与 `rename` 的「要么整份要么不动」在模型之外：一次换上在模型里是一步，它是一步靠的是 `Held::replace`（§8-27）。

* 不在的文档读作空字节（`a_missing_document_reads_as_no_bytes`），所以「这份文档本不该存在」就是 `base` 为空。
* `edit_against` 只在盘上仍是 `base` 时换上，否则拒且不动（`a_moved_document_is_refused_and_left`）；从同一版出发的两次保存，不论谁先拿到锁，恰好落下先到的那个（`two_saves_from_one_version_land_once`）。
* 每个写者都在锁里加一，`n` 个写者之后恰好加了 `n`（`increments_under_the_lock_add_up`）；读与写不在一把锁里时有反例（`without_the_lock_an_update_is_lost`），这就是 `revise` 把读放进门里的理由（D17）。
-/

namespace City.Document

/-- 一份文档的字节。 -/
abbrev Bytes := List Nat

/-- 盘上的一份文档：不在，或它的字节。 -/
abbrev Disk := Option Bytes

/-- 锁里那一次读：不在读作空字节。 -/
def read (disk : Disk) : Bytes :=
  disk.getD []

/-- 判定的答案：换成这些字节，或不换。 -/
abbrev Decision (E : Type) := Except E (Option Bytes)

/-- `revise`：读出此刻的字节交给 `act`，按它的答案换上或留着；判定拒了，盘不动。 -/
def revise {E : Type} (disk : Disk) (act : Bytes → Decision E) : Except E Unit × Disk :=
  match act (read disk) with
  | .ok (some body) => (.ok (), some body)
  | .ok none => (.ok (), disk)
  | .error refused => (.error refused, disk)

/-- `edit_against` 的拒绝。 -/
inductive Conflict where
  | VersionConflict
  deriving DecidableEq, Repr

/-- `edit_against`：盘上此刻不是 `base` 即拒，否则换成 `body`。 -/
def againstBase (base body : Bytes) (onDisk : Bytes) : Decision Conflict :=
  if onDisk = base then .ok (some body) else .error .VersionConflict

/-- 一串写者按拿到锁的先后走完，只看盘。 -/
def inTurn {E : Type} (disk : Disk) (writers : List (Bytes → Decision E)) : Disk :=
  writers.foldl (fun held act => (revise held act).2) disk

theorem a_moved_document_is_refused_and_left (disk : Disk) (base body : Bytes)
    (moved : read disk ≠ base) :
    revise disk (againstBase base body) = (.error .VersionConflict, disk) := by
  simp [revise, againstBase, moved]

theorem two_saves_from_one_version_land_once (disk : Disk) (base first second : Bytes)
    (at_base : read disk = base) (changed : first ≠ base) :
    revise disk (againstBase base first) = (.ok (), some first) ∧
      revise (some first) (againstBase base second) = (.error .VersionConflict, some first) := by
  constructor
  · simp [revise, againstBase, at_base]
  · simp [revise, againstBase, read, changed]

/-- 每个写者都在锁里把文档加长一个字节：文档的长度就是计数。 -/
def increment : Bytes → Decision Conflict :=
  fun onDisk => .ok (some (onDisk ++ [0]))

theorem increments_under_the_lock_add_up (disk : Disk) (n : Nat) :
    (read (inTurn disk (List.replicate n increment))).length = (read disk).length + n := by
  induction n generalizing disk with
  | zero => rfl
  | succ n grown =>
    simp only [List.replicate_succ, inTurn, List.foldl_cons]
    have step : (revise disk increment).2 = some (read disk ++ [0]) := rfl
    rw [step]
    have more := grown (some (read disk ++ [0]))
    simp only [inTurn] at more
    rw [more]
    simp [read]
    omega

/-- 不在一把锁里：两个写者各读一次此刻的字节，再各写一次自己读到的加一；第二个在第一个写之前读，后写的那次盖掉先写的。 -/
def unlocked (disk : Disk) : Disk :=
  let first := read disk
  let second := read disk
  let writes : List Disk := [some (first ++ [0]), some (second ++ [0])]
  writes.getLastD disk

theorem without_the_lock_an_update_is_lost :
    (read (unlocked none)).length = 1 ∧ (read (inTurn none [increment, increment])).length = 2 := by
  decide

end City.Document
