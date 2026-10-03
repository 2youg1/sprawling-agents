-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# playback 的投影：读者读不到的行不流进任何派生表，关键时刻只在真实结束事件上闭合

规定 `crates/accounting/src/playback/project.rs`、`crates/accounting/src/playback/reader.rs`、`crates/accounting/src/playback/links.rs`、`crates/accounting/src/playback/traced.rs` 与 `crates/accounting/src/trace.rs`（`crates/accounting/Spec.lean` §8-12、§8-25）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

**读界。** 一行碰到的每一栋楼都由读者判一次，答案复用 `kernel::ReadVerdict` 的三臂：`Open`、`Confidential`、`RulesUnreadable`。只有碰到的楼全是 `Open` 的行可见；另外两臂一律关闭，所以规则读不了不会朝宽的一侧失败。一行「碰到」哪些楼，Rust 从信封地址、它所在 run 的房间、它关闭的那一对的打开行、以及载荷里以已知楼开头的地址求出；本模型把这个集合当作行的一个字段。

**凭据扫描。** Rust 里碰到的楼全是 `Open` 的行，还要过 `kernel::secret::scan`：扫出凭据的行同样隐去，只记一条 `credential` 计数（`playback::project` 的 `Fate::Credential`）。这一条读的是行的内容，而前几节的 `readable` 只读 `touches`，所以 `hidden_content_never_reaches_the_bundle` 说的是「关闭的楼」这一半；凭据那一半在末节「导出的懒扫描」里：扫描是一个函数，一行的三种命运 `Fate` 都在模型里，范围内带凭据的行记作 `Credential`（`a_credential_line_is_withheld`），懒扫描与全扫描交出的命运逐条相同（`lazy_scan_equals_full_scan`）。「什么像凭据」的权威是 `kernel::secret::scan`。

**派生表只读可见行。** 事件表、run 表、关键时刻、消息、费用与 checkpoint 表都是 `derive` 在可见行上的值；被隐去的行只留下条数（以及 Rust 里列明的楼名与种类计数，这些是明示的元数据披露）。`hidden_content_never_reaches_the_bundle` 陈述的是：把被隐去的行改成任何别的内容（只要它仍被隐去、seq 不变），bundle 一字不变——所以被隐去的内容没有任何一条路流进 bundle。

**真实关闭的单调性。** 一个关键时刻（一个 run、一次审批、一个 PR、一封信）在 cutoff `c` 闭合，当且仅当某一行 seq ≤ c 且它是这个键的关闭事件。闭合只看关闭事件，不看选择窗口的右端：`closedAt` 的参数里没有窗口，这是定义的形状，不另立定理；在 cutoff c 闭合的键，在更晚的 cutoff、在追加了行的账本上仍闭合（`closure_is_monotone`、`closure_survives_appends`）。前缀扩展只对这些已真实闭合的对象成立：还开着的对象在更晚的 cutoff 可能闭合，那是变化而不是矛盾。

**调用的耗时。** 工具调用与模型调用的 `took` 是同一条规则：两端的时刻都量过、答复不是城补写的、答复不早于调用，才给出两者之差；其余一律是 unknown，从不是零（`took_is_measured`、`an_unmeasured_end_is_unknown`）。一条答复配哪一次调用是 Rust 的配对规则（工具调用按 id，模型调用按 `views::rounds::Attempts`），本模型把配好的两端当作输入。

**提交的证据只读到它的宣告行。** 一个提交的证据是宣告它的那一行 `s` 与之前的行折出的值；`s` 在 cutoff 以内时，cutoff 之后追加的行，不论内容是什么、能不能通过审计，都改变不了它（`evidence_ignores_lines_after_the_cutoff`）。Rust 里 `trace::History` 逐行折到这一行时问视图，`trace::trace_through` 只读 seq 小于这一行的行。

本模型不是 Rust 实现的证明；两者的一致由 `accounting::playback::tests` 在同一组场景上的比较守住。
-/

namespace Accounting.Playback.Project

