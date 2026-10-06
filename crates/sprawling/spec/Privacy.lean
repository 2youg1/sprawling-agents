-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 主机隐私协调契约

本分部规定 `bin::privacy::coordinator`、`bin::privacy::plan` 与 `bin::privacy::fault` 必须保持的性质：
每次系统写入之前有不可变的持久原值，读回不符从不取得拥有，读回不符时把原值写回，
写回也失败就进入只有人核对才能离开的 unknown。

## 1 需求分解
- 每个 privacy control 有自己的拥有栈；恢复只撤销该控制最近仍拥有的操作（D51）。
- apply 的目标只来自控制表（`bin::privacy::controls`）：控制表说不写的控制没有写入路径（Privacy.Controls D58）。
- 确认绑定页面显示的当前快照（expected）：fresh read 与它不同即拒绝（Privacy.Confirmation D57）。
- 先持久 Prepared，再发系统写入；写入后由本进程读回判定，不信写入者的自述。
- 读回等于修改值 → Applied/Restored；等于原值 → NotApplied；都不是 → 写回原值并再读：
  等于原值 → RolledBack，否则 unknown。
- 任一未结操作（prepared、attempted、rollingBack、unknown）阻止所有控制的新写入；
  unknown 只经人核对（Reconcile）离开，核对不写系统。
来源：人的决定——不逐项取得生产写入资格，运行时读回、不符即报错并恢复（Privacy.Controls D58）；
隐私需求——页面显示当前值，改过的显示原值，记录并可恢复，每项由人手动开启，没有一键开启。

## 2 验收标准
`reachable_durable`：任意可达状态的每次系统写入（含回滚写）的 Intent 已在 journal 中。
`reachable_ordered`：每次写入发生时，恢复意图引用的是该控制拥有栈的栈顶。
`reachable_catalogued`：journal 中每个 apply 意图的控制在控制表里是可写的。
`move_owns_only_matched`：一个 Intent 新进入拥有栈，只能发生在读回值等于它的修改值时，且它是 apply。
`step_rollback_ends`：回滚阶段的每一步要么留在回滚，要么以读回原值的 RolledBack 结束，
要么进入 unknown；RolledBack 不改变拥有栈。
`trace_keeps_originals`：任何轨迹都不改写已持久的原值。
`trace_unknown`：协调者与环境的任何轨迹都不离开 unknown；离开只经 `Reconcile`。
派生检查：§16。模型不证明平台 IO 的耐久性。

## 3 假设与歧义
一次成功的 durablePrepared 表示文件与新建父目录在平台契约下已经持久（Privacy.Journal）。
OS 写入没有 compare-and-swap：writeStarted 要求调用前最后一次读数等于 original，不证明外部程序
不能在随后的 OS 调用前改值；相同值的外部 ABA 不可辨识。
写入的结果在模型里是任意读回值：机器作用域经提升子进程写入（Privacy.Windows D59），UAC 被拒、
子进程失败或访问拒绝都表现为某个读回值，由同一判定处理。
模型 owner 是真实 OS 身份的抽象；生产以 Vault 中绑定的 SecretRef 核对（Privacy.State D52）。
模型的 owner 从初始状态起固定；生产的空 history 没有 owner，第一次写入前为实时身份建立绑定。
两者等价，因为第一次写入之前没有任何拥有可以保护。读目标失败不在模型里：模型的 live 总能读到。
损坏的 journal（malformed）进入没有 work 的 unknown，Reconcile 不能离开它；修复损坏日志
是本应用之外的操作，页面只报告拒绝与恢复路径。

## 4 现状分析
控制表（`bin::privacy::controls`、`bin::privacy::originals`、`bin::privacy::target`）、schema 3 的
磁盘投影、journal 的写入器、`bin::privacy::plan` 与 `bin::privacy::coordinator` 已实现；
coordinator 经两个端口（Host 与 Journal，§7）运行；平台适配器（Privacy.Windows）与生产 Host
`bin::privacy::windows::host` 已实现，`bin::privacy::cli` 的写入动词（Privacy.Cli）与页面的服务
（Privacy.Service）经它们进入 coordinator；wire 帧见 wire 的 Privacy 分部。
尚未实现的拒绝：原值无法经它的写入路径原样写回时（机器作用域的原值超过提升子进程的 1024 字节上界，
或注册表原值没有无损的原始编码），apply 应在 Prepared 之前拒绝（Privacy.Windows D59）；现在这样的
apply 照常写入，之后的恢复或回滚由子进程或适配器拒绝写回，以 NotApplied 或 unknown 如实结束。
控制表写入的值都是 4 字节，只有人的主机上已有的异常原值会走到这里；补上它需要在 planApply 增加
一个拒绝分支并重新证明 §2 的性质。

## 5 权威信源
原始读写：RegQueryValueExW/RegSetValueExW 规定原始类型与字节、缺值和访问失败
https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regqueryvalueexw
https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regsetvalueexw
计划任务的启用与停用：ScheduledTasks 模块
https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/disable-scheduledtask
每个控制的目标与推荐值的来源见 Privacy.Controls §5。

## 6 命名
privacy control（PrivacyControl）是一个可写目标及其推荐值；original item（PrivacyOriginal）
是需求清单里的一行原文；operation kind 是四种写法之一（docs/glossary.md）。
RawValue 保留值的存在性、类型与原始字节；keyExisted 单独记录键是否存在（D60）。
Snapshot 是一个控制的当前读数：注册表值或计划任务状态。
-/
namespace Sprawling.Privacy

