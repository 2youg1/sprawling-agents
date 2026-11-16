-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# scope：没有写下的就是没有准许

规定 `crates/desktop/src/scope.rs`（`desktop::scope`，`Scope::read`、`Scope::parse`、`Scope::admits`、`Admitted::sound`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

`DESKTOP.toml` 只表达一份 allowlist（窗口标题的 pattern、进程名的 pattern）、`record`／`clipboard` 两位开关与 `sound` 一个设备名。一行 pattern 匹配什么归 `spec/Scope/Pattern.lean`；这里把「哪些名字被某一行匹配」当作两个判定 `windows`、`processes`。一次 `tools/call` 在碰到平台之前先过这里（§7），所以「越界」在任何 Win32 调用之前就已判完。

六条性质：

1. **没有文件、坏文件都关成全拒，坏文件恒不退回默认允许**（`only_a_parsed_file_opens`、`a_closed_scope_refuses_everything`）：缺文件是没人给过的许可（`E_GATE_DENIED`），读不出是文件本身有缺陷（`E_CONFIG_INVALID`）。
2. **开关没开，那件工具恒被拒**（`a_switch_left_off_refuses_its_tool`）。
3. **要窗口的工具不指名窗口即拒**（`a_window_tool_must_name_a_window`）：整屏截取在本版恒被拒（§10 设计三）。
4. **给了的每一个标识都要落在自己那张表里**（`every_named_identifier_is_listed`）：同时给 title 与 process 而只中一个是拒绝，fail closed 的一致读法。
5. **空 allowlist 不容许任何窗口**（`an_empty_allowlist_admits_no_window`）。
6. **没写 `sound` 时，要声音的录制恒被拒**（`no_device_named_no_sound`）：设备名只从 scope 文件来，本 server 恒不替人选（D11）。

**拿掉「两个都要中」，一个进程没列出的窗口凭标题混进来**（`withoutBoth_admits_an_unlisted_process`）：本模型咬得动的演示。
-/

namespace Desktop.Scope

/-- 六件工具（`tools::ToolName`）。 -/
inductive Tool where
  | windows
  | snapshot
  | act
  | screenshot
  | record
  | clipboard
  deriving DecidableEq, Repr

/-- `names_a_window`：哪些工具必须指名一扇窗口；对六件穷尽，第七件在 Rust 里是编译错误。 -/
def namesAWindow : Tool → Bool
  | .snapshot => true
  | .act => true
  | .screenshot => true
  | .record => true
  | .windows => false
  | .clipboard => false

/-- 一份读得出的 scope 文件准许什么。 -/
structure Allowance where
  windows : String → Bool
  processes : String → Bool
  record : Bool
  clipboard : Bool
  sound : Option String

/-- 关上的 scope 带的码：没人写过的文件与读不出的文件是两件事。 -/
inductive Closed where
  | gateDenied
  | configInvalid
  deriving DecidableEq, Repr

inductive Scope where
  | closed (code : Closed)
  | opened (allowance : Allowance)

/-- 盘上那份文件的三种样子。 -/
inductive File where
  | absent
  | unreadable
  | parsed (allowance : Allowance)

/-- `Scope::read` 与 `Scope::parse`：恒不失败，缺与坏都关上。 -/
def read : File → Scope
  | .absent => .closed .gateDenied
  | .unreadable => .closed .configInvalid
  | .parsed allowance => .opened allowance

/-- 一次调用要碰什么：哪件工具、给了哪些标识。 -/
structure Reach where
  tool : Tool
  title : Option String
  process : Option String

inductive Refusal where
  | closed (code : Closed)
  | switchedOff
  | unnamed
  | unlisted
  | noDevice
  deriving DecidableEq, Repr

/-- 一个给了的标识是否落在它自己那张表里；没给的不判。 -/
def listed (table : String → Bool) : Option String → Bool
  | none => true
  | some named => table named

/-- `Allowance::admits`。 -/
def Allowance.admits (a : Allowance) (r : Reach) : Except Refusal Unit :=
  if r.tool = .record ∧ a.record = false then .error .switchedOff
  else if r.tool = .clipboard then (if a.clipboard then .ok () else .error .switchedOff)
  else if namesAWindow r.tool = false then .ok ()
  else if r.title = none ∧ r.process = none then .error .unnamed
  else if listed a.windows r.title ∧ listed a.processes r.process then .ok ()
  else .error .unlisted

