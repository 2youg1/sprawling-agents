-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address
import crates.kernel.spec.Approval
import crates.kernel.spec.Backpressure
import crates.kernel.spec.Blockage
import crates.kernel.spec.Budget
import crates.kernel.spec.Change
import crates.kernel.spec.Completion
import crates.kernel.spec.Config
import crates.kernel.spec.ConstsExternal
import crates.kernel.spec.ConstsPolicy
import crates.kernel.spec.Degradation
import crates.kernel.spec.Delegation
import crates.kernel.spec.Discard
import crates.kernel.spec.Error
import crates.kernel.spec.Event
import crates.kernel.spec.Event.Kind
import crates.kernel.spec.Event.Record
import crates.kernel.spec.Gate
import crates.kernel.spec.Goal
import crates.kernel.spec.Idem
import crates.kernel.spec.KeepWarm
import crates.kernel.spec.Layout
import crates.kernel.spec.Ledger
import crates.kernel.spec.Locator
import crates.kernel.spec.Model
import crates.kernel.spec.NodeId
import crates.kernel.spec.Plan
import crates.kernel.spec.PolicyLimit
import crates.kernel.spec.Pursuit
import crates.kernel.spec.Reach
import crates.kernel.spec.Registry
import crates.kernel.spec.Release
import crates.kernel.spec.Repair
import crates.kernel.spec.Retries
import crates.kernel.spec.Schema
import crates.kernel.spec.Secret
import crates.kernel.spec.Share
import crates.kernel.spec.Spine
import crates.kernel.spec.Stall
import crates.kernel.spec.Taint
import crates.kernel.spec.Tool
import crates.kernel.spec.Version
import crates.kernel.spec.WriteDomain

/-! # kernel 的规格

`sprawling-kernel`（库名 `kernel`，目录 `crates/kernel`）是纯判定函数层：只吃入参吐 verdict，零内部 crate 依赖，不持有任何落盘物。城里的每一个决定都建在它上面：地址与保留子树、事件信封与链、事件种类与它们的窗类、错误码与它们的 carrier、门、计划、政策常量与磁盘布局。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。两个标签各被用过两次（8-73、8-74），表里分开写出各自的分部，引用它们的地方写分部的路径。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部的末尾，别处引作 `kernel D<n>`；D1 到 D16 沿用这份规格在 Markdown 时 §12 的条目号，§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：地址、段边界与保留子树（`spec/Address.lean`）、城内磁盘布局（`spec/Layout.lean`）、写域与写入限制（`spec/WriteDomain.lean`）、事件种类与窗类（`spec/Event/Kind.lean`）、可忽略性、时刻与 `seq`（`spec/Event.lean`）、链（`spec/Ledger.lean`）、错误码的 carrier、拼写与重试三态（`spec/Error.lean`）、账本版本的读法（`spec/ConstsExternal.lean`），以及几张判定表（`spec/Discard.lean`、`spec/Approval.lean`、`spec/Delegation.lean`、`spec/Backpressure.lean`、`spec/Version.lean`、`spec/Idem.lean`、`spec/Pursuit.lean`）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型、trybuild 反例、kani 与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

kernel 是纯判定函数层：只吃入参吐 verdict，零内部 crate 依赖，不持有任何落盘物。下表是其余模块都建在其上的九个基础模块；kernel 的全部模块、各自的形状与章节锚由 `architecture.toml` 的模块图给出，本规格不另列第二份。

| 模块 | 形状（ARCHITECTURE §7） | 一句话 |
|---|---|---|
| `error` | 2 值类型＋6 数据面 | AxError＝码加一份装箱的细节，经 ErrorDraft 构造，恢复语必填；AxCode 全集与装载期码；carrier 声明位 |
| `address` | 2 值类型 | 相对 city root 路径 newtype；WriteDomain 原语；reserved prefix 判定 |
| `locator` | 2 值类型 | `cas:`／`file:` 文法解析与呈现；fail-closed |
| `event` | 2 值类型 | EventKind 全集；in-window／record-only 二分；EventRecord 规范字节；EventRef 私有铸造 |
| `version` | 2 值类型 | 乐观并发：Version 单调值＋base 新鲜度判定 |
| `idem` | 2 值类型 | IdemKey 确定性派生（BLAKE3 XOF 16 字节＋版本字节） |
| `consts_external` | 6 数据面 | 外部事实常量 |
| `consts_policy` | 6 数据面 | 政策常量 |
| `ledger` | 3 端口 | 唯一写入口 trait；链语义（GENESIS_PREV／chain_hash）；conformance 套件 |

每一个模块由 `spec/` 下按它的 Rust 路径命名的分部规定（§8 的表）；模块的全集、形状与锚点在 `architecture.toml` 的模块图里，`cargo xtask gates specalign` 检查每一行的锚点落在本 crate 的一个分部上。
-/

/-! ## 2 验收标准

- 每模块单测过 workspace lints（非测试代码零 unwrap/expect/panic/索引切片/裸算术/as）。
- `EventKind` 与 `AxCode` 的 variant 名册与本规格 §8-4／§8-1 表逐 variant 一致，由 `xtask specalign` 断言。
- 每个 EventKind 恰属 in-window／record-only 之一；两分与条数见 §8-4 表。
- 每个 AxCode 恰有一个 carrier 声明；装载期白名单封闭（`AxCode::carrier` 里落到 `Carrier::Loadtime` 的那一臂）。
- golden EventRecord：规范字节入 insta 快照，跨平台逐字节稳定。
- proptest：Address 解析拒绝面、is_within 前缀性质、Locator 往返、IdemKey 重算不变。
- conformance 套件对任意 `impl Ledger` 可跑（由 citysim 内存 Ledger 第二实现证明）。

