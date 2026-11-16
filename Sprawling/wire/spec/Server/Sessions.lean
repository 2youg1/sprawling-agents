-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception::sessions：会话令牌的寿命

规定 `reception::sessions`，以及 `server::door`、`server::seat`、`server::socket` 里让会话结束的那几行（`crates/wire/src/` 下同名的文件）。本文件是 `crates/wire/Spec.lean` 的一个分部；下面一节保留它在 wire 规格里的标签 §8-93s，是 §8-93（`spec/Server.lean`）本地门的一部分，别处引作 `crates/wire/Spec.lean §8-93s`，决定引作 `wire D55`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。三条性质对每一条序列成立，由 `reception::sessions` 旁的 proptest `forgotten_and_lapsed_tokens_stay_ended_and_seated_ones_stay_good` 在随机序列上对照 Rust。

### 8-93s 会话的寿命：撤销即断开，空闲即失效

```rust
// reception::sessions：纯，时刻是入参。
pub const SESSIONS_MAX: usize = 64;
pub const SESSION_IDLE_MS: u64 = 60_000;
pub struct Sessions { … }
impl Sessions { pub fn holds(&self, presented: &str, now: TimeMs) -> bool; }
pub struct Keys<'a> { pub face: &'a BindFace, pub sessions: &'a Sessions, pub now: TimeMs }
```

- **会话记得自己属于哪台设备，socket 记得自己出示的是哪个令牌**：令牌签出时记下签它的设备；一条 socket 以会话令牌完成 hello 时在门上「入座」（seat），占用记在那个会话上，socket 结束时离座（unseat）。native key 不属于任何设备，以它完成 hello 的 socket 不入座。
- **撤销即断开**：`LocalDoor::forget` 删掉设备行和它的全部会话，然后通知每一条 socket（一个 `tokio::sync::watch`）；socket 被通知时用 hello 时出示的凭据再问一次 `Keys::pairing`，不再握着就回一条 `E_GATE_DENIED` 拒绝并关闭。入座之后 socket 自己先判一次，所以在 hello 判定与入座之间被忘掉的设备同样被关掉。页面断线后照常重连，重连前的续签被拒，页面就忘掉自己的设备钥，回到配对页（`client/src/core/local/entering.ts`）。
- **空闲即失效**：一个令牌被至少一条 socket 占着时一直有效；没有 socket 占着时（刚签出、或最后一条 socket 刚结束），`SESSION_IDLE_MS` 之内不再入座就失效。POST 只判、不入座，所以 POST 不延长令牌的寿命。失效判在被问的那一刻（`holds` 带 `now`）；签新令牌时顺手清掉已失效的，表因此不涨。
- **容量**：至多 `SESSIONS_MAX` 个令牌。签新令牌时表还是满的，就丢最旧的空闲令牌；全都被占着才丢最旧的那个，门照样通知，它的 socket 随之关闭。容量淘汰不在下面的模型里：模型的性质对表没满的每一条序列成立，Rust 侧的 proptest 也只生成这样的序列。
- **时钟坏了**：判凭据要读时钟，读不到就拒绝这次请求或这次 hello，并带上时钟的错误；socket 离座时读不到时钟，就直接结束那个令牌，不按空闲计，因为页面每次拨号前都会重新签一个。
-/

/-! D55 会话令牌在最后一条 socket 结束后空闲 60 s 失效

**决定**：`SESSION_IDLE_MS` 是 60 000 ms，从令牌签出、或占着它的最后一条 socket 结束时算起；被 socket 占着的令牌不过期。

**理由**：页面每次拨号之前都用设备钥重新签一个令牌（`client/src/core/local/dial.ts`），所以一个令牌在没有 socket 的时候只需要盖住两段：签出到 hello 之间，与断线后页面在退避阶梯上（最长 10 s 一步）等待重拨、其间发出的 POST。60 s 盖得住这两段，又把一个被页面里的脚本偷走的令牌在页面关掉之后还能用的时间压到一分钟。这与 nonce 的寿命（`NONCE_LIFETIME_MS`）同一量级，都是「城刚铸出、马上要用」的东西。

