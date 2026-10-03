-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::gate

规定 `kernel::gate`（`crates/kernel/src/gate.rs` 与 `crates/kernel/src/gate/` 下的每一道门）：门、门册与拒绝矩阵。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-27 kernel::gate

```rust
pub enum GateOutcome { Allow, Deny { refusal: Box<AxError> }, Ask { question: Box<AxError> } }
// Ask 是具名例外：门答不了而人答得了，`question` 恒 `E_APPROVAL_PENDING` 且三段齐全，
// recovery 就是那句要人做的事。DOORS 遍历断言「会问人的门恰有一道」。

pub fn domain(domain: &WriteDomain, target: &Address, taint: &TaintSet) -> GateOutcome;   // 判一个文件
pub fn reach(domain: &WriteDomain, area: &Address, taint: &TaintSet) -> GateOutcome;      // 判一块声明的区域
pub enum EgressTarget { Loopback, Private, Public { host: String },
                        Connector { label: ServerLabel } }   // 分类由效果层解好址后注入
pub enum EgressOutcome { Allow { first_public_egress: bool }, Deny { refusal: Box<AxError> } }
pub fn egress(spans: &[SecretSpan], target: &EgressTarget, prior_public_egress: bool) -> EgressOutcome;
pub fn egress_target(list: &EgressAllowlist, target: &EgressTarget) -> EgressOutcome;
pub fn discard(req: &DiscardRequest, action_desc: &str) -> GateOutcome;
pub fn spawn(parent: Depth, kind: &DelegateKind) -> GateOutcome;      // E_DELEGATION_DEPTH 的塑形处
pub struct ConnectorCall<'a> { pub label: &'a ServerLabel, pub tool: &'a ToolName }
pub fn reaches_the_undoable(call: &ConnectorCall<'_>) -> bool;
pub fn undoable(call: &ConnectorCall<'_>, sandbox: &SandboxLimits, taint: &TaintSet) -> GateOutcome;
pub fn command(taint: &TaintSet) -> GateOutcome;                     // exec 的门：非空 taint 恒 Deny
pub fn attach(endpoint: Option<&EgressTarget>) -> GateOutcome;        // 唯一会 Ask 的门
pub fn host_of(url: &str) -> Result<Option<String>, AxError>;         // url 里的主机，全库一份
pub fn target_of(host: &str) -> EgressTarget;                         // Loopback／Private／Public 的唯一判定

pub enum DoorId { Domain, Reach, Egress, EgressHost, Discard, Spawn, Undoable, Attach, Command, Replacing }
pub const DOORS: [DoorId; 10];
impl DoorId { pub fn as_str(self) -> &'static str; }
#[cfg(feature = "conformance")]
pub mod conformance {
    pub fn sample(door: DoorId) -> Result<GateOutcome, DoorId>;    // 门自己的那个非 Allow 答案
    pub fn taint_readers() -> BTreeMap<DoorId, bool>;              // 哪几道门真的按 taint 改答
}
```

