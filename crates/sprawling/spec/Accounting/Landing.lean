-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 记账线程只做账：落 run 的重活不在记账线程上

规定 `crates/accounting/src/worker/driving/flight.rs`（`serve_flight`）、`crates/accounting/src/worker/dispatching/running.rs`（`land`）与 `crates/sprawling/src/serving/folding.rs`（`spawn_folding`）在落 run 的重活（transcript、检查点、合并）移出记账线程之后必须守住的性质。它是 `crates/sprawling/spec/Accounting/Worker.lean` §8-42-4 的模型：那一节说记账线程有三张嘴、按什么次序服务；这里说一张嘴上能做什么。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

落 run 分两半。重活在这个 run 的 lane 上做完：写 transcript、立最后一道检查点、把节点的工作合进干线，结果是一组已经算好的记录草稿。账的一半在记账线程上做：追加这组草稿，然后把 run 记成已冻结。于是记账线程上的每一步都只是追加，一次落 run 与一次 relay 请求的区别只在追加几条。

模型的性质分四组，每组对任意的消息序列成立：
* **relay 请求不排在一次落 run 的重活后面**——一条 relay 请求落在账本的哪个位置、在它之前记账线程做了什么，与任何一次落 run 的重活多重无关（`the_landing_work_never_delays_a_relay`、`a_relay_waits_only_for_appends`）；
* **只有一个追加者，记录只有一个全局次序**——账本恰是开始时的账本接上每条消息的记录，按服务次序，只增不减（`the_ledger_is_the_served_records_in_order`、`the_ledger_only_grows`）；
* **冻结之前落 run 的记录已在账上**——一个 run 一旦被看见已冻结，它落 run 的每一条记录都已在账本里（`a_frozen_run_has_its_landing_on_the_ledger`）；
* **视图广播按帧合并，不丢一条，正在看的 session 先走，谁也不饿**——一帧取走全部未发的记录，正在看的 session 的记录排在前面，一帧之后再没有未发的记录（`a_frame_carries_every_pending_record`、`a_frame_serves_the_watched_session_first`、`after_a_frame_nothing_waits`）。
-/

namespace Sprawling.Accounting.Landing

/-- 一个 run。 -/
abbrev Run := Nat
/-- 一个 session：User 在页面上看的是其中一个。 -/
abbrev Session := Nat

/-- 一条记录草稿；seq 与 prev 由追加时给出，所以模型里它的身份就是它在账本里的位置。 -/
structure Record where
  run : Run
  session : Session
  deriving DecidableEq

/-- 送进记账线程那一条队列的消息。 -/
inductive Wake where
  /-- lane 的一条 relay 请求。 -/
  | relay (draft : Record)
  /-- 一个 run 回家：`drafts` 是 lane 上已经做完的重活算出的落 run 记录，`heavy` 是那份重活的量。 -/
  | home (run : Run) (drafts : List Record) (heavy : Nat)

/-- 这条消息要追加的记录。 -/
def Wake.records : Wake → List Record
  | .relay d => [d]
  | .home _ ds _ => ds

/-- 记账线程持有的：账本，与哪些 run 已冻结。 -/
structure Accounts where
  ledger : List Record
  /-- 记账线程记下的「这个 run 已落定」：代码里是 `serve_flight` 取走它的 `Arrival`、`land` 写完它落 run 的记录之后的状态，
  不是 `run_frozen` 这一条记录。`run_frozen` 由 lane 在驱动结束时经 relay 写下，在模型里是 `Wake.relay` 的一条，
  所以它排在落 run 的记录之前；把它挪到落 run 记录之后要改账本的次序，属于改得了事件的那一版。 -/
  frozen : Run → Bool

/-! D37 落 run 的切线：读盘与写盘的重活在 lane 上，改写折叠的步骤留在记账线程上

