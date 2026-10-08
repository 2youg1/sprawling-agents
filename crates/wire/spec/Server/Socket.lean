-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::server::socket：一帧一次写出

规定 `server::socket`（`crates/wire/src/server/socket.rs`）把订阅到的记录怎样按帧写给一个会话。本文件是 `crates/wire/Spec.lean` 的一个分部；下面一节保留它在 wire 规格里的标签 §8-47h，别处引作 `crates/wire/Spec.lean §8-47h`，决定引作 `wire D45`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

### 8-47h 视图广播按帧合并（形状 4 adapter 里的一个纯函数）

```rust
// wire::server::socket
/// 一条会话一次从订阅里取到的东西：一条记录，或订阅报的一次跳过。
pub(crate) enum Arrival<R> { Record(Seq, R), Skipped }
/// 一条会话对它说的话：一段要补拉的区间，或一条记录。
pub(crate) enum Said<R> { Lagged(Lagged), Record(R) }
/// 一帧最多取多少条：取满就先写出这一帧，`select!` 的别的臂不被一阵长的突发饿住。
pub(crate) const FRAME_RECORDS_MAX: usize = 256;
/// 一帧：把已经到了的全部 `Arrival` 依次过 `Stream`，交回要说的话与之后的 `Stream`。纯函数。
pub(crate) fn framed<R>(stream: Stream, phase: SessionState, arrivals: Vec<Arrival<R>>) -> (Vec<Said<R>>, Stream);
```

一个会话的事件臂醒来时，订阅里往往已经排着一整批记录：折叠线程每折完一批就把这一批逐条广播（`crates/sprawling/src/serving/folding.rs`）。今天的事件臂每条记录 `send` 一次，`send` 是 `feed` 加 `flush`，于是一批 N 条就是 N 次刷到套接字。改成按帧：醒来取到第一条之后，用 `try_recv` 把已经到了的取完（至多 `FRAME_RECORDS_MAX` 条），交 `framed` 算出要说的话，逐条 `feed`，最后 `flush` 一次。线上的形状一字不变：每条记录仍是它自己的那一个 `Event` 帧（§8-47），每段跳过仍是一个 `Lagged` 帧（§8-41），合并的只是刷写。

模型证明的三条性质都对任意的到达序列成立：

* **按帧切与逐条处理说出的话相同**（`framing_is_invisible`）：一条到达序列不论在哪里切成帧，会话说出的话与之后的 `Stream` 都与一条一条处理时相同，所以 `FRAME_RECORDS_MAX` 取多大、`try_recv` 在哪里取空，都不改变会话对页面说的任何一句。
* **每条记录按 seq 次序到达、不重复**（`said_in_seq_order`、`said_at_most_once`、`every_record_reaches`）：活的会话说出的记录恰是订阅交来的记录、次序不变；广播按账本次序发（一城一条全局 seq，§8-47），所以说出的记录 seq 严格递增，没有一条说两次；没有跳过时，账本的每一条都说到。
* **没说到的记录都被一段 `Lagged` 点了名**（`every_seq_up_to_the_last_is_told`）：一个已经说到 seq `d` 的会话，之后说到的最后一条是 `e` 时，`d` 与 `e` 之间的每个 seq 不是作为记录说到，就落在一段说出的 `Lagged` 里，页面可以用 `HistoryRange` 把它要回来（§8-41）。

模型里的 seq 是 `Nat`；Rust 的 `decide_lag` 用 `checked_add` 与 `checked_sub`，`u64` 溢出的那一格是产生过 `next` 的账本到不了的（§8-41），模型不写它。

**派生的 Rust 检查**：`wire::server::socket::tests` 的 `proptest` 生成一条 seq 严格递增的账本（相邻两条之间的间隔取 1..4，模拟拼不出帧的记录），按随机的掩码把一些记录换成跳过（连续被跳过的几条只留一个 `Skipped`，与 `broadcast` 报 `Lagged(n)` 的方式相同），再随机切成帧。每一种切法都断言：按帧算出的话与整条一帧算出的话相同；说出的记录恰是没被跳过的那些，seq 严格递增；第一条说出的记录与最后一条之间的每个 seq 都被一条记录或一段 `Lagged` 覆盖。

**三个平台相同**：`try_recv`、`feed`、`flush` 是 tokio 与 axum 的跨平台接口；Windows 上一次 `flush` 对应一次 `WSASend`，Linux 与 macOS 上对应一次 `send`，合并省下的是这几次系统调用，三个平台都一样。
-/

/-! D45 视图广播按帧合并只合并刷写，不改帧的形状，也不按 session 重排

