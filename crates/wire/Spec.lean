-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.wire.spec.Aggregate
import crates.wire.spec.Answer
import crates.wire.spec.Answer.Agents
import crates.wire.spec.Answer.Automation
import crates.wire.spec.Answer.Commits
import crates.wire.spec.Answer.Config
import crates.wire.spec.Answer.Devices
import crates.wire.spec.Answer.Doctor
import crates.wire.spec.Answer.Document
import crates.wire.spec.Answer.DocumentBytes
import crates.wire.spec.Answer.DocumentVersions
import crates.wire.spec.Answer.Endpoints
import crates.wire.spec.Answer.Find
import crates.wire.spec.Answer.Github
import crates.wire.spec.Answer.Harnesses
import crates.wire.spec.Answer.Hunks
import crates.wire.spec.Answer.Identity
import crates.wire.spec.Answer.KnownHosts
import crates.wire.spec.Answer.McpHealth
import crates.wire.spec.Answer.Prefix
import crates.wire.spec.Answer.Preview
import crates.wire.spec.Answer.Proposals
import crates.wire.spec.Answer.Range
import crates.wire.spec.Answer.Release
import crates.wire.spec.Answer.Sessions
import crates.wire.spec.Answer.Toolkits
import crates.wire.spec.Auth
import crates.wire.spec.CarriedName
import crates.wire.spec.Command
import crates.wire.spec.Command.Kind
import crates.wire.spec.Command.Step
import crates.wire.spec.Command.Tuning
import crates.wire.spec.Control
import crates.wire.spec.Frames
import crates.wire.spec.Frames.Ask
import crates.wire.spec.Frames.Monitor
import crates.wire.spec.Guide
import crates.wire.spec.NamedFrames
import crates.wire.spec.Preference
import crates.wire.spec.Privacy
import crates.wire.spec.Reading
import crates.wire.spec.Reception
import crates.wire.spec.Reception.Admission
import crates.wire.spec.Reception.Entry
import crates.wire.spec.Reception.Inbound
import crates.wire.spec.Reception.Pairing
import crates.wire.spec.Reply
import crates.wire.spec.Server
import crates.wire.spec.Server.Committed
import crates.wire.spec.Server.Listener
import crates.wire.spec.Server.Socket

/-! # wire 的规格

`sprawling-wire`（库名 `wire`，目录 `crates/wire`，依赖 kernel）是人与城之间的那条线：Command／Query／Event 三分的帧与它们的编码、版本与 schema 哈希的握手、绑定面、干预动词、多座城一个界面，以及 Autonomy 应答者与三队列在线上的形状。本规格先于代码存在；实现不多不少地遵守本文。模块：`frames`（含 `command`／`answer`／`carried_name`／`named_frames`）、`server`（含 `reception`／`assets`）、`control`、`auth`、`aggregate`、`preference`、`privacy`、`reading`。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。标签被模块图的旧锚点与别的规格里的引用锚住，所以不重排，也不补空号。「每个动词从哪里够得到」那一节保留它的标签 §19 与 §19-1 到 §19-3，住在 `spec/Command/Kind.lean`，`xtask wiring` 读它。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部里，别处引作 `wire D<n>`；D1 到 D12 沿用这份规格在 Markdown 时 §12 的条目号，§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：握手与 `WIRE_V` 的进位（`spec/Frames.lean`）、绑定面与丢帧之后的区间（`spec/Reception.lean`）、HTTP 门的配对判定（`spec/Reception/Admission.lean`）、套接字拼不出 `PutSecret`（`spec/Command.lean`）、reach 与 class 两张表的性质（`spec/Command/Kind.lean`）、带基线的保存（`spec/Command/Step.lean`）、干预欠不欠 Handoff（`spec/Control.lean`）、携带的名字只拒空与控制字符（`spec/CarriedName.lean`）、答复反映到哪一条（`spec/Frames/Ask.lean`）、事件按帧写出时每条记录按 seq 次序到达且不重复、没说到的都被 `Lagged` 点名（`spec/Server/Socket.lean`）。其余分部只有节注释：它们写的是线上的形状、取舍与被否的备选，由 Rust 的类型、trybuild 反例、`tests/wire_contract.rs` 的 golden 与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `frames` | Command／Query／Event 三分的类型与 JSON 编码；版本＋schema 哈希握手帧 |
| `server` | WebSocket 服务；静态资源；绑定面判定（默认只绑回环） |
| `auth` | 令牌的铸造、展示形与常数时间比较；每一面都要一把钥匙，没有钥匙即拒绝启动（D54） |
| `control` | 人的干预动词入口；自持鉴权与幂等（做不到则并入 `server`——ARCHITECTURE §6 已写明这条退路） |
| `aggregate` | 多 City 只读聚合：只转发 Query 与 Event，恒不转发 Command |
| `reception` | 一个请求进不进门（§8-94）、配对与会话的状态（§8-95）、一帧进来之后的判定：读不出的帧、会动作的门先问凭据（§8-37、§8-40） |
| `preference` | 客户端读的那几张偏好枚举，值集在这里生成 |
| `privacy` | 主机隐私页与城共享的名字闭集（§8-87）、`Query::Privacy` 的答复（§8-88）与 `Command::PrivacyOperation` 的载荷和结果（§8-89） |
| `reading` | 一次回合的读法回到服务端（§8-21） |

**本 crate 是进程外边界的唯一守卫**。它不实现任何业务判定：Command 的执行、Query 的求值、Event 的产生全在上游（runtime／storage／city），本 crate 只负责「让非法的帧在类型层或握手层就不存在」。
-/

/-! ## 2 验收标准

