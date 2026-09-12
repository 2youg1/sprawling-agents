# kernel-SPEC.md

> crate：`kernel`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：十七节；按模块分章、每章自足。
> 施工一个模块只需读：§1–§7 总纲＋该模块章（§8-x）＋§9 以后的 crate 级各节。
> 本文记接口决策与数据面；每个模块「是什么、为什么」的短说明在它自己的模块文档头，结构面在 ARCHITECTURE.md。

## 1 需求分解

kernel 是纯判定函数层：只吃入参吐 verdict，零内部 crate 依赖，不持有任何落盘物。Stage 1 落地九个模块，每个可独立完成、独立验收：

| 模块 | 形状（ARCHITECTURE §7） | 一句话 |
|---|---|---|
| `error` | 2 值类型＋6 数据面 | AxError 七字段；AxCode 35（S2 期初增 `E_STORAGE_FATAL`，删 `E_SIGNAL_UNKNOWN`）；carrier 声明位 |
| `address` | 2 值类型 | 相对 city root 路径 newtype；WriteDomain 原语；reserved prefix 判定 |
| `locator` | 2 值类型 | `cas:`／`file:` 文法解析与呈现；fail-closed |
| `event` | 2 值类型 | EventKind 64（基集 55，另有 `autonomy_changed`、roadmap 三件、`login_started`、`endpoint_probed`、`roadmap_split`／`roadmap_blocked`、`pursuit_changed`、`governed_document_written`，共 65）；in-window／record-only 二分；EventRecord 规范字节；EventRef 私有铸造 |
| `version` | 2 值类型 | 乐观并发：Version 单调值＋base 新鲜度判定 |
| `idem` | 2 值类型 | IdemKey 确定性派生（BLAKE3 XOF 16 字节＋版本字节） |
| `consts_external` | 6 数据面 | 外部事实常量 5 项 |
| `consts_policy` | 6 数据面 | 政策常量 15 项（已落 12，3 项随类型延后，见 §8-8） |
| `ledger` | 3 端口 | 唯一写入口 trait；链语义（GENESIS_PREV／chain_hash）；conformance 套件 |

Stage 2 落地其余 18 个 kernel 模块（§8-10…§8-27）。**施工序＝依赖序**：

| 施工批 | 模块 | 理由 |
|---|---|---|
| 骨架端口 | `config` `tool`(port) `model`(port) | turn 相变函数携两 port，骨架先于决断面 |
| 决断地基 | `taint` | 一切携 Taint 的动作依赖它 |
| 四判定 | `write_domain` `budget` `backpressure` `stall` | 无互依，可同时落地 |
| 登记与委派 | `goal` `repair` `delegation` `registry` | registry 供 discard 门查 Asset |
| 完成与审批 | `spine` `completion` `approval` | gate 的 Escalate 需 ApprovalItem |
| 隐私与删除 | `secret` `discard` | Egress/Discard 两门的判定输入 |
| 组合面 | `gate` | 五门消费上述全部，故最后 |

已有章节只加不改。

## 2 验收标准

- 每模块单测过 workspace lints（非测试代码零 unwrap/expect/panic/索引切片/裸算术/as）。
- `EventKind` 64 个 variant、`AxCode` 36 个 variant 与本文 §8-4／§8-1 表逐 variant 一致（S2 起 `xtask specalign` 机器断言）。
- 每个 EventKind 恰属 in-window／record-only 之一；in-window 恰 8 件。
- 每个 AxCode 恰有一个 carrier 声明；装载期白名单恰 5 码且封闭。
- golden EventRecord：规范字节入 insta 快照，跨平台逐字节稳定。
- proptest：Address 解析拒绝面、is_within 前缀性质、Locator 往返、IdemKey 重算不变。
- conformance 套件对任意 `impl Ledger` 可跑（由 citysim 内存 Ledger 第二实现证明）。

Stage 2 追加：

- 类型加固十项全部有型可指；trybuild 八反例全集编译失败。
- kani 七 harness 入库（`#[cfg(kani)]`）：kani 没有 Windows 宿主，每条性质配 proptest 镜像本地可跑，kani 本体入 CI Linux job（CI 恢复时生效）。
- **CI 只证四条，理由不是成本而是信息**：十条 harness 里有七条的输入面是具体值——一个 `Address::parse(".sprawling/ledger")`、一个 `TaintSource::new("web:x")`、两个布尔分支——那是单测穿了一层证明的外衣，而同文件的 `#[cfg(test)]` 里已经有同一命题（`write_domain::reserved_target_is_outside_even_for_an_empty_domain`、`gate::the_domain_door_*`、`discard::the_decision_table_holds_in_order`），其中两条的 proptest 输入面比 harness 更宽（`discard::allow_implies_every_guard_passed` 取任意 u64，harness 只固定一个值）。**这七条恰好就是构造 `Vec`／`String`／`BTreeSet` 的那七条**：CBMC 推不出它们内部循环的上界，无界跑了六小时、`--default-unwind 32` 又跑了 45 分钟，两次都卡在 `discard::verification::tainted_never_allows`，两次都没有给出判决——**全局 unwind 界已被实验证伪，不是这个问题的解法**。CI 因此只跑两条：`backpressure`（41s）与 `secret::log2_q10_is_total`（53s）——任意 u64 上的全函数性、单调性与定点 log2 的终止性，都是抽样到不了的地方。**不传全局 unwind**：这两条的循环界是常数（`log2_q10` 十次），CBMC 自己推得出来。
- **第八条不收敛，原因不同**：`secret::verification::entropy_is_total_on_short_inputs` 的输入域是真的（四字节任意），卡住它的是形状：`entropy_millibits_per_char` 对 256 槽计数表逐槽调 `log2_q10`，而每次调用内部做十轮 u128 平方——交给求解器的是约 **2,560 次符号非线性乘法**，非线性乘法正是 SAT 求解器的死穴，十五分钟不返回。它不在 CI 里，也不靠改 unwind 界救：算术核心已由 `log2_q10_is_total` 单独证了，要证全函数得把 harness 改成**对单一槽**而不是对整张表。剩下七条保留在源码里但不入 CI，它们的权威是旁边的测试；要么改成真正符号化的 harness（不再构造堆集合），要么删掉。
- three-part refusal 矩阵：五门每条 Deny 路径的 refusal 三段非空且 alternative 可执行。
- conformance feature 全量导出：Ledger＋Tool＋Model 三套件（sandbox 随 S3）。

## 3 假设与歧义

1. **Locator 范围语义**：`L<a>-<b>` 行号 1 起、闭区间（编辑器与 sed 先例）；`B<a>-<b>` 字节偏移 0 起、闭区间（HTTP Range 先例）。两者均要求 `a<=b`，`L` 另要求 `a>=1`。
2. **Address 附加拒绝面**：设计点名拒绝绝对路径、`..`、空段、非 UTF-8；本文在同一 fail-closed 精神下追加拒绝反斜杠、`.` 段、首尾 `/`、控制字符与 NUL、`:`（Windows 盘符与 NTFS ADS 两面一式拒）。放宽属「对扩展开放」，收紧后不再放回。
3. **git-oid 长度**：S3 引 git2 前按 40 位十六进制小写受理（SHA-1 仓库）；其它长度 fail-closed 拒。SHA-256 仓库支持届时按方向加长度分支。
4. **`run` 字段恒在**：city 级事件（`city_initialized`、`log_truncated` 等）无所属 Run，取 `RunId::CITY`（nil UUID）哨兵值；uuid v7 的时间戳位保证真实 Run 恒不与 nil 撞。
5. **`who` 字段是自由字符串**：actor 文法属 city::resident（P1）；届时收紧为类型，本文届时更新。
6. **浮点拒绝在构造点**：Ledger 载荷禁浮点（确定性七条之 6）由 `Payload::new` 与其 `Deserialize` 双侧执行，serde_json 数字非 i64/u64 可表示即拒。
7. **存储写失败码**（S2 期初定）：增装载期第 5 码 `E_STORAGE_FATAL`（AxCode 36）承载 Ledger append 等存储写失败；与 `E_CAS_CORRUPT`（读到的对象不可信）分立，recovery 相反。

## 4 现状分析

kernel 为 S0 空壳（lib.rs 仅 crate 文档）。无既有实现约束；性能敏感点唯一：chain_hash 落在单写者关键路径（选 BLAKE3 的理由），除此之外全部远离热路径。

## 5 权威信源

**改这些类型前先读 provider 官方文档**（链接已同步入 `crates/kernel/src/model.rs` 与 `crates/gateway/src/dialect.rs` 的模块注释，以便下一位先看权威再动手）：

| 主题 | 出处 |
|---|---|
| Messages API 请求与响应 | <https://platform.claude.com/docs/en/api/messages> |
| 思考块、`signature`、工具往返中的保留规则 | <https://platform.claude.com/docs/en/build-with-claude/thinking> |
| 思考强度取值 | <https://platform.claude.com/docs/en/build-with-claude/effort> |
| 什么会作废缓存断点 | <https://platform.claude.com/docs/en/build-with-claude/prompt-caching> |
| OpenAI Chat Completions 请求与响应 | <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/> |
| OpenAI `reasoning.effort` | <https://developers.openai.com/api/docs/guides/reasoning> |

Address；Ledger／EventRecord／IdemKey／BLAKE3；Locator；乐观并发；AxError／AxCode／carrier；reserved prefix；常量三源；类型加固十项；确定性七条；`docs/glossary.md`（词汇）；ARCHITECTURE.md §4（缝清单）、§12（kernel 模块图）、§9（七形状）、§10（确定性与硬化）。

## 6 命名统一

概念名一律英文原词：Ledger、EventRecord、EventDraft、EventRef、EventKind、in-window／record-only、Locator、CAS、B3Hash、GitOid、IdemKey、Address、reserved prefix、WriteDomain、AxError、AxCode、three-part refusal、carrier event、Seq、TimeMs、RunId、Version、conformance。
Rust variant 名取 UpperCamelCase，serde 呈现名恒为线格式拼写（EventKind 蛇形小写；AxCode `E_` 全大写）。`B3Hash` 住 `locator`（`b3-` 算法标签的文法之家），`event`／`ledger` 引用之，全库仅此一个哈希值类型。

## 7 模块边界

模块间使用关系（同 crate 内、编译器可见）：

```
error ──(carrier)──▶ event(EventKind)
event ──(载荷校验/字段)──▶ error(AxError)、address(Address)、locator(B3Hash)、consts_external(EVENT_LOG_V)
locator ──▶ address、error
address ──▶ error
idem ──▶ event(RunId, Seq)
version ──（自足）
ledger ──▶ event、error、locator(B3Hash)
```

Stage 2 新增使用边（同 crate 内）：

```
taint ──（自足）
write_domain ──▶ address、event(RunId)、consts_policy(EDIT_WAR_FREEZE)
budget ──（自足）        backpressure ──（自足）
stall ──▶ locator(B3Hash)、consts_policy(LOOP_REPEAT_THRESHOLD)
goal/repair ──▶ address、event(RunId)
delegation ──（自足）     registry ──▶ locator、event(EventRef)、error
spine ──▶ locator、completion(Progress)、consts
completion ──▶ event(EventRef/EventKind)、budget(BudgetUse)
approval ──▶ registry(ResidentId)、locator、event(TimeMs)、taint、consts_policy
config ──（自足）         tool ──▶ address、event(Payload)、error
model ──▶ locator(B3Hash)、event(Payload)、tool(ToolCall)
secret ──▶ consts_external(SECRET_SHAPES)、consts_policy(SECRET_ENTROPY_MIN)、error
discard ──▶ address、locator、taint、budget(ByteLen)、registry、tool(ExecArm)、consts_policy
gate ──▶ 上述全部（组合面）＋idem
```

`error` 与 `event` 互相引用（error 声明 carrier 需 EventKind；event 载荷校验产 AxError）——同 crate 内合法，且二者本就是同一冻结面（C8）的两半，不视为耦合事故。

**本 crate 不做什么（否定式三条）**：
- 不做 I/O、不采样时钟、不生成随机数——RunId／时间戳／种子全部由调用方注入；`uuid` 依赖仅用于解析与格式化，恒不启用生成特性。
- 不实现任何端口——`Ledger` 的实现住 memory 与 citysim；kernel 只声明 trait 与链语义纯函数。
- 不认识文件系统、明文凭证与颜色——canonicalize、Vault、OKLCH 各归其效果面模块。

## 8 接口先行（按模块分章）

**`#[non_exhaustive]` 辖谁，不辖谁**：它辖**冻结面**——会被序列化、跨版本读回、或被城外读者依赖的枚举（`AxCode`、`EventKind`、`Effect`、`DialectKind`、`DelegateKind` 等）。它**不辖判定输出**：`StallVerdict`／`GoalVerdict`／`RepairVerdict`／`DelegationVerdict`／`RegisterVerdict`／`Admission` 一律**刻意穷尽**，理由与 runtime-SPEC 对 `PhaseOutcome` 写的同一句：新增一种结论必须逼每个调用方表态，不得掉进 catch-all。`crates/kernel/src/stall.rs` 与 `backpressure.rs` 的判定枚举是同一条规则的第二、第三处表述。

ARCHITECTURE.md §3「nothing here is published」是这条分界成立的前提：工作区之外没有下游，故 `#[non_exhaustive]` 在判定输出上买不到任何兼容性，只卖掉 §7 想要的那个编译期穷尽性。

### 8-1 kernel::error

**类型**：

```rust
#[non_exhaustive]                       // C8：对扩展开放
pub enum AxCode { PathNotFound, /* …36 variant，serde 呈现名见下表 */ }

pub struct GateRefusal {                // three-part refusal；三段必填
    rule: String, violation: String, alternative: String,
}

pub struct AxError {                    // 七字段，序列化字段序＝声明序
    code: AxCode, action: String, subject: String,
    nearby: Vec<String>, recovery: String, retriable: bool,
    gate: Option<GateRefusal>,
}

pub enum Carrier { Event(EventKind), Loadtime }
impl AxCode {
    pub fn carrier(&self) -> Carrier;   // 穷尽 match，无 catch-all；唯一声明位
    pub fn as_str(&self) -> &'static str; // "E_…" 拼写，serde 与 Display 共用
}
```

**构造**：字段私有；两个构造子＋组合子，使「Gate 拒绝码 ⇒ gate 三段在场」由构造路径保证：

```rust
impl AxError {
    /// Non-gate failure. `retriable` defaults to false (fail-closed).
    pub fn failure(code: AxCode, action: impl Into<String>, subject: impl Into<String>) -> Self;
    /// Gate refusal. Sets `gate` to the mandatory three parts.
    pub fn refusal(code: AxCode, action: impl Into<String>, subject: impl Into<String>, gate: GateRefusal) -> Self;
    pub fn with_nearby(self, nearby: Vec<String>) -> Self;
    pub fn with_recovery(self, recovery: impl Into<String>) -> Self;
    pub fn retriable(self) -> Self;     // 显式声明可重试，默认不可
    pub fn code(&self) -> &AxCode;  pub fn gate(&self) -> Option<&GateRefusal>;
}
```

「gate 码走 `refusal`」由构造纪律＋单测保证；S2 `kernel::gate` 是全库唯一 gate 码生产者，citysim 不变量 8 号在系统层复验。derive `Serialize/Deserialize`（Ledger 载荷需要）、`Clone/Debug/PartialEq`；`thiserror::Error` 提供 Display（`{code}: {action} on {subject}`）。

**AxCode 36 全集与 carrier 对应（specalign 数据面）**

> 协作组由六降为五——`E_SIGNAL_UNKNOWN` 已定义掉（三码之一；理由与实测见 `collab-SPEC.md` §8-1）。删除时全仓只有本文件提到它，零生产者。剩下两码（`E_WORKTREE_BUSY`／`E_DIGEST_SUSPECT`）已在各自 SPEC 里答过「能否定义掉」，答案是能保留——它们各自有一个真实的运行期情境。


| 组 | AxCode | carrier event |
|---|---|---|
| 基表 | `E_PATH_NOT_FOUND` | `tool_result` |
| 基表 | `E_TOOL_UNKNOWN` | `tool_result` |
| 基表 | `E_TOOL_UNAVAILABLE` | `tool_result` |
| 基表 | `E_INVALID_ARGS` | `tool_result` |
| 基表 | `E_OUTSIDE_WRITE_DOMAIN` | `gate_denied` |
| 基表 | `E_VERSION_CONFLICT` | `tool_result` |
| 基表 | `E_GATE_DENIED` | `gate_denied` |
| 基表 | `E_BUDGET_EXHAUSTED` | `budget_limit` |
| 基表 | `E_TIMEOUT` | `tool_result` |
| 基表 | `E_PROVIDER` | `provider_degraded` |
| 基表 | `E_EVIDENCE_MISSING` | `tool_result` |
| 基表 | `E_LOOP_SUSPECTED` | `watchdog_fired` |
| 基表 | `E_LOCATOR_INVALID` | `tool_result` |
| 基表 | `E_SANDBOX_DENIED` | `tool_result` |
| 协作 | `E_DRAFT_STALE` | `tool_result` |
| 协作 | `E_GOAL_CONFLICT` | `tool_result` |
| 协作 | `E_TAINTED_ACTION` | `gate_denied` |
| 协作 | `E_REPAIR_BUSY` | `tool_result` |
| 协作 | `E_DELEGATION_DEPTH` | `gate_denied` |
| 治理与设施 | `E_APPROVAL_PENDING` | `approval_requested` |
| 治理与设施 | `E_APPROVAL_DENIED` | `approval_resolved` |
| 治理与设施 | `E_CROSS_BUILDING_DENIED` | `gate_denied` |
| 治理与设施 | `E_DIGEST_SUSPECT` | `tool_result` |
| 治理与设施 | `E_CREDENTIAL_MISSING` | `tool_result` |
| 治理与设施 | `E_CONFIG_INVALID` | 装载期（无 carrier） |
| 治理与设施 | `E_CAS_CORRUPT` | 装载期（无 carrier） |
| 治理与设施 | `E_STORAGE_FATAL` | 装载期（无 carrier） |
| 治理与设施 | `E_WORKTREE_BUSY` | `tool_result` |
| 治理与设施 | `E_BROWSER_UNAVAILABLE` | `tool_result` |
| 治理与设施 | `E_ENDPOINT_DIALECT_UNSUPPORTED` | `endpoint_lost` |
| 治理与设施 | `E_WIRE_MISMATCH` | 装载期（无 carrier） |
| 治理与设施 | `E_LOG_VERSION_UNSUPPORTED` | 装载期（无 carrier） |
| 隐私与 Discard | `E_SECRET_EGRESS` | `gate_denied` |
| 隐私与 Discard | `E_DISCARD_IRREVERSIBLE` | `gate_denied` |
| 背压 | `E_BACKPRESSURE_SHED` | `tool_result` |
| 运行未知 | `E_TOOL_OUTCOME_UNKNOWN` | `tool_result` |

