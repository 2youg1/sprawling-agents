-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::console::program_status

规定 `crates/sprawling/src/console/program_status.rs`：城用 Program Status Protocol（OSC 7501）告诉它所在的终端，这座城此刻在做什么。本文件是 `crates/sprawling/Spec.lean` 的一个分部，控制台的其余接口在 `crates/sprawling/spec/Console.lean` §8-11。Lean 模型是「必须守住哪些性质」的权威，Rust 是「怎样守住」的权威。

协议本身的权威是它的规范页（Program Status Protocol，rev 0.3）：一条报告是 `ESC ] 7501 ; <pairs> ESC \`，`pairs` 是以 `:` 分隔的 `key=value`，`state` 必有，`msg` 是 UTF-8 文本的标准 base64；每条报告整条替换终端为这个程序存的那条记录；`state=clear` 不带 `id` 时删掉这个终端上的所有记录。模型里没有字节：编码只在 Rust 里，由 `bin::console::program_status` 的测试按规范的文法与上限逐条检查。

模型只有 UI 线程手里的那一小块状态（`Held`）与喂给它的三种事件，没有 I/O：一次 `Query::Metrics` 的答复在这里是 `Counts`，一条 `run_frozen` 记录在这里是 `ended`，生命周期到达 `Gone` 在这里是 `closed`。

要守住的性质（每一条都对模型容许的每一个状态与事件成立）：

* 写出的报告总与终端此刻存着的那条不同：同一条报告不连写两次（`a_report_differs_from_what_is_shown`）；
* 写出的报告就是此后终端存着的那条，不写时终端存着的不变（`a_report_is_what_is_shown_next`、`silence_keeps_what_is_shown`），所以 `Held.shown` 始终是终端手里的那条；
* 每次计数之后，终端存着的就是这次计数与最后一个结束的 run 判出的状态（`the_terminal_holds_the_judgement`）；
* 清掉之后再没有报告：`closed` 之后的任何一串事件都不写（`cleared_is_final`）；报告过东西的终端在 `closed` 时被清掉（`closing_clears`）。

报告的内容只能来自 `Counts` 与 `Rest`：`State` 的每个构造子只带计数或一个结局，所以记录里人打的字、模型的回复、路径与房间地址在类型上就到不了终端。

派生的检查：§5 的 `#eval` 打印判定表与每一个（终端存着的、结局、事件）的一步，`bin::console::program_status` 的测试逐行回放；实现一旦离开模型，那条测试变红。
-/

namespace Sprawling.Console.ProgramStatus

/-! ## 1 状态与事件 -/

/-- run 的三种结局，即 `kernel::Completion::name` 写进 `run_frozen` 的三个词。 -/
inductive Completion where
  | done
  | limit
  | cancelled
  deriving DecidableEq, Repr

/-- 没有 run 在跑、也没有请求待批时城报告什么：由最后一个结束的 run 怎样结束决定（Rust：`Rest`）。 -/
inductive Rest where
  | idle
  | done
  | failed
  deriving DecidableEq, Repr

/-- 一次 `Query::Metrics` 里状态用得到的两个数（Rust：`Counts`，取自 `runs_active` 与 `approvals_waiting`）。 -/
structure Counts where
  runs : Nat
  approvals : Nat
  deriving DecidableEq, Repr

/-- 城报告的状态（Rust：`State`）。协议的五个词里，`idle`、`done`、`error` 是 `resting` 的三种，`working`、`blocked` 各一种。 -/
inductive State where
  | resting (rest : Rest)
  | working (runs : Nat)
  | blocked (approvals : Nat)
  deriving DecidableEq, Repr

/-- UI 线程写给终端的一条（Rust：`Report`）。 -/
inductive Report where
  | of (state : State)
  | cleared
  deriving DecidableEq, Repr

/-- 终端此刻为这个进程存着什么（Rust：`Shown`）。 -/
inductive Shown where
  | nothing
  | reported (state : State)
  | cleared
  deriving DecidableEq, Repr

def Report.shown : Report → Shown
  | .of state => .reported state
  | .cleared => .cleared

/-- UI 线程手里的状态（Rust：`Held`）。 -/
structure Held where
  rest : Rest
  shown : Shown
  deriving DecidableEq, Repr

