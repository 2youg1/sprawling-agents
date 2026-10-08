-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::console::lifecycle

规定 `crates/sprawling/src/console/lifecycle.rs`：一个服务中的进程从开城到退出走过的面，以及每一个事件在每一个面上做什么。本文件是 `crates/sprawling/Spec.lean` 的一个分部，接口与取舍写在 `crates/sprawling/spec/Console.lean` §8-11。Lean 模型是「必须守住哪些性质」的权威，Rust 是「怎样守住」的权威。

模型只有面与事件，没有 I/O：台子上的 `OnceLock<Closing>` 在这里是 `State.cause`，worker 写交接在这里是事件 `landed`，4 s 的时限到了是事件 `deadline`。

要守住的性质（每一条都对模型容许的每一条轨迹成立）：

* 只有一次停城请求（`/quit`、页面的 `CloseCity`、信号、终端没了、服务失败）能进入 `Stopping`（`only_a_request_stops`）；
* 交互的两个面（`Cli`、`QuietHost`）收到信号形式的 `SIGINT` 不进入 `Stopping`，面也不变（`a_signalled_interrupt_leaves_an_interactive_face`）；
* 先到的缘由作数，之后的事件不改写它（`the_first_cause_stands`）；
* 一条轨迹至多写一次交接（`at_most_one_handoff`），写了交接的轨迹带着缘由（`a_handoff_names_its_cause`）；
* 时限一旦装上就一直装着，直到进程退出；时限到了，一定到达 `Gone`（`an_armed_deadline_stays_armed`、`an_armed_deadline_ends_the_process`）。

派生的检查：§5 的 `#eval` 打印每一个面与每一个事件的转移，`bin::console::lifecycle` 的测试逐行回放；实现一旦离开模型，那条测试变红。
-/

namespace Sprawling.Console.Lifecycle

/-! ## 1 面与事件 -/

/-- 收口怎样对待还在跑的 run（Rust：`wire::CloseMode`）。 -/
inductive Mode where
  | drain
  | interrupt
  deriving DecidableEq, Repr

/-- 收口有没有时限：只有终端没了才装上（Rust：`Deadline`）。 -/
inductive Deadline where
  | unarmed
  | armed
  deriving DecidableEq, Repr

/-- 进程退出时交接写没写（Rust：`Handoff`）。 -/
inductive Handoff where
  | written
  | skipped
  deriving DecidableEq, Repr

/-- 进程此刻的面（Rust：`Face`）。 -/
inductive Face where
  | opening
  | cli
  | quietHost
  | headless
  | stopping (mode : Mode) (deadline : Deadline)
  | gone (handoff : Handoff)
  deriving DecidableEq, Repr

/-- 开城结束时进入的面（Rust：`Surface`）。 -/
inductive Surface where
  | cli
  | quietHost
  | headless
  deriving DecidableEq, Repr

/-- 谁发出了 `/quit`（Rust：`Asker`）。 -/
inductive Asker where
  | console
  | page
  deriving DecidableEq, Repr

/-- 城为什么停（Rust：`accounting::worker::ClosedBy` 与 `Closing::Broken`）。 -/
inductive Cause where
  | console
  | page
  | interruptSignal
  | breakSignal
  | terminate
  | terminalLost
  | failed
  deriving DecidableEq, Repr

/-- 进程收到的事件（Rust：`Event`）。敲出来的按键不在这里：raw 模式下 Ctrl+C 只是一个键，它由控制台自己答，不是生命周期的事件。 -/
inductive Event where
  | ready (surface : Surface)
  | web
  | back
  | quit (asker : Asker) (mode : Mode)
  | interruptSignal
  | breakSignal
  | terminate
  | terminalLost
  | failed
  | landed
  | deadline
  deriving DecidableEq, Repr

def Surface.face : Surface → Face
  | .cli => .cli
  | .quietHost => .quietHost
  | .headless => .headless

def Asker.cause : Asker → Cause
  | .console => .console
  | .page => .page

/-- 键盘在城手里的两个面。 -/
def Face.interactive : Face → Bool
  | .cli | .quietHost => true
  | _ => false

def Face.isStopping : Face → Bool
  | .stopping _ _ => true
  | _ => false

def Face.isGone : Face → Bool
  | .gone _ => true
  | _ => false

def Face.armed : Face → Bool
  | .stopping _ .armed => true
  | _ => false

/-- 这个事件是不是一次停城请求。 -/
def Event.closes : Event → Bool
  | .quit _ _ | .interruptSignal | .breakSignal | .terminate | .terminalLost | .failed => true
  | _ => false

/-! ## 2 转移 -/

