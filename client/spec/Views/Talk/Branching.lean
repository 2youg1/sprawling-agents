-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# branching：先分支，再前往

规定 `client/src/views/talk/branching.ts` 的 `pressed` 与 `settled`，以及同一文件里 `keepBranches` 据它们发 `OpenSession`、移动主区的方式（`client/Spec.lean` §4-49、D44）。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`branching.test.ts` 逐条回放本文件末尾 `#eval` 打印的轨迹向量。

开一段新会话有六个入口：信箱「最近」段一行的分支键、过去一段会话里每一回合的分支键、当前一段对话里的分支键与选行器、命令面板的「带着这段对话去…」，以及 `/new` 与 `/compact`。前四个从一行分出（`OpenSession { from }`），后两个从头开、只带不带交接（`OpenSession { carry }`）；六个都经同一个 `keepBranches` 发出，主区只在页面的 belief 显示这个房间开了新的一段之后才移到那个房间。状态机不看新的一段从哪里开始，所以下面的模型对六个入口是同一个。

城不说一条拒绝答的是哪条命令（线上没有 idem），所以答复从 belief 读，与 `talk/handing.ts` 读一次派活的答复是同一条规则：按下那一刻记下两样——这个房间当前一段从哪个 seq 开始（`belief.sessions[room]`），以及最新的那条拒绝；之后 belief 里这个房间的一段比记下的新，就是城开了分支；出现一条比记下的新的拒绝，就是城拒了它。两样在同一拍到达时开成的一方算数，因为一段已经存在，按它不在处理就是对人说谎。

模型只留一个房间：房间当前一段的 seq 与拒绝的到达号都写成自然数，0 是「还没有」。

性质：

1. **只有新的一段让主区移动**（`a_go_answers_a_newer_session`）：一步发出「前往」，当且仅当那一步是在途时 belief 到达、且这个房间的一段比按下时记下的新；按下本身从不前往（`a_press_never_goes`）。
2. **拒绝让主区留在原处**（`a_refusal_keeps_main`）：在途时到来一条新的拒绝、房间没有新的一段，状态进 `refused`，不前往。
3. **同时到达时开成的算数**（`an_opened_session_outweighs_a_refusal`）。
4. **一次只开一个分支**（`one_branch_at_a_time`）：在途时再按，什么也不发、不改记下的东西。
5. **每一次前往都对应自己的那一次发出**（`every_go_follows_a_send`）：从空闲起走任意一段事件，前往的次数不多于离开页面的 `OpenSession` 的次数。

咬得动的演示：今天两个入口的写法——发出之后同一拍就前往（`stepEager`）——在城还没回答时就移动主区（`an_eager_branch_goes_before_the_city`）。
-/

namespace Client.Views.Talk.Branching

/-- 按下或 belief 变化时，页面读到的这个房间的两样事实：当前一段从哪个 seq 开始，与最新那条拒绝的到达号；0 是还没有。 -/
structure Seen where
  session : Nat
  refusal : Nat
  deriving DecidableEq, Repr

/-- 这个状态机停在哪：空闲、在途（带按下时记下的事实）、被拒。 -/
inductive Phase where
  | idle
  | opening (mark : Seen)
  | refused
  deriving DecidableEq, Repr

/-- 人按下分支键（`left`：命令离开了页面），或 belief 变成新的读数。 -/
inductive Event where
  | press (seen : Seen) (left : Bool)
  | arrive (seen : Seen)
  deriving DecidableEq, Repr

/-- 这个状态机做的事：`OpenSession` 离开页面，主区移到那个房间。 -/
inductive Act where
  | send
  | go
  deriving DecidableEq, Repr

/-- belief 里这个房间的一段比记下的新。 -/
def opened (mark seen : Seen) : Bool := mark.session < seen.session

/-- belief 里有一条比记下的新的拒绝。 -/
def refusedSince (mark seen : Seen) : Bool := seen.refusal ≠ 0 && seen.refusal ≠ mark.refusal

/-! D100 分支先等城开出新的一段再移动主区；三个入口共用一个状态机

