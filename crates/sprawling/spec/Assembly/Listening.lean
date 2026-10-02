-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 开城的次序：先占端口，再写第一行，最后才说 running

规定 `crates/sprawling/src/assembly/listening.rs` 的 `listen` 与 `main::city` 的横幅之间的次序（`bin::assembly::listening`、`main::city`；sprawling-SPEC.md 8-88）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

`listen` 依次做四步，每一步都可能拒绝：判定绑定面并 bind；建 CAS 目录、开账本读起视图；开写者线程，`JsonlLedger::open` 先取写者锁、再做断尾恢复、再 `open_for_service`；返回 `Listening`。账本的第一次写在取到写者锁之后。横幅只在拿到 `Listening` 之后印，这是类型排的：`serve_city` 没有 `Listening` 就走不到印横幅的那一段。

每一步的成败是参数；模型记下账本写了几行、横幅印没印。三条性质：

* **锁之前的拒绝一行不写**——端口被占、暴露的地址没有令牌、账本读不起来、写者锁被别的进程持着，账本都与之前相同；
* **横幅只在 `listen` 成功之后印**；
* **两个进程不会同时写一座城**——第二个进程要么在 bind 被拒，要么在取锁被拒，两者都在写任何东西之前。
-/

namespace Sprawling.Assembly.Listening

/-- `listen` 在哪一步拒绝。 -/
inductive Refusal where
  /-- 端口被占，或暴露的地址没有令牌。 -/
  | bind
  /-- CAS 目录建不成，或账本、快照读不起来。 -/
  | store
  /-- 写者锁被别的进程持着：`E_LEDGER_HELD`。 -/
  | held
  /-- 取到锁之后，断尾恢复或开城那几行写不进去。 -/
  | service
  deriving Repr, DecidableEq

/-- 各步成不成：`false` 是在这一步拒绝。 -/
structure Steps where
  bind : Bool
  store : Bool
  lock : Bool
  service : Bool
  deriving Repr, DecidableEq

/-- 一次开城的可见结果：拒绝或成功，账本多了几行，横幅印没印。`opening` 是取锁之后开城写下的行数（断尾恢复的一行与 `open_for_service` 的那几行）。 -/
structure Seen where
  refused : Option Refusal
  written : Nat
  banner : Bool
  deriving Repr, DecidableEq

/-- `listen`，再加上 `serve_city` 拿到 `Listening` 之后印横幅。取锁之后写不进去的那一步写了几行，Rust 不保证为零（行可能已经落下），模型保守地记作 `opening`。 -/
def openCity (s : Steps) (opening : Nat) : Seen :=
  if !s.bind then ⟨some .bind, 0, false⟩
  else if !s.store then ⟨some .store, 0, false⟩
  else if !s.lock then ⟨some .held, 0, false⟩
  else if !s.service then ⟨some .service, opening, false⟩
  else ⟨none, opening, true⟩

/-- 锁之前的拒绝一行不写。 -/
theorem refused_before_the_lock_writes_nothing (s : Steps) (opening : Nat) (r : Refusal)
    (h : (openCity s opening).refused = some r) (before : r ≠ .service) :
    (openCity s opening).written = 0 := by
  unfold openCity at h ⊢
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all

/-- 横幅只在 `listen` 成功之后印。 -/
theorem banner_only_after_listening (s : Steps) (opening : Nat)
    (h : (openCity s opening).banner = true) :
    (openCity s opening).refused = none ∧ s.bind ∧ s.store ∧ s.lock ∧ s.service := by
  unfold openCity at h ⊢
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all

/-- 第二个进程：端口或写者锁被第一个占着，它在写任何东西之前被拒，也不印横幅。 -/
theorem a_second_process_writes_nothing (s : Steps) (opening : Nat)
    (taken : s.bind = false ∨ s.lock = false) :
    (openCity s opening).written = 0 ∧ (openCity s opening).banner = false := by
  obtain ⟨b, st, l, sv⟩ := s
  rcases taken with h | h <;> cases b <;> cases st <;> cases l <;> cases sv <;> simp_all [openCity]

end Sprawling.Assembly.Listening