装载期五码（`E_CONFIG_INVALID` `E_CAS_CORRUPT` `E_STORAGE_FATAL` `E_WIRE_MISMATCH` `E_LOG_VERSION_UNSUPPORTED`）＝C9 唯一例外白名单，封闭且不得增长（第 5 码于 S2 期初增补）；`Carrier::Loadtime` 即其类型面。
两条呈现约束：`E_SECRET_EGRESS` 的 subject 只写 SecretRef 与位置、恒不回显命中字节；`E_DISCARD_IRREVERSIBLE` 的 alternative 必须可执行。执行点在各生产模块（S2），此处记为 carrier 表随附契约。

**carrier() 依赖 `EventKind`**：故它与 `event` 模块同一变更集落码；本表与其余全部（AxError／GateRefusal／AxCode／serde／构造子）不依赖它。

### 8-2 kernel::address

```rust
pub struct Address(String);             // 不变量在唯一构造点强制；无 setter
pub const RESERVED_PREFIX: &str = ".sprawling";

impl Address {
    /// Sole constructor. Grammar: relative, `/`-separated, segments of
    /// non-control UTF-8; rejects absolute (incl. drive/UNC), `..`, `.`,
    /// empty segments, backslash, NUL/control, leading/trailing `/`.
    pub fn parse(raw: &str) -> Result<Self, AxError>;   // E_INVALID_ARGS
    pub fn is_within(&self, prefix: &Address) -> bool;  // 段边界字节前缀；WriteDomain 原语
    pub fn is_reserved(&self) -> bool;                  // 任一段 == RESERVED_PREFIX（C17，见 8-28）
    pub fn as_str(&self) -> &str;
}
```

- 派生：`Clone/Debug/Display/PartialEq/Eq/PartialOrd/Ord/Hash`（BTreeMap 键）。
- serde：呈现为字符串；`Deserialize` 经 `parse` 复验（fail-closed 读盘）。
- Windows 按字节比较、不折叠大小写；`is_within` 自反（`a.is_within(a)`）。
- 符号链接 canonicalize 属效果面（S2 write_domain 的适配层）；本原语只对已规范化相对路径作证。
- 解析拒绝的 AxError：`action="parse address"`、`subject=原串`、recovery 指出违规成分与合法形态。

### 8-3 kernel::locator

```rust
pub struct B3Hash([u8; 32]);            // 全库唯一哈希值类型；hex64 小写呈现
pub struct GitOid([u8; 20]);            // 40 位十六进制小写；serde 与 B3Hash 同形
impl GitOid { pub fn parse(raw: &str) -> Option<Self> }   // 长度不对即拒，恒不补零
pub enum Range { Lines { from: u64, to: u64 },   // 1 起、闭区间
                 Bytes { from: u64, to: u64 } }  // 0 起、闭区间（HTTP Range 先例）
pub enum Locator {
    Cas  { hash: B3Hash, range: Option<Range> },
    File { address: Address, oid: GitOid, range: Option<Range> },
// `GitOid::parse` 是公开的：checkpoint 身份向两个方向旅行——wire 反序列化一个，
// 而一个被展示了 checkpoint 句子的客户端要把它变回 oid 才能对它动作。不开这个门，
// 调用方就自己解十六进制，而那正是 `Deserialize` 旁边那句「形状权威留在这里」要防的事。
}
impl Locator {
    /// Fail-closed: anything not exactly the grammar is E_LOCATOR_INVALID.
    /// Unknown scheme or algorithm tag is an error, never a fallback path.
    pub fn parse(raw: &str) -> Result<Self, AxError>;
}
impl fmt::Display for Locator { /* 规范拼写往返：parse(x).to_string() == 规范形 */ }
```

- 文法：`cas:b3-<hex64>[#(L|B)<a>-<b>]`｜`file:<address>@<hex40>[#(L|B)<a>-<b>]`。`file:` 以最后一个 `@` 切分（address 段内允许 `@`，oid 恒不含）。
- **规范回声断言**：解析成功后另断言 `Display(结果) == 原串`，不等即 `E_LOCATOR_INVALID`（recovery 给出规范拼写）——一条规则封死大写 hex、前导零、`+` 号等全部非规范变体。
- 十六进制恒小写（规范字节唯一化）；大写拒。`b3-` 外的算法标签拒（对扩展开放：新标签＝新 variant，旧解析不宽容）。
- `SecretRef`（`secret:`）恒不入本文法——`secret:` 前缀命中即 `E_LOCATOR_INVALID`，两套解析器分立（类型层理由）。
- serde：字符串形（Display/parse 往返）。
- `B3Hash::from_bytes([u8;32])`／`to_hex()`；`Range` 构造校验 `from<=to`（Lines 另 `from>=1`）。
- S2 增 `B3Hash::digest(bytes: &[u8]) -> B3Hash`（blake3 直算）：全库内容哈希的唯一产地——prefix 分段哈希与 stall 指纹均经此，不在 kernel 外直呼 blake3（一个哈希一个家）。`chain_hash` 保留为链语义专名（内部改经 digest）。

### 8-4 kernel::event

```rust
pub struct RunId(Uuid);                 // uuid v7 仅人读标识；kernel 不生成
impl RunId { pub const CITY: RunId;     // nil UUID：city 级事件哨兵
             pub fn from_bytes([u8;16]) -> Self;  pub fn parse(&str) -> Result<Self, AxError>; }
pub struct Seq(u64);
impl Seq { pub const FIRST: Seq;        // 0；创世行
           pub fn next(self) -> Result<Seq, AxError>; }   // checked_add
pub struct TimeMs(u64);                 // UTC 整数毫秒；只入参不采样

#[non_exhaustive]
pub enum EventKind { CityInitialized, /* …55 variant，serde 蛇形 */ }
pub enum WindowClass { InWindow, RecordOnly }
impl EventKind {
    pub fn window_class(&self) -> WindowClass;  // 穷尽 match；二分权威
    pub const ALL: [EventKind; 55];             // specalign 与计数断言的数据面
}

pub struct Payload(serde_json::Map<String, Value>);
impl Payload {
    /// Sole constructor: rejects any float anywhere in the tree
    /// (determinism rule 6). Deserialize re-validates on read.
    pub fn new(map: Map<String, Value>) -> Result<Self, AxError>;  // E_INVALID_ARGS
    pub fn empty() -> Self;
}

pub struct EventDraft {                 // 调用方给的一半：语义内容
    pub run: RunId, pub t: TimeMs, pub who: String,
    pub addr: Option<Address>, pub kind: EventKind, pub data: Payload,
    pub ig: bool,                       // 「可忽略」标记；写方默认 false
}
pub struct EventRecord { /* v, run, seq, prev, t, who, addr, kind, data, ig —— 字段私有 */ }
impl EventRecord {
    /// Adapter-side assembly: the Ledger implementation owns seq/prev/v.
    pub fn from_draft(draft: EventDraft, seq: Seq, prev: B3Hash) -> Self;  // v ＝ EVENT_LOG_V
    /// Canonical bytes: serde_json, struct field order = declaration order,
    /// payload keys sorted (serde_json BTreeMap), no trailing newline.
    pub fn canonical_line(&self) -> Result<Vec<u8>, AxError>;
    pub fn to_ref(&self) -> EventRef;   // 铸造需持有整条记录
    pub fn parse_line(raw: &[u8]) -> Result<Self, AxError>;  // 读侧：逐字段复验
    pub fn seq(&self) -> Seq;  pub fn kind(&self) -> EventKind;  pub fn v(&self) -> u32;
}
pub struct EventRef { seq: Seq, kind: EventKind }   // 字段私有；无公开构造子
```

- 序列化细节：`addr` 为 None 与 `ig` 为 false 时省略键；其余八键恒在；键序＝声明序 `v,run,seq,prev,t,who,addr,kind,data,ig`。此即 V8 跨平台字节一致的规范。
- 铸造纪律（15.3-1）：`EventRef` 唯二铸造路径＝Ledger append 流程（适配器持刚组装的 EventRecord 调 `to_ref`）与 replay 验链后逐条 `to_ref`。字段私有使字面量伪造编译不过（trybuild 反例）。
- `parse_line` 是读侧唯一入口：serde 反序列化＋Payload 复验；未知 kind 在此报错（呈现语义见 runtime::replay 章——携 `ig` 的行例外）。

**EventKind 66 全集与二分（specalign 数据面；「入窗」＝InWindow，共 8）**：

| 组 | kind | 窗类 |
|---|---|---|
| 创世与空间 | `city_initialized` | record-only（创世行，prev＝64 个 0） |
| 创世与空间 | `building_created` | record-only |
| 创世与空间 | `building_configured` | record-only |
| 基集 | `run_started` | record-only |
| 基集 | `run_forked` | record-only |
| 基集 | `prompt_assembled` | **in-window** |
| 基集 | `model_called` | **in-window** |
| 基集 | `model_returned` | **in-window** |
| 基集 | `tool_called` | **in-window** |
| 基集 | `tool_result` | **in-window** |
| 基集 | `result_offloaded` | **in-window** |
| 基集 | `gate_checked` | record-only |
| 基集 | `gate_denied` | record-only |
| 基集 | `checkpoint_committed` | record-only |
| 基集 | `handoff_written` | record-only |
| 基集 | `steer_received` | **in-window** |
| 基集 | `cancel_received` | record-only |
| 基集 | `watchdog_fired` | record-only |
| 基集 | `budget_limit` | record-only |
| 基集 | `run_frozen` | record-only |
| 基集 | `log_truncated` | record-only |
| 协作 | `signal_enqueued` | record-only |
| 协作 | `signal_consumed` | **in-window** |
| 协作 | `draft_held` | record-only |
| 协作 | `draft_resolved` | record-only |
| 协作 | `goal_registered` | record-only |
| 协作 | `goal_conflict` | record-only |
| 协作 | `arbitration_verdict` | record-only |
| 协作 | `repair_started` | record-only |
| 协作 | `repair_reused` | record-only |
| 协作 | `worktree_opened` | record-only |
| 协作 | `pr_opened` | record-only |
| 协作 | `pr_merged` | record-only |
| 协作 | `pr_rejected` | record-only |
| 协作 | `roadmap_claimed` | record-only |
| 协作 | `roadmap_finished` | record-only |
| 协作 | `roadmap_released` | record-only |
| 协作 | `roadmap_split` | record-only |
| 协作 | `roadmap_blocked` | record-only |
| 协作 | `pursuit_changed` | record-only |
| 治理与设施 | `approval_requested` | record-only |
| 治理与设施 | `approval_resolved` | record-only |
| 治理与设施 | `policy_created` | record-only |
| 治理与设施 | `policy_revoked` | record-only |
| 治理与设施 | `taint_promoted` | record-only |
| 治理与设施 | `cross_building_transfer` | record-only |
| 治理与设施 | `takeover_started` | record-only |
| 治理与设施 | `rollback_applied` | record-only |
| 治理与设施 | `city_halted` | record-only |
| 治理与设施 | `backpressure_shed` | record-only |
| 治理与设施 | `digest_invalidated` | record-only |
| 治理与设施 | `endpoint_attached` | record-only |
| 治理与设施 | `endpoint_probed` | record-only |
| 治理与设施 | `endpoint_lost` | record-only |
| 治理与设施 | `model_selected` | record-only |
| 治理与设施 | `provider_degraded` | record-only |
| 治理与设施 | `login_started` | record-only（订阅登录开始，载荷携 provider 与授权 URL——URL 里只有 PKCE challenge 与 state，恒无凭证） |
| 治理与设施 | `eval_run` | record-only |
| 治理与设施 | `asset_archived` | record-only |
| 治理与设施 | `credential_lent` | record-only |
| 隐私与 Discard | `secret_captured` | record-only（行内无明文无哈希前缀） |
| 隐私与 Discard | `secret_egress_blocked` | record-only |
| 隐私与 Discard | `file_discarded` | record-only |
| 隐私与 Discard | `discard_restored` | record-only |
| 隐私与 Discard | `autonomy_changed` | record-only |
| 治理与设施 | `governed_document_written` | record-only（人写下治理这座城的三份文件之一，载荷携 which 与字节数，恒不携正文——正文在盘上，账本记的是这件事发生过） |
| 治理与设施 | `toolkit_link_opened` | record-only（人请求接入一个外部应用，载荷只携 slug。**恒不携站位**——那是关于此刻的事实（channels-SPEC §8-31）；**恒不携 consent URL**——那是一张能力凭证，记进可重放的账本等于发给每一个重放的人） |

二分依据唯一：该事件载荷是否决定模型请求字节；不存在第三类。

### 8-5 kernel::version

```rust
pub struct Version(u64);
impl Version { pub const FIRST: Version;               // 1；首个可见版本
               pub fn next(self) -> Result<Version, AxError>; }
pub enum VersionVerdict { Fresh, Stale { current: Version } }
/// Optimistic-concurrency primitive: pure verdict, no bool.
pub fn check_base(current: Version, base: Version) -> VersionVerdict;
```

`Stale` 到 `E_VERSION_CONFLICT`＋新鲜 diff 的映射在 runtime::tools::edit（S3）：kernel 只判新鲜度，不认识 diff。`base > current` 同样 `Stale`（唯一真版本是 current；超前的 base 是调用方脑补）。

### 8-6 kernel::idem

```rust
pub struct IdemKey { v: u8, digest: [u8; 16] }   // 私有；无 From<Uuid>、无 Default、无随机
pub const IDEM_DERIVE_V: u8 = 1;
impl IdemKey {
    /// Deterministic dedup key for outward actions:
    /// BLAKE3-XOF 16 bytes over `run(16B) || seq(8B LE) || action_canonical`.
    /// Fixed-width prefix makes the framing injective; same inputs after
    /// resume/replay re-derive the identical key.
    pub fn derive(run: &RunId, seq: Seq, action_canonical: &[u8]) -> IdemKey;
}
// 定义点仍在 kernel::idem；取关联函数而非自由函数，避免裸名 `derive` 入 crate 门面。
impl fmt::Display for IdemKey { /* "idem<v>-<hex32>" */ }
```

serde：字符串形。动作规范化（action_canonical 的构造规则）属工具面——它在那一面有且只有一个实现，`ToolCall::action`（§8-23）；本模块只定派生函数与框架。

### 8-7 kernel::consts_external

外部事实 5 项（改它＝外界变了）：

```rust
pub const CACHE_BREAKPOINTS_MAX: u32 = 4;
pub const PROMPT_CACHE_TTL_SECS: u64 = 300;
pub const EVENT_LOG_V: u32 = 1;                  // EventRecord.v 的唯一来源
pub const L0_TOOLS: [&str; 3] = ["exec", "edit", "status"];
pub struct SecretShape { pub provider: &'static str, pub prefix: &'static str,
                         pub charset: SecretCharset, pub len: (u16, u16) }   // 闭区间
pub enum SecretCharset { Base62, Base64Url, HexLower, Base36Lower }
pub const SECRET_SHAPES: [SecretShape; N] = [ /* 公开 provider 令牌形状，见 §14 */ ];
```

`SECRET_SHAPES` 是数据不是代码（零分支）；消费者是 S2 `kernel::secret::scan` 与 `xtask secret`。

### 8-8 kernel::consts_policy

政策常量（改它须 EVAL 证据）。比值以整数对表示（kernel 判定路径禁浮点，C10/16.3-6）：

```rust
pub struct Ratio { pub num: u32, pub den: u32 }   // 分子/分母；恒不约简
pub const STARTUP_BUDGET_TOKENS: u64 = 2000;
pub const CTX_REMINDER_RATIO: Ratio = Ratio { num: 1, den: 2 };      // 0.5
pub const LOOP_REPEAT_THRESHOLD: u32 = 3;
pub const OFFLOAD_MIN_BYTES: u64 = 16_384;
pub const INTERVAL_CAP_BYTES: u64 = 65_536;                          // 一次区间读／检索的窗口预算
pub const DRAFT_HELD_ESCALATE: u32 = 3;
pub const EDIT_WAR_FREEZE: u32 = 2;
pub const SECRET_ENTROPY_MIN: Ratio = Ratio { num: 7, den: 2 };      // 3.5 bits/char
pub const DISCARD_FILES_MAX: u32 = 16;
pub const DISCARD_BYTES_MAX: u64 = 1_048_576;
pub const DISCARD_RETENTION_DAYS: u32 = 30;
pub const POLICY_IDLE_DAYS: u32 = 90;
pub const CLOCK_ZONES_MAX: u32 = 4;
pub const WORKTREE_MAX_BYTES: u64 = 2_147_483_648;                   // 2 GiB
```

**两项图片政策**：

```rust
pub const IMAGE_MAX_BYTES: u64 = 2_097_152;   // 2 MiB：一张图的字节上限
pub const IMAGES_PER_TURN: u32 = 4;           // 一回合最多几张图
```

两个数都是「一句拒绝说得出、一个人改得动」的上限，与 `WORKTREE_MAX_BYTES` 同口径。2 MiB 取自两家 provider 都能收下的 base64 体量（base64 膨胀 4/3，2 MiB 上线约 2.7 MiB），4 张取自一回合窗口预算：再多就是把窗口花在像素上而不是任务上。

`WORKTREE_MAX_BYTES` 是上限而非磁盘余量探测：余量是一台机器当下的事实，上限则是一句拒绝说得出、一个人改得动的数；建树前校，故一座过大的城是被拒而不是被拷到一半（`memory::worktree`）。

16 项中 3 项随类型延后（表先行、值后到，位置恒在本模块）：`AUTONOMY_DEFAULT`（需 `Autonomy`，S2 approval 卡落）；`CLOCK_STAMP_DEFAULT`（需时钟档枚举；该枚举住 kernel 何处属 S2 config 卡决策——kernel 不得依赖 runtime）；`SUBAGENT_CTX_LOCK_DEFAULT`（子代理上下文锁不存在，**此项永不落地**）。余下两项落地前，本模块不提供任何替身值。

### 8-9 kernel::ledger（缝清单文件，全库五真缝之一）

```rust
/// The only write entrance to history (ARCHITECTURE §1-2). Implementations
/// own seq/prev assignment and byte production; callers never serialize.
/// Contract: Ok(ref) ⇒ the record is durable in that adapter's medium and
/// `ref` points at it; Err ⇒ nothing observable was appended (torn bytes
/// are the reopen path's business, not the caller's).
pub trait Ledger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
}

pub const GENESIS_PREV: B3Hash;                       // 32 个零字节（hex64 全 0）
/// Chain rule: prev of line k+1 = blake3(raw bytes of
/// line k, excluding the line terminator). One hash function, one home.
pub fn chain_hash(raw_line: &[u8]) -> B3Hash;

#[cfg(feature = "conformance")]
pub mod conformance {
    /// Read-back surface for verification only; production callers never read
    /// through the Ledger handle (projections do). Lives behind the feature
    /// so the production port stays write-only.
    pub trait LedgerInspect { fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError>; }
    /// One assertion suite for every implementation (V3). `fresh` must yield
    /// an empty ledger each call.
    pub fn assert_ledger_conformance<L: Ledger + LedgerInspect>(fresh: impl FnMut() -> L);
}
```