inductive RawValue where
  | absent
  | present (kind : Nat) (bytes : List UInt8)
  deriving DecidableEq, Repr

/-- 任务定义用摘要表示（去掉启用标志后的任务 XML 的 SHA-256，抽象为 Nat）。 -/
inductive TaskState where
  | absent
  | present (enabled : Bool) (definition : Nat)
  deriving DecidableEq, Repr

inductive Snapshot where
  | registry (value : RawValue)
  | task (state : TaskState)
  deriving DecidableEq, Repr

/-- 不写的原因是闭集；研究给出的每个不写结论落在其中一个（Privacy.Controls D58）。
needsOperationKind：写它需要本版本没有的 operation kind（例如一次启用写多个值，Privacy.Controls D64）。 -/
inductive Reason where
  | absent
  | obsolete
  | undeterminable
  | needsOperationKind
  deriving DecidableEq, Repr

/-- 推荐值：注册表与用户环境变量是固定的原始值；计划任务是「停用，定义不变」，
所以它的目标取决于该主机读到的定义。 -/
inductive Recommendation where
  | value (target : RawValue)
  | taskDisabled
  deriving DecidableEq, Repr

inductive Writability where
  | writes (recommendation : Recommendation)
  | notWritten (reason : Reason)
  deriving DecidableEq, Repr

/-- 该主机的目标快照；none 表示该主机没有该目标（任务不存在），不写。
目标种类与快照种类一致由 Rust 的 Target 类型保证，模型不再区分错配。 -/
def Recommendation.target : Recommendation → Snapshot → Option Snapshot
  | .value v, _ => some (.registry v)
  | .taskDisabled, .task (.present _ definition) => some (.task (.present false definition))
  | .taskDisabled, _ => none

structure Intent where
  operation : Nat
  control : Nat
  owner : Nat
  original : Snapshot
  modified : Snapshot
  keyExisted : Bool
  restoreOf : Option Nat
  expires : Nat
  deriving DecidableEq, Repr

inductive Action where
  | apply
  | restore
  deriving DecidableEq, Repr

/-- 一次操作命令。expected 是页面在人确认时显示的当前快照；operation 是协调者分配的
新编号；owner 是实时读到的 OS 身份；expires 是协调者在接受命令时由 SystemClock 定的期限。 -/
structure Request where
  action : Action
  control : Nat
  operation : Nat
  owner : Nat
  expected : Snapshot
  expires : Nat
  deriving DecidableEq, Repr

inductive Phase where
  | idle
  | prepared
  | attempted
  | rollingBack
  | unknown
  deriving DecidableEq, Repr

/-- Finished 的结果。abandoned 只由核对产生：核对时目标既不是原值也不是修改值，
本应用不再把这次操作算作拥有，也不写系统。 -/
inductive Outcome where
  | applied
  | notApplied
  | restored
  | rolledBack
  | abandoned
  deriving DecidableEq, Repr

structure Receipt where
  intent : Intent
  outcome : Outcome
  deriving DecidableEq, Repr

/-! ## 7 模块边界
`bin::privacy::plan` 只做判定（本节的 planApply、planRestore、judgeReadback、settle、reconciled，
以及回滚后的读回判定），无 IO、无时钟；`bin::privacy::fault` 是失败的闭集与它到 AxError 的映射；
`bin::privacy::coordinator` 拥有跨存储的次序（Step），经两个端口运行：
- Host：时钟（accounting 的 Clock）、实时 OS 身份的取样、按身份核对或建立 owner 引用、
  按控制读与写目标。读返回快照与父键是否存在，或访问拒绝、其他失败；写只报告访问拒绝、
  UAC 被拒或其他失败，从不报告结论。第二实现是测试的 FaultHost。
- Journal：持锁 fold 出的 history 与 append_durable；生产实现是 `bin::privacy::journal` 的
  LockedJournal，第二实现是测试的内存 journal，它经同一个 History::fold 判定每一行。
journal 只追加；平台适配器只读写控制表给出的目标；确认不保存服务端挂起状态
（Privacy.Confirmation D57）。城的 Ledger 不出现在模型里：主机隐私不进入城市重放。
-/

def setAt {α : Type} (f : Nat → α) (c : Nat) (v : α) : Nat → α :=
  fun d => if d = c then v else f d

structure State where
  owner : Nat
  catalog : Nat → Writability
  live : Nat → Snapshot
  phase : Phase := .idle
  work : Option Intent := none
  journal : List Intent := []
  receipts : List Receipt := []
  writes : List (Intent × List Intent) := []
  owned : Nat → List Intent := fun _ => []

/-! ## 8 接口先行
planApply 与 planRestore 的拒绝次序就是生产判定的次序：控制表、身份、未结操作、页面过期，
然后才是目标本身的判定。两者返回穷尽枚举，只有 write 带出 Intent。
-/

/-! D68 Rust 的判定只接受已核对身份的 history，不写的原项不是控制
模型的 planApply 先查控制表、再比身份；Rust 用类型持有这两支，判定函数里没有它们：
- 请求只能命名 PrivacyControl，而控制表的每一行都有写入值（Privacy.Controls），不写的原项
  只是 PrivacyOriginal，没有控制，所以 notWritable 在 Rust 中写不出来，reachable_catalogued 由类型成立。
- History 的拥有栈、未结操作与 owner 只经 History::holdings(authorize) 交出（Privacy.State），
  authorize 是 Host 按实时身份核对 owner 引用；不符即返回身份拒绝，判定看不到任何历史值。
  notOwner 因此是 authorize 的拒绝，不是判定的一支。