**决定**：`land` 的每一步按它读写的状态分成两半。只读这个 run 的 `Flown` 与盘、不碰记账线程任何折叠的步骤在 lane 上做完，然后才发 `Wake::Home`：transcript 的物化（往 lanes 共用的 CAS 里放字节、在房间旁写文件，诊断行经 lane 的 `Notes` 写出）与检查点清扫（`Checkpoint::wave_post`，把这次驱动删掉的文件对着它的第一道检查点比出来），结果随 `Flown::Model` 的 `swept` 回家。记账线程只把清扫结果折成记录草稿追加，再做改写折叠的步骤：房间队列与 backlog 成员的归还、三张 desk 的结算（信号、清扫行、计划认领）、评审请求、worktree 租约的归还、`conclude`（待答项、往下派活、接班、答复派活者）、`answer_knocks`、认领的归还与 `advance_pursuits`。今天的 `land` 里没有合并这一步；合并落地时（roadmap TF4）它属于 lane 那一半。三个平台相同：搬动的只是哪条 `std::thread` 调用同一个函数。

**理由**：transcript 的物化与清扫的 diff 是 `land` 里仅有的两步与 run 的大小成正比的盘上工作，它们在记账线程上时，期间到达的 relay 请求都排在它们后面（`the_landing_work_never_delays_a_relay` 证明搬走之后 relay 在账本里的位置与它们多重无关）。留下的步骤每一步都改写一个只有记账线程持有的折叠，搬走它们就要给折叠第二个写者。租约的归还留下，是因为它必须排在清扫之后（§8-113），且只是解开一个 worktree 的锁。

**被否**：①整个 `land` 搬进 lane：房间队列、认领簿与派活表就有了第二个写者；②把清扫留在记账线程、只搬 transcript：清扫的 diff 随 run 写过的文件数增长，是两步里更重的那一步。

**重开参数**：吞吐台（roadmap TP1）在 relay 排队的 p99 里仍量到落 run 的一段时，逐步给留下的步骤计时，再议哪一步搬走。
-/

/-- 服务一条消息：追加它的记录；回家的 run 在它的记录追加之后才记成已冻结，两者是同一步。 -/
def serve (a : Accounts) : Wake → Accounts
  | .relay d => { a with ledger := a.ledger ++ [d] }
  | .home r ds _ => { ledger := a.ledger ++ ds, frozen := fun u => u == r || a.frozen u }

/-- 按到达次序服务一串消息。 -/
def serveAll (a : Accounts) : List Wake → Accounts
  | [] => a
  | w :: ws => serveAll (serve a w) ws

theorem serveAll_append (a : Accounts) (xs ys : List Wake) :
    serveAll a (xs ++ ys) = serveAll (serveAll a xs) ys := by
  induction xs generalizing a with
  | nil => rfl
  | cons x xs ih => exact ih (serve a x)

/-! ## 只有一个追加者，记录只有一个全局次序 -/

theorem the_ledger_is_the_served_records_in_order (a : Accounts) (ws : List Wake) :
    (serveAll a ws).ledger = a.ledger ++ ws.flatMap Wake.records := by
  induction ws generalizing a with
  | nil => simp [serveAll]
  | cons w ws ih =>
    simp only [serveAll, ih, List.flatMap_cons]
    cases w <;> simp [serve, Wake.records]

theorem the_ledger_only_grows (a : Accounts) (ws : List Wake) :
    a.ledger <+: (serveAll a ws).ledger := by
  rw [the_ledger_is_the_served_records_in_order]
  exact List.prefix_append _ _

/-! ## relay 请求不排在一次落 run 的重活后面 -/

/-- 把每次回家的重活量换成别的值，记录不变。 -/
def Wake.reweigh (f : Run → Nat) : Wake → Wake
  | .relay d => .relay d
  | .home r ds _ => .home r ds (f r)

/-- 记账线程做的一切与落 run 的重活多重无关：重活在 lane 上，记账线程上没有它。 -/
theorem the_landing_work_never_delays_a_relay (a : Accounts) (ws : List Wake) (f : Run → Nat) :
    serveAll a (ws.map (Wake.reweigh f)) = serveAll a ws := by
  induction ws generalizing a with
  | nil => rfl
  | cons w ws ih =>
    simp only [List.map_cons, serveAll]
    cases w <;> simp [Wake.reweigh, serve, ih]

