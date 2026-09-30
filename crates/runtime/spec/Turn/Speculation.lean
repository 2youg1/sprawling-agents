-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 模型还在生成时起跑只读工具

规定 `crates/gateway/src/anthropic/stream.rs` 里的提前交出（`completed_call`），以及运行时回合循环据一条提前交出的调用行事时守的规则：`crates/runtime/src/turn/speculation.rs` 起跑读调用并按位置缓存结果，`crates/runtime/src/turn/wave.rs` 把它们记进账本（gateway-SPEC.md 第 8 节，runtime-SPEC.md 8-3）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

每条工具调用的 `content_block_stop` 一到，解码器就交出这条调用。运行时可以在回答结算之前起跑它，并把它的返回值存进一个按调用在回答中的位置做键的缓存。回答结算之前 Ledger 不追加任何记录；结算之后，调用按模型发出的次序入账，有缓存结果的取缓存，没有的照常执行。

两条性质，各一组定理：

* **推测结果不是事件**——无论回答怎样结束，有缓存与没有缓存时回答产生的 Ledger 相同；被截断或被取消的回答一条工具调用也不追加，所以它的缓存整份丢弃；
* **Ledger 顺序即串行顺序**——缓存只为回答中第一条写调用之前的只读调用填入时，结算后的 Ledger 与串行执行追加的 Ledger 逐条相同。

推测之所以停在第一条写调用，原因在第二条性质：提前起跑的读看到的是回答里的调用执行之前的世界，而同一回答里排在它前面的写会改变那个世界（`speculating_past_a_write_changes_the_ledger`）。
-/

namespace Runtime.Turn.Speculation

/-- 执行一条工具调用会不会改变世界。 -/
inductive Kind where
  | read
  | write
  deriving Repr, DecidableEq

/-- 解码器交出的一条工具调用，以调用 id 识别。 -/
structure Call where
  id : Nat
  kind : Kind
  deriving Repr, DecidableEq

/-- 工具调用读写的状态，抽象成一个版本号。 -/
abbrev World := Nat

/-- 执行一条调用返回什么、留下什么。只读调用让世界保持原样，所以提前起跑它是安全的。 -/
structure Tools where
  answer : Call → World → Nat
  effect : Call → World → World
  readsOnly : ∀ c w, c.kind = .read → effect c w = w

/-- 关于一条工具调用的 Ledger 记录。 -/
inductive Event where
  | toolCalled (id : Nat)
  | toolResult (id : Nat) (result : Nat)
  deriving Repr, DecidableEq

/-- 模型的回答怎样结束。 -/
inductive Ending where
  | settled
  | truncated
  | cancelled
  deriving Repr, DecidableEq

/-- 串行执行：按发出次序逐条执行，每条看到的是它之前那些调用留下的世界。 -/
def serial (t : Tools) : World → List Call → List Event
  | _, [] => []
  | w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (t.answer c w) :: serial t (t.effect c w) rest

/-- 带缓存的结算执行：每个位置上的调用，该位置有缓存结果就取它，没有就照常执行。 -/
def withCache (t : Tools) : List (Option Nat) → World → List Call → List Event
  | _, _, [] => []
  | cached :: more, w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (cached.getD (t.answer c w)) ::
      withCache t more (t.effect c w) rest
  | [], w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (t.answer c w) :: withCache t [] (t.effect c w) rest

def Call.isRead (c : Call) : Bool := c.kind == .read

/-- 推测算出什么：第一条写调用之前的只读调用，每条都对着回答开始时的世界执行。它们按什么次序完成无关紧要，因为缓存按位置做键。 -/
def speculated (t : Tools) (w : World) (calls : List Call) : List (Option Nat) :=
  (calls.takeWhile Call.isRead).map (some <| t.answer · w)

/-- 一个回答追加的记录。被切断的回答不追加任何工具调用：失败属于那次模型调用，与其他模型失败记在同一处。 -/
def ledger (t : Tools) (w : World) (ending : Ending) (calls : List Call)
    (cache : List (Option Nat)) : List Event :=
  match ending with
  | .settled => withCache t cache w calls
  | .truncated | .cancelled => []

/-! ## Ledger 顺序即串行顺序 -/

theorem no_cache_is_serial (t : Tools) (w : World) (calls : List Call) :
    withCache t [] w calls = serial t w calls := by
  induction calls generalizing w with
  | nil => rfl
  | cons c rest ih => simp [withCache, serial, ih]

theorem speculation_keeps_serial_order (t : Tools) (w : World) (calls : List Call) :
    withCache t (speculated t w calls) w calls = serial t w calls := by
  induction calls with
  | nil => rfl
  | cons c rest ih =>
    cases hk : c.kind with
    | read =>
      have hr : c.isRead = true := by simp [Call.isRead, hk]
      simp only [speculated, List.takeWhile_cons, hr, List.map_cons, ite_true] at ih ⊢
      simp [withCache, serial, t.readsOnly c w hk, ih]
    | write =>
      have hr : c.isRead = false := by simp [Call.isRead, hk]
      simp [speculated, hr, no_cache_is_serial]

/-! ## 推测结果不是事件 -/

theorem speculation_is_not_an_event (t : Tools) (w : World) (ending : Ending)
    (calls : List Call) :
    ledger t w ending calls (speculated t w calls) = ledger t w ending calls [] := by
  cases ending with
  | settled => simp [ledger, speculation_keeps_serial_order, no_cache_is_serial]
  | truncated | cancelled => rfl

theorem a_cut_answer_discards_its_cache (t : Tools) (w : World) (ending : Ending)
    (calls : List Call) (cache : List (Option Nat)) (cut : ending ≠ .settled) :
    ledger t w ending calls cache = [] := by
  cases ending with
  | settled => contradiction
  | truncated | cancelled => rfl

/-! ## 推测为什么停在第一条写调用 -/

/-- 一个写就把版本加一、读就报出版本的世界。 -/
def versioned : Tools where
  answer c w := match c.kind with
    | .read => w
    | .write => 0
  effect c w := match c.kind with
    | .read => w
    | .write => w + 1
  readsOnly c w h := by simp [h]

/-- 落选的设计：每条读都提前起跑，写之后的读也不例外。那条读报出的是写之前的世界。 -/
def everyReadEarly (t : Tools) (w : World) (calls : List Call) : List (Option Nat) :=
  calls.map fun c => if c.isRead then some (t.answer c w) else none

theorem speculating_past_a_write_changes_the_ledger :
    let calls := [⟨1, .write⟩, ⟨2, .read⟩]
    withCache versioned (everyReadEarly versioned 0 calls) 0 calls ≠
      serial versioned 0 calls := by decide

end Runtime.Turn.Speculation