**被否**：①令牌不过期，只靠容量淘汰与进程退出——关掉的页面留下的令牌在城的整个寿命里都有效；②按绝对寿命过期（签出后 N 分钟，不论有没有 socket）——一条开了一整天的 socket 会在中途失去凭据，而 socket 是只在 hello 时出示凭据的；③POST 也延长寿命——一个偷到令牌的脚本只要不断 POST，令牌就永远不失效。

**重开参数**：页面不再每次拨号都重签，或者出现不经 socket、只靠 POST 长时间工作的客户端。
-/

namespace Wire.Server.Sessions

/-- 最后一条 socket 结束之后，会话令牌还认多久（`SESSION_IDLE_MS`）。 -/
def idle : Nat := 60000

/-- 一个令牌此刻的占用：`seated more` 是 `more + 1` 条 socket 占着它，`idleSince time` 是从 `time` 起没有 socket 占着它。 -/
inductive Hold where
  | seated (more : Nat)
  | idleSince (time : Nat)
  deriving DecidableEq, Repr

/-- 一个签出的会话令牌：令牌（模型里是一个抽象的值）、签出它的设备与它的占用。 -/
structure Session where
  token : Nat
  device : Nat
  hold : Hold
  deriving DecidableEq, Repr

/-- 门里与会话有关的那一半：配过对的设备，与签出的令牌。 -/
structure State where
  paired : List Nat
  sessions : List Session
  deriving Repr

/-- 这个会话在 `now` 还认不认：被占着就认，空闲就只认 `idle` 那么久。 -/
def Session.live (now : Nat) (s : Session) : Bool :=
  match s.hold with
  | .seated _ => true
  | .idleSince time => decide (now < time + idle)

/-- `Sessions::holds`：出示 `token` 在 `now` 算不算握着一个活着的会话令牌。 -/
def holds (state : State) (token now : Nat) : Bool :=
  state.sessions.any (fun s => s.token == token && s.live now)

/-- 门上的一步，带它自己的时刻。`mint` 是签出令牌（`BrowserDoor::open_session`），`seat` 与 `unseat` 是一条 socket 入座与离座，`forget` 是 `BrowserDoor::forget`。 -/
inductive Step where
  | pair (device : Nat)
  | mint (device token now : Nat)
  | seat (token now : Nat)
  | unseat (token now : Nat)
  | forget (device : Nat)
  deriving DecidableEq, Repr

/-- 一条 socket 以 `token` 入座：令牌此刻还认，占用就多一条。 -/
def seated (token now : Nat) (s : Session) : Session :=
  if s.token = token ∧ s.live now = true then
    match s.hold with
    | .seated more => { s with hold := .seated (more + 1) }
    | .idleSince _ => { s with hold := .seated 0 }
  else s

/-- 一条占着 `token` 的 socket 离座：占用少一条，最后一条走时从 `now` 起空闲。 -/
def unseated (token now : Nat) (s : Session) : Session :=
  if s.token = token then
    match s.hold with
    | .seated 0 => { s with hold := .idleSince now }
    | .seated (more + 1) => { s with hold := .seated more }
    | .idleSince time => { s with hold := .idleSince time }
  else s

/-- 门上的一步。签出令牌之前先清掉此刻已失效的；只有配过对的设备签得出令牌。 -/
def step (state : State) : Step → State
  | .pair device => { state with paired := device :: state.paired }
  | .mint device token now =>
    if device ∈ state.paired then
      { state with
        sessions := { token := token, device := device, hold := .idleSince now } ::
          state.sessions.filter (Session.live now) }
    else state
  | .seat token now => { state with sessions := state.sessions.map (seated token now) }
  | .unseat token now => { state with sessions := state.sessions.map (unseated token now) }
  | .forget device =>
    { paired := state.paired.filter (fun d => d != device),
      sessions := state.sessions.filter (fun s => s.device != device) }