fresh read 在判定之前由 coordinator 取得，判定无 IO；读失败（访问拒绝或其他）在判定之前拒绝，
所以它与未结操作同时成立时报告读失败。两者都不写系统，次序只影响报告哪一个。
回放：applyVectors 不含 notWritable 行；notOwner 行经 History::holdings 的拒绝回放。
被否：把 owner 与控制表作为判定的输入——判定会在核对身份之前拿到拥有栈，
「身份在披露任何历史之前」就只靠调用者的自律；而 notWritable 一支只能用测试专用的伪控制触发。
重开参数：若控制表出现「已列出但不写」的控制（而不是原项），notWritable 回到判定。
-/
inductive ApplyPlan where
  | write (intent : Intent)
  | notWritable (reason : Reason)
  | notOwner
  | unresolved
  | changed
  | targetAbsent
  | alreadyRecommended
  deriving DecidableEq, Repr

inductive RestorePlan where
  | write (intent : Intent)
  | notOwner
  | unresolved
  | nothingOwned
  | changed
  | conflict
  deriving DecidableEq, Repr

def planApply (s : State) (r : Request) (keyExisted : Bool) : ApplyPlan :=
  match s.catalog r.control with
  | .notWritten reason => .notWritable reason
  | .writes recommendation =>
    if r.owner ≠ s.owner then .notOwner
    else if s.phase ≠ .idle then .unresolved
    else if r.expected ≠ s.live r.control then .changed
    else match recommendation.target (s.live r.control) with
      | none => .targetAbsent
      | some target =>
        if target = s.live r.control then .alreadyRecommended
        else .write
          { operation := r.operation, control := r.control, owner := s.owner,
            original := s.live r.control, modified := target, keyExisted := keyExisted,
            restoreOf := none, expires := r.expires }

/-! D51 恢复只选该控制最新仍拥有的操作，不能凭成员关系越过后来操作；
恢复终态只弹出这一层，早先原值继续作为不可变历史保留。
拒绝按 id 搜索任意旧操作，因为外部改变后再次 apply 会产生不同 original。
恢复不查控制表：控制表修订把某控制改为不写时，本应用已改过的值仍必须能恢复。
当前值不等于栈顶的修改值即 conflict，不覆盖别人后来写下的值。
-/
def planRestore (s : State) (r : Request) (keyExisted : Bool) : RestorePlan :=
  if r.owner ≠ s.owner then .notOwner
  else if s.phase ≠ .idle then .unresolved
  else match s.owned r.control with
    | [] => .nothingOwned
    | top :: _ =>
      if r.expected ≠ s.live r.control then .changed
      else if s.live r.control ≠ top.modified then .conflict
      else .write
        { operation := r.operation, control := r.control, owner := s.owner,
          original := s.live r.control, modified := top.original, keyExisted := keyExisted,
          restoreOf := some top.operation, expires := r.expires }

def planned (s : State) (r : Request) (keyExisted : Bool) : Option Intent :=
  match r.action with
  | .apply => match planApply s r keyExisted with
    | .write i => some i
    | _ => none
  | .restore => match planRestore s r keyExisted with
    | .write i => some i
    | _ => none

inductive Readback where
  | matches
  | stillOriginal
  | other
  deriving DecidableEq, Repr

def judgeReadback (i : Intent) (value : Snapshot) : Readback :=
  if value = i.modified then .matches
  else if value = i.original then .stillOriginal
  else .other

/-- 读回结论对应的 Finished；other 没有结论，进入回滚。 -/
def settle (i : Intent) : Readback → Option Outcome
  | .matches => some (if i.restoreOf = none then .applied else .restored)
  | .stillOriginal => some .notApplied
  | .other => none

/-- 拥有栈只在 Applied 时压栈、Restored 时弹栈；其他结果不改变它。 -/
def own (owned : Nat → List Intent) (i : Intent) : Outcome → Nat → List Intent
  | .applied => setAt owned i.control (i :: owned i.control)
  | .restored => setAt owned i.control (owned i.control).tail
  | .notApplied => owned
  | .rolledBack => owned
  | .abandoned => owned

/-- 核对时的结论：读回规则照常，第三值记为 abandoned。 -/
def reconciled (i : Intent) (value : Snapshot) : Outcome :=
  (settle i (judgeReadback i value)).getD .abandoned