conformance 六断言（对任意实现同一套）：
1. 首条 append 得 `Seq::FIRST`，记录 prev＝`GENESIS_PREV`；
2. seq 连续无洞（逐条 +1）；
3. 链续：第 k 行 prev＝`chain_hash(第 k-1 行原始字节)`；
4. 写方规范：每行 `parse_line` 后 `canonical_line` 与原始字节逐字节相等；
5. `v` 恒＝`EVENT_LOG_V`，`ref.kind`＝draft.kind；
6. 确定性：同一 draft 序列灌两个 fresh 实例，raw_lines 逐字节相同。

### 8-10 kernel::taint

```rust
pub struct TaintSource(String);            // 非空来源标签（如 "web:example.com"）；文法 P1 随 Endpoint 收紧
pub struct TaintSet(BTreeSet<TaintSource>); // 空集＝内生数据；并集半格
impl TaintSet { pub fn empty() -> Self;  pub fn union(&self, other: &TaintSet) -> TaintSet;
                pub fn is_empty(&self) -> bool;  pub fn contains(&self, s: &TaintSource) -> bool; }

pub struct Tainted<T> { /* value, taint —— 字段私有 */ }
impl<T> Tainted<T> {
    /// Sole entrance for external content. Custody
    /// composition (secret scan before CAS) is the effect layer's wiring
    /// at this call site (S3); the type itself stays pure.
    pub fn new(value: T, source: TaintSource) -> Self;
    pub fn peek(&self) -> &T;                                   // 借用读，拿不走所有权
    pub fn map<U>(self, f: impl FnOnce(&T) -> U) -> Tainted<U>; // 派生：同集保持
    pub fn join<U, V>(self, other: Tainted<U>, f: impl FnOnce(&T, &U) -> V) -> Tainted<V>; // 并集
    pub fn taint(&self) -> &TaintSet;
}
```

- **无解包面**：无 `into_inner`、无 `Deref`、字段私有——「摘干净再传下游」编译不过（trybuild 反例）。`map` 取 `FnOnce(&T)`（借用入参），闭包无法把所有权搬出环外。
- **Tainted 恒不 serde**：`Deserialize` 即第二构造入口，伪造空 Taint 即洗白；`TaintSet` 可 serde（事件载荷需要来源清单）。
- kani：`join` 输出 taint ⊇ 两入参（并集单调不丢）；proptest 镜像同性质（kani 没有 Windows 宿主，CI Linux 跑）。
- 上游错误文本、摘要继承、动作构造器强制并集：均在消费方模块（discard／approval／gate，以及 S3 的 pipeline／digest）逐处落实，本模块只供类型。

### 8-11 kernel::write_domain

```rust
pub struct WriteDomain { /* prefixes: BTreeSet<Address> —— 私有 */ }
impl WriteDomain {
    /// C17 at the construction point: any reserved-prefix member is refused.
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;   // E_INVALID_ARGS
    pub fn admits(&self, target: &Address) -> DomainVerdict;
    pub fn prefixes(&self) -> impl Iterator<Item = &Address>;
}
#[non_exhaustive]
pub enum DomainVerdict { Within, Outside { prefixes: Vec<String> } }  // prefixes 供 three-part 的 nearby

pub struct EditSample { pub addr: Address, pub run: RunId }            // 切片序＝时序
#[non_exhaustive]
pub enum EditWarVerdict { Calm, Freeze { addr: Address } }
pub fn observe_edit_war(samples: &[EditSample]) -> EditWarVerdict;
```

- `admits`：目标 `is_reserved()` 恒 Outside（构造点已拒，判定点再拒＝fail-closed 双层）；否则 ∃prefix 使 `target.is_within(prefix)` → Within。空前缀集合法（只读角色），恒 Outside。
- **edit war 依据**：同 addr 的样本按序去重相邻同 Run 后得 run 序列 r₁…rₙ；「夺回」＝rᵢ==rᵢ₋₂ 且 rᵢ≠rᵢ₋₁；夺回数 ≥ `EDIT_WAR_FREEZE`(2) → Freeze（A→B→A→B 即两次夺回）。逐 addr 独立计，首个达阈的 addr 入 verdict（BTreeMap 序）。
- kani：reserved 目标恒不 Within；`admits` 全函数无 panic。

### 8-12 kernel::budget（钱与量的整数化）

```rust
pub struct UsdMicros(u64);  pub struct Tokens(u64);  pub struct ByteLen(u64);   // 15.3-6 钱与量整数化，三新型同家
// 各：pub const fn new(u64) / pub const fn get() / pub fn checked_add(self, o) -> Option<Self>
// checked_add 取 Option 而非 Result：溢出怎么算归调用点定（读作 E_INVALID_ARGS），
// 在原语层预先选一个错误故事会迫使调用方反封 AxError。

pub struct BudgetUse { pub usd: UsdMicros, pub tokens: Tokens }    // serde（Progress::Unplanned 载荷、Evidence.budget）
```

**花费闸的判定面不存在**：`BudgetCap`／`BudgetLevel`／`BudgetLadder`／`BudgetLayer`／`SpendVerdict`／`admit_spend`／`CtxLock`／`CtxVerdict`／`observe_ctx` 连同 `kernel::gate::spend` 一并删除，`SUBAGENT_CTX_LOCK_DEFAULT` 因此永不落地。

- **理由是刹车只留一个**：`Halt` 停一个范围并终止该范围内的后台成员（`runtime::backlog::halt` 是承兑点）。一座必须停下的城由人说停，而不是由一个没人能在事前算准的上限替他说停。两套刹车里，花费闸这一套从来没有生产调用方——`gate::spend` 的唯一调用点是它自己的测试，`BudgetCap` 在派活面上一路默认值传到冻结。
- **留下的是记账而不是闸**：`BudgetUse` 与 `memory::attribution` 的五路归因、成本页原样保留。**报告花了多少**与**事前不许花**是两件事，删去的只是后者。
- **不在此列**：`xtask/budgets.toml`（门的价目册，同名异物）与 `Fuel`（wasm 客的停机保证）。
- `BudgetUse` 保留 serde，因为它是 `Progress::Unplanned` 与 `Completion::Evidence` 的载荷字段，账本里已有历史行读得回去。
- kani：`admit_spend` 的 harness 随函数删除；`crates/kernel` 剩余 harness 数由 11 降为 10，CI 所证三条中的 `budget` 一条随之消失（ARCHITECTURE §11 的数字同集更新）。

### 8-13 kernel::backpressure

```rust
pub struct QueueStats { pub depth: u64, pub capacity: u64 }
pub struct ItemMeta { pub cost: u64 }        // 槽位数：Signal＝1，受理新 Run 的 fd 预留可 >1
pub enum ShedReason { CapacityExhausted }
pub enum Admission { Admit, Shed { reason: ShedReason } }
/// Decides whether the queue admits one more item. Pure and total:
/// depth + cost ≤ capacity admits; checked arithmetic, overflow sheds.
pub fn admit(stats: &QueueStats, item: &ItemMeta) -> Admission;
```

- 削峰是 city-wide 准入姿态：同一函数服务 Signal 队列与 fd 预留（capacity 语义由调用方赋）；队列与计数器住 memory::queue（S3）与调用方。
- kani：全函数无溢出；单调性——同 capacity/cost 下 depth 更小恒不更难 Admit。
- 饱饱不饿死（活性）属 citysim liveness（P2），非本函数可证。

### 8-14 kernel::stall

```rust
pub struct ActionFingerprint(B3Hash);        // 动作规范字节的摘要；derive(bytes) 内调 B3Hash::digest
pub enum StallVerdict { Ok, Stall { repeats: u32 } }
/// Sole stall criterion. Sample = recent fingerprints
/// in time order; a tail run of identical prints ≥ LOOP_REPEAT_THRESHOLD
/// is a stall. Counters and queues live with the caller, never here.
pub fn observe(recent: &[ActionFingerprint]) -> StallVerdict;
```

- 判尾部连续：历史中早先的重复不算（已被新动作打断＝已恢复）。阬值取 `LOOP_REPEAT_THRESHOLD`(3)。
- watchdog（S3）只消费 verdict 不转发依据；`E_LOOP_SUSPECTED` 的塑形在处置面。

### 8-15 kernel::goal

```rust
pub struct GoalId(String);                   // 非空
pub enum GoalResource { Path(Address), External(String) }  // External 非空（外部不可分资源名）
pub struct GoalEntry { pub id: GoalId, pub owner: String, pub resources: Vec<GoalResource>,
                       pub statement: String, pub standing: bool }
pub enum GoalVerdict { Clear, Conflict { with: GoalId } }
/// Same-resource mutual exclusion only: detection is
/// kernel's, arbitration is not. Paths conflict on prefix overlap either
/// way; External conflicts on equality; Path vs External never.
pub fn detect_conflict(registered: &[GoalEntry], candidate: &GoalEntry) -> GoalVerdict;
```

- 报首冲突（registered 切片序，确定）；id 去重归登记方（调用方持表）；candidate 自冲突不判（同 owner 同 id 重提交属幂等）。
- `E_GOAL_CONFLICT` 的塑形在注册回传（S3 工具面）；kernel 只出 verdict。

### 8-16 kernel::repair

```rust
pub enum RepairVerdict { Lease, Queued { holder: RunId } }
/// One live lease per scope subtree: overlap either
/// way queues; the same holder re-requesting its exact scope re-leases
/// (idempotent). State (the active map) lives with the caller.
pub fn request(active: &BTreeMap<Address, RunId>, scope: &Address, who: &RunId) -> RepairVerdict;
```

- 重叠依据同 goal：`scope.is_within(s) || s.is_within(scope)`；报首个重叠的 holder（BTreeMap 序确定）。

### 8-17 kernel::delegation

```rust
#[non_exhaustive] pub enum DelegateKind { Resident, Ephemeral }
impl DelegateKind { pub fn as_str(self) -> &'static str; }   // 一个词一个权威（工具解析与 status 打印同源）
/// Depth-zero position; the only type with a delegate method (15.3-10).
pub struct Delegator(/* 私有单元 */);
impl Delegator { pub fn root() -> Delegator;                     // 铸造点：装配/citysim
                 pub fn delegate(&self, kind: DelegateKind) -> Delegate; }
pub struct Delegate { /* kind —— 私有 */ }                        // 无 delegate 方法：trybuild 反例
impl Delegate { pub fn kind(&self) -> &DelegateKind; }

pub enum Depth { Root, Delegated }
pub enum DelegationVerdict { Allow, Deny }
/// Dynamic half of the two-layer guard (static half = the missing method).
pub fn admit(parent: Depth, kind: &DelegateKind) -> DelegationVerdict;   // Delegated 恒 Deny
```

- `root()` 公开是诚实的承认：类型封的是「从 Delegate 值铸子代」这条路，「谁持有 Delegator」由装配纪律看守；动态 `admit` 是第二层（「深度两层拦」）。
- `E_DELEGATION_DEPTH` 不消解；塑形在 gate::spawn（gate 码唯一生产者）。

### 8-18 kernel::registry

```rust
pub struct ResidentId(String);               // 非空；`role@building.n` 文法 P1 随 city::resident 收紧
pub struct Claim { pub locator: Locator, pub by: String }        // 证词：未验证产出
pub struct Artifact { /* locator, verified_by —— 私有 */ }
impl Artifact {
    /// Sole constructor: player–referee in the type. Verification evidence
    /// must be a tool_result or model_returned ref, else E_EVIDENCE_MISSING.
    pub fn verify(claim: Claim, evidence: EventRef) -> Result<Artifact, AxError>;
    pub fn locator(&self) -> &Locator;  pub fn verified_by(&self) -> &EventRef;
}

pub struct Registry { /* artifacts: BTreeMap<String, Artifact>, assets: BTreeSet<String>,
                         residents: BTreeSet<ResidentId> —— 私有 */ }
pub enum RegisterVerdict { Registered, AlreadyRegistered }
impl Registry {
    pub fn new() -> Registry;
    pub fn register_artifact(&mut self, artifact: Artifact) -> RegisterVerdict;   // 键＝locator 规范拼写
    pub fn promote_asset(&mut self, locator: &Locator) -> Result<RegisterVerdict, AxError>; // 未登记 → E_PATH_NOT_FOUND
    pub fn register_resident(&mut self, id: ResidentId) -> RegisterVerdict;
    pub fn artifact(&self, locator: &Locator) -> Option<&Artifact>;
    pub fn is_asset(&self, locator: &Locator) -> bool;           // Discard 门的查询面
}
```

- Registry 是值不是存储：状态住调用方（S3 起由 projection 重建）；kernel 只定登记规则与查询面。
- 评分归 eval（P3）；promotion 只登记不评分。

### 8-19 kernel::spine（六列树）

```rust
pub enum RoadmapStatus { NotStarted, InProgress, Done, Blocked, AwaitingApproval }   // 携 serde（snake_case）
pub const ROADMAP_STATUS_SPELLINGS: [(RoadmapStatus, &str); 5];   // 表内拼写，单套
pub const ROADMAP_COLUMNS: usize = 6;
impl RoadmapStatus { pub fn spelling(self) -> &'static str; }     // 唯一拼写产地
pub enum EvidenceCell { Empty, Invalid { raw: String }, Present(Locator) }
pub struct RoadmapRow { pub id: NodeId, pub item: String, pub weight: u32,
                        pub needs: Vec<NodeId>, pub status: RoadmapStatus,
                        pub evidence: EvidenceCell }
pub enum RoadmapShape { WellFormed { rows: Vec<RoadmapRow> }, Malformed { problems: Vec<String> } }
pub struct NewChild { pub item: String, pub weight: u32 }
pub fn check_roadmap_shape(text: &str) -> RoadmapShape;
pub fn set_roadmap_status(text: &str, id: &NodeId, status: RoadmapStatus,
                          evidence: Option<&Locator>) -> Result<String, AxError>;
pub fn insert_children(text: &str, parent: &NodeId, children: &[NewChild]) -> Result<String, AxError>;

pub const MEMO_OUTLINE_FIELDS: [&str; 6];
pub enum MemoShape { WellFormed, Malformed { missing: Vec<&'static str> } }
pub fn check_memo_shape(text: &str) -> MemoShape;
pub enum ScopeChange { Keep, Add, Drop }
pub enum WriteMoment { BeforeReport, AfterFeedback, OnPlanChange }
```

- **本模块只管文法，结构归 `plan`。** `check_*` 答「写得像不像一张 roadmap」；这些行**彼此怎么挂、各值多少、哪个能动**是 `kernel::plan` 的事。分开是因为两种坏法的修法不同：一行写错了改那一行，依赖成环了要重想这件事怎么排。
- **六列，且索引是路径**。`| # | Item | Weight | Needs | Status | Evidence |`。`2.3.1` 挂在 `2.3` 下，于是**一张表就说清了多级计划**，不需要第二个文件描述层级；`Weight` 是同一父下诸行之间的**比例**（空格＝1，整层同乘不变），`Needs` 是必须先完成的行。旧四列表**不是被兼容而是被报告**：把四列当六列读会把状态词读进 weight 格，所以 `check_roadmap_shape` 报 `4 columns` 并让整栋楼落到 `Progress::Unplanned`——**一份读不懂的计划没有分母，这件事必须看得见。**
- **拼写单套且不区分大小写**：运行时文档全面英文化后，中英两套拼写会成为「一个 Resident 允许写什么」的第二个权威；而大小写不入契约是因为 `done` 这类行表达的事实表装得下，把它判成 Malformed 等于拿一个读者不接受的理由把该行逐出分母。**线上拼写另有一套**（`snake_case` 标识符）：线帧是给程序读的，表格是给人读的，让客户端硬编码 `Awaiting approval` 正是短语表存在要防的事。
- **写者与读者同住**：`set_roadmap_status` 与 `insert_children` 是这张表仅有的两个编辑入口，两者共用同一段「哪几行是这张表」的判定（`locate_table`／`body_row`）。三条契约不变：只改首个表；输出行规范化，故**同一次改写两次得到同一字节**；`Done` 缺证据恒拒（`E_EVIDENCE_MISSING`）。
- **`insert_children` 只往后编号，不补空位**：一个计划索引是一个名字，复用它等于悄悄搬走别人的证据。子节点落在父节点**最后一个后代之后**，于是阅读顺序不变、昨天看到的编号今天仍指同一件活。
- **`WriteMoment` 有消费者**：正因为可写时刻是封闭的，读者才可以在两次写之间**持有已解析的树**而不是每问一次就把每栋楼的文件重解析一遍（`bin::plan_view`）。

### 8-32 kernel::share（形状 2 value）

```rust
pub struct Share(/* u64 私有：十亿分之一 */);
pub const WHOLE_PPB: u64 = 1_000_000_000;
impl Share {
    pub const WHOLE: Share;                 // 唯一原点
    pub const NONE:  Share;                 // 加不出来，故可公开
    pub fn ppb(self) -> u64;
    pub fn split(self, weights: &[u32]) -> Result<Vec<Share>, AxError>;   // 取走自己，分出恰好等于自己的诸份
}
pub fn gather(parts: &[Share]) -> Share;    // 自由函数，不是 Add
```

- **守恒不是被检查的规则，而是类型唯一能表达的事。** 份额只有两种来路：整份计划，或者**分掉另一份得到的一片**。`split` 按值取走输入，于是「凭空多出一份」拼不出来（trybuild `forge_share`）。没有构造器、没有算术、没有 `Deserialize`——**从文件里读回来的数字必须先从整体里分出来才能成为份额**，这正是 `PlanTree::build` 每次自根重分的理由：守恒是重新推导出来的，不是被信任的。
- **这样就解掉了自评偏差**：一个 Agent 怎么切自己那一支都行，但它切不出比父节点更多的份。**封顶取代仲裁**，也就不需要「权重从哪来」的第二权威。
- **十亿分之一而不是分数**：两份份额在任何机器上以同样方式比较与相加，反复细分不会把分母撑大，判定路径上没有浮点（ARCHITECTURE §10 规则 6）。整除余数按**先来先得**发给靠前的几份——这是一条规则而不是一次舍入，于是同一份计划在任何机器上分法相同。
- **`gather` 是自由函数而不是 `Add`**：它是「一根枝的叶子加起来是多少」这件事，不是谁都可以随手做的算术。混了两份计划的调用方会得到饱和在整份上的结果，那是诚实答案。

### 8-33 kernel::plan（形状 1 判定）