- **wire**：Command 与 Query 的变体数由计数断言钉住，数字只写在 `tests/wire_contract.rs` 里（§16；两张名表由 `named_frames!` 从变体表生成，故计数断言核的是「变体数没被无声改动」，不再是「两张手写表与枚举是否一致」——见 §8-38）；每个改状态 Command 携 `IdemKey`（类型强制，无可省字段）；`PutSecret` 的 `value: Sealed<String>` 不实现 `Serialize`——**「远程录凭证」这条帧编译不出来**，以 trybuild 反例钉死。
- **握手**：版本＋schema 哈希不配即断连并回 `E_WIRE_MISMATCH`（装载期码，无 carrier）；schema 哈希由 wire 类型集派生，改一个 variant 即变。golden 钉住当前哈希，改哈希必须与本规格 同集变更。
  当前 schema 与 shape 的 golden 由 `tests/wire_contract.rs` 定义，版本由 `frames::WIRE_V` 定义；帧表与查询表的当前内容见 §8 各章。
  `PutSecret` 无线格式——它经 `/enroll` 路由在进程内成形，见 §8-2 录入口。

**`Query::RunHistory { run, before, limit }` → `Answer::History`**：一个会话的历史按 run 取。`Query::History` 是城全局的最后一页，按它在客户端过滤，一个较早的会话就不在那一页里；`Query::RunView` 回答「这个 run 在不在、走到哪」，不回答「这个会话是什么」。

**答面复用 `HistoryAnswer` 而不新增一个。** 它已经带 `earlier` 游标，形状正是「往回翻」；再造一个只会让「一页历史长什么样」有两个答案。

**扫描量没有第二个上限。** `storage::index` 持了 run 索引之后，一次 `RunHistory` 只取属于这个 run 的 seq，**扫描量与 `limit` 同阶**，而 `limit` 已由 `HISTORY_MAX` 封顶。再设一个不防任何事的上限，只会让下一个读代码的人以为它还在防什么。

**`earlier` 的含义**：「从这条之前接着问」，`None` 即「到头了」；「答是空的而 `earlier` 是 `Some`」这一态**不存在**——服务端不需要把「我这一段没扫到」告诉客户端。

**建 run→seq 的索引**：不建索引，一次 `RunHistory` 要扫整本账，扫描量与账本长度同阶，而这是「打开一个较早的会话」这个动作的全部延迟。那份要随账本同步的派生状态已经存在：`storage::LedgerIndex` 常驻于 `Views` 并每次查询 `refresh`，run 表只是它多一个字段，搭同一趟刷新、同一份 cache、同一条「存疑即重建」的反射，不新增同步义务。至于「第二个权威」：索引回答的是「在哪」，从不回答「是什么」，它可弃且存疑即重建；账本仍是唯一权威。接面与内存代价见 `crates/storage/Spec.lean` §8-4。
- **server**：默认绑定回环；没有给钥匙时**拒绝启动**并回 `E_CONFIG_INVALID`（不是启动后再拒连——这是绑定面判定，不是请求面判定）。**每一面都有凭据是一条不变量，不是一句注释**：绑定判定把钥匙装进 `BindFace` 的两个臂，壳只持有这个面，于是「回环而什么都不要」与「暴露着却不要求任何东西」都是类型上不存在的状态（§8-41）。每个请求先过入口判定（Host、Origin、`Sec-Fetch-Site`，§8-94），跨站的 Origin 与不在名单的 Host 在升级之前就回 403。
- **auth**：令牌比较恒为常数时间（不早退）；比较函数以「逐字节差异位置不影响耗时」的性质测试看守。地基是 `server::constant_time_eq` 与 `decide_handshake`；`auth` 模块接令牌的生成、展示与持久化。
- **aggregate**：**类型化保证**——聚合上游连接的发送面在类型上只接受 `Query`，没有一个能塞进 `Command` 的方法（不是运行时 `if`，是类型上不存在该入口）；以 trybuild 反例钉死。

分部里的定理是模型对性质的证明：

- `spec/Frames.lean`：哈希对名字表单射、名字不变而改形的两份构建 `WIRE_V` 不同时，语法不同的两份构建握不上手（`distinct_grammars_never_handshake`）；不进位有反例（`a_reshaped_grammar_meets_the_old_one_without_a_bump`）；两次推送之间 `WIRE_V` 至多进一位，有改形就进（`numbered_moves_at_most_once`、`a_reshaping_commit_is_numbered_past_the_push`、`no_reshaping_keeps_the_number`）。
- `spec/Reception.lean`：`decide_bind` 恰在没有钥匙时拒绝（`decideBind_refuses_exactly_without_a_key`），给出的面索要的恰是给它的那把（`a_served_face_demands_the_given_key`）；`Lagged` 的区间恰好补上断口（`a_lagged_range_fills_the_gap`、`consecutive_records_owe_nothing`），还没发过记录或还没被欢迎的会话不欠区间。
- `spec/Reception/Admission.lean`：空手的来者不论面在回环还是暴露都不能转写、录凭证或落文件（`no_door_acts_unpaired`）；`/acp` 从不拒绝（`the_acp_door_never_refuses`）。
- `spec/Reception/Entry.lean`：Host 不在名单的请求从不进门（`a_foreign_host_never_enters`），Origin 不在名单的、跨站的与预检除了取页面都不进门（`a_foreign_origin_never_enters`、`a_cross_site_fetch_never_enters`、`a_preflight_never_enters`），配对的路只进浏览器（`pairing_admits_only_a_browser`）。
- `spec/Reception/Pairing.lean`：任何一条猜测序列里，被判的猜测彼此隔一秒（`judged_guesses_are_a_second_apart`），每次都对着上一次换上的码（`each_code_is_judged_once`），只有猜中当时的码才配对（`only_the_current_code_pairs`）。
- `spec/Command.lean`：`WireCommand` 拼不出 `PutSecret`，从线上来的命令进城后也不是它（`a_socket_cannot_spell_put_secret`、`nothing_from_the_wire_enrols_a_secret`）。
- `spec/Command/Kind.lean`：没有一个 Command 属 `Read`（`no_command_is_a_read`）；class 为 `Act` 的动词 reach 都是 `client`（`what_a_device_may_do_a_person_may_draw`）；不由人点的动词都 `LocalOnly`（`a_verb_no_person_draws_stays_local`）；只有 `PutSecret` 是 `sealed`（`only_put_secret_is_sealed`）。
- `spec/Command/Step.lean`：拒绝之后盘上不动（`a_refused_save_leaves_the_file`），保留子树先判（`the_reserved_subtree_is_refused_first`），两个同基线的保存只落先到的那个（`two_saves_from_one_version_land_once`）。
- `spec/Control.lean`：欠 Handoff 的恰是 `Steer` 与 `Cancel`（`a_handoff_is_owed_exactly_by_steer_and_cancel`）。
- `spec/CarriedName.lean`：非空、无控制字符的名字原样携带，不看任何名单（`an_unknown_name_is_carried`）。
- `spec/Frames/Ask.lean`：`as_of` 取下一条待折叠的记录，创世之前问出的答复不含创世；取「最后折过的一条」分不开空视图与只折了创世的视图（`the_last_folded_reading_confuses_nothing_with_genesis`）。