/-- 一个还在服务的面收到的停城请求：进入哪一档、有没有时限、缘由是什么。
`SIGINT` 在交互面上不是请求（键盘在城手里，它只能来自别的进程）；终端没了只在有终端的面上是请求。 -/
def asked (face : Face) : Event → Option (Mode × Deadline × Cause)
  | .quit asker mode => some (mode, .unarmed, asker.cause)
  | .interruptSignal => if face.interactive then none else some (.drain, .unarmed, .interruptSignal)
  | .breakSignal => some (.drain, .unarmed, .breakSignal)
  | .terminate => some (.interrupt, .unarmed, .terminate)
  | .terminalLost => if face.interactive then some (.interrupt, .armed, .terminalLost) else none
  | .failed => some (.drain, .unarmed, .failed)
  | .ready _ | .web | .back | .landed | .deadline => none

/-- 服务中的面（`opening`、`cli`、`quietHost`、`headless`）怎样走。 -/
def serving (face : Face) (event : Event) : Face × Option Cause :=
  match asked face event with
  | some (mode, deadline, cause) => (.stopping mode deadline, some cause)
  | none =>
    match face, event with
    | .opening, .ready surface => (surface.face, none)
    | .cli, .web => (.quietHost, none)
    | .quietHost, .back => (.cli, none)
    | face, _ => (face, none)

/-- 收口中再来一次显式停止：`drain` 升到 `interrupt`，`interrupt` 里再来一次就不写交接立刻退出。 -/
def again (mode : Mode) (deadline : Deadline) : Face :=
  match mode with
  | .drain => .stopping .interrupt deadline
  | .interrupt => .gone .skipped

/-- 收口中的面怎样走。`SIGTERM` 第二次到达即退出，时限归发信号的一方；终端没了只装上时限，不算一次升档。 -/
def stopping (mode : Mode) (deadline : Deadline) : Event → Face
  | .quit _ _ | .interruptSignal | .breakSignal => again mode deadline
  | .terminate => .gone .skipped
  | .terminalLost => .stopping .interrupt .armed
  | .landed => .gone .written
  | .deadline =>
    match deadline with
    | .armed => .gone .skipped
    | .unarmed => .stopping mode deadline
  | .ready _ | .web | .back | .failed => .stopping mode deadline

/-- 一步：下一个面，以及这一步新给出的缘由（只在进入 `Stopping` 的那一步有）。 -/
def step : Face → Event → Face × Option Cause
  | .stopping mode deadline, event => (stopping mode deadline event, none)
  | .gone handoff, _ => (.gone handoff, none)
  | face, event => serving face event

/-- 面加上台子上的缘由与写过的交接数。 -/
structure State where
  face : Face
  cause : Option Cause
  handoffs : Nat

def wrote (face : Face) (event : Event) : Bool :=
  face.isStopping && event == .landed

def State.next (s : State) (event : Event) : State :=
  { face := (step s.face event).1
    cause := s.cause <|> (step s.face event).2
    handoffs := if wrote s.face event then s.handoffs + 1 else s.handoffs }

/-- 从 `s` 经过任意一串事件到达 `t`。 -/
inductive Reaches : State → State → Prop where
  | here (s : State) : Reaches s s
  | next {s t : State} (event : Event) : Reaches s t → Reaches s (t.next event)

/-! ## 3 穷举：每一个面与每一个事件 -/

def allModes : List Mode := [.drain, .interrupt]
def allDeadlines : List Deadline := [.unarmed, .armed]

def allFaces : List Face :=
  [.opening, .cli, .quietHost, .headless] ++
  allModes.flatMap (fun m => allDeadlines.map (fun d => Face.stopping m d)) ++
  [.gone .written, .gone .skipped]

def allEvents : List Event :=
  [.ready .cli, .ready .quietHost, .ready .headless, .web, .back] ++
  [Asker.console, Asker.page].flatMap (fun a => allModes.map (fun m => Event.quit a m)) ++
  [.interruptSignal, .breakSignal, .terminate, .terminalLost, .failed, .landed, .deadline]

theorem every_face_is_listed (face : Face) : face ∈ allFaces := by
  cases face with
  | stopping mode deadline => cases mode <;> cases deadline <;> decide
  | gone handoff => cases handoff <;> decide
  | _ => decide

theorem every_event_is_listed (event : Event) : event ∈ allEvents := by
  cases event with
  | ready surface => cases surface <;> decide
  | quit asker mode => cases asker <;> cases mode <;> decide
  | _ => decide

/-- 对每一个面与每一个事件成立的性质，由穷举证明。 -/
theorem everywhere {p : Face → Event → Bool}
    (checked : allFaces.all (fun f => allEvents.all (fun e => p f e)) = true)
    (face : Face) (event : Event) : p face event = true := by
  have hf := List.all_eq_true.mp checked face (every_face_is_listed face)
  exact List.all_eq_true.mp hf event (every_event_is_listed event)