/-- 从 `state` 起走完 `steps`。 -/
def run (state : State) (steps : List Step) : State := steps.foldl step state

theorem seated_token (token now : Nat) (s : Session) : (seated token now s).token = s.token := by
  unfold seated; split
  · split <;> rfl
  · rfl

theorem seated_device (token now : Nat) (s : Session) : (seated token now s).device = s.device := by
  unfold seated; split
  · split <;> rfl
  · rfl

theorem unseated_token (token now : Nat) (s : Session) : (unseated token now s).token = s.token := by
  unfold unseated; split
  · split <;> rfl
  · rfl

theorem unseated_device (token now : Nat) (s : Session) :
    (unseated token now s).device = s.device := by
  unfold unseated; split
  · split <;> rfl
  · rfl

/-- 设备 `device` 不在配对表里，也没有一个会话。 -/
def Clear (device : Nat) (state : State) : Prop :=
  device ∉ state.paired ∧ ∀ s ∈ state.sessions, s.device ≠ device

theorem forget_clears (device : Nat) (state : State) : Clear device (step state (.forget device)) := by
  refine ⟨?_, ?_⟩
  · simp [step]
  · intro s hs
    simp only [step, List.mem_filter, bne_iff_ne] at hs
    exact hs.2

theorem step_keeps_clear (device : Nat) (state : State) (x : Step)
    (other : x ≠ .pair device) (clear : Clear device state) : Clear device (step state x) := by
  obtain ⟨unpaired, none⟩ := clear
  cases x with
  | pair d =>
    have differs : d ≠ device := fun same => other (same ▸ rfl)
    refine ⟨?_, none⟩
    simp only [step, List.mem_cons, not_or]
    exact ⟨fun same => differs same.symm, unpaired⟩
  | mint d token now =>
    by_cases known : d ∈ state.paired
    · simp only [step, known, ↓reduceIte]
      refine ⟨unpaired, ?_⟩
      intro s hs
      simp only [List.mem_cons, List.mem_filter] at hs
      rcases hs with fresh | ⟨kept, _⟩
      · subst fresh
        exact fun same => unpaired (same ▸ known)
      · exact none s kept
    · simp only [step, known, ↓reduceIte]
      exact ⟨unpaired, none⟩
  | seat token now =>
    refine ⟨unpaired, ?_⟩
    intro s hs
    simp only [step, List.mem_map] at hs
    obtain ⟨t, kept, rfl⟩ := hs
    rw [seated_device]
    exact none t kept
  | unseat token now =>
    refine ⟨unpaired, ?_⟩
    intro s hs
    simp only [step, List.mem_map] at hs
    obtain ⟨t, kept, rfl⟩ := hs
    rw [unseated_device]
    exact none t kept
  | forget d =>
    refine ⟨?_, ?_⟩
    · simp only [step, List.mem_filter, not_and]
      exact fun listed => absurd listed unpaired
    · intro s hs
      simp only [step, List.mem_filter] at hs
      exact none s hs.1

theorem run_keeps_clear (device : Nat) :
    ∀ (state : State) (steps : List Step), (∀ x ∈ steps, x ≠ .pair device) →
      Clear device state → Clear device (run state steps)
  | _, [], _, clear => clear
  | state, x :: rest, others, clear =>
    run_keeps_clear device (step state x) rest
      (fun y listed => others y (List.mem_cons_of_mem x listed))
      (step_keeps_clear device state x (others x (by simp)) clear)

/-- **撤销即断开**：忘掉一台设备之后，只要它没有重新配对，任何一条后续序列里它都没有一个会话；占用记在会话上，所以也没有一条 socket 占着它的令牌。 -/
theorem a_forgotten_device_keeps_no_session (device : Nat) (state : State) (steps : List Step)
    (unpaired : ∀ x ∈ steps, x ≠ .pair device) :
    ∀ s ∈ (run (step state (.forget device)) steps).sessions, s.device ≠ device :=
  (run_keeps_clear device _ steps unpaired (forget_clears device state)).2