/-- 开控制台时的状态：终端什么也没存，城还没有结束过 run。 -/
def Held.opening : Held := { rest := .idle, shown := .nothing }

/-- 喂给 UI 线程的事件（Rust：`Event`）。 -/
inductive Event where
  | ended (completion : Completion)
  | counted (counts : Counts)
  | closed
  deriving DecidableEq, Repr

/-! ## 2 判定与一步 -/

/-! D77 城以 OSC 7501 报告整座城的状态：由两个计数与最后一个结束的 run 判出，只在交互面上写，不做支持探测

决定：控制台在 CLI 与安静宿主两个面上，以 Program Status Protocol（OSC 7501）向它所在的终端报告整座城一条根记录（不带 `id`）。状态由 `Query::Metrics` 的 `approvals_waiting` 与 `runs_active` 判出：有请求待批是 `blocked`（`kind=permission`），否则有 run 在跑是 `working`，否则由最后一个结束的 run 的结局决定：`done` 报 `done`，`limit` 报 `error`，`cancelled` 报 `idle`（协议要求人取消时报 `idle`）。开控制台、换面、读到 `run_started`、`run_frozen`、`approval_requested`、`approval_resolved` 时各问一次计数；与终端存着的那条相同就不写；进程退出交还终端之前写 `state=clear`。`msg` 只写计数（「2 runs are going」「1 request waits for an answer」），不写记录里的任何字。`SPRAWLING_PROGRAM_STATUS=never` 关掉报告。

理由：控制台本来就在 UI 线程上收每一条提交的记录，也本来就经同一个答询函数问 `Query::Metrics`，而视图在广播之前已经发布（`bin::serving::folding`），所以读到一条记录时问出的计数已经含着它；计数的权威是视图，控制台不再另折一份 run 与请求的集合。协议 rev 0.3 允许不探测直接发报告，并要求终端忽略不认识的 OSC；在 Windows 上，报告经 crossterm 的命令写出，不支持虚拟终端序列的旧控制台走 WinAPI 那一臂，那一臂什么也不做。

被否：①先发 `OSC 7501 ; ?` 探测——控制台的输入由 crossterm 解析，它不认 OSC 回复，在 Unix 上把回复拆成 Alt+] 与一串字符键，终端的回答会变成输入框里的字；Pi 与 Claude Code 能探测，是因为它们自己解析输入。②控制台里另折一份 run 与待批请求的集合——计数就有了第二个权威，开城前就已待批的请求它也看不见。③每个房间或每个 run 一条子记录——一座城里同时在跑的 run 可以多过协议的记录上限（终端至少保留 64 条），房间地址也会因此进到终端。④转发 ACP agent 自己的 OSC 7501——城经 stdio 管道接 ACP agent，它们没有终端，不发这条序列，进度经 `session/update` 到城。

重开参数：crossterm 能把 OSC 回复交给调用方时加上探测；协议的键或状态词在新的 revision 里改变时；终端开始按子记录分组展示、且记录上限放宽时，再看每个房间一条。
-/

/-- 状态由两个计数与最后一个结束的 run 判出：待批先于在跑，在跑先于结局（D77）。 -/
def judged (counts : Counts) (rest : Rest) : State :=
  if counts.approvals > 0 then .blocked counts.approvals
  else if counts.runs > 0 then .working counts.runs
  else .resting rest

/-- 一个结局让城此后静下来时报告什么：人取消的报 `idle`，撞到上限的报 `error`。 -/
def Completion.rest : Completion → Rest
  | .done => .done
  | .limit => .failed
  | .cancelled => .idle

/-- 一步：此后的状态，以及这一步写给终端的报告（至多一条）。清掉之后什么也不写。 -/
def step (held : Held) (event : Event) : Held × Option Report :=
  match held.shown, event with
  | .cleared, _ => (held, none)
  | _, .ended completion => ({ held with rest := completion.rest }, none)
  | shown, .counted counts =>
    if shown = .reported (judged counts held.rest) then (held, none)
    else ({ held with shown := .reported (judged counts held.rest) },
          some (.of (judged counts held.rest)))
  | .nothing, .closed => ({ held with shown := .cleared }, none)
  | .reported _, .closed => ({ held with shown := .cleared }, some .cleared)

