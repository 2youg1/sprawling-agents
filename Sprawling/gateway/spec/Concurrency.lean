-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::concurrency

规定 `concurrency`（`crates/gateway/src/concurrency.rs`）：一个端点的并发名额，纯判定，只读注入的时刻与调用的结果；以及取号之前的前缀等待 `hold`（§8-40，住 `crates/gateway/src/` 下新开的 `hold.rs`）。本文件是 `crates/gateway/Spec.lean` 的一个分部，名额的形状与常量的理由在 D17，前缀等待的在 D37 到 D43。

模型是一个端点的名额状态在一条事件轨迹上的演变。事件五种：取（take）、还（give back）、被限流（429，带或不带 `Retry-After`）、一次成功、时间流逝（tick）。每个带时刻的事件自带它发生的时刻，状态不持有时钟，所以三个平台相同。

下面证明六条性质，各在一条轨迹上量化：
1. 取到的名额从不让在用数超过当下的上限；没有收窄的轨迹上，在用数始终不超过上限。收窄不收回已在飞的调用，所以「在用数 ≤ 上限」在一次收窄之后可以暂时不成立，直到在飞的调用陆续归还——这是对 D17「名额在用不超过上限」的精确读法。
2. 上限始终在 1 到配置值之间。
3. 在 `Retry-After` 给出的时刻之前，上限不会变大；上限变大的唯一一步是那个时刻之后的一次成功。
4. 连续 `WIDEN_AFTER` 次成功（在那个时刻之后）让上限加一，直到配置值。
5. 一次取答「取到」「已满」或「等到某一刻」：已满只在名额都在用时答（由一次归还唤醒），等到的时刻严格晚于此刻，所以调用方不会忙等。
6. 排队按号先来后到：取到名额的号严格按拿号的次序（「排队」一节，D20）。

文末「前缀等待」一节另证四条，各在任意一条到达、pre-warm 结束与看时钟的轨迹上量化：
7. 一个前缀键同一时刻至多一次 pre-warm 在飞，且 pre-warm 只为冷键发出。
8. pre-warm 一结束（答、拒、被丢下），等它的调用全被放行；门看过时钟之后，还在等的调用都等了不到 `HOLD_MAX`。
9. 一个在等的调用离开等待的唯一去处是号队：pre-warm 失败不会让它失败，也没有调用丢在门前。
10. 没被挡的调用按到达的次序进号队，再由性质 6 按同一次序取到名额。

与 Rust 的对应：`holdUntil` 为 0 读作 Rust 的 `hold_until: None`（任何时刻都不早于 0）；`giveBack` 的自然数减法即 Rust 的 `saturating_sub`；Rust 的 `Instant` 加法溢出时不设 `hold_until`，模型的 `Nat` 没有这一档。派生检查：`concurrency::tests` 的 proptest `permits_keep_the_lean_properties` 对随机轨迹每走一步断言性质 1（取到之后在用数不超过上限）、2、3、5，以及性质 4 的「每次放宽恰加一」；`a_narrowed_limit_widens_after_the_retry_after_instant` 在 `Retry-After` 前后各判一次性质 4。性质 1 的轨迹形（`run_keeps_within`）由 proptest `a_trace_without_narrowing_keeps_in_use_within_the_limit` 检查：一段随机前缀（可以收窄）把状态带到任意可达处，在用数不超过上限时，再走一段去掉收窄的随机轨迹，每一步断言在用数不超过上限；它先对一个在 `in_use == limit` 时仍放行的 `take` 变红过。
-/

namespace Gateway.Concurrency

/-- 连续多少次成功之后上限加一（`concurrency::WIDEN_AFTER`）。 -/
def WIDEN_AFTER : Nat := 8

/-- 一个端点的名额状态（`concurrency::Permits`）。`cap` 是配置的上限，不随事件变。 -/
structure Permits where
  cap : Nat
  limit : Nat
  inUse : Nat
  streak : Nat
  holdUntil : Nat
  deriving Repr

/-- 一次取的回答（`concurrency::Take`）。 -/
inductive Answer where
  | granted
  | full
  | waitUntil (t : Nat)
  deriving DecidableEq, Repr

/-- 一条轨迹上的一步。 -/
inductive Event where
  | take (now : Nat)
  | giveBack
  | rateLimited (now : Nat) (retryAfter : Option Nat)
  | success (now : Nat)
  | tick (now : Nat)

/-- 状态良好：上限在 1 到配置值之间，连胜数短于放宽所需。 -/
def Valid (s : Permits) : Prop :=
  1 ≤ s.limit ∧ s.limit ≤ s.cap ∧ s.streak < WIDEN_AFTER

def answer (s : Permits) (now : Nat) : Answer :=
  if now < s.holdUntil then .waitUntil s.holdUntil
  else if s.inUse < s.limit then .granted
  else .full

def narrow (s : Permits) (now : Nat) (retryAfter : Option Nat) : Permits :=
  { s with
    limit := max 1 (s.limit / 2)
    streak := 0
    holdUntil := match retryAfter with
      | some d => max s.holdUntil (now + d)
      | none => s.holdUntil }

/-- 一次成功：连胜加一；凑满 `WIDEN_AFTER` 且已过 `holdUntil` 且未到配置值时上限加一、连胜归零；
凑满却还不能放宽时连胜停在差一次的位置，于是那个时刻之后的第一次成功就放宽。 -/
def succeed (s : Permits) (now : Nat) : Permits :=
  if s.streak + 1 < WIDEN_AFTER then { s with streak := s.streak + 1 }
  else if s.holdUntil ≤ now ∧ s.limit < s.cap then { s with limit := s.limit + 1, streak := 0 }
  else s

def step (s : Permits) : Event → Permits
  | .take now =>
    match answer s now with
    | .granted => { s with inUse := s.inUse + 1 }
    | .full => s
    | .waitUntil _ => s
  | .giveBack => { s with inUse := s.inUse - 1 }
  | .rateLimited now ra => narrow s now ra
  | .success now => succeed s now
  | .tick _ => s

def run (s : Permits) (trace : List Event) : Permits := trace.foldl step s

theorem run_cons (s : Permits) (e : Event) (es : List Event) :
    run s (e :: es) = run (step s e) es := rfl

/-! ### 性质 5：取不会让调用方忙等 -/

theorem wait_is_later (s : Permits) (now t : Nat) (h : answer s now = .waitUntil t) : now < t := by
  unfold answer at h
  split at h
  · cases h; assumption
  · split at h <;> cases h

theorem full_only_when_all_in_use (s : Permits) (now : Nat) (h : answer s now = .full) :
    s.limit ≤ s.inUse := by
  unfold answer at h
  split at h
  · cases h
  · split at h
    · cases h
    · omega

/-! ### 性质 1：取到的名额不越过上限 -/

theorem granted_within_limit (s : Permits) (now : Nat) (h : answer s now = .granted) :
    (step s (.take now)).inUse ≤ s.limit := by
  simp only [step, h]
  unfold answer at h
  split at h
  · cases h
  · split at h
    · omega
    · cases h

def Narrows : Event → Bool
  | .rateLimited _ _ => true
  | _ => false

theorem step_keeps_within (s : Permits) (e : Event) (hn : Narrows e = false)
    (hw : s.inUse ≤ s.limit) :
    (step s e).inUse ≤ (step s e).limit := by
  cases e with
  | take now =>
    have hg := granted_within_limit s now
    cases ha : answer s now with
    | granted =>
      have h := hg ha
      simp only [step, ha] at h ⊢
      exact h
    | full => simp only [step, ha]; exact hw
    | waitUntil t => simp only [step, ha]; exact hw
  | giveBack => simp only [step]; omega
  | rateLimited now ra => simp [Narrows] at hn
  | success now =>
    simp only [step, succeed]
    split
    · exact hw
    · split
      · simp only; omega
      · exact hw
  | tick now => exact hw

/-! ### 性质 2：上限在 1 到配置值之间 -/