/-- 令牌 `token` 在 `time` 不认了：它的每个会话在 `time` 都已失效。 -/
def Dead (token time : Nat) (state : State) : Prop :=
  ∀ s ∈ state.sessions, s.token = token → s.live time = false

/-- 一步不早于 `time`，也没有再签出 `token`（Rust 的令牌是 32 字节的熵，不会签出第二次）。离座、配对与忘掉不带时刻的条件：它们都不让一个失效的会话重新被认。 -/
def Later (token time : Nat) : Step → Prop
  | .pair _ => True
  | .mint _ minted now => minted ≠ token ∧ time ≤ now
  | .seat _ now => time ≤ now
  | .unseat _ _ => True
  | .forget _ => True

theorem dead_stays_dead (s : Session) (time now : Nat) (dead : s.live time = false)
    (after : time ≤ now) : s.live now = false := by
  unfold Session.live at *
  cases held : s.hold with
  | seated _ => simp [held] at dead
  | idleSince since =>
    simp only [held, decide_eq_false_iff_not, Nat.not_lt] at *
    omega

theorem seated_skips_the_dead (token now : Nat) (s : Session) (dead : s.live now = false) :
    seated token now s = s := by
  unfold seated
  simp [dead]

theorem unseated_keeps_the_dead (token now time : Nat) (s : Session) (dead : s.live time = false) :
    (unseated token now s).live time = false := by
  unfold unseated
  split
  · unfold Session.live at *
    cases held : s.hold with
    | seated more => simp [held] at dead
    | idleSince since => simpa [held] using dead
  · exact dead

theorem step_keeps_dead (token time : Nat) (state : State) (x : Step) (later : Later token time x)
    (dead : Dead token time state) : Dead token time (step state x) := by
  cases x with
  | pair d => exact dead
  | mint d minted now =>
    by_cases known : d ∈ state.paired
    · simp only [step, known, ↓reduceIte]
      intro s hs same
      simp only [List.mem_cons, List.mem_filter] at hs
      rcases hs with fresh | ⟨kept, _⟩
      · subst fresh
        exact absurd same later.1
      · exact dead s kept same
    · simp only [step, known, ↓reduceIte]
      exact dead
  | seat seatedToken now =>
    intro s hs same
    simp only [step, List.mem_map] at hs
    obtain ⟨t, kept, rfl⟩ := hs
    rw [seated_token] at same
    have gone : t.live now = false := dead_stays_dead t time now (dead t kept same) later
    rw [seated_skips_the_dead seatedToken now t gone]
    exact dead t kept same
  | unseat leaving now =>
    intro s hs same
    simp only [step, List.mem_map] at hs
    obtain ⟨t, kept, rfl⟩ := hs
    rw [unseated_token] at same
    exact unseated_keeps_the_dead leaving now time t (dead t kept same)
  | forget d =>
    intro s hs same
    simp only [step, List.mem_filter] at hs
    exact dead s hs.1 same

theorem run_keeps_dead (token time : Nat) :
    ∀ (state : State) (steps : List Step), (∀ x ∈ steps, Later token time x) →
      Dead token time state → Dead token time (run state steps)
  | _, [], _, dead => dead
  | state, x :: rest, later, dead =>
    run_keeps_dead token time (step state x) rest
      (fun y listed => later y (List.mem_cons_of_mem x listed))
      (step_keeps_dead token time state x (later x (by simp)) dead)

theorem dead_of_not_holds (state : State) (token time : Nat)
    (refused : holds state token time = false) : Dead token time state := by
  intro s hs same
  unfold holds at refused
  rw [List.any_eq_false] at refused
  have := refused s hs
  simp only [same, beq_self_eq_true, Bool.true_and] at this
  simpa using this

theorem not_holds_of_dead (state : State) (token now : Nat) (dead : Dead token now state) :
    holds state token now = false := by
  unfold holds
  rw [List.any_eq_false]
  intro s hs
  by_cases same : s.token = token
  · simp [dead s hs same]
  · simp [same]