- **门只答 Allow 或 Deny（D1「默认 YOLO」这条规则），`attach` 是唯一的具名例外**：一个需要人点「可以」的动作，要么本来就该做，要么本来就不该做，两者都是规则。随之删去 `GateOutcome::Escalate`、`GateContext`、`gate::item`、`gate::commitment`、`gate::govern`、`gate::delegation`、`gate::dedup`。`attach` 例外所授予的不是一个动作，是那个人自己的浏览器里**全部登录态的读取权**——邮箱、银行、公司后台；`browser::Profile` 整套按楼隔离在附着的那一刻全部失效，所以没有任何一条城的规则答得了它，而人的动作（在运行中的浏览器里亲自打开远程调试并声明地址）就是答案。`attach` 的 Ask 经 bench 原样回到模型：`E_APPROVAL_PENDING` 加一句 recovery，那个 run 不往下走，而人在城之外完成授权。**会问人的门有且只有这一道，数量本身是一条可断言的性质**：`refusal_matrix` 遍历 `DOORS` 断言 Ask 恰好一条。今天六类升级各得的固定答案与其理由：

  | 类别 | 答案 | 为什么 | 人在哪里改 |
  |---|---|---|---|
  | `Commitment` | Allow（无门） | 承诺一个计划是居民自己的工作，账本记录 | 账本与 `status` |
  | `DiscardEscalate` | Allow（`gate::discard` 只拦无还原与污染） | `Discard` 没有 `Restoration` 就拼不出来，任何删除都能回滚 | 楼的 write domain |
  | `Delegation` | Allow（无门；`spawn` 仍拦深度） | 委派一层深由类型保证 | — |
  | `BudgetLimit` | Deny（无门；预算耗尽冻结 run） | 预算是 `CONFIG.toml` 里的规则，超了就冻 | `CONFIG.toml` 的 `[budget]` |
  | `Governance` | Deny（无门；`Effect::Govern` 在效果层拒） | 一个 run 不得改写审判它自己的规则 | 人改 TOML |
  | `Undoable` | 按楼层 `[sandbox] trusted` 答 Allow／Deny；tainted 恒 Deny | 已有的 reach 规则，不再多一道问 | `CONFIG.toml` 的 `[sandbox] trusted` |

- **门是数据面**：`DOORS` 是门册，refusal 矩阵遍历它而不是一道一道点名；`conformance::sample` 对 `DoorId` 穷尽匹配，于是新增一道门而不给样本编译不过。样本调用真门，矩阵判的是一个 run 会收到的那条拒绝。
- **gate 是全库唯一 gate 码生产者**：每一道门的 Deny 恒经 `AxError::refusal`（三段必填）；Domain 门 nearby＝domain 前缀表；Undoable 门 nearby＝该楼层信任的连接器表；Discard 门 alternative 恒可执行；Egress 门 subject 只写位置与跨度数，恒不回显命中字节。
- **Taint 有真判决**：`gate::undoable` 对非空 taint 恒 Deny（`E_TAINTED_ACTION`），信任与否都拦——楼层信任的是连接器，不是一张网页借它按下的键；`gate::discard` 对非空 taint 恒 Deny；`gate::command` 对非空 taint 恒 Deny——外来内容启动的 run 不跑命令，因为一条命令能做的事没有哪道门能逐项预判，而 Ask 会把一张网页写下的命令原样递给人去按「准」。`conformance::taint_readers` 是这条不变量的机器面，单测断言它每一项为真，于是「taint 只往拒绝文案里加一句」这种恒假分支回不来。
- **首次公网出网**：`egress` 对 Public 且 `!prior_public_egress` 置 `first_public_egress`；NetNotice 挂信封属 pipeline。Loopback/Private 恒不触发。
- `tests/refusal_matrix.rs` 与 `gate/tests.rs` 守门组合 fail-closed：每道门每条 Deny 路径的拒绝三段非空，tainted 的调用在 `undoable`／`discard`／`command` 恒 Deny。
-/

/-!
### 8-49 kernel::gate::undoable：拿不回来的那一类外部效应（形状 1 判定）

```rust
/// 「哪一台 server 上的哪一件工具」——这两样恒同行，故是一个有名字的值。
pub struct ConnectorCall<'a> { pub label: &'a ServerLabel, pub tool: &'a ToolName }

/// 一件 connector 工具的远端名字，是不是这座城收不回来的那一类。
pub fn reaches_the_undoable(call: &ConnectorCall<'_>) -> bool;

/// 够得着运行这座城的机器桌面的调用：楼层信任该连接器且参数不带 taint 才放行，否则拒。
pub fn undoable(call: &ConnectorCall<'_>, sandbox: &SandboxLimits, taint: &TaintSet) -> GateOutcome;
```

**问题**：城里每一条「会造成后果」的路径都配了一条回头路——`Discard` 没有 `Restoration` 就构造不出来，工具波前后各有一个 git checkpoint，写域外的写会被拒。桌面连接器一条都对不上：`desktop.act` 在这个人自己的机器上按下的键，`desktop.clipboard` 覆盖掉的那段文本，城里没有任何一处存过它们的旧值，也没有任何一处能把它们放回去。