/-- `kernel::ReadVerdict` 的三臂。 -/
inductive Verdict where
  | open
  | confidential
  | rulesUnreadable
  deriving Repr, DecidableEq

/-- 一行：seq、它碰到的楼、它关闭的关键时刻键（若有）、以及内容（抽象成一个数）。 -/
structure Line where
  seq : Nat
  touches : List Nat
  closes : Option Nat
  content : Nat
  deriving Repr, DecidableEq

/-- 读者对一行的判断：碰到的每一栋楼都 `Open` 才可见。 -/
def readable (verdict : Nat → Verdict) (l : Line) : Bool :=
  l.touches.all (fun b => verdict b == .open)

def visible (verdict : Nat → Verdict) (lines : List Line) : List Line :=
  lines.filter (readable verdict)

def hiddenCount (verdict : Nat → Verdict) (lines : List Line) : Nat :=
  (lines.filter (fun l => !readable verdict l)).length

/-- bundle：可见行、它们的派生表、被隐去的条数。 -/
structure Bundle (α : Type) where
  events : List Line
  derived : α
  withheld : Nat

def bundle {α : Type} (derive : List Line → α) (verdict : Nat → Verdict) (lines : List Line) :
    Bundle α :=
  let seen := visible verdict lines
  { events := seen, derived := derive seen, withheld := hiddenCount verdict lines }

/-! ## 读者读不到的行不流进派生表 -/

theorem every_event_is_readable {α : Type} (derive : List Line → α) (verdict : Nat → Verdict)
    (lines : List Line) : ∀ l ∈ (bundle derive verdict lines).events, readable verdict l = true := by
  intro l hl
  simp [bundle, visible, List.mem_filter] at hl
  exact hl.2

/-- 一次改写只动被隐去的行，并让它仍被隐去。 -/
def TouchesOnlyHidden (verdict : Nat → Verdict) (g : Line → Line) : Prop :=
  ∀ l, (readable verdict l = true → g l = l) ∧
    (readable verdict l = false → readable verdict (g l) = false)

theorem visible_ignores_hidden (verdict : Nat → Verdict) (g : Line → Line)
    (hg : TouchesOnlyHidden verdict g) (lines : List Line) :
    visible verdict (lines.map g) = visible verdict lines := by
  induction lines with
  | nil => rfl
  | cons l rest ih =>
    cases hr : readable verdict l with
    | true =>
      have hl : g l = l := (hg l).1 hr
      simp [visible, hl, hr] at ih ⊢
      exact ih
    | false =>
      have hgl : readable verdict (g l) = false := (hg l).2 hr
      simp [visible, hgl, hr] at ih ⊢
      exact ih

theorem hidden_count_ignores_hidden (verdict : Nat → Verdict) (g : Line → Line)
    (hg : TouchesOnlyHidden verdict g) (lines : List Line) :
    hiddenCount verdict (lines.map g) = hiddenCount verdict lines := by
  induction lines with
  | nil => rfl
  | cons l rest ih =>
    cases hr : readable verdict l with
    | true =>
      have hl : g l = l := (hg l).1 hr
      simp [hiddenCount, hl, hr] at ih ⊢
      exact ih
    | false =>
      have hgl : readable verdict (g l) = false := (hg l).2 hr
      simp [hiddenCount, hgl, hr] at ih ⊢
      exact ih

theorem hidden_content_never_reaches_the_bundle {α : Type} (derive : List Line → α)
    (verdict : Nat → Verdict) (g : Line → Line) (hg : TouchesOnlyHidden verdict g)
    (lines : List Line) :
    bundle derive verdict (lines.map g) = bundle derive verdict lines := by
  simp only [bundle, visible_ignores_hidden verdict g hg, hidden_count_ignores_hidden verdict g hg]

/-- 实现可达：一栋机密楼的行被隐去，公开楼的行留下，隐去条数是 1。 -/
theorem a_confidential_line_is_counted_not_shown :
    let verdict : Nat → Verdict := fun b => if b = 1 then .confidential else .open
    let lines := [⟨0, [0], none, 7⟩, ⟨1, [0, 1], none, 8⟩, ⟨2, [], none, 9⟩]
    (bundle (fun seen => seen.length) verdict lines).withheld = 1 ∧
      (bundle (fun seen => seen.length) verdict lines).derived = 2 := by
  decide