/-! ## 9 工作流程
durablePrepared 是成功 sync 的观察，不是写文件前的意图。writeStarted 表示发生了一次 OS 写入，
readback 是本进程随后读到的值。rollbackStarted 是第二次 OS 写入（把 original 写回），
不检查期限，因为回滚必须发生。lapsed：Prepared 已持久而期限已过，不写系统，以 notApplied 结束。
崩溃时有 work 即 unknown；写入后读不到目标（读回失败）与崩溃同样处理，进入 unknown，
因为没有读数就既不能判定，也不能确认回滚是否需要。
-/
/-! D62 同一时刻至多一个未结操作；「恢复全部」是逐个控制的单项恢复
协调者一次只持有一个 work：Prepared、写、读回、Finished 走完才开始下一个控制。
「恢复全部」按控制逐个执行这个次序，每个控制各自读回、各自记录，部分成功如实返回；
机器作用域的每次写入各经一次提升（一次 UAC）。
被否：把多个控制的 Prepared 先写成一批、一次提升写完再逐个读回——那要求 journal 同时容纳多个
未结操作，读回不符的回滚、unknown 的吸收与 fold 的「未结操作阻止新写入」都要改成按控制判定，
而节省的 UAC 次数不超过人当初逐项应用时已经确认过的次数。
重开参数：若一个人常态下拥有的机器作用域控制多到逐个 UAC 不可接受，再把 work 推广为按控制的集合。
-/
inductive Step : State → State → Prop where
  | read (s) : Step s s
  | durablePrepared (s) (r) (i) (keyExisted : Bool) (now : Nat)
      (plan : planned s r keyExisted = some i) (unexpired : now < r.expires)
      (fresh : ∀ old ∈ s.journal, old.operation ≠ r.operation) :
      Step s {s with phase := .prepared, work := some i, journal := i :: s.journal}
  | lapsed (s) (i) (prepared : s.phase = .prepared) (work : s.work = some i)
      (now : Nat) (expired : i.expires ≤ now) :
      Step s {s with phase := .idle, work := none, receipts := ⟨i, .notApplied⟩ :: s.receipts}
  | writeStarted (s) (i) (prepared : s.phase = .prepared) (work : s.work = some i)
      (fresh : s.live i.control = i.original) (readback : Snapshot)
      (now : Nat) (unexpired : now < i.expires) :
      Step s {s with
        phase := .attempted, live := setAt s.live i.control readback,
        writes := (i, s.owned i.control) :: s.writes}
  | durableReceipt (s) (i) (outcome) (attempted : s.phase = .attempted)
      (work : s.work = some i)
      (verified : settle i (judgeReadback i (s.live i.control)) = some outcome) :
      Step s {s with
        phase := .idle, work := none, receipts := ⟨i, outcome⟩ :: s.receipts,
        owned := own s.owned i outcome}
  | rollbackStarted (s) (i) (attempted : s.phase = .attempted) (work : s.work = some i)
      (mismatch : judgeReadback i (s.live i.control) = .other) (readback : Snapshot) :
      Step s {s with
        phase := .rollingBack, live := setAt s.live i.control readback,
        writes := (i, s.owned i.control) :: s.writes}
  | rolledBack (s) (i) (rolling : s.phase = .rollingBack) (work : s.work = some i)
      (restored : s.live i.control = i.original) :
      Step s {s with phase := .idle, work := none, receipts := ⟨i, .rolledBack⟩ :: s.receipts}
  | rollbackLost (s) (i) (rolling : s.phase = .rollingBack) (work : s.work = some i)
      (other : s.live i.control ≠ i.original) :
      Step s {s with phase := .unknown}
  | external (s) (c) (value) : Step s {s with live := setAt s.live c value}
  | crash (s) : Step s {s with phase := if s.work.isSome then .unknown else s.phase}
  | malformed (s) : Step s {s with phase := .unknown}

/-- 人的核对：页面显示当前读数，人确认后发送的命令带着它；fresh read 不符即不生效。
核对不写系统，只按读回规则给未结操作一个 Finished。 -/
inductive Reconcile : State → State → Prop where
  | settle (s) (i) (owner : Nat) (expected : Snapshot) (unknown : s.phase = .unknown)
      (work : s.work = some i) (identity : owner = s.owner)
      (shown : expected = s.live i.control) :
      Reconcile s {s with
        phase := .idle, work := none,
        receipts := ⟨i, reconciled i (s.live i.control)⟩ :: s.receipts,
        owned := own s.owned i (reconciled i (s.live i.control))}

inductive Move : State → State → Prop where
  | step {s t} : Step s t → Move s t
  | reconcile {s t} : Reconcile s t → Move s t

inductive Trace (R : State → State → Prop) : State → State → Prop where
  | rest (s) : Trace R s s
  | next {s t u} : R s t → Trace R t u → Trace R s u

inductive Reachable (owner : Nat) (catalog : Nat → Writability) (initial : Nat → Snapshot) :
    State → Prop where
  | initial : Reachable owner catalog initial { owner := owner, catalog := catalog, live := initial }
  | next {s t} : Reachable owner catalog initial s → Move s t → Reachable owner catalog initial t

/-! ## 10 实现逻辑
journal 列表按最新在前表示磁盘 append 的投影；旧 Intent 从不编辑。
写入前同一用户跨进程锁覆盖 load、fresh read、Prepared、write、读回、回滚和 receipt。
Prepared 操作编号新鲜性只检查历史，不按墙钟决定 id。
-/

theorem planApply_write {s r k i} (h : planApply s r k = .write i) :
    i.restoreOf = none ∧ i.control = r.control ∧ i.original = r.expected ∧
    (∃ recommendation, s.catalog r.control = .writes recommendation) := by
  unfold planApply at h
  split at h
  · cases h
  · rename_i recommendation _
    split at h
    · cases h
    split at h
    · cases h
    split at h
    · cases h
    rename_i _ _ same
    split at h
    · cases h
    split at h
    · cases h
    cases h
    exact ⟨rfl, rfl, (Classical.not_not.mp same).symm, recommendation, by assumption⟩

theorem planRestore_write {s r k i} (h : planRestore s r k = .write i) :
    i.control = r.control ∧ i.original = r.expected ∧
    ∃ top rest, s.owned r.control = top :: rest ∧ i.restoreOf = some top.operation := by
  unfold planRestore at h
  split at h
  · cases h
  split at h
  · cases h
  split at h
  · cases h
  · rename_i top rest stack
    split at h
    · cases h
    rename_i same
    split at h
    · cases h
    cases h
    exact ⟨rfl, (Classical.not_not.mp same).symm, top, rest, stack, rfl⟩