**所以答案取自楼层事先写下的信任，而不是当场问人**（D1「默认 YOLO」）：`CONFIG.toml` 的 `[sandbox] trusted` 列了这个连接器就放行，没列就拒（`E_GATE_DENIED`，替代路径指向那张表）。一条写在配置里的信任是人看得见整张表时做的决定。**taint 压过信任**：参数来自外来内容的调用恒拒，因为楼层信任的是连接器，不是一张网页借它按下的键。

**依据是远端名字的前缀 `desktop.`，且判得精确而不是猜**：一件 connector 工具在城里的名字是 `{label}_{sanitise(远端名)}`，`label` 就在 `Effect::Connector` 里带着，所以把 `{label}_` 从头上摘掉剩下的就是远端名，无须猜。一栋楼把这台 server 挂成 `desk`，工具叫 `desk_desktop_act`；挂成 `desktop`，工具叫 `desktop_desktop_act`——两种都判得出来，而「名字里含 desktop」这种读法会把一栋楼自己写的 `notes_desktop_layout` 也判进去。

**`ConnectorCall` 是一个值而不是两个参数**：label 与工具名单独拿出来都判不了任何事——依据恰恰是「把 label 从工具名头上摘掉之后剩下什么」，故它们是同一个事实的两半。

**信任按 label 记，不按工具名记**：楼层回答的是「这个连接器可以碰运行中的机器吗」，一个连接器一行。

**这道门与 `RULES.toml` 的 `desktop` 是两回事，次序也固定**（`crates/city/Spec.lean` §8-25）：楼那一位开关决定这台 server **接不接得上**，这道门决定接上之后**每一次调用放不放行**。

**恒不为它新增 `Effect` 变体**：`Effect` 是路由字段，`Connector` 已经把这一类调用路由到出网门了；再加一格会让每一处 `match Effect` 都要回答一个与它无关的问题。这道门叠在出网门之后，两道各答各的——出网门答「这些字节能出去吗」，本门答「这个后果收得回来吗」。
-/

/-! D1 定规：默认 YOLO，门只答放行或拦住

这一条是人定的。

**决定**：删 `GateOutcome::Escalate`；今天会升级问人的六类各得一个固定答案（表在 §8-27）；Inbox只装设计问题（§8-21）；`Autonomy` 二态、默认 `Owner`。

**理由**：一个默认被绕过的闸是死代码加假安全感。一个需要人点「可以」的动作，要么本来就该做，要么本来就不该做，两者都是规则；规则写在人改得动的地方（`CONFIG.toml` 的 `[budget]` 与 `[sandbox] trusted`、楼的 `RULES.toml`），而不是每次会话问四遍。人的精力应当花在只有人答得了的那一类上：设计问题。

**被否**：①「clerk 代答一切审批」——把人该做的规则判断交给模型每次重新猜一遍，多一次模型调用换一个本来就该是常量的答案；②「保留 `Escalate` 但默认放行」——名义上 YOLO，实际上每次会话问人四次。

**同集删净**：`GateContext`、`gate::item`、`gate::commitment`、`gate::govern`、`gate::delegation`、`gate::dedup`（由 `idem::claim` 与 `IdemGuard` 接替，§8-6）、`PolicyClass`／`PolicyMatcher`／`Policy`／`PolicyApplication`／`PolicyExpiry`／`PolicyRevocation`／`match_item`／`expiry`／`POLICY_IDLE_DAYS`、`ApprovalSource`、`AnswerVerdict::HumanOnly`、`Autonomy::Deferred`、`EscalateReason`／`DISCARD_BYTES_MAX`。

**「tainted → Deny」是真分支**：`gate::undoable` 与 `gate::discard` 按 taint 改答，`gate::conformance::taint_readers` 是这条的机器面，单测逐门断言。

**重开参数**：出现一类没有 `Restoration` 的效果——那时的正确做法是让它不可拼写，而不是把 `Escalate` 加回来。
-/