每个模型都带一个可实现的正常路径（同一份构建握得上手、一个可实现的断口、一次落得下的保存、表里至少一个 `Act` 动词），所以这些保证不是从一个满足不了的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

- **本 crate 是 tokio 的异步消费者**：套接字服务跑在 tokio＋axum 上；gateway 与 endpoint 用 `reqwest::blocking`，不引 tokio（`crates/gateway/Spec.lean` §3／§13）。
- **没有第二条传输路径**：即使浏览器与服务在同一台机器上也是网络连接，故不存在「同进程内存通道」这条优惠。唯一例外是 `PutSecret`——它不是靠运行时判断走内存通道，而是**类型上不可序列化**，因此远程连接根本编不出这条帧。
- **编码是 JSON，且编码本身不进冻结面**。选 JSON 的理由是不对称：浏览器原生支持，且开发者能在网络面板直接读帧。
- **`control` 自持鉴权与幂等，独立成模块**。ARCHITECTURE §6 预留了「做不到则并入 server」的退路，这里不需要它：`control` 持有一条 `server` 不知道也不该知道的策略——**哪些 Command 是干预，以及一次干预必须留下什么**（「任何中断都以 Handoff 收尾，下一位拿得到完整现场」）。那是判定，不是转调。
- **令牌的整个生命周期住 `auth`**（铸造、展示形、摘要、常数时间比对），`server::decide_handshake` 调用它：握手是令牌的一个读者，不是它的第二个家。
- **Signal 不在 Command 面**：Signal 的投递与消费住 `collab::inbox`。
- **`Welcome.resume_from` 读自 `LedgerHead`，`Welcome.epoch` 是创世记录的链哈希**：`decide_frame` 的第四个参数是 `WelcomeFacts { city, head, epoch }`——城名、账本头与 epoch 合成一个值，因为 `decide_frame` 已占满 4 个参数。`LedgerHead` 是 `ServeConfig.head` 递进来的一个 `AtomicU64`：装配层以重建视图时读到的最后一条记录的 `seq` 起头，折叠线程在每条记录**广播之前**把头推到它的 `seq`，socket 在 hello 时读一次。头放在原子量里而不放在视图锁里，因为读者可能长时间持有视图，而 hello 跑在 tokio 任务上，读头只是一次 Acquire load。先推头、后广播，加上会话在 hello 之前已订阅事件流，保证 `resume_from` 之后的记录必在流上：头之前而在订阅之后广播的记录会同时出现在流上与补拉里，所以边界上只可能重复、不可能缺失。`epoch` 是 `kernel::ledger::chain_hash(创世行)`，装配层在重建视图时读一次，由 `ServeConfig.epoch` 递进来：同一份账本的 epoch 永不改变，换了账本（重新 init、换了城目录）epoch 必变，所以客户端见到与上次不同的 epoch 就丢弃 belief、按快照重建，而不是拿旧水位去新账本里补拉。

- **V0.0.9 的线上改形已定形、尚未落进类型**：D22 到 D34 写下了形状；`Command` 与 `Query` 的帧名表由 `specalign` 与 schema golden 逐项对账，所以新帧与新字段由实现它们的那一个变更集同时加进 Rust、`spec/Command/Kind.lean`、golden 与 `client/src/wire.ts`，届时删去本条。信号与派活发出即生效（roadmap TP3）若要新帧，形状由它的设计写成本规格的下一条决定。

模型自己的假设写在定理的假设里，不写成公理：哈希对名字表单射是 `distinct_grammars_never_handshake` 的前提（blake3 的抗碰撞给出它）；名字不变而改形必进位是同一条定理的前提（D1 给出它，`tests/wire_contract.rs` 的形状摘要提醒改的人）；`decide_lag` 的 `checked_add` 溢出一支在模型里不写，因为产出 `next` 的账本到不了那一步。
-/

/-! ## 4 现状分析

装配消费者是 `crates/sprawling`（`serve` 把处理器注入 `ServeConfig`）；客户端 `client/` 读的 `client/src/wire.ts` 由 `cargo xtask wire-ts` 从本 crate 的 schema 生成（§8-16）。
-/

/-! ## 5 权威信源

wire 面全节（Command 表、Query 表、编码与握手、绑定面三段）；聚合层硬约束「聚合层只转发 Query 与 Event，恒不转发 Command」；干预动词语义表；`Sealed<T>` 的不可序列化性质；`kernel::error` 的装载期六码白名单（`crates/kernel/Spec.lean` §8-1，封闭）。外部：axum（内含 tokio-tungstenite）与 tokio，版本钉在根 `Cargo.toml`。
-/

/-! ## 6 命名统一

Command／Query／Event（三分的原名，不译）；命令与查询的原名逐字取 `COMMAND_NAMES`／`QUERY_NAMES`，两张表由 `named_frames!` 从变体表生成（§8-38），本文不另列；control surface（不译）；配对令牌＝pairing token；握手＝handshake。

Lean 里的名字与 Rust 的对应（门比的是同一个拼写，tools/xtask/Spec.lean §8-43）：

