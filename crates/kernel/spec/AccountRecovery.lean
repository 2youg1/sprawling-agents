-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Error

/-!
# kernel::account_recovery

规定 `account_recovery`（`crates/kernel/src/` 下同名的文件）：一次逻辑请求面对一个 Provider 的有序账号时，先用哪个账号、失败后在原号上再发还是换号、何时停下。本文件是 `crates/kernel/Spec.lean` 的一个分部，节标签 §8-86，别处引作 `crates/kernel/Spec.lean §8-86` 或它的决定 `kernel D54`、`kernel D55`。

本分部先于 Rust 模块写成：`crates/kernel/src/` 下还没有这个文件，模块图在它落地的那个变更里登记一行，形状是 `state machine`。模块落地之前，模型路径的恢复仍由 `runtime::watchdog` 的 `Retry` 三态规则执行（`crates/runtime/spec/Watchdog.lean` §8-9）；这条规则就是下面单账号名册（`Roster.Single`）的处置，所以落地不改变单账号端点的行为（`a_single_account_ignores_the_disposition`）。

模型证明，对任意名册、任意人设上限、任意失败与修复的轨迹：请求错误不再发送；说「换号」的失败不在原号上重发；一个账号一轮里的发送不超过 1＋k；一轮里每个账号至多到一次，总发送不超过 n·(1＋k)，且不超过 `AtMost` 上限＋1；效果不明的请求不换号；Session 的绑定只在回答时移动。

### 8-86 kernel::account_recovery（形状 5 状态机）

```rust
// kernel::account_recovery —— 纯状态机，无 I/O、无时钟；ServerLabel 是账号 id
pub enum AccountRetries { One, Two }                    // 同一账号上最多再发几次；缺省值只在 gateway 的 TuningDefaults::DEFAULTS，Query::Config 读回给页面
pub enum Roster { Single, Several { accounts: Vec<ServerLabel>, retries: AccountRetries } }
                                                        // Several 恒有两个以上账号，名册序即优先级
pub enum RoundEnd { Refused, Unknown, Cap, Exhausted }
pub enum AccountStep { Resend, Switch { to: ServerLabel }, Stop { why: RoundEnd } }
pub struct AccountRound { /* roster、usable、cap: Retries、held、current、tried: BTreeSet、here、total —— 私有 */ }
impl AccountRound {
    /// 一次逻辑请求的第一次发送之前：held 仍在名册里且可兑付即用它，否则名册里第一个可兑付的。
    pub fn start(roster: Roster, usable: BTreeSet<ServerLabel>, cap: Retries, held: Option<ServerLabel>)
        -> Result<AccountRound, RoundEnd>;              // 一个可兑付的都没有：Err(Exhausted)，不发请求
    pub fn current(&self) -> Option<&ServerLabel>;      // Single 答 None
    pub fn next(&self, failure: &AxError) -> AccountStep;   // 只读 failure.retry() 与 failure.account()
    pub fn apply(self, step: &AccountStep) -> AccountRound;  // Stop 不改状态
    pub fn admits_repair(&self) -> bool;                // 修复段换门重发之前问这一句
    pub fn repaired(self) -> AccountRound;              // 修复段真的重发了：Several 记作原号上的一次发送
}
```

`next` 只返回信息、`apply` 只推进状态（命令与查询分离），两个驱动方——模型路径的 `runtime::watchdog` 与 `runtime::run::drive`（`crates/runtime/spec/Watchdog.lean` §8-9），以及 accounting 的 `web_search` 工具——都只执行 `AccountStep`，各自不另写一个重试循环、不另判一次选哪个账号。下面的 Lean 定义就是这些方法的参考语义：`next` 一臂对一臂，`apply`、`admits_repair`、`repaired` 逐字段相同；`tried` 在 Rust 是集合，在模型是到过的顺序（最新的在前），因为证明要数它的长度。

**本轮**（round）是一次逻辑请求：从第一次发送起，到回答、停下或 `Halt` 为止。模型调用每一次 `Model::call` 开一轮，`web_search` 每一次工具调用开一轮。恢复轮次与已用的余额不入账、不持久化：进程重启后的下一次请求从绑定的账号开新的一轮（绑定本身由 Ledger 重建，`crates/gateway/spec/Router.lean` D30）。

**驱动方怎样执行每一种处置**（实现约束，由各驱动方旁的测试守住，不由本模型证明）：

| 处置 | 模型路径（`Watchdog`／`drive`） | `web_search` |
|---|---|---|
| `Resend` | 按退避表与 `retry-after` 等，等待中 `Halt` 照样够得着，再在原号上发 | 同一张退避表，受工具 deadline 与取消约束 |
| `Switch { to }` | 不等，`Model::select_account(to)` 之后立刻发；连续失败的计数归零，因为退避是对那个失败账号的陈述 | 用 `to` 的凭据立刻发 |
| `Stop { Refused }`／`Stop { Cap }` | 原错误原样，冻结原因 `ProviderRefused`（与本模块之前相同） | 原错误作为失败的工具结果交给模型 |
| `Stop { Unknown }` | 原错误原样，带着 `effect_unknown`，冻结原因 `ProviderRefused` | 同上 |
| `Stop { Exhausted }` | 一条 `E_PROVIDER_ACCOUNTS_EXHAUSTED`，carrier 与 `E_PROVIDER` 相同，run 以 `Cancelled` 冻结；subject 列出 Provider 与每个账号 id 及其最后一种失败，不含 Key 与引用；恢复语：补录凭据或额度、或新增账号，然后再派活 | 同一个码作为失败的工具结果 |