/-- **失效之后不复活**：一个令牌在某一刻不认了——它的设备被忘掉，或者它空闲过了 `idle`——此后任何一条时刻不早于那一刻、也没有再签出它的序列里，它都不再被认，所以也没有 socket 能以它入座。 -/
theorem an_expired_token_never_admits_again (token time : Nat) (state : State)
    (steps : List Step) (refused : holds state token time = false)
    (later : ∀ x ∈ steps, Later token time x) (now : Nat) (after : time ≤ now) :
    holds (run state steps) token now = false := by
  apply not_holds_of_dead
  intro s hs same
  have dead := run_keeps_dead token time state steps later (dead_of_not_holds state token time refused)
  exact dead_stays_dead s time now (dead s hs same) after

/-- 会话 `s` 的令牌与设备还在，而且被占着。 -/
def Seated (token device : Nat) (state : State) : Prop :=
  ∃ s ∈ state.sessions, s.token = token ∧ s.device = device ∧ ∃ more, s.hold = .seated more

/-- 一步既不忘掉 `device`，也不让占着 `token` 的 socket 离座。 -/
def Spares (token device : Nat) : Step → Prop
  | .forget d => d ≠ device
  | .unseat leaving _ => leaving ≠ token
  | _ => True

theorem step_keeps_seated (token device : Nat) (state : State) (x : Step)
    (spares : Spares token device x) (held : Seated token device state) :
    Seated token device (step state x) := by
  obtain ⟨s, present, same, owner, more, sat⟩ := held
  have live : ∀ now, s.live now = true := by
    intro now
    unfold Session.live
    rw [sat]
  cases x with
  | pair d => exact ⟨s, present, same, owner, more, sat⟩
  | mint d minted now =>
    by_cases known : d ∈ state.paired
    · simp only [step, known, ↓reduceIte]
      exact ⟨s, List.mem_cons_of_mem _ (List.mem_filter.mpr ⟨present, live now⟩), same, owner,
        more, sat⟩
    · simp only [step, known, ↓reduceIte]
      exact ⟨s, present, same, owner, more, sat⟩
  | seat seating now =>
    refine ⟨seated seating now s, List.mem_map_of_mem present, ?_, ?_, ?_⟩
    · rw [seated_token]; exact same
    · rw [seated_device]; exact owner
    · unfold seated
      split
      · rw [sat]; exact ⟨more + 1, rfl⟩
      · exact ⟨more, sat⟩
  | unseat leaving now =>
    have other : s.token ≠ leaving := fun clash => spares (clash.symm.trans same)
    refine ⟨s, ?_, same, owner, more, sat⟩
    simp only [step, List.mem_map]
    refine ⟨s, present, ?_⟩
    unfold unseated
    simp [other]
  | forget d =>
    refine ⟨s, ?_, same, owner, more, sat⟩
    simp only [step, List.mem_filter, bne_iff_ne]
    exact ⟨present, fun clash => spares (clash.symm.trans owner)⟩

/-- **占着的令牌不过期**：一个被 socket 占着的令牌，在任何一条既不忘掉它的设备、也不让占着它的 socket 离座的序列里，一直在、一直被占着，于是不论过去多久都认。空闲的时限只从最后一条 socket 离座时算起。 -/
theorem a_seated_token_outlasts_its_idle_time (token device : Nat) :
    ∀ (state : State) (steps : List Step), (∀ x ∈ steps, Spares token device x) →
      Seated token device state → Seated token device (run state steps)
  | _, [], _, held => held
  | state, x :: rest, spares, held =>
    a_seated_token_outlasts_its_idle_time token device (step state x) rest
      (fun y listed => spares y (List.mem_cons_of_mem x listed))
      (step_keeps_seated token device state x (spares x (by simp)) held)

/-- 一条可以实现的序列：签出、入座、离座，60 s 内还认，过了就不认。 -/
example :
    let after := run { paired := [1], sessions := [] }
      [.mint 1 7 0, .seat 7 10, .unseat 7 100]
    (holds after 7 60099, holds after 7 60100) = (true, false) := rfl

end Wire.Server.Sessions