- **决策**：每个开新会话的入口都调用 `views/talk/branching.ts` 的 `keepBranches` 交出的 `open(room, opening, went)`，它是全页唯一的一份（`ui().branches`）。`opening` 是 `Opening`：从一行分出（`branch`，带 `origin`），或从头开（`fresh`，带 `carry`）。按下时记下 `belief.sessions[room]` 与 `belief.refusal`，发出 `OpenSession`（分支是 `{ carry: nothing, from: origin }`，从头开是 `{ carry, from: null }`）；之后 belief 里这个房间的一段比记下的新，才把主区移到 `#/talk/<room>` 并调用 `went`（信箱借它收起，当前对话借它把选行器挑中的话放回框里、画分支的边界线，命令面板借它关上）；出现一条更新的拒绝时进 `refused`，主区不动，发起它的那一行写出原因，`E_BUSY` 时旁边是停下这个房间正在跑的 run 的 `/stop`（`belief/live.ts` 的 `newestWorking`，与框里 `/stop` 停的是同一个 run）。这条拒绝由那一行说，角落不再弹同一条（`branchClaims`）；`/new` 与 `/compact` 的拒绝由主区那一段说，措辞说的是新会话而不是分支。在途时再按任何分支键什么也不发，键说明有一个分支正在打开。
- **理由**：`u.send` 只说帧离开了页面，不说城接受了它；房间里有 run 在跑时城以 `E_BUSY` 拒绝开新的一段（`crates/accounting/src/worker/commanding/sessions.rs` 的 `open_session`）。先移动主区，页面就停在那个房间正在跑的会话上，框开着，人接下来打的字成了对正在跑的 run 的转向。belief 已经经 `session_opened` 知道新的一段，所以「开成了」是一个读得到的事实，不必猜一个时长。状态放在全页一份里而不是各入口一份，是因为角落要在拒绝到达的那一刻知道它归谁，第四个入口也只能经过这一份。
- **被击败的备选**：①发出后等一个固定时长再移动——城慢时照样早移，城快时白等；②每个入口各写一份等待——两个分支入口曾各写一份「发了就走」，`/new` 与 `/compact` 又写了第三份（#120），同一个缺陷正是这样一次次出现；③只在页面认为房间有 run 在跑时不发——城才是这件事的权威，页面的读数可能落后，拒绝照样要有地方说。
- **重开参数**：线上的拒绝带上它答的命令的 idem，那时答复按 idem 认，不再按「比记下的新」认。
-/

def step : Phase → Event → Phase × List Act
  | .opening mark, .press _ _ => (.opening mark, [])
  | .idle, .press seen left | .refused, .press seen left =>
    if left then (.opening seen, [.send]) else (.idle, [])
  | .opening mark, .arrive seen =>
    if opened mark seen then (.idle, [.go])
    else if refusedSince mark seen then (.refused, [])
    else (.opening mark, [])
  | .idle, .arrive _ => (.idle, [])
  | .refused, .arrive _ => (.refused, [])

/-- 一段事件之后停在哪，以及一路做了的事，按次序。 -/
def run (p : Phase) : List Event → Phase × List Act
  | [] => (p, [])
  | e :: es =>
    let (q, acts) := step p e
    let (r, more) := run q es
    (r, acts ++ more)

theorem a_go_answers_a_newer_session (p : Phase) (e : Event) (h : Act.go ∈ (step p e).2) :
    ∃ mark seen, p = .opening mark ∧ e = .arrive seen ∧ mark.session < seen.session := by
  cases p with
  | idle => cases e with
    | press seen left => cases left <;> simp [step] at h
    | arrive seen => simp [step] at h
  | refused => cases e with
    | press seen left => cases left <;> simp [step] at h
    | arrive seen => simp [step] at h
  | opening mark => cases e with
    | press seen left => simp [step] at h
    | arrive seen =>
      by_cases ho : opened mark seen = true
      · exact ⟨mark, seen, rfl, rfl, by simpa [opened] using ho⟩
      · by_cases hr : refusedSince mark seen = true <;> simp [step, ho, hr] at h

