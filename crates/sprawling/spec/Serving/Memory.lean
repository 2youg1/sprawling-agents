-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 进程里只放工作集：按字节计预算的缓存

规定城的常驻内存（§8-173）：哪些工作集住在进程里、每一个由谁拥有、什么给它设上界；一个按字节计预算、能从盘上读回的缓存必须守住的性质；私有字节在三个平台上怎样读；从盘上读回时用的定位读；分配器对照与一小时斜率的测量计划。缓存的 Rust 模块是 `storage::resident`（`crates/storage/src/resident.rs`，接口在 `crates/storage/spec/Resident.lean` §8-40），它的 doc comment 链回本分部。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

## 8-173 常驻内存（形状：状态机）

### 清点：进程里的每一份工作集

下表按「谁拥有、今天什么给它设上界、它随什么增长」列出进程里的缓存与工作集。增长量一栏是让它无界的那个变量；「有界」指今天的代码里有一个上界，「随城增长」指它随账本或 run 的数目长，没有上界。

| 工作集 | 拥有者 | 今天的上界 | 增长量 |
|---|---|---|---|
| 视图折叠（`Governance`、各房间与 run 的视图） | `accounting::views` | 无；随账本折叠，每个 run 留一份摘要 | run 数、记录数 |
| 对话窗口（`Conversation` 的 `messages` 与 `held`） | `runtime::conversation` | 模型的上下文窗口经 `runtime::compaction` 与 `runtime::offload` 压住；进程里没有按字节计的上界 | 回合数、工具结果的字节 |
| 冻结 run 的热视图墓碑（`HotView::evicted`） | `storage::hot` | 无；每个冻结的 run 一个 `RunId` | run 数 |
| 待批项的 `sent` 表 | `accounting::worker` | 无；每个 run 一条（两个短字串加 16 字节，`crates/sprawling/spec/Accounting/Worker.lean` 记下了这个代价） | run 数 |
| 账本旁索引（seq → 段与偏移） | `storage::index::ledger` | 无；每条记录一项 | 记录数 |
| 楼规缓存（`RulesCache`） | `city::policy::cache` | 每栋读过的楼一份，按文件戳失效 | 楼数 |
| sieve 历史（`SieveHistory`） | `runtime::sieve` | 每个 run 内每种命令一个 `Locator` | 命令种数 |
| 还在跑的命令的输出环（`OutputRing`） | `bin::serving::output_ring` | 每个 run 64 KiB 或最新的一块（§8-115） | 活的 run 数 |
| 监视页历史 | `bin::monitor` | 300 个点，没人看时释放（§8-94） | 无 |
| 摘要缓存（`DigestCache`） | `storage::digest_cache` | 在盘上，进程里不留 | 无 |
| playback 导出 | `accounting::playback` | 走账本是流式的，但整份 `Document` 与整份 `Bundle` 在内存里，上界 `BUNDLE_MAX_BYTES`（32 MiB） | 选中的记录数 |
| doctor 扫描 | `bin::doctor` | 子进程的输出整份读进内存（`doctor::asking`） | 输出的字节 |

账本与 CAS 已经在盘上，所以上表里随城增长的几项（视图、墓碑、`sent`、旁索引）都可以改成「在盘上，按需读回」；它们读回时走下面的缓存模型。

### 模型：一份按字节计预算的缓存

缓存里的每一项属于一个 run（`owner`），以盘上的地址（`key`）为名，值是那个地址上的字节。盘上的内容按地址定（CAS 与账本都是只追加、内容寻址的），所以模型把盘写成一个不变的函数 `disk`。四种操作：`insert` 读盘并放进缓存，再从最旧的一项丢起，直到常驻字节不超过预算；`read` 命中就答缓存里的值，不命中就读盘并放进缓存；`evict` 丢掉最旧的一项（操作系统要内存时、或 W6b 的节拍逐出）；`freeze` 丢掉这个 run 的每一项，并记下它已冻结，此后不再为它放任何东西。

三条性质，都对从空缓存开始的每一条操作序列成立：

* 常驻字节从不超过预算（`resident_within_budget`）；
* 冻结的 run 不留任何东西（`frozen_run_holds_nothing`）；
* 每一次读答出的都是盘上那个地址的字节，被逐出的项读回时也一样（`read_returns_disk_bytes`、`evicted_reads_back`）。

