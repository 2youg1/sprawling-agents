-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::cas

规定 `cas`、`cas::origin`（`crates/storage/src/` 下同名的文件）。BLAKE3 寻址存储：临时文件再 rename、去重、块的来源，以及文档的一版就是库里的一个对象。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-3 storage::cas

```rust
pub struct Cas { /* Box<dyn Vfs>、dir —— 非泛型，理由同 jsonl（Vfs 不得漏入公开签名） */ }
impl Cas {
    pub fn open(dir: &Path) -> Result<Self, StorageError>;           // RealFs；建目录＋清别的进程留下的 tmp/*.part
    pub(crate) fn open_with(vfs: Box<dyn Vfs>, dir: &Path) -> …;    // 测试注入点
    /// Content-addressed put: tmp + rename, dedup by existence.
    pub fn put(&mut self, bytes: &[u8]) -> Result<B3Hash, StorageError>;
    /// put＋记下这块是为哪个 run、哪栋楼存的；两者都落盘才返回 Ok。
    pub fn put_for(&mut self, bytes: &[u8], origin: &BlockOrigin) -> Result<B3Hash, StorageError>;
    /// 这块的全部来源，先存先列；没记过来源的块与没见过的哈希都是空表。
    pub fn origins(&self, hash: &B3Hash) -> Result<Vec<BlockOrigin>, StorageError>;
    pub fn contains(&self, hash: &B3Hash) -> bool;                  // 存在判定无可失败面，不包 Result
    /// Full read re-verifies the hash (cheap: BLAKE3 GB/s); mismatch ⇒ CasCorrupt.
    pub fn get(&self, hash: &B3Hash) -> Result<Vec<u8>, StorageError>;
    /// Range read per Locator semantics (L: 1-based closed; B: 0-based closed).
    /// Reads only the named bytes; trusts the object as verified at put.
    pub fn get_range(&self, hash: &B3Hash, range: &Range) -> Result<Vec<u8>, StorageError>;
    /// 对象的字节数，不读内容；没有这个对象即 CasMissing（§8-36）。
    pub fn size(&self, hash: &B3Hash) -> Result<u64, StorageError>;
}
```

**范围读只读要答的那一段，并且不校验。** 取回走 `Vfs::read_at`：`B` 式一次定位读取 `to-from+1` 字节，**短答即越界**（文件到头了，拒而不夹取）；`L` 式自对象开头按 64 KiB 块扫换行，扫到第 `to` 行的终止符即止，付的是答案**之前**的字节，从不付答案之后的字节。语义仍归 `storage::cas::ranges` 一处（`of_object`），`Cas::get_range` 只做存在判定与路径解析。

**两条路里选了「不校验，调用方明示接受」，另一条（分块哈希）落选。** 一个对象的地址覆盖整份内容，拿它校验一个片段就必须把整份读回来重算 BLAKE3——那正是本条要去掉的代价。要让片段可校验就得改写入面：put 时另存一棵分块哈希树（BLAKE3 的可验证流式形态），于是每个对象多一份旁挂物、多一条要与对象保持同步的事实、并且旧对象无树可用。买到的是「范围读能发现位腐烂」，而位腐烂**已经**由 `get` 的全读复算发现，且 D4 已把 `CasCorrupt` 记为不可定义掉的外部事故。因此：**范围读信任 put 时的校验，需要地址被证明的调用方走 `get`**；这句话在 `cas/ranges.rs` 的模块文档里逐字重复一遍，因为改那段代码的人先读的是它。

```rust
pub struct BlockOrigin { pub run: RunId, pub building: Address }
```

**块的来源在存块时记下，不事后推断。** 为一个 run 存块的调用方（转录、卸载、截图）走 `put_for`；上架的技能包是全城的，走 `put`，不带来源。来源记录在 `<dir>/from/<hex 前 2>/<hex64>`，一行一个 `<run> <address>`：同一份字节可能为两栋楼各存一次，所以一个块可以有多个来源，同一来源再存不重复记。记录追加后 `sync_data`＋`sync_dir`。读回来不成形的行（崩溃截断的追加尾）不授予任何东西——读取界往关的一侧失败。已有字节不以换行结尾时（一次崩溃截断的追加尾），新记录前先补一个换行：否则新行接在残尾后面，读回时与残尾一起被丢，而 `put_for` 已经返回了 `Ok`。另一条路是按账本里哪一行写了这个哈希来推断来源，落选：模型写的文字会落进带地址的行，那样的归属可以伪造。