**谁看见 `held`**：模型路径的 `held` 是 Session 的绑定（`gateway::router` 的 `EndpointBook` 从 Ledger 折出，`crates/gateway/spec/Router.lean` D30），派活时取它开第一轮，此后每一轮取适配器当前的账号；回答之后 `model_returned` 提交绑定，下一轮从新绑定开始。`web_search` 不记 Session 绑定（D54），每一轮的 `held` 恒缺席，所以总从名册里第一个可兑付的账号开始。
-/

namespace Kernel.AccountRecovery

open Kernel.Error (Retry AccountDisposition)

/-- 同一账号上最多再发几次（`AccountRetries`）。人在 Provider 的高级表单上选一或二；只在名册有两个以上账号时被读。 -/
inductive AccountRetries where
  | One
  | Two
  deriving DecidableEq, Repr

def AccountRetries.count : AccountRetries → Nat
  | .One => 1
  | .Two => 2

/-- 人设的总上限（`kernel::Retries`，`spec/Retries.lean` §8-72）。在一轮里它封住整个逻辑请求跨账号的重发总数。 -/
inductive Retries where
  | UntilHalted
  | AtMost (retries : Nat)
  deriving DecidableEq, Repr

/-- 已经发出 `sent` 次之后，人设的上限还许不许再发一次。与 `Watchdog` 今天的读法相同：`AtMost(0)` 发一次就停。 -/
def Retries.admits : Retries → Nat → Bool
  | .UntilHalted, _ => true
  | .AtMost retries, sent => decide (sent ≤ retries)

/-- 上限守住了：`AtMost m` 之下一轮至多发 m＋1 次。 -/
def Retries.holds : Retries → Nat → Prop
  | .UntilHalted, _ => True
  | .AtMost retries, sent => sent ≤ retries + 1

/-- 一次发送失败时 `next` 读的两格：`AxError::retry` 与 `AxError::account`（`spec/Error.lean` D53）。 -/
structure Failure where
  retry : Retry
  account : AccountDisposition
  deriving DecidableEq, Repr

/-- 这一轮面对的名册。`Single` 是旧的单凭据登记与只列一个账号的登记：没有第二个账号可换，处置照旧（D54）。`Several` 是两个以上账号，名册序就是优先级。 -/
inductive Roster (Account : Type) where
  | Single
  | Several (accounts : List Account) (retries : AccountRetries)

/-- 一轮为什么停下。 -/
inductive RoundEnd where
  | Refused
  | Unknown
  | Cap
  | Exhausted
  deriving DecidableEq, Repr

/-- 一次失败之后的处置：在原号上再发、换到名册里的另一个账号、或停下。 -/
inductive AccountStep (Account : Type) where
  | Resend
  | Switch (to : Account)
  | Stop (why : RoundEnd)
  deriving DecidableEq, Repr

/-- 一轮的状态。`usable` 是凭据能否兑付（vault 里有这个引用，或环境变量提供了它），开轮时读一次；`held` 是开轮时 Session 持有的账号；`tried` 是本轮到过的账号，最新的在前；`here` 是当前账号上的发送数；`total` 是本轮的发送数。 -/
structure AccountRound (Account : Type) where
  roster : Roster Account
  usable : Account → Bool
  cap : Retries
  held : Option Account
  current : Option Account
  tried : List Account
  here : Nat
  total : Nat

variable {Account : Type} [DecidableEq Account]

/-! D54 选号与恢复只有一个家：kernel 的 `AccountRound`

**决定**：一个 Provider 有多个账号时，「先用哪个账号」与「失败后原号再发、换号还是停下」都由 `AccountRound` 一处回答。规则：

