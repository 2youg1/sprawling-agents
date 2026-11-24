-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# door：设置 → 远程里的门开关、「更换城钥匙」与确认码输入框

设置里的远程控制发送带独立操作标识的命令。请求开门或更换城钥匙时，只有同一操作的 `E_APPROVAL_PENDING` 让确认框出现。确认保持原有控制台批准，并带上原批准请求的标识，避免接着另一个页面的请求执行。关门不需确认码，其结果独立于确认框。没有离开页面的命令可以重试。

操作层先按标识筛选回执，其他通知归其原操作。只有主机报告该次执行成功才显示完成。计时结束与连接中断都把仍在途的结果标为待核对，清除显示的确认码。匹配的迟到回执仍能完成状态变化。若无回执，操作者先按状态区的操作标识核对控制台，再解除待核对状态并决定是否重新发起；解除不记作完成。成功、拒绝与失联状态都用常驻状态区域宣布。若输入焦点随确认框消失，交还启动控件；操作外的已有焦点保留。

性质：

1. **取消总把焦点还给打开者**：任何一段按键与回答之后，若输入框在等码，Escape 落回的是按下的那个控件，输入框清空（`cancel_returns_to_the_opener`）。
2. **码只从等码的输入框发出**：一步发出 `confirm c`，当且仅当那一步是在等码时的提交，`c` 是框里打的字，且不空（`a_code_leaves_only_from_awaiting`）；任意一段事件发出的每个码都不空（`every_sent_code_was_typed`）。
3. **拒绝清空输入框**：请求或确认在途时来一条拒绝，框里没有字，焦点回到打开者（`a_refusal_clears_the_input`）。
4. **没发出去的命令回到打开者，不编拒绝**：请求或确认在途时来 `unsent`，状态回到 `idle`、框里没有字、焦点在那个控件上；任何状态下 `unsent` 都不把状态机带进 `refused`（`an_unsent_command_returns_to_the_opener`、`unsent_invents_no_refusal`）。
5. **焦点在框里、框里有字，只在码被要着的时候**：从初始状态走任意一段事件都成立（`the_input_lives_only_while_a_code_is_asked`）。

咬得动的演示：Escape 把焦点放到别处的写法（`stepLoose`）把焦点丢在页面上（`a_loose_escape_loses_the_opener`）。
-/

namespace Client.Views.Door

/-- 两个要码的控件：开门（连同开多久）与更换城钥匙。 -/
inductive Opener where
  | door
  | key
  deriving DecidableEq, Repr

/-- 这个状态机停在哪：没事、请求在途、等码、确认在途、被拒（等人再按一次）。 -/
inductive Phase where
  | idle
  | requesting (o : Opener)
  | awaiting (o : Opener)
  | confirming (o : Opener)
  | refused (o : Opener)
  | unknownRequest (o : Opener)
  | unknownConfirm (o : Opener)
  deriving DecidableEq, Repr

/-- 焦点在哪：这个状态机之外、某个控件上、输入框里。 -/
inductive Focus where
  | elsewhere
  | opener (o : Opener)
  | input
  deriving DecidableEq, Repr

structure Door where
  phase : Phase
  typed : String
  focus : Focus
  deriving DecidableEq, Repr

def initial : Door := ⟨.idle, "", .elsewhere⟩

/-- 这个状态机发出的命令。 -/
inductive Sent where
  | openDoor
  | replaceKey
  | confirm (code : String)
  deriving DecidableEq, Repr

def asked : Opener → Sent
  | .door => .openDoor
  | .key => .replaceKey

/-- 人按的键、城的回答，与页面自己知道的「命令没离开页面」。 -/
inductive Event where
  | press (o : Opener)
  | pending
  | refusal
  | unsent
  | settled
  | timedOut
  | checked
  | type (text : String)
  | submit
  | escape