/-! ## 4 性质 -/

/-- 只有一次停城请求能让一个还在服务的面进入 `Stopping`。 -/
theorem only_a_request_stops (face : Face) (event : Event)
    (serving : face.isStopping = false) (alive : face.isGone = false)
    (stops : (step face event).1.isStopping = true) : event.closes = true := by
  have h := everywhere (p := fun f e =>
    !(f.isStopping == false && f.isGone == false && (step f e).1.isStopping) || e.closes)
    (by decide) face event
  simp_all

/-- 交互面上收到信号形式的 `SIGINT`：面不变，也不给缘由。 -/
theorem a_signalled_interrupt_leaves_an_interactive_face (face : Face)
    (interactive : face.interactive = true) :
    step face .interruptSignal = (face, none) := by
  have h := everywhere (p := fun f _ =>
    !f.interactive || decide (step f .interruptSignal = (f, none)))
    (by decide) face .web
  simp_all

/-- 一个面进入 `Stopping` 的那一步一定给出缘由。 -/
theorem entering_names_a_cause (face : Face) (event : Event)
    (serving : face.isStopping = false) (stops : (step face event).1.isStopping = true) :
    (step face event).2.isSome = true := by
  have h := everywhere (p := fun f e =>
    !(f.isStopping == false && (step f e).1.isStopping) || (step f e).2.isSome)
    (by decide) face event
  simp_all

/-- 交接只在收口中读到 `landed` 时写，写完即 `Gone`；别的步不会到达写过交接的 `Gone`。 -/
theorem written_only_by_landing (face : Face) (event : Event)
    (reached : (step face event).1 = .gone .written) :
    wrote face event = true ∨ face = .gone .written := by
  have h := everywhere (p := fun f e =>
    !(decide ((step f e).1 = .gone .written)) || wrote f e || decide (f = .gone .written))
    (by decide) face event
  simp_all

theorem landing_writes (face : Face) (event : Event) (w : wrote face event = true) :
    (step face event).1 = .gone .written := by
  have h := everywhere (p := fun f e => !(wrote f e) || decide ((step f e).1 = .gone .written))
    (by decide) face event
  simp_all

theorem gone_has_no_landing (event : Event) : wrote (.gone .written) event = false := by
  cases event <;> rfl

/-- 先到的缘由作数。 -/
theorem the_first_cause_stands {s t : State} {cause : Cause}
    (trace : Reaches s t) (first : s.cause = some cause) : t.cause = some cause := by
  induction trace with
  | here => exact first
  | next event _ ih => simp [State.next, ih]

/-- 交接计数的不变式：写过交接的 `Gone` 恰好一次，其余面零次。 -/
def Counted (s : State) : Prop :=
  (s.face = .gone .written ∧ s.handoffs = 1) ∨ (s.face ≠ .gone .written ∧ s.handoffs = 0)

theorem counted_steps (s : State) (event : Event) (c : Counted s) : Counted (s.next event) := by
  by_cases w : wrote s.face event = true
  · have reached := landing_writes s.face event w
    have notGone : s.face ≠ .gone .written := by
      intro g
      rw [g, gone_has_no_landing] at w
      contradiction
    have zero : s.handoffs = 0 := by
      rcases c with ⟨g, _⟩ | ⟨_, z⟩
      · exact absurd g notGone
      · exact z
    left
    simp [State.next, reached, w, zero]
  · have unwritten : wrote s.face event = false := by simpa using w
    rcases c with ⟨g, one⟩ | ⟨ng, zero⟩
    · left
      constructor
      · simp [State.next, g, step]
      · simp [State.next, unwritten, one]
    · right
      constructor
      · intro reached
        rcases written_only_by_landing s.face event reached with wr | g
        · rw [unwritten] at wr
          contradiction
        · exact ng g
      · simp [State.next, unwritten, zero]

/-- 一条轨迹至多写一次交接。 -/
theorem at_most_one_handoff {s t : State} (trace : Reaches s t)
    (fresh : s.face ≠ .gone .written) (none_yet : s.handoffs = 0) : t.handoffs ≤ 1 := by
  have keeps : Counted t := by
    induction trace with
    | here => exact Or.inr ⟨fresh, none_yet⟩
    | next event _ ih => exact counted_steps _ event ih
  rcases keeps with ⟨_, one⟩ | ⟨_, zero⟩ <;> omega

/-- 缘由的不变式：收口中与写过交接的 `Gone` 都带着缘由。 -/
def Named (s : State) : Prop :=
  (s.face.isStopping = true ∨ s.face = .gone .written) → s.cause.isSome = true

