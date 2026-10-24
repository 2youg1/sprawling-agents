-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::edit 的准入

规定 `tools::edit`、`tools::stamp`、`tools::path_lock`（`crates/runtime/src/` 下同名的文件）。一次 edit 什么时候可以落盘：每个 run 记下它读到的每个文件的戳，edit 只在「这个 run 最后一次看到的就是文件现在的样子」时落盘；对不存在的路径 `old: ""` 新建。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面的节保留它在 runtime 规格里的标签 §8-64，别处引作 `crates/runtime/Spec.lean §8-64`。

这一分部是形式规格：§8-64-2 的模型对它容许的每一条迹成立，Rust 的派生检查是 `runtime::tools::stamp::tests` 里的 proptest，它在随机的多 run 迹上驱动 `ReadStamps` 与 `admission`，断言与本模型相同的结论。
-/

/-!
### 8-64 runtime::tools::edit 的准入（形状 1 判定＋形状 4 适配器）

```rust
// crates/runtime/src/tools/stamp.rs
pub struct Stamp { /* len、modified、identity —— 私有 */ }
impl Stamp { pub(crate) fn of(path: &Path) -> std::io::Result<Option<Stamp>>; }   // None＝路径上没有文件
#[derive(Clone, Default)]
pub struct ReadStamps { /* Arc<Mutex<BTreeMap<Address, Stamp>>> —— 私有 */ }      // 每个 run 一张，read 与 edit 共用
pub(crate) enum Intent { Replace, Create }                                      // Create＝old 为空
pub(crate) enum Admission { Lands, Unread, Moved, Absent, Present }
pub(crate) fn admission(seen: Option<&Stamp>, current: Option<&Stamp>, intent: Intent) -> Admission;

// crates/runtime/src/tools/path_lock.rs
#[derive(Clone, Default)]
pub struct PathLocks { /* 城级：进程里所有 run 共用一张 —— 私有 */ }
impl PathLocks { pub(crate) fn hold(&self, path: &Path) -> PathGuard; }        // 等到没人持有再持有；PathGuard 落下即放

// crates/runtime/src/tools/edit.rs
pub struct EditDesk { pub policy: PolicyReader, pub seen: ReadStamps, pub locks: PathLocks }
impl EditTool {
    pub fn new(city_root: &Path, domain: Address, writable: WriteDomain, desk: EditDesk)
        -> Result<EditTool, AxError>;
}
// args：{path, old, new}；old == "" 是新建。
```

- **戳是（长度、修改时间、文件身份）**，不是内容哈希：read 不再为一份可能永远不改的文件算 BLAKE3。身份在 Unix 是 `(dev, ino)`，在 Windows 是（卷序列号、文件索引）。edit 自己的写经暂存文件改名（`crates/storage/Spec.lean` §8-32），每次写都换一个身份，所以同一 run 的连续 edit 靠写后记下的新戳，而粗粒度时钟下同长度、同修改时间的外来改写只会漏过身份不变的原地改写——那一条由锁内的唯一匹配兜住，严格拒绝只是提示，替换是否正确由唯一匹配决定。
- **什么算看过**：read 一个文件，答复是整份（没被截断），或者调用显式给了 `offset`／`limit`（窗口读）；读前读后各取一次戳，两次不等就不记。被截断的整读不算，因为模型以为自己看到了整份。catalog、`cas:` 定位符、包内文档不记戳：edit 改不到它们。edit 落盘后记下新戳；被「读后被改」拒绝时，拒绝里带着 `old` 附近的当前片段，这个片段算一次窗口读，记下当前戳，所以模型照片段改好 `old` 直接重试，不必整份重读。正因为拒绝算一次观察，它总要给出一段：`old` 的首行已不在文件里时，片段取文件开头，免得这个 run 凭一次什么也没看到的「观察」被准入。
- **读戳只在内存里，每个 run 一张，不进账本。** 能从账本起出来的 run——重启后续接会话（新 run 经 `origin` 继承母 run 的对话）、fork、`succeed` 的继任、`delegate` 的子 run——起点都没有读戳，第一次 edit 前要读，所以没有要从账本重建的读戳；运行时也没有同一个 run 跨进程续跑。
- **按路径的锁**：键是这个 run 写入的树里的位置（`write_root` 加地址），所以两个工作树里的同一地址是两把锁；键按大小写折叠后比较，因为 Windows 与 macOS 的卷上 `Notes.md` 与 `notes.md` 是同一个文件，不折叠就是同一文件上的两把锁，在区分大小写的卷上折叠只让只差大小写的两个文件互相等待。锁内依次是取戳、读字节、判准入、唯一匹配、写、取新戳；在模型里这就是一步 `edit`。它只串行化 edit 与 edit：exec 与人的编辑器不拿这把锁，对它们只剩判定与改名之间的窗口。链接检查（`crates/storage/Spec.lean` §8-25）在取锁之前做一次，好让经链接的 edit 答「经链接」而不是「没读过」；改名之前再做一次，因为等锁与读字节之间可能有人放了链接。
- **错误**：读后被改答 `E_VERSION_CONFLICT`，why 里是片段；没读过答 `E_INVALID_ARGS`，恢复语「先 read，窗口读即可」；路径已存在却 `old: ""` 答 `E_VERSION_CONFLICT`；路径不存在而 `old` 非空答 `E_INVALID_ARGS`，恢复语指向 `old: ""`。
-/

