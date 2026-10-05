-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 本机隐私协调契约

## 1 需求分解
每次修改保留紧邻原值；先持久 Prepared 再发系统写入；读取不落盘；
恢复只有仍拥有且现值匹配的对象；未知结果、损坏历史、身份错配拒绝新写，支持或管理来源
未知拒写，确认的 control、definition、owner 和 expected 全部绑定。

## 2 验收标准
`reachable_ordered` 对任意可达轨迹约束恢复次序，`trace_keeps_originals`
约束原记录不可变，`trace_unknown` 约束未知吸收态。
`reachable_durable` 对任意步骤轨迹证明每次系统写入都有不可变持久原值。
本模型规定主机 privacy coordinator 必须保持的性质；模型的构建与 Rust
实现的对应检查分别验收，模型不能证明平台 IO 已满足耐久性。

## 3 假设与歧义
一次成功的 durablePrepared 事件表示文件与新建父目录在平台契约下已经持久。
OS 写入无 compare-and-swap；writeStarted 保存调用前最后一次读数，不证明
外部程序不能在随后 OS API 调用前改值。相同值的外部 ABA 不可辨识。
模型 owner 是真实 OS identity 的抽象，生产不得由 USERNAME 环境字符串代替。

## 4 现状分析
当前 CLI 没有 privacy 动词，也没有 journal、coordinator 或平台适配器。
本契约先确定顺序与恢复规则；派生 Rust 检查与生产接线属于接口缺口。

## 5 权威信源
Microsoft RegQueryValueExW/RegSetValueExW 规定原始类型/字节及缺值和访问失败；
about_Telemetry 与 about_Environment_Variables 规定消费者和启动时环境继承。
目标是否进入可写目录取决于平台资格，不由模型假定。
正式消费者来源：
https://github.com/PowerShell/PowerShell/blob/v7.5.7/src/System.Management.Automation/utils/Telemetry.cs
用户持久环境的位置与继承：
https://learn.microsoft.com/en-us/dotnet/api/system.environment.setenvironmentvariable
原始读取与写入：
https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regqueryvalueexw
https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regsetvalueexw

## 6 命名
RawValue 保留值存在性/类型/原始字节；keyExisted 独立记录键是否存在，恢复
删除本次值但保留父键，所以键存在性不冒充值是否恢复成功。
-/
namespace Sprawling.Privacy

inductive RawValue where
  | absent
  | present (kind : Nat) (bytes : List UInt8)
  deriving DecidableEq, Repr

structure Intent where
  operation : Nat
  owner : Nat
  original : RawValue
  modified : RawValue
  keyExisted : Bool
  restoreOf : Option Nat
  expires : Nat
  deriving DecidableEq, Repr

inductive Action where
  | apply
  | restore (owned : Intent)
  deriving DecidableEq, Repr

structure Request where
  action : Action
  control : Nat
  definition : Nat
  operation : Nat
  owner : Nat
  expected : RawValue
  target : RawValue
  expires : Nat
  code : Nat
  deriving DecidableEq, Repr

inductive Phase where
  | idle
  | prepared
  | attempted
  | unknown
  deriving DecidableEq, Repr

inductive Outcome where
  | applied
  | notApplied
  | restored
  deriving DecidableEq, Repr

structure Receipt where
  intent : Intent
  outcome : Outcome
  deriving DecidableEq, Repr

/-! ## 7 模块边界
journal 只追加，不执行 OS；coordinator 拥有跨存储顺序；confirmation 拥有
一次性许可；平台适配器只读写闭集对象。城的 Ledger 不出现在模型里。
-/
inductive Qualification where
  | localSupported
  | managed
  | managementUnknown
  | unsupported
  deriving DecidableEq, Repr

structure State where
  owner : Nat
  control : Nat
  definition : Nat
  qualification : Qualification
  live : RawValue
  recommendation : RawValue
  pending : Option Request := none
  confirmed : Option Request := none
  phase : Phase := .idle
  work : Option Intent := none
  journal : List Intent := []
  receipts : List Receipt := []
  writes : List (Intent × List Intent) := []
  owned : List Intent := []
  deriving DecidableEq, Repr