def Held.next (held : Held) (event : Event) : Held := (step held event).1

/-- 从 `h` 经过任意一串事件到达 `g`。 -/
inductive Reaches : Held → Held → Prop where
  | here (h : Held) : Reaches h h
  | next {h g : Held} (event : Event) : Reaches h g → Reaches h (g.next event)

/-! ## 3 性质 -/

/-- 写出的报告总与终端此刻存着的不同：同一条不连写两次。 -/
theorem a_report_differs_from_what_is_shown (held : Held) (event : Event) (report : Report)
    (wrote : (step held event).2 = some report) : report.shown ≠ held.shown := by
  rcases held with ⟨rest, shown⟩
  cases shown with
  | cleared => cases event <;> simp [step] at wrote
  | nothing =>
    cases event with
    | ended _ => simp [step] at wrote
    | closed => simp [step] at wrote
    | counted counts =>
      simp [step] at wrote
      subst wrote
      simp [Report.shown]
  | reported state =>
    cases event with
    | ended _ => simp [step] at wrote
    | closed =>
      simp [step] at wrote
      subst wrote
      simp [Report.shown]
    | counted counts =>
      by_cases same : state = judged counts rest
      · subst same
        simp [step] at wrote
      · simp [step, same] at wrote
        subst wrote
        simp only [Report.shown, ne_eq, Shown.reported.injEq]
        exact fun e => same e.symm

/-- 写出的报告就是此后终端存着的那条。 -/
theorem a_report_is_what_is_shown_next (held : Held) (event : Event) (report : Report)
    (wrote : (step held event).2 = some report) : (step held event).1.shown = report.shown := by
  rcases held with ⟨rest, shown⟩
  cases shown with
  | cleared => cases event <;> simp [step] at wrote
  | nothing =>
    cases event with
    | ended _ => simp [step] at wrote
    | closed => simp [step] at wrote
    | counted counts =>
      simp [step] at wrote
      subst wrote
      simp [step, Report.shown]
  | reported state =>
    cases event with
    | ended _ => simp [step] at wrote
    | closed =>
      simp [step] at wrote
      subst wrote
      simp [step, Report.shown]
    | counted counts =>
      by_cases same : state = judged counts rest
      · subst same
        simp [step] at wrote
      · simp [step, same] at wrote
        subst wrote
        simp [step, same, Report.shown]

/-- 不写的一步不改终端存着的那条，只有 `closed` 例外：没报告过东西的终端无须清，状态照样记为已清。 -/
theorem silence_keeps_what_is_shown (held : Held) (event : Event)
    (quiet : (step held event).2 = none) (open_ : event ≠ .closed) :
    (step held event).1.shown = held.shown := by
  rcases held with ⟨rest, shown⟩
  cases shown with
  | cleared => simp [step]
  | nothing =>
    cases event with
    | ended _ => simp [step]
    | closed => exact absurd rfl open_
    | counted counts => simp [step] at quiet
  | reported state =>
    cases event with
    | ended _ => simp [step]
    | closed => exact absurd rfl open_
    | counted counts =>
      by_cases same : state = judged counts rest
      · subst same
        simp [step]
      · simp [step, same] at quiet

/-- 每次计数之后，终端存着的就是这次判出的状态（清掉之前）。 -/
theorem the_terminal_holds_the_judgement (held : Held) (counts : Counts)
    (live : held.shown ≠ .cleared) :
    (step held (.counted counts)).1.shown = .reported (judged counts held.rest) := by
  rcases held with ⟨rest, shown⟩
  cases shown with
  | cleared => exact absurd rfl live
  | nothing => simp [step]
  | reported state =>
    by_cases same : state = judged counts rest
    · subst same
      simp [step]
    · simp [step, same]

/-- 清掉的一步什么也不写，状态不动。 -/
theorem cleared_steps (held : Held) (event : Event) (cleared : held.shown = .cleared) :
    step held event = (held, none) := by
  rcases held with ⟨rest, shown⟩
  simp only at cleared
  subst cleared
  cases event <;> simp [step]