- `Wire.Command.Kind.Command` 的构造子 ↔ `command::kind` 里 `enum Command` 的变体，逐字相同；`Reach` 的四个取值 ↔ §19-1 的 `client`／`push`／`handshake`／`sealed`；`VerbClass` ↔ `remote_access::door::VerbClass`；`Command.reach`、`Command.verbClass` ↔ §19-2 的两列。
- `Wire.Command.Carrying` ↔ `Command<Secret>`，`WireCommand` ↔ `Command<NoSecret>`（`NoSecret` 在模型里是 `Empty`），`fromWire` ↔ `impl From<WireCommand> for Command`。
- `Wire.Reception.BindFace`／`BindVerdict`／`decideBind` ↔ `BindFace`／`BindVerdict`／`decide_bind`；`Address` 是模型对 `SocketAddr` 只取「是不是回环」的那一位；`decideLag`／`Stream` ↔ `decide_lag`／`reception::Stream`。
- `Wire.Reception.Admission.Door`／`Pairing`／`Admission`／`decideAdmission` ↔ 同名的 Rust 类型与 `decide_admission`；`judge` ↔ `Keys::pairing`。
- `Wire.Reception.Entry.Arrival`／`Caller`／`Entry`／`Refusal`／`decideEntry` ↔ `Arrival`／`Caller`／`Entry`／`EntryRefusal`／`decide_entry`；`OriginSeen` 与 `FetchSite` 是模型对 Origin 与 `Sec-Fetch-Site` 只取「相对名单是哪一种」的那几位。
- `Wire.Reception.Pairing.guess`／`Guess` ↔ `BrowserDoor::guess`／`Guess`；码在模型里是一个抽象的值。
- `Wire.Control.Intervention`／`ControlVerdict`／`classify` ↔ `control` 的同名类型与函数。
- `Wire.Command.Step.Refusal` 的三个构造子 ↔ `E_OUTSIDE_WRITE_DOMAIN`、`E_VERSION_CONFLICT`、`E_INVALID_ARGS`；`putRange` ↔ 城对 `Command::PutRange` 的判定（`accounting::worker::commanding::saving`）。
- `Wire.CarriedName.parse` ↔ `carried_name!` 生成的构造点；`Wire.Frames.Ask.asOf` ↔ `Answered.as_of`。
-/

/-! ## 7 模块边界

```
                    ┌── wire（类型与编码；无 I/O，纯数据与纯函数）
server（tokio＋axum）┤
  ├ WS 端点         ├── auth（令牌判定；常数时间比较）
  ├ 静态资源         └── control（干预动词入口；鉴权与幂等）
  └ /enroll、/transcribe、/drop（HTTP）
aggregate ──▶ 上游 City 的 WS 连接（发送面类型上只收 Query）
```

**不做什么**：不执行 Command（只解码并交给装配层注入的处理器）；不求值 Query（同上）；不产 Event（Event 载荷即 `EventRecord`，产地在各效果模块）；不做业务鉴权之外的策略；不声明 `pub` trait（不在 ARCHITECTURE §3 缝清单内——**处理器以函数指针或具体类型注入，不是端口**）；不内置任何穿透或中继（明拒：「内置一种就是替用户做了一个安全决定」）。
-/

/-! ## 8 接口先行

客户端是 TypeScript 的 `client/`，它的 SPEC 是 `client/Spec.lean`。

三条不可动摇的形状约定，它们决定接口而非被接口决定：