/-! ## 8 接口先行
request 保存确认时的读数；answer 消耗一次请求；prepare 成功才产生持久
工作项；writeStarted 才表示发生一次 OS 调用；durableReceipt 后才答成功。
-/
/-! D50 恢复只选最新仍拥有的操作，不能凭成员关系越过后来操作；
恢复终态只弹出这一层，早先原值继续作为不可变历史保留。
拒绝按 id 搜索任意旧操作，因为外部改变后再次 apply 会产生不同 original。
-/
def restoresLatest (i : Intent) (owned : List Intent) : Prop :=
  match i.restoreOf with
  | none => True
  | some operation => ∃ latest, owned.head? = some latest ∧ latest.operation = operation

def planValid (s : State) (r : Request) : Prop :=
  s.qualification = .localSupported ∧ r.control = s.control ∧
  r.definition = s.definition ∧
  r.owner = s.owner ∧ r.expected = s.live ∧ r.target ≠ s.live ∧
  match r.action with
  | .apply => r.target = s.recommendation
  | .restore old => s.owned.head? = some old ∧ old.owner = s.owner ∧
      s.live = old.modified ∧ r.target = old.original

instance (s : State) (r : Request) : Decidable (planValid s r) := by
  unfold planValid
  cases r.action <;> infer_instance

def preparedIntent (s : State) (r : Request) (keyExisted : Bool) : Intent :=
  { operation := r.operation, owner := s.owner, original := s.live,
    modified := r.target, keyExisted := keyExisted, expires := r.expires,
    restoreOf := match r.action with | .apply => none | .restore old => some old.operation }

/-! ## 9 工作流程
Step 的 durablePrepared 是成功 sync 的观察，不是写文件前的意图；
写成功但终态未持久留下 attempted，crash 后为 unknown，拒绝新的修改。
-/
inductive Step : State → State → Prop where
  | read (s) : Step s s
  | request (s) (r) (idle : s.phase = .idle) :
      Step s {s with pending := some r, confirmed := none}
  | answer (s) (r) (code now owner : Nat)
      (pending : s.pending = some r)
      (matches : code = r.code ∧ now < r.expires ∧ owner = s.owner ∧ r.owner = owner) :
      Step s {s with pending := none, confirmed := some r}
  | rejectedAnswer (s) : Step s {s with pending := none, confirmed := none}
  | durablePrepared (s) (r) (keyExisted : Bool)
      (idle : s.phase = .idle) (confirmed : s.confirmed = some r)
      (valid : planValid s r) (now : Nat) (unexpired : now < r.expires)
      (fresh : ∀ old ∈ s.journal, old.operation ≠ r.operation) :
      Step s {s with confirmed := none, pending := none, phase := .prepared,
        work := some (preparedIntent s r keyExisted),
        journal := preparedIntent s r keyExisted :: s.journal}
  | failedPrepare (s) : Step s {s with pending := none, confirmed := none}
  | writeStarted (s) (i) (prepared : s.phase = .prepared) (work : s.work = some i)
      (fresh : s.live = i.original) (readback : RawValue)
      (now : Nat) (unexpired : now < i.expires)
      (qualified : s.qualification = .localSupported) :
      Step s {s with phase := .attempted, live := readback, writes := (i, s.owned) :: s.writes}
  | durableReceipt (s) (i) (outcome) (attempted : s.phase = .attempted)
      (work : s.work = some i)
      (verified : match outcome with
        | .applied => s.live = i.modified ∧ i.restoreOf = none
        | .notApplied => s.live = i.original
        | .restored => s.live = i.modified ∧ i.restoreOf ≠ none) :
      Step s {s with phase := .idle, work := none,
        receipts := ⟨i, outcome⟩ :: s.receipts,
        owned := match outcome with
          | .applied => i :: s.owned
          | .notApplied => s.owned
          | .restored => s.owned.tail}
  | qualification (s) (value) : Step s {s with qualification := value}
  | external (s) (value) : Step s {s with live := value}
  | crash (s) : Step s {s with pending := none, confirmed := none,
      phase := if s.work.isSome then .unknown else s.phase}
  | malformed (s) : Step s {s with pending := none, confirmed := none, phase := .unknown}