布局：`<dir>/b3/<hex 前 2>/<hex64>`；临时件 `<dir>/tmp/<hex64>.<pid>.<本进程第几次 put>.part`（每次 put 一个自己的名字：同内容并发写者各自写满、各自 rename 到同一目标，无随机源；开句柄只清 pid 不是本进程的 tmp，理由见 D1；同名残留先 truncate 再写）。put 四步：hash→已存在即去重返回→写 tmp＋`sync_data`→`rename`＋`sync_dir`（分片目录）。范围取回越界＝`RangeOutOfBounds`（fail-closed，不静默夹取）；`L` 式行切分按 `\n`，末行无终止符同计一行；返回字节含行间 `\n`、不含末行终止符；`B` 式按 0 起闭区间直切。
rename 入 Vfs；FaultFs 模型：rename 原子；新目标目录项在 `sync_dir` 前不存活，断电即整体消失（源已移除）——看似比真实更损，但 put 尚未返回 Ok，无可观察效果被丢失，A3 点 2 的断言面（已命名对象恒不腐蚀）不受影响。
-/

/-!
### 8-36 文档的一版是内容库里的一个对象（`storage::cas`）

```rust
impl Cas {
    pub fn size(&self, hash: &B3Hash) -> Result<u64, StorageError>;   // Vfs::size；不存在即 CasMissing
}
```

- **版本身份就是地址。** 页面读的一份文档的一版，身份是整份字节的 BLAKE3（`crates/documents/Spec.lean` D3），正是 `put` 给同一份字节的地址；读面把一版放进来走的就是 `put`，不另开目录、不另记索引（accounting-SPEC §8-21）。
- **按版本取一段，先问长度。** 一个窗口要知道这一版多长才能把请求夹在末尾之内（`documents::lift`），然后用 `get_range` 的 `B` 式读那一段；`get_range` 越界即拒的规则不变，夹取是 `documents` 的判定，不是本模块的。`size` 走 `Vfs::size`，不读内容，所以问一个两百兆对象的长度不付两百兆。
- **不校验。** 与范围读同一条理由（§8-3）：长度来自文件系统，地址覆盖的是整份内容。
- 验收：`cas::tests::a_stored_object_states_its_size_and_a_missing_one_is_named`。
-/

/-!
## 模型：已命名的对象恒不腐蚀

内容库的保证是：`b3/` 下以哈希 `h` 命名的对象，内容的哈希就是 `h`。`put` 靠「写临时件、`sync_data`、`rename` 成对象名」守住它；同一个进程里多个句柄并发 `put`、进程在任何一步崩溃，都不能让一个对象名指向别的字节。

模型里一次 `put` 是一个写者 `p`（Rust 里是「这个进程的第几次 put」），它要存的字节是 `bytesOf p`，它的临时件名是 `name (hash (bytesOf p)) p`。一次 `put` 走四步：开临时件（已有同名对象即去重返回，否则建或截空临时件）、写满、`rename` 成对象名；崩溃让所有没写完的 `put` 死掉，下一个句柄清掉别的进程的临时件。各步可以任意交错。

* 临时件名对写者是单射时（D1：内容哈希、进程、本进程的 put 序号），任何交错之后每个对象的内容都哈希到它的名字（`named_objects_never_corrupt`）。
* 临时件只按内容哈希命名时，两次同内容的 `put` 交错一下，后开的那次截空了先开的那次正要 `rename` 的文件，对象就带着空内容落在它的名字上（`shared_temporary_names_corrupt_an_object`）；这就是 D1 否掉的那种命名。
* 一次没有别人打扰的 `put` 让对象名恰好指向它的字节（`a_put_names_its_bytes`）；对象已在时 `put` 什么都不写（`a_second_put_of_the_same_bytes_writes_nothing`）。

`get` 全读复算哈希、范围读信任 `put` 时的校验（§8-3），都建立在这条不变量上；位腐烂与外部改写在模型之外（D4）。
-/

namespace Storage.Cas

/-- 字节。 -/
abbrev Bytes := List Nat
/-- 一次 `put`：Rust 里的 `<pid>.<本进程第几次 put>`。 -/
abbrev Put := Nat

/-- 一次 `put` 走到哪一步。 -/
inductive Phase where
  | idle
  | opened
  | written
  | done
  | dead
  deriving DecidableEq, Repr

/-- 盘上的对象、临时件，与每次 `put` 的进度。 -/
structure World (H N : Type) where
  objects : H → Option Bytes
  tmp : N → Option Bytes
  phase : Put → Phase

/-- 改一个点的函数。 -/
def update {α β : Type} [DecidableEq α] (f : α → β) (a : α) (b : β) : α → β :=
  fun x => if x = a then b else f x

/-- 交错的一步：某次 `put` 走一步，或进程崩溃。 -/
inductive Step where
  | open_ (p : Put)
  | write (p : Put)
  | rename (p : Put)
  | crash
  deriving DecidableEq, Repr

variable {H N : Type} [DecidableEq H] [DecidableEq N]