1. **`Command` 是穷尽枚举，每个改状态臂携 `IdemKey`**——「双击两下不开两个 Run」由类型保证，不由服务端去重表保证（去重表是第二道，`kernel::idem::claim` 已有）。
2. **`PutSecret::value: Sealed<String>`**——`Sealed<T>` 无 `Serialize`（`crates/kernel/Spec.lean` §8-25），故含它的枚举也无法整体派生 `Serialize`。这迫使 `Command` 的序列化实现**手写并对该臂显式拒绝**，而不是让宏悄悄地把它序列化出去。手写点即唯一权威，`E_WIRE_MISMATCH` 在此产出。
3. **`aggregate` 的上游发送面签名只接受 `Query`**——不是 `fn send(&self, frame: Frame)` 再运行时判断，是 `fn query(&self, q: Query)` 且没有第二个发送方法。

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-0 | `crates/wire/spec/CarriedName.lean` |
| 8-1 | `crates/wire/spec/Frames.lean` |
| 8-2 | `crates/wire/spec/Server.lean` |
| 8-3 | `crates/wire/spec/Auth.lean` |
| 8-4 | `crates/wire/spec/Control.lean` |
| 8-5 | `crates/wire/spec/Aggregate.lean` |
| 8-6 | `crates/wire/Spec.lean（本文件）` |
| 8-7 | `crates/wire/spec/Command.lean` |
| 8-8 | `crates/wire/spec/Frames.lean` |
| 8-15 | `crates/wire/spec/Server.lean` |
| 8-16 | `crates/wire/spec/Frames.lean` |
| 8-17 | `crates/wire/spec/Answer/Commits.lean` |
| 8-18 | `crates/wire/spec/Answer/Commits.lean` |
| 8-18b | `crates/wire/spec/Command.lean` |
| 8-19 | `crates/wire/spec/Command/Step.lean` |
| 8-20 | `crates/wire/spec/Answer/Hunks.lean` |
| 8-21 | `crates/wire/spec/Reading.lean` |
| 8-22 | `crates/wire/spec/Server.lean` |
| 8-23 | `crates/wire/spec/Answer.lean` |
| 8-24 | `crates/wire/spec/Answer/Commits.lean` |
| 8-25 | `crates/wire/spec/Answer/Doctor.lean` |
| 8-26 | `crates/wire/spec/Command.lean` |
| 8-27 | `crates/wire/spec/Server.lean` |
| 8-28 | `crates/wire/spec/Answer/Endpoints.lean` |
| 8-29 | `crates/wire/spec/Command/Tuning.lean` |
| 8-32 | `crates/wire/spec/Frames.lean` |
| 8-33 | `crates/wire/spec/Answer/Doctor.lean` |
| 8-34 | `crates/wire/spec/Answer/McpHealth.lean` |
| 8-35 | `crates/wire/spec/Answer/Prefix.lean` |
| 8-35b | `crates/wire/spec/Answer/Toolkits.lean` |
| 8-36 | `crates/wire/spec/Answer/Release.lean` |
| 8-37 | `crates/wire/spec/Reception/Inbound.lean` |
| 8-38 | `crates/wire/spec/NamedFrames.lean` |
| 8-39 | `crates/wire/spec/Preference.lean` |
| 8-40 | `crates/wire/spec/Reception/Admission.lean` |
| 8-41 | `crates/wire/spec/Reception.lean` |
| 8-42 | `crates/wire/spec/Answer.lean` |
| 8-43 | `crates/wire/spec/Command/Step.lean` |
| 8-44 | `crates/wire/spec/Command.lean` |
| 8-45 | `crates/wire/spec/Command.lean` |
| 8-46 | `crates/wire/spec/Server/Listener.lean` |
| 8-47 | `crates/wire/spec/Server/Committed.lean` |
| 8-47b | `crates/wire/spec/Reading.lean` |
| 8-47c | `crates/wire/spec/Command.lean` |
| 8-47d | `crates/wire/spec/Frames/Ask.lean` |
| 8-47e | `crates/wire/spec/Answer/Config.lean` |
| 8-47f | `crates/wire/spec/Answer/Release.lean` |
| 8-47g | `crates/wire/spec/Frames/Monitor.lean` |
| 8-47h | `crates/wire/spec/Server/Socket.lean` |
| 8-47i | `crates/wire/spec/Frames/Monitor.lean` |
| 8-48 | `crates/wire/spec/Reading.lean` |
| 8-48b | `crates/wire/spec/Answer.lean` |
| 8-48c | `crates/wire/spec/Command.lean` |
| 8-48d | `crates/wire/spec/Frames.lean` |
| 8-48e | `crates/wire/spec/Answer.lean` |
| 8-49 | `crates/wire/spec/Server.lean` |
| 8-50 | `crates/wire/spec/Answer/Doctor.lean` |
| 8-51 | `crates/wire/spec/Answer/KnownHosts.lean` |
| 8-52 | `crates/wire/spec/Answer/Harnesses.lean` |
| 8-53 | `crates/wire/spec/Reading.lean` |
| 8-54 | `crates/wire/spec/Answer/Commits.lean` |
| 8-55 | `crates/wire/spec/Reading.lean` |
| 8-56 | `crates/wire/spec/Reading.lean` |
| 8-57 | `crates/wire/spec/Command.lean` |
| 8-59 | `crates/wire/spec/Answer/Identity.lean` |
| 8-60 | `crates/wire/spec/Command/Step.lean` |
| 8-61 | `crates/wire/spec/Command/Step.lean` |
| 8-62 | `crates/wire/spec/Answer/Automation.lean` |
| 8-63 | `crates/wire/spec/Answer.lean` |
| 8-64 | `crates/wire/spec/Frames/Monitor.lean` |
| 8-65 | `crates/wire/spec/Command/Kind.lean` |
| 8-66 | `crates/wire/spec/Command/Kind.lean` |
| 8-67 | `crates/wire/spec/Answer/Github.lean` |
| 8-68 | `crates/wire/spec/Guide.lean` |
| 8-69 | `crates/wire/spec/Answer/Document.lean` |
| 8-70 | `crates/wire/spec/Answer/Range.lean` |
| 8-71 | `crates/wire/spec/Answer/Sessions.lean` |
| 8-72 | `crates/wire/spec/Command/Step.lean` |
| 8-73 | `crates/wire/spec/Answer/Proposals.lean` |
| 8-74 | `crates/wire/spec/Answer/Preview.lean` |
| 8-75 | `crates/wire/spec/Answer/Preview.lean` |
| 8-76 | `crates/wire/spec/Reading.lean` |
| 8-77 | `crates/wire/spec/Answer/Config.lean` |
| 8-78 | `crates/wire/spec/Answer/Commits.lean` |
| 8-79 | `crates/wire/spec/Reading.lean` |
| 8-83 | `crates/wire/spec/Answer/DocumentVersions.lean` |
| 8-80 | `crates/wire/spec/Answer/DocumentBytes.lean` |
| 8-81 | `crates/wire/spec/Answer/DocumentBytes.lean` |
| 8-82 | `crates/wire/spec/Answer/Find.lean` |
| 8-84 | `crates/wire/spec/Preference.lean` |
| 8-85 | `crates/wire/spec/Answer/Endpoints.lean` |
| 8-86 | `crates/wire/spec/Answer/Config.lean` |
| 8-87 | `crates/wire/spec/Privacy.lean` |
| 8-88 | `crates/wire/spec/Privacy.lean` |
| 8-89 | `crates/wire/spec/Privacy.lean` |
| 8-90 | `crates/wire/spec/Answer/Agents.lean` |
| 8-91 | `crates/wire/spec/Answer/Devices.lean` |
| 8-92 | `crates/wire/spec/Answer/Endpoints.lean` |
| 8-93 | `crates/wire/spec/Server.lean` |
| 8-94 | `crates/wire/spec/Reception/Entry.lean` |
| 8-95 | `crates/wire/spec/Reception/Pairing.lean` |
| 19 | `crates/wire/spec/Command/Kind.lean` |
| 19-1 | `crates/wire/spec/Command/Kind.lean` |
| 19-2 | `crates/wire/spec/Command/Kind.lean` |
| 19-3 | `crates/wire/spec/Command/Kind.lean` |

### 8-6 crate 面的两项

**一、`server` feature**（默认开）：`server = ["dep:tokio", "dep:axum"]`。`accounting`、`xtask` 与 `fuzz` 要本 crate 的词汇而不要监听器，取 `default-features = false`；`just features` 编译一次关掉它的构建，`cargo clippy --all-features` 编译开着它的。

**被否**：把 wire 拆成另一个 crate。那要改 ARCHITECTURE §2 的拓扑，而 feature 边界已足以表达「词汇与监听器分开」这一件事。

