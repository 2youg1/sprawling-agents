-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 的运行环境代次模型

本模型拥有 doctor 的缓存短路和 Python 修复判定。真实进程探测、下载、安装、
文件锁与错误文本由 Rust 适配器验证；模型不声称覆盖网络或安装器行为。
-/

namespace Mei.Environment

structure Stamp where
  release : String
  deriving DecidableEq, Repr

structure Cache where
  stamp : Stamp
  pythonVersion : String
  pythonExecutable : String
  uvVersion : String
  uvExecutable : String
  deriving DecidableEq, Repr

inductive Decision where
  | reuse (environment : Cache)
  | inspect
  deriving DecidableEq, Repr

/-- 只有当前 Mei 代次的完整记录才能短路；更新或缺席都重新探测并重建整条记录。 -/
def decide (release : String) (cached : Option Cache) : Decision :=
  match cached with
  | some environment =>
    if environment.stamp.release == release then .reuse environment else .inspect
  | none => .inspect

theorem current_cache_is_reused (release : String) (environment : Cache)
    (current : environment.stamp.release = release) :
    decide release (some environment) = .reuse environment := by
  simp [decide, current]

theorem absent_or_updated_cache_is_inspected (release : String)
    (cached : Option Cache)
    (stale : ∀ environment, cached = some environment → environment.stamp.release ≠ release) :
    decide release cached = .inspect := by
  cases cached with
  | none => rfl
  | some environment => simp [decide, stale environment rfl]

inductive Support where
  | absent | supported | endOfLife
  deriving DecidableEq, Repr

/-- 受支持的现有 Python 不追新；缺席或 EOL 才安装最新稳定版。 -/
def installsPython : Support → Bool
  | .supported => false
  | .absent | .endOfLife => true

theorem supported_python_is_not_replaced :
    installsPython .supported = false := rfl

theorem missing_or_expired_python_is_repaired (status : Support)
    (unusable : status = .absent ∨ status = .endOfLife) :
    installsPython status = true := by
  cases status <;> simp_all [installsPython]

example : decide "0.17.0" none = .inspect := by native_decide

end Mei.Environment