def restoresLatest (i : Intent) (owned : List Intent) : Prop :=
  match i.restoreOf with
  | none => True
  | some operation => ∃ latest, owned.head? = some latest ∧ latest.operation = operation

theorem planned_facts {s r k i} (h : planned s r k = some i) :
    i.control = r.control ∧ i.original = r.expected ∧ restoresLatest i (s.owned i.control) ∧
    (i.restoreOf = none → ∃ recommendation, s.catalog i.control = .writes recommendation) := by
  unfold planned at h
  split at h
  · split at h
    · cases h
      rename_i i' write
      obtain ⟨none, control, original, catalog⟩ := planApply_write write
      refine ⟨control, original, ?_, fun _ => control ▸ catalog⟩
      simp [restoresLatest, none]
    · cases h
  · split at h
    · cases h
      rename_i i' write
      obtain ⟨control, original, top, rest, stack, restoreOf⟩ := planRestore_write write
      refine ⟨control, original, ?_, fun absent => by simp [restoreOf] at absent⟩
      simp only [restoresLatest, restoreOf, control, stack]
      exact ⟨top, rfl, rfl⟩
    · cases h

def Durable (s : State) : Prop :=
  (∀ entry, entry ∈ s.writes → entry.1 ∈ s.journal) ∧
  (∀ i, s.work = some i → i ∈ s.journal)

theorem move_durable {s t : State} (h : Durable s) (move : Move s t) : Durable t := by
  rcases h with ⟨writes, work⟩
  cases move with
  | reconcile move =>
      cases move
      exact ⟨writes, fun _ hi => by cases hi⟩
  | step move =>
      cases move with
      | read => exact ⟨writes, work⟩
      | durablePrepared r i =>
          constructor
          · intro entry hi
            exact List.mem_cons_of_mem _ (writes entry hi)
          · intro j hj
            simp only [Option.some.injEq] at hj
            subst j
            exact List.mem_cons_self
      | writeStarted i _ hi =>
          constructor
          · intro entry hentry
            rcases List.mem_cons.mp hentry with equal | previous
            · subst entry
              exact work i hi
            · exact writes entry previous
          · exact work
      | durableReceipt => exact ⟨writes, fun _ hi => by cases hi⟩
      | lapsed => exact ⟨writes, fun _ hi => by cases hi⟩
      | rollbackStarted i _ hi =>
          constructor
          · intro entry hentry
            rcases List.mem_cons.mp hentry with equal | previous
            · subst entry
              exact work i hi
            · exact writes entry previous
          · exact work
      | rolledBack => exact ⟨writes, fun _ hi => by cases hi⟩
      | rollbackLost => exact ⟨writes, work⟩
      | external => exact ⟨writes, work⟩
      | crash => exact ⟨writes, work⟩
      | malformed => exact ⟨writes, work⟩

theorem reachable_durable {owner catalog initial s}
    (h : Reachable owner catalog initial s) : Durable s := by
  induction h with
  | initial => simp [Durable]
  | next _ move ih => exact move_durable ih move

/-- 尚未执行完的意图引用的是它那个控制的最新拥有项；写入记录同时保存执行时的拥有栈。 -/
def Ready (s : State) : Prop :=
  ∀ i, s.work = some i → restoresLatest i (s.owned i.control)

def Ordered (s : State) : Prop :=
  (∀ entry, entry ∈ s.writes → restoresLatest entry.1 entry.2) ∧ Ready s

theorem move_ordered {s t : State} (h : Ordered s) (move : Move s t) : Ordered t := by
  rcases h with ⟨writes, ready⟩
  cases move with
  | reconcile move =>
      cases move
      exact ⟨writes, fun _ hi => by cases hi⟩
  | step move =>
      cases move with
      | read => exact ⟨writes, ready⟩
      | durablePrepared r i keyExisted now plan =>
          refine ⟨writes, ?_⟩
          intro j hj
          simp only [Option.some.injEq] at hj
          subst j
          exact (planned_facts plan).2.2.1
      | writeStarted i _ hi =>
          refine ⟨?_, ready⟩
          intro entry hentry
          rcases List.mem_cons.mp hentry with equal | previous
          · subst entry
            exact ready i hi
          · exact writes entry previous
      | durableReceipt => exact ⟨writes, fun _ hi => by cases hi⟩
      | lapsed => exact ⟨writes, fun _ hi => by cases hi⟩
      | rollbackStarted i _ hi =>
          refine ⟨?_, ready⟩
          intro entry hentry
          rcases List.mem_cons.mp hentry with equal | previous
          · subst entry
            exact ready i hi
          · exact writes entry previous
      | rolledBack => exact ⟨writes, fun _ hi => by cases hi⟩
      | rollbackLost => exact ⟨writes, ready⟩
      | external => exact ⟨writes, ready⟩
      | crash => exact ⟨writes, ready⟩
      | malformed => exact ⟨writes, ready⟩

/-- 任意可达轨迹的每次写入都使用执行时该控制最近拥有的修改；外部写不允许跳层。 -/
theorem reachable_ordered {owner catalog initial s}
    (h : Reachable owner catalog initial s) : Ordered s := by
  induction h with
  | initial => simp [Ordered, Ready, restoresLatest]
  | next _ move ih => exact move_ordered ih move

/-- journal 里的每个 apply 意图都来自控制表里可写的控制：不写的原项没有到达系统的路径。 -/
def Catalogued (s : State) : Prop :=
  ∀ i ∈ s.journal, i.restoreOf = none → ∃ recommendation, s.catalog i.control = .writes recommendation

