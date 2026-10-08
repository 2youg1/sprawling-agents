-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::fork

规定 `fork`、`fork::lineage`、`fork::indexed`（`crates/runtime/src/` 下同名的文件）。分叉：一个新 run，它窗口里的历史是另一个 run 的逐字节前缀，从母 run 自己的记录重建。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-2 runtime::fork


```rust
/// Byte-identical fork prefix (A19): raw lines 0..=at_seq of the verified
/// mother sequence. `at_seq` past the tail is E_INVALID_ARGS, never a
/// silent clamp to the end.
pub fn prefix(mother: &VerifiedLedger, at_seq: Seq) -> Result<Vec<Vec<u8>>, AxError>;
/// The run_forked draft for the city Ledger. Caller supplies the new run
/// id, the room it lands in, and the clock reading; fork itself is pure.
pub fn fork_draft(origin: Origin, new_run: RunId, addr: Address, t: TimeMs, who: String)
    -> Result<EventDraft, AxError>;      // data = {"from": …, "at_seq": …}
/// What a new session inherits: the mother's conversation, rebuilt from
/// her own records, cut at the last line a conversation can be cut at,
/// read through the ledger's resident index: the named line for its run,
/// then that run's own lines, and nothing else read.
pub fn inherited_indexed(index: &storage::LedgerIndex, dir: &Path, at_seq: Seq)
    -> Result<Inherited, AxError>;
pub struct Inherited { pub messages: Vec<ChatMessage>, pub at: Seq }
```

**`inherited_indexed` 是「分叉」这个词真正的执行体，而它是重建而不是复制。** 一个 run 的 conversation 由持有它的循环逐回合折起来，进程一停就没有了；折它的那些记录都在账本上。这个函数走母 run 自己的线——开场任务读 `run_started`、助手消息读 `model_returned`、工具结果读 `tool_result`、人中途说的话读 `steer_received`——并把它们**折过流动循环折过的那同一个 `Conversation` 类型**，所以一条分支拿到的次序就是母亲发出去的次序，而不是对同一批记录的第二次读法。**账本已经过一次脱敏**，分支继承的是母亲真正发出去的那份文本。

**切点只有一处权威，而且它往回退。** 一次工具波是好几行，只有它的末尾是能切的地方：`tool_called` 已写、`tool_result` 未写的中间，是一条「assistant 消息的工具调用没有答案」的半个回合，任何 provider 都拒。所以折叠（`fold_run`）维护一个「开着的一波」，命中中途就退回上一次安全点，并在 `Inherited::at` 里如实回报它**实际用到**的那一行——调用方把这一行写进 `run_forked`，于是页面显示的分叉点与模型真正拿到的那一段是同一个事实。

**上下文提醒不重建，也重建不了**：它是这座城在跟模型说这一跑自己的预算，没有属于它自己的记录，而一条分支带着自己的量表开始。这条差异写在函数自己的文档里，因为它是一处诚实的不完整，不是漏掉的一步。**回合边界的压缩会重建**：母亲窗口里每回合的 exchange 是收尾边界压缩后的字节，`fold_run` 在同一边界对同一材料重放同一判定（8-44），分支拿到的是母亲真正发出去的那一份，而不是账本里更全的那一份。

**写账本的那一方走索引（`fork::indexed`）。** 持有账本的 worker 为一次分支重建只需要母 run 自己的那几行：`inherited_indexed` 用 `LedgerIndex::line_at` 读 `at_seq` 那一行定出母 run，再按 `run_seqs_before` 只读这条 run 的行，每一行先过 `storage::read_line`——它与验链门逐行所用的 `LineCheck::advance` 共用同一条分类规则：已知 kind 解析成记录，带 `ig` 的新 kind 跳过，其余（撕裂、不规范、无 `ig` 的未知 kind）以 `LineFault` 的码拒绝；切点与消息都由 `fold_run` 一处判定。它不验链：worker 是这本账唯一的写者，打开时已经过尾部恢复；而 `verify_ledger_dir` 为这一问把整本历史读进内存、逐行验链再解析（94 MB 的账本上是 +67 MiB 的瞬时内存）。拒绝写成人能照做的话：`at_seq` 不在索引里是 `outside`，恢复语给出母序列止于哪个 seq，母 run 在这之前没有 `run_started` 是同一条 `E_INVALID_ARGS`。
**母亲自己是分支时，先重建她开场时继承的那段。** 流动循环里母亲的窗口以 `RunPlan::inherited` 开头，那段对话不在她自己的线上；她的 `run_forked`（`run` 是她、`from`／`at_seq` 指向祖母的切点）才是它的出处。所以 `inherited` 先找属主 run 的 `run_forked`，按其 `at_seq` 递归重建祖母的对话，经 `push_inherited` 放在最前，再折母亲自己的线——与 `Run::begin` 同一顺序。递归只往账本更早处走（`at_seq` 必须早于那条 `run_forked`，否则拒），所以一定终止。