- 类型加固十项全部有型可指；`crates/kernel/tests/ui` 的 trybuild 反例全部编译失败。
- kani 三 harness 入库（`#[cfg(kani)]`）：kani 没有 Windows 宿主，每条性质配 proptest 镜像本地可跑，kani 本体入 CI Linux job。
- **三条全被 CI 证，理由不是成本而是信息**：`backpressure::verification::admit_is_total_and_monotone_in_depth`、`backpressure::verification::an_overflowing_sum_never_admits`、`secret::scan::verification::log2_q10_is_total`——任意 u64 上的全函数性、单调性、溢出时的 fail-closed 与定点 log2 的终止性，都是抽样到不了的地方。**不传全局 unwind**：三条的循环界要么没有循环，要么是常数（`log2_q10` 十次），CBMC 自己推得出来。
- **入库的 harness 恒被证，这是本节的收口条件**：写了不证的 harness 让「本仓有 kani 覆盖」这句话比事实强，故一条性质要么以求解器解得动的形状入库，要么按下一条记为已关闭的决定并从源码删除。
- **纪律在这里，名单不在这里**：每条不证的 harness 上方带一行 `// not-proved: <理由>`，`cargo xtask proof` 读这行来跳过它并打印理由，`cargo xtask proof --list` 打印今天将被证的名单。**源码标记是名单的唯一权威**，本规格因此只写「不证要写明理由」这条纪律，不再养第二份清单。**今天没有一条 harness 带这行标记**：机制留着，是给下一条写得出、暂时证不动的性质一个当场说明自己的地方。
- **命题的载体是堆集合或非线性算术的性质不交给 kani**，由同文件的 `#[test]` 或 proptest 守：CBMC 推不出 `Vec`、`BTreeSet`、`String` 内部循环的上界，全局 unwind 界也解不开它，而取几个具体值则退化成一条更贵的单测。按这条规则由测试守的性质：discard 的「Unplanned 恒 Deny」「Tainted 恒 Deny」（`discard/verdict.rs` 的 `#[test]`）、taint 的并集单调（proptest `join_output_contains_both_inputs`）、保留子树恒不在写域内（`#[test]` `reserved_target_is_outside_even_for_an_empty_domain`）、熵扫描的全函数性（proptest `scan_is_total_and_in_bounds`；它逐槽调 `log2_q10`，交给求解器是数千次符号非线性乘法）。`an_overflowing_sum_never_admits` 守 `depth + cost` 溢出时恒 Shed，对任意 u64 成立。
- three-part refusal 矩阵：`DOORS` 每道门每条 Deny 路径的 refusal 三段非空且 alternative 可执行。
- conformance feature 全量导出：Ledger＋Tool＋Model 三套件。

分部里的定理是模型对性质的证明：

- `spec/Event/Kind.lean`：`EventKind` 的每个种类恰落在一个窗类里（`EventKind.windowClass` 是穷尽的定义），入窗的恰是那九种（`the_in_window_kinds`）；名册完整、无重复（`EventKind.all_complete`、`EventKind.all_nodup`）。
- `spec/Error.lean`：每个码恰有一个 carrier（`AxCode.carrier` 是穷尽的定义），装载期白名单恰是那七个码（`the_loadtime_whitelist_is_closed`），门的码都由 `gate_denied` 携带；只要拼写是单射，每个码的拼写读回它自己（`parse_inverts_as_str`），单射去掉即有反例（`a_shared_spelling_loses_a_code`）；不论构造器按什么次序调用，「不是 `Yes` 却带等待」拼不出来（`no_order_of_calls_waits_without_retrying`）。
- `spec/Event.lean`：`ig` 不藏认得的种类，一行被跳过当且仅当它的种类未知且写方标了 `ig`（`a_line_is_skipped_exactly_when_unknown_and_marked`）；有时刻的行恰是那四种之一且时刻就是信封的 `t`，早于时刻版本的行没有时刻；`Seq::next` 恒加一。
- `spec/Ledger.lean`：被覆盖的行决定声索，单射时声索决定被覆盖的行，最后一行不被链证明，追加一行不动已欠的声索。
- `spec/ConstsExternal.lean`：打得开的账本版本恰是从首版本到本版本的那一段（`opens_exactly_from_first_to_current`）。
- `spec/Address.lean`：`is_within` 自反、传递、反对称；保留子树向下封闭，任一段受保护即保留，只判首段有反例（`judging_only_the_first_segment_lets_a_building_rules_through`）；读本楼不问规则，读不出的规则关上那栋楼。
- `spec/Layout.lean`：`of_ledger` 是 `ledger` 的逆，且只认账本目录；治理一个 scope 的文件落在保留子树里，居民自己写的文件落在明处。
- `spec/WriteDomain.lean`：收下的目标恒不在保留区里，空前缀集什么都不收，只写文档的写域永不收计划文件，写入限制只收窄不放宽，`Create` 不改动已有文件。
- `spec/Discard.lean`、`spec/Approval.lean`、`spec/Delegation.lean`、`spec/Backpressure.lean`、`spec/Version.lean`、`spec/Idem.lean`、`spec/Pursuit.lean`：各自判定表的性质，例如没有计划或带 taint 的删除恒被拒且规模不改答、居民不批准自己的问题、委派一层深、溢出的和恒被削且准入对深度单调、超前的 base 是陈旧的、一把键只领一次凭证、完成当且仅当没有就绪的活也没有在途的 run。

每个模型都带一个可实现的正常路径（`Seq::next` 从创世行走到 1、前缀为一栋楼的 `Everything` 收楼里的一个文件、`retriable_after` 造出可再试的 draft、深度零位派得出委派），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