```rust
pub struct NodeId(/* String 私有：点分十进制 */);   // 携 Serialize；Deserialize 走 parse
impl NodeId {
    pub fn parse(raw: &str) -> Result<NodeId, AxError>;
    pub fn parent(&self) -> Option<NodeId>;
    pub fn ordinal(&self) -> u32;
    pub fn depth(&self) -> usize;
    pub fn is_ancestor_of(&self, other: &NodeId) -> bool;
    pub fn ancestors(&self) -> Vec<NodeId>;
    pub fn child(&self, ordinal: u32) -> Result<NodeId, AxError>;
    pub fn as_str(&self) -> &str;
}
pub const NODE_DEPTH_MAX: usize = 10;

pub enum StopCause { Blocked { note }, HandedBack { note }, FrozeWithoutEvidence,
                     Stalled { repeats }, GateOverdue { waited_ms } }
impl StopCause { pub fn status(&self) -> RoadmapStatus; pub fn is_red(&self) -> bool; pub fn line(&self) -> String; }

pub struct Held(/* NodeId 私有 */);         // #[must_use]
impl Held {
    pub fn id(&self) -> &NodeId;
    pub fn finish(self, evidence: Locator) -> PlanExit;   // 绿
    pub fn stop(self, why: StopCause) -> PlanExit;        // 红，或回到就绪集
}
pub enum PlanExit { Finished { id, evidence }, Stopped { id, why } }

pub struct PlanNode { pub row: RoadmapRow, pub share: Share, pub children: Vec<NodeId> }
pub struct PlanTree { /* BTreeMap<NodeId, PlanNode> 私有 */ }
impl PlanTree {
    pub fn build(rows: Vec<RoadmapRow>) -> Result<PlanTree, AxError>;
    pub fn get(&self, id: &NodeId) -> Option<&PlanNode>;
    pub fn nodes(&self) -> impl Iterator<Item = &PlanNode>;
    pub fn needs_of(&self, id: &NodeId) -> BTreeSet<NodeId>;
    pub fn ready(&self) -> Vec<NodeId>;
    pub fn claim(&self, id: &NodeId) -> Result<Held, AxError>;
    pub fn progress(&self) -> Progress;
}
```

- **构造点拒五种形状**（fail-closed，与 `Locator` 同）：索引重复、父行缺失、依赖指向不存在的行、依赖自指、依赖成环。第五种用 Kahn 剥层，剥不掉的就在环上，**报成一条走法而不是一个集合**——修的人需要看见该剪哪条边。拿到 `PlanTree` 的调用方因此永远不必再问「这份计划讲不讲得通」。
- **一根枝把自己的份额整份分给子节点**，`build` 自根向下重分一次，于是**总量恒为整份计划**，不需要给分母编版本（否决「分母版本化」，因为守恒之下它在解一个不存在的问题）。
- **只有叶子进分子**。一根枝的活就是它的子节点，两边都算等于把同一份力气数两遍；`build` 因此拒绝「枝说 Done 而子节点没说」，于是枝的状态列是一句读者可信的摘要而不是第二种意见。
- **进度两个数一起走**（`PlannedProgress` 增 `done_ppb`／`blocked_ppb`）：份额说走了多少路，叶子数说这份计划最后原来有多少片。**只看份额会被慷慨的拆分骗**，只看叶子数不知道轻重；两个一起看，「先挑软柿子」的形状会自己显出来。
- **就绪集是纯函数**：叶子、无人认领、且自己与**每一层祖先**的依赖都已 Done。祖先那一条是必须的——否则 `2.3.1` 会在 `2.3` 等的那件事还没好时就开工。
- **`claim` 是拿到 `Held` 的唯一路径**，且三种拒绝各说各的：不在表里、是枝或已被拿走（`E_GOAL_CONFLICT`——两个 run 想要同一个节点就是目标冲突）、还在等什么（**点名等谁**）。第三段永远给出一个真能拿的节点。
- **计划门禁就是 `Held` 的形状**：一个私有字段使它只能由 `claim` 铸出，两个按值取走的方法使它只能花在两扇门上。**没有第三个出口**；一个只是结束了的 run 把它花在 `FrozeWithoutEvidence` 上，那正是 `blockage` 里红色的来处。`HandedBack` 是唯一不红的停：它把节点放回就绪集，而把它和「卡住」合成一个取值，要么让没人拒绝过的活搁浅，要么在有人只是没预算了的时候把计划涂红。

### 8-34 kernel::blockage（形状 1 判定）

```rust
pub struct RedNode { pub at: NodeId, pub why: StopCause }
pub struct Blockage { pub source: NodeId, pub why: StopCause, pub reaches: Vec<NodeId> }
impl Blockage { pub fn line(&self) -> String; }
pub struct Notice { pub to: String, pub about: NodeId, pub line: String }
pub fn spread(tree: &PlanTree, red: &[RedNode]) -> Vec<Blockage>;
pub fn notices(blocked: &[Blockage], holders: &BTreeMap<NodeId, String>) -> Vec<Notice>;
```

- **红不是新机制**：来源全是这座城已经记着的事实——冻结而无证据、门升给人超期、`kernel::stall` 判定原地打转、居民自己说卡住了。本模块只答那些事实答不了的一问：**既然 2.3.1 红了，还有什么动不了。**
- **答案指名源头而不是罗列症状**：一份计划只有一个真问题时，产出是一条「整条 2.3 支线卡在 2.3.1」，而不是十七个红点让人自己往回找。首屏放得下一个原因，放不下一串后果。
- **红走两条路，而它们是同一关系的两面**：沿树向上（子节点卡住则枝卡住），沿依赖边向前（等一个红节点就是等一件不会来的事）。`reaches` 恒不含 `source`，且**已自带原因的节点不算别人的后果**——顺着列表读下来，每个问题只遇到一次。
- **`notices` 让交流由事实触发而不是由人触发**：每个 blockage 对每个持有者只发一条（一个信箱里四份同一个问题就是一个没人读的信箱），报告者不会收到自己那条。持有者是一个字符串而不是地址——kernel 不假设「谁持有」是一个地址，装配层用房间地址填它。

### 8-35 kernel::pursuit（形状 1 判定）

```rust
pub enum PursuitState { Running, Paused }        // 携 serde
pub struct Pursuit { /* 私有；无 serde */ }
impl Pursuit {
    pub fn declare(at: &Delegator, goal: String) -> Result<Pursuit, AxError>;
    pub fn goal(&self) -> &str;
    pub fn state(&self) -> PursuitState;
    pub fn pause(&mut self);
    pub fn resume(&mut self);
}
pub enum PursuitVerdict { Work { next: NodeId }, Waiting { in_flight: u32 }, Paused, Finished }
pub fn observe(state: PursuitState, ready: &[NodeId], in_flight: u32) -> PursuitVerdict;
```

- **不叫 Endless，按它是什么命名**：本仓已有三处叫 standing 的东西（`assembly::Standing`、`city::Standing`、`GoalEntry.standing`），再加一个会让词汇表出现第四个含义。
- **一个社会停下来不是因为有人喊停，是因为没有就绪的活了。** 依据只此一条：就绪集为空**且**没有在途的 run。两半都要——就绪集空而四个 run 在跑，意思是活在别人手上，不是活干完了。
- **钱明确不是停机条件**。本仓的成本面受众是 Agent（给它优化的材料），不是刹车；一个读预算的停机条件回答的是一个这里没人问的问题。
- **`observe` 收状态而不收 `Pursuit`**：判定不依赖目标说了什么，而一个必须先持有 `Pursuit` 才能发问的读者，等于要拿深度零位才能**读**这座城。**声明是被守的动作，看不是。**
- **子代理拼不出来**：`declare` 收 `&Delegator`，而 `Delegate` 造不出一个（trybuild `delegate_declares_pursuit`）。与 `delegation` 同一个两层守卫，理由也同一个：一个能让全城通宵干活的子代理，就是一个能替你决定通宵干什么的子代理。
- **pause 与 clear 是两件事，都要**：暂停留着目标，清除把它丢掉（丢掉值本身，于是不会被误恢复）。取消一个 **run** 是第三件事，住在 run 那边。

### 8-20 kernel::completion

```rust
pub struct Evidence(/* Vec<EventRef> 私有 */);
impl Evidence {
    /// Non-empty and every ref kind ∈ {tool_result, model_returned},
    /// else E_EVIDENCE_MISSING. A6's type half.
    pub fn new(refs: Vec<EventRef>) -> Result<Evidence, AxError>;
    pub fn refs(&self) -> &[EventRef];
}
#[non_exhaustive] pub enum Completion { Done(Evidence), Limit, Cancelled }

pub struct PlannedProgress { pub done: u32, pub blocked: u32, pub total: u32 }
impl PlannedProgress { pub fn ratio(&self) -> (u32, u32); }      // (done, total)；呈现方自算百分比
pub struct UnplannedProgress { pub steps: u32, pub budget: BudgetUse }   // 无 ratio 方法：类型层诚实（A17）
#[non_exhaustive] pub enum Progress { Planned(PlannedProgress), Unplanned(UnplannedProgress) }
```

- 两态分两 struct 而非 enum 携字段：百分比方法只能长在 Planned 上，Unplanned 拿不到——「界面拿不到百分比就画不出百分比」的类型形态。
- `EventRef` 新增 `pub fn kind(&self) -> EventKind`（Evidence 校验需读 kind；公开面变更随本 SPEC 同集）。

### 8-21 kernel::approval

```rust
pub struct ApprovalId(String);               // 非空；uuid v7 由效果层发，kernel 不生成
#[non_exhaustive] pub enum ApprovalSource { Gate, Agent }
#[non_exhaustive] pub enum ApprovalClass { Commitment, BudgetLimit, DiscardEscalate, AgentQuestion,
                                           Delegation, Governance, Undoable }
// Delegation：第一次派生要人点头。Governance：改写一个 scope 的规则要人点头。
// Undoable：伸到城外、且城里没有任何一处收得回来的后果——目下就是运行中的机器自己的桌面（§8-45）。
// 三者都无对应 PolicyClass variant——一条「豁免改规则」的常设规则会把自己废掉；同样地，一条豁免掉每一次未来点击的常设规则，豁免掉的正是「有人看着」这件事本身。
pub struct ClusterKey { pub class: ApprovalClass, pub detail: String }
pub struct ApprovalItem { pub id: ApprovalId, pub source: ApprovalSource, pub actor: String,
                          pub action_desc: String, pub artifact: Locator, pub cluster_key: ClusterKey,
                          pub created: TimeMs, pub tainted: bool }

#[non_exhaustive] pub enum PolicyClass { AgentQuestion }          // 可免审类：三必经人类无 variant 可写（类型层禁止）
pub struct PolicyMatcher { pub class: PolicyClass, pub detail_prefix: String }
#[non_exhaustive] pub enum PolicyVerdict { Allow, Deny }
pub struct Policy { pub id: String, pub matcher: PolicyMatcher, pub verdict: PolicyVerdict,
                    pub source: ApprovalId, pub created: TimeMs, pub last_hit: Option<TimeMs> }
                    // 无 revocable 字段：恒真字段不入型（false 不可表示）
#[non_exhaustive] pub enum PolicyApplication { Applies(PolicyVerdict), NotApplicable }
pub fn match_item(policy: &Policy, item: &ApprovalItem) -> PolicyApplication;   // tainted 恒 NotApplicable（C15）
#[non_exhaustive] pub enum PolicyExpiry { Active, Expired }
pub fn expiry(policy: &Policy, now: TimeMs) -> PolicyExpiry;      // idle ≥ POLICY_IDLE_DAYS → Expired；checked
#[non_exhaustive] pub enum PolicyRevocation { Revoked, Expired, Superseded }   // policy_revoked reason 数据面

#[non_exhaustive] pub enum Autonomy { Owner, Delegate(ResidentId), Deferred }
#[non_exhaustive] pub enum Answerer { Human, Resident(ResidentId) }
#[non_exhaustive] pub enum AnswerVerdict { May, HumanOnly, SelfApprovalBarred, NotTheDelegate }
pub fn may_answer(autonomy: &Autonomy, item: &ApprovalItem, answerer: &Answerer) -> AnswerVerdict;
```

- **三必经人的类型化**：`PolicyClass` 不含 Commitment/BudgetLimit/DiscardEscalate，免审规则对三类**不可表示**；`match_item` 对 `tainted` 恒 NotApplicable（C15 的 Taint 条）。
- `may_answer`：Human 恒 May；Resident r 仅当 autonomy==Delegate(r)（否则 NotTheDelegate）且 item 不属三类且 !tainted（否则 HumanOnly）且 item.actor ≠ r（否则 SelfApprovalBarred）。Deferred 下 Resident 恒 HumanOnly——没有人应答是事实的名字，不是新判定。
- verdict 先落账再生效、前拦不烧 token：效果层顺序约束（S3/S4），kernel 只出判定。
- `AUTONOMY_DEFAULT: Autonomy = Owner` 落 consts_policy。

### 8-22 kernel::config

```rust
#[non_exhaustive] pub enum ClockStampGranularity { Off, Minute, FiveMinute, Hour }   // 类型住 kernel 非 runtime
pub struct LayeredValue<T> { pub city: Option<T>, pub building: Option<T>, pub resident: Option<T> }
impl<T> LayeredValue<T> { pub fn resolve(&self) -> Option<&T>; }  // resident→building→city 下层覆盖上层

pub struct FrozenConfig { pub clock_stamp: ClockStampGranularity }   // Run 起点冻结；[model]/[clock] zones 字段 S3 只加
pub struct LiveConfig {}                                             // 热载面；S4 起填（PowerMode 等）
pub fn freeze(clock_stamp: &LayeredValue<ClockStampGranularity>) -> FrozenConfig;   // 缺省 CLOCK_STAMP_DEFAULT
```

- **无字段交集可机械判**：单测将两型缺省值 serde 成 JSON，断言键集交集为空；新增字段自动入判。
- `CLOCK_STAMP_DEFAULT: ClockStampGranularity = Off` 落 consts_policy。

**时钟分区（config）**：`ClockZone { id, offset_min }`（已解析偏移，恒不记时区名——重解会随时区库版本分叉重放历史）；`FrozenConfig` 增 `clock_zones: Vec<ClockZone>`，`freeze` 增梯入参；zones 梯整表覆盖（下层写即替换上层全表）。本段属 kernel::config（§8-22），就近登记于此避免拆章。

**思考强度（config）**：`FrozenConfig` 增 `effort: Option<Effort>`（类型住 §8-24），`freeze` 增该梯入参，缺省 `None`＝不写该字段、由 provider 自行决定。

**沙箱限额（config）**：`SandboxLimits { shell: bool, fuel: u64, mounts: Vec<Address> }`，`FrozenConfig` 增 `sandbox` 字段，`freeze` 增该梯入参。三条口径：①**整值解析而非逐字段合并**——一层说到 sandbox 就说全部，于是欠说的层只会收窄而恒不会悄悄放开上层没提过的能力；②**主机事实不入城**（CPython 工件路径、shell 可执行文件位置走环境变量）——一座城被搬到另一台机器时不该带着运行中的机器的路径；③冻结的理由与工具表相同：**能改变可达范围的东西恒不在回合中变宽**，否则变宽的那一刻没有人审过。缺省 `fuel = SANDBOX_FUEL_DEFAULT`（`consts_policy`，2×10⁸），`shell = false`——shell 是唯一一条从参数读不出可达范围的臂。

**外部 MCP server（config）**：`McpServer { label: ServerLabel, transport: McpTransport }`，`McpTransport { Stdio { command, args, env }, Http { url, headers }, Sse { url, headers } }`——**穷尽枚举而非两个裸字段**：一行既写 command 又写 url 就是一行要读者去猜的配置，故配置层当场拒（`ServerLabel` 住 §8-23）。**枚举是闭的**（无 `#[non_exhaustive]`）：读者全在这一个二进制里，通配臂只会把下一种 transport 从必须表态的模块面前藏起来。`env` 与 `headers` 皆为名在前、值在后的成对表，值可以是 `secret:realm/name` 引用——交给子进程的名字收不回来，故兑付发生在起进程／发请求的那一格，而恒不写进配置文件。`Sse` 自成一支而不是 `Http` 的一个开关：两者开法与败法都不同。`FrozenConfig` 增 `mcp: Vec<McpServer>`，`freeze` 增该梯入参，缺省空表＝这栋楼不接任何外部 server。三条口径：①**整表覆盖**，与 zones／sandbox 同一条理由——一层说到 `[[mcp]]` 就说全部，欠说的层只会收窄而恒不会悄悄接上上层没提过的服务；②**冻结的理由就是工具表本身**——外部工具在 Run 起点入 catalog，而 provider 把工具数组哈希在 system prompt 之前，Run 内变宽的工具表既自毁缓存又没有人审过；③**命令与参数是主机事实**（一个可执行文件在运行中的机器上的位置），故它们住 `CONFIG.toml` 而恒不入 Ledger 载荷——一座城被搬到另一台机器时不该带着运行中的机器的路径。

**环境变量透传（config）**：`SandboxLimits` 增 `env_passthrough: Vec<EnvVarName>`，缺省空表；`EnvVarName` 是本模块的新值类型（形状 2），唯一构造点 `EnvVarName::parse`。

- **动机是一次实测**：同一条 PATH 下，完整环境的 `cargo build` 成功，而 `env -i PATH="$PATH" cargo build` 在链接处失败——rustc 的 MSVC 链接器要读 `%ProgramFiles(x86)%\...\vswhere.exe` 才找得到 `link.exe`，环境被洗掉之后它退回裸的 `link.exe`，而 PATH 上第一个 `link.exe` 是 Git 附带的 coreutils 那个（报 `link: missing operand`）。于是住在城里的 resident today 跑不动 `just check`。
- **解法不是加长 `ENV_ALLOWLIST`**：那份常量旁边的注释正是为阻止这件事而写的——**子进程继承到的东西，它忘不掉**。加长它会让每一栋楼、每一次 `exec` 都多继承一份没人审过的东西。改成由**楼自己逐名声明**：说得出名字的那几个才进得去。
- **口径与 `mounts` 逐条同形**：整值上梯（一层说到 `[sandbox]` 就说全部）、同一条冻结理由（可达范围恒不在 Run 内变宽）、同一个「在解析点拒」的位置。`mounts` 拒保留区，`env_passthrough` 拒凭据形状的名字。
- **`EnvVarName::parse` 拒四类**：空名；含 `=`（那是赋值号，不是名字的一部分）；含 NUL 或控制字符；以及 `secret::names_a_credential` 判为凭据形状的名字（`consts_policy::CREDENTIAL_NAME_MARKERS`，子串命中即判，大小写不敏感）。**拒在解析点而不在使用点**：一个名字一旦递给子进程就收不回来，所以判定必须发生在配置被读进来的那一刻。
- **凭据形状的名字为何由 `kernel::secret` 判**：这座城已经有一处「什么东西看起来像凭据」的权威，名字这一面长在同一处而不是第二处。依据是标记词子串（`SECRET`／`TOKEN`／`KEY`／`PASSWORD`／`PASSWD`／`CREDENTIAL`／`AUTH`／`SESSION`／`COOKIE`／`PRIVATE`／`SIGNATURE`），**故意宁滥勿缺**：`KEYBOARD` 一并被拒是可接受的代价，因为拒绝带着三段式的替代路径，而漏掉一个 `AWS_SECRET_ACCESS_KEY` 不带任何提示。