**决定**：一帧是一个会话一次醒来时已经到了的记录；它们按到达次序逐条成为各自的 `Event` 帧，`flush` 一次。

**理由**：合并刷写不改线上的任何一个字节，页面一行不用改；`framing_is_invisible` 证明它对页面不可见。**被否**：①把一帧的记录装进一个新的批量帧——那是线上形状的改动（`WIRE_V`、`wire.ts`、`Door.lean`），而且省下的只是每条记录十几个字节的信封；②正在看的 session 的记录排在一帧的前面——服务端不知道页面在看哪个 session（没有一个帧携带它），而且一旦按 session 重排，全城一条的 seq 在一个会话里就不再递增，`decide_lag` 会把重排读成断口、发出假的 `Lagged`，页面的折叠也按 seq 丢弃比已见更旧的记录。重开的条件：页面经一个帧告诉服务端它在看哪个 session，并且 `Stream` 与页面的折叠都改成按 session 判断口与次序。
-/

namespace Wire.Server.Socket

/-- 一条会话一次从订阅里取到的东西。 -/
inductive Arrival where
  | record (seq : Nat)
  | skipped

/-- 会话对页面说的一句：一段要补拉的区间（两端都含），或一条记录。 -/
inductive Said where
  | lagged (lo hi : Nat)
  | record (seq : Nat)

/-- `reception::Stream`：上一条已发的记录，与此后是否欠一段。 -/
structure Stream where
  delivered : Option Nat
  owed : Bool

def opening : Stream := ⟨none, false⟩

/-- 欠的那一段从哪里起：已发的下一条，或账本的第一条（`Seq::FIRST` 是 0）。 -/
def lagFrom : Option Nat → Nat
  | some d => d + 1
  | none => 0

/-- `reception::decide_lag`。 -/
def decideLag (delivered : Option Nat) (next : Nat) : List Said :=
  if 1 ≤ next ∧ lagFrom delivered ≤ next - 1 then [.lagged (lagFrom delivered) (next - 1)] else []

/-- `Stream::before` 加上记录本身：先说欠的那一段，再说这条记录。 -/
def before (s : Stream) (next : Nat) : List Said × Stream :=
  let lag := match s.delivered, s.owed with
    | none, false => []
    | d, _ => decideLag d next
  (lag ++ [.record next], ⟨some next, false⟩)

/-- 一次到达：会话还没活（没欢迎）时什么也不说。 -/
def step (live : Bool) (s : Stream) : Arrival → List Said × Stream
  | .record n => if live then before s n else ([], s)
  | .skipped => ([], if live then ⟨s.delivered, true⟩ else s)

/-- 一帧：依次处理已经到了的全部到达。 -/
def framed (live : Bool) : Stream → List Arrival → List Said × Stream
  | s, [] => ([], s)
  | s, a :: as =>
    let first := step live s a
    let rest := framed live first.2 as
    (first.1 ++ rest.1, rest.2)

/-- 一条到达序列不论在哪里切成两帧，说出的话与之后的 `Stream` 都与一帧相同；
切成任意多帧时对每一刀用一次。 -/
theorem framing_is_invisible (live : Bool) (s : Stream) (xs ys : List Arrival) :
    framed live s (xs ++ ys) =
      ((framed live s xs).1 ++ (framed live (framed live s xs).2 ys).1,
        (framed live (framed live s xs).2 ys).2) := by
  induction xs generalizing s with
  | nil => simp [framed]
  | cons a as ih => simp [framed, ih, List.append_assoc]

/-- 说出的记录的 seq，按说出的次序。 -/
def records : List Said → List Nat
  | [] => []
  | .record n :: rest => n :: records rest
  | .lagged _ _ :: rest => records rest

/-- 到达里的记录的 seq，按到达的次序。 -/
def arrived : List Arrival → List Nat
  | [] => []
  | .record n :: rest => n :: arrived rest
  | .skipped :: rest => arrived rest

theorem records_append (xs ys : List Said) : records (xs ++ ys) = records xs ++ records ys := by
  induction xs with
  | nil => rfl
  | cons x xs ih => cases x <;> simp [records, ih]

theorem records_decideLag (d : Option Nat) (n : Nat) : records (decideLag d n) = [] := by
  unfold decideLag
  split <;> rfl

theorem records_before (s : Stream) (n : Nat) : records (before s n).1 = [n] := by
  unfold before
  split <;> simp [records_append, records_decideLag, records]

/-- 活的会话说出的记录恰是订阅交来的记录，次序不变。 -/
theorem records_said (s : Stream) (as : List Arrival) :
    records (framed true s as).1 = arrived as := by
  induction as generalizing s with
  | nil => rfl
  | cons a as ih =>
    cases a with
    | record n => simp [framed, step, records_append, records_before, ih, arrived]
    | skipped => simp [framed, step, ih, arrived]