theorem move_catalogued {s t : State} (h : Catalogued s) (move : Move s t) : Catalogued t := by
  cases move with
  | reconcile move => cases move; exact h
  | step move =>
      cases move with
      | durablePrepared r i keyExisted now plan =>
          intro j hj
          rcases List.mem_cons.mp hj with equal | previous
          · subst j
            exact (planned_facts plan).2.2.2
          · exact h j previous
      | read => exact h
      | writeStarted => exact h
      | durableReceipt => exact h
      | lapsed => exact h
      | rollbackStarted => exact h
      | rolledBack => exact h
      | rollbackLost => exact h
      | external => exact h
      | crash => exact h
      | malformed => exact h

theorem reachable_catalogued {owner catalog initial s}
    (h : Reachable owner catalog initial s) : Catalogued s := by
  induction h with
  | initial => intro i hi; cases hi
  | next _ move ih => exact move_catalogued ih move

theorem settle_applied {i v} (h : settle i (judgeReadback i v) = some .applied) :
    v = i.modified ∧ i.restoreOf = none := by
  unfold judgeReadback at h
  split at h
  · rename_i same
    refine ⟨same, ?_⟩
    simp only [settle, Option.some.injEq] at h
    split at h
    · assumption
    · cases h
  · split at h <;> simp [settle] at h

theorem reconciled_applied {i v} (h : reconciled i v = .applied) :
    v = i.modified ∧ i.restoreOf = none := by
  unfold reconciled at h
  cases hs : settle i (judgeReadback i v) with
  | none => simp [hs] at h
  | some outcome =>
      simp only [hs, Option.getD_some] at h
      subst h
      exact settle_applied hs

theorem own_grows {owned : Nat → List Intent} {i j : Intent} {c : Nat} {outcome : Outcome}
    (now : j ∈ own owned i outcome c) (before : j ∉ owned c) :
    outcome = .applied ∧ j = i := by
  cases outcome with
  | applied =>
      refine ⟨rfl, ?_⟩
      simp only [own, setAt] at now
      split at now
      · rename_i same
        subst same
        rcases List.mem_cons.mp now with equal | previous
        · exact equal
        · exact absurd previous before
      · exact absurd now before
  | restored =>
      simp only [own, setAt] at now
      split at now
      · rename_i same
        subst same
        exact absurd (List.mem_of_mem_tail now) before
      · exact absurd now before
  | notApplied => exact absurd now before
  | rolledBack => exact absurd now before
  | abandoned => exact absurd now before

/-- 读回不符永不取得拥有：一个 Intent 新进入某个拥有栈，只能是正在执行的 apply，
且当时读到的值等于它的修改值。 -/
theorem move_owns_only_matched {s t : State} (move : Move s t) {c : Nat} {j : Intent}
    (now : j ∈ t.owned c) (before : j ∉ s.owned c) :
    s.work = some j ∧ s.live j.control = j.modified ∧ j.restoreOf = none := by
  cases move with
  | reconcile move =>
      cases move with
      | settle i _ _ _ work =>
          obtain ⟨applied, same⟩ := own_grows now before
          subst same
          exact ⟨work, reconciled_applied applied⟩
  | step move =>
      cases move with
      | durableReceipt i outcome _ work verified =>
          obtain ⟨applied, same⟩ := own_grows now before
          subst same applied
          exact ⟨work, settle_applied verified⟩
      | read => exact absurd now before
      | durablePrepared => exact absurd now before
      | lapsed => exact absurd now before
      | writeStarted => exact absurd now before
      | rollbackStarted => exact absurd now before
      | rolledBack => exact absurd now before
      | rollbackLost => exact absurd now before
      | external => exact absurd now before
      | crash => exact absurd now before
      | malformed => exact absurd now before

theorem planApply_idle {s r k i} (h : planApply s r k = .write i) : s.phase = .idle := by
  unfold planApply at h
  split at h
  · cases h
  · split at h
    · cases h
    split at h
    · cases h
    · rename_i idle; simpa using idle

theorem planRestore_idle {s r k i} (h : planRestore s r k = .write i) : s.phase = .idle := by
  unfold planRestore at h
  split at h
  · cases h
  split at h
  · cases h
  · rename_i idle; simpa using idle

theorem planned_idle {s r k i} (h : planned s r k = some i) : s.phase = .idle := by
  unfold planned at h
  split at h
  · split at h
    · rename_i write; exact planApply_idle write
    · cases h
  · split at h
    · rename_i write; exact planRestore_idle write
    · cases h

/-- 回滚结束于原值或 unknown：回滚阶段的任一步留在回滚、进入 unknown，或在读回原值时
以 RolledBack 结束且不改变拥有栈。 -/
theorem step_rollback_ends {s t : State} (move : Step s t) (rolling : s.phase = .rollingBack) :
    t.phase = .rollingBack ∨ t.phase = .unknown ∨
    (t.phase = .idle ∧ t.owned = s.owned ∧ ∃ i, s.work = some i ∧
      s.live i.control = i.original ∧ t.receipts = ⟨i, .rolledBack⟩ :: s.receipts) := by
  cases move with
  | read => exact Or.inl rolling
  | durablePrepared r i keyExisted now plan =>
      have idle := planned_idle plan
      simp [rolling] at idle
  | lapsed _ prepared => simp [rolling] at prepared
  | writeStarted _ prepared => simp [rolling] at prepared
  | durableReceipt _ _ attempted => simp [rolling] at attempted
  | rollbackStarted _ attempted => simp [rolling] at attempted
  | rolledBack i _ work restored => exact Or.inr (Or.inr ⟨rfl, rfl, i, work, restored, rfl⟩)
  | rollbackLost => exact Or.inr (Or.inl rfl)
  | external => exact Or.inl rolling
  | crash =>
      by_cases h : s.work.isSome
      · exact Or.inr (Or.inl (by simp [h]))
      · exact Or.inl (by simp [h, rolling])
  | malformed => exact Or.inr (Or.inl rfl)