- **为何必须冻结**：provider 官方文档记明「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」——强度是缓存前缀的一部分。Run 内可变的强度＝Run 内自毁的缓存，故它落 `FrozenConfig` 而非 `LiveConfig`；设置面改它对**下一个 Run** 生效。这是那句「`[model]` 字段 S3 只加」预留位置的第一个真实居民。
- `None` 与 `Some(Effort::Off)` 是两件事：前者不写字段（provider 缺省，Anthropic 新模型即 adaptive thinking），后者显式关闭思考。不用 `Effort::Off` 兼任「未声明」，否则「没设过」与「设成关」在类型上不可分辨。

### 8-23 kernel::tool（缝清单文件）

```rust
pub struct ToolName(String);        // 非空；ascii 小写/数字/下划线（进 catalog 与事件的名）
pub struct ServerLabel(String);     // 非空；ascii 小写/数字，恒不含下划线（见下）
pub struct TimeoutMs(u64);          // 声明即承诺可协作取消
#[non_exhaustive] pub enum Effect { Read, Write { domain: Address }, Egress,
                                    Connector { label: ServerLabel }, Spawn, Govern, Spend }   // 决定过哪道门
// Spawn：起第二个 Agent。不归 Read——一次派生能花多少、能碰什么，调用方自己的任何一道门都不管；
// 管得住它的只有人。故它自成一类，`kernel::gate::delegation` 恒 Escalate，是否已获准由调用方的 granted 集回答。
// Govern：改写一个 scope 被判的规则。刻意不归 Write——保留子树在每个写域之外，写门本就会拒；
// 而它拒的理由正是「这件事归人」。`kernel::gate::govern` 恒 Escalate，且把提案正文截前 600 字进 action_desc：
// 一个只被告知「要改规则」的人是在猜。
#[non_exhaustive] pub enum Temporal { Timeless, Timestamped }
#[non_exhaustive] pub enum CostTier { Free, Light, Heavy }        // 三档起步，对扩展开放；路由/预算消费在 S3
#[non_exhaustive] pub enum RenderIntent { Generic, Terminal, Diff { locations: Vec<Address> } }
                                    // meta 级声明用空 locations；逐调用的 locations 是 args 的纯函数（S3 工具侧）
pub struct ToolMeta { pub name: ToolName, pub disclosure: String, pub params: Payload,
                      pub effect: Effect, pub cost_tier: CostTier, pub timeout: Option<TimeoutMs>,
                      pub render: RenderIntent, pub temporal: Temporal }   // 八字段，缺一不可
pub struct ToolCall { pub id: String, pub name: ToolName, pub args: Payload }
                                    // id：tool_use↔tool_result 对号是两 Dialect 的 wire 硬性要求；
                                    // 脚本适配器用确定性合成 id（call-<n>）
impl ToolCall {
    /// The bytes that say what this call does: the name, then the
    /// arguments. `IdemKey::derive` takes them as `action_canonical`;
    /// `id` stays out, because two calls differing only by wire id are
    /// the same action.
    pub fn action(&self) -> Result<Vec<u8>, AxError>;
}
pub struct ToolOutcome { pub result: Payload, #[serde(default)] pub attachments: Vec<ImageRef> }

pub trait Tool {
    fn meta(&self) -> &ToolMeta;
    /// Fail-closed identity: a call whose name differs from meta().name
    /// must return E_INVALID_ARGS, never route silently.
    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError>;
}
#[cfg(feature = "conformance")]
pub fn assert_tool_conformance<T: Tool>(tool: &mut T);   // 八字段完备＋name 文法＋错名调用拒收

pub enum ExecArm { Program { path: String, args: Vec<String> }, Python { code: String }, Shell { text: String } }
                                    // 三臂恒三（L0 冻结面），故穷尽不标 non_exhaustive；discard::forecast 的入参
```

- **`params` 复用 `Payload`**：键序 BTreeMap＋拒浮点白拿；schema 约定属 S3 工具实珰。
- **conformance 三断言**：①meta 八字段形状合法（name 文法、disclosure 非空）；②错名调用拒收（E_INVALID_ARGS）；③拒收后工具仍可用（再次正确调用不受污染）。
- ExecArm 住本模块而非 runtime：discard::forecast（S2）先于 exec 工具（S3）需要它；工具面参数枚举属 tool 面（「可枚举的必用枚举」）。
- **`Effect::Connector { label }`**：目的地由**登记**而非逐调用参数定的那一类出站。`Egress` 的主语是一次调用（去哪台主机写在 args 里），`Connector` 的主语是一件工具（它恒只通往那一台 server）。**两者不得合并**：合并后要么让模型去填一个城自己已经知道的 `host`（一个可以填错的事实），要么让出站门拿不到目标而无法判定。发现它的时刻就是接线的时刻：`Effect::Egress` 写下时没有调用方，而第一次真调用当场拿到 `E_INVALID_ARGS: declares Egress but named no host`。
- **`ToolCall::action` 住本模块**：`IdemKey::derive` 的第三个入参由什么构成，本是 §8-6 明写「属工具面」的一条规则，而它此前**一处也不在工具面**——`bin::assembly` 与 `citysim::executor` 各写了一遍，且两遍不等：前者取 name 加 args，后者只取 name。**一条规则两个权威**（与下条 `ServerLabel` 同一理由），而这一次两个权威已经漂移出后果：`runtime::run` 每回合采一次 `t` 并把同一个 `t` 发给一波里的每次调用，于是 citysim 那一遍使**一波之内两次同名调用得同一把键**，`ToolBench::invoke` 的 dedup 当场判 `Duplicate`，第二次以「this call was already made」被拒——一件模型只能读作自己出错的事。规则回到它被指定的那一面，且由 `ToolCall` 自己回答，因为**它就是那个动作**。`id` 不进动作字节：两次只有 wire id 不同的调用是同一个动作。
- **`action` 上报序列化失败而不吞掉它**：搬进来之前那句是 `serde_json::to_string(&call.args).unwrap_or_default()`，而 `unwrap_or_default` 在这里产空串，会让两次参数不同的调用得同一把键——正是本条要消灭的那种碰撞。`Payload` 拒浮点且键恒为字符串，故这条失败臂今天不可达；但「不可达所以取默认值」与「不可达所以据实上报」之间，只有后者在它变得可达那天仍然是对的。
- **位次仍归调用方**：`seq` 说的是「这次调用坐在这一跑的第几位」，只有驱动那一跑的一方知道。把它一并收进 `ToolBench` 会让键在一次驱动内恒不重复，于是 dedup 永不触发，`dedup_runs_before_the_side_effect`（同一把键调两次、断言第二次不落地）连同它守的那条不变量一起变得写不出来。**收窄接口不值这个价**，故只搬动作字节；citysim 改用与 `bin::assembly` 同形的每跑计数器，是因为钟读数当位次逐字违反确定性第 7 条（「never from a clock」），而不是因为位次该归本模块。
- **`ServerLabel` 住本模块而非 protocol**：它是一台 MCP server 在城里的名字，也是它每件工具名的第一段（`{label}_{tool}`），故它的文法就是 `ToolName` 的文法减下划线——写在两个 crate 里就是一条规则两个权威。**减下划线是判定而非口味**：允许它会让 `apps_foo_bar` 同时读作两种拆法，而这个名字要路由一次调用。迁入后 `city::config_layers` 在文件边界就能解析它（city 只见 kernel），于是「非法标签」在 Run 存在之前就不可表示。

### 8-24 kernel::model（缝清单文件）

```rust
#[non_exhaustive]
pub struct BuildingPolicy { pub confidential: bool }      // S2 最小；构造子 new(confidential)，字段 S3+ 只加
pub struct ModelRequest { pub policy: BuildingPolicy, pub segments: [B3Hash; 4] }
                                    // segments＝冻结 prefix 分段哈希（与 prompt_assembled 同源）；线格式字段 S3 只加
pub struct ModelReturn { pub message: Payload, pub calls: Vec<ToolCall> }
                                    // message＝助手内容（入窗载荷）；calls＝请求的工具波（空＝本回合无工具，回合层据此收束）
pub trait Model {
    /// One provider call; adapters never sample clocks or read globals.
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;
}
#[cfg(feature = "conformance")]
pub fn assert_model_conformance<M: Model>(model: &mut M);
```

**canonical 会话类型族**（城内规范 Dialect 的缝上定义；gateway::dialect 只做翻译，两适配器与剧本模型消费同一形）：

```rust
#[non_exhaustive] pub enum Role { User, Assistant }                      // wire 枚举，开放
#[non_exhaustive] pub enum StopReason { EndTurn, ToolUse, MaxTokens }
#[non_exhaustive] pub enum ModelTag { Main, Digest, Transcribe }         // ALL: [ModelTag; 3]
pub struct SystemBlock { pub text: String, pub cache: bool }             // cache＝显式断点标记
#[non_exhaustive] pub enum ContentBlock { Text{text} | Thinking{thinking, signature}
                                        | RedactedThinking{data}
                                        | ToolUse{id, name: ToolName, input: Payload}
                                        | ToolResult{tool_use_id, content, is_error} }
pub struct ChatMessage { pub role: Role, pub content: Vec<ContentBlock> }
pub struct ToolDef { pub name: ToolName, pub description: String, pub input_schema: Payload }
pub struct Ceiling(NonZeroU64);  // 零不可表达：new(0) 即 None
pub struct ChatRequest { pub model: String, pub max_tokens: Option<Ceiling>, pub system: Vec<SystemBlock>,
                         pub messages: Vec<ChatMessage>, pub tools: Vec<ToolDef> }
pub struct ModelUsage { pub input_tokens: Tokens, pub output_tokens: Tokens,
                        pub cache_read_tokens: Tokens, pub cache_write_tokens: Tokens }
pub struct ChatResponse { pub content: Vec<ContentBlock>, pub stop: StopReason, pub usage: ModelUsage }
pub fn message_payload(content: &[ContentBlock]) -> Result<Payload, AxError>;  // model_returned 载荷的唯一成形处
pub fn value_has_float(value: &serde_json::Value) -> bool;               // wire 面浮点禁令的判定原语
```

- 工具入参／schema 用 Payload：浮点禁令在缝上即成立（这些字节逐字进 Ledger 载荷）；provider 送浮点工具入参＝E_WIRE_MISMATCH（fail-closed，城规优先）。
- ModelRequest 携 chat 字段（turn 的 Assembling 相组 ChatRequest 入请求）；ModelReturn 携 usage/stop/billed 三字段，另有 `bare()`（脚本最小构造）与 `from_response(resp, billed)`（tool_use 块→波，全量入账）两构造面；turn 的 model_returned 载荷随之增 usage／stop／billed_usd_micros（在场才写）。
- BuildingPolicy 住本缝而非 city：kernel 不能依赖外层，city::policy（P1）是它的**求值器**不是定义处（依赖反转，同 ledger 缝）。
- **一种活一个标签，不是一种活一个存储**：`ModelTag` 答的是「哪个端点、哪个模型接这类活」，而这件事已有一套机制——人登记一个 endpoint，再为一个标签选一个模型。因此转写进的是 `Transcribe` 这个 variant，而不是第二张表单与第二个凭据入口；多一个存储就是给同一个问题立第二个答案。`ALL` 是界面枚举标签时走的那条路，新增一个 variant 即改它的长度。
- conformance 两断言：①良性请求得 Ok 且 message/calls 形状合法（类型已保大半）；②Err 后适配器不中毒（再调仍得应答）。确定性不入 conformance（真 model 非确定），剑本适配器的确定性由 citysim 自证。

**思考记录与思考强度**（思考记录原样保留，消息往返恒按 provider 官方规定处理）

```rust
#[non_exhaustive] pub enum Effort { None, Low, Medium, High, XHigh, Max }   // 全序；Ord 按声明序
pub fn content_from_message(message: &Payload) -> Result<Vec<ContentBlock>, AxError>;  // 契约变更，见下
pub struct ChatRequest { /* …既有五字段… */ pub effort: Option<Effort> }
```

- **两个思考块，逐字保留**。provider 官方规定：「During tool use, you must pass thinking blocks back to the API for the last assistant message. Include the complete unmodified block back」；改动即 400 `invalid_request_error`，报文为「`thinking` or `redacted_thinking` blocks in the latest assistant message cannot be modified」。故 canonical 侧两个变体缺一不可，字段名与线上同名（`thinking`／`signature`／`data`），使翻译无重命名、使 Ledger 载荷可直接对照官方文档校读。`signature` 是「an encrypted copy of the full reasoning」，由 provider 验签，城内恒不解析、不截断、不重排。
- **为何不是「可选保留」**：两条独立理由各自足以定案。其一，丢弃即违约（上一条）。其二，`message_payload` 是 `model_returned` 载荷的唯一成形处，脱机重建窗口靠它；入账前剥掉思考块，重建出的窗口就是一个从未发送过的窗口——那是判负条件三（历史失真），而它一旦成立，本设计的一切保证同时作废。**第三条理由是缓存**：改写助手消息即换缓存前缀。
- **`content_from_message` 契约收紧**：`content` 键缺席仍折为空块表（脚本载荷的既定行为）；`content` 键在场却解不出，此前经 `.ok().unwrap_or_default()` 静默折为空——一个未知块类型会让整条助手消息在窗口里消失，而 Ledger 里它还在。两条历史就是这么长出来的。改为在场即必须解出，否则 `E_WIRE_MISMATCH`。返回类型随之由 `Vec<ContentBlock>` 变 `Result<Vec<ContentBlock>, AxError>`。
- **`Effort` 六级**：两家实际在用的就是 `none/low/medium/high/xhigh/max`，不另列其他方案。一处差别写清楚：**Anthropic 的 `effort` 只收五级**（官方 SDK 类型 `Literal["low","medium","high","xhigh","max"]`），`none` 不是它的取值，关思考在另一个字段 `thinking:{type:"disabled"}`；官方另记「Setting `effort` to `"high"` produces exactly the same behavior as omitting the `effort` parameter entirely」。OpenAI 侧六级同名（其 `minimal` 属 gpt-5 旧拼写，不入城内梯子）。故**两种兼容格式都拼得出全部六级**，否决「兼容格式拼不出就拒」这条路径；dialect 里只留 fail-closed 通配臂，含义改为「日后新增的级别尚未教会写」，恒不夹取到邻级。
- **不建每模型强度支持表**：任何 provider API 都不返回「本模型支持哪几级」。造一张我们填不满的表，就是给 provider 的真实行为立第二个权威；模型自己拒的原样透出。
- **`max_tokens` 是模型的事实，不是调用方的偏好**：Anthropic 要求每请求必带 `max_tokens`，且开思考时它是「思考＋回答」的总上限；OpenAI 则可缺席。两家的 `GET /v1/models` 都不返回该上限，所以它探不到，只能随模型登记。权威定在 `gateway::market::ModelEntry.max_output_tokens`，`CallShape.max_tokens` 由选型点从那一行解出；**任何调用处手写数字即错**——截断会发生在一个账上找不到理由的地方。
- **没人登记过的上限，载为「没人登记过」**：`Ceiling` 包 `NonZeroU64`，`ChatRequest.max_tokens` 是 `Option<Ceiling>`，于是「零」在类型上不存在，「缺席」也不等于零。缺席时 OpenAI 形不写该字段、取供应方自己的默认；Anthropic 形写不出请求，于是**拒**（`E_CONFIG_INVALID`，恢复语指向模型登记处），绝不在兼容格式那一层现编一个数。理由是实测：一个目录不认识的模型曾以 `max_tokens: 0` 上线，供应方答空、`stop` 记 `end_turn`、那次 run 冻结为「做完了」——**一个零上限造出的是一条看起来完成了的假历史**。

**模型看得见图**（`kernel::model::image`；形状 2 value）

```rust
#[derive(Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum ImageType { Png, Jpeg, Webp, Gif }        // 封闭枚举：城内认得的四种图
impl ImageType { pub fn mime(&self) -> &'static str; }   // "image/png" 等，两条 wire 共用
pub struct ImageRef { pub locator: Locator, pub media_type: ImageType,
                      pub width: u32, pub height: u32 }
#[non_exhaustive] pub enum ContentBlock { /* …既有五变体… */
    Image(ImageRef),
    ToolResult { tool_use_id, content, is_error, #[serde(default)] attachments: Vec<ImageRef> } }
```

- **账上存 locator 与整数尺寸，恒不存字节**。`ImageRef.locator` 是一条 `cas:` Locator，字节住 `memory::cas`；Ledger 载荷因而仍只有整数与短字符串，浮点禁令与载荷体量规则两条同时成立。宽高用 `u32` 而非比例：像素数是整数事实。
- **`ImageRef` 只定义一次**。Image 块与 ToolResult 的 `attachments` 是同一个值——四个字段一字不差——所以块写成 `Image(ImageRef)`（serde 内部标签，线上仍是 `{"kind":"image", "locator":…, "media_type":…, "width":…, "height":…}` 的扁平形），而不是把四个字段抄两遍。抄两遍就是一个概念两个权威，日后加一个字段要改两处。
- **`attachments` 带 `#[serde(default)]`**：既有 Ledger 里每一条 `tool_result` 都没有这个键，缺席即空表，所以每一份历史照旧重放。这是「只加不改」在 wire 面的具体形式。
- **`ImageType` 封闭而非 `non_exhaustive`**：它不是 provider 送来的开放词汇，而是城内决定收哪几种图；封闭枚举让 `mime()` 的 `match` 穷尽，加一种图必须同时回答「它的 MIME 是什么」。
- **conformance 增一条**：良性请求之外再发一次「含一个 Image 块」的请求，适配器同样必须返回而不是 panic。剧本模型（citysim）据此照旧通过——它不解释块，只按剧本作答。

### 8-25 kernel::secret

```rust
pub struct SecretRef { /* realm, name —— 私有 */ }
impl SecretRef { pub fn parse(raw: &str) -> Result<Self, AxError>;   // secret:<realm>/<name>；形状非法＝E_CONFIG_INVALID
                 pub fn realm(&self) -> &str;  pub fn name(&self) -> &str; }
// Display "secret:<realm>/<name>"；serde 字符串形；恒不入 Locator 文法（两解析器分立）

pub struct SecretSpan { pub start: usize, pub len: usize, pub provider: Option<&'static str> }
                                    // provider 来自形状表命中；None＝熵侦测器命中
/// Custody's detection half: shape table first,
/// entropy second. Pure, no regex, no backtracking; kani-provable
/// termination. Replacement/vaulting is the effect layer's (S3).
pub fn scan(bytes: &[u8]) -> Vec<SecretSpan>;

pub struct Sealed<T: zeroize::Zeroize>(/* secrecy::SecretBox<T> */);
impl<T: zeroize::Zeroize> Sealed<T> {
    pub fn new(value: Box<T>) -> Sealed<T>;
    /// Call sites are whitelisted by `xtask secret` (gateway::endpoint/
    /// native only, S3); the type refuses Debug/Display/Serialize so a
    /// sealed value cannot reach any sink even by accident.
    pub fn expose(&self) -> &T;
}
```