1. **Locator 范围语义**：`L<a>-<b>` 行号 1 起、闭区间（编辑器与 sed 先例）；`B<a>-<b>` 字节偏移 0 起、闭区间（HTTP Range 先例）。两者均要求 `a<=b`，`L` 另要求 `a>=1`。
2. **Address 附加拒绝面**：设计点名拒绝绝对路径、`..`、空段、非 UTF-8；本规格在同一 fail-closed 精神下追加拒绝反斜杠、`.` 段、首尾 `/`、控制字符与 NUL、`:`（Windows 盘符与 NTFS ADS 两面一式拒）、以点或空白结尾的段（见 8-55）。放宽属「对扩展开放」，收紧后不再放回。
3. **git-oid 长度**：按 40 位十六进制小写受理（SHA-1 仓库）；其它长度 fail-closed 拒。SHA-256 仓库尚不受理：要受理它，`locator` 按仓库的对象格式加一条长度分支。
4. **`run` 字段恒在**：city 级事件（`city_initialized`、`log_truncated` 等）无所属 Run，取 `RunId::CITY`（nil UUID）哨兵值；uuid v7 的时间戳位保证真实 Run 恒不与 nil 撞。
5. **信封的 `who` 是字符串**：`EventDraft.who` 与 `EventRecord.who` 按字符串读写；载荷里的行动者用闭集 `kernel::event::Who { City, Person, Resident(Address) }`（`event/who.rs`，如 `RunStarted.dispatched_by`）。信封收紧为 `Who` 要连带账本旧行的读兼容，尚未决定。
6. **浮点拒绝在构造点**：Ledger 载荷禁浮点（确定性七条之 6）由 `Payload::new` 与其 `Deserialize` 双侧执行，serde_json 数字非 i64/u64 可表示即拒。
7. **存储写失败码**：装载期码 `E_STORAGE_FATAL` 承载 Ledger append 等存储写失败；与 `E_CAS_CORRUPT`（读到的对象不可信）分立，recovery 相反。
8. **深度上限在构造点**：读侧 `parse_line` 走 serde_json，递归上限 128 在第 128 层容器处拒绝，故一行最多 127 层，其中信封（`EventRecord` 这个对象）占 1 层；于是 `Payload::new` 与其 `Deserialize` 双侧拒绝嵌套超过 `PAYLOAD_DEPTH_MAX`＝126 层的载荷（载荷自身的对象算第 1 层），码 `E_INVALID_ARGS`，recovery 是把正文存进 CAS、载荷只带它的 locator。模型给的工具参数（`ToolCalled.args`）与工具结果（`ToolAnswer`）原样进 `data`，所以写得进却读不回的一行会让整条链重放失败；拒在写侧，读侧永远读得动自己写下的东西。浮点与深度在同一趟迭代遍历里判，只用一个 `(值, 层数)` 栈：不递归，所以敌意载荷耗不掉写方的栈；一个载荷只分配这一次，落选的是两趟分开的遍历（深度一趟、浮点一趟递归），它每层、每个节点各分配一个 `Vec`，在 dev 构建上对同一份两百来个节点的载荷交错计时，慢三倍多（每次约 58 µs 对 17 µs）。
9. **没有写方的 kind 不定型**：`credential_lent`、`backpressure_shed`、`digest_invalidated` 在 `EventKind` 里有名字，但本树没有任何写方。结构体要以写方的字节为准（record 模块规则 1），没有写方就没有可对齐的字节，故它们留在 `record` 之外；哪天出现写方，它的第一版就经 `Payload::of` 写，结构体随之落在 `record` 下。

模型自己的假设写在各分部的定理假设里，不写成公理：拼写函数（`spec/Error.lean`）、受保护的段名与 ASCII 折叠（`spec/Address.lean`）、布局的目录名与文件名（`spec/Layout.lean`）、`EVENT_LOG_V` 与首版本（`spec/ConstsExternal.lean`）、`u64` 的上界（`spec/Event.lean`、`spec/Backpressure.lean`）都是参数，它们的值只住 Rust；摘要函数是单射这一条写在 `spec/Ledger.lean` 的定理假设里。
-/

/-! ## 4 现状分析

性能敏感点唯一：chain_hash 落在单写者关键路径（选 BLAKE3 的理由），除此之外全部远离热路径。
-/

/-! ## 5 权威信源

**改这些类型前先读 provider 官方文档**（链接也写在 `crates/gateway/src/anthropic.rs` 与 `crates/gateway/src/openai.rs` 的模块注释里，以便下一位先看权威再动手）：

| 主题 | 出处 |
|---|---|
| Messages API 请求与响应 | <https://platform.claude.com/docs/en/api/messages> |
| 思考块、`signature`、工具往返中的保留规则 | <https://platform.claude.com/docs/en/build-with-claude/thinking> |
| 思考强度取值 | <https://platform.claude.com/docs/en/build-with-claude/effort> |
| 什么会作废缓存断点 | <https://platform.claude.com/docs/en/build-with-claude/prompt-caching> |
| OpenAI Chat Completions 请求与响应 | <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/> |
| OpenAI `reasoning.effort` | <https://developers.openai.com/api/docs/guides/reasoning> |

Address；Ledger／EventRecord／IdemKey／BLAKE3；Locator；乐观并发；AxError／AxCode／carrier；reserved prefix；常量三源；类型加固十项；确定性七条；`docs/glossary.md`（词汇）；ARCHITECTURE.md §4（缝清单）、§9（七形状）、§10（确定性与硬化）；`architecture.toml`（kernel 的模块图）。
-/

/-! ## 6 命名统一

概念名一律英文原词：Ledger、EventRecord、EventDraft、EventRef、EventKind、in-window／record-only、Locator、CAS、B3Hash、GitOid、IdemKey、Address、reserved prefix、WriteDomain、AxError、AxCode、three-part refusal、carrier event、Seq、TimeMs、RunId、Version、conformance。
Rust variant 名取 UpperCamelCase，serde 呈现名恒为线格式拼写（EventKind 蛇形小写；AxCode `E_` 全大写）。`B3Hash` 住 `locator`（`b3-` 算法标签的文法之家），`event`／`ledger` 引用之，全库仅此一个哈希值类型。

Lean 里的名字与 Rust 的对应：与 kernel 枚举同名的每一个 `inductive`（`EventKind`、`WindowClass`、`AxCode`、`Carrier`、`Retry`、`LogVersion`、`ReadVerdict`、`WriteDomain`、`DomainVerdict`、`DocumentReason`、`WriteLimit`、`DiscardRequest`、`DiscardVerdict`、`DenyReason`、`Autonomy`、`Answerer`、`AnswerVerdict`、`DelegateKind`、`Depth`、`DelegationVerdict`、`Admission`、`ShedReason`、`VersionVerdict`、`PursuitState`、`PursuitVerdict`）逐构造子与 Rust 的变体同名，`cargo xtask gates specalign` 双向对账；函数名照 Rust 拼写（`is_within`、`is_reserved`、`may_read`、`of_ledger`、`readable_log_v`、`records_a_moment`、`check_base`、`claim`、`may_answer`、`admit`、`observe`、`decide`），`EventKind.windowClass` 对应 `EventKind::window_class`，`AxCode.carrier` 对应 `AxCode::carrier`。模型里另起的名字（`Spelled`、`LineClass`、`Draft`、`Step`、`Names`、`Files`、`permits`、`relative`）在 Rust 没有同名的类型，各自的文档写明它模型的是什么。
-/

/-! ## 7 模块边界

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

判定模块的使用边（同 crate 内；此后新增的模块及其边以 `architecture.toml` 的模块图为准）：

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
- 不实现任何端口——`Ledger` 的实现住 storage 与 citysim；kernel 只声明 trait 与链语义纯函数。
- 不认识文件系统、明文凭证与颜色——canonicalize、Vault、OKLCH 各归其效果面模块。
- 不持 Markdown 词法器——界面读文档时由客户端 `client/src/core/prose.ts` 分词，Rust 侧没有调用者；在 kernel 再放一份只会与客户端那份悄悄分叉。服务端真要分词的那一天，词法器随它的第一个调用者一起进来。
-/

/-! D4 定规：kernel 不拆 crate