/-! D26 Govern 类工具的只读操作在 run 里放行：效果按一次调用定，不按工具定

**决定**：`kernel::Tool` 多一个带默认实现的方法 `effect_of(&self, call: &ToolCall) -> Result<Effect, AxError>`（与 `subject`、`writes` 同取整条调用，工具用同一份文法读它），默认答登记里的 `ToolMeta.effect`；`rules` 与 `city` 两件治理工具覆写它，`rules read`、`city list` 这类不改规则的操作答 `Effect::Read`，改规则的操作仍答 `Effect::Govern`，在 run 里照旧被拒。`runtime::bench` 按 `effect_of` 的答案选门，也按它判 taint 与写后 checkpoint；`tool_called.effect`（§8-75 (b)）与并行读波的「只读」判定仍读登记的 `ToolMeta.effect`——治理工具登记为 Govern，于是它的只读操作不进并行读波，账本上也照旧记作 Govern，没有新的写面。`Kernel.Gate.GovernReads` 证明：任何一串治理调用里，被放行的恰是只读操作，改写规则的操作一条也不放行。没有新的事件种类。

**理由**：测试城里 `rules read` 与 `city list` 被拒（E_GATE_DENIED，Govern），而 MAYOR.md 说市长持有 rules。拒 Govern 的理由是「一个 run 不得改写审判它自己的规则」（§8 门册的 Governance 行），读规则不改写什么，拒它只是让模型看不到约束它的东西。按调用定效果是推断（roadmap §7：「F3 在 run 里放行 Govern 类工具的只读操作」）。

**被否**：①改 MAYOR.md 的说法、继续全拒：市长看不到自己的规则就只能猜；②把只读操作拆成另一件工具：工具清单多一项，而它与 `rules` 说的是同一个对象。

**重开参数**：User 选改说明而不放行时，删掉两件工具的覆写，默认实现即回到全拒。
-/

namespace Kernel.Gate.GovernReads

/-- 治理工具（`rules`、`city`）的一个操作，按它对规则做什么分两类：只读，或改写。
`rules read`、`city list` 属前者，`rules propose`、`city raise`、`city adopt` 属后者。 -/
inductive Op where
  | reads
  | rewrites
  deriving DecidableEq, Repr

/-- 效果层看得到的两种效果；其余效果与这一条性质无关。 -/
inductive CallEffect where
  | read
  | govern
  deriving DecidableEq, Repr

/-- `Tool::effect_of`（D26）：效果按一次调用的操作定，不按工具定。 -/
def effectOf : Op → CallEffect
  | .reads => .read
  | .rewrites => .govern

/-- 效果层在 run 里的判定（§8-27 的 `Governance` 行）：Govern 恒拒，Read 无门。 -/
def admittedInRun : CallEffect → Bool
  | .read => true
  | .govern => false

/-- 一个 run 发出的一串治理调用里，效果层放行的那些。 -/
def admitted (calls : List Op) : List Op :=
  calls.filter fun op => admittedInRun (effectOf op)

/-- 任何一串调用里，被放行的恰是其中的只读操作，次序不变。 -/
theorem admitted_are_exactly_the_reads (calls : List Op) :
    admitted calls = calls.filter (· = .reads) := by
  unfold admitted
  congr 1
  funext op
  cases op <;> rfl

/-- 任何一串调用里，没有一条改写规则的操作被放行：一个 run 仍改不了审判它的规则。 -/
theorem no_rewrite_is_admitted (calls : List Op) : Op.rewrites ∉ admitted calls := by
  rw [admitted_are_exactly_the_reads]
  simp

/-- 任何一串调用里，每一条只读操作都被放行：模型看得到约束它的规则。 -/
theorem every_read_is_admitted (calls : List Op) (seen : Op.reads ∈ calls) :
    Op.reads ∈ admitted calls := by
  rw [admitted_are_exactly_the_reads]
  simp [seen]

end Kernel.Gate.GovernReads