namespace Runtime.Tools.Edit

/-! #### 8-64-1 模型

路径、run、戳都是自然数。每一次落盘铸一个新戳 `fresh`，之后 `fresh` 加一，所以一个戳一旦被覆盖就不会再回到这个路径上——这是 Rust 里「改名换身份」在模型里的样子。 -/

structure World where
  content : Nat → Option Nat
  seen : Nat → Nat → Option Nat
  fresh : Nat

inductive Admission where
  | lands
  | unread
  | moved
  | absent
  | present
  deriving DecidableEq, Repr

/-- D105 的判定，与 `runtime::tools::stamp::admission` 同构。 -/
def admission (seen current : Option Nat) (creating : Bool) : Admission :=
  match current with
  | none => if creating then .lands else .absent
  | some c =>
    if creating then .present
    else match seen with
      | none => .unread
      | some s => if s = c then .lands else .moved

inductive Step where
  /-- 整份读或窗口读：记下当前戳。 -/
  | read (r : Nat) (p : Nat)
  /-- 不经 edit 的写者：exec、人的编辑器。 -/
  | write (p : Nat)
  /-- 锁内的一次 edit；`unique` 是 `old` 在当前文本里恰好命中一次。 -/
  | edit (r : Nat) (p : Nat) (creating : Bool) (unique : Bool)
  deriving Repr