/-! ## 真实关闭的单调性 -/

/-- 键 `key` 在 cutoff `c` 已闭合：某一行 seq ≤ c 且是它的关闭事件。 -/
def closedAt (key c : Nat) (lines : List Line) : Bool :=
  lines.any (fun l => decide (l.seq ≤ c) && l.closes == some key)

theorem closure_is_monotone (key c c' : Nat) (lines : List Line) (hle : c ≤ c')
    (h : closedAt key c lines = true) : closedAt key c' lines = true := by
  simp only [closedAt, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h ⊢
  obtain ⟨l, hl, hs, hk⟩ := h
  exact ⟨l, hl, Nat.le_trans hs hle, hk⟩

theorem closure_survives_appends (key c : Nat) (lines more : List Line)
    (h : closedAt key c lines = true) : closedAt key c (lines ++ more) = true := by
  simp only [closedAt] at h ⊢
  rw [List.any_append, h, Bool.true_or]

/-! 窗口的右端不是关闭事件：一个在窗口内打开、到 cutoff 仍没有关闭行的键没有闭合，不论窗口在哪里结束。`closedAt` 不读选择，它的参数里没有窗口。 -/

/-! ## 调用的耗时：量出来的才给，其余是 unknown -/

/-- 配好的一次调用：调用行与答复行各自的时刻（`EventRecord::moment`，没量过或那一端不可见为 `none`），以及答复是不是城在重启后补写的（`E_TOOL_OUTCOME_UNKNOWN`；模型的答复从不是）。 -/
structure Timed where
  called : Option Nat
  answered : Option Nat
  supplied : Bool
  deriving Repr, DecidableEq

/-- `took`：两端都量过、答复不是补写、答复不早于调用时是两者之差（Rust 的 `{"measured": …}`），其余是 `none`（Rust 的 `"unknown"`）。 -/
def took (c : Timed) : Option Nat :=
  match c.called, c.answered with
  | some a, some b => if c.supplied = false ∧ a ≤ b then some (b - a) else none
  | _, _ => none

/-- 给出的耗时都是量出来的：两端都有时刻，答复不是补写，答复不早于调用，值是两者之差。 -/
theorem took_is_measured (c : Timed) (d : Nat) (h : took c = some d) :
    ∃ a b, c.called = some a ∧ c.answered = some b ∧ c.supplied = false ∧ a ≤ b ∧ d = b - a := by
  obtain ⟨called, answered, supplied⟩ := c
  cases called with
  | none => simp [took] at h
  | some a =>
    cases answered with
    | none => simp [took] at h
    | some b =>
      simp only [took] at h
      split at h
      · rename_i hold
        exact ⟨a, b, rfl, rfl, hold.1, hold.2, (Option.some.inj h).symm⟩
      · exact absurd h (by simp)

/-- 少一端的时刻就没有耗时：旧版本的行、没有答复、一端被隐去，都不会被画成零。 -/
theorem an_unmeasured_end_is_unknown (c : Timed) (h : c.called = none ∨ c.answered = none) :
    took c = none := by
  obtain ⟨called, answered, supplied⟩ := c
  rcases h with h | h
  · simp only at h
    subst h
    cases answered <;> rfl
  · simp only at h
    subst h
    cases called <;> rfl

/-- 实现可达：量过的一对给出差值，补写的答复与没有答复的调用都是 unknown。 -/
theorem a_measured_call_is_timed_and_the_rest_are_not :
    took ⟨some 2000, some 2350, false⟩ = some 350 ∧ took ⟨some 3000, some 9000, true⟩ = none ∧
      took ⟨some 9500, none, false⟩ = none := by
  decide

/-! ## 提交的证据只读到它的宣告行 -/

/-- 一个提交的证据：宣告它的那一行 `s` 与之前的行折出的值。 -/
def evidenceAt {α : Type} (fold : List Line → α) (s : Nat) (lines : List Line) : α :=
  fold (lines.filter (fun l => decide (l.seq ≤ s)))

/-- 宣告行在 cutoff 以内时，cutoff 之后追加的行改变不了证据，所以复核时这个提交不会因为后来的历史变成 `unread`。 -/
theorem evidence_ignores_lines_after_the_cutoff {α : Type} (fold : List Line → α) (s c : Nat)
    (lines more : List Line) (hs : s ≤ c) (later : ∀ l ∈ more, c < l.seq) :
    evidenceAt fold s (lines ++ more) = evidenceAt fold s lines := by
  have none : more.filter (fun l => decide (l.seq ≤ s)) = [] := by
    rw [List.filter_eq_nil_iff]
    intro l hl
    have := later l hl
    simp only [decide_eq_true_eq]
    omega
  simp [evidenceAt, List.filter_append, none]

/-! ## 导出的懒扫描：范围外的行只在有人要它的命运时才扫（D47）

`playback::project::apply` 今天对每一条碰到的楼全是 `Open` 的行跑 `kernel::secret::scan`，范围内外都扫，因为一行的命运（`Fate`：`Shown`、`Closed`、`Credential`）不止决定它自己进不进 bundle，还流进三处：`remember()`（可见的 `run_started` 给出 run 的 policy；不可见的 `tool_called` 记进 `hidden_calls`，一个提交的 trace 可能点它的名），`links.note` 的配对（一对的另一端是否可见，决定 bundle 写 `Withheld` 还是 `Outside`，以及调用的名字与耗时），以及范围外但配了对的行进 `context`。40 万行的城上，导出于是不论选几行都付约 25 s 的全史扫描（`crates/accounting/Spec.lean` §8-25 的读数）。

**D47：懒扫描。** 范围外、楼全 `Open` 的行先只记下它在段文件里的位置，不扫；只有三种情况才读回它的字节去扫：它在范围内；它的种类要 `remember()`（`run_started`、`tool_called`）；有一条范围内的行与它同属一个配对键（同一次调用、同一个关键时刻、同一封信），不论那条范围内的行在它之前还是之后。楼不全 `Open` 的行不必扫：它的命运已是 `Closed`。本节证明，对任意历史、任意扫描函数，懒扫描交出的三样东西——范围内每行的命运（也就是 bundle 的 events、withheld 的楼与 credential 计数）、`remember()` 读到的命运、配对两端的命运——与全扫描逐条相同（`lazy_scan_equals_full_scan`），而且懒扫描从不读一条它没选去扫的行（`lazy_scan_reads_only_scanned`），扫的行数不多于全扫描（`lazy_scans_no_more`）。

本模型把「这一行与哪一行配对」压成一个键（`Line.key`），一行最多一个；Rust 里一行可以碰到几个键（`links::Touch`），这时只要其中一个键有范围内的成员就得扫，模型的论证逐键照搬。次序不在模型里：配对键是全史上的关系，而 Rust 是按 seq 单遍走的，所以范围外的行先于与它配对的范围内行出现时，Rust 要在后者到来时按记下的位置读回前者去扫，并补上 `links.note` 当时因为不知道可见性而没有写的字段（`asked` 的名字与时刻、信的发信人）——这是实现要守的，派生检查专门造这种次序。

落选：给扫描器提速而照旧全扫（例如 `hex_run::is_labelled_hex_secret` 把字段名判定提前到熵计算之前）——它同样要做，却只把 25 s 按常数缩小，小选择仍付全史的代价；直接跳过范围外的行——一条配了对的范围外凭据行会被当作 `Shown`，它的调用名字与耗时漏进 bundle，见 `forgetting_a_pairing_leaks`。重新打开的参数：一条行的命运开始取决于它与范围内行配对之外的关系（例如 policy 要读范围外每条 `run_policy_changed`），那时那种关系要加进 `scanned`。三个平台相同：模型只用 seq 与段内偏移（整数），读回一行是 `storage` 的按偏移读，在 Windows、macOS、Linux 上同一个接口。

**派生检查的规格（待实现，`accounting::playback::tests`）。** proptest 生成历史：每行随机取种类（`tool_called`、`tool_result`、`run_started`、其他）、配对键（三个里取一个或没有）、碰到的楼是否全 `Open`、载荷里是否带一个凭据形状的值；再随机取一个 seq 区间作选择。用今天的全扫描实现（在 `#[cfg(test)]` 里保留为参照）与懒扫描各导出一次，比较整份 `Document` 相等；另数懒扫描调了几次 `kernel::secret::scan`，不多于全扫描。生成器必须覆盖「范围外的凭据行先于与它配对的范围内行」这一种次序。H10 的那条：范围内一条带凭据的行被隐去、`withheld.credential = "1"`，是 `a_credential_line_is_withheld` 的 Rust 版，作为一条固定向量。**必须变红的坏实现**：`forgetfulScanned`，只扫范围内与要 `remember()` 的行、忘了配对——上面的整份比较在 `forgetting_a_pairing_leaks` 那段历史上变红。 -/

/-- 懒扫描模型里的一行：seq、在不在选择里、碰到的楼是否全 `Open`（`readable`）、配对键、种类是否要 `remember()`。凭据扫描是行之外的一个函数（`kernel::secret::scan` 读行的字节），所以「读没读一行」是「调没调这个函数」。 -/
structure Scanned where
  seq : Nat
  inRange : Bool
  open_ : Bool
  key : Option Nat
  remembered : Bool
  deriving Repr, DecidableEq

/-- `playback::project` 的 `Fate`。 -/
inductive Fate where
  | shown
  | closed
  | credential
  deriving Repr, DecidableEq

/-- 全扫描：楼全 `Open` 的每一行都扫。 -/
def fullFate (scan : Scanned → Bool) (l : Scanned) : Fate :=
  if !l.open_ then .closed else if scan l then .credential else .shown

/-- 有一条范围内的行与它同一个配对键。 -/
def paired (lines : List Scanned) (l : Scanned) : Bool :=
  l.key.isSome && lines.any (fun m => m.inRange && m.key == l.key)

/-- D47：懒扫描选去扫的行。 -/
def scanned (lines : List Scanned) (l : Scanned) : Bool :=
  l.inRange || l.remembered || paired lines l

/-- 懒扫描的命运：没选去扫的行不调 `scan`，被当作 `Shown`——若有谁读到它，它就会漏出去，所以下面要证明没有谁读到。 -/
def lazyFate (pick : List Scanned → Scanned → Bool) (scan : Scanned → Bool) (lines : List Scanned)
    (l : Scanned) : Fate :=
  if !l.open_ then .closed else if pick lines l then (if scan l then .credential else .shown)
  else .shown

/-- 导出读到的命运：范围内每一行的（events、withheld、credential 计数），`remember()` 读的，配对两端的。 -/
structure Read where
  inRange : List (Nat × Fate)
  remembered : List (Nat × Fate)
  pairing : List (Nat × Fate)
  deriving Repr, DecidableEq

def readWith (fate : Scanned → Fate) (lines : List Scanned) : Read :=
  { inRange := (lines.filter (·.inRange)).map fun l => (l.seq, fate l)
    remembered := (lines.filter (·.remembered)).map fun l => (l.seq, fate l)
    pairing := (lines.filter (paired lines)).map fun l => (l.seq, fate l) }

def fullScan (scan : Scanned → Bool) (lines : List Scanned) : Read := readWith (fullFate scan) lines

def lazyScan (scan : Scanned → Bool) (lines : List Scanned) : Read :=
  readWith (lazyFate scanned scan lines) lines

theorem map_filter_congr {α β : Type} (p : α → Bool) (f g : α → β) (xs : List α)
    (h : ∀ x ∈ xs, p x = true → f x = g x) : (xs.filter p).map f = (xs.filter p).map g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    have ht : ∀ y ∈ xs, p y = true → f y = g y := fun y hy => h y (List.mem_cons_of_mem x hy)
    cases hp : p x with
    | true => simp [hp, ih ht, h x List.mem_cons_self hp]
    | false => simp [hp, ih ht]

/-- 选去扫的行上，懒扫描与全扫描给同一个命运。 -/
theorem lazy_fate_on_scanned (scan : Scanned → Bool) (lines : List Scanned) (l : Scanned)
    (h : scanned lines l = true) : lazyFate scanned scan lines l = fullFate scan l := by
  simp [lazyFate, fullFate, h]

theorem read_congr (f g : Scanned → Fate) (lines : List Scanned)
    (h : ∀ l ∈ lines, scanned lines l = true → f l = g l) : readWith f lines = readWith g lines := by
  have hin : ∀ l ∈ lines, l.inRange = true → f l = g l := fun l hl hr =>
    h l hl (by simp [scanned, hr])
  have hre : ∀ l ∈ lines, l.remembered = true → f l = g l := fun l hl hr =>
    h l hl (by simp [scanned, hr])
  have hpa : ∀ l ∈ lines, paired lines l = true → f l = g l := fun l hl hr =>
    h l hl (by simp [scanned, hr])
  simp only [readWith, Read.mk.injEq]
  exact ⟨map_filter_congr _ _ _ lines fun l hl hr => by simp [hin l hl hr],
    map_filter_congr _ _ _ lines fun l hl hr => by simp [hre l hl hr],
    map_filter_congr _ _ _ lines fun l hl hr => by simp [hpa l hl hr]⟩

/-- **懒扫描与全扫描逐条相同**：范围内每行的命运、`remember()` 读的命运、配对两端的命运，在任意历史、任意扫描函数上都一样。 -/
theorem lazy_scan_equals_full_scan (scan : Scanned → Bool) (lines : List Scanned) :
    lazyScan scan lines = fullScan scan lines :=
  read_congr _ _ lines fun l _ h => lazy_fate_on_scanned scan lines l h

/-- **懒扫描从不读没选去扫的行**：两个扫描函数只要在选去扫的行上相同，懒扫描交出的就相同，所以那些行的字节不必读回。 -/
theorem lazy_scan_reads_only_scanned (scan scan' : Scanned → Bool) (lines : List Scanned)
    (h : ∀ l ∈ lines, scanned lines l = true → scan l = scan' l) :
    lazyScan scan lines = lazyScan scan' lines :=
  read_congr _ _ lines fun l hl hs => by simp [lazyFate, hs, h l hl hs]

/-- 懒扫描扫的行数不多于全扫描：它扫的是全扫描所扫的（楼全 `Open` 的行）里选去扫的那些。 -/
theorem lazy_scans_no_more (lines : List Scanned) :
    ((lines.filter (·.open_)).filter (scanned lines)).length ≤ (lines.filter (·.open_)).length :=
  List.length_filter_le _ _

/-- H10：范围内一条楼全 `Open`、带凭据的行被隐去，记作 `Credential`；它配对的范围外行同样被扫。 -/
theorem a_credential_line_is_withheld :
    let lines : List Scanned := [⟨1, false, true, some 5, true⟩, ⟨2, true, true, some 5, false⟩]
    let scan : Scanned → Bool := fun l => l.seq == 2
    (lazyScan scan lines).inRange = [(2, .credential)] ∧
      ((lazyScan scan lines).inRange.filter (·.2 == .credential)).length = 1 := by
  decide

/-- 坏实现：只扫范围内与要 `remember()` 的行，忘了配对。 -/
def forgetfulScanned (_lines : List Scanned) (l : Scanned) : Bool := l.inRange || l.remembered

/-- 一条范围外的凭据行（调用的打开行，种类不要 `remember()`）与范围内的答复同一个键：坏实现把它当作 `Shown`，配对一端漏了出去。 -/
theorem forgetting_a_pairing_leaks :
    let lines : List Scanned := [⟨1, false, true, some 5, false⟩, ⟨2, true, true, some 5, false⟩]
    let scan : Scanned → Bool := fun l => l.seq == 1
    readWith (lazyFate forgetfulScanned scan lines) lines ≠ fullScan scan lines := by
  decide

end Accounting.Playback.Project