/-! D1 临时件按「内容哈希＋写者进程＋本进程的 put 序号」命名，开句柄只清别的进程写的临时件

一个进程同时开着多个 `Cas` 句柄（驱动 run 的各条 lane、读图、浏览器工具、前缀视图各开各的），若临时件只以内容哈希命名、且每次开句柄都清空 `tmp/`，则后开的句柄会删掉另一个句柄写好尚未 rename 的临时件（rename 报 `NotFound`，run 以 `E_STORAGE_FATAL` 停下），两个句柄同写一份字节时一方的 `truncate` 还会截掉另一方正要 rename 的文件、给对象留下错误内容。按写者分名后两种撞车都不存在；残留只可能来自已经退出的进程，而同一座城同时只有一个进程（`LedgerHeld`），所以 pid 不同即残留。代价：本进程里 put 失败留下的临时件要到下次启动才清。被否：全进程只开一个句柄（每个开句柄的调用方都得改，且下一个新调用方仍会踩中）；在 put 里遇 `NotFound` 重写一次（仍不防 `truncate` 截断别人的文件）。**重开参数**：若允许两个进程同时写同一座城的 CAS，改为按锁或租约判定残留。
-/

/-- 一步怎样改盘。`open_` 已有同名对象即去重返回；`rename` 找不到自己的临时件（`NotFound`）则这次 `put` 失败；崩溃让开着的 `put` 都死掉，下一个句柄清掉它们的临时件。 -/
def step (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes) (w : World H N) :
    Step → World H N
  | .open_ p =>
    if w.phase p = .idle then
      if (w.objects (hash (bytesOf p))).isSome then { w with phase := update w.phase p .done }
      else { w with tmp := update w.tmp (name (hash (bytesOf p)) p) (some []),
                    phase := update w.phase p .opened }
    else w
  | .write p =>
    if w.phase p = .opened then
      { w with tmp := update w.tmp (name (hash (bytesOf p)) p) (some (bytesOf p)),
               phase := update w.phase p .written }
    else w
  | .rename p =>
    if w.phase p = .written then
      match w.tmp (name (hash (bytesOf p)) p) with
      | some c => { objects := update w.objects (hash (bytesOf p)) (some c),
                    tmp := update w.tmp (name (hash (bytesOf p)) p) none,
                    phase := update w.phase p .done }
      | none => { w with phase := update w.phase p .dead }
    else w
  | .crash =>
    { w with tmp := fun _ => none,
             phase := fun p => match w.phase p with
               | .opened | .written => .dead
               | .idle => .idle
               | .done => .done
               | .dead => .dead }

/-- 走完一串交错的步。 -/
def run (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes) (w : World H N) :
    List Step → World H N
  | [] => w
  | s :: rest => run hash name bytesOf (step hash name bytesOf w s) rest

/-- 不变量：每个对象哈希到它的名字；写满了的 `put`，它的临时件里恰是它的字节。 -/
def Sound (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes) (w : World H N) : Prop :=
  (∀ h c, w.objects h = some c → hash c = h) ∧
    (∀ p, w.phase p = .written → w.tmp (name (hash (bytesOf p)) p) = some (bytesOf p))

theorem step_sound (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes)
    (distinct : ∀ h h' p q, name h p = name h' q → p = q)
    (w : World H N) (s : Step) (sound : Sound hash name bytesOf w) :
    Sound hash name bytesOf (step hash name bytesOf w s) := by
  obtain ⟨objs, tmps⟩ := sound
  cases s with
  | open_ p =>
    simp only [step]
    split
    · split
      · exact ⟨objs, fun q hq => by
          by_cases e : q = p
          · subst e; simp [update] at hq
          · simp only [update, e, if_false] at hq; exact tmps q hq⟩
      · refine ⟨objs, fun q hq => ?_⟩
        by_cases e : q = p
        · subst e; simp [update] at hq
        · simp only [update, e, if_false] at hq
          have ne : name (hash (bytesOf q)) q ≠ name (hash (bytesOf p)) p :=
            fun h => e (distinct _ _ _ _ h)
          simp only [update, ne, if_false]
          exact tmps q hq
    · exact ⟨objs, tmps⟩
  | write p =>
    simp only [step]
    split
    · refine ⟨objs, fun q hq => ?_⟩
      by_cases e : q = p
      · subst e; simp [update]
      · simp only [update, e, if_false] at hq
        have ne : name (hash (bytesOf q)) q ≠ name (hash (bytesOf p)) p :=
          fun h => e (distinct _ _ _ _ h)
        simp only [update, ne, if_false]
        exact tmps q hq
    · exact ⟨objs, tmps⟩
  | rename p =>
    simp only [step]
    split
    · rename_i written
      split
      · rename_i c hc
        have hc' : c = bytesOf p := by
          rw [tmps p written] at hc; exact (Option.some.inj hc).symm
        subst hc'
        refine ⟨fun h c' hh => ?_, fun q hq => ?_⟩
        · by_cases e : h = hash (bytesOf p)
          · subst e; simp only [update, if_true, Option.some.injEq] at hh; subst hh; rfl
          · simp only [update, e, if_false] at hh; exact objs h c' hh
        · by_cases e : q = p
          · subst e; simp [update] at hq
          · simp only [update, e, if_false] at hq
            have ne : name (hash (bytesOf q)) q ≠ name (hash (bytesOf p)) p :=
              fun h => e (distinct _ _ _ _ h)
            simp only [update, ne, if_false]
            exact tmps q hq
      · refine ⟨objs, fun q hq => ?_⟩
        by_cases e : q = p
        · subst e; simp [update] at hq
        · simp only [update, e, if_false] at hq; exact tmps q hq
    · exact ⟨objs, tmps⟩
  | crash =>
    refine ⟨objs, fun q hq => ?_⟩
    simp only [step] at hq
    split at hq <;> simp at hq