一项自己就超过预算时，`insert` 不留它，读它的人每次从盘上读；这是「从不超过」而不是 `OutputRing` 那种「不超过或只剩一块」的原因：常驻内存的验收标准是整个进程的私有字节，一块不受上界管的值会让预算失去意义。

预算按字节计而不按项数计的理由见 D42。

### 从模型导出的 Rust 检查

`crates/storage/src/resident/tests.rs` 是一个 `proptest`：生成器产生 `insert`／`read`／`evict`／`freeze` 的任意序列（run 取 0..4，地址取 0..16，值的长度取 0..2×预算，让「一项超过预算」那条分支被走到），盘是一个按地址给字节的 `BTreeMap`。每一步之后断言：常驻字节不超过预算；每个冻结的 run 名下没有项；每次读答出的字节等于盘上的字节。按项数修剪（只留最近的 N 项）的那一版在这个 proptest 上变红，提交次序里先是它的红，再是按字节修剪的绿。

### 私有字节：三个平台各读什么

见下面的 D43。

### 定位读：从盘上读回一项

不用 `mmap`：`memmap2::Mmap::map` 是 `unsafe fn`，本工作区禁 `unsafe`。读回一项用定位读，不移动文件游标，所以多个读者可以共用一个打开的句柄：Windows 上 `std::os::windows::fs::FileExt::seek_read`，Linux 与 macOS 上 `std::os::unix::fs::FileExt::read_exact_at`，都是 std 的安全接口。Windows 的 `seek_read` 会移动游标且可能读不满，所以 Windows 臂循环到读满或读到文件尾。读回的页留在操作系统的文件缓存里，那部分是可回收的，不计入私有字节。

### 预算：每份缓存一个命名的常量

下面是从清点里推出来的提议值，不是读数；W7 的读数出来之后改常量。`RESIDENT_TOTAL_BYTES` 已是 `storage::resident` 里的命名常量；其余三个在它们的工作集改成经缓存读回时，写成拥有者模块里的命名常量。

| 常量 | 拥有者 | 提议值 | 依据 |
|---|---|---|---|
| `VIEWS_RESIDENT_BYTES` | `accounting::views` | 8 MiB | 空城空闲时私有 2.4 MiB，`resident_empty_idle` 的预算 30 MiB，给视图留四分之一 |
| `CONVERSATION_RESIDENT_BYTES` | `runtime::conversation` | 每个活的 run 4 MiB | 长回合 1000 步时请求窗口 1.2 MiB，留三倍余量；超出的工具结果走 `offload` |
| `INDEX_RESIDENT_BYTES` | `storage::index::ledger` | 4 MiB | 每项约 40 字节，十万条记录 |
| `RESIDENT_TOTAL_BYTES` | `storage::resident` | 24 MiB | 视图与旁索引的 12 MiB 加 12 MiB 的读回缓存；对话窗口与 OutputRing 按活的 run 另计；空城空闲时总和仍低于 `resident_empty_idle` 的 30 MiB 预算 |

### 测量计划（W7 读）

* 内存节拍：按 Roadmap M0 第 7 条，默认每 100 ms 采一次私有字节（上面每个平台的那个数），报 p50、p99、max、高于 p50 的累计时长与每小时斜率，n 与读数绑定的条件随读数写下。
* 一小时斜率：TP1 的负载（N 个并发 run，各自做读写与 exec）跑一小时，斜率按最小二乘对时间求；验收标准是斜率在两次空负载一小时读数的差以内。
* 盘读的延迟峰：在交互路径上（切换 session、打开信箱、滚动）读回被逐出的项时，那个交互的 p99 不超过 16 ms。
* 分配器对照：系统分配器（Windows 的堆、glibc 或 musl 的 malloc、macOS 的 libmalloc）与 mimalloc 两臂，同一负载，比私有字节的 p99、斜率与工具调用的 p99；mimalloc 只作对照臂，读数明显更好时再交人定默认。Linux 的发行件是静态 musl，musl 的 malloc 慢且归还内存的行为不同，所以 Linux 臂的结论单独写。

### 待做

1. 内存节拍：100 ms 的采样与 p50／p99／max／高于 p50 的时长／每小时斜率，进 `bin::monitor`。
2. 清点表里随城增长的四项（视图、墓碑、`sent`、旁索引）逐一改成经 `storage::resident` 读回，每一项一个上表的常量，读回用上面的定位读。
3. playback 导出把 `Document` 改成边走边编码，doctor 扫描按行读子进程的输出，不整份读进内存。
4. `tools/xtask/budgets.toml` 加内存节拍与一小时斜率的两行（由那一波拥有 budgets.toml 的车道提交）。
-/