**`addr` 落在 `run_forked` 的那一行上**：新 run 的 id 说的是「谁继续谁」，而地址说的是**哪个房间的这次继承已经用掉了**——`accounting::worker::folds::session` 只用这两个字段回答「这个房间的当前一段是否还欠一段对话」。
-/

/-!
### 8-58 分叉照母 run 开篇的写法重建第一条消息（`runtime::fork`、`runtime::run::charter`，形状 1 判定）


```rust
pub use kernel::event::record::Opening;           // runtime::conversation 与根上各一处再导出（`crates/kernel/Spec.lean` §8-82-1）
pub struct Charter<'a> { /* …既有字段… */ pub opening: Option<Opening> }
// RunPlan::charter 填 Some(self.opening)；harness 的 charter 填 None
```

- **开篇的写法进账本。** `Charter::open` 把 `opening` 照录进 `run_started.opening`，与 `policy`、`naming` 同一处写。
- **`fold_run` 读它，按下表重建母 run 的第一条消息**：

| `run_started.opening` | 重建成 | 与母 run 发出的字节 |
|---|---|---|
| `WithPerson` | `WithPerson`：人的原话 | 相同 |
| `Inherited` | `Inherited`：`Task: …\nGoal: …` | 相同 |
| `FromJob` | `Inherited` | 第一条起不同 |
| 缺席（加键之前的行） | `job` 在场为 `Inherited`，否则 `WithPerson` | 加键之前的读法，不变 |

- **只有 `FromJob` 改写，理由写在 `fold_run` 旁。** 那一句说「任务在上面的 JOB.md 里」，指的是母 run 前缀 run 段里的那份文本；分支的前缀带的是它自己的 brief，不是母 run 的 JOB.md，照抄那一句就是让分支去读一份它从没拿到的文件。改写的代价是 provider 的前缀缓存从第一条消息起不命中，这一种开篇的分支每次都付；换成把母 run 的 JOB.md 抄进分支的 run 段，run 段就与母 run 的不同，缓存在 system 那一段已经不命中，付的一样多，还在分支里多了一份没人派给它的任务（D14）。
- **为什么不用 `goal` 是否为空来猜。** 「有目标才写 job 文件」是 `city::write_brief` 的规则；分叉里按 `goal` 猜写法，就是同一条规则的第二个权威，哪天 brief 的规则改了，分叉会悄悄猜错，而只有缓存命中率会说出来。
- 验收：`fork::request_tests` 的 `a_branch_first_request_opens_with_the_bytes_of_the_mothers_last`（母 run 与分支都经 `drive` 真跑；分支经 `inherited_indexed` 从账本重建；分支第一个请求的消息序列以母 run 最后一个请求的消息序列开头，逐条序列化字节相同）。真实组装出来的前缀经 gateway 按兼容格式渲染后的整份请求，在 accounting 一侧比（`crates/sprawling/Spec.lean` §8-141）。
-/

namespace Runtime.Fork

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `fork::prefix`：验过的母序列从第一行到 `at_seq` 的原始行（A19）。`at_seq` 过了尾就拒绝，恒不静默截到末尾。行的内容与本模型无关，所以行是任意类型。`prefix` 在 Lean 里是关键字，故写作 `«prefix»`。 -/
def «prefix» {Line : Type} (mother : List Line) (at_seq : Nat) : Except Code (List Line) :=
  if at_seq < mother.length then .ok (mother.take (at_seq + 1)) else .error .InvalidArgs