theorem step_valid (s : Permits) (e : Event) (hv : Valid s) : Valid (step s e) := by
  obtain ⟨h1, h2, h3⟩ := hv
  cases e with
  | take now =>
    simp only [step]
    split <;> exact ⟨h1, h2, h3⟩
  | giveBack => exact ⟨h1, h2, h3⟩
  | rateLimited now ra =>
    refine ⟨?_, ?_, ?_⟩ <;> simp only [step, narrow, WIDEN_AFTER] <;> omega
  | success now =>
    simp only [step, succeed]
    split
    · exact ⟨h1, h2, by assumption⟩
    · split
      · rename_i h
        refine ⟨?_, ?_, ?_⟩ <;> simp only [WIDEN_AFTER] <;> omega
      · exact ⟨h1, h2, h3⟩
  | tick now => exact ⟨h1, h2, h3⟩

theorem run_valid (s : Permits) (trace : List Event) (hv : Valid s) : Valid (run s trace) := by
  induction trace generalizing s with
  | nil => exact hv
  | cons e es ih => rw [run_cons]; exact ih _ (step_valid s e hv)

theorem run_keeps_within (s : Permits) (trace : List Event) (hv : Valid s)
    (hw : s.inUse ≤ s.limit) (hn : ∀ e ∈ trace, Narrows e = false) :
    (run s trace).inUse ≤ (run s trace).limit := by
  induction trace generalizing s with
  | nil => exact hw
  | cons e es ih =>
    rw [run_cons]
    exact ih _ (step_valid s e hv)
      (step_keeps_within s e (hn e (List.Mem.head _)) hw)
      (fun e' he' => hn e' (List.Mem.tail _ he'))

/-! ### 性质 3：`Retry-After` 的时刻之前不放宽 -/

theorem holdUntil_never_earlier (s : Permits) (e : Event) :
    s.holdUntil ≤ (step s e).holdUntil := by
  cases e with
  | take now => simp only [step]; split <;> simp
  | giveBack => simp [step]
  | rateLimited now ra =>
    cases ra <;> simp only [step, narrow] <;> omega
  | success now =>
    simp only [step, succeed]
    split
    · simp
    · split <;> simp
  | tick now => simp [step]

theorem widens_only_after_hold (s : Permits) (e : Event) (hv : Valid s)
    (h : s.limit < (step s e).limit) :
    ∃ now, e = .success now ∧ s.holdUntil ≤ now := by
  obtain ⟨h1, _, _⟩ := hv
  cases e with
  | take now =>
    simp only [step] at h
    split at h <;> simp at h
  | giveBack => simp [step] at h
  | rateLimited now ra => simp only [step, narrow] at h; omega
  | success now =>
    refine ⟨now, rfl, ?_⟩
    simp only [step, succeed] at h
    split at h
    · simp at h
    · split at h
      · rename_i hc; exact hc.1
      · simp at h
  | tick now => simp [step] at h

/-- 一条轨迹上每一步都发生在 `holdUntil` 之前时，上限不变大。 -/
def Before (hold : Nat) : Event → Prop
  | .take now => now < hold
  | .giveBack => True
  | .rateLimited now _ => now < hold
  | .success now => now < hold
  | .tick now => now < hold

theorem no_widening_before_hold (s : Permits) (trace : List Event) (hv : Valid s)
    (hb : ∀ e ∈ trace, Before s.holdUntil e) :
    (run s trace).limit ≤ s.limit := by
  induction trace generalizing s with
  | nil => exact Nat.le_refl _
  | cons e es ih =>
    rw [run_cons]
    have hs : (step s e).limit ≤ s.limit := by
      apply Nat.le_of_not_lt
      intro hlt
      obtain ⟨now, he, hle⟩ := widens_only_after_hold s e hv hlt
      have := hb e (List.Mem.head _)
      rw [he] at this
      simp only [Before] at this
      omega
    have hrest := ih (step s e) (step_valid s e hv) (fun e' he' =>
      match e', hb e' (List.Mem.tail _ he') with
      | .take _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .giveBack, h => h
      | .rateLimited _ _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .success _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .tick _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e))
    omega

/-! ### 性质 4：连续 `WIDEN_AFTER` 次成功放宽一格 -/

theorem successes_widen_by_one (s : Permits) (now : Nat) (hz : s.streak = 0)
    (hh : s.holdUntil ≤ now) (hc : s.limit < s.cap) :
    (run s (List.replicate WIDEN_AFTER (.success now))).limit = s.limit + 1 := by
  obtain ⟨cap, limit, inUse, streak, hold⟩ := s
  simp only at hz hh hc
  subst hz
  simp [WIDEN_AFTER, List.replicate, run, step, succeed, hh, hc]

theorem step_keeps_cap (s : Permits) (e : Event) : (step s e).cap = s.cap := by
  cases e with
  | take now => simp only [step]; split <;> rfl
  | giveBack => rfl
  | rateLimited now ra => rfl
  | success now =>
    simp only [step, succeed]
    split
    · rfl
    · split <;> rfl
  | tick now => rfl

theorem run_keeps_cap (s : Permits) (trace : List Event) : (run s trace).cap = s.cap := by
  induction trace generalizing s with
  | nil => rfl
  | cons e es ih => rw [run_cons, ih, step_keeps_cap]

/-- 成功再多，上限也停在配置值。 -/
theorem successes_stop_at_cap (s : Permits) (trace : List Event) (hv : Valid s) :
    (run s trace).limit ≤ s.cap := by
  rw [← run_keeps_cap s trace]
  exact (run_valid s trace hv).2.1


/-!
## 排队：按号先来后到（D20）

`Full` 时调用拿一张号排队，名额空出时只有队首那张号取到；排得太久的号被拒出队（`QUEUE_WAIT_MAX`）。模型只看号：到达、给队首一个名额、拒掉某张号。下面证明，在任意一条轨迹上，取到名额的号按拿号的次序排列——后到的调用从不先于先到的取到（`grants_follow_arrival`）。派生检查：`crates/gateway/src/endpoint/permit.rs` 的 proptest `grants_follow_arrival` 对随机的到达、任一排队者的尝试、归还与超时轨迹驱动 `Gate` 的号队列（`admit` 的每一步都是它），断言取到的号严格递增；它先对一个把名额给最新那张号的队列变红过。
-/

/-- 一个端点的排队（`endpoint::permit` 的 `Queue`，在 `Gate` 里）。`granted` 是观察用的影子：Rust 不留已取到的号，proptest 自己记下它们。 -/
structure Queue where
  next : Nat
  waiting : List Nat
  granted : List Nat
  deriving Repr

inductive QueueEvent where
  /-- 一次调用拿到号 `next`，排到队尾。 -/
  | arrive
  /-- 一个名额空出，队首那张号取到。 -/
  | grantHead
  /-- 号 `t` 等过了 `QUEUE_WAIT_MAX`，被拒出队。 -/
  | shed (t : Nat)
  deriving Repr

def Queue.empty : Queue := ⟨0, [], []⟩

def queueStep (q : Queue) : QueueEvent → Queue
  | .arrive => { q with next := q.next + 1, waiting := q.waiting ++ [q.next] }
  | .grantHead =>
    match q.waiting with
    | [] => q
    | h :: t => { q with waiting := t, granted := q.granted ++ [h] }
  | .shed t => { q with waiting := q.waiting.filter (fun w => w != t) }

def queueRun (q : Queue) (trace : List QueueEvent) : Queue := trace.foldl queueStep q

/-- 排队的不变量：队里与已取到的号各自递增，已取到的都早于还在排的，所有号都早于下一张。 -/
structure QueueInv (q : Queue) : Prop where
  waitingSorted : q.waiting.Pairwise (· < ·)
  grantedSorted : q.granted.Pairwise (· < ·)
  grantedFirst : ∀ g ∈ q.granted, ∀ w ∈ q.waiting, g < w
  waitingIssued : ∀ w ∈ q.waiting, w < q.next
  grantedIssued : ∀ g ∈ q.granted, g < q.next

theorem queueInv_empty : QueueInv Queue.empty :=
  ⟨List.Pairwise.nil, List.Pairwise.nil, by simp [Queue.empty], by simp [Queue.empty],
    by simp [Queue.empty]⟩