/-! D43 私有字节在三个平台上各读什么：Windows 读提交的私有字节，Linux 读 smaps_rollup，macOS 在有安全接口之前读虚拟大小并标明

`bin::monitor::counters::own_process` 报的 `private_bytes` 按平台读：

* Windows：`memory-stats` 的 `PagefileUsage`（`PROCESS_MEMORY_COUNTERS`），就是私有提交字节（`PrivateUsage`）；安全接口，已在依赖树里。
* Linux：std 读 `/proc/self/smaps_rollup`，取 `Private_Clean` 与 `Private_Dirty` 之和（内核 4.14 起有，容器里也有）；没有这个文件或它不给这两行时，读 `/proc/self/status` 的 `RssAnon`（匿名驻留页，不含换出的私有页，所以偏低）；两个都读不到时读作 0。都是普通文件，std 读它们不需要 `unsafe`，与 §8-96 读 `/proc/self/io` 同一种做法。
* macOS：`phys_footprint` 只能经 `task_info(TASK_VM_INFO)` 取得，那是一次 FFI 调用。按 AGENTS.md 平台调用的次序先找对外只给安全接口的 crate；没有找到之前，macOS 上报 `memory-stats` 的 `task_basic_info.virtual_size`，它是虚拟大小，比私有字节大得多，不为它写 `unsafe`。

读数用的是哪一种，由平台决定（Linux 的 `RssAnon` 退路在运行时决定）；`wire::frames::monitor::Sample` 没有携带来源的字段，所以这一点写在这里与 `crates/sprawling/spec/Monitor.lean` §8-96，屏幕上不标。给 `Sample` 加来源字段是一次线上协议的改动，等下一次改监视帧时一起做。

理由：Linux 的两个文件 std 就能读，代价是一次小文件读（smaps_rollup 在内核里按段求和，段多时比 status 慢）；macOS 的 `phys_footprint` 是活动监视器报的那个数，但它要 FFI。被否的做法：三个平台都报 RSS（工作集）：RSS 包括可回收的文件页，缓存交给操作系统的文件缓存之后 RSS 会随读盘上升，而这正是本设计要鼓励的。重开的条件：出现以安全接口读 `phys_footprint` 的 crate。
-/

namespace Sprawling.Serving.Memory

structure Entry (R K V : Type) where
  owner : R
  key : K
  value : V

structure Cache (R K V : Type) where
  entries : List (Entry R K V)
  frozen : List R

inductive Op (R K : Type) where
  | insert (owner : R) (key : K)
  | read (owner : R) (key : K)
  | evict
  | freeze (owner : R)

variable {R K V : Type}
variable (size : V → Nat) (budget : Nat) (disk : K → V)

/-- 常驻字节：各项的值的字节数之和。 -/
def resident (entries : List (Entry R K V)) : Nat :=
  (entries.map fun e => size e.value).sum

/-! D42 缓存的预算按字节计，不按项数计

理由：项的大小相差几个数量级（一条记录几百字节，一份工具结果可以是几 MiB），按项数计的上界只在项一样大时才对应一个字节数，而验收标准（Roadmap M0 第 7 条）是私有字节。被否的做法：按项数计的缓存（下面的 `Rival`），它在 `entry_budget_breaks` 里给出一条两步的序列，常驻字节超过它想代表的字节预算。重开的条件：缓存里的项改成定长。
-/
/-- 从最旧的一项丢起，直到常驻字节不超过预算。 -/
def trim : List (Entry R K V) → List (Entry R K V)
  | [] => []
  | oldest :: rest =>
    if resident size (oldest :: rest) > budget then trim rest else oldest :: rest

def empty : Cache R K V := ⟨[], []⟩

/-- 每一步都守住的不变式：不超预算；每一项是盘上的字节；冻结的 run 名下没有项。 -/
def Good (c : Cache R K V) : Prop :=
  resident size c.entries ≤ budget ∧
    ∀ e ∈ c.entries, e.value = disk e.key ∧ e.owner ∉ c.frozen

theorem resident_cons (e : Entry R K V) (es : List (Entry R K V)) :
    resident size (e :: es) = size e.value + resident size es := by
  simp [resident]