**决定**：kernel 保持一个 crate；策略模块（`consts_policy`、`gate`、`config` 等）不另立 crate。

**理由**：拆分只在「改一个策略模块时，少重编一批下游 crate」时才有收益，而使用面让这批下游几乎为空。`error` 被每个依赖 kernel 的 crate 引用，`event` 与 `address`／`locator`／`consts_policy` 被其中大多数引用；改动最频繁的是 `event`、`gate`、`model`、`consts_policy`。把其中一个拆出去，新 crate 仍依赖 `error` 与 `event`，改它时仍要重编它的全部引用者：`consts_policy` 与 `gate` 的引用者都包括 runtime 与 sprawling，省下的 crate 大多在它们上游，而 runtime 与 sprawling 本来就要重编，关键路径没有变短；kernel 自身的重编时间也省不掉，因为被拆模块依赖的 `error`／`event` 仍在 kernel 里。

**被否**：①把策略常量与门拆成 `kernel-policy`——多一个 crate、多一份 SPEC 与 API 基线、多一条 `depmap` 边，换来的是关键路径之外几个 crate 的重编；②按 `error`／`event` 拆出底层 crate——它们被全部下游引用，改它们照样全量重编，拆了只是多一层。

**重开参数**：任一条成立即重开——①某个高频修改的 kernel 模块的引用者降到 kernel 依赖者的一半以下，且省下的 crate 里有 runtime 或 sprawling；②`cargo build --workspace --timings` 显示，改一个策略模块后被省下的那批 crate 占增量重编时间 10% 以上；③kernel 单 crate 的重编时间超过一次策略改动增量重编总时间的一半。
-/

/-! ## 8 接口先行

**本 crate 无 `#[non_exhaustive]`：每个枚举都是闭的，穷尽性交给编译器。** 给 `AxCode`、`EventKind`、`Effect`、`DialectKind`、`DelegateKind` 这类面标开放，下游就得写永远打不到的 `_ =>`，而新增一个变体本该在每个没处理它的读者那里成为编译错误。

ARCHITECTURE.md §3「nothing here is published」是这条判定成立的前提：工作区之外没有下游，`#[non_exhaustive]` 在这里买不到任何兼容性。**重开参数**：任一 kernel 类型发布到 crates.io 的那天——那天起工作区之外才有读者，兼容性才第一次值钱。

