-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.remote_access.spec.Door

/-!
# 远程门的逐帧授权

规定 `crates/sprawling/src/outside/conduit.rs` 的 `Conduit::judge`（`bin::outside::conduit`；sprawling-SPEC.md 8-139）：一段远程会话里，设备封好的每一帧在到城之前怎样被判。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。谁能做什么（`Authority`、`Verb`、`permits`）的权威是 `crates/remote_access/spec/Door.lean`，这里只引用它；一帧属于哪一类由 `bin::outside::verbs::passage` 穷尽匹配给出，在模型里是帧已经带着的类。

打开封装、读 JSON 都可能失败：封装打不开是会话的错误，连接结束；JSON 读不成是一帧封好的 `E_WIRE_MISMATCH` 拒绝，不到城。问候（`Hello`）换上城的令牌再发，版本与 schema 仍是设备的，所以城照样判这个页面说不说它的 wire。

四条性质：

* **到城的只有放行的帧**——转发出去的若不是问候，就是设备的权限 `permits` 的那一类；
* **只看的设备从不转发一个动手的动词**，只在城所在的机器上做的动词（`localOnly`）谁也不转发；
* **读不懂的帧不到城**；
* **会话已不被门持有时什么也不转发**，连接结束。
-/

namespace Sprawling.Outside.Conduit

open RemoteDoor

/-- 打开封装之后读出的东西：锁屏，一帧读不懂的文字，或一帧按类判过的线协议帧。 -/
inductive Opened where
  | lock
  | unreadable
  | greeting
  | judged (verb : Verb)
  deriving Repr, DecidableEq

/-- 判一帧的结果（Rust：`Step`；`Answer` 分出两种拒绝，`Forward` 分出问候）。 -/
inductive Step where
  /-- 放行的原文发给城的 `/ws`。 -/
  | forward
  /-- 问候换上城的令牌再发。 -/
  | forwardGreeting
  /-- 设备收到一帧封好的拒绝：读不懂（`E_WIRE_MISMATCH`）。 -/
  | refuseUnreadable
  /-- 设备收到一帧封好的拒绝：权限不够（`E_GATE_DENIED`）。 -/
  | refuseVerb (verb : Verb)
  /-- 交回给监听去关门。 -/
  | lock
  deriving Repr, DecidableEq

/-- 会话已不被门持有：连接结束。 -/
structure Ended where
  deriving Repr, DecidableEq

/-- `Conduit::judge`。`authority` 是 `Doorway::authority` 此刻的答案，`none` 即会话已不被门持有。 -/
def judge (authority : Option Authority) : Opened → Except Ended Step
  | .lock => .ok .lock
  | .unreadable => .ok .refuseUnreadable
  | .greeting => .ok .forwardGreeting
  | .judged v =>
    match authority with
    | none => .error {}
    | some a => if permits a v then .ok .forward else .ok (.refuseVerb v)

/-- 到城的只有放行的帧。 -/
theorem forwarded_only_if_permitted (authority : Option Authority) (o : Opened)
    (h : judge authority o = .ok .forward) :
    ∃ a v, authority = some a ∧ o = .judged v ∧ permits a v = true := by
  cases o with
  | lock => simp [judge] at h
  | unreadable => simp [judge] at h
  | greeting => simp [judge] at h
  | judged v =>
    cases authority with
    | none => simp [judge] at h
    | some a =>
      by_cases hp : permits a v = true
      · exact ⟨a, v, rfl, rfl, hp⟩
      · simp [judge, hp] at h

/-- 只看的设备从不转发一个动手的动词。 -/
theorem watching_never_acts : judge (some .watch) (.judged .act) = .ok (.refuseVerb .act) := rfl

/-- 只在城所在的机器上做的动词（`localOnly`）谁也不转发。 -/
theorem local_only_is_refused (a : Authority) :
    judge (some a) (.judged .localOnly) = .ok (.refuseVerb .localOnly) := by
  simp [judge, local_only_is_never_remote]

/-- 读不懂的帧不到城。 -/
theorem unreadable_reaches_no_city (authority : Option Authority) :
    judge authority .unreadable = .ok .refuseUnreadable := rfl

/-- 会话已不被门持有时，按类判的帧不转发，连接结束。 -/
theorem ended_session_forwards_nothing (v : Verb) : judge none (.judged v) = .error {} := rfl

/-- 只看的设备仍然可以问。 -/
example : judge (some .watch) (.judged .read) = .ok .forward := rfl

end Sprawling.Outside.Conduit