**二、kernel 类型再导出**：本 crate 的公开签名上出现的 kernel 类型一律从 `lib.rs` 再导出——**发帧的边界 crate 欠对方一套读帧的词汇**（C-REEXPORT）。再导出集就是 `lib.rs` 的 `pub use kernel::…` 那几行；动它就是动公开面，与本节同一提交更新。
-/

/-! ## 9 工作流程

启动时 `decide_bind` 判绑定面（非回环无令牌即拒启动）→ 监听 → 一条连接先收 `Hello`，`decide_handshake` 判版本、schema 哈希与配对 → 回 `Welcome`（`resume_from`、`city`、`epoch`）→ 之后每帧经 `reception` 判定：`Command` 交装配层注入的 sink，`Query` 交视图求值并以 `Answered` 回，订阅到的记录以 `Event` 推送，落后的订阅者收到 `Lagged` 并经 `HistoryRange` 补拉（§8-41）。
-/

/-! ## 10 实现逻辑

各模块的实现规则写在它的 §8 小节里。

### 两个设计

**握手的失败处置：断连 vs 降级协商。** 取断连。理由是这条错配的真实来源早已写明——「浏览器可能缓存了旧前端而服务端已经升级」，而降级协商要求服务端同时维护两套 wire 语义，那是两个权威。断连＋提示刷新把一个协议问题还原成一个刷新动作。**被否**：版本协商（多版本共存）——它的成本在每次改 wire 时都要付，而收益只在「用户不肯刷新」这一个场景里兑现。

**携字节的通路：WebSocket 帧 vs 独立 HTTP 路由。** 取独立 HTTP 路由（`/enroll` 的凭证、`/transcribe` 的录音）。**被否**：在 WS 上自制分片协议——那是重新实现 HTTP 已经做好的事，且会让字节的传输失败与命令失败混在同一条通路上难以区分。

### 入窗的字节与它的代价

零字节，因为 wire 面是人与服务端之间的协议，不进入任何 prefix。间接影响有一条：`Steer` 经 control surface 送达后，落点是**结果信封**（追加在下一次工具调用结果末尾，前缀 `user`），那几个字节由 `runtime::pipeline` 计入，不由本 crate 计入。
-/

/-! ## 11 边界枚举

握手哈希不配／令牌错／绑定非回环无令牌／读不出的帧（§8-37）／客户端半关连接／聚合上游断线／Event 推送背压（`Lagged`，§8-41）。
-/

/-! ## 12 错误处理

本 crate 不新增错误码，它用的码与「为什么不能把它定义掉」：

- **`E_WIRE_MISMATCH`**：不可——它是装载期六码之一（封闭白名单），且它的存在理由就是「浏览器缓存旧前端」这一 WebUI 特有错配。类型无法定义掉跨版本的字节。握手之后解不出的帧同归此码、两侧的处置见 §8-37。
- **`E_CONFIG_INVALID`**：不可——没有钥匙的绑定必须在**启动时**拒绝，这是配置判定不是请求判定。
- **`E_PAIRING_REFUSED`**（kernel 的码）：配对码或开页码不对、用过或太早，§8-95。
- **没有 signal-unknown 码**：握手的 schema 哈希保证同一连接的两端共享同一份词汇，一个本版本不认的 Signal 种类只能来自更新的二进制写的 Ledger，而那已由版本方向门拒在外面（`crates/collab/Spec.lean` §8 的 `collab::inbox` 一条）。

失败之后什么保持不变是各接口的一部分：绑定判定在套接字存在之前拒绝（§8-2、§8-46）；握手失败即断连（§10「两个设计」），握手之后读不出的帧答一次带计数的拒绝而不关连接（§8-37）；一次保存或一次提案的决定被拒时盘上不动（§8-72、§8-73，`spec/Command/Step.lean`）；`Committed::new` 拼不出帧时那条记录不推，下一条到达时会话发出 `Lagged`（§8-47）。

决定的条目与它们住的地方：