theorem move_keeps_originals {s t : State} (move : Move s t) :
    ∀ i, i ∈ s.journal → i ∈ t.journal := by
  cases move with
  | reconcile move => cases move; intro i hi; exact hi
  | step move =>
      cases move <;> intro i hi
      all_goals first | exact hi | exact List.mem_cons_of_mem _ hi

theorem trace_keeps_originals {s t : State} (trace : Trace Move s t) :
    ∀ i, i ∈ s.journal → i ∈ t.journal := by
  induction trace with
  | rest => intro i hi; exact hi
  | next move _ ih =>
      intro i hi
      exact ih i (move_keeps_originals move i hi)

theorem step_unknown {s t : State} (move : Step s t) (unknown : s.phase = .unknown) :
    t.phase = .unknown := by
  cases move with
  | durablePrepared r i keyExisted now plan =>
      have idle := planned_idle plan
      simp [unknown] at idle
  | crash => simp [unknown]
  | _ => simp_all

/-- 协调者与环境的任何轨迹都不离开 unknown；离开只经人的 Reconcile。 -/
theorem trace_unknown {s t : State} (trace : Trace Step s t) (unknown : s.phase = .unknown) :
    t.phase = .unknown := by
  induction trace with
  | rest => exact unknown
  | next move _ ih => exact ih (step_unknown move unknown)

/-! ## 11 边界枚举
下面的向量由 #eval 打印，`bin::privacy::plan` 的测试逐条回放（§16）。
控制表：控制 1 写注册表值 1；控制 2 不写（undeterminable）；控制 3 停用计划任务。
-/
def one : RawValue := .present 4 [1, 0, 0, 0]
def zero : RawValue := .present 4 [0, 0, 0, 0]
def text : RawValue := .present 1 [49, 0, 0, 0]

def vectorCatalog : Nat → Writability
  | 1 => .writes (.value one)
  | 3 => .writes .taskDisabled
  | _ => .notWritten .undeterminable

def vectorState (phase : Phase) (live : Snapshot) (task : Snapshot) (owned : List Intent) : State :=
  { owner := 7, catalog := vectorCatalog, phase := phase,
    live := fun c => if c = 3 then task else live,
    owned := fun c => if c = 1 then owned else [] }

def applied : Intent :=
  { operation := 1, control := 1, owner := 7, original := .registry .absent,
    modified := .registry one, keyExisted := false, restoreOf := none, expires := 100 }

def request (action : Action) (control owner : Nat) (expected : Snapshot) : Request :=
  { action, control, operation := 2, owner, expected, expires := 100 }

/-- apply 判定：每行是（控制、阶段、当前值、请求的 expected、身份），以及判定结果。
不含 notWritable 行：Rust 中它由类型排除（D68）。 -/
def applyVectors : List ApplyPlan :=
  [ planApply (vectorState .idle (.registry .absent) (.task .absent) []) (request .apply 1 8 (.registry .absent)) false,
    planApply (vectorState .unknown (.registry .absent) (.task .absent) []) (request .apply 1 7 (.registry .absent)) false,
    planApply (vectorState .idle (.registry zero) (.task .absent) []) (request .apply 1 7 (.registry .absent)) false,
    planApply (vectorState .idle (.registry one) (.task .absent) []) (request .apply 1 7 (.registry one)) false,
    planApply (vectorState .idle (.registry .absent) (.task .absent) []) (request .apply 1 7 (.registry .absent)) false,
    planApply (vectorState .idle (.registry text) (.task .absent) []) (request .apply 1 7 (.registry text)) true,
    planApply (vectorState .idle (.registry .absent) (.task .absent) []) (request .apply 3 7 (.task .absent)) false,
    planApply (vectorState .idle (.registry .absent) (.task (.present false 5)) []) (request .apply 3 7 (.task (.present false 5))) false,
    planApply (vectorState .idle (.registry .absent) (.task (.present true 5)) []) (request .apply 3 7 (.task (.present true 5))) false ]

#eval applyVectors

/-- restore 判定：身份、未结、无拥有、页面过期、冲突、写回原值（含原值缺席）。 -/
def restoreVectors : List RestorePlan :=
  [ planRestore (vectorState .idle (.registry one) (.task .absent) [applied]) (request .restore 1 8 (.registry one)) true,
    planRestore (vectorState .prepared (.registry one) (.task .absent) [applied]) (request .restore 1 7 (.registry one)) true,
    planRestore (vectorState .idle (.registry one) (.task .absent) []) (request .restore 1 7 (.registry one)) true,
    planRestore (vectorState .idle (.registry one) (.task .absent) [applied]) (request .restore 1 7 (.registry zero)) true,
    planRestore (vectorState .idle (.registry zero) (.task .absent) [applied]) (request .restore 1 7 (.registry zero)) true,
    planRestore (vectorState .idle (.registry one) (.task .absent) [applied]) (request .restore 1 7 (.registry one)) true ]

#eval restoreVectors