外部开放枚举（`serde_json::Value`、`std::io::ErrorKind`、`git2::Delta`、tungstenite 的帧类型）与 `EventKind` 这样的词汇表仍会逼出通配臂：前者不归我们关，后者穷举一遍就是 `EventKind` 的第二份拷贝。这两类各带 `#[expect(clippy::wildcard_enum_match_arm, reason = …)]`，理由写在当处；除此之外通配臂即红。

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/kernel/spec/Error.lean` |
| 8-2 | `crates/kernel/spec/Address.lean` |
| 8-29 | `crates/kernel/spec/Address.lean` |
| 8-28 | `crates/kernel/spec/Address.lean` |
| 8-73（受保护元数据） | `crates/kernel/spec/Address.lean` |
| 8-55 | `crates/kernel/spec/Address.lean` |
| 8-3 | `crates/kernel/spec/Locator.lean` |
| 8-84 | `crates/kernel/spec/Locator.lean` |
| 8-4 | `crates/kernel/spec/Event.lean` |
| 8-75 | `crates/kernel/spec/Event/Record.lean` |
| 8-82 | `crates/kernel/spec/Event/Record.lean` |
| 8-82-1 | `crates/kernel/spec/Event/Record.lean` |
| 8-82-2 | `crates/kernel/spec/Event/Record.lean` |
| 8-79 | `crates/kernel/spec/Event/Record.lean` |
| 8-81 | `crates/kernel/spec/Event/Record.lean` |
| 8-83 | `crates/kernel/spec/Event/Record.lean` |
| 8-5 | `crates/kernel/spec/Version.lean` |
| 8-6 | `crates/kernel/spec/Idem.lean` |
| 8-7 | `crates/kernel/spec/ConstsExternal.lean` |
| 8-8 | `crates/kernel/spec/ConstsPolicy.lean` |
| 8-10 | `crates/kernel/spec/Taint.lean` |
| 8-11 | `crates/kernel/spec/WriteDomain.lean` |
| 8-46 | `crates/kernel/spec/WriteDomain.lean` |
| 8-78 | `crates/kernel/spec/WriteDomain.lean` |
| 8-12 | `crates/kernel/spec/Budget.lean` |
| 8-13 | `crates/kernel/spec/Backpressure.lean` |
| 8-74（降级） | `crates/kernel/spec/Degradation.lean` |
| 8-14 | `crates/kernel/spec/Stall.lean` |
| 8-15 | `crates/kernel/spec/Goal.lean` |
| 8-16 | `crates/kernel/spec/Repair.lean` |
| 8-17 | `crates/kernel/spec/Delegation.lean` |
| 8-18 | `crates/kernel/spec/Registry.lean` |
| 8-19 | `crates/kernel/spec/Spine.lean` |
| 8-32 | `crates/kernel/spec/Share.lean` |
| 8-33 | `crates/kernel/spec/Plan.lean` |
| 8-48 | `crates/kernel/spec/NodeId.lean` |
| 8-34 | `crates/kernel/spec/Blockage.lean` |
| 8-35 | `crates/kernel/spec/Pursuit.lean` |
| 8-20 | `crates/kernel/spec/Completion.lean` |
| 8-21 | `crates/kernel/spec/Approval.lean` |
| 8-47 | `crates/kernel/spec/Approval.lean` |
| 8-22 | `crates/kernel/spec/Config.lean` |
| 8-23 | `crates/kernel/spec/Tool.lean` |
| 8-52 | `crates/kernel/spec/Tool.lean` |
| 8-24 | `crates/kernel/spec/Model.lean` |
| 8-53 | `crates/kernel/spec/Model.lean` |
| 8-40 | `crates/kernel/spec/Model.lean` |
| 8-77 | `crates/kernel/spec/Model.lean` |
| 8-80 | `crates/kernel/spec/Model.lean` |
| 8-25 | `crates/kernel/spec/Secret.lean` |
| 8-26 | `crates/kernel/spec/Discard.lean` |
| 8-27 | `crates/kernel/spec/Gate.lean` |
| 8-49 | `crates/kernel/spec/Gate.lean` |
| 8-30 | `crates/kernel/spec/Change.lean` |
| 8-45 | `crates/kernel/spec/Schema.lean` |
| 8-50 | `crates/kernel/spec/Reach.lean` |
| 8-54 | `crates/kernel/spec/Release.lean` |
| 8-56 | `crates/kernel/spec/Layout.lean` |
| 8-76 | `crates/kernel/spec/Layout.lean` |
| 8-72 | `crates/kernel/spec/Retries.lean` |
| 8-73（上限类政策值） | `crates/kernel/spec/PolicyLimit.lean` |
| 8-74（缓存保温） | `crates/kernel/spec/KeepWarm.lean` |
| 8-4（种类与窗类的表） | `crates/kernel/spec/Event/Kind.lean` |
| 8-9 | `crates/kernel/spec/Ledger.lean` |
| 8-51 | `crates/kernel/spec/Ledger.lean` |
-/

/-! ## 9 工作流程

写路径：调用方组 `EventDraft`（Payload 构造点已拒浮点）→ `Ledger::append`（适配器：定 seq/prev → `EventRecord::from_draft` → `canonical_line` → 落介质）→ 返回 `EventRef`。
读路径：适配器/replay 逐行 `parse_line` → 验 v/链/seq → `to_ref` 铸引用。
错误路径：一切构造与解析失败即 `AxError`（fail-closed），生产模块负责把它送到 carrier event。
-/

/-! D2 定规：回滚＝分支＋git 还原

这一条是人定的。

**决定**：回滚不设动词。回到过去的两条路都是既有的：对话走**分支**（`OpenSession { from }` 从另一条线的某一行开新的一段），文件走 **git 还原**（工具波前后的检查点提交在 `refs/sprawling/runs/` 之下，可读、可 revert）。`Command::Rollback`／`Command::Takeover` 两帧与 `rollback_applied`／`takeover_started` 两个事件词删除；「人接管一条在跑的线」由既有干预动词（Steer／Cancel）承担。

**理由**：两帧自上线起没有执行者（装配层以 `not_built` 作答），两个事件词没有生产者——一个拼得出、执行不了的动词是对客户端的假承诺，事件词则是账本文法里的一段空文法。补齐执行面要引入文件快照库或改写账本：账本 append-only 不动，文件快照库被两案共同拒绝。没有它们，「回到检查点」就是让城去做 git 已经会做的事，并为它编一段没有载体的历史。

**被否**：补执行面（地址预验一个函数、回滚全成或全拒、Takeover 恒以 Handoff 收尾）——它把承诺补实，代价却是文件快照库与第二份文件历史，与「git 检查点是文件的唯一还原载体」冲突。

**重开参数**：出现「检查点提交不足以还原」的实据——例如跨楼多工作树需要一次原子还原，或人要在页面上单键回到某条检查点。重开时先回答「还原的是文件还是对话」：两者各自的载体今天都在（git／分支），缺的只是入口，而不是机制。
-/

/-! ## 10 实现逻辑

0. **AxError 内部装箱**：`{ code, Box<ErrorDetail> }`，serde flatten 保持 wire 形与字段序不变。理由：AxError 走每一道缝的返回位，扁平结构超过 clippy `result_large_err` 的阈值；装箱后只剩码与一个指针，接口与序列化形态不变。
1. 全模块零 I/O、零时钟、零随机；BTreeMap/BTreeSet only（Payload 经 serde_json::Map 默认 BTreeMap 间接满足）。
2. hex 编解码手写（16 行内，查表小写），不引 hex crate——C12 精神：依赖面只进钉版清单所列。
3. `EventKind`/`AxCode` 的 serde 呈现名逐 variant `#[serde(rename = …)]`（AxCode）与 `#[serde(rename_all = "snake_case")]`（EventKind）；`as_str` 与 serde 用同一份拼写（单测对拍）。
4. `Payload` 校验先判深度、再拒浮点：深度检查逐层迭代，不占调用栈；浮点检查递归下降 serde_json::Value，`Number::is_i64 || is_u64` 之外即拒，数组与对象深入。次序是这条递归的界：它只走深度检查已放行的至多 `PAYLOAD_DEPTH_MAX` 层，一个程序拼出的深嵌套值因此在深度处被拒，不会先把写方的栈耗尽。读侧 parse_line 的深度由 serde_json 的递归上限封住，对浮点恒拒。
5. `canonical_line` 用 `serde_json::to_vec`；`addr`/`ig` 的省略由 `skip_serializing_if` 表达；无 pretty、无空格。

### 三个设计（crate 级）

**A（选中）：规范字节住 kernel**——`EventRecord::canonical_line` 是全库唯一字节产地，jsonl／citysim 内存 Ledger／replay 三个消费者共用；conformance 断言 4 因此可写。杠杆：V8「三平台字节一致」收敛为一个函数的性质；换适配器不换字节。
**B（落选）：各适配器自产字节**（kernel 只给结构体，序列化归落盘方）——貌似「端口薄」，实则把规范散进每个适配器：内存 Ledger 与 jsonl 各持一份 serde 配置，漂移即 A19/A15 失真，而 conformance 只能对拍两实现、无法指认哪份是规范。落选理由：链对原始字节计算，字节即语义，语义必须一处。翻案条件：出现「同一记录合法多形」的需求（现设计明拒此需求）。

**第二对（error 侧）**：carrier 声明在 `AxCode::carrier()` 穷尽 match（选中）vs 分立静态表 `[(AxCode, Carrier); N]`。选中方案让「新增码忘配 carrier」成为编译错误（非穷尽 match 不过编译）；静态表则要靠测试数分支。落选表的唯一优势是 specalign 好解析——但 specalign 对齐的是 SPEC 表与 enum，match 臂同样可数。

**第三对（gate 侧）**：各门分立函数（选中）vs 单一 `gate::check(ActionEnvelope) -> GateOutcome` 总入口。总入口看似接口更窄，实则要造一个能同时表达五种异质入参的胖信封（写目标、出网目标、预算梯、审批决定、删除请求的交集形状），每门只读其中一角——胖信封即接口谎言，且无法逐门 kani（状态空间相乘）。五函数共享 GateOutcome 与 refusal 塑形纪律，组合在调用方（效果层按 Effect 字段选门）。落选的总入口若日后出现（如 wire 面需单帧过门），作为薄路由层另立，不回收五函数。

### 模型体验

零字节：kernel 本身不进任何 Run 的 prefix。它对模型的可见面只经两物间接达成——AxError 的 three-part refusal（错误即教学，边界反馈优于开头说教）与 in-window 事件的载荷字节（由上层模块产生）。本 crate 的任何变更不影响 prefix 缓存。
-/

/-! ## 11 边界枚举