| 决定 | 标题 | 分部 |
|---|---|---|
| D1 | `WIRE_V` 在两次推送之间最多进一位，进在第一个改形的提交 | `crates/wire/spec/Frames.lean` |
| D2 | 写者点名的四个类型不在 `server` feature 之后 | `crates/wire/spec/Reply.lean` |
| D3 | 本批上线的字段各读自一个权威，缺席即没有 | `crates/wire/spec/Reading.lean` |
| D4 | `Dispatch` 带一个 `policy`，而不是四个平铺的字段 | `crates/wire/spec/Command.lean` |
| D5 | 设置页上的卡片的改写在城里做，页面只交值 | `crates/wire/spec/Answer/Identity.lean` |
| D6 | 动词类是 §19-2 的一列，由中继的穷尽匹配实现、门机器对照 | `crates/wire/spec/Command/Kind.lean` |
| D7 | GitHub 导入是一条只读查询，由二进制跑 gh；指南进度是一个按城的文件，整份写 | `crates/wire/spec/Answer/Github.lean` |
| D8 | 文档答复带版本，三种「没有文本」各是一种答复，范围按版本读 | `crates/wire/spec/Answer/Document.lean` |
| D9 | 一个房间的 session 列表从视图折叠作答，不从按地址的索引读账本 | `crates/wire/spec/Answer/Sessions.lean` |
| D10 | 保存带基线版本与文本编辑，回执是账本行；提案按文档成批决定 | `crates/wire/spec/Command/Step.lean` |
| D11 | Markdown 在城里读，页面按版本与窗口问它的块 | `crates/wire/spec/Answer/Preview.lean` |
| D12 | 对话流按文字问块，不按版本，不随增量带块 | `crates/wire/spec/Answer/Preview.lean` |
| D13 | 外壳读的字段在本版之内增加，各自可缺，各读自账本上写下它的那一行 | `crates/wire/spec/Reading.lean` |
| D14 | 档位、玻璃与混合档透明度随偏好进城，三者都可缺 | `crates/wire/spec/Preference.lean` |
| D15 | 全城开着的提案是一问，答文档与提出时刻，不答正文 | `crates/wire/spec/Answer/Proposals.lean` |
| D16 | `Bound` 在绑定时读下监听器的地址，`local_addr` 只是读出它 | `crates/wire/spec/Server/Listener.lean` |
| D17 | 冻下的名字在城里读成类型，线上不带摘要 | `crates/wire/spec/Reading.lean` |
| D18 | 文件的字节经一个 `Query` 回答逐窗送到页面，不开第二扇 HTTP 门 | `crates/wire/spec/Answer/DocumentBytes.lean` |
| D19 | 找文件是城里的一次有界走树，不是页面一层一层地问 `Listing` | `crates/wire/spec/Answer/Find.lean` |
| D20 | 送页面的两条路由是一个公开函数，城的端口与远程监听各把它并进自己的路由表 | `crates/wire/spec/Server.lean` |
| D21 | session 的标签住在人的偏好文件里，按 `(city, room, began)` 存 | `crates/wire/spec/Preference.lean` |
| D22 | V0.0.9 的线上改形一次进位，由一个变更集落地 | `crates/wire/spec/Frames.lean` |
| D23 | harness 一行说三态：启动程序缺失、harness 没装或没登录、可用 | `crates/wire/spec/Answer/Harnesses.lean` |
| D24 | 更新检查同时问 npm 与 crates.io，按这份二进制的安装方式给出更新命令 | `crates/wire/spec/Answer/Release.lean` |
| D25 | 依赖项页逐行已够；城目录前的扫描上页面，是 `DoctorAnswer` 的一件 | `crates/wire/spec/Answer/Doctor.lean` |
| D26 | 沙箱臂按机制族命名，每个名字在三个平台上都有可填的臂 | `crates/wire/spec/Answer/Doctor.lean` |
| D27 | 一段 session 带出显示名、模型、思考强度、工作区与最后一条回复；改名与改运行策略各是一帧命令 | `crates/wire/spec/Answer/Sessions.lean` |
| D28 | `Call` 带出整数微秒的耗时；skill 与 MCP 的使用各是一个查询 | `crates/wire/spec/Reading.lean` |
| D29 | 配色的覆盖是人的偏好，经 `PutPreferences` 写进偏好文件的 `[ui]` | `crates/wire/spec/Preference.lean` |
| D32 | `InstallSkill` 是往书架上加 skill 的命令，来源是一个四臂的值 | `crates/wire/spec/Command.lean` |
| D33 | skill 与 MCP 的使用各从哪一行折出、按天怎么数、怎样导出 | `crates/wire/spec/Reading.lean` |
| D34 | 停在同步 `send` 上的 run 在 `RunSummary` 上多一个 `waiting`；`wait` 是工具参数 | `crates/wire/spec/Reading.lean` |
| D35 | `Used` 的两个缓存数在 provider 没报时缺席，页面写「未知」 | `crates/wire/spec/Reading.lean` |
| D45 | 视图广播按帧合并只合并刷写，不改帧的形状，也不按 session 重排 | `crates/wire/spec/Server/Socket.lean` |
| D46 | 采样节拍是 `Monitoring` 的第四个变体，按城记住 | `crates/wire/spec/Frames/Monitor.lean` |
| D47 | `Answer::Unavailable` 带上没看成的原因 | `crates/wire/spec/Server.lean` |
| D48 | 每个 shell 解释器的读数是一个查询，与 skill、MCP 的使用同一遍折叠 | `crates/wire/spec/Reading.lean` |
| D49 | 设置页靠读回的 tuning 整份重发一次挂接，Key 状态按账号 id 答 | `crates/wire/spec/Answer/Endpoints.lean` |
| D50 | 线上的值是主机读到的原样，页面原样送回作 expected | `crates/wire/spec/Privacy.lean` |
| D51 | 正文字号的下限是类型 `BodyPx`，没有上限 | `crates/wire/spec/Preference.lean` |
| D52 | 没有「全部恢复」的帧：页面为每个仍拥有的控制各发一次 Restore | `crates/wire/spec/Privacy.lean` |
| D53 | V0.0.11 的线上改形一次进位，由第一条车道落地 | `crates/wire/spec/Frames.lean` |
| D54 | 每个调用方都要非环境凭据，入口先判 Host 与 Origin | `crates/wire/spec/Reception/Entry.lean` |
-/

/-! ## 13 依赖选型

| 依赖 | 用途 | 依据与替代 |
|---|---|---|
| `tokio` | 异步运行时 | 替代（自写 reactor）重做一件现成的事 |
| `axum` | HTTP 静态资源、几条 HTTP 路由、WS 升级 | tokio 官方序列，且其 WS 支持内含 tokio-tungstenite，省掉一层版本对齐 |
| `tokio-tungstenite` | WS 协议 | 经 axum 传递依赖；**不直接依赖**，避免两处版本权威 |
| `serde`／`serde_json` | 帧编码 | 已在 workspace |
| `kernel` | AxError／EventRecord／IdemKey／Sealed／Address | 唯一上游 |
| `aws-lc-rs` | `/session` 验设备钥的 Ed25519 签名（只在 `server` feature 里） | 已在 workspace（`remote_access` 用它）；替代（自写或另引一个 Ed25519 库）是同一件事的第二个实现 |

**不引**：任何通用 RPC 框架（wire 是一组具名 variant，不是一个可扩展的服务定义）；任何 session 中间件（鉴权面是native key加会话令牌两件，都在 `reception` 里判）；任何穿透／中继库。**WebTransport／QUIC 同此**：重开条件写在 §8-41 末节（城真的在回环之外且实测有队头阻塞，两条都成立才重开），此前它不因「需要第二种协议」而回来。

规格本身只 import 工具链的库与本 crate 的分部：ARCHITECTURE.md §3 的 `depmap` 允许 wire 依赖 kernel，今天的模型不需要 kernel 的分部。
-/

/-! ## 14 硬编码声明