theorem a_press_never_goes (p : Phase) (seen : Seen) (left : Bool) :
    Act.go ∉ (step p (.press seen left)).2 := by
  cases p <;> cases left <;> simp [step]

theorem a_refusal_keeps_main (mark seen : Seen) (hs : seen.session ≤ mark.session)
    (hr : seen.refusal ≠ 0) (hn : seen.refusal ≠ mark.refusal) :
    step (.opening mark) (.arrive seen) = (.refused, []) := by
  have ho : opened mark seen = false := by simp [opened]; omega
  simp [step, ho, refusedSince, hr, hn]

theorem an_opened_session_outweighs_a_refusal (mark seen : Seen) (hs : mark.session < seen.session) :
    step (.opening mark) (.arrive seen) = (.idle, [.go]) := by
  simp [step, opened, hs]

theorem one_branch_at_a_time (mark seen : Seen) (left : Bool) :
    step (.opening mark) (.press seen left) = (.opening mark, []) := by
  rfl

/-- 一个在途的分支算一次还没有答复的发出。 -/
def pending : Phase → Nat
  | .opening _ => 1
  | .idle | .refused => 0

def count (a : Act) (acts : List Act) : Nat := (acts.filter (· == a)).length

theorem step_balances (p : Phase) (e : Event) :
    count .go (step p e).2 + pending (step p e).1 ≤ count .send (step p e).2 + pending p := by
  cases p with
  | idle => cases e with
    | press seen left => cases left <;> simp [step, count, pending]
    | arrive seen => simp [step, count, pending]
  | refused => cases e with
    | press seen left => cases left <;> simp [step, count, pending]
    | arrive seen => simp [step, count, pending]
  | opening mark => cases e with
    | press seen left => simp [step, count, pending]
    | arrive seen =>
      by_cases ho : opened mark seen = true
      · simp [step, ho, count, pending]
      · by_cases hr : refusedSince mark seen = true <;> simp [step, ho, hr, count, pending]

theorem run_balances (es : List Event) : ∀ p : Phase,
    count .go (run p es).2 + pending (run p es).1 ≤ count .send (run p es).2 + pending p := by
  induction es with
  | nil => intro p; simp [run, count]
  | cons e es ih =>
    intro p
    have hs := step_balances p e
    have hr := ih (step p e).1
    simp only [run, count, List.filter_append, List.length_append] at hr hs ⊢
    omega

theorem every_go_follows_a_send (es : List Event) :
    count .go (run .idle es).2 ≤ count .send (run .idle es).2 := by
  have h := run_balances es .idle
  simp [pending] at h
  omega

/-- 今天两个入口的写法：命令离开页面的同一拍就前往。 -/
def stepEager : Phase → Event → Phase × List Act
  | _, .press _ true => (.idle, [.send, .go])
  | p, e => step p e

theorem an_eager_branch_goes_before_the_city (seen : Seen) :
    Act.go ∈ (stepEager .idle (.press seen true)).2 := by
  simp [stepEager]

/-- 轨迹向量：每条是事件序列，以及从空闲起 `run` 给出的结果。 -/
def vectors : List (List Event) :=
  [ [.press ⟨3, 0⟩ true, .arrive ⟨3, 0⟩, .arrive ⟨9, 0⟩],
    [.press ⟨3, 0⟩ true, .arrive ⟨3, 1⟩, .arrive ⟨9, 1⟩],
    [.press ⟨3, 1⟩ true, .arrive ⟨3, 1⟩, .arrive ⟨3, 0⟩, .arrive ⟨3, 2⟩, .press ⟨3, 2⟩ true, .arrive ⟨9, 2⟩],
    [.press ⟨3, 0⟩ true, .press ⟨3, 0⟩ true, .arrive ⟨9, 1⟩],
    [.press ⟨0, 0⟩ false, .arrive ⟨4, 0⟩, .press ⟨4, 0⟩ true, .arrive ⟨4, 0⟩],
    [.arrive ⟨2, 1⟩, .press ⟨2, 1⟩ true, .press ⟨2, 1⟩ false, .arrive ⟨5, 3⟩] ]

#eval vectors.map fun events => (events, run .idle events)

end Client.Views.Talk.Branching