/-- 分叉点过了母序列的尾，拒绝，而不是截到末尾。 -/
theorem a_fork_past_the_tail_is_refused {Line : Type} (mother : List Line) (at_seq : Nat)
    (past : mother.length ≤ at_seq) : «prefix» mother at_seq = .error .InvalidArgs := by
  unfold «prefix»
  rw [ite_eq_right (by omega)]

/-- 分叉前缀恰好是母序列的头 `at_seq + 1` 行，一行不多一行不少。 -/
theorem a_fork_is_the_mothers_first_lines {Line : Type} (mother : List Line) (at_seq : Nat)
    (lines : List Line) (forked : «prefix» mother at_seq = .ok lines) :
    lines = mother.take (at_seq + 1) ∧ lines.length = at_seq + 1 := by
  unfold «prefix» at forked
  split at forked
  · injection forked with taken
    subst taken
    exact ⟨rfl, by simp; omega⟩
  · cases forked

/-- 母 run 的一行，按切点关心的样子：一条带 `calls` 条调用的 `model_returned`、一条 `tool_result`、其余任何一行。 -/
inductive Line where
  | returned (calls : Nat)
  | result
  | other
  deriving DecidableEq, Repr

/-- 读完这些行之后，开着的一波还欠几条 `tool_result`（`fold_run` 的 `Wave::expect`）。 -/
def owed (lines : List Line) : Nat :=
  lines.foldl (fun waiting line => match line with
    | .returned calls => calls
    | .result => waiting - 1
    | .other => waiting) 0

/-- 第 `i` 行之后能不能切：读到这一行为止，没有一条调用还在等它的结果。 -/
def safeAt (lines : List Line) (i : Nat) : Bool :=
  owed (lines.take (i + 1)) == 0

/-- 从 `n` 往回找第一个能切的位置。 -/
def retreat (safe : Nat → Bool) : Nat → Option Nat
  | 0 => if safe 0 then some 0 else none
  | n + 1 => if safe (n + 1) then some (n + 1) else retreat safe n

/-- `fold_run` 的切点：要求在 `at_seq` 切，实际用到的是不晚于它的最后一个安全点，并如实交回（`Inherited::at`）。 -/
def cut (lines : List Line) (at_seq : Nat) : Option Nat :=
  retreat (safeAt lines) at_seq

/-- 往回找到的位置能切，且不晚于要求的那一行。 -/
theorem retreat_lands_on_a_safe_line (safe : Nat → Bool) :
    ∀ (n i : Nat), retreat safe n = some i → safe i = true ∧ i ≤ n
  | 0, i, found => by
    cases here : safe 0 with
    | true =>
      simp [retreat, here] at found
      subst found
      exact ⟨here, Nat.le_refl 0⟩
    | false => simp [retreat, here] at found
  | n + 1, i, found => by
    cases here : safe (n + 1) with
    | true =>
      simp [retreat, here] at found
      subst found
      exact ⟨here, Nat.le_refl _⟩
    | false =>
      simp [retreat, here] at found
      have earlier := retreat_lands_on_a_safe_line safe n i found
      exact ⟨earlier.1, by omega⟩

/-- 往回只退到必须退的地方：找到的位置与要求的那一行之间，没有一个能切的位置被跳过。 -/
theorem retreat_goes_back_no_further_than_needed (safe : Nat → Bool) :
    ∀ (n i : Nat), retreat safe n = some i → ∀ j, i < j → j ≤ n → safe j = false
  | 0, _, _, _, after, within => by omega
  | n + 1, i, found, j, after, within => by
    cases here : safe (n + 1) with
    | true =>
      simp [retreat, here] at found
      omega
    | false =>
      simp [retreat, here] at found
      by_cases top : j = n + 1
      · subst top
        exact here
      · exact retreat_goes_back_no_further_than_needed safe n i found j after (by omega)