- **默认绑定地址恒为回环**——它不是配置的默认值那么软，而是「非回环需要额外条件才允许」的判定起点。
- **schema 哈希的派生框架**（哪些类型入哈希、以什么序）一旦定下即是冻结面：改框架＝旧客户端全部拒配。故派生框架与 `IDEM_DERIVE_V` 同规，携版本字节。
-/

/-! ## 15 影响面

- 改 `Command`／`Query`／帧：名字变了，schema golden 随之变；名字不变而形状变了，按 D1 进 `WIRE_V`；`client/src/wire.ts` 重新生成（`cargo xtask wire-ts`），`crates/sprawling` 的处理器穷尽匹配随之改，§19-2 的 reach 表增删一行（`xtask wiring`）。
- 改 `ServeConfig`：波及 `crates/sprawling` 的 `serve` 装配点。
-/

/-! ## 16 测试与约束

- 计数断言：`tests/wire_contract.rs` 逐名核两张名表与枚举变体数是否同步；**数字不写在这里**——一个被抄进本文的计数就是同一条规则的第二个家（同 §8-1 对 golden 的处置：本文只记哈希值本身，因为那是一个不可推导的输出而不是一条可重算的规则）。
- trybuild 两反例：远程 `PutSecret`（含 `Sealed` 的帧不可序列化）；`aggregate` 发 Command（发送面无该入口）。
- 常数时间比较的性质测试：差异位置不影响比较耗时。
- 握手 golden：schema 哈希入快照；改 wire 类型必须同时改快照与本规格。
- 绑定面判定的单元测试：回环／非回环×有钥匙／无钥匙四格，恰是两格「无钥匙」拒绝启动；并断言判定给出的面里的摘要就是给它的那一个。
- 入口判定的回归测试：`tests/entry.rs` 在进程内对路由表发请求，跨站的 Origin 与外来的 Host 在升级之前就得到 403。
- 约束：非测试代码遵守 C3 硬化全条；全库禁裸 spawn（确定性第 3 条）——**本 crate 的并发必须是结构化的，带取消令牌**，这是引入 tokio 后第一条要守住的线。

形式化的义务由证明清偿：`lake build crates.wire.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些检查守住，它们是行为比对，不是精化证明：

- 握手与 `WIRE_V`：`tests/wire_contract.rs` 的 schema 哈希 golden 与 `the_wire_shape_is_pinned_so_a_change_meets_the_version_rule`。
- 绑定面与丢帧区间：`tests/wire_contract.rs` 的 `the_binding_face_has_exactly_one_refusing_cell`；`reception::tests` 里 `decide_lag` 与 `Stream` 的测试。
- HTTP 门：`reception::admission` 旁的测试与 `tests/enrolment.rs`。
- 入口判定：`reception::entry` 旁的穷尽表测试 `every_entry_property_holds_over_every_input`，逐格对照 `spec/Reception/Entry.lean` 的五条性质；`tests/entry.rs` 的回归。
- 配对：`reception::pairing` 旁的 proptest `every_trace_judges_each_code_once_and_a_second_apart`，生成器覆盖 `spec/Reception/Pairing.lean` 的猜测序列（到达时刻、猜对或猜错、每步的熵）。
- 按帧写出：`server::socket::tests` 的 proptest `framing_tells_every_record_once_in_order`，生成器覆盖 `spec/Server/Socket.lean` 的到达序列与切帧方式。
- `PutSecret`：`tests/trybuild.rs` 的两个反例与 `put_secret_has_no_byte_form_in_either_direction`。
- reach 与 class：`cargo xtask gates wiring` 把 `Command.reach`、`Command.verbClass` 的臂与 `enum Command`、`run_command`、`client/src`、`command_class` 逐个动词对照；Lean 侧的 `def` 漏一个构造子即编不过。
- 带基线的保存：accounting 的 `worker::commanding::tests::saving`。
- 干预：`control` 旁的测试。
- 携带的名字：`carried_name` 的 `a_carried_name_rejects_empty_and_control_characters`。

没有 Lean 模型的分部，其要求由类型与 `cargo nextest run -p sprawling-wire --all-features` 的测试守住：信封与名表（`named_frames!` 与 `tests/wire_contract.rs` 的计数）、`wire_schema` 与 `client/src/wire.ts` 的生成（`cargo xtask wire-ts`）、各答面的形状、`aggregate` 只转发 Query（trybuild 反例）、常数时间比较、`Committed` 拼出的帧与 `ServerFrame::Event` 逐字节相等、读不出的帧不关连接。
-/

/-! ## 17 文档关系

- ARCHITECTURE 模块表的 wire 各行。
- `client/Spec.lean`：线上形状变了的那一侧。
- `crates/sprawling/Spec.lean`：`serve` 子命令的装配面。

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 wire 各行的 `spec` 锚点一起重看。ARCHITECTURE.md §7「The wire」与 §8 的「a new `Command` or `Query` frame」一行引 D1。
- `architecture.toml` 的模块图：wire 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- tools/xtask/Spec.lean §8-43、§8-45：`xtask wiring` 读 `spec/Command/Kind.lean` 里 `Command.reach` 与 `Command.verbClass` 的受限形状；改那两个 `def` 的写法就是改门读到的东西。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- kernel 的规格（`crates/kernel/Spec.lean`）：线上携带的 kernel 类型、事件种类、错误码与装载期白名单的权威；remote_access 的规格（`crates/remote_access/Spec.lean`）：`VerbClass` 与远程门的权限；documents 的规格（`crates/documents/Spec.lean`）：线上直接携带的 `Span`、`Window`、`Preview`、`Laid` 等的权威。
- `tools/adversary/src/Sprawling/Door.lean` 与 `Regression.lean`：从外面说这条线的检查器，线上改形时同一个变更集改它们（AGENTS.md）。
- 引本规格的其他规格与 rustdoc 写 `crates/wire/Spec.lean §8-n` 或 `wire D<n>`，本 crate 的源码引它自己的分部；一节换了分部，它的标签不变，引用不必改。
-/