空 Payload（合法，`{}`）；`ig:true` 且未知 kind（读侧放行跳过——replay 章）；`Seq::MAX.next()`（E_INVALID_ARGS，实践不可达但算术必 checked）；`Range` 端点相等（合法，单行/单字节）；`B0-0`（首字节）；`L1-1`（首行）；地址单段（合法）；`RESERVED_PREFIX` 恰为全路径（is_reserved 真）；`".sprawlingx/a"`（首段非 `.sprawling`，不 reserved——段边界判定）；Locator 尾随空白（拒）；hex 奇数长（拒）；`E_...` 码字符串反序列化未知码（serde 报错→读侧 fail-closed）。
-/

/-! ## 12 错误处理

本 crate 的码与它们的 carrier 在 `spec/Error.lean`（§8-1）；下面逐码回答「能否定义掉」，装载期白名单里每一码进表的理由也在这里。

已激活的码：

- `E_INVALID_ARGS`（address/payload/seq 构造拒）：不可定义掉——解析面即 Taint 边界，输入天然不可信；类型把「构造后非法」定义掉了，「构造时非法」必须留码。
- `E_LOCATOR_INVALID`：同上；且与宽松接受严格互斥（fail-closed 是策略）。
- `E_VERSION_CONFLICT`（verdict 映射在 `runtime::tools::edit`）：不可定义掉——乐观并发的存在理由就是冲突可发生。
- `E_LOG_VERSION_UNSUPPORTED`／`E_CAS_CORRUPT`：住装载期白名单，产生地在 memory/runtime（见各自 SPEC）。
- `E_STORAGE_FATAL`（存储写失败，装载期）：不可定义掉——磁盘满与介质 Io 失败在设计边界外；宁停不脏要求它直达进程级 fatal，不得伪装成可重试。S2 期初增设；storage 的 Io 映射已改正（storage D7）。
- `E_LEDGER_HELD`（另一个进程持着这座城的账本，装载期）：不可定义掉——两个进程打开同一座城，是人的两个普通动作（双击两次、两个终端各开一次）。它只能住装载期白名单：被拒的一方恰恰是写不了账本的那一方，给它一个 carrier，就等于让第二个写者把「我被拒了」写进别人的账本。能定义掉的那部分（被拒的一方先写了东西）已由 storage 的写者锁先于一切读写定义掉（`crates/storage/Spec.lean` §8-1）。它也不能借 `E_BUSY`：那一码的 carrier 是 `tool_result`，而一个码只有一个 carrier。
- `E_HISTORY_UNPROVEN`（服务中的城还在证明它开城时的历史，装载期）：不可定义掉——城从快照起步，快照之前的历史由后台证明走一遍（`crates/storage/Spec.lean` §8-30、sprawling-SPEC 8-122），而开城不等它。它只能住装载期白名单：证明完成之前，写者拒绝每一次追加，被拒的一方此刻恰恰写不了账本。它不能借 `E_LEDGER_HELD`：那一码的 recovery 是停下另一个进程，这一码的 recovery 是等几秒再发一次（「the city is still proving the history it opened from; send it again once the log says the history is proved」）。它也不能借 `E_BUSY`：那一码点名一条正在工作的 run，这里没有 run。

其余的码（逐码答「能否定义掉」）：

- `E_OUTSIDE_WRITE_DOMAIN`：不可——写目标是运行期输入，类型只能封构造后非法，封不住越域目标。
- `E_GATE_DENIED`：不可——Undoable 门与 Egress 主机门用它；其余门各有专码。
- `E_TAINTED_ACTION`：不可——被 taint 拒掉的动作需要自述来路的码。生产者是 `gate::undoable`、`gate::discard` 与 `gate::command`，三处都对非空 taint 恒 Deny。
- `E_BUDGET_EXHAUSTED`：不可——耗尽是审批不是错误，但模型需要可机读的码知道自己停在哪。
- `E_LOOP_SUSPECTED`：不可——停滞是观测事实；定义掉它等于假定模型不会循环。
- `E_GOAL_CONFLICT`／`E_REPAIR_BUSY`：不可——同资源相斥与修复串行化是机制存在理由；Queued/Conflict 是合法结局，码只在回传面携信息。
- `E_DELEGATION_DEPTH`：不消解（明裁：边界反馈优于沉默缺席）。
- `E_APPROVAL_PENDING`／`E_APPROVAL_DENIED`：不可——一个设计问题停住提问的那个 run，而人可以答「不」；两者都是用户可达状态。`E_APPROVAL_PENDING` 有一个门的生产者：`gate::attach` 的 Ask（§8-27），它请求的是人的动作而不是 Approval Inbox 里的一条答案，所以不产生 `ApprovalItem`；`E_APPROVAL_DENIED` 仍只由人答题面对产生。
- `E_EVIDENCE_MISSING`：部分定义掉——无证据 Done 已不可构造（类型半）；构造时拒绝仍需此码（运行时半，A6 双守）。
- `E_PLAN_MISSING`：不可——没有计划的楼上设常设目标，pursuit 找不到一步可做就当场「完成」，人看到的是一句 `finished` 而什么也没发生。这一码只在计划缺席或为空时于设目标处拒绝：`Roadmap.md` 读不出来保留它自己的码（`E_STORAGE_FATAL`），表格不成形是 `E_INVALID_ARGS` 并列出坏行——那份计划是人写的，恢复动作不该请市长另写一份盖掉它。`subject` 恒为 `<楼地址>: <常设目标>`，客户端据此给出「让市长写计划」的预填表单（client-SPEC 4-35a），由人提交。人定的是拒绝加按钮，胜过「先让市长自动写计划」：后者替人派出一次有成本的 run，而人只是想设一个目标。
- `E_SECRET_EGRESS`／`E_DISCARD_IRREVERSIBLE`：不可——两门存在的理由即这两类越界可发生；类型已把「无 Restoration 的 Discard 值」定义掉，Unplanned 请求（exec 预判路）是剩余不可消部分。
- `E_CONFIG_INVALID`：不可——SecretRef 形状非法与明文入配置必须在反序列化即拒。
- `E_MODEL_UNCHOSEN`（这一类模型还没有人选定）：不可——城在没接供应方、没选模型时也要能开，所以「这一类没有模型」是人可达的状态。它不并进 `E_CONFIG_INVALID`：那一码还答「会话中途换了模型」「端点已不在」等情形，出路各不相同（去设置 对 开新对话），而客户端只能按码给出路。生产者只有 `gateway::router` 的 `EndpointBook::select`；账本此刻可写，所以 carrier 是 `tool_result`，不进装载期白名单。