/-- 还没活的会话什么也不说。 -/
theorem nothing_said_before_welcome (s : Stream) (as : List Arrival) :
    (framed false s as).1 = [] := by
  induction as generalizing s with
  | nil => rfl
  | cons a as ih => cases a <;> simp [framed, step, ih]

/-- 订阅按账本次序交来记录时，说出的记录 seq 严格递增。 -/
theorem said_in_seq_order (s : Stream) (as : List Arrival)
    (ledgerOrder : (arrived as).Pairwise (· < ·)) :
    (records (framed true s as).1).Pairwise (· < ·) := by
  rw [records_said]
  exact ledgerOrder

/-- 没有一条记录说两次。 -/
theorem said_at_most_once (s : Stream) (as : List Arrival)
    (ledgerOrder : (arrived as).Pairwise (· < ·)) :
    (records (framed true s as).1).Nodup :=
  (said_in_seq_order s as ledgerOrder).imp Nat.ne_of_lt

theorem arrived_map (ledger : List Nat) : arrived (ledger.map .record) = ledger := by
  induction ledger with
  | nil => rfl
  | cons n ns ih => simp [arrived, ih]

/-- 没有跳过时，账本的每一条都说到，按账本的次序。 -/
theorem every_record_reaches (s : Stream) (ledger : List Nat) :
    records (framed true s (ledger.map .record)).1 = ledger := by
  rw [records_said, arrived_map]

/-- 一句话说到了 seq `k`：它就是那条记录，或 `k` 落在它的区间里。 -/
def Said.covers : Said → Nat → Prop
  | .record n, k => n = k
  | .lagged lo hi, k => lo ≤ k ∧ k ≤ hi

/-- 说出的话里有一句说到了 `k`。 -/
def Told (out : List Said) (k : Nat) : Prop := ∃ x ∈ out, x.covers k

theorem told_append_left {xs : List Said} {k : Nat} (ys : List Said) (h : Told xs k) :
    Told (xs ++ ys) k := by
  obtain ⟨x, hx, hc⟩ := h
  exact ⟨x, List.mem_append_left ys hx, hc⟩

theorem told_append_right (xs : List Said) {ys : List Said} {k : Nat} (h : Told ys k) :
    Told (xs ++ ys) k := by
  obtain ⟨x, hx, hc⟩ := h
  exact ⟨x, List.mem_append_right xs hx, hc⟩

/-- 一个已经说到 `d` 的会话说下一条记录 `n` 时，`d` 之后、`n` 及以前的每个 seq 都说到了。 -/
theorem before_tells (s : Stream) (d n k : Nat) (hd : s.delivered = some d)
    (lo : d < k) (hi : k ≤ n) : Told (before s n).1 k := by
  unfold before
  rcases s with ⟨delivered, owed⟩
  simp only at hd
  subst hd
  simp only
  by_cases same : k = n
  · exact told_append_right _ ⟨.record n, List.mem_singleton.mpr rfl, same.symm⟩
  · apply told_append_left
    have gap : decideLag (some d) n = [.lagged (d + 1) (n - 1)] := by
      simp only [decideLag, lagFrom]
      exact if_pos (by omega)
    exact ⟨.lagged (d + 1) (n - 1), by simp [gap], by simp only [Said.covers]; omega⟩

/-- 一个已经说到 `d` 的会话，之后说到的最后一条是 `e` 时，`d` 与 `e` 之间的每个 seq
不是作为记录说到，就落在一段说出的 `Lagged` 里。 -/
theorem every_seq_up_to_the_last_is_told (as : List Arrival) :
    ∀ (s : Stream) (d e k : Nat), s.delivered = some d →
      (framed true s as).2.delivered = some e → d < k → k ≤ e →
      Told (framed true s as).1 k := by
  induction as with
  | nil =>
    intro s d e k hd he lo hi
    simp only [framed] at he
    rw [hd] at he
    cases he
    omega
  | cons a as ih =>
    intro s d e k hd he lo hi
    cases a with
    | skipped =>
      simp only [framed, step, ite_true] at he ⊢
      exact told_append_right _ (ih _ d e k hd he lo hi)
    | record n =>
      simp only [framed, step, ite_true] at he ⊢
      by_cases near : k ≤ n
      · exact told_append_left _ (before_tells s d n k hd lo near)
      · have moved : (before s n).2.delivered = some n := rfl
        exact told_append_right _ (ih _ n e k moved he (by omega) hi)

end Wire.Server.Socket