/-! ## 10 实现逻辑
history 列表按最新在前表示磁盘 append 的投影；旧 Intent 从不编辑。
写入前同一用户跨进程锁覆盖 load、fresh read、Prepared、write 和 receipt。
Prepared 操作编号新鲜性只检查历史，不按墙钟决定 id。
-/
def Durable (s : State) : Prop :=
  (∀ entry, entry ∈ s.writes → entry.1 ∈ s.journal) ∧
  (∀ i, s.work = some i → i ∈ s.journal)

theorem step_durable {s t : State} (h : Durable s) (move : Step s t) : Durable t := by
  rcases h with ⟨writes, work⟩
  cases move with
  | read => exact ⟨writes, work⟩
  | request => exact ⟨writes, work⟩
  | answer => exact ⟨writes, work⟩
  | rejectedAnswer => exact ⟨writes, work⟩
  | durablePrepared s r keyExisted idle confirmed valid now unexpired fresh =>
      constructor
      · intro i hi
        exact List.mem_cons_of_mem _ (writes i hi)
      · intro i hi
        simp only [Option.some.injEq] at hi
        subst i
        exact List.mem_cons_self
  | failedPrepare => exact ⟨writes, work⟩
  | writeStarted s i prepared hi fresh readback now unexpired qualified =>
      constructor
      · intro j hj
        rcases List.mem_cons.mp hj with equal | previous
        · subst j
          exact work i hi
        · exact writes j previous
      · exact work
  | durableReceipt =>
      constructor
      · exact writes
      · intro i hi
        cases hi
  | qualification => exact ⟨writes, work⟩
  | external => exact ⟨writes, work⟩
  | crash => exact ⟨writes, work⟩
  | malformed => exact ⟨writes, work⟩

/-- 未执行的恢复意图只能引用最新拥有项，写入记录同时保存执行时的拥有栈。 -/
def Ready (s : State) : Prop :=
  ∀ i, s.work = some i → restoresLatest i s.owned

def Ordered (s : State) : Prop :=
  (∀ entry, entry ∈ s.writes → restoresLatest entry.1 entry.2) ∧ Ready s

theorem step_ready {s t : State} (ready : Ready s) (move : Step s t) : Ready t := by
  cases move with
  | read => exact ready
  | request => exact ready
  | answer => exact ready
  | rejectedAnswer => exact ready
  | durablePrepared s r keyExisted idle confirmed valid now unexpired fresh =>
      intro i hi
      simp only [Option.some.injEq] at hi
      subst i
      cases action : r.action with
      | apply => simp [restoresLatest, preparedIntent, action]
      | restore old =>
          refine ⟨old, ?_, rfl⟩
          have validRestore := valid.2.2.2.2.2.2
          simp only [action] at validRestore
          exact validRestore.1
  | failedPrepare => exact ready
  | writeStarted => exact ready
  | durableReceipt => intro i hi; cases hi
  | qualification => exact ready
  | external => exact ready
  | crash => exact ready
  | malformed => exact ready

theorem step_ordered {s t : State} (h : Ordered s) (move : Step s t) : Ordered t := by
  constructor
  · cases move with
    | writeStarted s i prepared work fresh readback now unexpired qualified =>
        intro entry present
        rcases List.mem_cons.mp present with equal | previous
        · subst entry
          exact h.2 i work
        · exact h.1 entry previous
    | read => exact h.1
    | request => exact h.1
    | answer => exact h.1
    | rejectedAnswer => exact h.1
    | durablePrepared => exact h.1
    | failedPrepare => exact h.1
    | durableReceipt => exact h.1
    | qualification => exact h.1
    | external => exact h.1
    | crash => exact h.1
    | malformed => exact h.1
  · exact step_ready h.2 move

inductive Reachable (owner control definition : Nat) (qualification : Qualification)
    (initial recommendation : RawValue) : State → Prop where
  | initial : Reachable owner control definition qualification initial recommendation
      ⟨owner, control, definition, qualification, initial, recommendation⟩
  | next {s t} : Reachable owner control definition qualification initial recommendation s → Step s t →
      Reachable owner control definition qualification initial recommendation t

theorem reachable_durable {owner control definition qualification initial recommendation s}
    (h : Reachable owner control definition qualification initial recommendation s) : Durable s := by
  induction h with
  | initial => simp [Durable]
  | next previous move ih => exact step_durable ih move