未在本节列出的码，其「能否定义掉」写在生产它的模块所属 SPEC 章。

决定的条目与它们住的地方：

| 决定 | 标题 | 住处 |
|---|---|---|
| D1 | 定规：默认 YOLO，门只答放行或拦住 | `crates/kernel/spec/Gate.lean` |
| D2 | 定规：回滚＝分支＋git 还原 | 本文件 |
| D3 | 定规：上限类政策值的拒因句式从类型给出，调用方拼不出第二种拒因 | `crates/kernel/spec/PolicyLimit.lean` |
| D4 | 定规：kernel 不拆 crate | 本文件 |
| D5 | 定规：降级的线相对于设备，停止接新活的拒绝带着读数 | `crates/kernel/spec/Degradation.lean` |
| D6 | 定规：请求借用会话与工具表，断点是请求的注记 | `crates/kernel/spec/Model.lean` |
| D7 | 定规：上下文提醒第二道阈值的缺省与合法域只有一个家，可选覆盖走既有配置梯子 | `crates/kernel/spec/Config.lean` |
| D8 | 定规：受保护元数据名单只有 kernel::address 一个家 | `crates/kernel/spec/Address.lean` |
| D9 | harness 的汇报与回答按本城的词入账 | `crates/kernel/spec/Event/Record.lean` |
| D10 | 信封 `t` 记这一行等来的时刻，`EVENT_LOG_V` 因此为 2 | `crates/kernel/spec/Event.lean` |
| D11 | 一次调用的效果与呈现，在调用那一刻从登记读出、记进 `tool_called` | `crates/kernel/spec/Event/Record.lean` |
| D12 | 运行策略是四个值，一次以新形状入账 | `crates/kernel/spec/Model.lean` |
| D13 | 远程门的五种事件全是 record-only，载荷里不带钥匙 | `crates/kernel/spec/Event/Record.lean` |
| D14 | 进程死后的 run 冻成 `cancelled`，载荷记下原因，不加第四种结局 | `crates/kernel/spec/Event/Record.lean` |
| D15 | 一份文档的一次写是一个种类，修改提案的一生是三个种类，都是 record-only | `crates/kernel/spec/Event/Record.lean` |
| D16 | 摘要在二进制格式里写字节，在人读的格式里写十六进制 | `crates/kernel/spec/Locator.lean` |
| D18 | 成熟度是 kernel 的一个枚举常量，每一个读者从它渲染 | `crates/kernel/spec/Release.lean` |
-/

/-! ## 13 依赖选型

`serde`＋`serde_json`（规范字节与载荷）；`thiserror`（Display/Error derive）；`blake3`（唯一哈希）；`uuid`（v7 仅解析/格式化＋serde 特性，恒不启用生成特性——kernel 禁随机）；`secrecy`＋`zeroize`（Sealed）。版本由根 `Cargo.toml` 与 `Cargo.lock` 给出。dev：`proptest`、`insta`、`trybuild`。不引：hex、rand、chrono/time（时间是入参）、regex（C12：熵与形状判定手写定点算法）。

规格本身不加依赖：分部只 import 工具链的库与本 crate 的分部；kernel 是 ARCHITECTURE.md §3 `depmap` 块的根，它的分部不 import 任何别的 crate 的分部。
-/

/-! ## 14 硬编码声明

- `RESERVED_PREFIX = ".sprawling"`（冻结面）。
- `GENESIS_PREV = [0u8; 32]`（「创世行 prev＝64 个 0」）。
- `IDEM_DERIVE_V = 1` 与派生框架 `run(16B)||seq(8B LE)||action`（换框架＝升版本字节，旧键不撞新键）。
- Locator 文法字面（`cas:`、`file:`、`b3-`、`#L`/`#B`）：`spec/Locator.lean` §8-3 的文法节即权威。
- `SECRET_SHAPES` 条目（公开 provider 令牌前缀，随外界增补）：`sk-ant-`（Anthropic）、`sk-proj-`（OpenAI）、`ghp_`/`gho_`（GitHub）、`AKIA`（AWS AccessKeyId）、`glpat-`（GitLab）、`xoxb-`（Slack）、`AIza`（Google API key）、`sk-or-v1-`（OpenRouter）、`sk-ai-v1-`（zenmux）、`gsk_`（Groq）。字符集与长度按各 provider 公开文档；条目形状见 §8-7。
- **聚合型转发商的令牌体是纯小写十六进制，故它们必须有形状条目而不能依赖熵侦测器**。熵侦测器的 `mixed_alphabet` 要求同时出现大写、小写与数字，这一条件本身是对的（城自己的 blake3 十六进制与 uuid 均单一大小写，否则每一行账本都会亮），但它使 `sk-or-v1-` 与 `sk-ai-v1-` 这类 64 位小写十六进制令牌两道侦测器都不响——形状表是它们唯一的网。S2 模块头早已写明「全小写的密钥避开本侦测器」，本条是那句话的具体后果。

- `MATURITY = Maturity::PreAlpha`：这棵树切出的发布处在哪一级；tag 的中缀、`status` 的说法与文档里的字样都从它渲染，进 alpha 只改它（`spec/Release.lean` D18）。
- `AUTONOMY_DEFAULT = Autonomy::Owner`、`CLOCK_STAMP_DEFAULT = ClockStampGranularity::Minute`（直写，随类型落位）。
- 定点 log2 小数位数 10（熵判定内部事务）；`ENTROPY_SPAN_MIN_BYTES = 20`（熵侦测器最短跨度：主流 API key 最短约 20 字符；pub(crate)，改动随本规格）。
- `HEX_SPAN_MIN_BYTES = 32`、`HEX_ENTROPY_MIN_MILLIBITS = 3100`（hex 侦测器，pub(crate)，改动随本规格）。证据：以固定种子的 splitmix 生成每档长度各 1000 个随机小写 hex 样本，以产品的 `entropy_millibits_per_char` 读数（最小／均值／最大，millibit）：28 字符 2952／3553／3922；32 字符 3144／3610／3929；40 字符 3307／3691／3933；48 字符 3404／3751／3933；64 字符 3544／3819／3970。32 是常见密钥最短的 128 bit；3100 让 32 字符及以上的全部样本通过，又高于 8 个符号均匀出现的 3000（如 `0f1e2d3c` 重复），把有规律的 hex 挡在外面。