theorem named_steps (s : State) (event : Event) (n : Named s) : Named (s.next event) := by
  intro after
  cases hc : s.cause with
  | some c => simp [State.next, hc]
  | none =>
    have before : ¬ (s.face.isStopping = true ∨ s.face = .gone .written) := by
      intro b
      have := n b
      rw [hc] at this
      contradiction
    have notStopping : s.face.isStopping = false := by
      cases h : s.face.isStopping
      · rfl
      · exact absurd (Or.inl h) before
    have notWritten : s.face ≠ .gone .written := fun g => before (Or.inr g)
    rcases after with stops | reached
    · have := entering_names_a_cause s.face event notStopping stops
      simp [State.next, hc, this]
    · rcases written_only_by_landing s.face event reached with wr | g
      · simp [wrote, notStopping] at wr
      · exact absurd g notWritten

/-- 写了交接的轨迹带着缘由：从一个还没有收口的状态出发，到达写过交接的 `Gone` 时缘由已经在台子上。 -/
theorem a_handoff_names_its_cause {s t : State} (trace : Reaches s t)
    (fresh : s.face.isStopping = false) (notWritten : s.face ≠ .gone .written)
    (done : t.face = .gone .written) : t.cause.isSome = true := by
  have keeps : Named t := by
    clear done
    induction trace with
    | here =>
      intro b
      rcases b with stops | g
      · rw [fresh] at stops
        contradiction
      · exact absurd g notWritten
    | next event _ ih => exact named_steps _ event ih
  exact keeps (Or.inr done)

/-- 时限一旦装上，每一步之后仍装着，或者进程已经退出。 -/
theorem armed_steps (face : Face) (event : Event) (armed : face.armed = true) :
    (step face event).1.armed = true ∨ (step face event).1.isGone = true := by
  have h := everywhere (p := fun f e =>
    !f.armed || (step f e).1.armed || (step f e).1.isGone)
    (by decide) face event
  simp_all

theorem gone_stays (face : Face) (event : Event) (g : face.isGone = true) :
    (step face event).1.isGone = true := by
  have h := everywhere (p := fun f e => !f.isGone || (step f e).1.isGone) (by decide) face event
  simp_all

theorem an_armed_deadline_stays_armed {s t : State} (trace : Reaches s t)
    (armed : s.face.armed = true) : t.face.armed = true ∨ t.face.isGone = true := by
  induction trace with
  | here => exact Or.inl armed
  | next event _ ih =>
    rcases ih with a | g
    · exact armed_steps _ event a
    · exact Or.inr (gone_stays _ event g)

/-- 时限到了，一定到达 `Gone`。 -/
theorem an_armed_deadline_ends_the_process (face : Face) (armed : face.armed = true) :
    (step face .deadline).1.isGone = true := by
  have h := everywhere (p := fun f _ => !f.armed || (step f .deadline).1.isGone)
    (by decide) face .web
  simp_all

/-! ## 5 转移向量

每一行是「面 事件 下一个面 缘由」，名字是 Rust 的变体名；`bin::console::lifecycle` 的测试逐行回放这张表。 -/

def Mode.code : Mode → String
  | .drain => "Drain"
  | .interrupt => "Interrupt"

def Deadline.code : Deadline → String
  | .unarmed => "Unarmed"
  | .armed => "Armed"

def Face.code : Face → String
  | .opening => "Opening"
  | .cli => "Cli"
  | .quietHost => "QuietHost"
  | .headless => "Headless"
  | .stopping m d => s!"Stopping/{m.code}/{d.code}"
  | .gone .written => "Gone/Written"
  | .gone .skipped => "Gone/Skipped"

def Surface.code : Surface → String
  | .cli => "Cli"
  | .quietHost => "QuietHost"
  | .headless => "Headless"

def Asker.code : Asker → String
  | .console => "Console"
  | .page => "Page"

def Event.code : Event → String
  | .ready s => s!"Ready/{s.code}"
  | .web => "Web"
  | .back => "Back"
  | .quit a m => s!"Quit/{a.code}/{m.code}"
  | .interruptSignal => "InterruptSignal"
  | .breakSignal => "BreakSignal"
  | .terminate => "Terminate"
  | .terminalLost => "TerminalLost"
  | .failed => "Failed"
  | .landed => "Landed"
  | .deadline => "Deadline"

def Cause.code : Cause → String
  | .console => "Console"
  | .page => "Page"
  | .interruptSignal => "InterruptSignal"
  | .breakSignal => "BreakSignal"
  | .terminate => "Terminate"
  | .terminalLost => "TerminalLost"
  | .failed => "Failed"

def vector (face : Face) (event : Event) : String :=
  let (next, cause) := step face event
  s!"{face.code} {event.code} {next.code} {(cause.map Cause.code).getD "-"}"

def vectors : List String :=
  allFaces.flatMap (fun f => allEvents.map (vector f))

#eval IO.println (String.intercalate "\n" vectors)

end Sprawling.Console.Lifecycle
