-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::cas

规定 `cas`、`cas::origin`（`crates/storage/src/` 下同名的文件）。BLAKE3 寻址存储：临时文件再 rename、去重、块的来源，以及文档的一版就是库里的一个对象。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
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
