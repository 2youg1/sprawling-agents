-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 的运行环境代次模型

本模型拥有 doctor 的两项判定：缓存短路，以及由版本号与 `now` 推导出的修复计划。
真实进程探测、下载、安装、文件锁与错误文本由 Rust 适配器验证；模型不声称覆盖网络或
安装器行为。

Python 3 各 minor 的 EOL 时刻取 `python/peps` 仓库 `release_management/` 目录下的
`python-releases.toml`（`docs/third-party.md` §1 跟踪其提交）。
下面的 `#eval` 打印边界向量，`Mei/src/doctor_tests.rs` 的
`the_lean_environment_vectors_replay` 逐条重放；`Mei/src/doctor/support.rs` 是同一张表
的第二份实现，两者之一漂移都会让该测试变红。
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

/-- Python 3 的 minor 版本；探测只报告 major.minor 及其后的补丁号。 -/
abbrev Minor := Nat

/-- 官方 EOL 时刻（Unix 秒）。`none` 表示本表尚未记录该 minor，按受支持处理。 -/
def endOfLife : Minor → Option Nat
  | 0 => some 1246060800
  | 1 => some 1333929600
  | 2 => some 1455926400
  | 3 => some 1506643200
  | 4 => some 1552867200
  | 5 => some 1601424000
  | 6 => some 1640217600
  | 7 => some 1687824000
  | 8 => some 1728259200
  | 9 => some 1761868800
  | 10 => some 1790812800
  | 11 => some 1822348800
  | 12 => some 1853971200
  | 13 => some 1885507200
  | 14 => some 1917043200
  | 15 => some 1948579200
  | 16 => some 1980201600
  | _ => none

inductive Support where
  | absent | supported | endOfLife
  deriving DecidableEq, Repr

/-- 探测结果决定支持状态：缺席是 `absent`，`now` 到达 EOL 时刻即 `endOfLife`。 -/
def support (probed : Option (Nat × Minor)) (now : Nat) : Support :=
  match probed with
  | none => .absent
  | some (major, minor) =>
    if major != 3 then .endOfLife
    else match endOfLife minor with
      | some instant => if now < instant then .supported else .endOfLife
      | none => .supported

/-- 一次修复要么只装 uv，要么先刷新 uv 再装 Python。 -/
inductive Repair where
  | uv | python
  deriving DecidableEq, Repr

/-- 受支持的现有 Python 不追新；缺席或 EOL 才安装最新稳定版；uv 缺席才装 uv。 -/
def plan (python : Support) (uvPresent : Bool) : Option Repair :=
  match python with
  | .supported => if uvPresent then none else some .uv
  | .absent | .endOfLife => some .python

theorem a_supported_python_with_uv_needs_no_repair :
    plan .supported true = none := rfl

theorem a_supported_python_without_uv_repairs_uv_only :
    plan .supported false = some .uv := rfl

theorem an_unusable_python_repairs_python (python : Support)
    (unusable : python = .absent ∨ python = .endOfLife) (uvPresent : Bool) :
    plan python uvPresent = some .python := by
  cases unusable with
  | inl absent => simp [absent, plan]
  | inr expired => simp [expired, plan]

theorem a_supported_python_is_never_replaced (uvPresent : Bool) :
    plan .supported uvPresent ≠ some .python := by
  cases uvPresent <;> simp [plan]

/-- 一条向量打印成一行，给 Rust 逐条重放：`python|none major minor now uv|none repair`。 -/
def vectorLine (probed : Option (Nat × Minor)) (now : Nat) (uvPresent : Bool) : String :=
  let status := support probed now
  let repair :=
    match plan status uvPresent with
    | none => "none"
    | some .uv => "uv"
    | some .python => "python"
  let python :=
    match probed with
    | none => "none 0 0"
    | some (major, minor) => s!"python {major} {minor}"
  let uv := if uvPresent then "uv" else "none"
  s!"{python} {now} {uv} {repair}"

#eval vectorLine (some (3, 12)) 1790812800 true
#eval vectorLine (some (3, 12)) 1790812800 false
#eval vectorLine (some (3, 10)) 1790812799 true
#eval vectorLine (some (3, 10)) 1790812800 true
#eval vectorLine (some (3, 9)) 1761868799 true
#eval vectorLine (some (3, 9)) 1761868800 true
#eval vectorLine (some (3, 2)) 1455926400 false
#eval vectorLine (some (3, 4)) 1552867200 true
#eval vectorLine (some (3, 16)) 1980201599 false
#eval vectorLine (some (3, 16)) 1980201600 true
#eval vectorLine (some (3, 17)) 4102444800 true
#eval vectorLine (some (2, 7)) 0 false
#eval vectorLine (some (3, 8)) 1728259200 true
#eval vectorLine (some (3, 8)) 1728259199 true
#eval vectorLine (some (3, 0)) 1246060800 true
#eval vectorLine none 0 true
#eval vectorLine none 0 false

end Mei.Environment