theorem resident_sublist {l₁ l₂ : List (Entry R K V)} (h : l₁.Sublist l₂) :
    resident size l₁ ≤ resident size l₂ := by
  induction h with
  | slnil => exact Nat.le_refl _
  | cons a _ ih => rw [resident_cons]; omega
  | cons_cons a _ ih => rw [resident_cons, resident_cons]; omega

theorem trim_bounded (es : List (Entry R K V)) : resident size (trim size budget es) ≤ budget := by
  induction es with
  | nil => simp [trim, resident]
  | cons x rest ih =>
    unfold trim
    split
    · exact ih
    · omega

theorem trim_mem (es : List (Entry R K V)) : ∀ e ∈ trim size budget es, e ∈ es := by
  induction es with
  | nil => simp [trim]
  | cons x rest ih =>
    intro e he
    unfold trim at he
    split at he
    · exact List.mem_cons_of_mem x (ih e he)
    · exact he

theorem good_empty : Good size budget disk (empty : Cache R K V) := by
  simp [Good, empty, resident]

variable [DecidableEq R] [DecidableEq K]

def place (c : Cache R K V) (owner : R) (key : K) : Cache R K V :=
  if owner ∈ c.frozen then c
  else
    { c with
      entries := trim size budget
        (c.entries.filter (fun e => !(e.owner == owner && e.key == key)) ++
          [⟨owner, key, disk key⟩]) }

def lookup (c : Cache R K V) (owner : R) (key : K) : Option V :=
  (c.entries.find? fun e => e.owner == owner && e.key == key).map (·.value)

/-- 一次读：答出的字节与读之后的缓存。 -/
def read (c : Cache R K V) (owner : R) (key : K) : V × Cache R K V :=
  match lookup c owner key with
  | some v => (v, c)
  | none => (disk key, place size budget disk c owner key)

def step (c : Cache R K V) : Op R K → Cache R K V
  | .insert owner key => place size budget disk c owner key
  | .read owner key => (read size budget disk c owner key).2
  | .evict => { c with entries := c.entries.drop 1 }
  | .freeze owner =>
    { entries := c.entries.filter (fun e => !(e.owner == owner)), frozen := owner :: c.frozen }

def replay (ops : List (Op R K)) : Cache R K V :=
  ops.foldl (step size budget disk) empty

theorem good_place (c : Cache R K V) (owner : R) (key : K) (h : Good size budget disk c) :
    Good size budget disk (place size budget disk c owner key) := by
  unfold place
  split
  · exact h
  · rename_i hf
    refine ⟨trim_bounded size budget _, ?_⟩
    intro e he
    have hm := trim_mem size budget _ e he
    rcases List.mem_append.mp hm with hold | hnew
    · exact h.2 e (List.mem_filter.mp hold).1
    · rw [List.mem_singleton] at hnew
      subst hnew
      exact ⟨rfl, hf⟩

theorem good_step (c : Cache R K V) (op : Op R K) (h : Good size budget disk c) :
    Good size budget disk (step size budget disk c op) := by
  cases op with
  | insert owner key => exact good_place size budget disk c owner key h
  | read owner key =>
    simp only [step, read]
    split
    · exact h
    · exact good_place size budget disk c owner key h
  | evict =>
    refine ⟨Nat.le_trans (resident_sublist size (List.drop_sublist 1 c.entries)) h.1, ?_⟩
    intro e he
    exact h.2 e (List.mem_of_mem_drop he)
  | freeze owner =>
    refine ⟨Nat.le_trans (resident_sublist size (List.filter_sublist)) h.1, ?_⟩
    intro e he
    have hp := (List.mem_filter.mp he).2
    have hold := h.2 e (List.mem_filter.mp he).1
    refine ⟨hold.1, ?_⟩
    intro hin
    rcases List.mem_cons.mp hin with heq | hrest
    · simp [heq] at hp
    · exact hold.2 hrest

theorem good_foldl (ops : List (Op R K)) (c : Cache R K V) (h : Good size budget disk c) :
    Good size budget disk (ops.foldl (step size budget disk) c) := by
  induction ops generalizing c with
  | nil => exact h
  | cons op rest ih => exact ih _ (good_step size budget disk c op h)

theorem good_replay (ops : List (Op R K)) :
    Good size budget disk (replay size budget disk ops : Cache R K V) :=
  good_foldl size budget disk ops empty (good_empty size budget disk)