- **扫描两侦测器**：①形状表（SECRET_SHAPES：前缀＋字符集＋长度窗）为主；②熵阈为辅——无前缀命中的 token 段（base62/base64url 字符连段，长度 ≥ `ENTROPY_SPAN_MIN_BYTES=20`，pub(crate) 内部事务）且每字符熵 ≥ `SECRET_ENTROPY_MIN`（3.5 bits/char）。两集合并，重叠段归形状命中（provider 信息更多）。
- **熵的整数化**：kernel 禁浮点——香农熵以 millibit（1/1000 bit）计：定点 log2（shift-and-square，10 位小数位，循环界常数）；判式 `mb·den ≥ num·1000`（checked）。kani：任意输入终止、无 panic、无溢出。
- **Sealed 取 secrecy::SecretBox**（secrecy 0.10.3＋zeroize 1.9.0，钉版 B.7）：drop 即零化；无 Debug/Display/Serialize/Clone；trybuild 反例＝Sealed 值入 EventRecord/format! 编译不过。`PutSecret` 的命令面（S4）直用本类型。
- 误报是既知常态（入口无损可逆，出口才拒）；`E_SECRET_EGRESS` 的 subject 恒不回显命中字节（塑形在 gate::egress）。

### 8-26 kernel::discard

```rust
#[non_exhaustive]
pub enum Restoration { Tracked(Locator), Interred(Locator), Rebuildable { reason: String } }
pub struct Discard { /* paths, plan, taint, total_bytes —— 私有 */ }
impl Discard {
    /// Sole constructor (C14): restoration mandatory and scheme-checked —
    /// Tracked wants file:, Interred wants cas:, Rebuildable wants a
    /// non-empty reason; violations are E_DISCARD_IRREVERSIBLE.
    pub fn new(paths: Vec<Address>, plan: Restoration, taint: TaintSet, total_bytes: ByteLen)
        -> Result<Discard, AxError>;
    pub fn paths(&self) -> &[Address];  pub fn plan(&self) -> &Restoration;
    pub fn taint(&self) -> &TaintSet;   pub fn total_bytes(&self) -> ByteLen;
}

#[non_exhaustive] pub enum DiscardRequest { Planned(Discard),
                                            Unplanned { paths: Vec<Address>, taint: TaintSet, total_bytes: ByteLen } }
#[non_exhaustive] pub enum EscalateReason { FilesOverMax, BytesOverMax, RegistryAsset, Tainted }
#[non_exhaustive] pub enum DenyReason { NoRestoration }
#[non_exhaustive] pub enum DiscardVerdict { Allow, Escalate { reason: EscalateReason }, Deny { reason: DenyReason } }
/// The fifth door's decision table, sole authority —
/// gate::discard delegates wholly and only shapes the refusal.
pub fn decide(req: &DiscardRequest, registry: &Registry) -> DiscardVerdict;

#[non_exhaustive] pub enum DiscardForecast { Clear, Suspected { pattern: String } }
pub fn forecast(arm: &ExecArm) -> DiscardForecast;
```

- **decide 表**：Unplanned → Deny{NoRestoration}（无还原不可构造使 Planned 恒有 plan，Deny 只剩这一条路）；Planned：taint 非空 → Escalate{Tainted}（恒，无视规模）；paths 数 > `DISCARD_FILES_MAX` → Escalate{FilesOverMax}；total_bytes > `DISCARD_BYTES_MAX` → Escalate{BytesOverMax}；任一 path 命中 Registry Asset（以 file: Locator 前缀形归一查 is_asset；S2 取路径字符串相等）→ Escalate{RegistryAsset}；余 Allow。判序固定（Tainted→Files→Bytes→Asset），确定可重放。
- **forecast 三臂预判力递减**：Program 读 `(path, args)` 整体——basename ∈ {rm, rmdir, del} 或 git 携 reset --hard/clean 或 find 携 -delete；Python/Shell 子串表（rm 、rmdir、-delete、git reset --hard、git clean、os.remove、shutil.rmtree、os.unlink；Shell 另含 `>` 截断重定向）——可被混淆绕过，恒保守；git 兑底在 S3 checkpoint。子串表是 pub(crate) 数据面。
- kani：Discard 门 fail-closed——Unplanned 恒不 Allow；Tainted 恒不 Allow。

### 8-27 kernel::gate

```rust
pub struct GateContext { pub actor: String, pub now: TimeMs, pub item_id: ApprovalId }   // Escalate 造 item 所需；全由调用方注入
#[non_exhaustive] pub enum GateOutcome { Allow, Escalate { item: ApprovalItem }, Deny { refusal: Box<AxError> } }

pub fn domain(domain: &WriteDomain, target: &Address, taint: &TaintSet) -> GateOutcome;   // 判一个文件
pub fn reach(domain: &WriteDomain, area: &Address, taint: &TaintSet) -> GateOutcome;    // 判一块声明的区域（§8-46 末段）
#[non_exhaustive] pub enum EgressTarget { Loopback, Private, Public { host: String },
                                          Connector { label: ServerLabel } }   // 分类由效果层解好址后注入
#[non_exhaustive] pub enum EgressOutcome { Allow { first_public_egress: bool }, Deny { refusal: Box<AxError> } }
pub fn egress(spans: &[SecretSpan], target: &EgressTarget, prior_public_egress: bool) -> EgressOutcome;
// 没有 `spend` 门：它会判的 ladder 不存在，且它从无生产调用方。门由五减四。
#[non_exhaustive] pub enum CommitmentDecision { Approved, Denied }
pub fn commitment(decision: Option<&CommitmentDecision>, taint: &TaintSet, ctx: &GateContext,
                  action_desc: &str, artifact: &Locator) -> GateOutcome;
pub fn discard(req: &DiscardRequest, registry: &Registry, ctx: &GateContext,
               action_desc: &str, artifact: &Locator) -> GateOutcome;
pub fn spawn(parent: Depth, kind: &DelegateKind) -> GateOutcome;      // E_DELEGATION_DEPTH 的塑形处

#[non_exhaustive] pub enum DedupVerdict { Fresh, Duplicate }
pub fn dedup(seen: &BTreeSet<IdemKey>, key: &IdemKey) -> DedupVerdict;   // 去重恒先于副作用：调用序纪律＋citysim 不变量看守
```

- **`dedup` 的承兑人已经存在**：这个纯函数的 `seen` 集合是调用方的状态，故它成不成立取决于有没有人持有那个集合。今天持有它的有两处：工具面的 `runtime::bench`（一波之内同一把键只调一次），与命令面的 `bin::assembly::commanding::entrance`（`serve_one` 判在任何副作用之前，且重复的键得到第一次的答案）。**本模块的立面不变**——集合仍是调用方的，kernel 仍只回答成员关系；此处记的是「谁在兑现它」，因为一道没有调用方的门与没有门等价（`adversary/adversary-SPEC.md` §4 第三个发现量到的正是这件事）。
- **gate 是全库唯一 gate 码生产者**：五门 Deny 恒经 `AxError::refusal`（三段必填）；Domain 门 nearby＝domain 前缀表；Discard 门 alternative 恒可执行（分批或 Interred 后重试）；Egress 门 subject 只写位置与跨度数，恒不回显命中字节。
- **Escalate 的二源归一**（只有两源）：commitment 无决 → item{class: Commitment}；discard Escalate → item{class: DiscardEscalate}；均 source=Gate、tainted＝taint 非空（C15 标记位）。commitment 携 Denied 决定 → Deny（E_APPROVAL_DENIED，非 gate 码故用 failure 形）。
- **Taint 升档的 S2 实例**：Discard 门 Tainted 恒 Escalate（住 discard::decide）＋Escalate item 的 tainted 标记位（封 Policy/代答）。其余门的升档语义随其审批面出现时实例化（P1/P2），不造无消费者的规则。
- **首次公网出网**：`egress` 对 Public 且 `!prior_public_egress` 置 `first_public_egress`；NetNotice 挂信封属 pipeline（S3）。Loopback/Private 恒不触发（对 localhost 提醒注入只会训练模型忽略提醒）。
- kani：四门组合 fail-closed——reserved 目标恒不 Allow；spans 非空恒 Deny；Unplanned Discard 恒不 Allow；Delegated 再派生恒不 Allow。

### 8-29 kernel::address::SessionName（形状 2 value）

```rust
pub struct SessionName(String);
impl SessionName {
    pub fn parse(raw: &str) -> Result<Self, AxError>;   // 唯一构造点；修剪两端
    pub fn as_str(&self) -> &str;
}
```

一个人给一次会话的名字会变成它干活的目录，所以它必须恰好是一个地址段。用 `String` 就是把这条规则散到每个记得它的调用方里。

- **段规则向 `Address::parse` 问，不重写**：反斜杠、`:`、控制字符这些依据只有一份；本类型只多拒 `/`、`.`／`..`、`.sprawling` 与 64 字符上限。
- **修剪两端而不改中间**：人多敲一个空格不是意图；把中间的空格换成连字符则是替他取名。
- **64 字符**：超过这个长度的不是名字，是一件被写进名字栏的任务；它还要当别人机器上的目录名。
- **serde 进口复验**：`Deserialize` 走 `parse`（同 `Address`），于是一个从线上来的名字不会因为发送方没检查而变成路径。

### 8-30 kernel::change（形状 2 value）

```rust
pub enum Lines { Counted { added: u32, removed: u32 }, Binary }
pub enum How   { Added, Modified, Deleted, Renamed { from: String } }
pub struct FileChange { pub path: String, pub how: How, pub lines: Lines }
```

**为什么在 kernel 而不在产地**：三处需要这个形状——`memory::changes` 从两棵树上读出它、
`channels::wire` 携它、`web` 画它——而 `channels` 看不见 `memory`。定义两份再互相转换，
就是「一个文件变更是什么」有两个定义，而漂开的总是没人看的那个。这与 `Restoration` 当初落在这里
是同一条理由。

**`Lines` 是穷举枚而不是两个 `u32`**：二进制文件没有行数。把它拼成 `Counted { 0, 0 }`
与「碰过但没改」同形，而那是界面在报一个没人做过的测量——一条断言钉住两者序列化后不同形。

**`How::Renamed` 自带来处**：改名与「删一个加一个」是关于同两棵树的两个事实；
一个在判断 agent 是搬了代码还是重写了代码的人，需要这个差别。

**没有任何字段能装补丁文本**：补丁文本就是文件内容，而文件内容离开运行中的机器是
`secret::scan` 存在的理由。hunk 必须单独请求并同样受扫，所以它拼不进这个类型。

### 8-31 kernel::highlight（形状 1 判定）

```rust
pub enum Token { Heading, Strong, Emphasis, Code, Fence, Meta, Link, Marker, Quote }
pub struct Span { pub start: u32, pub len: u32, pub token: Token }
pub fn markdown(text: &str) -> Vec<Span>;
```

**落点只有一个，而它全是 Markdown。** 界面唯一读文件的地方是 `BuildingDoc.text`，
而 `read_building` 只收 `BUILDING.md` 与楼根目录下的 `*.md`。agent 写给下一个 agent 的计划与
交接就是这些文件，而人读它们时需要的是标题、列表、行内代码与围栏块彼此分开。

**为什么不上线不上服务端。** 另一种方案是服务端分词、线上走 span，理由是 syntect 在 wasm 里太重。
那条理由对 syntect 成立，对一个 Markdown 词法器不成立——它就几 KB。为一个尚不存在的第二实现
先把线格式撑大，是 ARCHITECTURE 明禁的「以假想复用为理由的抽象」。
**缝在 `markdown(&str) -> Vec<Span>` 这个签名上**：将来真需要语法引擎时，它去服务端、
线格式那时再长。

**在 kernel 而不在 web**：同 `kernel::change` 的理由——无 I/O、无时钟、输出穷举枚，
而且服务端有一天也要用它。`channels` 转出类型与函数，`web` 调用。

**偏移量恒在字符边界上**：切片由客户端拿着 `start`/`len` 去做，落在多字节字符中间的
偏移会让一页中文文档直接炸。一条断言钉住：每一个 span 都切得出来。

**Span 恒不重叠、按 `start` 升序**：重叠的 span 让渲染方必须自己决定谁赢，
那就是把词法规则的一半搬到了视图里。围栏块内部整块是 `Code`，不再分词——
本版没有语法引擎，而把 `**x**` 在 Rust 代码里读成粗体是在编造。

### 8-28 C17 从「首段」扩到「任一段」（形状 2 value 的一条原语）

```rust
pub fn is_reserved(&self) -> bool;   // 任一段 == RESERVED_PREFIX（原：仅首段）
```

**改它的理由是一个现存的洞，不是一个新需求。** 一次派活的写域由 `city::policy::write_domain()` 给出，而 `docs/templates/BUILDING.md` 的「Write domains」一节出厂就是一句空括号说明，于是 `write_prefixes` 为空、回落到 `[self.addr]`——**默认写域是整栋楼**。`runtime::tools::edit` 对路径只有 `WriteDomain::admits` 一道依据，`city::load` 又在**每次派活**时重读 `BUILDING.md`。三条合起来：一个 agent 现在就改得了它自己那栋楼的 `BUILDING.md` 与 `CONFIG.toml`——它自己的写域、`confidential`、思考强度与 MCP server 全在那两个文件里，而改动在下一次派活即生效。词汇表写着「一个 agent 改不了自己的账与自己的配置」，BUILDING.md 自己的第一行写着「agents read it and leave it unchanged」——**两句话今天都没有任何东西执行**。

- **一条规则，三处实例**：一个 scope 的治理字节住在它自己的 `.sprawling/` 里。城是 `<city>/.sprawling/`（今天已然），楼是 `<building>/.sprawling/`，房间是 `<building>/<room>/.sprawling/`。城的现行布局因此不是特例，而是同一条规则在根 scope 上的实例。
- **失效关闭，只会拒绝得更多**：改后 `is_reserved` 对任何含 `.sprawling` 段的地址答真，`WriteDomain::new` 与 `admits` 两处因此同时收紧。今天库里没有任何代码造得出嵌套的 `.sprawling` 路径，故本改动在行为上是空的，只把不变式先立起来。
- **不改的东西**：`Address::parse` 的文法不变（`.sprawling` 仍然是一个合法段名，只是含它的地址不再可写）；`RESERVED_PREFIX` 常量不改名，词汇表里它仍叫 reserved prefix。
- **字节随后才搬**：`CONFIG.toml` 与 `BUILDING.md` 搬进 `<building>/.sprawling/`，楼自己的 skill 存货放进 `<building>/.sprawling/skills/`。先立不变式再搬东西，是为了搬的那一刻目的地已经受保护；反过来就会有一段时间配置坐在新位置上而仍然可写。

## 8.5 两个设计（crate 级）

**A（选中）：规范字节住 kernel**——`EventRecord::canonical_line` 是全库唯一字节产地，jsonl／citysim 内存 Ledger／replay 三个消费者共用；conformance 断言 4 因此可写。杠杆：V8「三平台字节一致」收敛为一个函数的性质；换适配器不换字节。
**B（落选）：各适配器自产字节**（kernel 只给结构体，序列化归落盘方）——貌似「端口薄」，实则把规范散进每个适配器：内存 Ledger 与 jsonl 各持一份 serde 配置，漂移即 A19/A15 失真，而 conformance 只能对拍两实现、无法指认哪份是规范。落选理由：链对原始字节计算，字节即语义，语义必须一处。翻案条件：出现「同一记录合法多形」的需求（现设计明拒此需求）。

**第二对（error 侧）**：carrier 声明在 `AxCode::carrier()` 穷尽 match（选中）vs 分立静态表 `[(AxCode, Carrier); 35]`。选中方案让「新增码忘配 carrier」成为编译错误（非穷尽 match 不过编译）；静态表则要靠测试数分支。落选表的唯一优势是 specalign 好解析——但 specalign 对齐的是 SPEC 表与 enum，match 臂同样可数。

**第三对（S2，gate 侧）**：五门分立函数（选中）vs 单一 `gate::check(ActionEnvelope) -> GateOutcome` 总入口。总入口看似接口更窄，实则要造一个能同时表达五种异质入参的胖信封（写目标、出网目标、预算梯、审批决定、删除请求的交集形状），每门只读其中一角——胖信封即接口谎言，且无法逐门 kani（状态空间相乘）。五函数共享 GateOutcome 与 refusal 塑形纪律，组合在调用方（效果层按 Effect 字段选门）。落选的总入口若日后出现（如 wire 面需单帧过门），作为薄路由层另立，不回收五函数。

## 9 工作流程

写路径：调用方组 `EventDraft`（Payload 构造点已拒浮点）→ `Ledger::append`（适配器：定 seq/prev → `EventRecord::from_draft` → `canonical_line` → 落介质）→ 返回 `EventRef`。
读路径：适配器/replay 逐行 `parse_line` → 验 v/链/seq → `to_ref` 铸引用。
错误路径：一切构造与解析失败即 `AxError`（fail-closed），生产模块负责把它送到 carrier event（S2 起）。

## 10 实现逻辑

0. **AxError 内部装箱**：`{ code, Box<其余六字段> }`，serde flatten 保持 wire 形与字段序不变。理由：AxError 走每一道缝的返回位，扁平七字段 176 字节超 `result_large_err` 阈（128）；装箱后 16 字节，接口与序列化形态零变化。
1. 全模块零 I/O、零时钟、零随机；BTreeMap/BTreeSet only（Payload 经 serde_json::Map 默认 BTreeMap 间接满足）。
2. hex 编解码手写（16 行内，查表小写），不引 hex crate——C12 精神：依赖面只进钉版清单所列。
3. `EventKind`/`AxCode` 的 serde 呈现名逐 variant `#[serde(rename = …)]`（AxCode）与 `#[serde(rename_all = "snake_case")]`（EventKind）；`as_str` 与 serde 用同一份拼写（单测对拍）。
4. `Payload` 校验递归下降 serde_json::Value：`Number::is_i64 || is_u64` 之外即拒；数组与对象深入。递归深度由输入方（我们自己的写方）有界，读侧 parse_line 对深度不设限但对浮点恒拒。
5. `canonical_line` 用 `serde_json::to_vec`；`addr`/`ig` 的省略由 `skip_serializing_if` 表达；无 pretty、无空格。

## 11 边界枚举

空 Payload（合法，`{}`）；`ig:true` 且未知 kind（读侧放行跳过——replay 章）；`Seq::MAX.next()`（E_INVALID_ARGS，实践不可达但算术必 checked）；`Range` 端点相等（合法，单行/单字节）；`B0-0`（首字节）；`L1-1`（首行）；地址单段（合法）；`RESERVED_PREFIX` 恰为全路径（is_reserved 真）；`".sprawlingx/a"`（首段非 `.sprawling`，不 reserved——段边界判定）；Locator 尾随空白（拒）；hex 奇数长（拒）；`E_...` 码字符串反序列化未知码（serde 报错→读侧 fail-closed）。

## 12 错误处理（逐码答「能否定义掉」——规则十）

已激活的码：