1. **没有并列**。名册序就是优先级。开轮时，Session 持有的账号仍在名册里且可兑付就用它，否则用名册里第一个可兑付的；换号时用名册里第一个可兑付而本轮没到过的。
2. **说「换号」的失败（`Advance`）立刻换号**，不在原号上重发：401、额度用尽、该账号引用的凭据缺失（gateway D31）。
3. **说「稍后再问」的失败（`Yes` 且 `Keep`）在原号上再发至多 k 次**（k 取 `AccountRetries`，一或二），用完再换号；连接没建起、408、限流的 429、5xx 与列明的流内错误属于这一族。
4. **效果不明的失败（`Unknown`）只在原号上、在同一份 k 次余额内再发，从不换号**；余额用完以 `Unknown` 停下，原错误带着 `effect_unknown` 交出。
5. **请求错误（`No` 且 `Keep`）不再发**，以 `Refused` 停下。
6. **人设的 `AtMost(m)` 封住整个逻辑请求**：一轮的总发送数不超过 m＋1，跨账号合计；`UntilHalted` 之下停它的是 `Halt`，多账号时还有 n·(1＋k) 这道界。
7. **没有可换的账号时以 `Exhausted` 停下**，`Exhausted` 先于 `Cap` 判：一轮停下的原因若两者都成立，说出账号用尽的那句对人更有用。
8. **单账号名册照旧**：`Single` 不读 `account`，`Yes` 与 `Unknown` 再发直到人设上限，`No` 停下——与本模块之前 `Watchdog` 的规则逐臂相同，修复段的换门重发也照旧不计入上限。k 只在多账号时被读，`request_max_retries` 的含义不变。
9. **搜索账号不做 Session 亲和**：`web_search` 用同一个 `AccountRound`，`held` 恒缺席。亲和是为了保住按账号隔离的 prompt cache，搜索没有这份缓存；不出错时它总落在名册的第一个账号上，本来就不轮换。

**理由**：模型路径（runtime）与搜索路径（accounting）分处两个 crate，`kernel` 是二者唯一共同的内层，`Retries` 也住在这里（§8-72）；两处各写一个循环，重试与选号就各有两个权威。账号名册是有序列表（gateway D28），用上移／下移表达优先级，所以不需要轮询游标：轮询会让新 Session 在账号之间轮换，打掉 prompt cache，而人没有要求分摊用量。`Unknown` 不换号，因为换号会在另一个账号上再计一次费，而丢了回答不证明是账号的问题。单账号照旧，因为单账号端点没有可换的账号，任何新规则对它只会是行为变化。

**被否**：①每个驱动方自己挑「第一个可兑付的账号」再自己重发一次——两个选号权威、两个重试循环；②`Watchdog::on_model_failure(&mut dyn Model)` 既改模型状态又返回处置——违反命令与查询分离，而且把模型路径的写法强加给搜索；③把 `Unknown` 一律改成冻结——改变每一个单账号端点的现行行为（`Watchdog` 的「不知道照样再问」），而这里要的只是请求错误不再发送、效果不明的请求不在别的账号上重发；④整数优先级加 `Active`／`Disabled`／`Revoked` 状态——优先级并列需要游标事件，撤销已经由「提交不含该账号的完整列表」表达（gateway D28）。

**重开参数**：人要求在账号之间分摊用量；或出现一种失败，`Unknown` 时能从对端读出效果是否落地。
-/

/-- 开轮时选哪个账号：持有的账号仍在名册里且可兑付就用它，否则名册里第一个可兑付的（D54 第 1 条）。 -/
def opening (accounts : List Account) (usable : Account → Bool) : Option Account → Option Account
  | some account => if account ∈ accounts ∧ usable account = true then some account else accounts.find? usable
  | none => accounts.find? usable

/-- `AccountRound::start`：第一次发送之前开轮。开轮即记下第一次发送；名册里一个可兑付的都没有时不发请求，答 `Exhausted`。 -/
def AccountRound.start (roster : Roster Account) (usable : Account → Bool) (cap : Retries)
    (held : Option Account) : Except RoundEnd (AccountRound Account) :=
  match roster with
  | .Single => .ok { roster, usable, cap, held, current := none, tried := [], here := 1, total := 1 }
  | .Several accounts _ =>
    match opening accounts usable held with
    | some first =>
      .ok { roster, usable, cap, held, current := some first, tried := [first], here := 1, total := 1 }
    | none => .error .Exhausted

/-! D55 凭据能否兑付在开轮时读一次；一个账号的凭据缺失只让这个账号出局，vault 本身的故障让整轮停下

**决定**：`usable` 是开轮时对名册里每个账号问一次「它的引用此刻能否兑付」（vault 里有这个条目，或环境变量提供了它；不兑付、不读 Key 的值），开轮与换号都跳过不可兑付的账号。发送时兑付仍可能失败：引用在 vault 里不见了，端点的兑付处（gateway D31）把那条 `E_CREDENTIAL_MISSING` 标成 `Advance`，于是只有这个账号出局；vault 锁住或不可用的错误不标 `Advance`，以 `Refused` 停下整轮。

**理由**：一个账号的引用缺失是这个账号的事，换下一个账号能修好；vault 锁住是所有账号共用的存储的事，换号只会把同一个错误在每个账号上各买一遍，最后报出的「账号用尽」也指错了出路。开轮时先问一次，免得把一次注定失败的兑付算作一次发送。

**被否**：①开轮不问、全靠发送时的 `E_CREDENTIAL_MISSING` 换号——结果相同，但每个缺凭据的账号都要占掉一次发送与一行 `model_called`；②给「整轮停下」另设一种 `AccountDisposition`——vault 故障本来就是 `No` 且 `Keep`，`Refused` 已经说出「不再发」，第三臂没有新的处置可给。