/-- 常驻字节从不超过预算。 -/
theorem resident_within_budget (ops : List (Op R K)) :
    resident size (replay size budget disk ops : Cache R K V).entries ≤ budget :=
  (good_replay size budget disk ops).1

/-- 冻结的 run 不留任何东西。 -/
theorem frozen_run_holds_nothing (ops : List (Op R K)) (owner : R)
    (h : owner ∈ (replay size budget disk ops : Cache R K V).frozen) :
    ∀ e ∈ (replay size budget disk ops : Cache R K V).entries, e.owner ≠ owner := by
  intro e he heq
  exact ((good_replay size budget disk ops).2 e he).2 (heq ▸ h)

theorem read_good (c : Cache R K V) (owner : R) (key : K) (h : Good size budget disk c) :
    (read size budget disk c owner key).1 = disk key := by
  unfold read
  split
  · rename_i v hv
    unfold lookup at hv
    rcases Option.map_eq_some_iff.mp hv with ⟨e, hfind, hval⟩
    have hmem := List.mem_of_find?_eq_some hfind
    have hkey : e.key = key := by
      have hp := List.find?_some hfind
      simp only [Bool.and_eq_true, beq_iff_eq] at hp
      exact hp.2
    rw [← hval, (h.2 e hmem).1, hkey]
  · rfl

/-- 每一次读答出的都是盘上那个地址的字节。 -/
theorem read_returns_disk_bytes (ops : List (Op R K)) (owner : R) (key : K) :
    (read size budget disk (replay size budget disk ops) owner key).1 = disk key :=
  read_good size budget disk _ owner key (good_replay size budget disk ops)

/-- 被逐出的项读回时，字节与放进去时相同。 -/
theorem evicted_reads_back (ops : List (Op R K)) (owner : R) (key : K) :
    (read size budget disk (replay size budget disk (ops ++ [.insert owner key, .evict])) owner
      key).1 = disk key :=
  read_returns_disk_bytes size budget disk _ owner key

/-! 被否的设计：按项数计的缓存只留最近的 `cap` 项。 -/
namespace Rival

def trimCount (cap : Nat) (es : List (Entry R K V)) : List (Entry R K V) :=
  es.drop (es.length - cap)

def stepCount (cap : Nat) (c : Cache R K V) : Op R K → Cache R K V
  | .insert owner key | .read owner key =>
    { c with entries := trimCount cap (c.entries ++ [⟨owner, key, disk key⟩]) }
  | .evict => { c with entries := c.entries.drop 1 }
  | .freeze owner =>
    { entries := c.entries.filter (fun e => !(e.owner == owner)), frozen := owner :: c.frozen }

def replayCount (cap : Nat) (ops : List (Op R K)) : Cache R K V :=
  ops.foldl (stepCount disk cap) empty

end Rival

/-- 按项数计的上界不是字节上界：预算 10 字节、按「每项约 5 字节」定成 2 项，
两次放进 9 字节与 8 字节的项，常驻 17 字节。 -/
theorem entry_budget_breaks :
    ∃ ops : List (Op Nat Nat),
      10 < resident id (Rival.replayCount (fun k : Nat => k) 2 ops : Cache Nat Nat Nat).entries :=
  ⟨[.insert 0 9, .insert 0 8], by decide⟩

/-! 每条分支各走一次：超预算的一项不留；命中答缓存；冻结之后不再放。 -/

example : (replay id 10 (fun k : Nat => k) [.insert 0 11] : Cache Nat Nat Nat).entries.length = 0 := by
  decide

example : (replay id 10 (fun k : Nat => k) [.insert 0 4, .insert 1 5, .read 0 4] :
    Cache Nat Nat Nat).entries.length = 2 := by
  decide

example : (replay id 10 (fun k : Nat => k) [.insert 0 4, .insert 1 5, .insert 2 6] :
    Cache Nat Nat Nat).entries.length = 1 := by
  decide

example : (replay id 10 (fun k : Nat => k) [.freeze 0, .insert 0 4, .read 0 4] :
    Cache Nat Nat Nat).entries.length = 0 := by
  decide

example : (replay id 10 (fun k : Nat => k) [.insert 0 4, .evict] :
    Cache Nat Nat Nat).entries.length = 0 := by
  decide

end Sprawling.Serving.Memory