/-- 任意可达轨迹的每次恢复都使用执行时最近拥有的修改；外部写不允许跳层。 -/
theorem reachable_ordered {owner control definition qualification initial recommendation s}
    (h : Reachable owner control definition qualification initial recommendation s) : Ordered s := by
  induction h with
  | initial => simp [Ordered, Ready]
  | next previous move ih => exact step_ordered ih move

inductive Trace : State → State → Prop where
  | rest (s) : Trace s s
  | next {s t u} : Step s t → Trace t u → Trace s u

theorem step_keeps_originals {s t : State} (move : Step s t) :
    ∀ i, i ∈ s.journal → i ∈ t.journal := by
  cases move <;> intro i hi
  all_goals first | exact hi | exact List.mem_cons_of_mem _ hi

theorem trace_keeps_originals {s t : State} (trace : Trace s t) :
    ∀ i, i ∈ s.journal → i ∈ t.journal := by
  induction trace with
  | rest => intro i hi; exact hi
  | next move remaining ih =>
      intro i hi
      exact ih i (step_keeps_originals move i hi)

theorem step_unknown {s t : State} (move : Step s t) (unknown : s.phase = .unknown) :
    t.phase = .unknown := by
  cases move <;> simp_all

theorem trace_unknown {s t : State} (trace : Trace s t) (unknown : s.phase = .unknown) :
    t.phase = .unknown := by
  induction trace with
  | rest => exact unknown
  | next move remaining ih => exact ih (step_unknown move unknown)

/-- 恢复顺序反例的派生输入：旧项拒绝，最新项可撤销；raw 类型与字节不归一化。 -/
def restoreVectors : List (State × Request) :=
  [.absent, .present 1 [], .present 2 [37, 0, 0, 0]].flatMap fun original =>
    let first : Intent := ⟨1, 7, original, .present 1 [49, 0, 0, 0], true, none, 100⟩
    let second : Intent := ⟨2, 7, .present 1 [48, 0, 0, 0], first.modified, true, none, 100⟩
    let state : State :=
      { owner := 7, control := 1, definition := 1, qualification := .localSupported,
        live := second.modified, recommendation := first.modified,
        journal := [second, first], owned := [second, first] }
    [first, second].map fun old =>
      (state, { action := .restore old, control := 1, definition := 1,
        operation := 3, owner := 7, expected := state.live, target := old.original,
        expires := 100, code := 11 })

#eval restoreVectors.map fun (s, r) => (s.live, r.target, decide (planValid s r))

/-! ## 11 边界枚举
外部写任意字节/类型、写调用后读取第三值、崩溃、坏历史、错误/过期码、
身份错配、恢复缺席、同 id 重放都要由 Rust 生产 coordinator 派生检查覆盖。

## 12 错误处理
unknown 没有 prepare/write 出口，读取不自动补终态；恢复冲突要求重新 inspect，
不能强制覆盖。核对 unknown 的明确确认动作需单独建模，不能伪造失败收据。

## 13 依赖选型
journal 使用标准库 try_lock/write_all/sync_all，OS 使用经版本核对的安全原始
注册表 API；本模型未新增依赖，也没有认定某个注册表 crate 已通过验收。

## 14 硬编码声明
TTL、闭集 control、目录版本、支持类型由对应生产 authority 决定并传入模型；
owner/code/operation 的 Nat 是抽象，不是允许任意用户输入这些值。

## 15 影响面
首个消费者是本地 CLI，下一阶段 UI/helper 走同一 coordinator；Home 只给路径。

## 16 测试与约束
模型由 `lake build crates.sprawling.spec.Privacy` 验收。
Rust derived trace checks 尚无生产消费者；证明只覆盖模型，不证明 IO adapter。
真实 Windows 功能验收必须使用 disposable runner，绝不改 User 主机设置。

## 17 文档关系
本契约与 ARCHITECTURE.md §9/§11、accounting 的 Home 路径权威对应。
生产模块形成前必须登记模块图；平台资格、模型构建与派生 Rust 检查分别
验收，正式目标的来源不能替代 OS 功能验收。
-/
end Sprawling.Privacy