def step (d : Door) : Event → Door × Option Sent
  | .press o =>
    match d.phase with
    | .idle | .refused _ => (⟨.requesting o, "", .opener o⟩, some (asked o))
    | .requesting _ | .awaiting _ | .confirming _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .pending =>
    match d.phase with
    | .requesting o | .unknownRequest o => (⟨.awaiting o, "", .input⟩, none)
    | .idle | .awaiting _ | .confirming _ | .refused _ | .unknownConfirm _ => (d, none)
  | .type text =>
    match d.phase with
    | .awaiting _ => ({ d with typed := text }, none)
    | .idle | .requesting _ | .confirming _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .submit =>
    match d.phase with
    | .awaiting o => if d.typed = "" then (d, none) else (⟨.confirming o, d.typed, .input⟩, some (.confirm d.typed))
    | .idle | .requesting _ | .confirming _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .escape =>
    match d.phase with
    | .awaiting o => (⟨.idle, "", .opener o⟩, none)
    | .idle | .requesting _ | .confirming _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .refusal =>
    match d.phase with
    | .requesting o | .confirming o | .unknownRequest o | .unknownConfirm o => (⟨.refused o, "", .opener o⟩, none)
    | .idle | .awaiting _ | .refused _ => (d, none)
  | .unsent =>
    match d.phase with
    | .requesting o | .confirming o => (⟨.idle, "", .opener o⟩, none)
    | .idle | .awaiting _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .settled =>
    match d.phase with
    | .confirming o | .unknownConfirm o => (⟨.idle, "", .opener o⟩, none)
    | .idle | .requesting _ | .awaiting _ | .refused _ | .unknownRequest _ => (d, none)
  | .timedOut =>
    match d.phase with
    | .requesting o => (⟨.unknownRequest o, "", .opener o⟩, none)
    | .confirming o => (⟨.unknownConfirm o, "", .opener o⟩, none)
    | .idle | .awaiting _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | .checked =>
    match d.phase with
    | .unknownRequest o | .unknownConfirm o => (⟨.idle, "", .opener o⟩, none)
    | .idle | .requesting _ | .awaiting _ | .confirming _ | .refused _ => (d, none)

def matched (request receipt : Nat) (d : Door) (event : Event) : Door × Option Sent :=
  if request = receipt then step d event else (d, none)

theorem a_foreign_receipt_changes_nothing (request receipt : Nat) (d : Door) (event : Event)
    (foreignReceipt : request ≠ receipt) :
    matched request receipt d event = (d, none) := by simp [matched, foreignReceipt]

theorem a_timeout_never_reports_completion (d : Door) (o : Opener)
    (h : d.phase = .requesting o ∨ d.phase = .confirming o) :
    (step d .timedOut).1.phase = .unknownRequest o ∨
      (step d .timedOut).1.phase = .unknownConfirm o := by
  rcases h with h | h <;> simp [step, h]

theorem checking_never_sends (d : Door) : (step d .checked).2 = none := by
  cases hp : d.phase <;> simp [step, hp]

def run (d : Door) (es : List Event) : Door := es.foldl (fun d e => (step d e).1) d

/-- 一段事件发出的全部命令，按发出的次序。 -/
def sends (d : Door) : List Event → List Sent
  | [] => []
  | e :: es => (step d e).2.toList ++ sends (step d e).1 es

theorem cancel_returns_to_the_opener (es : List Event) (o : Opener)
    (h : (run initial es).phase = .awaiting o) :
    (step (run initial es) .escape).1 = ⟨.idle, "", .opener o⟩ := by
  simp [step, h]

theorem a_code_leaves_only_from_awaiting (d : Door) (e : Event) (c : String)
    (h : (step d e).2 = some (.confirm c)) :
    ∃ o, d.phase = .awaiting o ∧ e = .submit ∧ c = d.typed ∧ c ≠ "" := by
  cases e with
  | submit =>
    cases hp : d.phase with
    | awaiting o =>
      by_cases ht : d.typed = ""
      · simp [step, hp, ht] at h
      · simp [step, hp, ht] at h
        exact ⟨o, rfl, rfl, h.symm, h ▸ ht⟩
    | idle | requesting _ | confirming _ | refused _ | unknownRequest _ | unknownConfirm _ => simp [step, hp] at h
  | press o => cases o <;> cases hp : d.phase <;> simp [step, hp, asked] at h
  | pending | refusal | unsent | settled | timedOut | checked | type _ | escape => cases hp : d.phase <;> simp [step, hp] at h