规格不抄这些值：每个模型把它用到的值当参数（§3），值的唯一的家是上面列出的 Rust 常量，`cargo xtask docnum` 检查文档里引用的数与代码一致。`EventKind.windowClass` 与 `AxCode.carrier` 两张表是例外：它们是枚举的表，由 `cargo xtask gates specalign` 逐臂与 kernel 编译出来的值对账。
-/

/-! ## 15 影响面

storage::jsonl／storage::cas／runtime::replay／runtime::fork／citysim 全部消费本 crate 的公开面；全部 kernel 决断模块建立在 error/event 之上。公开面变更须与本规格同一变更集（apisync 机器看守）。

改 `EventKind` 或 `AxCode` 的一个变体，同一个变更集改 `spec/Event/Kind.lean` 或 `spec/Error.lean` 里的那一臂（`specalign` 判）；改了与某个 `inductive` 同名的枚举，同一个变更集改那个 `inductive`；改了被模型化的判定（§2 列出的那几个），同一个变更集改模型与它的证明。
-/

/-! ## 16 测试与约束

- 单测（各模块文件内 `#[cfg(test)]`，测试模块头挂放宽 allow）：serde 拼写对拍（as_str×serde×表）；EventKind 计数与 in-window 计数（以 `ALL` 数）；carrier 全映射非重复覆盖 `AxCode::ALL`；构造子不变量（refusal 三段在场、failure 无 gate、`retry` 默认 `Retry::No`）；Payload 拒浮点（含嵌套）；Address/Locator 拒绝面正反例；Seq/Version checked 溢出；IdemKey 版本字节在场。
- proptest：`Address::parse` 往返与 `is_within` 自反/传递/反对称；`Locator` Display↔parse 往返；`IdemKey` 重算恒等＋近旁输入不等样例；`Payload` 任意整数树恒过、含浮点树恒拒。
- golden（insta）：创世行＋一条 `building_created` 的 `canonical_line` 字节。
- 读界（§8-2 `may_read`）：`address::tests` 的三类读者矩阵——本楼读本楼、他楼读非机密楼、楼外读机密楼，外加机密楼读自己与读他楼——逐格判出 `ReadVerdict`；规则闭包在目标落在读者本楼时被调用即失败；规则读不出判 `RulesUnreadable` 且原样带回那条 `AxError`。
- conformance：对一个最小内存实现自证可跑；citysim 实现二证。
- 约束：`cargo clippy --workspace --all-targets -- -D warnings` 零告警；无 `unsafe`；文件前三行 MPL 头。
- 各模块测试面（逐模块文件内 `#[cfg(test)]`，kani harness 另配 proptest 镜像）：taint 并集单调／map 保集；write_domain reserved 恒拒／夺回计数；budget `checked_add` 溢出；backpressure 单调；stall 尾部连续语义；goal/repair 重叠矩阵；delegation 静动双层；registry verify 拒非证据 kind；spine 表解析正反例；completion 空证据／错 kind 拒；approval 应答真值表十二行遍历＋自审拒；config 字段交集空断言；tool/model conformance 自证；secret 双语料＋熵边界；discard 决策表全分支＋forecast 三臂正反；gate 门册遍历（`DOORS` 每行一条 `conformance::sample`，refusal 三段非空）＋taint 有真判决＋`claim` 认领一次。

形式化的义务由证明清偿：`lake build crates.kernel.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查，它们是行为比对，不是精化证明：

- 种类与窗类、码与 carrier：`cargo xtask gates specalign` 把 `spec/Event/Kind.lean` 与 `spec/Error.lean` 的两张表、以及每一个同名的 `inductive`，与 kernel 编译出来的枚举逐项对账；`event::kind::tests` 与 `error::code` 的测试检查计数、拼写的往返与单射。
- 地址与保留子树：`address::tests`（判例表 `tools/fixtures/address.jsonl`、`is_within` 的 proptest、任一段与 Windows 别名的保留判例、三类读者的读界矩阵）。
- 布局：`layout::tests`（保留子树在每个写域之外、`of_ledger` 的往返与拒绝）。
- 写域：`write_domain` 的测试（`reserved_target_is_outside_even_for_an_empty_domain`、`Documents` 的两种 `NotWritable`）与 `gate::tests` 里 `Create` 的拒与 `Full` 的放。
- 可忽略性与版本：`storage::jsonl` 的 `an_ignorable_line_from_a_newer_vocabulary_is_kept_and_chained`、`consts_external` 的 `readable_log_v` 测试、`event::moment::tests`。
- 判定表：`discard/verdict.rs` 的测试、`approval::tests` 的十二行真值表、`delegation` 与 `pursuit` 的测试、kani 的 `backpressure::verification` 两条与它们的 proptest 镜像、`idem` 与 `version` 的测试、`ToolBench` 的 `dedup_runs_before_the_side_effect`。
- 链：`Ledger` 的 conformance 套件（§8-9 的六条断言），由 `storage::JsonlLedger` 与 citysim 的内存 Ledger 各跑一次。

没有 Lean 模型、由 Rust 守住的：Payload 的浮点与深度、规范字节与 golden、Locator 文法、密钥扫描与熵、计划树与份额守恒、门的拒词与 `DOORS` 矩阵、降级读数、保温续期、模型端口的三扇门。它们的分部只有节注释，要求由类型、trybuild 反例（`crates/kernel/tests/ui`）、kani 与 `cargo nextest run -p sprawling-kernel` 的各模块测试守住；把其中一条写成定理，是下一次改它时的事。
-/

/-! ## 17 文档关系

- `pub trait` 只能声明在 seam 清单文件里（ARCHITECTURE §4；kernel 的是 `ledger.rs`、`tool.rs`、`model.rs`），由 `xtask depmap` 检查；所以这些模块拆成目录时，trait 留在原路径。
- `spec/Event/Kind.lean` 与 `spec/Error.lean` 的两张表是 `xtask specalign` 的数据面：改 enum 必同集改表（§15）。
- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 kernel 各行的 `spec` 锚点一起重看。
- `architecture.toml` 的模块图：kernel 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- xtask 的规格 §8-43（门读 Lean 的受限形状）：`specalign` 读本规格的哪几种形状；它改了，§8-1 与 §8-4 的两张表与同名的 `inductive` 一起重看。
- `tools/adversary/Spec.lean` 第 5 节：检验器从门外判账本的链与错误码，不 import 本规格；`spec/Ledger.lean` 的单射假设在那里被记为一条环境假设。
- 引本规格的其他规格与 rustdoc 写 `crates/kernel/Spec.lean §8-n` 或 `kernel D<n>`；一节换了分部，它的标签不变，引用不必改。kernel 的模块文档指向规定它的分部。
-/