/-- 清掉之后再没有报告：之后的任何一串事件都停在清掉的状态。 -/
theorem cleared_is_final {h g : Held} (trace : Reaches h g) (cleared : h.shown = .cleared) :
    g = h := by
  induction trace with
  | here => rfl
  | next event _ ih =>
    subst ih
    simp [Held.next, cleared_steps _ event cleared]

/-- 报告过东西的终端在 `closed` 时被清掉。 -/
theorem closing_clears (held : Held) (state : State) (shown : held.shown = .reported state) :
    step held .closed = ({ held with shown := .cleared }, some .cleared) := by
  rcases held with ⟨rest, shown'⟩
  simp only at shown
  subst shown
  simp [step]

/-- 一条走到每种状态的轨迹：保证不是从一个无法满足的前提推出来的。 -/
example :
    let a := step Held.opening (.counted ⟨1, 0⟩)
    let b := step a.1 (.counted ⟨1, 1⟩)
    let c := step b.1 (.ended .done)
    let d := step c.1 (.counted ⟨0, 0⟩)
    let e := step d.1 (.counted ⟨0, 0⟩)
    let f := step e.1 .closed
    a.2 = some (.of (.working 1)) ∧ b.2 = some (.of (.blocked 1)) ∧ c.2 = none ∧
      d.2 = some (.of (.resting .done)) ∧ e.2 = none ∧ f.2 = some .cleared := by
  decide

/-! ## 4 穷举的取值 -/

def allCompletions : List Completion := [.done, .limit, .cancelled]
def allRests : List Rest := [.idle, .done, .failed]
def someCounts : List Counts :=
  [⟨0, 0⟩, ⟨1, 0⟩, ⟨2, 0⟩, ⟨0, 1⟩, ⟨1, 1⟩, ⟨0, 2⟩, ⟨3, 2⟩]
def someShown : List Shown :=
  [.nothing, .reported (.working 1), .reported (.blocked 1), .reported (.resting .done),
   .reported (.resting .idle), .cleared]
def someEvents : List Event :=
  [.ended .done, .ended .limit, .ended .cancelled, .counted ⟨0, 0⟩, .counted ⟨1, 0⟩,
   .counted ⟨0, 1⟩, .closed]

/-! ## 5 转移向量

两张表，名字是 Rust 的变体名。`judged <runs> <approvals> <rest> <state>` 是判定；`step <shown> <rest> <event> <shown'> <rest'> <report>` 是一步。`bin::console::program_status` 的测试逐行回放。 -/

def Completion.code : Completion → String
  | .done => "done"
  | .limit => "limit"
  | .cancelled => "cancelled"

def Rest.code : Rest → String
  | .idle => "Idle"
  | .done => "Done"
  | .failed => "Failed"

def State.code : State → String
  | .resting rest => s!"Resting/{rest.code}"
  | .working runs => s!"Working/{runs}"
  | .blocked approvals => s!"Blocked/{approvals}"

def Shown.code : Shown → String
  | .nothing => "Nothing"
  | .reported state => s!"Reported/{state.code}"
  | .cleared => "Cleared"

def Report.code : Report → String
  | .of state => s!"Of/{state.code}"
  | .cleared => "Cleared"

def Event.code : Event → String
  | .ended completion => s!"Ended/{completion.code}"
  | .counted counts => s!"Counted/{counts.runs}/{counts.approvals}"
  | .closed => "Closed"

def judgedRow (counts : Counts) (rest : Rest) : String :=
  s!"judged {counts.runs} {counts.approvals} {rest.code} {(judged counts rest).code}"

def stepRow (shown : Shown) (rest : Rest) (event : Event) : String :=
  let (held, report) := step { rest, shown } event
  s!"step {shown.code} {rest.code} {event.code} {held.shown.code} {held.rest.code} {(report.map Report.code).getD "-"}"

def vectors : List String :=
  (someCounts.flatMap fun c => allRests.map (judgedRow c)) ++
  (someShown.flatMap fun s => allRests.flatMap fun r => someEvents.map (stepRow s r))

#eval IO.println (String.intercalate "\n" vectors)

end Sprawling.Console.ProgramStatus