/-- **已命名的对象恒不腐蚀。** 临时件名对写者单射时，从任何满足不变量的盘出发（例如空库），任意交错的 `put` 与崩溃之后，每个对象的内容都哈希到它的名字。 -/
theorem named_objects_never_corrupt (hash : Bytes → H) (name : H → Put → N)
    (bytesOf : Put → Bytes) (distinct : ∀ h h' p q, name h p = name h' q → p = q) :
    ∀ (trace : List Step) (w : World H N), Sound hash name bytesOf w →
      ∀ h c, (run hash name bytesOf w trace).objects h = some c → hash c = h := by
  intro trace
  induction trace with
  | nil => intro w sound; exact sound.1
  | cons s rest ih =>
    intro w sound
    exact ih _ (step_sound hash name bytesOf distinct w s sound)

/-- 空库：没有对象、没有临时件、没有开过的 `put`。 -/
def World.empty : World H N := ⟨fun _ => none, fun _ => none, fun _ => .idle⟩

omit [DecidableEq H] [DecidableEq N] in
theorem empty_sound (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes) :
    Sound hash name bytesOf (World.empty : World H N) :=
  ⟨fun _ _ h => by simp [World.empty] at h, fun _ h => by simp [World.empty] at h⟩

/-- **只按内容哈希给临时件起名会腐蚀对象。** 两次 `put` 存同一份字节 `[1]`（哈希取恒等，便于读）：第一次写满之后，第二次开同名的临时件把它截空，第一次再 `rename`，对象 `[1]` 的内容就成了空的。 -/
theorem shared_temporary_names_corrupt_an_object :
    (run id (fun h _ => h) (fun _ => [1]) (World.empty : World Bytes Bytes)
      [.open_ 0, .write 0, .open_ 1, .rename 0]).objects [1] = some [] := by
  rfl

/-- 一次没有别人打扰的 `put`：对象名恰好指向它的字节。 -/
theorem a_put_names_its_bytes (hash : Bytes → H) (name : H → Put → N) (bytesOf : Put → Bytes)
    (w : World H N) (p : Put) (idle : w.phase p = .idle) (absent : w.objects (hash (bytesOf p)) = none) :
    (run hash name bytesOf w [.open_ p, .write p, .rename p]).objects (hash (bytesOf p)) =
      some (bytesOf p) := by
  simp [run, step, idle, absent, update]

/-- 对象已在：`put` 去重返回，盘上的对象与临时件都不变。 -/
theorem a_second_put_of_the_same_bytes_writes_nothing (hash : Bytes → H) (name : H → Put → N)
    (bytesOf : Put → Bytes) (w : World H N) (p : Put) (idle : w.phase p = .idle)
    (present : (w.objects (hash (bytesOf p))).isSome) :
    (step hash name bytesOf w (.open_ p)).objects = w.objects ∧
      (step hash name bytesOf w (.open_ p)).tmp = w.tmp := by
  simp [step, idle, present]

end Storage.Cas

/-! D2 文档的版本不另设存放处，进的就是内容库

一份文档的版本身份是它整份字节的 BLAKE3，与内容库的地址同一个算法、同一个值，所以「把这一版留下来以便按版本读」就是 `put`，「按版本读一段」就是 `size` 加 `get_range`（§8-36）。被否：在城的保留子树里另开一个版本目录——同一份字节两个地址，两处各自要原子写、要清残留、要校验；只记文件路径与修改时间，按版本读时回去读文件——文件动过之后那一版就没了。代价：内容库替页面读过的每一个大文件的每一版留一份，直到内容库长出回收。**重开参数**：内容库有了回收，被回收的版本要一个与「从没有过」分开的答复。
-/