/-- `Scope::admits`。 -/
def admits : Scope → Reach → Except Refusal Unit
  | .closed code, _ => .error (.closed code)
  | .opened a, r => a.admits r

/-- `Admitted::sound`：scope 文件点名的那一个设备。 -/
def sound (a : Allowance) : Except Refusal String :=
  match a.sound with
  | none => .error .noDevice
  | some device => .ok device

theorem only_a_parsed_file_opens (f : File) (a : Allowance) (h : read f = .opened a) :
    f = .parsed a := by
  cases f with
  | absent => simp [read] at h
  | unreadable => simp [read] at h
  | parsed b => simp only [read, Scope.opened.injEq] at h; rw [h]

theorem a_closed_scope_refuses_everything (code : Closed) (r : Reach) :
    admits (.closed code) r = .error (.closed code) := rfl

theorem a_switch_left_off_refuses_its_tool (a : Allowance) (r : Reach)
    (off : (r.tool = .record ∧ a.record = false) ∨ (r.tool = .clipboard ∧ a.clipboard = false)) :
    a.admits r = .error .switchedOff := by
  unfold Allowance.admits
  rcases off with ⟨tool, recording⟩ | ⟨tool, clip⟩
  · simp [tool, recording]
  · simp [tool, clip]

theorem a_window_tool_must_name_a_window (a : Allowance) (r : Reach)
    (window : namesAWindow r.tool = true) (on : r.tool = .record → a.record = true)
    (title : r.title = none) (process : r.process = none) :
    a.admits r = .error .unnamed := by
  unfold Allowance.admits
  have notClipboard : r.tool ≠ .clipboard := by
    intro clip; rw [clip] at window; simp [namesAWindow] at window
  have recordOn : ¬(r.tool = .record ∧ a.record = false) := by
    intro ⟨rec, off⟩; rw [on rec] at off; simp at off
  simp [recordOn, notClipboard, window, title, process]

theorem every_named_identifier_is_listed (a : Allowance) (r : Reach)
    (window : namesAWindow r.tool = true) (h : a.admits r = .ok ()) :
    listed a.windows r.title = true ∧ listed a.processes r.process = true := by
  unfold Allowance.admits at h
  have notClipboard : r.tool ≠ .clipboard := by
    intro clip; rw [clip] at window; simp [namesAWindow] at window
  by_cases off : r.tool = .record ∧ a.record = false
  · simp [off] at h
  · by_cases unnamed : r.title = none ∧ r.process = none
    · simp [off, notClipboard, window, unnamed] at h
    · by_cases both : listed a.windows r.title = true ∧ listed a.processes r.process = true
      · exact both
      · simp [off, notClipboard, window, unnamed, both] at h

theorem an_empty_allowlist_admits_no_window (a : Allowance)
    (noTitles : ∀ t, a.windows t = false) (noProcesses : ∀ p, a.processes p = false)
    (r : Reach) (window : namesAWindow r.tool = true) : a.admits r ≠ .ok () := by
  intro h
  have ⟨byTitle, byProcess⟩ := every_named_identifier_is_listed a r window h
  unfold Allowance.admits at h
  have notClipboard : r.tool ≠ .clipboard := by
    intro clip; rw [clip] at window; simp [namesAWindow] at window
  cases title : r.title with
  | some t => rw [title] at byTitle; simp [listed, noTitles] at byTitle
  | none =>
    cases process : r.process with
    | some p => rw [process] at byProcess; simp [listed, noProcesses] at byProcess
    | none =>
      by_cases off : r.tool = .record ∧ a.record = false
      · simp [off] at h
      · simp [off, notClipboard, window, title, process] at h

theorem no_device_named_no_sound (a : Allowance) (none : a.sound = Option.none) :
    sound a = .error .noDevice := by
  simp [sound, none]

/-!
## 咬得动的演示

只要求「中一个」，`every_named_identifier_is_listed` 不再成立：标题列过、进程没列过的窗口，凭标题就被准许。
-/

def admitsWithoutBoth (a : Allowance) (r : Reach) : Except Refusal Unit :=
  if r.title = none ∧ r.process = none then .error .unnamed
  else if listed a.windows r.title ∨ listed a.processes r.process then .ok ()
  else .error .unlisted

theorem withoutBoth_admits_an_unlisted_process :
    admitsWithoutBoth
      ⟨fun t => t == "Notepad", fun _ => false, false, false, none⟩
      ⟨.act, some "Notepad", some "evil.exe"⟩ = .ok () := by
  rfl

end Desktop.Scope