**重开参数**：出现一种存储，部分账号的凭据可读而其余不可读（例如按账号分开的硬件密钥）。
-/

/-- 可以换去的账号：可兑付，且本轮没到过。 -/
def AccountRound.fresh (round : AccountRound Account) (account : Account) : Bool :=
  round.usable account && !decide (account ∈ round.tried)

/-- 换号时的目标：名册里第一个可以换去的账号。 -/
def AccountRound.successor (round : AccountRound Account) : Option Account :=
  match round.roster with
  | .Single => none
  | .Several accounts _ => accounts.find? round.fresh

/-- 在原号上再发，除非人设的上限不许。 -/
def AccountRound.resend (round : AccountRound Account) : AccountStep Account :=
  if round.cap.admits round.total then .Resend else .Stop .Cap

/-- 换号；没有可换的账号即 `Exhausted`，它先于上限判（D54 第 7 条）。 -/
def AccountRound.switch (round : AccountRound Account) : AccountStep Account :=
  match round.successor with
  | none => .Stop .Exhausted
  | some to => if round.cap.admits round.total then .Switch to else .Stop .Cap

/-- `AccountRound::next`：一次失败之后的处置，一种 (`retry`, `account`) 一臂（D54）。`Advance` 只与 `No` 同行（`spec/Error.lean` D53），其余组合照样有答案，所以 `next` 是全函数。 -/
def AccountRound.next (round : AccountRound Account) (failure : Failure) : AccountStep Account :=
  match round.roster with
  | .Single =>
    match failure.retry with
    | .No => .Stop .Refused
    | .Yes => round.resend
    | .Unknown => round.resend
  | .Several _ retries =>
    match failure.retry, failure.account with
    | .Unknown, _ => if round.here ≤ retries.count then round.resend else .Stop .Unknown
    | .Yes, .Advance => round.switch
    | .No, .Advance => round.switch
    | .Yes, .Keep => if round.here ≤ retries.count then round.resend else round.switch
    | .No, .Keep => .Stop .Refused

/-- `AccountRound::apply`：驱动方执行了处置之后推进状态。`Stop` 不改状态。 -/
def AccountRound.apply (round : AccountRound Account) : AccountStep Account → AccountRound Account
  | .Resend => { round with here := round.here + 1, total := round.total + 1 }
  | .Switch to =>
    { round with current := some to, tried := to :: round.tried, here := 1, total := round.total + 1 }
  | .Stop _ => round

/-- `AccountRound::admits_repair`：修复段（`crates/runtime/spec/Turn/Recovery.lean` 的 `BlockingResend`）换阻塞门重发之前问这一句。多账号时那次重发是原号上的一次发送，要原号还有余额、人设的上限还许；单账号照旧放行（D54 第 8 条）。 -/
def AccountRound.admits_repair (round : AccountRound Account) : Bool :=
  match round.roster with
  | .Single => true
  | .Several _ retries => decide (round.here ≤ retries.count) && round.cap.admits round.total

/-- `AccountRound::repaired`：修复段真的重发了。多账号记作原号上的一次发送；单账号照旧不计。 -/
def AccountRound.repaired (round : AccountRound Account) : AccountRound Account :=
  match round.roster with
  | .Single => round
  | .Several _ _ => round.apply .Resend

/-- 一次回答绑定的账号：答出这次请求的当前账号；单账号名册没有账号可绑，绑定不动。 -/
def AccountRound.answered (round : AccountRound Account) : Option Account :=
  round.current.or round.held

/-- 驱动方在一轮里看见的事：一次发送失败了，或修复段想换门重发。 -/
inductive RoundEvent where
  | failed (failure : Failure)
  | repair
  deriving DecidableEq, Repr

/-- 驱动方照处置执行一条轨迹：失败问 `next`，停下即止；修复先问 `admits_repair`，不放行时修复段把原错误交给下一段，轮的状态不动。 -/
def AccountRound.walk (round : AccountRound Account) :
    List RoundEvent → AccountRound Account × Option RoundEnd
  | [] => (round, none)
  | .repair :: rest => (if round.admits_repair then round.repaired else round).walk rest
  | .failed failure :: rest =>
    match round.next failure with
    | .Stop why => (round, some why)
    | step => (round.apply step).walk rest

/-- 多账号一轮的不变量：到过的账号不重复且都在名册里，当前账号是最后到的那个，当前账号上的发送不超过 1＋k，总发送数不超过「之前每个账号各 1＋k，加上当前账号上的」。 -/
def Sound (accounts : List Account) (retries : AccountRetries) (round : AccountRound Account) : Prop :=
  round.tried.Nodup ∧ (∀ account, account ∈ round.tried → account ∈ accounts) ∧
  round.current = round.tried.head? ∧ round.tried ≠ [] ∧ round.here ≤ 1 + retries.count ∧
  round.total + (1 + retries.count) ≤ round.tried.length * (1 + retries.count) + round.here