/-- 要求的那一行本身能切，就不退。 -/
theorem a_safe_cut_is_kept (safe : Nat → Bool) (n : Nat) (here : safe n = true) :
    retreat safe n = some n := by
  cases n with
  | zero => simp [retreat, here]
  | succ m => simp [retreat, here]

/-- **分支从不继承半个交换。** 切点之前的每一波都收齐了它的结果：一条有调用没有答案的助手消息，任何 provider 都拒。 -/
theorem the_cut_never_splits_a_wave (lines : List Line) (at_seq i : Nat)
    (found : cut lines at_seq = some i) :
    owed (lines.take (i + 1)) = 0 ∧ i ≤ at_seq := by
  have landed := retreat_lands_on_a_safe_line (safeAt lines) at_seq i found
  exact ⟨by simpa [safeAt] using landed.1, landed.2⟩

/-- 切点落在一波之内时往回退到这一波之前，并如实交回实际用到的那一行。 -/
example : cut [.other, .returned 2, .result, .result, .returned 1] 4 = some 3 := by decide

/-- 一行开场之后立刻切，切在开场那一行。 -/
example : cut [.other, .returned 1, .result] 0 = some 0 := by decide

/-- 开篇的写法（`kernel::event::record::Opening`，runtime 再导出）。 -/
inductive Opening where
  | FromJob
  | Inherited
  | WithPerson
  deriving DecidableEq, Repr

/-! D14 开篇的写法记在 `run_started` 上，`FromJob` 的分支仍改写第一条消息

**决定**：`Opening` 搬进 kernel，`run_started.opening` 记它；`fork::fold_run` 照记下的写法重建母 run 的第一条消息，只有 `FromJob` 改写成 `Inherited`（§8-58）。

**理由**：provider 的前缀缓存只认逐字节相同的前缀，分叉的第一个请求要以母 run 最后一个请求的字节开头；重建者唯一的材料是账本，所以写法必须在账本上。值的定义只能有一处：runtime 依赖 kernel，载荷住 kernel，所以枚举搬过去，runtime 再导出它，调用方的路径一处不改。`FromJob` 那一句指向母 run 前缀里的 JOB.md，分支没有那份文本，照抄是一条指向空处的话；把文本抄进分支的 run 段会让 run 段与母 run 不同，缓存一样不命中。

**被否**：①在 kernel 另立一个两值的记录类型、runtime 保留自己的三值枚举——同一件事两个定义，两边的词迟早不一致；②按 `goal` 是否为空推断写法——那是 `city::write_brief` 的规则在分叉里的第二份；③`FromJob` 的分支也照抄那一句——分支读到一条找不到对象的指示；④把母 run 的 JOB.md 抄进分支的 run 段——缓存在 system 段就不命中，还多一份没人派给分支的任务。

**重开参数**：分支的前缀能带上母 run 的 run 段（例如分支沿用母 run 的 brief 而不另写一份）时，`FromJob` 也照抄。
-/

/-- `fork::rebuilt`：照 `run_started.opening` 重建母 run 第一条消息的写法（§8-58 的表）。第二个参数是 `run_started.job` 在不在场，只用来读加键之前的行。 -/
def rebuilt : Option Opening → Bool → Opening
  | some .FromJob, _ => .Inherited
  | some .Inherited, _ => .Inherited
  | some .WithPerson, _ => .WithPerson
  | none, true => .Inherited
  | none, false => .WithPerson

/-- 只有 `FromJob` 被改写；另外两种照母 run 发出的写法重建，provider 的前缀缓存从第一条消息起命中。 -/
theorem only_from_job_is_rewritten (opening : Opening) (job : Bool) :
    rebuilt (some opening) job ≠ opening → opening = .FromJob := by
  cases opening <;> cases job <;> decide

/-- 分支永远不说「任务在上面的 JOB.md 里」：它的前缀带的是它自己的 brief，不是母 run 的那一份。 -/
theorem a_branch_never_points_at_a_job_file_it_was_not_given (opening : Option Opening)
    (job : Bool) : rebuilt opening job ≠ .FromJob := by
  cases opening with
  | none => cases job <;> decide
  | some spoken => cases spoken <;> cases job <;> decide

end Runtime.Fork