- `E_INVALID_ARGS`（address/payload/seq 构造拒）：不可定义掉——解析面即 Taint 边界，输入天然不可信；类型把「构造后非法」定义掉了，「构造时非法」必须留码。
- `E_LOCATOR_INVALID`：同上；且与宽松接受严格互斥（fail-closed 是策略）。
- `E_VERSION_CONFLICT`（verdict 映射在 S3）：不可定义掉——乐观并发的存在理由就是冲突可发生。
- `E_LOG_VERSION_UNSUPPORTED`／`E_CAS_CORRUPT`：住装载期白名单，产生地在 memory/runtime（见各自 SPEC）。
- `E_STORAGE_FATAL`（存储写失败，装载期）：不可定义掉——磁盘满与介质 Io 失败在设计边界外；宁停不脏要求它直达进程级 fatal，不得伪装成可重试。S2 期初增设；memory 的 Io 映射已改正（memory-SPEC §12）。

S2 激活的码（逐码答「能否定义掉」）：

- `E_OUTSIDE_WRITE_DOMAIN`：不可——写目标是运行期输入，类型只能封构造后非法，封不住越域目标。
- `E_GATE_DENIED`：不可——Commitment 拒绝需要通用拒码；其余四门各有专码。
- `E_TAINTED_ACTION`：不可——Taint 升档后被拒的动作需要自述来路的码（生产在 P2 注入剧本接入时）。
- `E_BUDGET_EXHAUSTED`：不可——耗尽是审批不是错误，但模型需要可机读的码知道自己停在哪。
- `E_LOOP_SUSPECTED`：不可——停滞是观测事实；定义掉它等于假定模型不会循环。
- `E_GOAL_CONFLICT`／`E_REPAIR_BUSY`：不可——同资源相斥与修复串行化是机制存在理由；Queued/Conflict 是合法结局，码只在回传面携信息。
- `E_DELEGATION_DEPTH`：不消解（明裁：边界反馈优于沉默缺席）。
- `E_APPROVAL_PENDING`／`E_APPROVAL_DENIED`：不可——前拦等待与拒批都是用户可达状态。
- `E_EVIDENCE_MISSING`：部分定义掉——无证据 Done 已不可构造（类型半）；构造时拒绝仍需此码（运行时半，A6 双守）。
- `E_SECRET_EGRESS`／`E_DISCARD_IRREVERSIBLE`：不可——两门存在的理由即这两类越界可发生；类型已把「无 Restoration 的 Discard 值」定义掉，Unplanned 请求（exec 预判路）是剩余不可消部分。
- `E_CONFIG_INVALID`：不可——SecretRef 形状非法与明文入配置必须在反序列化即拒。

其余未激活码随其生产模块的 SPEC 章逐码作答（S3+）。

## 13 依赖选型

`serde`＋`serde_json`（规范字节与载荷；B.7 钉版）；`thiserror`（Display/Error derive；B.7）；`blake3`（唯一哈希，B.7 钉 S1；1.8.6 现行 stable）；`uuid`（v7 仅解析/格式化＋serde 特性，恒不启用生成特性——kernel 禁随机）。S2 增：`secrecy` 0.10.3＋`zeroize` 1.9.0（Sealed；B.7 钉 Stage 2–3，2026-08 复核为最新）。dev：`proptest`、`insta`、`trybuild`。不引：hex、rand、chrono/time（时间是入参）、regex（C12：熵与形状判定手写定点算法）。

## 14 硬编码声明

- `RESERVED_PREFIX = ".sprawling"`（冻结面）。
- `GENESIS_PREV = [0u8; 32]`（「创世行 prev＝64 个 0」）。
- `IDEM_DERIVE_V = 1` 与派生框架 `run(16B)||seq(8B LE)||action`（换框架＝升版本字节，旧键不撞新键）。
- Locator 文法字面（`cas:`、`file:`、`b3-`、`#L`/`#B`）：本 SPEC 的文法节即权威。
- `SECRET_SHAPES` 条目（公开 provider 令牌前缀，随外界增补）：`sk-ant-`（Anthropic）、`sk-proj-`（OpenAI）、`ghp_`/`gho_`（GitHub）、`AKIA`（AWS AccessKeyId）、`glpat-`（GitLab）、`xoxb-`（Slack）、`AIza`（Google API key）、`sk-or-v1-`（OpenRouter）、`sk-ai-v1-`（zenmux）、`gsk_`（Groq）。字符集与长度按各 provider 公开文档；条目形状见 §8-7。
- **聚合型转发商的令牌体是纯小写十六进制，故它们必须有形状条目而不能依赖熵侦测器**。熵侦测器的 `mixed_alphabet` 要求同时出现大写、小写与数字，这一条件本身是对的（城自己的 blake3 十六进制与 uuid 均单一大小写，否则每一行账本都会亮），但它使 `sk-or-v1-` 与 `sk-ai-v1-` 这类 64 位小写十六进制令牌两道侦测器都不响——形状表是它们唯一的网。S2 模块头早已写明「全小写的密钥避开本侦测器」，本条是那句话的具体后果。

Stage 2 追加：

- **`SUBAGENT_CTX_LOCK_DEFAULT = Tokens(65_536)`**（本 SPEC 定值，携证据）：主流上下文窗口 128k–200k token；Ephemeral 适用面（一次检索/摘要/跑测）按 20 回合×每回合约 3k token 上界估 60k；取 2^16 使锁高于任务上界、低于最小主流窗口之半——内耗循环在母窗口三分之一处被机械截断，正常任务不受掤。待 EVAL（P3）重估。
- `AUTONOMY_DEFAULT = Autonomy::Owner`、`CLOCK_STAMP_DEFAULT = ClockStampGranularity::Off`（直写，随类型落位）。
- 定点 log2 小数位数 10（熵判定内部事务）；`ENTROPY_SPAN_MIN_BYTES = 20`（熵侦测器最短跨度：主流 API key 最短约 20 字符；pub(crate)，改动随本 SPEC）。
- Roadmap 状态五值与 Memo 六字段的中文拼写：P2 spine_files 模板落盘时复审是否双语。

## 15 影响面

memory::jsonl／memory::cas／runtime::replay／runtime::fork／citysim 全部消费本 crate 的公开面；S2 全部 kernel 决断模块建立在 error/event 之上。公开面变更须与本文同一变更集（apisync 机器看守）。`consts_policy` 三项延后条目是显式债务。

## 16 测试与约束

- 单测（各模块文件内 `#[cfg(test)]`，测试模块头挂放宽 allow）：serde 拼写对拍（as_str×serde×表）；EventKind 计数 55／in-window 计数 8（以 `ALL` 数）；carrier 全映射非重复覆盖 35；构造子不变量（refusal 三段在场、failure 无 gate、retriable 默认 false）；Payload 拒浮点（含嵌套）；Address/Locator 拒绝面正反例；Seq/Version checked 溢出；IdemKey 版本字节在场。
- proptest：`Address::parse` 往返与 `is_within` 自反/传递/反对称；`Locator` Display↔parse 往返；`IdemKey` 重算恒等＋近旁输入不等样例；`Payload` 任意整数树恒过、含浮点树恒拒。
- golden（insta）：创世行＋一条 `building_created` 的 `canonical_line` 字节。
- conformance：对一个最小内存实现自证可跑；citysim 实现二证。
- 约束：`cargo clippy --workspace --all-targets -- -D warnings` 零告警；无 `unsafe`；文件前三行 MPL 头。
- S2 各模块测试面（逐模块文件内 `#[cfg(test)]`＋kani 镜像 proptest）：taint 并集单调／map 保集；write_domain reserved 恒拒／夺回计数；budget 溢出＝Exhausted／逐层报首超；backpressure 单调；stall 尾部连续语义；goal/repair 重叠矩阵；delegation 静动双层；registry verify 拒非证据 kind；spine 表解析正反例＋tally 对账三情形；completion 空证据／错 kind 拒；approval 三必经人矩阵＋自审拒＋tainted 封 Policy；config 字段交集空断言；tool/model conformance 自证；secret 双语料＋熵边界；discard 决策表全分支＋forecast 三臂正反；gate 五门矩阵＋dedup＋refusal 三段非空。

## 17 模型体验

零字节：kernel 本身不进任何 Run 的 prefix。它对模型的可见面只经两物间接达成——AxError 的 three-part refusal（错误即教学，边界反馈优于开头说教）与 in-window 事件的载荷字节（由上层模块产生）。本 crate 的任何变更不影响 prefix 缓存。

## 18 文档同步

- ARCHITECTURE.md §6 kernel 表：状态逐模块翻为已建。
- 本文 §8-4／§8-1 两表是 S2 `xtask specalign` 的数据面：改 enum 必同集改表。
- `consts_policy` 三项延后：锁、Autonomy、时钟档三项随各自的类型落地，§8-8 与 §14 已登。
- 设计缺口（存储写失败码）：已消——S2 期初增 `E_STORAGE_FATAL`。


### 模型端口多一扇门：说到一半的话

```rust
pub type Increments<'a> = &'a mut dyn FnMut(&str);

pub trait Model {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;
    fn call_streaming(&mut self, req: &ModelRequest, onto: Increments<'_>)
        -> Result<ModelReturn, AxError> { self.call(req) }
}
```

**默认实现是必要前提。** 它让「没有流的适配器」成为诚实的而不是坏的：调用方在同一时刻拿到同一个 `ModelReturn`，只是没看到任何增量。citysim 的脚本模型、离线重放、`gateway::native` 都不必改一个字。

**`Increments` 一个参数、无返回值，是刻意的。** 增量不是判断：下游任何东西都不得据它分支，而一个能拒绝的 sink 会让一个显示细节有能力弄失败一次调用。

**覆盖它的适配器欠同一个 `ModelReturn`，包括同样的失败。** 流被切断是一次读取错误，永远不是一个变短的回答——`ModelReturn` 恒不由增量拼出来。写进账本的那句话只从 `ModelReturn` 来，一次，在调用结算之后。

## 8-48 `kernel::node_id`：`NodeId` 搬出 `plan`，成为自己的模块

`kernel::plan` 的模块表行是 `decision`：树判定什么可以开工、一个枝值多少、一个持有节点走两个出口里的哪一个。
`NodeId` 不判定任何事——它只说清「一个良构地址长什么样」，并在**唯一的构造点**把别的一律拒掉，
好让下游没有一处需要再问一遍。那是 `value`，§9 的形状 2。一个文件装两个形状，正是 §9 说该分家的依据。

**手写的 `Deserialize` 是这次搬迁里唯一需要小心的东西**，它也正是这个类型存在的理由：
derive 出来的那个会把线上任意字符串收下、交回一个从没过过 `parse` 的 `NodeId`。搬家时它跟着走，没有被 derive 顶替。

**公开路径一字未动**：`kernel::NodeId` 仍是 `kernel::NodeId`，因为 `lib.rs` 的 `pub use` 吸收了这次移动。
`cargo public-api` 记的是**定义模块**，所以 `channels` 与 `collab` 的基线里
`kernel::plan::NodeId` 变成了 `kernel::node_id::NodeId`——**签名的形状没变，变的是它在哪儿被定义**。
两份基线因此同变更集重生，两份 SPEC 各记一行。

1,085 → 958，`[file_length.predating]` 里的那一行随之划掉。

### 8-36 kernel::gate 目录化（形状：判定簇即目录）

`gate.rs`（875 行）按判定簇切为 `gate/` 目录：`domain.rs`（73–107）、`egress.rs`（117–289，
含 `EgressTarget`／`EgressOutcome`／`EgressAllowlist`）、`spend.rs`（294–321）、
`commitment.rs`（325–362，含 `CommitmentDecision`）、`govern.rs`（366–510，
`discard`／`delegation`／`govern`／`spawn`＋`PROPOSAL_EXCERPT`）、`dedup.rs`（514–528，
含 `DedupVerdict`）。共享私有 `item()`（50–68）归 `gate/item.rs`（`pub(crate)`，
四门 Escalate 的唯一造项点）。`GateContext`／`GateOutcome` 留 `gate.rs`（改索引文件，
零逻辑）。簇间零调用边（各门只调 `item`＋本簇外模块判定函数）；对外签名逐字节不变。
完成检查：SPEC 同变更集 → modmap＋apisync 绿 → citysim 同种子字节重放。

### 8-37 kernel::plan／spine 目录化（形状：树／份额／节点／阻塞）

`plan.rs`（958 行）按节点类型（`plan/node.rs`：`StopCause`／`Held`／`PlanExit`／`PlanNode`）／
树结构（`plan/tree.rs`：`PlanTree` 的安置／断言／除法／查询／`claim`／`progress`，测试住 `plan/tree/tests.rs`）／
份额分发（`plan/share.rs`：`hand_out` 提为 `pub(crate)` 自由函数，`PlanTree.nodes` 开 `pub(crate)` 可见）／
阻塞查询（`plan/blocking.rs`：`refusal`／`first_cycle` 提为 `pub(crate)` 自由函数）四簇切目录；
`Held` 增 `pub(crate) of` 构造器（原元组构造跨文件不可见），`PlanTree` 补回其 `derive(Debug, Clone, PartialEq, Eq)`；
`spine.rs`（883 行）按行类型（`spine/row.rs`：`RoadmapStatus`／`EvidenceCell`／`RoadmapRow`／
`RoadmapShape`／`NewChild`／`ROADMAP_COLUMNS`／拼写表）／文法（`spine/grammar.rs`：`parse_status` 移入、
`locate_table` 等四私有函数开 `pub(crate)`、测试住文件内）／改写（`spine/rewrite.rs`：
`draw_row`／`set_roadmap_status`／`insert_children`／`well_formed`／`rewrite`，测试住
`spine/rewrite/tests.rs`）／备忘（`spine/memo.rs`：`MEMO_OUTLINE_FIELDS`／`check_memo_shape`／
`ScopeChange`／`WriteMoment`）四簇切目录；`spine.rs` 剩 38 行索引。
依赖单向：`plan` 用 `spine` 的行类型，`spine` 不反向用 `plan`。对外签名逐字节不变。
完成检查：同 8-36。

### 8-38 kernel 值簇目录化（形状：值归值，判归判）

`event.rs`（775）按标识（`event/identity.rs`：`RunId`／`Seq`／`TimeMs`）／种（`event/kind.rs`）／
载荷（`event/payload.rs`：`Payload`／`EventDraft`／`EventRecord`／`EventRef`，insta 快照随测搬
`event/snapshots/` 并改名）切分；`error.rs`（464）按码（`error/code.rs`）／拒（`error/refusal.rs`）／
形（`error/shape.rs`，避 `module_inception`）切分；`discard.rs`（561）按请求（`discard/request.rs`）／
判定（`discard/verdict.rs`，含 kani）／预报（`discard/forecast.rs`，含 proptest）切分；`secret.rs`（441）按
跨度（`secret/span.rs`）／扫描（`secret/scan.rs`，含 kani＋proptest）／封存（`secret/sealed.rs`）切分。
各改索引零逻辑，对外签名逐字节不变（下游 11 基线仅规范路径记法，各 SPEC §6 同集一句；
secret 门白名单随 `sealed.rs` 搬家）。完成检查：同 8-36。

### 8-39 kernel::model 目录化

`model.rs`（516）切成四文件：`model/wire.rs` 收线上会话的词汇（dialect 无关的规范形）（`BuildingPolicy`／`Role`／
`StopReason`／`SystemBlock`／`DialectKind`／`ModelTag`／`Effort`／`ContentBlock`／`ChatMessage`／
`ToolDef`／`ChatRequest` 含 `empty`／`ModelUsage`／`ChatResponse`）；`model/seam.rs` 收一次调用两个方向
所载之物与内容↔载荷两转换（`ModelRequest`／`ModelReturn` 含 `bare`／`from_response`、`message_payload`／
`content_from_message`／`value_has_float`）；`model/conformance.rs` 收 feature 门后的一致性断言；
`model/tests.rs` 收原 `mod tests`（6 个 `#[test]`，断言与名字不动，补 `AxCode`／`Payload`／`B3Hash`／
`Map` 四行 `use`，因父文件不再直接引它们）。`model.rs` 剩 75 行：`//!` 文档、两 `mod`、两 `pub use`、
`Increments` 与 `pub trait Model` —— **trait 必须留在原路径**，`xtask depmap` 只准 seam 清单文件
（ARCHITECTURE §3 记的正是 `crates/kernel/src/model.rs`）声明 `pub trait`。
无字段开放（子模块间只引 `pub` 类型）。`lib.rs` 的 13 行再导出一字未改，故公共面路径不变、
apisync 未重写基线。完成检查：`cargo check`／`clippy -D warnings`／`nextest`（205 passed）／
`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-40 kernel::locator 目录化

`locator.rs`（477）只作一次切分：原内联 `mod tests` 整段迁到 `locator/tests.rs`（8 个 `#[test]`
含 2 条 `proptest!`，断言与名字一字不动，`use super::*` 与 `use crate::error::AxCode` 原样保留），
父文件尾部改留 `#[cfg(test)] mod tests;` 并原样带上那份 `#[allow(...)]` 列表。`locator.rs` 剩 375 行：
文法、`B3Hash`／`GitOid`／`Range`／`Locator` 四型与全部解析、呈现、十六进制原语都留在原路径，
故规范路径与公共面逐字节不变，apisync 未重写基线。无字段开放。完成检查：`cargo check`／
`clippy -D warnings`／`nextest`／`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-41 kernel::highlight 目录化

`highlight.rs`（437）只作一次切分：原内联 `mod tests` 整段迁到 `highlight/tests.rs`（12 个 `#[test]`，
断言与名字一字不动，`use super::*` 原样保留，两个夹具 `cut`／`tokens` 随测试同迁、不复制），
父文件尾部改留 `#[cfg(test)] mod tests;` 并原样带上那份含 `clippy::arithmetic_side_effects` 的
`#[allow(...)]` 列表。`highlight.rs` 剩 304 行：`Token`／`Span` 两型、`markdown` 与全部行内词法
原语（`read_line`／`marker_len`／`opens_fence`／`fence_len`／`inline`／`scan`／`links`／`claim`／
`push`）都留在原路径 —— `scan` 带 `argument_count` 豁免，键 `crates/kernel/src/highlight.rs::scan`，
因此不得搬家。规范路径与公共面逐字节不变，apisync 未重写基线。无字段开放。完成检查：
`cargo check`／`clippy -D warnings`／`nextest`／`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-42 kernel::tool 目录化

`tool.rs`（430）只作一次切分：原内联 `mod tests` 整段迁到 `tool/tests.rs`（6 个 `#[test]`，断言与名字
一字不动，`use super::*` 原样保留），父文件尾部改留 `#[cfg(test)] mod tests;` 并原样带上那份
`#[allow(...)]` 列表。`tool.rs` 剩 332 行：`ToolName`／`ServerLabel`／`TimeoutMs`／`Effect`／
`Temporal`／`CostTier`／`RenderIntent`／`ToolMeta`／`ToolCall`／`ToolOutcome`／`ExecArm` 与
`pub trait Tool`、feature 门后的 `conformance` 子模块都留在原路径 —— **trait 必须留在原路径**，
`xtask depmap` 只准 seam 清单文件（ARCHITECTURE §3 记的正是 `crates/kernel/src/tool.rs`）
声明 `pub trait`。规范路径与公共面逐字节不变，apisync 未重写基线。无字段开放。完成检查：
`cargo check`／`clippy -D warnings`／`nextest`／`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-43 kernel::config 目录化

`config.rs`（418）只作一次切分：原内联 `mod tests` 整段迁到 `config/tests.rs`（9 个 `#[test]`，断言与
名字一字不动，`use super::*` 与 `use std::collections::BTreeSet` 原样保留），父文件尾部改留
`#[cfg(test)] mod tests;` 并原样带上那份 `#[allow(...)]` 列表。`config.rs` 剩 193 行：
`ClockStampGranularity`／`LayeredValue`／`ClockZone`／`SandboxLimits`／`McpServer`／
`McpTransport`／`FrozenConfig`／`LiveConfig` 与 `freeze` 都留在原路径 —— `freeze` 带
`argument_count` 豁免，键 `crates/kernel/src/config.rs::freeze`，因此不得搬家。规范路径与公共面
逐字节不变，apisync 未重写基线。无字段开放。完成检查：`cargo check`／`clippy -D warnings`／
`nextest`／`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-44 kernel::approval 目录化

`approval.rs`（413）只作一次切分：原内联 `mod tests` 整段迁到 `approval/tests.rs`（3 个 `#[test]`，
断言与名字一字不动，`use super::*` 与三个夹具 `item`／`policy`／`resident` 原样保留，缩进整体退
四格），父文件尾部改留 `#[cfg(test)] mod tests;` 并原样带上那份 `#[allow(...)]` 列表。
`approval.rs` 剩 262 行：`ApprovalId`／`ApprovalSource`／`ApprovalClass`／`ClusterKey`／
`ApprovalItem`／`PolicyClass`／`PolicyMatcher`／`PolicyVerdict`／`Policy`／`PolicyApplication`／
`PolicyExpiry`／`PolicyRevocation`／`Autonomy`／`Answerer`／`AnswerVerdict` 与
`match_item`／`expiry`／`may_answer` 全部留在原路径，kernel 作为依赖树根，公共面的规范路径
逐字节不变，apisync 未重写基线。无字段开放。完成检查：`cargo check`／`clippy -D warnings`／
`nextest`／`xtask modmap`／`length`／`header`／`apisync` 全绿。