/-- 一轮的不变量：人设的上限守住了；多账号时还有上面那几条。 -/
def AccountRound.Sound (round : AccountRound Account) : Prop :=
  round.cap.holds round.total ∧
  match round.roster with
  | .Single => True
  | .Several accounts retries => Kernel.AccountRecovery.Sound accounts retries round

omit [DecidableEq Account] in
theorem resend_sound (round : AccountRound Account) (h : round.Sound)
    (fits : round.cap.admits round.total = true)
    (within : ∀ accounts retries, round.roster = .Several accounts retries → round.here ≤ retries.count) :
    (round.apply .Resend).Sound := by
  obtain ⟨cap, shape⟩ := h
  refine ⟨?_, ?_⟩
  · cases hc : round.cap with
    | UntilHalted => simp [AccountRound.apply, Retries.holds, hc]
    | AtMost m =>
      simp [Retries.admits, hc] at fits
      simp [AccountRound.apply, Retries.holds, hc]
      omega
  · cases hr : round.roster with
    | Single => simp [AccountRound.apply, hr]
    | Several accounts retries =>
      have w := within accounts retries hr
      simp only [hr] at shape
      obtain ⟨nodup, inside, head, nonempty, here, total⟩ := shape
      simp only [AccountRound.apply, hr, Kernel.AccountRecovery.Sound]
      exact ⟨nodup, inside, head, nonempty, by omega, by omega⟩

theorem successor_fresh (round : AccountRound Account) (to : Account)
    (found : round.successor = some to) :
    to ∉ round.tried ∧ round.usable to = true ∧
      ∃ accounts retries, round.roster = .Several accounts retries ∧ to ∈ accounts := by
  unfold AccountRound.successor at found
  cases hr : round.roster with
  | Single => simp [hr] at found
  | Several accounts retries =>
    simp only [hr] at found
    have fresh := List.find?_some found
    have mem := List.mem_of_find?_eq_some found
    simp [AccountRound.fresh] at fresh
    exact ⟨fresh.2, fresh.1, accounts, retries, rfl, mem⟩

theorem switch_sound (round : AccountRound Account) (h : round.Sound)
    (fits : round.cap.admits round.total = true) (to : Account)
    (found : round.successor = some to) :
    (round.apply (.Switch to)).Sound := by
  obtain ⟨notin, _, accounts, retries, hr, mem⟩ := successor_fresh round to found
  obtain ⟨cap, shape⟩ := h
  refine ⟨?_, ?_⟩
  · cases hc : round.cap with
    | UntilHalted => simp [AccountRound.apply, Retries.holds, hc]
    | AtMost m =>
      simp [Retries.admits, hc] at fits
      simp [AccountRound.apply, Retries.holds, hc]
      omega
  · simp only [hr] at shape
    obtain ⟨nodup, inside, head, nonempty, here, total⟩ := shape
    simp only [AccountRound.apply, hr, Kernel.AccountRecovery.Sound]
    refine ⟨List.nodup_cons.mpr ⟨notin, nodup⟩, ?_, rfl, List.cons_ne_nil _ _, by omega, ?_⟩
    · intro account held
      cases List.mem_cons.mp held with
      | inl same => exact same ▸ mem
      | inr earlier => exact inside account earlier
    · simp only [List.length_cons, Nat.add_mul, Nat.one_mul]
      omega