/-- 一条 relay 请求之前，记账线程只做了排在它前面的消息的追加：它落在开始时的长度加上前面每条消息的记录条数处。 -/
theorem a_relay_waits_only_for_appends (a : Accounts) (ws : List Wake) (d : Record) :
    (serveAll a (ws ++ [.relay d])).ledger
      = a.ledger ++ ws.flatMap Wake.records ++ [d] := by
  rw [the_ledger_is_the_served_records_in_order, List.flatMap_append]
  simp [Wake.records]

/-! ## 冻结之前落 run 的记录已在账上 -/

theorem a_frozen_run_has_its_landing_on_the_ledger (a : Accounts) (ws : List Wake) (r : Run)
    (thawed : a.frozen r = false) (seen : (serveAll a ws).frozen r = true) :
    ∃ ds h, Wake.home r ds h ∈ ws ∧ ∀ d ∈ ds, d ∈ (serveAll a ws).ledger := by
  induction ws generalizing a with
  | nil => simp_all [serveAll]
  | cons w ws ih =>
    simp only [serveAll] at seen ⊢
    cases w with
    | relay d =>
      obtain ⟨ds, h, mem, onLedger⟩ := ih (serve a (.relay d)) (by simpa [serve] using thawed) seen
      exact ⟨ds, h, List.mem_cons_of_mem _ mem, onLedger⟩
    | home u ds h =>
      by_cases same : u = r
      · subst same
        refine ⟨ds, h, List.mem_cons_self, fun d hd => ?_⟩
        have grows := the_ledger_only_grows (serve a (.home u ds h)) ws
        obtain ⟨rest, eq⟩ := grows
        rw [← eq]
        simp [serve, hd]
      · have still : (serve a (.home u ds h)).frozen r = false := by
          simp [serve, Ne.symm same, thawed]
        obtain ⟨ds', h', mem, onLedger⟩ := ih (serve a (.home u ds h)) still seen
        exact ⟨ds', h', List.mem_cons_of_mem _ mem, onLedger⟩

/-! ## 视图广播按帧合并 -/

/-- 一帧：未发的记录全部取走，正在看的 session 的在前，其余的按账本次序在后。 -/
def frame (watched : Session) (pending : List Record) : List Record :=
  pending.filter (fun d => d.session == watched) ++ pending.filter (fun d => !(d.session == watched))

/-- 一帧不丢也不添一条记录。 -/
theorem a_frame_carries_every_pending_record (watched : Session) (pending : List Record)
    (d : Record) : d ∈ frame watched pending ↔ d ∈ pending := by
  simp only [frame, List.mem_append, List.mem_filter]
  cases d.session == watched <;> simp

theorem a_frame_keeps_the_count (watched : Session) (pending : List Record) :
    (frame watched pending).length = pending.length := by
  induction pending with
  | nil => rfl
  | cons d ds ih =>
    simp only [frame, List.length_append, List.filter_cons, List.length_cons] at ih ⊢
    cases d.session == watched <;> simp <;> omega

/-- 一帧里正在看的 session 的记录排在任何别的 session 的记录之前。 -/
theorem a_frame_serves_the_watched_session_first (watched : Session) (pending : List Record) :
    ∃ first rest, frame watched pending = first ++ rest ∧
      (∀ d ∈ first, d.session = watched) ∧ (∀ d ∈ rest, d.session ≠ watched) := by
  refine ⟨pending.filter (fun d => d.session == watched),
    pending.filter (fun d => !(d.session == watched)), rfl, ?_, ?_⟩
  · intro d hd
    simpa using (List.mem_filter.mp hd).2
  · intro d hd
    simpa using (List.mem_filter.mp hd).2

/-- 广播器：账本里前 `sent` 条已经发出。 -/
structure Broadcast where
  sent : Nat

/-- 发一帧：发出账本里还没发的全部记录。 -/
def Broadcast.send (_b : Broadcast) (ledger : List Record) : Broadcast := ⟨ledger.length⟩

/-- 一帧之后再没有未发的记录：不论正在看哪个 session，别的 session 至多等一帧。 -/
theorem after_a_frame_nothing_waits (b : Broadcast) (ledger : List Record) :
    ledger.drop (b.send ledger).sent = [] := by
  simp [Broadcast.send]

end Sprawling.Accounting.Landing