### 8-45 kernel::schema（形状 4 adapter）——线上每个值的 JSON Schema，从 serde 读的那一份声明派生

**需求**：客户端（`client/src/wire.ts`）由 Rust 的 wire 类型生成，而 wire 携带的值大半是 kernel 的（`RunId`／`Seq`／`Address`／`EventRecord`／`AxError`／`ApprovalItem`……）。它们的 JSON 形状必须有且只有一个权威，而那个权威已经存在：类型声明上的 `#[serde(...)]`。

**接口**：feature `schema`（缺省关，`schemars` 为可选依赖）。开启时，每个出现在帧里的类型带 `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`——派生宏读的正是 serde 读的那些属性（`rename_all`／`transparent`／`flatten`／`skip_serializing_if`／`deny_unknown_fields`／`try_from`），所以形状不可能与编码漂开。手写 serde 的五个值（`AxCode`／`IdemKey`／`B3Hash`／`GitOid`／`Locator`）在 `crates/kernel/src/schema.rs` 里各写一条 `impl JsonSchema`：一律是带 `pattern` 的 `string`，`AxCode` 的 `enum` 取自 `AxCode::ALL`——同一张表既产 `as_str` 也产 schema。这五条住一个文件而不是各回原文件，因为 `locator.rs` 已有 375 行，三条 impl 会把它推过 400 行预算；文件只装 `impl` 与它们引用的形状字符串，一处判定也没有。

**缺省公开面不变**：feature 关着时 `cargo public-api -p kernel` 逐字节同以前，基线不动；`--all-features` 下多出的只是 `JsonSchema` 实现。产品二进制不开它。

**被否**：（a）在 channels 用 schemars 的 remote derive 镜像这四十个类型——每一个镜像都是同一形状的第二个权威，kernel 改一个字段名，镜像静默不动，客户端在握手通过后误读；（b）不加 feature、无条件派生——把 `schemars` 压进产品二进制，换来的只是省一个 cfg。

### 8-46 kernel::write_domain 增 `WriteDomain::Documents`（形状 1 判定 + 形状 2 value）

**需求**：City Hall 的两位居民（Mayor、clerk）只写 Markdown。给他们一个「整栋楼可写」的写域，再靠提示词请他们别碰代码，是把不变量交给措辞；写域本身要能表达「只写文档」。

**接口**：

```rust
pub struct DomainPrefixes { /* prefixes: Vec<Address> —— 私有 */ }
impl DomainPrefixes {
    /// C17 的唯一构造点：任一 reserved 成员拒整个前缀集。
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;      // E_INVALID_ARGS
    pub fn iter(&self) -> impl Iterator<Item = &Address>;
}

pub enum WriteDomain {
    /// 前缀内的一切文件。
    Everything(DomainPrefixes),
    /// 前缀内的 Markdown 文档，且永不含任何 `Roadmap.md`。
    Documents(DomainPrefixes),
}
impl WriteDomain {
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;      // ＝ Everything，旧调用点逐字不变
    pub fn documents(prefixes: Vec<Address>) -> Result<Self, AxError>;
    pub fn admits(&self, target: &Address) -> DomainVerdict;
    pub fn prefixes(&self) -> impl Iterator<Item = &Address>;
}

#[non_exhaustive]
pub enum DomainVerdict {
    Within,
    Outside { prefixes: Vec<String> },
    /// 落在前缀内，但不是这个写域写的那种文件。
    NotWritable { reason: DocumentReason },
}

#[non_exhaustive]
pub enum DocumentReason { NotMarkdown, ThePlan }

pub const ROADMAP_FILE: &str = "Roadmap.md";   // 随 ROADMAP_COLUMNS 住 spine::row
```

- **变体带私有值而非公开字段**：`WriteDomain::Documents` 可以被外部写出来，但只能拿一个已经过 C17 检查的 `DomainPrefixes` 去写。一个构造点，不因为枚举化而多出第二个。
- `admits` 判定序（fail-closed，先拒后放）：target `is_reserved()` → `Outside`；不在任一前缀内 → `Outside`；`Everything` → `Within`；`Documents` → 文件名不以 `.md` 结尾 → `NotWritable { NotMarkdown }`；文件名等于 `ROADMAP_FILE`（任意深度、任意楼）→ `NotWritable { ThePlan }`；否则 `Within`。
- **为什么计划文件在写域这一层就拒**：`Roadmap.md` 的唯一编辑入口是 `plan` 工具（`kernel::spine::rewrite` 的重写），它保证六列表格重写后仍成立。放任 `edit` 直接改，等于给同一条规则第二个权威，而漂掉的那个总是没人读的那个。
- `gate::domain` 对 `NotWritable` 出 `E_OUTSIDE_WRITE_DOMAIN` 三段式：规则「这个写域只写 Markdown 文档」，违规指出是哪一种，替代给出「改 `.md`」或「用 `plan` 工具」。
- 被否：给 `WriteDomain` 加一个 `documents: bool` 字段——布尔旗标不是穷尽枚举，且「只写文档」与「什么都写」是两条策略而不是一条策略的一个开关。

**一个区域不是一个文件。** `Effect::Write { domain }` 是工具在建造时**声明**的区域（`hall/mayor`），不是某次调用要写的文件；`bench::admit` 却把这块区域交给 `gate::domain` 判，于是 `Documents` 域对着 `hall/mayor` 答「不是 Markdown 文档」——**Mayor 从建城起就写不了任何一份文档**，而写文档正是它唯一被交代的事。假供应方回一次 `edit <city>/hall/note.md` 即复现（`E_OUTSIDE_WRITE_DOMAIN`，主语是居民地址）。修法是让两个问题各有一个权威：

```rust
impl WriteDomain { pub fn reaches(&self, area: &Address) -> bool; }   // admits 的前缀半段：非保留区、且在某个前缀内
pub fn reach(domain: &WriteDomain, area: &Address, taint: &TaintSet) -> GateOutcome;   // 区域：只问够不够得到
```

- `gate::reach` 判声明的区域：`reaches` 为假出与 `domain` 同一段 `Outside` 三段式（同一处产出，`outside` 提为二者共用），为真 `Allow`；它永不问文件名，因为区域没有文件名。
- `gate::domain` 判文件，一字不改；它的调用方从 bench 挪到 **`runtime::tools::edit`** 拿到路径的那一刻（runtime-SPEC §8-36）——从前 edit 自己用 `admits` 只处理 `Outside`、漏掉 `NotWritable`，于是 `Documents` 域内的 `<city>/hall/foo.rs` 在工具这一层是放行的，只是被门口那条错判挡住了而已；现在门口只判区域，工具这一层必须判全，而它判全的方式是调同一个门。
- 被否：让 bench 读 `call.args["path"]`——那把 bench 和一个工具的参数名绑在一起，而 `exec` 同样声明 `Write` 却没有路径。

### 8-47 kernel::approval：City Hall 的两个常量与 clerk 的默认代答

```rust
// consts_policy
pub const HALL_BUILDING: &str = "hall";
pub const HALL_MAYOR: &str = "hall/mayor";
pub const HALL_CLERK: &str = "hall/clerk";
```

- `Autonomy` **不加变体**：`Owner | Delegate(ResidentId) | Deferred` 已经能说出「clerk 代答」——`Delegate(ResidentId::new(HALL_CLERK))`。新增一个 `Clerk` 变体会让同一件事有两种写法，而 `may_answer` 要为两种都作答。
- `may_answer` 逻辑一字不改：clerk 之所以能答，是因为它就是被任命的 delegate；三必经人类与 tainted 依旧 `HumanOnly`，clerk 自己发起的条目依旧 `SelfApprovalBarred`。kernel 侧只加常量与一条断言 clerk 走通全路的测试。
- genesis 侧（`bin::assembly`）在 `city_initialized` 之后写一条 `autonomy_changed`，值为 `delegate:hall/clerk`——记录在账上而不是写死在缺省值里，因为「谁来答」是这座城市的一个决定，人可以改它，改动要有一行历史。

### 8-49 kernel::gate::undoable：拿不回来的那一类外部效应（形状 1 判定）

```rust
/// 「哪一台 server 上的哪一件工具」——这两样恒同行，故是一个有名字的值。
pub struct ConnectorCall<'a> { pub label: &'a ServerLabel, pub tool: &'a ToolName }

/// 一件 connector 工具的远端名字，是不是这座城收不回来的那一类。
pub fn reaches_the_undoable(call: &ConnectorCall<'_>) -> bool;

/// 是就升给人，否就放行。恒不 Deny。
pub fn undoable(ctx: &GateContext, call: &ConnectorCall<'_>,
                artifact: &Locator, taint: &TaintSet) -> GateOutcome;
```

**问题**：城里每一条「会造成后果」的路径都配了一条回头路——`Discard` 没有 `Restoration` 就构造不出来，工具波前后各有一个 git fence，写域外的写会被拒。桌面连接器一条都对不上：`desktop.act` 在这个人自己的机器上按下的键，`desktop.clipboard` 覆盖掉的那段文本，城里没有任何一处存过它们的旧值，也没有任何一处能把它们放回去。

**故它走的是 Escalate，不是 Deny**（与 `delegation`／`govern` 同一形状）。拒绝会让这件工具等于不存在；放行则是让一个模型在没人看着的时候按下别人的键盘。中间那一格正是 Gate 存在的理由：**这是人的决定**，且 `GateOutcome` 本来就有这一格。

**依据是远端名字的前缀 `desktop.`，且判得精确而不是猜**：一件 connector 工具在城里的名字是 `{label}_{sanitise(远端名)}`，`label` 就在 `Effect::Connector` 里带着，所以把 `{label}_` 从头上摘掉剩下的就是远端名，无须猜。一栋楼把这台 server 挂成 `desk`，工具叫 `desk_desktop_act`；挂成 `desktop`，工具叫 `desktop_desktop_act`——两种都判得出来，而「名字里含 desktop」这种读法会把一栋楼自己写的 `notes_desktop_layout` 也判进去。

**`ConnectorCall` 是一个值而不是两个参数**：label 与工具名单独拿出来都判不了任何事——依据恰恰是「把 label 从工具名头上摘掉之后剩下什么」，故它们是同一个事实的两半（`xtask length` 的 4 参数尺子把这一点问了出来）。

**cluster key 取 label，不取工具名**：人被问的是「这个连接器可以碰运行中的机器吗」，一个问题一次。逐工具问会训练人闭着眼点过去，而那正是这道门想防的事。

**这道门与 `BUILDING.md` 的 `desktop:` 是两回事，次序也固定**（city-SPEC §8-25）：楼那一位开关决定这台 server **接不接得上**，这道门决定接上之后**每一次调用要不要问人**。楼说「是」不等于人对每一次点击说「是」。

**恒不为它新增 `Effect` 变体**：`Effect` 是路由字段，`Connector` 已经把这一类调用路由到出网门了；再加一格会让每一处 `match Effect` 都要回答一个与它无关的问题。这道门叠在出网门之后，两道各答各的——出网门答「这些字节能出去吗」，本门答「这个后果收得回来吗」。


## 8-52 一次工具调用产出的图

`ToolOutcome` 多一个字段 `attachments: Vec<ImageRef>`，`#[serde(default)]`，旧历史读成空表。

**为什么不放进 `result` 里**：`result` 是给模型读的文本载荷，一个埋在 JSON 里的 `cas:` 定位符对模型永远只是一串字。要让模型**看见**这张图，它必须成为 `ContentBlock::ToolResult.attachments` 的一员——那是 `ContentBlock::Image` 备好的位置，而 `runtime::turn::wave` 是唯一一处把 `ToolOutcome` 变成 `ContentBlock` 的地方，于是这个字段是那条路上唯一缺的一段。

**字节不在这里**：`ImageRef` 携定位符与两个整数边长，字节住 `memory::cas`，出线前的最后一刻才由 `gateway::endpoint` 取出来编码。账本因此仍是一份人能读的文件。

**唯一的生产者是 `browser` 工具的 `screenshot`**（`bin::browser_tool`）；其余每一个工具显式写空表，因为「没有图」是一句要说出口的话，不是一个可以省略的默认。

### 8-53 `kernel::Increment`：模型正在产出的一小块，以及它来自哪一路（形状 2 值）

```rust
pub enum Increment { Said(String), Thought(String) }
pub type Increments<'a> = &'a mut dyn FnMut(&Increment);
```

- **两路而不是一路**：散文是答案，推理是得到答案的过程。把两者并进一个缓冲区，对一个把大部分输出花在推理上的模型，等于把草稿当答案给人看。
- **哪一路是这一小块自己的一部分**，于是没有任何下游读者需要猜。`ContentBlock::Thinking` 是它结算之后的落点，两处说的是同一件事的两个阶段。
- **同一块里两路都有时散文优先**：没有供应方这样发；真发了，它是在同一瞬间既回答又推理，而人在等的是答案。

### 8-50 `kernel::reach`：一次调用停在哪一段（形状 2 值）

```rust
pub enum Named { Resolved(u32), NotFound, Refused(String), ProxiedAway }
pub enum Connected { Open, Refused, Silent, Failed(String), Skipped }
pub enum Answered { Status(u16), NameNotUsable(String), HandshakeFailed(String), Unreachable(String) }
pub enum Through { Direct, Environment(String), Excluded, LocalAddress, Disabled }
pub enum Proxying { ExceptLocal, Always, Never }
pub struct Reach { host, named, connected, answered, through, elapsed_ms }
```

- **为什么值在 kernel 而读数在 gateway**：这套词汇要同时被 gateway（做测量）与 channels（往线上送）叫出名字，而 channels 不依赖 gateway。与 `DialectKind` 同一条依赖倒置：**定义住在这里，求值住在拿得到套接字的那一层**。
- **四段各有各的下一步**：名字解不出（检查拼写或代理）、连不上或没人应（防火墙、端口、没起来的代理）、握手失败（主机名不是合法 DNS 名、证书不受信）、供应方答了状态（401 是密钥，404 是 base_url 末尾多了路径）。**一条 `error sending request for url (...): operation timed out` 里这四种全长一个样**，而人对着它无事可做。
- **`Resolved(0)` 不可表达**：解出零个地址就是 `NotFound`，不是「解出了，零个」。
- **`ProxiedAway` 是一段诚实的缺席**：有代理时名字与套接字都由代理去做，城自己再解一次名，报的是一条请求不会走的路。
- **`Through` 的五格里有四格都是「没走代理」，分开是因为下一步不同**：没人指定（`Direct`）、这台电脑自己的 `NO_PROXY` 排除了它（`Excluded`）、城按规则把打到这台电脑的调用摘了出来（`LocalAddress`）、人为这个端点定下了 `Never`（`Disabled`）。一个人对着 `direct` 无事可做，对着这四句里的任何一句都有。
- **`Proxying` 是一条设置而不是一个常量**：「打到这台电脑的调用不走代理」对常见的那一类机器是对的——代理拦回环会让本地推理服务器由别人的网关代答 502——但它对每一台机器都成立这件事从来没有被证明过。另外两格各自对应一类真实的机器：把出网一律送进回环上的审计中继、因而要求连回环也走代理的组织（`Always`），以及虚拟网卡已经在路由层接管全部流量、于是一条过期的代理变量只会弄坏调用的机器（`Never`）。**败给的方案**：把它做成城一级的开关——那需要一条命令、一个折叠、一份投影和「改设置要重建哪些客户端」的答案，而端点的 `EndpointTuning` 本来就是「人关于这个端点还定下了什么」的存放处，探测与调用共用它因而恒不会各说各话。
- **时间是参数**：`elapsed_ms` 由调用方盖戳，因为全城只有 Main 采样时钟。

### 8-51 `Ledger` 端口的第二个方法：一波一屏障

```rust
pub trait Ledger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError>;  // 默认逐条 append
}
```

- **为什么端口要长这一只手**：一次持久写的代价是一道磁盘屏障，而屏障的价钱与骑在它上面的记录条数无关。实测（windows-x86_64 NVMe 一档机器）一条一屏障 585.2 µs／条，五十条一屏障 13.2 µs／条，其中真正的写约 2.5 µs。手上已经攥着一波的调用方按条交付，付的就是四十倍于磁盘所要的价钱。`memory::JsonlLedger` 从一开始就能一波一屏障，端口却没有一句话让它做——于是它在生产代码里没有调用方。
- **默认实现是诚实的**：逐条 `append`，任何没有批量能力的存储照此就是正确的，不必为了满足端口去假装合并。
- **契约逐元素成立**：答 `Ok` 即整波已落盘，refs 按给入顺序回来。第一条拒绝结束整波，其前的记录可能已经落盘——这与单条 `append` 在它后面那条失败时给出的承诺完全一样。
- **否决「显式屏障动作」**：让 `append` 只写不同步、另给一个 flush 动作，会让一条已经发出的 `EventRef` 指向一条可能还不存在的历史，而那正是这个类型存在的全部意义。