theorem queueStep_inv {q : Queue} (h : QueueInv q) (e : QueueEvent) : QueueInv (queueStep q e) := by
  cases e with
  | arrive =>
    refine ⟨?_, h.grantedSorted, ?_, ?_, ?_⟩
    · simp only [queueStep]
      rw [List.pairwise_append]
      exact ⟨h.waitingSorted, List.pairwise_singleton _ _,
        fun a ha b hb => by simp at hb; subst hb; exact h.waitingIssued a ha⟩
    · intro g hg w hw
      simp only [queueStep, List.mem_append, List.mem_singleton] at hw
      rcases hw with hw | hw
      · exact h.grantedFirst g hg w hw
      · subst hw; exact h.grantedIssued g hg
    · intro w hw
      simp only [queueStep, List.mem_append, List.mem_singleton] at hw
      rcases hw with hw | hw
      · exact Nat.lt_succ_of_lt (h.waitingIssued w hw)
      · subst hw; exact Nat.lt_succ_self _
    · intro g hg
      exact Nat.lt_succ_of_lt (h.grantedIssued g hg)
  | grantHead =>
    rcases h with ⟨hw, hg, hgw, hwi, hgi⟩
    cases hq : q.waiting with
    | nil => simp only [queueStep, hq]; exact ⟨hw, hg, hgw, hwi, hgi⟩
    | cons x t =>
      simp only [queueStep, hq]
      rw [hq] at hw hgw hwi
      have hx : ∀ w ∈ t, x < w := fun w hw' => List.rel_of_pairwise_cons hw hw'
      refine ⟨hw.of_cons, ?_, ?_, ?_, ?_⟩
      · rw [List.pairwise_append]
        exact ⟨hg, List.pairwise_singleton _ _,
          fun a ha b hb => by simp at hb; subst hb; exact hgw a ha b (by simp)⟩
      · intro g hg' w hw'
        simp only [List.mem_append, List.mem_singleton] at hg'
        rcases hg' with hg' | hg'
        · exact hgw g hg' w (List.mem_cons_of_mem _ hw')
        · subst hg'; exact hx w hw'
      · intro w hw'
        exact hwi w (List.mem_cons_of_mem _ hw')
      · intro g hg'
        simp only [List.mem_append, List.mem_singleton] at hg'
        rcases hg' with hg' | hg'
        · exact hgi g hg'
        · subst hg'; exact hwi g (by simp)
  | shed t =>
    refine ⟨h.waitingSorted.filter _, h.grantedSorted, ?_, ?_, h.grantedIssued⟩
    · intro g hg w hw
      simp only [queueStep, List.mem_filter] at hw
      exact h.grantedFirst g hg w hw.1
    · intro w hw
      simp only [queueStep, List.mem_filter] at hw
      exact h.waitingIssued w hw.1

theorem queueRun_inv (q : Queue) (trace : List QueueEvent) (h : QueueInv q) :
    QueueInv (queueRun q trace) := by
  induction trace generalizing q with
  | nil => exact h
  | cons e es ih => exact ih (queueStep q e) (queueStep_inv h e)

/-- 在任意一条轨迹上，取到名额的号严格按拿号的次序：后到的调用从不先取到。 -/
theorem grants_follow_arrival (trace : List QueueEvent) :
    (queueRun Queue.empty trace).granted.Pairwise (· < ·) :=
  (queueRun_inv Queue.empty trace queueInv_empty).grantedSorted


/-!
## 前缀等待：冷前缀先 pre-warm 一次，跟随者等它（§8-40，D37–D39、D41–D43）

### 8-40 前缀等待（形状 1 判定，加上门里的接线）

provider 的缓存条目在第一条响应开始之后才可读。几个调用同时带着同一段冷的共享前缀发到一个端点时，谁也读不到谁的写入，每个都按写价（基础输入价的 1.25 倍）重写同一段字节。端点的门（§8-6）是每个模型调用都经过的一处，所以这里在取号之前加一道等待：一段冷前缀的第一个调用先发一次 pre-warm，只写共享部分；同键后到的调用等它；pre-warm 答、拒、超时或被丢下时，等它的调用一起去取号，各自读缓存。

接口（Rust 一侧还没有写，照此实现；判定与接线分在两个文件，判定的性质由下面的模型证明）：

```rust
// gateway 的 hold 模块（src 下的 hold.rs）—— 纯判定，只读传给它的时刻
pub(crate) const HOLD_MAX: Duration = Duration::from_secs(30);              // D41
pub(crate) struct PrefixKey(B3Hash);                                        // D38
pub(crate) struct Prewarm(u64);       // 一次 pre-warm 的编号；等它的调用认编号，不认键
pub(crate) enum Arrival {
    Go,                                            // 不等，去取号
    Wait { on: Prewarm, until: Instant },          // 等这一次 pre-warm，至多到 until
    Send { prewarm: Prewarm, until: Instant },     // 自己发这一次 pre-warm，再等它
}
pub(crate) struct Holds { /* warm: BTreeMap<PrefixKey, Warm>, warming: BTreeMap<PrefixKey, (Prewarm, Instant)> */ }
struct Warm { until: Instant, by: Option<(Prewarm, PrewarmEnd)> }  // 热到何时；最近放行它的那次 pre-warm 与结局
impl Holds {
    pub(crate) fn arrive(&mut self, key: PrefixKey, now: Instant) -> Arrival;  // 模型的 arrive (some k)
    pub(crate) fn settle(&mut self, prewarm: Prewarm, ended: PrewarmEnd, now: Instant); // 模型的 settle
    pub(crate) fn expire(&mut self, now: Instant);                             // 模型的 tick
    pub(crate) fn waits(&self, prewarm: Prewarm) -> bool;                      // 这次 pre-warm 还挡着人吗
    pub(crate) fn ended(&self, key: &PrefixKey, prewarm: Prewarm) -> Option<PrewarmEnd>; // 它怎样结束的
}

// endpoint 下的 prewarm 模块（endpoint 目录里的 prewarm.rs）—— pre-warm 请求与键（D38、D39、D42）
impl Endpoint {
    /// 这次调用该等的那段前缀：键与要发的 pre-warm；D39 说不等时 `None`。
    pub(super) fn prewarm_for(&self, account: Option<&kernel::ServerLabel>, req: &ModelRequest)
        -> Option<Prewarming>;
    /// 发一次 pre-warm：同 `call` 的那条阻塞的路，请求超时是 `timeout`。
    pub(super) fn prewarm(&self, prewarm: &Prewarming, timeout: Duration) -> Result<ModelReturn, AxError>;
}
pub(super) struct Prewarming { key: PrefixKey, request: ModelRequest<'static>, body: serde_json::Value }
```

`settle` 收下结局只为记账：放行谁不读它（模型的 `settle` 不读 `Ending`），结局记在键的热度旁，被放行的调用醒来用 `ended` 读出自己那一行该写什么。热度至少留 `CACHE_LIFETIME`，远长于 `HOLD_MAX`，所以醒来的调用读得到；读不到时（它醒得比缓存寿命还晚）记作 `timed_out`，因为它至少等满了一次 `HOLD_MAX`。`expire` 同时丢掉已过 `until` 的热度，表的大小以五分钟里用过的键为界。

接线在 `endpoint::permit`：`Holds` 与 `Queue` 住在 `Gate` 的同一把锁里，同一个 `Condvar` 唤醒两种等待者。`Gated::through` 依次做五件事：①清掉上一次调用留下的等待记录（D43）；②`prewarm_for` 求键，`None` 时直接到⑤；③进锁、`expire(now)`、`arrive(key, now)`；④`Wait` 时在 `Condvar` 上等到 `waits(on)` 为假或过了 `until`（过了就再 `expire` 一次），`Send` 时出锁发 pre-warm（D42：取号的期限是 `until`，请求超时取 `timeout_ms` 与 `until` 剩下的较短者，守卫在 `Drop` 里以 `abandoned` `settle`），回来后进锁 `settle`、`notify_all`；发的那个调用的结局：答了是 `answered`，回来时已到 `until`（请求超时或取不到号）是 `timed_out`，其余的失败是 `refused`；⑤照旧 `Gate::admit` 取号。`Gate::admit` 因此多一个上限参数 `bound: Duration`，一张号至多等这么久：真调用传 `QUEUE_WAIT_MAX`，pre-warm 传此刻到 `until` 剩下的时间。

派生检查（与实现同一次改动写下，各自先对一个故意写错的实现变红一次）：
- `hold::tests` 的 proptest `holds_keep_the_lean_properties`：生成器在 0 到 3 的四个键上随机走 `Arrive(Option<key>)`、`Settle(任一在飞的 Prewarm)`、`Tick`，每步时钟前进 0 到 `HOLD_MAX` 的一半（单调，与 `clock` 取 `max` 同义），驱动 `Holds` 的四个方法；测试自己记下每个在等的调用（它拿到的 `Prewarm`）与到达号队的次序，每一步断言：性质 7（每个键在飞的 pre-warm 至多一次，且 `Send` 只在键不热时答）、性质 8（`settle` 之后等它的调用 `waits` 全为假；`expire(now)` 之后每个仍在等的调用 `now < 到达 + HOLD_MAX`）、性质 9（一个在等的调用只在它的 `Prewarm` 被结掉或过期时离开，离开即算到达号队）与性质 10（到达即走的调用按到达次序进号队）。错的实现：对每个冷键的到达都答 `Send`，它让性质 7 变红。
- `endpoint::permit` 的三例，在回环假 provider 上，经一个把名字 `provider.test` 解析到回环端口的 `Transport::detoured` 去够它：回环地址是 `reach::is_local`，D39 第 1 条在那里从不等，所以假 provider 要以一个不在这台电脑上的名字出现。`a_cold_wave_sends_one_prewarm_and_then_every_call`：三条线程带同一段冷前缀同时调同一个端点，记下的交换是一次 pre-warm（`max_tokens` 为 1、只带共享的 system 块、一条 `"."` 消息）再三次真请求；`a_prefix_just_used_is_not_held`：一次调用之后同键的一波三个调用不等、不再 pre-warm；`a_refused_prewarm_releases_every_call`：假 provider 拒 pre-warm（429），三次真调用照样发出、各自成功，每个跟随者的 `prefix_held` 都答 `refused`。
- `runtime::turn` 的 `a_held_attempt_is_recorded_between_its_call_and_its_return`：模型端口答出一次等待时，`prefix_held` 紧跟那次尝试的 `model_called`，在 `model_returned` 之前。

**现状**：issue #85 的端到端验收（替身 provider 上一波三件委派进同一栋楼，经真实的 `RunWorker`，写在 `crates/sprawling/tests/` 下）还没有。`just provider` 的替身在回环端口上听，D39 第 1 条让发往这台电脑的调用从不等，所以它在今天的替身上看不见任何 pre-warm；它要等替身能以一个不在这台电脑上的名字被够到（例如 assembly 接受一个只在测试里用的解析表）。上面 `endpoint::permit` 的三例在门这一层覆盖同样的三种结局。
-/

/-! D42 pre-warm 只带共享部分与一条最小的消息，消息上不打断点，只要一个输出 token

**决定**：pre-warm 是一次普通的阻塞调用（`Endpoint::call`，不走流），用发它的那个调用的端点与账号，请求由 `prewarm_for` 从那个调用的请求造出：`policy`、`segments`、`model`、`tools` 与 `effort` 照抄；`system` 只留 `shared` 指的那几块（kernel D60），`cache` 标记照抄，所以写入落在最后一个共享边界上；`messages` 是一条 user 消息，只有一个文本块 `"."`；`breakpoint` 是 `MessageBreakpoint::Unmarked`；`max_tokens` 是 1，在人写的 overrides 之后写进请求体。#87 让 0 可以表达之后改成 0：provider 页面说 `max_tokens: 0` 的请求不生成任何输出、立刻返回。pre-warm 不写 `model_called`，它的花费与结局写在 `prefix_held` 里（D43）。

**理由**：pre-warm 要写下的正是跟随者要读的那段字节，所以除了必须不同的消息与上限，什么都不改。消息上不打断点，是因为页面说否则缓存条目会以这条占位消息为键，后续请求读不到它；不带对话造的请求本来就得到这个计划（`runtime::prefix::BreakpointPlan`）。`max_tokens` 写在 overrides 之后，是因为一次产出文字的 pre-warm 只多花钱、什么也不多买。

**被否**：①把第一个子 run 的真请求当 pre-warm：它的 Resident 段与任务也写进缓存，跟随者用不上，而且要等它整段 prefill；②流式发 pre-warm：一个只要一个 token 的回答没有可流的东西；③照抄 `max_tokens`：一次 pre-warm 可能生成几千个 token。

**重开参数**：#87 落地，上限改成 0；provider 提供只写缓存、不生成的专门接口。
-/

/-! D43 一次等待是 `prefix_held` 这一行，经模型端口交给回合层写下

**决定**：一次等待是一行 record-only 的 `prefix_held`（载荷与种类见 kernel D61），记在等的那个 run 的名下（与 `model_called` 一样不带 `addr`），紧跟在那次尝试的 `model_called` 之后、在 `model_returned` 或那次调用的失败之前。拒绝原样里可能有 provider 自己写的字，所以它走回合层携带别人内容的那条路（`Carried`），进账前扫一遍凭据的形状。gateway 不持 Ledger 句柄（§7），所以 `Gated` 把这次调用的等待记在自己身上，经模型端口的只读方法 `prefix_held`（kernel D60）交给回合层，回合层在每次尝试返回之后（成功与失败两臂）问一次，有就写；每次调用开始时记录清空，不等的调用答 `None`、不写行。行里有：从到门前到被放行的整数微秒（注入的单调时钟）；等的那次 pre-warm 怎样结束（`answered`、`refused`、`timed_out`、`abandoned`）；发 pre-warm 的那个调用的行另带它的花费（provider 报的四个 token 数与账单额）或它的拒绝原样。`model_returned` 的 `took_us` 从尝试发出算起，包含这段等待，读者减去 `waited_us` 得到对端用的时间。

**理由**：人看见一个子 run 晚出发几秒，要能在它自己的历史里读到为什么；pre-warm 是一次真请求，它的写价要出现在城的历史里，否则这一波的花费少记一份。花费只记在发它的那一行，因为一笔钱只入一次账。

**被否**：①只写进诊断日志：日志不是历史，页面与重放读不到；②挂在 `model_returned` 上作一个可缺席的键（kernel D20 的做法）：调用失败时没有这一行，而等待与 pre-warm 的花费照样发生了；③每个跟随者的行也带一份花费：同一笔钱记 N 次。

**现状**：`storage::attribution` 只读 `model_returned`（`cache_renewed` 也不读），所以 pre-warm 的花费进了历史、还没进归因报告；它与 `cache_renewed` 的花费在同一次改动里进归因。
-/

namespace Hold

/-! D41 一次等待至多 `HOLD_MAX`（30 s），pre-warm 不论怎样结束都放行等它的调用

**决定**：`HOLD_MAX` 是 30 s，从 pre-warm 发出算起；到了这一刻，等它的调用一起被放行（`tick`），这次 pre-warm 从 `warming` 拿掉，它晚到的回答不再放行任何人（`settle` 在键不在飞时什么也不动）。发 pre-warm 的那个调用自己的等待也受这一刻限制：pre-warm 取号以它为 `Gate::admit` 的期限，请求超时取端点 `timeout_ms` 与到这一刻剩下的时间中较短的一个。pre-warm 失败的几种样子——被拒（429 照常经守卫的 `settle` 收窄名额；不收这个上限的面答 400）、超时、取不到号、发它的线程在途中展开（守卫在 `Drop` 里结掉它）——对等它的调用都一样：放行，去取号，成败只看自己的那次请求（`a_held_call_leaves_only_to_be_sent`）。等待的时间不计入 `QUEUE_WAIT_MAX` 与 `timeout_ms`，所以一个调用取到名额之前至多等 `HOLD_MAX` 加 `QUEUE_WAIT_MAX`。

**取消**：门的签名没有取消信号（D20），一个在等的 run 在它的调用返回之后的安全点才看见取消；`HOLD_MAX` 封住这段延迟。

**理由**：跟随者等的是共享部分的一次 prefill，几万 token 的前缀在常见的 provider 上要几秒；30 s 给慢的 provider 留出余量，又短于让人以为子 run 卡住的时间。pre-warm 失败时放行跟随者而不是让它们也失败，这是人定下的：pre-warm 是本城为省钱多发的一次请求，它的失败不是任何一个子 run 的失败。

**被否**：①等到 pre-warm 自己的请求超时（`timeout_ms`，可达几分钟）；②pre-warm 失败后再发一次：拒了一次的 provider 多半也拒第二次，每个跟随者都要再等一轮；③跟随者随 pre-warm 一起失败：违反上面人定下的那一条。

**重开参数**：读数（跟随者多等的出发延迟，按机器档次与 provider 报告）显示 pre-warm 的往返常超过 30 s 时改它；它是一个常量，改它不改形状。
-/

/-- 一次 pre-warm 至多挡等它的调用多久，毫秒（`hold::HOLD_MAX`）。 -/
def HOLD_MAX : Nat := 30000

/-- provider 的缓存寿命，毫秒（`kernel::consts_external::PROMPT_CACHE_TTL_SECS` 乘 1000）：一个键在有调用出发之后热这么久。 -/
def CACHE_LIFETIME : Nat := 300000

/-- 一个在等的调用：到达时拿到的编号、它的前缀键、到达的时刻（`since`，已按 `clock` 取过 `max`）。Rust 不存这张表：在等的调用是在 `Gate` 的 `Condvar` 上等的线程，各自记着它等的那次 pre-warm 的编号（`hold::Prewarm`）；proptest 自己记下它们。 -/
structure Held where
  call : Nat
  key : Nat
  since : Nat
  deriving DecidableEq, Repr

/-- 一次在飞的 pre-warm：它的键与发出的时刻（`hold::Holds` 的 `warming` 表的一行）。 -/
structure Warming where
  key : Nat
  since : Nat
  deriving DecidableEq, Repr

/-- 一个调用怎样走到取号那一步：到达即走（`direct`），或等过一次 pre-warm 后被放行（`released`）。 -/
inductive Route where
  | direct
  | released
  deriving DecidableEq, Repr

/-! D37 等待放在端点的门里、取号之前；冷键的第一个调用自己发 pre-warm，同键后到的调用等同一次

**决定**：`Gated::through` 先问 `Holds`，再 `Gate::admit`。键不热、也没有它的 pre-warm 在飞时，到达的调用发一次 pre-warm 并等它；同键后到的调用等同一次；pre-warm 一结束，等它的调用一起去取号。这两条是人定下的：冷的一波里，跟随者等一次共享部分的 prefill；等待放在端点的名额门旁，fork、日程与多个父 run 由同一条规则覆盖。

**理由**：委派的一波、人连着 fork 几次、日程同时派活进一栋楼的几个房间、两个父 run 同时往一栋楼委派，只在端点的门前相遇；`accounting::worker` 的派发循环只看得见同一个父 run 的委派。等在取号之前而不是队里：一张被挡的号若排在队首，会挡住它后面与这段前缀无关的调用；等在号之前，号队的模型与 `grants_follow_arrival` 不改，没被挡的调用照到达的次序取号（`unheld_keep_arrival_order`）。第一个调用自己也等，是因为门在它出发时不知道会不会有第二个调用来，而它出发之后才来的调用已经赶不上它的写入（写入在它的响应开始之后才可读），那时再发 pre-warm 就是第三份写价。代价：一段冷前缀上只有一个调用时，它多等一次往返，多付一次共享部分的读价（0.1 倍）与一个输出 token。

**被否**：①等第一个真调用的响应开始：门今天收不到「响应已开始」，而且等的是那个调用的整段 prefill，共享部分之外还有它自己的 Resident 段与任务；②在 worker 的派发循环里等：只覆盖委派；③收集窗（冷键的第一个调用先等一小段时间看有没有同伴）：每个冷调用都付这段固定的延迟，而它该多长猜不准；④等在号队里（号照发、被挡的号让后面的先取）：队首被挡时挡住无关的调用，排队的次序也要重证。

**重开参数**：门能在流里看见响应开始（`message_start`）时，重议让第一个真调用当写入者、跟随者等它的响应开始，省掉 pre-warm 这一次往返。
-/

/-- 一个端点的前缀等待（`hold::Holds`，住在 `Gate` 里、与号队共用一把锁）。`clock` 是见过的最晚时刻：Rust 的 `Instant` 不倒退，模型对每个事件的时刻取 `max` 得到同一点。`warm` 是每个键最近一次有调用出发的时刻加 `CACHE_LIFETIME`；`sent` 是调用到达号队的次序，是观察用的影子。 -/
structure Holds where
  clock : Nat
  next : Nat
  warm : List (Nat × Nat)
  warming : List Warming
  held : List Held
  sent : List (Nat × Route)
  deriving Repr

/-- 一次 pre-warm 怎样结束：答了、被拒（含超时与取不到号），或发它的线程在途中展开、守卫在 `Drop` 里结掉它。`settle` 不读它：等它的调用不论哪种结局都被放行（D41）。 -/
inductive Ending where
  | answered
  | refused
  | abandoned
  deriving DecidableEq, Repr

/-! D38 前缀键是账号与 pre-warm 请求体的摘要；共享边界由请求自己说出

**决定**：前缀键 `PrefixKey` 是 BLAKE3（作答账号的 id，接一个零字节，再接 pre-warm 请求体的 JSON 字节）；请求体是 D42 的那条 pre-warm 经 `Endpoint::wire_request` 编出的值。端点不进字节：一个 `Holds` 住在一个端点的 `Gate` 里（§8-6），键只在同一个端点上比。请求体里有端点自己的模型 id（`config.model`）、工具表、到最后一个共享边界为止的 system 块与它们的 `cache_control` 标记、按 offer 编好的思考字段与 effort（D35），以及人写下的 overrides；speed 一进请求体（#71）就进键，每个子 run 自己的 effort（#74）同理。没有命名账号的旧登记，账号那一格是空串。模型里键是一个自然数，只被比较。

共享边界由请求自己说出：`ChatRequest.shared`（kernel D60）说 `system` 的前几块是同一栋楼里每个居民都逐字节带着的。runtime 组回合请求时把它定在 Building 段的边界上，因为 Resident 段由身份与目录拼成（`crates/runtime/src/prefix/segment.rs`），各个子 run 不同。

**理由**：缓存能不能命中由发出去的字节决定，所以键直接取那些字节，而不是列一张字段表：表要与请求体保持一致，每加一个影响缓存的设置就要记得加一格，忘了的那一次，两个缓存不同的调用会共用一次 pre-warm，后一个照样按写价重写。账号进键，是因为 provider 的缓存按账号分开。

**被否**：①字段表（端点、账号、模型、工具摘要、system 摘要、思考、effort、speed）：同一件事的第二份定义，见上；②`ModelRequest.segments` 的前两个哈希加模型：不含工具与思考设置；③gateway 自己数 `system` 里第几个带断点的块：那是 runtime 的布局在 gateway 里的第二份拼写。

**代价**：Anthropic 面上每次调用多编一次 pre-warm 请求体（共享的 system 与工具，不含对话）并算一次摘要；`endpoint::permit::hold_tests` 的 `#[ignore]` 测试 `instrument_prewarm_for` 量它：共享部分约一万 token 时，未优化的测试构建上 p50 约 1.7 ms、p99 约 2.2 ms（x86_64 Windows，16 线程的台式机）；与一次往返相比很小，发布构建上更小。

**重开参数**：某个 provider 的缓存还取决于请求体之外的东西（例如一个请求头）时，那样东西进键。
-/

inductive HoldEvent where
  /-- 一个调用到了门前；`none` 是 D39 说不等的调用。 -/
  | arrive (key : Option Nat) (now : Nat)
  /-- 键 `key` 的那次 pre-warm 结束了。 -/
  | settle (key : Nat) (ending : Ending) (now : Nat)
  /-- 门看一眼时钟（Rust 在每次进锁与每次醒来时调 `Holds::expire`）。 -/
  | tick (now : Nat)

def Holds.empty : Holds := ⟨0, 0, [], [], [], []⟩

def isWarm (s : Holds) (k t : Nat) : Bool := s.warm.any (fun e => e.1 == k && decide (t < e.2))

def isWarming (s : Holds) (k : Nat) : Bool := s.warming.any (fun w => w.key == k)

def rewarm (warm : List (Nat × Nat)) (keys : List Nat) (t : Nat) : List (Nat × Nat) :=
  keys.map (fun k => (k, t + CACHE_LIFETIME)) ++ warm.filter (fun e => !keys.contains e.1)

def expired (t : Nat) (w : Warming) : Bool := decide (w.since + HOLD_MAX ≤ t)

def lapsed (s : Holds) (t : Nat) (h : Held) : Bool :=
  s.warming.any (fun w => w.key == h.key && expired t w)

def goDirect (s : Holds) (t : Nat) : Holds :=
  { s with clock := t, next := s.next + 1, sent := s.sent ++ [(s.next, .direct)] }

def join (s : Holds) (k t : Nat) : Holds :=
  { s with clock := t, next := s.next + 1, held := s.held ++ [⟨s.next, k, t⟩] }

/-! D39 什么时候不等

**决定**：下面任一条成立，调用不进等待，直接取号（模型里的 `arrive none`，或 `isWarm` 那一臂）：
1. 端点不是 Anthropic 面，或 `reach::is_local(base_url)`：只有 Anthropic 面按显式断点缓存并收写价，这台电脑上的服务器的缓存是它自己的 KV。机密楼（`policy.confidential`）的请求也不等：真调用会在门后被拒，而 pre-warm 带的正是这栋楼不许离开这台电脑的字节。
2. 请求说没有共享部分（`SharedPrefix::Unshared`：顾问、OCR、探测与测试造的请求），或 `shared` 指的块数超过 `system` 的长度。
3. 共享部分短于模型的最小可缓存长度：pre-warm 请求体的字节数除以 `kernel::consts_policy::BYTES_PER_TOKEN`（4）小于 `cache_min_tokens`。`cache_min_tokens` 是预置表 `ModelPreset` 的一列，从厂商页面读（页面按模型给 512 到 4,096），行里没有时取 `CACHE_MIN_TOKENS_DEFAULT`（4,096，页面列出的最大值），常量住在 `provider::preset`。
4. 编不出 pre-warm 请求体（`wire_request` 拒）：真调用会以同一个错报出来，等待不替它报。
5. 键是热的：一个带这个键的调用在 `CACHE_LIFETIME`（`kernel::consts_external::PROMPT_CACHE_TTL_SECS`）之内离开过门前去取号，常见的是母 run 自己的那次调用。离开门前就算开始：不论 pre-warm 答了、被拒、超时还是被丢下，被放行的调用都在那一刻出发，所以同一波里接着到的调用不再发第二次 pre-warm。
6. 真调用在门内就会被拒的请求（登记为只读文本的模型收到了图片）：pre-warm 付一次写价，换来的缓存这次调用读不到，所以不等也不写。

**理由**：等待只在能省下写价时才值得它的延迟。1 与 3 之下没有可读的缓存；5 之下缓存已经在，或者这一波已经出发，再挡只会更晚。按字节估 token 对中文偏少（一个汉字三个字节、约一个 token），所以估错的方向是少等，不是白等。缺省取最大值而不是最小值：取小了，一段模型缓存不了的前缀也会被 pre-warm 一次，付的是全价输入。

**被否**：①缺省取页面上最小的 512：见上；②不论长短都等：短前缀的 pre-warm 只花钱不省钱；③把 3 交给 provider（先发 pre-warm，再看 usage 里有没有写）：每段短前缀都多一次往返。

**现状**：预置表还没有一行写下 `cache_min_tokens`：这次改动没有重读各模型的厂商页面，而一个没有出处的数字不进表，所以今天每个模型都用缺省值 4,096，宁可少等。

**重开参数**：provider 在模型列表里给出最小可缓存长度时，这一列改读它；OpenAI 面的自动前缀缓存开始收写价，或它的读价折扣大到值得一次往返时，重议 1。
-/

/-- 到达：键是热的就走并续上热度；有这个键的 pre-warm 在飞就等它；否则自己发一次并等它（D37）。 -/
def arrive (s : Holds) (key : Option Nat) (now : Nat) : Holds :=
  match key with
  | none => goDirect s (max s.clock now)
  | some k =>
    if isWarm s k (max s.clock now) then
      { goDirect s (max s.clock now) with warm := rewarm s.warm [k] (max s.clock now) }
    else if isWarming s k then join s k (max s.clock now)
    else { join s k (max s.clock now) with warming := s.warming ++ [⟨k, max s.clock now⟩] }

/-- 放行等键 `k` 的每个调用：它们此刻出发，所以键从此刻起是热的。 -/
def release (s : Holds) (k t : Nat) : Holds :=
  { s with
    clock := t
    warm := rewarm s.warm [k] t
    warming := s.warming.filter (fun w => w.key != k)
    held := s.held.filter (fun h => h.key != k)
    sent := s.sent ++ (s.held.filter (fun h => h.key == k)).map (fun h => (h.call, Route.released)) }

/-- pre-warm 结束：键还在飞就放行等它的调用；已因超时拿掉的键，晚到的回答什么也不动。 -/
def settle (s : Holds) (k : Nat) (_ : Ending) (now : Nat) : Holds :=
  if isWarming s k then release s k (max s.clock now) else { s with clock := max s.clock now }

/-- 时钟走到 `now`：发出已满 `HOLD_MAX` 的 pre-warm 从 `warming` 拿掉，等它们的调用一起放行（D41）。 -/
def tick (s : Holds) (now : Nat) : Holds :=
  { s with
    clock := max s.clock now
    warm := rewarm s.warm ((s.warming.filter (expired (max s.clock now))).map (·.key)) (max s.clock now)
    warming := s.warming.filter (fun w => !expired (max s.clock now) w)
    held := s.held.filter (fun h => !lapsed s (max s.clock now) h)
    sent := s.sent ++ (s.held.filter (lapsed s (max s.clock now))).map
      (fun h => (h.call, Route.released)) }

def holdStep (s : Holds) : HoldEvent → Holds
  | .arrive key now => arrive s key now
  | .settle k e now => settle s k e now
  | .tick now => tick s now

def holdRun (s : Holds) (trace : List HoldEvent) : Holds := trace.foldl holdStep s

theorem holdRun_cons (s : Holds) (e : HoldEvent) (es : List HoldEvent) :
    holdRun s (e :: es) = holdRun (holdStep s e) es := rfl

/-- 没被挡的调用到达号队的次序。 -/
def unheldOf (sent : List (Nat × Route)) : List Nat :=
  (sent.filter (fun p => p.2 == .direct)).map (·.1)

def unheld (s : Holds) : List Nat := unheldOf s.sent

/-- 前缀等待的不变量：在飞的 pre-warm 键各不相同、都不晚于 `clock`；每个在等的调用都有一次同键、不晚于它到达的 pre-warm 在飞；编号都已发出；每个已到的调用不是在等就是已到号队；没被挡的调用按编号递增到达号队。 -/
structure HoldsInv (s : Holds) : Prop where
  warmingDistinct : (s.warming.map (·.key)).Pairwise (· ≠ ·)
  warmingPast : ∀ w ∈ s.warming, w.since ≤ s.clock
  heldCovered : ∀ h ∈ s.held, ∃ w ∈ s.warming, w.key = h.key ∧ w.since ≤ h.since
  heldIssued : ∀ h ∈ s.held, h.call < s.next
  sentIssued : ∀ p ∈ s.sent, p.1 < s.next
  accounted : ∀ c, c < s.next → (∃ h ∈ s.held, h.call = c) ∨ (∃ r, (c, r) ∈ s.sent)
  unheldSorted : (unheld s).Pairwise (· < ·)

theorem holdsInv_empty : HoldsInv Holds.empty :=
  ⟨List.Pairwise.nil, by simp [Holds.empty], by simp [Holds.empty], by simp [Holds.empty],
    by simp [Holds.empty], by simp [Holds.empty], by simp [Holds.empty, unheld, unheldOf]⟩

theorem not_warming {s : Holds} {k : Nat} (h : isWarming s k = false) :
    ∀ w ∈ s.warming, w.key ≠ k := by
  intro w hw hk
  have : isWarming s k = true := by
    simp only [isWarming, List.any_eq_true, beq_iff_eq]
    exact ⟨w, hw, hk⟩
  simp_all

theorem unheld_released (sent : List (Nat × Route)) (l : List Held) :
    unheldOf (sent ++ l.map (fun h => (h.call, Route.released))) = unheldOf sent := by
  simp [unheldOf, List.filter_append, List.filter_map, Function.comp_def]

theorem unheld_direct (sent : List (Nat × Route)) (c : Nat) :
    unheldOf (sent ++ [(c, Route.direct)]) = unheldOf sent ++ [c] := by
  simp [unheldOf, List.filter_append]


theorem unheld_mem {s : Holds} {c : Nat} (h : c ∈ unheld s) : ∃ r, (c, r) ∈ s.sent := by
  simp only [unheld, unheldOf, List.mem_map, List.mem_filter] at h
  obtain ⟨p, ⟨hp, _⟩, rfl⟩ := h
  exact ⟨p.2, hp⟩

theorem goDirect_inv {s : Holds} (h : HoldsInv s) (t : Nat) (ht : s.clock ≤ t) (warm : List (Nat × Nat)) :
    HoldsInv { goDirect s t with warm := warm } := by
  refine ⟨h.warmingDistinct, ?_, h.heldCovered, ?_, ?_, ?_, ?_⟩
  · intro w hw; exact Nat.le_trans (h.warmingPast w hw) ht
  · intro x hx; exact Nat.lt_succ_of_lt (h.heldIssued x hx)
  · intro p hp
    simp only [goDirect, List.mem_append, List.mem_singleton] at hp
    rcases hp with hp | hp
    · exact Nat.lt_succ_of_lt (h.sentIssued p hp)
    · subst hp; exact Nat.lt_succ_self _
  · intro c hc
    simp only [goDirect] at hc ⊢
    rcases Nat.lt_succ_iff_lt_or_eq.mp hc with hc | hc
    · rcases h.accounted c hc with ⟨x, hx, hxc⟩ | ⟨r, hr⟩
      · exact Or.inl ⟨x, hx, hxc⟩
      · exact Or.inr ⟨r, List.mem_append_left _ hr⟩
    · subst hc; exact Or.inr ⟨.direct, by simp⟩
  · show (unheldOf (s.sent ++ [(s.next, Route.direct)])).Pairwise (· < ·)
    rw [unheld_direct, List.pairwise_append]
    refine ⟨h.unheldSorted, List.pairwise_singleton _ _, ?_⟩
    intro a ha b hb
    simp only [List.mem_singleton] at hb
    subst hb
    obtain ⟨r, hr⟩ := unheld_mem ha
    exact h.sentIssued _ hr


/-- 加入一个调用到等候里，`warming` 换成一张包含原表、键各不相同、都不晚于 `t` 的表，且这张表里有它的键。 -/
theorem join_inv {s : Holds} (h : HoldsInv s) (k t : Nat)
    (warming : List Warming) (hsub : ∀ w ∈ s.warming, w ∈ warming)
    (hd : (warming.map (·.key)).Pairwise (· ≠ ·)) (hp : ∀ w ∈ warming, w.since ≤ t)
    (hk : ∃ w ∈ warming, w.key = k ∧ w.since ≤ t) :
    HoldsInv { join s k t with warming := warming } := by
  refine ⟨hd, hp, ?_, ?_, ?_, ?_, h.unheldSorted⟩
  · intro x hx
    simp only [join, List.mem_append, List.mem_singleton] at hx
    rcases hx with hx | hx
    · obtain ⟨w, hw, hwk, hws⟩ := h.heldCovered x hx
      exact ⟨w, hsub w hw, hwk, hws⟩
    · subst hx; exact hk
  · intro x hx
    simp only [join, List.mem_append, List.mem_singleton] at hx
    rcases hx with hx | hx
    · exact Nat.lt_succ_of_lt (h.heldIssued x hx)
    · subst hx; exact Nat.lt_succ_self _
  · intro p hp'
    exact Nat.lt_succ_of_lt (h.sentIssued p hp')
  · intro c hc
    simp only [join] at hc ⊢
    rcases Nat.lt_succ_iff_lt_or_eq.mp hc with hc | hc
    · rcases h.accounted c hc with ⟨x, hx, hxc⟩ | ⟨r, hr⟩
      · exact Or.inl ⟨x, List.mem_append_left _ hx, hxc⟩
      · exact Or.inr ⟨r, hr⟩
    · subst hc; exact Or.inl ⟨⟨s.next, k, t⟩, by simp, rfl⟩

theorem arrive_inv {s : Holds} (h : HoldsInv s) (key : Option Nat) (now : Nat) :
    HoldsInv (arrive s key now) := by
  have ht : s.clock ≤ max s.clock now := Nat.le_max_left _ _
  cases key with
  | none => exact goDirect_inv h _ ht s.warm
  | some k =>
    simp only [arrive]
    split
    · exact goDirect_inv h _ ht _
    · split
      · rename_i hw
        simp only [isWarming, List.any_eq_true, beq_iff_eq] at hw
        obtain ⟨w, hw, hwk⟩ := hw
        exact join_inv h k _ s.warming (fun _ hx => hx) h.warmingDistinct
          (fun x hx => Nat.le_trans (h.warmingPast x hx) ht)
          ⟨w, hw, hwk, Nat.le_trans (h.warmingPast w hw) ht⟩
      · rename_i hw
        have hnot := not_warming (Bool.eq_false_iff.mpr hw)
        refine join_inv h k _ _ (fun _ hx => List.mem_append_left _ hx) ?_ ?_ ⟨⟨k, _⟩, by simp, rfl, Nat.le_refl _⟩
        · rw [List.map_append, List.pairwise_append]
          refine ⟨h.warmingDistinct, List.pairwise_singleton _ _, ?_⟩
          intro a ha b hb
          simp only [List.map_cons, List.map_nil, List.mem_singleton] at hb
          subst hb
          obtain ⟨w, hw, rfl⟩ := List.mem_map.mp ha
          exact hnot w hw
        · intro x hx
          simp only [List.mem_append, List.mem_singleton] at hx
          rcases hx with hx | hx
          · exact Nat.le_trans (h.warmingPast x hx) ht
          · subst hx; exact Nat.le_refl _

/-- 把等候里 `p` 选中的调用放行到 `sent`，`warming` 换成原表的一张子表，留下的调用仍各有一次在飞的 pre-warm。 -/
theorem move_inv {s : Holds} (h : HoldsInv s) (t : Nat) (ht : s.clock ≤ t) (p : Held → Bool)
    (keep : Warming → Bool) (warm : List (Nat × Nat))
    (hk : ∀ x ∈ s.held, p x = false → ∃ w ∈ s.warming, keep w = true ∧ w.key = x.key ∧ w.since ≤ x.since) :
    HoldsInv
      { s with
        clock := t
        warm := warm
        warming := s.warming.filter keep
        held := s.held.filter (fun x => !p x)
        sent := s.sent ++ (s.held.filter p).map (fun x => (x.call, Route.released)) } := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · exact h.warmingDistinct.sublist (List.filter_sublist.map _)
  · intro w hw
    exact Nat.le_trans (h.warmingPast w (List.mem_filter.mp hw).1) ht
  · intro x hx
    have hx' := List.mem_filter.mp hx
    obtain ⟨w, hw, hkeep, hwk, hws⟩ := hk x hx'.1 (by simpa using hx'.2)
    exact ⟨w, List.mem_filter.mpr ⟨hw, hkeep⟩, hwk, hws⟩
  · intro x hx
    exact h.heldIssued x (List.mem_filter.mp hx).1
  · intro q hq
    simp only [List.mem_append, List.mem_map, List.mem_filter] at hq
    rcases hq with hq | ⟨x, ⟨hx, _⟩, rfl⟩
    · exact h.sentIssued q hq
    · exact h.heldIssued x hx
  · intro c hc
    rcases h.accounted c hc with ⟨x, hx, hxc⟩ | ⟨r, hr⟩
    · cases hpx : p x
      · exact Or.inl ⟨x, List.mem_filter.mpr ⟨hx, by simp [hpx]⟩, hxc⟩
      · refine Or.inr ⟨.released, List.mem_append_right _ ?_⟩
        exact List.mem_map.mpr ⟨x, List.mem_filter.mpr ⟨hx, hpx⟩, by rw [hxc]⟩
    · exact Or.inr ⟨r, List.mem_append_left _ hr⟩
  · show (unheldOf (s.sent ++ (s.held.filter p).map (fun x => (x.call, Route.released)))).Pairwise (· < ·)
    rw [unheld_released]
    exact h.unheldSorted

theorem settle_inv {s : Holds} (h : HoldsInv s) (k : Nat) (e : Ending) (now : Nat) :
    HoldsInv (settle s k e now) := by
  have ht : s.clock ≤ max s.clock now := Nat.le_max_left _ _
  simp only [settle]
  split
  · simp only [release]
    refine move_inv h _ ht (fun x => x.key == k) (fun w => w.key != k) _ ?_
    intro x hx hp
    obtain ⟨w, hw, hwk, hws⟩ := h.heldCovered x hx
    refine ⟨w, hw, ?_, hwk, hws⟩
    simp only [beq_eq_false_iff_ne, ne_eq] at hp
    simp [hwk, hp]
  · exact ⟨h.warmingDistinct, fun w hw => Nat.le_trans (h.warmingPast w hw) ht, h.heldCovered,
      h.heldIssued, h.sentIssued, h.accounted, h.unheldSorted⟩

theorem tick_inv {s : Holds} (h : HoldsInv s) (now : Nat) : HoldsInv (tick s now) := by
  have ht : s.clock ≤ max s.clock now := Nat.le_max_left _ _
  simp only [tick]
  have key := move_inv h _ ht (lapsed s (max s.clock now)) (fun w => !expired (max s.clock now) w)
    (rewarm s.warm ((s.warming.filter (expired (max s.clock now))).map (·.key)) (max s.clock now)) ?_
  · simpa using key
  · intro x hx hp
    obtain ⟨w, hw, hwk, hws⟩ := h.heldCovered x hx
    refine ⟨w, hw, ?_, hwk, hws⟩
    cases he : expired (max s.clock now) w
    · rfl
    · have : lapsed s (max s.clock now) x = true := by
        simp only [lapsed, List.any_eq_true, Bool.and_eq_true, beq_iff_eq]
        exact ⟨w, hw, hwk, he⟩
      simp_all

theorem holdStep_inv {s : Holds} (h : HoldsInv s) (e : HoldEvent) : HoldsInv (holdStep s e) := by
  cases e with
  | arrive key now => exact arrive_inv h key now
  | settle k e now => exact settle_inv h k e now
  | tick now => exact tick_inv h now

theorem holdRun_inv (s : Holds) (trace : List HoldEvent) (h : HoldsInv s) :
    HoldsInv (holdRun s trace) := by
  induction trace generalizing s with
  | nil => exact h
  | cons e es ih => rw [holdRun_cons]; exact ih _ (holdStep_inv h e)


/-- 性质 7：任意轨迹上，一个键同一时刻至多一次 pre-warm 在飞。 -/
theorem one_prewarm_per_key (trace : List HoldEvent) :
    ((holdRun Holds.empty trace).warming.map (·.key)).Nodup :=
  (holdRun_inv Holds.empty trace holdsInv_empty).warmingDistinct

/-- 一次 pre-warm 只为冷键发出：一次到达让键开始在飞，它在到达的那一刻不热。 -/
theorem a_prewarm_starts_only_for_a_cold_key (s : Holds) (k now : Nat)
    (before : isWarming s k = false) (after : isWarming (holdStep s (.arrive (some k) now)) k = true) :
    isWarm s k (max s.clock now) = false := by
  cases hw : isWarm s k (max s.clock now)
  · rfl
  · simp only [holdStep, arrive, hw, ite_true, goDirect] at after
    exact absurd (show isWarming s k = true from after) (by simp [before])

/-- 性质 8 的前一半：pre-warm 一结束（不论哪种 `Ending`），等它的调用一个不剩。 -/
theorem a_settled_prewarm_releases_every_call_on_it {s : Holds} (h : HoldsInv s) (k : Nat)
    (e : Ending) (now : Nat) : ∀ x ∈ (holdStep s (.settle k e now)).held, x.key ≠ k := by
  intro x hx
  simp only [holdStep, settle] at hx
  split at hx
  · simp only [release, List.mem_filter, bne_iff_ne, ne_eq] at hx
    exact hx.2
  · rename_i hw
    obtain ⟨w, hw', hwk, _⟩ := h.heldCovered x hx
    intro hxk
    exact not_warming (Bool.eq_false_iff.mpr hw) w hw' (hwk.trans hxk)

/-- 性质 9：一个在等的调用离开等待的唯一去处是号队——任何事件，包括任何一种 `Ending` 的 `settle`，都不让它失败。 -/
theorem a_held_call_leaves_only_to_be_sent (s : Holds) (e : HoldEvent) :
    ∀ x ∈ s.held, x ∈ (holdStep s e).held ∨ (x.call, Route.released) ∈ (holdStep s e).sent := by
  intro x hx
  cases e with
  | arrive key now =>
    left
    cases key with
    | none => exact hx
    | some k =>
      simp only [holdStep, arrive]
      split
      · exact hx
      · split
        · exact List.mem_append_left _ hx
        · exact List.mem_append_left _ hx
  | settle k e now =>
    simp only [holdStep, settle]
    split
    · simp only [release]
      cases hk : x.key == k
      · left
        exact List.mem_filter.mpr ⟨hx, by simpa using hk⟩
      · right
        exact List.mem_append_right _ (List.mem_map.mpr ⟨x, List.mem_filter.mpr ⟨hx, hk⟩, rfl⟩)
    · left; exact hx
  | tick now =>
    simp only [holdStep, tick]
    cases hl : lapsed s (max s.clock now) x
    · left; exact List.mem_filter.mpr ⟨hx, by simp [hl]⟩
    · right
      exact List.mem_append_right _ (List.mem_map.mpr ⟨x, List.mem_filter.mpr ⟨hx, hl⟩, rfl⟩)

/-- 性质 8 的后一半：门看过时钟之后，每个还在等的调用到达不足 `HOLD_MAX`。 -/
theorem held_within_bound {s : Holds} (h : HoldsInv s) (now : Nat) :
    ∀ x ∈ (holdStep s (.tick now)).held, (holdStep s (.tick now)).clock < x.since + HOLD_MAX := by
  intro x hx
  simp only [holdStep, tick, List.mem_filter, Bool.not_eq_true'] at hx ⊢
  obtain ⟨hx, hl⟩ := hx
  obtain ⟨w, hw, hwk, hws⟩ := h.heldCovered x hx
  have he : expired (max s.clock now) w = false := by
    cases he : expired (max s.clock now) w
    · rfl
    · have : lapsed s (max s.clock now) x = true := by
        simp only [lapsed, List.any_eq_true, Bool.and_eq_true, beq_iff_eq]
        exact ⟨w, hw, hwk, he⟩
      simp_all
  simp only [expired, decide_eq_false_iff_not, Nat.not_le] at he
  omega

/-- 没有调用丢在门前：任意轨迹上，每个到过的调用不是在等，就是已到号队。 -/
theorem every_call_is_held_or_sent (trace : List HoldEvent) :
    ∀ c, c < (holdRun Holds.empty trace).next →
      (∃ x ∈ (holdRun Holds.empty trace).held, x.call = c) ∨
      (∃ r, (c, r) ∈ (holdRun Holds.empty trace).sent) :=
  (holdRun_inv Holds.empty trace holdsInv_empty).accounted

/-- 性质 10：没被挡的调用按到达的次序到达号队；号队之后的次序由 `grants_follow_arrival` 守住，所以它们也按到达的次序取到名额。 -/
theorem unheld_keep_arrival_order (trace : List HoldEvent) :
    (unheld (holdRun Holds.empty trace)).Pairwise (· < ·) :=
  (holdRun_inv Holds.empty trace holdsInv_empty).unheldSorted

/-- 可实现的正常路径：一段冷前缀上的一波三个调用等一次 pre-warm，答了之后一起出发；之后同键的调用与不等的调用到达即走。 -/
example :
    let s := holdRun Holds.empty
      [.arrive (some 7) 0, .arrive (some 7) 1, .arrive (some 7) 2, .settle 7 .answered 5,
       .arrive (some 7) 6, .arrive none 7]
    s.sent = [(0, .released), (1, .released), (2, .released), (3, .direct), (4, .direct)] ∧
      s.held = [] ∧ s.warming = [] := by
  decide

/-- pre-warm 超时：满 `HOLD_MAX` 的那一眼放行两个调用，晚到的拒绝什么也不动，同一波接着到的调用不发第二次 pre-warm。 -/
example :
    let s := holdRun Holds.empty
      [.arrive (some 7) 0, .arrive (some 7) 1, .tick 29999, .tick 30000, .settle 7 .refused 30001,
       .arrive (some 7) 30002]
    s.sent = [(0, .released), (1, .released), (2, .direct)] ∧ s.warming = [] := by
  decide

end Hold

end Gateway.Concurrency