theorem every_sent_code_was_typed :
    ∀ (es : List Event) (d : Door) (c : String), Sent.confirm c ∈ sends d es → c ≠ "" := by
  intro es
  induction es with
  | nil => intro d c h; simp [sends] at h
  | cons e es ih =>
    intro d c h
    simp only [sends, List.mem_append] at h
    rcases h with h | h
    · cases hs : (step d e).2 with
      | none => simp [hs] at h
      | some s =>
        simp [hs] at h
        subst h
        obtain ⟨_, _, _, _, hc⟩ := a_code_leaves_only_from_awaiting d e c hs
        exact hc
    · exact ih _ c h

theorem a_refusal_clears_the_input (d : Door) (o : Opener)
    (h : d.phase = .requesting o ∨ d.phase = .confirming o) :
    (step d .refusal).1 = ⟨.refused o, "", .opener o⟩ := by
  rcases h with h | h <;> simp [step, h]

theorem a_stray_refusal_changes_nothing (d : Door)
    (h : d.phase = .idle ∨ (∃ o, d.phase = .awaiting o) ∨ (∃ o, d.phase = .refused o)) :
    step d .refusal = (d, none) := by
  rcases h with h | ⟨o, h⟩ | ⟨o, h⟩ <;> simp [step, h]

theorem an_unsent_command_returns_to_the_opener (d : Door) (o : Opener)
    (h : d.phase = .requesting o ∨ d.phase = .confirming o) :
    step d .unsent = (⟨.idle, "", .opener o⟩, none) := by
  rcases h with h | h <;> simp [step, h]

theorem unsent_invents_no_refusal (d : Door) (o : Opener)
    (h : (step d .unsent).1.phase = .refused o) : d.phase = .refused o := by
  cases hp : d.phase <;> simp [step, hp] at h ⊢ <;> exact h

/-- 焦点在框里、框里有字，都只在等码或确认在途时。 -/
def Asked (d : Door) : Prop :=
  (d.focus = .input → ∃ o, d.phase = .awaiting o ∨ d.phase = .confirming o) ∧
  (d.typed ≠ "" → ∃ o, d.phase = .awaiting o ∨ d.phase = .confirming o)

theorem step_keeps_asked (d : Door) (e : Event) (h : Asked d) : Asked (step d e).1 := by
  obtain ⟨hf, ht⟩ := h
  cases e <;> cases hp : d.phase <;> simp [step, hp, Asked] at hf ht ⊢ <;>
    first
    | exact ⟨hf, ht⟩
    | (constructor <;> intro _ <;> exact ⟨_, Or.inl rfl⟩)
    | (constructor <;> intro _ <;> exact ⟨_, Or.inr rfl⟩)
    | (rename_i text; constructor <;> intro _ <;> exact ⟨_, Or.inl rfl⟩)
    | (split <;> simp_all)

theorem the_input_lives_only_while_a_code_is_asked (es : List Event) : Asked (run initial es) := by
  have start : Asked initial := by simp [Asked, initial]
  unfold run
  generalize initial = d at start
  induction es generalizing d with
  | nil => exact start
  | cons e es ih => exact ih _ (step_keeps_asked d e start)

/-- Escape 把焦点放到这个状态机之外的写法。 -/
def stepLoose (d : Door) : Event → Door × Option Sent
  | .escape =>
    match d.phase with
    | .awaiting _ => (⟨.idle, "", .elsewhere⟩, none)
    | .idle | .requesting _ | .confirming _ | .refused _ | .unknownRequest _ | .unknownConfirm _ => (d, none)
  | e => step d e

theorem a_loose_escape_loses_the_opener :
    (stepLoose (run initial [.press .door, .pending]) .escape).1.focus = .elsewhere := by
  rfl

end Client.Views.Door