def World.look (w : World) (r : Nat) (p : Nat) (v : Option Nat) : World :=
  { w with seen := fun r' q => if r' = r ∧ q = p then v else w.seen r' q }

def World.land (w : World) (p : Nat) : World :=
  { w with
    content := fun q => if q = p then some w.fresh else w.content q
    fresh := w.fresh + 1 }

def step (w : World) : Step → World
  | .read r p =>
    match w.content p with
    | none => w
    | some c => w.look r p (some c)
  | .write p => w.land p
  | .edit r p creating unique =>
    match admission (w.seen r p) (w.content p) creating with
    | .lands => if creating || unique then (w.land p).look r p (some w.fresh) else w
    | .moved => w.look r p (w.content p)
    | .unread => w
    | .absent => w
    | .present => w

def runTrace (w : World) (ts : List Step) : World := ts.foldl step w

/-- 一个 step 是不是 run `r` 对路径 `p` 的一次观察。edit 不论结局都算：保守地算多，
定理只会更弱。 -/
def observes (r : Nat) (p : Nat) : Step → Bool
  | .read r' p' => r' == r && p' == p
  | .write _ => false
  | .edit r' p' _ _ => r' == r && p' == p

/-- 每个戳都比 `fresh` 小：铸出的戳从不提前。 -/
def Inv (w : World) : Prop :=
  (∀ p c, w.content p = some c → c < w.fresh) ∧ (∀ r p s, w.seen r p = some s → s < w.fresh)

/-- run `r` 最后看到的就是 `p` 现在的样子。 -/
def Current (w : World) (r : Nat) (p : Nat) : Prop :=
  w.seen r p = w.content p ∧ w.content p ≠ none

/-! #### 8-64-2 性质 -/

theorem a_city_with_nothing_written_holds_the_invariant :
    Inv { content := fun _ => none, seen := fun _ _ => none, fresh := 0 } := by
  constructor <;> intros <;> simp_all

theorem look_keeps_the_invariant (w : World) (r : Nat) (p : Nat) (v : Option Nat)
    (h : Inv w) (hv : ∀ s, v = some s → s < w.fresh) : Inv (w.look r p v) := by
  refine ⟨h.1, ?_⟩
  intro r' q s hs
  simp only [World.look] at hs
  split at hs
  · exact hv s hs
  · exact h.2 r' q s hs

theorem land_keeps_the_invariant (w : World) (p : Nat) (h : Inv w) : Inv (w.land p) := by
  refine ⟨?_, ?_⟩
  · intro q c hc
    simp only [World.land] at hc ⊢
    split at hc
    · cases hc; omega
    · have := h.1 q c hc; omega
  · intro r q s hs
    simp only [World.land] at hs ⊢
    have := h.2 r q s hs; omega

theorem every_step_keeps_the_invariant (w : World) (s : Step) (h : Inv w) : Inv (step w s) := by
  cases s with
  | read r p =>
    simp only [step]
    split
    · exact h
    · rename_i c hc
      exact look_keeps_the_invariant w r p (some c) h (fun s hs => by cases hs; exact h.1 p c hc)
  | write p => exact land_keeps_the_invariant w p h
  | edit r p creating unique =>
    simp only [step]
    split
    · split
      · apply look_keeps_the_invariant _ r p _ (land_keeps_the_invariant w p h)
        intro s hs; cases hs; simp [World.land]
      · exact h
    · apply look_keeps_the_invariant w r p _ h
      intro s hs; exact h.1 p s hs
    · exact h
    · exact h
    · exact h

theorem a_trace_keeps_the_invariant (w : World) (ts : List Step) (h : Inv w) :
    Inv (runTrace w ts) := by
  induction ts generalizing w with
  | nil => exact h
  | cons s rest ih => exact ih (step w s) (every_step_keeps_the_invariant w s h)

/-- 一个 step 只把路径的内容留着不动，或者换成新铸的戳。 -/
theorem content_moves_only_to_the_fresh_stamp (w : World) (s : Step) (p : Nat) :
    (step w s).content p = w.content p ∨ (step w s).content p = some w.fresh := by
  cases s with
  | read r q =>
    simp only [step]; split <;> simp [World.look]
  | write q =>
    simp only [step, World.land]; split <;> simp_all
  | edit r q creating unique =>
    simp only [step]
    split
    · split
      · simp only [World.look, World.land]; split <;> simp_all
      · simp
    · simp [World.look]
    · simp
    · simp
    · simp

/-- 不是 `r` 对 `p` 的观察，就不动 `r` 对 `p` 的戳。 -/
theorem only_an_observation_moves_a_stamp (w : World) (s : Step) (r : Nat) (p : Nat)
    (hs : observes r p s = false) : (step w s).seen r p = w.seen r p := by
  cases s with
  | read r' q =>
    simp only [observes, Bool.and_eq_false_iff, beq_eq_false_iff_ne, ne_eq] at hs
    simp only [step]; split
    · rfl
    · simp only [World.look]
      split
      · rename_i heq; exact absurd heq (by omega)
      · rfl
  | write q => simp [step, World.land]
  | edit r' q creating unique =>
    simp only [observes, Bool.and_eq_false_iff, beq_eq_false_iff_ne, ne_eq] at hs
    have ne : ¬ (r = r' ∧ p = q) := by omega
    simp only [step]
    split
    · split
      · simp [World.look, World.land, ne]
      · rfl
    · simp [World.look, ne]
    · rfl
    · rfl
    · rfl

/-- 别人的一次落盘让 `r` 对 `p` 的戳过期。 -/
theorem another_writer_makes_the_stamp_stale (w : World) (s : Step) (r : Nat) (p : Nat)
    (h : Inv w) (hs : observes r p s = false) (moved : (step w s).content p ≠ w.content p) :
    ¬ Current (step w s) r p := by
  intro ⟨same, _⟩
  rw [only_an_observation_moves_a_stamp w s r p hs] at same
  rcases content_moves_only_to_the_fresh_stamp w s p with keep | minted
  · exact moved keep
  · rw [minted] at same
    have := h.2 r p w.fresh same
    omega

/-- 过期的戳在没有新观察时一直过期。 -/
theorem a_stale_stamp_stays_stale (w : World) (s : Step) (r : Nat) (p : Nat)
    (h : Inv w) (hs : observes r p s = false) (stale : ¬ Current w r p) :
    ¬ Current (step w s) r p := by
  intro ⟨same, present⟩
  rw [only_an_observation_moves_a_stamp w s r p hs] at same
  rcases content_moves_only_to_the_fresh_stamp w s p with keep | minted
  · rw [keep] at same present
    exact stale ⟨same, present⟩
  · rw [minted] at same
    have := h.2 r p w.fresh same
    omega

theorem a_stale_stamp_stays_stale_along_a_trace (w : World) (ts : List Step) (r : Nat) (p : Nat)
    (h : Inv w) (quiet : ∀ s ∈ ts, observes r p s = false) (stale : ¬ Current w r p) :
    ¬ Current (runTrace w ts) r p := by
  induction ts generalizing w with
  | nil => exact stale
  | cons s rest ih =>
    have hs := quiet s (List.mem_cons_self ..)
    exact ih (step w s) (every_step_keeps_the_invariant w s h)
      (fun s' m => quiet s' (List.mem_cons_of_mem _ m))
      (a_stale_stamp_stays_stale w s r p h hs stale)

/-- 一个 run 的戳不是当前的，它的 edit 就不落盘：内容一个字节不动。 -/
theorem an_edit_on_a_stale_stamp_lands_nothing (w : World) (r : Nat) (p : Nat) (unique : Bool)
    (stale : ¬ Current w r p) : (step w (.edit r p false unique)).content p = w.content p := by
  simp only [step]
  split
  · rename_i hadm
    simp only [admission] at hadm
    split at hadm
    · simp at hadm
    · rename_i c hc
      simp only [Bool.false_eq_true, ↓reduceIte] at hadm
      split at hadm
      · simp at hadm
      · rename_i s' hs'
        split at hadm
        · rename_i eq
          exact absurd ⟨by rw [hs', hc, eq], by rw [hc]; simp⟩ stale
        · simp at hadm
  · simp [World.look]
  · rfl
  · rfl
  · rfl

/-- 主定理：run `r` 最后一次观察 `p` 之后，只要有别人落过盘，`r` 的 edit 就不落盘。
`ts` 是那次观察之后的一切交错，任何 run、任何写者、任何顺序。 -/
theorem an_admitted_edit_follows_its_runs_last_look (w : World) (ts : List Step)
    (r : Nat) (p : Nat) (unique : Bool) (h : Inv w)
    (quiet : ∀ s ∈ ts, observes r p s = false)
    (written : (runTrace w ts).content p ≠ w.content p) :
    (step (runTrace w ts) (.edit r p false unique)).content p = (runTrace w ts).content p := by
  apply an_edit_on_a_stale_stamp_lands_nothing
  induction ts generalizing w with
  | nil => exact absurd rfl written
  | cons s rest ih =>
    have hs := quiet s (List.mem_cons_self ..)
    have quietRest : ∀ s' ∈ rest, observes r p s' = false :=
      fun s' m => quiet s' (List.mem_cons_of_mem _ m)
    have h1 := every_step_keeps_the_invariant w s h
    by_cases moved : (step w s).content p = w.content p
    · apply ih (step w s) h1 quietRest
      simp only [runTrace, List.foldl_cons] at written
      rw [← moved] at written
      exact written
    · exact a_stale_stamp_stays_stale_along_a_trace (step w s) rest r p h1 quietRest
        (another_writer_makes_the_stamp_stale w s r p h hs moved)

/-- 没有丢失的更新：两个 run 看到同一份内容，先落盘的那个让后来的那个被拒。 -/
theorem two_runs_on_one_look_land_once (w : World) (r₁ r₂ : Nat) (p : Nat) (u₁ u₂ : Bool)
    (h : Inv w) (other : r₁ ≠ r₂)
    (landed : (step w (.edit r₁ p false u₁)).content p ≠ w.content p) :
    (step (step w (.edit r₁ p false u₁)) (.edit r₂ p false u₂)).content p
      = (step w (.edit r₁ p false u₁)).content p := by
  apply an_edit_on_a_stale_stamp_lands_nothing
  apply another_writer_makes_the_stamp_stale w _ r₂ p h _ landed
  simp only [observes, Bool.and_eq_false_iff, beq_eq_false_iff_ne, ne_eq]
  exact Or.inl other

/-- 没读过就不落盘。 -/
theorem an_unread_file_is_refused (w : World) (r : Nat) (p : Nat) (unique : Bool)
    (unread : w.seen r p = none) (present : w.content p ≠ none) :
    (step w (.edit r p false unique)).content p = w.content p := by
  apply an_edit_on_a_stale_stamp_lands_nothing
  intro ⟨same, _⟩
  rw [unread] at same
  exact present same.symm

/-- 新建只落在没有文件的路径上。 -/
theorem a_create_lands_only_where_nothing_stands (w : World) (r : Nat) (p : Nat) (unique : Bool)
    (landed : (step w (.edit r p true unique)).content p ≠ w.content p) :
    w.content p = none := by
  cases hc : w.content p with
  | none => rfl
  | some c =>
    exfalso; apply landed
    simp [step, admission, hc]

/-- 「读后被改」的拒绝带着当前片段，它算一次观察：拒绝之后这个 run 就是当前的。 -/
theorem a_refusal_for_a_moved_file_counts_as_a_look (w : World) (r : Nat) (p : Nat)
    (unique : Bool) (hm : admission (w.seen r p) (w.content p) false = .moved) :
    Current (step w (.edit r p false unique)) r p := by
  have present : w.content p ≠ none := by
    intro hn; rw [hn] at hm; simp [admission] at hm
  simp only [step, hm]
  exact ⟨by simp [World.look], by simpa [World.look] using present⟩

end Runtime.Tools.Edit

/-! D105 edit 以每个 run 的读戳准入，读后被改则拒绝并给出当前片段

**决定**：edit 不收模型抄写的版本。运行时为每个 run 记下它看过的每个文件的戳（§8-64），edit 在按路径的锁里取当前戳：没读过拒绝；读后被改拒绝，拒绝里带 `old` 附近的当前片段并把它记作一次窗口读；戳是当前的，`old` 恰好命中一次才落盘。对不存在的路径 `old: ""` 新建，路径已存在拒绝。§8-64-2 证明：一个 run 最后一次观察某路径之后，只要别人落过盘，它对这个路径的 edit 就不落盘；两个 run 在同一份内容上的 edit 只落一次。

**理由**：抄写版本让每次读多花约 62 个 token 的哈希串，让每次 read 为可能永远不改的文件算 BLAKE3，还造出「抄错哈希 → 冲突 → 重读」这一类回合。一座城里许多 agent 同时改文件，在过期理解上得到的「唯一匹配」更可能是错的，所以取严格的一侧：读后被改就拒，而不是像 Claude Code 那样只要 `old` 仍唯一就照改并注明。

**被否**：①保留版本但给模型一个短句柄、运行时映射回完整 Locator——省了 token，没省回合，read 仍要哈希；②宽松：读后被改而 `old` 仍唯一就照改——上面的理由；③把读戳写进账本、继承时折叠回来——与「fork、继任、子 run 起点没有读戳」这条决定相反，还让载荷多一个模型看不到的键。

**重开参数**：模型或 provider 开始在一次调用里批量改多个文件、需要跨文件的原子性时；或出现不经暂存改名就落盘的 edit 路径，身份不再随写改变时。
-/