/-- 读回判定与 Finished：apply 与 restore 各三种读回，以及核对时的第三值。 -/
def readbackVectors : List (Readback × Option Outcome × Outcome) :=
  let restore : Intent :=
    { applied with operation := 2, original := .registry one, modified := .registry .absent,
                   restoreOf := some 1 }
  [applied, restore].flatMap fun i =>
    [i.modified, i.original, .registry zero].map fun v =>
      (judgeReadback i v, settle i (judgeReadback i v), reconciled i v)

#eval readbackVectors

/-! ## 12 错误处理
拒绝的穷尽集合是 ApplyPlan 与 RestorePlan 的非 write 分支；写入后的失败只有三种去向：
NotApplied（读回原值，报错，含 UAC 被拒与访问拒绝）、RolledBack（读回第三值并已写回原值，
报错 readback_mismatch）、unknown（写回后仍不是原值，或崩溃）。unknown 拒绝所有新写入，
只经 Reconcile 离开；读取不自动补终态。恢复冲突要求重新读取，不能强制覆盖。
lapsed 以 NotApplied 结束并报错 expired；写入后读不到目标、或回滚后读不到原值，都以 unknown 结束。
Finished 未能持久时，系统可能已被写入，操作在磁盘上仍未结，下次读取报告 unresolved。
每个失败在 `bin::privacy::fault` 一处映射到 AxError，带失败的动作、对象、稳定码与恢复，
不共用一个码。稳定码是闭集：identity、clock、history、unreadable、unresolved、changed、
target_absent、nothing_owned、conflict、nothing_unresolved、history_full、expired、access_denied、
elevation_declined、not_applied、readback_mismatch、unknown、receipt_lost。
写入报告的失败（访问拒绝、UAC 被拒、其他）只决定读回原值时报告哪个码，从不决定结论。

## 13 依赖选型
journal 使用标准库 File::lock/write_all/sync_all（Privacy.Journal）；注册表使用 winreg 的安全
原始值接口，计划任务使用受保护路径下 Windows PowerShell 的 ScheduledTasks 模块（Privacy.Windows）。

## 14 硬编码声明
期限 TTL 由 `bin::privacy::coordinator` 一处定义，为 60 秒，从协调者接受命令时算起：它覆盖取得锁、
fold、fresh read（计划任务经 Windows PowerShell 读取，冷启动要几秒）与 Prepared，期间进程若被挂起
（休眠、调试暂停）超过它，fresh read 证明的「当前值就是人看到的值」已经过时，就不再写；
改变它只改变人需要重试的频率，不改变任何性质。控制表、推荐值、作用域由 `bin::privacy::controls`
一处定义；模型中 owner、operation、control、definition 的 Nat 是抽象。

## 15 影响面
调用者：`bin::privacy::cli`（本地 CLI，一次性 runner 的验收经它进入）与 `bin::privacy::service`
（装配层为页面服务，Privacy.Service）走同一个 coordinator。Home 只给 journal 路径。

## 16 测试与约束
模型由 `lake build crates.sprawling.spec.Privacy` 验收，无 sorry/admit/axiom。
派生检查：`bin::privacy::plan` 的测试逐条回放 applyVectors、restoreVectors、readbackVectors；
`bin::privacy::coordinator` 的 proptest 用 FaultHost 与内存 journal 生成任意命令与故障序列
（写失败、写入第三值、读回失败、Prepared 持久失败、Finished 持久失败、回滚失败、外部改值、期限已过），
断言 reachable_durable（每次系统写入之前 journal 的最后一行是同一控制的 Prepared，写入值是它的
修改值或回滚时的原值）、move_owns_only_matched（Applied 与 Restored 只在目标真实值等于修改值时记录）、
step_rollback_ends（RolledBack 只在目标真实值等于原值时记录）与 trace_unknown（unknown 之后、
核对之前没有系统写入）对应的性质；另有一条经真实 LockedJournal 的 apply 与 restore。证明只覆盖模型，不证明 IO 适配器。
真实 Windows 行为只在一次性 GitHub Actions runner 上验收（`.github/workflows/privacy-windows-acceptance.yml`，
windows-2022 与 windows-2025）：`crates/sprawling/tests/privacy_disposable.rs` 的测试全部 #[ignore]，
开头要求 GITHUB_ACTIONS=true、RUNNER_OS=Windows 与 SPRAWLING_DISPOSABLE_PRIVACY=1 同时成立，
否则失败而不写任何东西，所以默认测试与人的主机都不会运行它们。测试经构建出的二进制的 privacy CLI
进入生产路径（提升子进程就是这个可执行文件），每个用例一个临时 home，读回另用 reg.exe 与
Get-ScheduledTask 独立核对：四种 operation kind 各一次 apply 与 restore，覆盖原值缺席与原值存在且
不同（预置值按原字节恢复，而不是删除）；恢复冲突（apply 后外部改成第三个值或重新启用任务，
restore 返回 conflict 且值不变）；已是写入值时 already_written 且历史不增行；
全部 88 个控制逐个 apply → 读回 → restore → 读回，restore 不等于原值或出现 unknown、
readback_mismatch 即失败，apply 以 not_applied 结束只记录不失败。工作流在测试前后导出受影响的
注册表键并逐字节比较，扫描新建的空键单列为残留键；runner 账户已提升，所以它不能说明标准账户
能否写 HKCU\Software\Policies，产物如实写明这一点。测试永不写人的主机。

## 17 文档关系
本契约与 ARCHITECTURE.md §9/§11、Privacy.Controls（控制表）、Privacy.Confirmation（确认）、
Privacy.State（磁盘投影）、Privacy.Windows（平台）对应；新增 operation kind 或改变读回规则时
重新审视这些分部。
-/
end Sprawling.Privacy