/-- 处置只有两种会再发：`Resend` 要人设的上限还许、且（多账号时）原号还有余额；`Switch` 要上限还许、且目标是名册里下一个可兑付而本轮没到过的账号。 -/
theorem next_moves (round : AccountRound Account) (failure : Failure) :
    (round.next failure = .Resend → round.cap.admits round.total = true ∧
      ∀ accounts retries, round.roster = .Several accounts retries → round.here ≤ retries.count) ∧
    (∀ to, round.next failure = .Switch to →
      round.cap.admits round.total = true ∧ round.successor = some to) := by
  rcases failure with ⟨retry, account⟩
  unfold AccountRound.next
  cases hr : round.roster with
  | Single =>
    cases retry <;>
      simp [AccountRound.resend] <;> split <;> simp_all
  | Several accounts retries =>
    cases retry <;> cases account <;>
      simp [AccountRound.resend, AccountRound.switch] <;>
      (repeat' split) <;> simp_all

theorem walk_sound (round : AccountRound Account) (h : round.Sound) (events : List RoundEvent) :
    (round.walk events).1.Sound := by
  induction events generalizing round with
  | nil => exact h
  | cons event rest ih =>
    cases event with
    | repair =>
      simp only [AccountRound.walk]
      apply ih
      split
      · rename_i admitted
        unfold AccountRound.repaired
        cases hr : round.roster with
        | Single => exact h
        | Several accounts retries =>
          simp only [AccountRound.admits_repair, hr, Bool.and_eq_true, decide_eq_true_eq] at admitted
          apply resend_sound round h admitted.2
          intro accounts' retries' same
          rw [hr] at same
          cases same
          exact admitted.1
      · exact h
    | failed failure =>
      simp only [AccountRound.walk]
      have moves := next_moves round failure
      split
      · exact h
      · rename_i notStop
        apply ih
        generalize decided : round.next failure = step at notStop moves ⊢
        cases step with
        | Resend =>
          obtain ⟨fits, within⟩ := moves.1 rfl
          exact resend_sound round h fits within
        | Switch to =>
          obtain ⟨fits, found⟩ := moves.2 to rfl
          exact switch_sound round h fits to found
        | Stop why => exact absurd rfl (notStop why)

theorem walk_keeps (round : AccountRound Account) (events : List RoundEvent) :
    (round.walk events).1.roster = round.roster ∧ (round.walk events).1.held = round.held ∧
      (round.walk events).1.cap = round.cap := by
  induction events generalizing round with
  | nil => exact ⟨rfl, rfl, rfl⟩
  | cons event rest ih =>
    cases event with
    | repair =>
      simp only [AccountRound.walk]
      split
      · obtain ⟨roster, held, cap⟩ := ih round.repaired
        unfold AccountRound.repaired at roster held cap ⊢
        cases hr : round.roster <;> simp_all [AccountRound.apply]
      · exact ih round
    | failed failure =>
      simp only [AccountRound.walk]
      split
      · exact ⟨rfl, rfl, rfl⟩
      · obtain ⟨roster, held, cap⟩ := ih (round.apply (round.next failure))
        generalize round.next failure = step at roster held cap ⊢
        cases step <;> simp_all [AccountRound.apply]

private theorem nodup_length_le :
    ∀ (inner outer : List Account), inner.Nodup → (∀ a, a ∈ inner → a ∈ outer) →
      inner.length ≤ outer.length
  | [], _, _, _ => Nat.zero_le _
  | a :: rest, outer, nodup, inside => by
    obtain ⟨notin, nodup'⟩ := List.nodup_cons.mp nodup
    have present : a ∈ outer := inside a (List.mem_cons_self ..)
    have smaller : ∀ b, b ∈ rest → b ∈ outer.erase a := by
      intro b held
      have other : b ≠ a := fun same => notin (same ▸ held)
      exact (List.mem_erase_of_ne other).mpr (inside b (List.mem_cons_of_mem _ held))
    have shorter := nodup_length_le rest (outer.erase a) nodup' smaller
    have erased := List.length_erase_of_mem present
    have positive : 0 < outer.length := List.length_pos_of_mem present
    simp only [List.length_cons]
    omega

theorem opening_usable (accounts : List Account) (usable : Account → Bool) (held : Option Account)
    (first : Account) (opened : opening accounts usable held = some first) :
    first ∈ accounts ∧ usable first = true := by
  cases held with
  | none => exact ⟨List.mem_of_find?_eq_some opened, List.find?_some opened⟩
  | some account =>
    simp only [opening] at opened
    split at opened
    · rename_i keeps
      cases opened
      exact keeps
    · exact ⟨List.mem_of_find?_eq_some opened, List.find?_some opened⟩

theorem start_sound (roster : Roster Account) (usable : Account → Bool) (cap : Retries)
    (held : Option Account) (round : AccountRound Account)
    (started : AccountRound.start roster usable cap held = .ok round) : round.Sound := by
  have opens : cap.holds 1 := by cases cap <;> simp [Retries.holds]
  unfold AccountRound.start at started
  cases roster with
  | Single =>
    cases started
    exact ⟨opens, trivial⟩
  | Several accounts retries =>
    simp only at started
    split at started
    · rename_i first opened
      cases started
      have ⟨mem, _⟩ := opening_usable accounts usable held first opened
      refine ⟨opens, ?_⟩
      simp only [Kernel.AccountRecovery.Sound]
      refine ⟨List.nodup_cons.mpr ⟨List.not_mem_nil, List.nodup_nil⟩, ?_, rfl, List.cons_ne_nil _ _,
        by omega, by simp only [List.length_singleton]; omega⟩
      intro account held'
      rw [List.mem_singleton.mp held']
      exact mem
    · cases started

/-! ## 性质 -/

/-- **请求错误不再发送。** 不论名册、预算与本轮走到哪里，`No` 且 `Keep` 的失败一律停下：400、403、404、422、窗口溢出、读不懂的回答、本侧造不出的请求，以及 vault 锁住或不可用（它不标 `Advance`，见 D55）。 -/
theorem a_request_error_is_never_sent_again (round : AccountRound Account) :
    round.next ⟨.No, .Keep⟩ = .Stop .Refused := by
  unfold AccountRound.next
  cases round.roster <;> rfl

/-- **说「换号」的失败从不在原号上重发，换到的也不是本轮到过的账号。** 只看 `No` 且 `Advance`：`spec/Error.lean` 的 `no_order_of_calls_advances_a_resendable_failure` 证明错误的构造路径造不出别的 `Advance`。 -/
theorem an_unusable_account_is_never_asked_again (round : AccountRound Account) (h : round.Sound) :
    round.next ⟨.No, .Advance⟩ ≠ .Resend ∧
      ∀ to, round.next ⟨.No, .Advance⟩ = .Switch to → to ∉ round.tried ∧ round.current ≠ some to := by
  have moves := next_moves round ⟨.No, .Advance⟩
  refine ⟨?_, ?_⟩
  · unfold AccountRound.next AccountRound.switch
    cases round.roster with
    | Single => simp
    | Several accounts retries =>
      simp only
      split
      · simp
      · split <;> simp
  · intro to switched
    obtain ⟨_, found⟩ := moves.2 to switched
    obtain ⟨notin, _, accounts, retries, hr, _⟩ := successor_fresh round to found
    refine ⟨notin, ?_⟩
    intro same
    obtain ⟨_, shape⟩ := h
    simp only [hr, Kernel.AccountRecovery.Sound] at shape
    obtain ⟨_, _, head, _⟩ := shape
    rw [same] at head
    exact notin (List.mem_of_mem_head? head.symm)

/-- **效果不明的请求从不换号。** 在另一个账号上重发会在那个账号上再计一次费，而丢了回答不证明是账号的问题；它只在原号上、在原号的余额内重发（D54）。 -/
theorem an_unknown_effect_never_switches (round : AccountRound Account)
    (account : AccountDisposition) (to : Account) :
    round.next ⟨.Unknown, account⟩ ≠ .Switch to := by
  unfold AccountRound.next AccountRound.resend
  cases round.roster with
  | Single => simp only; split <;> simp
  | Several accounts retries =>
    cases account <;> simp only <;> (repeat' split) <;> simp

/-- **一个账号在一轮里的发送数不超过 1＋k**，k 是 `AccountRetries` 的次数；修复段换门的重发也算在内。 -/
theorem sends_on_one_account_stay_within_the_budget (round : AccountRound Account) (h : round.Sound)
    (accounts : List Account) (retries : AccountRetries) (hr : round.roster = .Several accounts retries)
    (events : List RoundEvent) :
    (round.walk events).1.here ≤ 1 + retries.count := by
  obtain ⟨_, shape⟩ := walk_sound round h events
  rw [(walk_keeps round events).1, hr] at shape
  exact shape.2.2.2.2.1

/-- **一轮里每个账号至多到一次，总发送数不超过 n·(1＋k)**，n 是名册的长度。 -/
theorem a_round_visits_each_account_once (round : AccountRound Account) (h : round.Sound)
    (accounts : List Account) (retries : AccountRetries) (hr : round.roster = .Several accounts retries)
    (events : List RoundEvent) :
    (round.walk events).1.tried.Nodup ∧
      (round.walk events).1.total ≤ accounts.length * (1 + retries.count) := by
  obtain ⟨_, shape⟩ := walk_sound round h events
  rw [(walk_keeps round events).1, hr] at shape
  obtain ⟨nodup, inside, _, _, here, total⟩ := shape
  have visits := nodup_length_le _ accounts nodup inside
  have scaled := Nat.mul_le_mul_right (1 + retries.count) visits
  exact ⟨nodup, by omega⟩

/-- **人设的上限 `AtMost m` 封住整个逻辑请求：一轮的总发送数不超过 m＋1**，跨账号合计。 -/
theorem the_cap_bounds_every_round (round : AccountRound Account) (h : round.Sound)
    (retries : Nat) (hc : round.cap = .AtMost retries) (events : List RoundEvent) :
    (round.walk events).1.total ≤ retries + 1 := by
  obtain ⟨cap, _⟩ := walk_sound round h events
  rw [(walk_keeps round events).2.2, hc] at cap
  exact cap

/-- **绑定只在回答时移动。** 任意失败与修复的轨迹都不改动 Session 持有的账号；一轮以 `Exhausted` 或别的原因停下时，绑定留在上一次成功的账号上。 -/
theorem the_binding_moves_only_on_an_answer (round : AccountRound Account) (events : List RoundEvent) :
    (round.walk events).1.held = round.held :=
  (walk_keeps round events).2.1

omit [DecidableEq Account] in
/-- 回答绑定的是答出这次请求的账号：名册里的、本轮到过的那一个。 -/
theorem an_answer_binds_the_account_that_answered (round : AccountRound Account) (h : round.Sound)
    (accounts : List Account) (retries : AccountRetries) (hr : round.roster = .Several accounts retries) :
    ∃ account, round.answered = some account ∧ round.current = some account ∧ account ∈ accounts := by
  obtain ⟨_, shape⟩ := h
  simp only [hr, Kernel.AccountRecovery.Sound] at shape
  obtain ⟨_, inside, head, nonempty, _⟩ := shape
  cases tried : round.tried with
  | nil => exact absurd tried nonempty
  | cons first rest =>
    rw [tried] at head inside
    refine ⟨first, ?_, head, inside first (List.mem_cons_self ..)⟩
    simp [AccountRound.answered, head]

/-- **Session 亲和**：持有的账号仍在名册里且可兑付，新的一轮就从它开始，不论它排在第几。 -/
theorem a_held_account_opens_the_round (accounts : List Account) (retries : AccountRetries)
    (usable : Account → Bool) (cap : Retries) (account : Account)
    (listed : account ∈ accounts) (redeemable : usable account = true) :
    ∃ round, AccountRound.start (.Several accounts retries) usable cap (some account) = .ok round ∧
      round.current = some account := by
  simp [AccountRound.start, opening, listed, redeemable]

/-- 名册里没有一个可兑付的账号时，一轮不发请求就以 `Exhausted` 停下。 -/
theorem a_roster_with_nothing_redeemable_sends_nothing (accounts : List Account)
    (retries : AccountRetries) (usable : Account → Bool) (cap : Retries) (held : Option Account)
    (none_usable : ∀ account, account ∈ accounts → usable account = false) :
    AccountRound.start (.Several accounts retries) usable cap held = .error .Exhausted := by
  have absent : opening accounts usable held = none := by
    cases opened : opening accounts usable held with
    | none => rfl
    | some first =>
      have ⟨mem, ok⟩ := opening_usable accounts usable held first opened
      simp [none_usable first mem] at ok
  simp [AccountRound.start, absent]

/-- **单账号端点照旧**：没有第二个账号可换时，`account` 那一格不改变任何处置，处置只由 `retry` 与人设的上限决定，与这一模块出现之前 `Watchdog` 的规则相同（D54）。 -/
theorem a_single_account_ignores_the_disposition (round : AccountRound Account)
    (single : round.roster = .Single) (retry : Retry) :
    round.next ⟨retry, .Advance⟩ = round.next ⟨retry, .Keep⟩ := by
  unfold AccountRound.next
  rw [single]

/-! ## 轨迹向量

下面的 `#eval` 打印给 Rust 逐条重放的轨迹向量：每一条给出名册、不可兑付的账号、人设上限、开轮时持有的账号与一串事件，答出开轮选中的账号与每个事件之后的处置（修复行的处置是 `Resend`，或缺席表示不放行）。Rust 侧的检查住在 `account_recovery` 模块旁的测试里，与本分部的定理一同点名本文件。 -/

/-- 一条轨迹的处置序列；停下即止。 -/
def AccountRound.decisions (round : AccountRound Account) :
    List RoundEvent → List (RoundEvent × Option (AccountStep Account))
  | [] => []
  | .repair :: rest =>
    if round.admits_repair then (.repair, some .Resend) :: round.repaired.decisions rest
    else (.repair, none) :: round.decisions rest
  | .failed failure :: rest =>
    match round.next failure with
    | .Stop why => [(.failed failure, some (.Stop why))]
    | step => (.failed failure, some step) :: (round.apply step).decisions rest

/-- 一条向量：开轮，然后走事件。 -/
def vector (roster : Roster String) (missing : List String) (cap : Retries) (held : Option String)
    (events : List RoundEvent) : Option (Option String × List (RoundEvent × Option (AccountStep String))) :=
  match AccountRound.start roster (fun account => !missing.contains account) cap held with
  | .ok round => some (round.current, round.decisions events)
  | .error _ => none

/-- 限流的 429：原号再发两次，第三次失败换到 b。 -/
def busy : RoundEvent := .failed ⟨.Yes, .Keep⟩
/-- 401、额度用尽、凭据缺失。 -/
def unusable : RoundEvent := .failed ⟨.No, .Advance⟩
/-- 400 一类的请求错误。 -/
def refused : RoundEvent := .failed ⟨.No, .Keep⟩
/-- 断流：效果不明。 -/
def lost : RoundEvent := .failed ⟨.Unknown, .Keep⟩

#eval vector (.Several ["a", "b"] .Two) [] .UntilHalted none [busy, busy, busy, busy]
#eval vector (.Several ["a", "b"] .Two) [] .UntilHalted none [unusable, unusable]
#eval vector (.Several ["a", "b"] .Two) [] .UntilHalted none [refused]
#eval vector (.Several ["a", "b"] .Two) [] .UntilHalted none [lost, lost, lost]
#eval vector (.Several ["a", "b"] .Two) [] (.AtMost 1) none [busy, busy]
#eval vector (.Several ["a", "b", "c"] .One) ["b"] .UntilHalted (some "c") [busy, busy, unusable]
#eval vector (.Several ["a", "b"] .One) [] .UntilHalted (some "b") [.repair, busy, .repair]
#eval vector (.Several ["a", "b"] .Two) ["a", "b"] .UntilHalted none []
#eval vector .Single [] (.AtMost 2) none [busy, lost, busy]
#eval vector .Single [] .UntilHalted none [.repair, unusable]

/-- 正常路径可实现：两个账号、b 被持有，开轮落在 b，一次限流之后在 b 上再发。 -/
example : vector (.Several ["a", "b"] .Two) [] .UntilHalted (some "b") [busy] =
    some (some "b", [(busy, some .Resend)]) := by
  decide

end Kernel.AccountRecovery
