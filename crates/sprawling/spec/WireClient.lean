-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::wire_client

规定 `crates/sprawling/src/wire_client.rs` 与 `crates/sprawling/src/wire_client/`：CLI 经它对一座在服务的城说话的第二个 wire 客户端（`bin::wire_client`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-10 第二个 wire 客户端

**为什么存在**：ARCHITECTURE §8 写着「the wire is the whole API；一个第二客户端就照着它写」，而今天只有一个客户端——按仓库自己的依据（§4：一个适配器是假想缝，两个才成立），`wire::frames` 因此是一条假想缝。

```rust
// bin::wire_client（形状 4 adapter）
pub(crate) struct Listen { pub quiet: Duration, pub until: Until }  // 何时停止收听，两者同行
pub(crate) enum Until { Quiet, Event(kernel::EventKind), Run { under: Address, milestone: Milestone } }  // 调用方在等什么
pub(crate) enum Milestone { Started, Frozen }                         // 由 run_started / run_frozen 标记
pub(crate) struct Heard { pub frames: u32, pub refusals: u32, pub answers: u32, pub awaited: Awaited, pub run: Option<RunId> }
pub(crate) enum Awaited { Nothing, Arrived, Missing }
pub(crate) enum Spoken { Refused, Answered, Quiet, Unfinished }
pub(crate) fn call(at: &str, frame: &str, token: Option<&str>, listen: Listen) -> Result<Heard, Unheard>;
pub(crate) fn send(at: &str, outgoing: &wire::ClientFrame, token: Option<&str>, listen: Listen) -> Result<Heard, Unheard>;
pub(crate) fn enrol(at: &str, realm: &str, name: &str, value: &str) -> Result<String, AxError>;
pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)>;   // "realm/name"
```

- **握手在进程内算，不手抄**。`WIRE_V` 与 `schema_hash()` 直接取自 `wire`，故改一条命令名字时本客户端**不可能**落后。因此删掉了那个一次性的 Python 探针——它在工作区外复刻了 `schema_hash()` 与 `IdemKey::derive()`，那本身就是第二个权威。
- **一个查询恰好一个答复，收到就走**。发出的是 `Ask` 时，`call` 在收到第一帧 `Answered`（其 `AskOutcome` 是答复或拒绝）或 `Refusal` 时打印它并退出，之前推来的 `Event`／`Log`／`Delta` 照样逐行打印；城的答复在十几毫秒内到达，再等一整段安静窗口只是让进程白占两秒。安静窗口在这里只剩上限的作用：答复迟迟不来时，`call` 退 3——什么都没回来是 `Quiet`，只回来了别的帧是 `Unfinished`。
- **一条命令收到城安静为止，或收到调用方点名的那种事件为止**。发出的是 `Command` 时，“安静”是一段无帧的时长（`--quiet-ms`，默认 2000），而不是帧数：一条 Dispatch 会产生多少事件是城的事，客户端猜不到。调用方知道自己在等哪件事时，写 `--until <kind>`（`kind` 取 `EventKind` 的 snake_case 拼写，由 serde 读，不另立名表）：`call` 在打印第一条该种类的 `Event` 或一条 `Refusal`（被拒的命令不会再产生事件）后退出，不再白等一整段安静窗口。窗口此时仍是两帧之间的上限；窗口先到而点名的事件没来，`Heard.awaited` 为 `Missing`，`spoken()` 给 `Unfinished`，退出码是 3 而不是 0——在等的事没发生，读成成功就是把失败读成成功。查询只有一个答复，`--until` 对查询不起作用。
- **何时结束由 `wire_client::Ending` 一处决定**。`Ending::of` 按发出帧的种类与 `Until` 穷尽匹配，得 `Reply`（查询与握手）、`Quiet`、`Event(kind)`、`Run { under, milestone, run }` 四者之一；每一帧由 `Reply::of` 经 `wire::ServerFrame` 读一次，`Reply::Event { kind, run, addr }` 同时供 `--until` 比种类、供 dispatch 认出自己的 run。在等的帧到没到只记在 `Heard.awaited` 一处，`call` 与 `dispatch` 读的是同一个 `spoken()`。被否决的备选：把「收到答复就走」做成一个布尔参数——它会让 `call(…, false)` 这样的调用点说不出自己在等什么；以及让 dispatch 另记一份「里程碑到没到」——它与 `Awaited` 答的是同一个问题，两份记录迟早说两样话。
- **`sprawling dispatch <addr> <task> [--detach] [-m|--model <id>] [--at] [--token]`（`bin::main::dispatch`，形状 adapter）是一次派活的整条路**：进程内铸幂等键（`IdemKey::derive(RunId::CITY, Seq::FIRST, 16 字节 OS 熵)`，与控制台每行一把键同一个构造；键从命令行拿进来就等于让人或 agent 再抄一遍 wire 的键格式），模式取 `Mode::PlanGoal`、强度与目标留空、`session` 为 `None`（地址是楼时城向模型要房间名，是房间时续写那个会话）。它等的是 `Until::Run { under, milestone }`，`Ending::of` 把它变成 `Ending::Run { under, milestone, run }`：`under` 是派去的地址，第一条 `addr` 等于它或在它之下的 `run_started` 认定这次的 run，`milestone = Milestone::Frozen` 时等到**那个** run 的 `run_frozen`（别的 run 冻结不算），`milestone = Milestone::Started`（`--detach`）时收到 `run_started` 就走；两者收到 `Refusal` 都立即结束，安静窗口（默认 120000 ms：一次模型调用返回前不出帧，派到楼时城先调一次模型给房间起名，run 才开始；每来一帧都重新计时）只作上限，窗口在里程碑之前关上时 `Heard.awaited` 为 `Missing`，城说过话则 `spoken()` 给 `Spoken::Unfinished`，一帧没回则给 `Quiet`，都退 3 而不是 0。`--detach` 的 stdout 只有 run id 一行，逐帧 JSONL 改走 stderr，于是 `id=$(sprawling dispatch --detach …)` 可直接用；不带 `--detach` 时逐帧 JSONL 走 stdout，与 `call` 一致。退出码表与 `call` 同一张，映射只写在 `main::calling` 的 `exit_of`（`Spoken` → `Exit`）与 `tell_unheard`（`Unheard` → `Exit`）两处，两个动词都调它们：0 答复（里程碑已到）、1 拒绝、2 命令行、3 安静或未到里程碑、4 `--at` 没有城。被否决的备选：让 `dispatch` 拼一段 JSON 再交给 `call`——`call --until run_frozen` 等的是任何一个 run 的冻结，认不出这一次派出的那个 run，而不带 `--until` 只能等安静；一次 run 的长短是模型的事，安静窗口要么截断它要么让每次派活白等。
- **`-m|--model <id>` 点名这一次的模型**：它原样进 `WireCommand::Dispatch.model`（`crates/wire/Spec.lean` §8-48），不带时为 `None`：房间自己那层已冻结的模型仍有 tag 登记时取那个 tag，否则取 `main` tag 的模型——冻结的模型已无 tag 登记时，`main` 的模型由 `choose_shape` 以换模型拒掉（§8-79），拒绝语点名动过的那一项，恢复语给出人现在做得到的出路；在这里以「无 tag 登记」拒，两者都说不出。继任交接、唤醒敲门和不带 `-m` 的后续派活都带 `None`，若 `None` 一律取 `main`，在用 `-m` 开的房间里它们会被 `choose_shape` 以换模型拒掉，接力就断在半路。装配在 `agree_to_work` 里按这个 id 在 `ModelBook::choices()` 找到登记它的 tag，同一个 id 登记在几个 tag 下时取 `ModelTag` 次序里最前的那个，`main` 最前——`choices()` 按 `BTreeMap<ModelTag, _>` 的键序给出，所以这个取法不随登记先后变，再经 `ModelBook::select` 取端点与登记行，所以保密楼只用回环端点的检查、端点是否仍挂着、订阅凭证续期都与 `main` 同一条路；找不到时以 `E_CONFIG_INVALID` 拒，恢复语是「在设置页把它登记到一个 tag 下再派」，拒在写任何东西之前。选中的 id 由 `choose_shape` 冻进房间自己那层 `CONFIG.toml`，与 tag 解析出的模型同一扇门，所以一个已冻结的会话照旧拒绝换模型。被否决的备选：用 `SelectModel` 先改 tag 再派——那是整座城的配置，会在同时跑着的别人的 run 底下换模型。
- **输出是 JSONL，一行一帧**。发明一种人看的排版就是为 wire 里的每一个类型再写一遍它长什么样，而那份渲染一定会漂。
- **退出码带信息**：收到过 `Refusal` 退 1；命令之后一帧都没回来（`Quiet`），或回来了帧而在等的那一帧没来（`Unfinished`），退 3；否则退 0。`Quiet` 与 `Unfinished` 分成两支，是因为前者连城是否收到这一帧都无从断言，后者城在说话、只是事情还没做到。一个驱动它的 agent 不应当为了知道「成不成」去解析 JSON。
- **`enrol` 只从 stdin 读，恒不从 argv 读**。argv 进进程表、进 shell 历史、进父进程的日志；这比浏览器路径更好的地方就在这里，因为页面那条路要先把明文拿进一个标签页的内存。**输出只有引用**，恒不回显值。
- **依赖不新增包**：`tokio-tungstenite` 正是 axum 的 `ws` 特性已经携带的那一份，直接依赖它在 `Cargo.lock` 里**增加零个包**（实测 496 → 496）；换一个别的 WebSocket 库就是把同一个协议的两份实现放进同一个二进制。不开 TLS：控制面走 `ws://`，而一座要经 TLS 到达的城是一座前面站着终结器的城。
- **未做且已知**：`/enroll` 仍在工人取走凭据之前就答 201（详 `crates/wire/Spec.lean` §8）。`enrol` 因此报的是「已受理」而不是「已入库」，这句话写在输出里而不是留给人去撞。

**本章测试**：`split_reference` 对 `realm/name`、缺斜杠、空段、多斜杠四类输入给出正确答案；握手帧的 `wire_v` 与 `schema` 逐字节等于 `wire` 自己的值（这条断言就是「不存在第二份握手权威」的可执行形式）。`wire_client::tests` 用一个替身城证明：查询在窗口到期之前随答复返回；带 `--until` 的命令在窗口到期之前随点名的事件返回，之前的事件照样打印；点名的事件没来而别的事件来了，结果是 `Unfinished` 而不是已答复。`wire_client::ending::tests` 证明：派到楼的等待认出在它房间里开始的 run，别的 run 冻结不结束它；`--detach` 在 run 开始时结束并带回 run id；窗口在里程碑之前关上是 `Unfinished`。真城验收：`call` 一条必被拒的命令，收到 `refusal` 且退 1。
-/

/-!
## 8-41 门说的话与门做的事：静默有自己的退出码，重放的命令只做一次（`bin::wire_client`、`accounting::worker::commanding::entrance`）

仓外的对抗性检验器（`tools/adversary/Spec.lean` §4）留了两条未修的发现。两条都只在**门外**可观测，
两条都伤同一类调用方——一个拿退出码分支、拿重试兜底的 agent。本节一次答完，因为它们是同一个承诺的两半：
**门说出口的话必须等于门做的事**。

### 发现一：静默不是接受，故它不是 0

**原因**：窗口内没收到拒绝，不等于城照办了。`AttachEndpoint` 指向一个连不上的 base URL 时，产品侧探测 15 s，客户端默认窗口 2 s（`tools/adversary/Spec.lean` §4 第一个发现）；把没有拒绝读成 0，「拒绝没赶上静默窗口」与「城照办了」就是同一个码。

**依据**：`call` 与 `dispatch` 有四种结局。

```rust
// bin::wire_client::ending（形状 1 decision：四支穷尽枚举，壳只做映射）
pub(crate) enum Spoken { Refused, Answered, Quiet, Unfinished }
pub(crate) struct Heard { frames: u32, refusals: u32, answers: u32, awaited: Awaited, run: Option<RunId> }
impl Heard { pub(crate) fn spoken(&self) -> Spoken; }
```

| 结局 | 退出码 | 它断言的事实 |
|---|---|---|
| `Refused` | 1 | 城在窗口内拒绝了这一帧 |
| `Answered` | 0 | 城在窗口内答了话，且没有拒绝 |
| `Quiet` | 3 | 帧发出去了，窗口内**一帧都没回来**——城是否受理，此处无法断言 |
| `Unfinished` | 3 | 城回了话，但调用方在等的那一帧（查询的答复、点名的事件、派出的 run 的里程碑）没在窗口内来——活可能还在做，也可能没开始 |
| （用法错误） | 2 | 参数或帧不是这条命令能读的东西；帧在开 socket 之前解析，所以与城无关 |
| （没有城） | 4 | `--at` 那里没有东西完成握手；帧没有被任何城听见 |

- **`answers` 与 `frames` 是两件事**。握手的 `Welcome` 也是一帧，故 `frames` 恒 ≥ 1；能区分静默的只有
  「命令发出**之后**回来的帧数」。把这一个数放进 `Heard` 而不是在壳里减一，是因为「减一」会把握手协议的形状
  抄到第二个地方。
- **3 而不是复用 1**（被否决的备选：静默即拒绝）。静默不是拒绝：城可能已经受理，只是答案比窗口慢。
  把它读成拒绝，会让一个 agent 在城正在照办的时候重试——而重试的无害性正是发现二在修的东西。
- **0 与 1 的含义一个字不改**，故已有的脚本只在原本被误读为成功的那一档上改变行为，这正是要改的那一档。
- **窗口不变长**（被否决的备选：把默认 `--quiet-ms` 提到 20000）。窗口多长是调用方的事；把它调大只是把同一个
  歧义推后 18 秒，而 `Quiet` 让调用方**知道自己撞上了窗口**，于是加窗口重试是它能做的一个决定。

**本节测试**：`a_city_that_says_nothing_inside_the_window_is_not_a_success`——一个脚本化的 WebSocket 服务端
答完 `Welcome` 后闭口不言；`call` 返回的 `Heard` 的 `spoken()` 必须是 `Quiet`，`answers` 为 0。

### 发现二：一把必须带而无人读的钥匙

**原因**：23 个状态变更命令每一个都带 `IdemKey`，`kernel::idem::claim` 把这道门实现成纯函数，
而它在自身模块之外**没有调用者**。同一条 `Halt` 发两次，账本里两条 `city_halted`。
`accounting::worker::desk` 只合并**还在队列上或正在被执行**的同键命令（`clockwork.rs` 那条测试钉的就是它），
一旦第一条跑完，重放就是第二次副作用。

**依据：判在命令入口，判在任何副作用之前**（`crates/kernel/Spec.lean` §8-6 的原话）。

```rust
// accounting::worker::commanding::entrance（形状 1 decision：状态是集合，判定借 kernel::gate::dedup）
pub(in crate::assembly) struct Entrance { /* seen: BTreeSet<IdemKey>, refused: BTreeMap<…>, carrying: Option<IdemKey> */ }
impl Entrance {
    pub(in crate::assembly) fn answered(&self, key: &IdemKey) -> Option<Result<(), AxError>>;
    pub(in crate::assembly) fn begin(&mut self, key: IdemKey);
    pub(in crate::assembly) fn settle(&mut self, outcome: &Result<(), AxError>);
    pub(in crate::assembly) fn absorb(&mut self, data: &Payload);   // 账本回放
    pub(in crate::assembly) fn carrying(&self) -> Option<IdemKey>;   // 正在处理的命令的键
}
// 盖键：键与载荷的纯函数，记账线程上的 RunWorker::record_for 与不借 worker 的 Stamping::record_for 共用
pub(in crate::assembly) fn stamped(key: Option<IdemKey>, data: Payload) -> Result<Payload, AxError>;
pub(in crate::assembly) fn repeated(name: &str) -> String;   // 重复命令留下的那行诊断
pub(in crate::assembly) const IDEM_FIELD: &str = "idem";
```

- **门是 `serve_one`，不是 `handle`**。`serve_one` 是「一个人发出的命令变成什么」的唯一权威
  （`bin::assembly` 的 rustdoc 原话），也是 wire、控制台与 ACP 三条路唯一的汇合点——`assembly::attending`
  是它在产品里的唯一调用方。`handle` 是执行者，留给夹具与内部调用方按顺序驱动一座城；
  **门与执行者分开，是因为「判过了吗」与「怎么做」是两个问题**，而把它们合成一个方法会让
  每一个内部调用方都被迫带一把它并没有从人那里收到的钥匙。
- **判定借 `kernel::idem::claim`，不在这里重写**。`Entrance` 持有那个 `BTreeSet`，kernel 只回答成员关系——
  这正是那个纯函数的 SPEC 说的「seen 集合是调用方的状态」。因此不改 kernel 的立面。
- **重复的键得到第一次的答案，且不再写第二次**。第一次是 `Ok` 就答 `Ok`（沉默地成功，因为那件事已经做过了）；
  第一次是拒绝就把**同一份** `AxError` 再交一次，于是重试的人两次读到同一句话，而不是第二次读到
  「没有东西在等」这种由第一次的副作用造出来的第二种拒绝。
- **重启后靠账本认出做过的事**：`serve_one` 在一条命令的执行期间把钥匙挂在 `carrying` 上，
  `record_where` 与 `record_for` 把它写进那条记录的 payload（键名 `idem`）。
  `Standing::fold` 已经在开城时逐行走一遍账本，`Entrance::absorb` 搭在同一趟上，不多读一遍。
  于是**一条命令留下了历史，它的钥匙就在历史里**。
- **已知边界，如实写在这里**：`run_started` 由 `runtime::run::lifecycle` 直接写进账本，装配点碰不到它，
  故一条**跑到一半就被进程死亡打断**的派活，其钥匙不在账本上，重启后重发会再跑一次。这恰好是重试**应当**
  被允许的那一档——那次派活没有结论。跑完的派活会经 `settling::landing` 的 `record_for` 落下带钥匙的记录，
  于是重启后再发同一把钥匙，答的是第一次的结果。
- **被否决的备选一：在 `accounting::worker::desk` 上记住所有见过的钥匙**。桌子没有账本，重启即失忆；且第一次的结果
  在桌子上不可得，它只能沉默地丢弃重放，而不是回答。
- **被否决的备选二：把 `IdemKey` 从线格式上撤掉**。那是把承诺删掉而不是兑现它，且 23 个命令的重试语义会
  一起消失。

**本节测试**：`the_same_dispatch_twice_under_one_key_opens_one_room_and_starts_one_run`——同一条 `Dispatch`
经 `serve_one` 送两次，钥匙相同：账本里恰有一条 `run_started`，房间恰有一个（此前是 `["one", "one-2"]`）；
`a_repeat_is_answered_with_what_the_first_ask_was_answered`——被拒的命令重发收到逐字相同的那份拒绝，
成功的命令重发不被拒也不再落账；`a_key_already_in_the_history_is_recognised_after_a_restart`——
一座重新打开的城认得账本里那把钥匙。

### `bin::wire_client` 的两件事各有文件

socket 上的一次对话与 HTTP 上的一次托管是两件事，同处一个文件时 `wire_client.rs` 越过了 400 行上限。

| 文件 | 管什么 |
|---|---|
| `wire_client.rs` | 与城的一次 WebSocket 对话：`Heard`／`Spoken` 与三支退出码、握手 `hello`、`call`／`converse`／`next_frame`／`report`，以及两处共用的不可达判词 `unreachable_city` |
| `wire_client/enrolment.rs` | 把一份明文交给同机的 `/enroll` 路由并取回替代它的引用：`split_reference` 与 `enrol`，连同钉住引用形状的 2 个 `#[test]` |

- **`unreachable_city` 仍只有一个家**：它留在父模块，`enrolment.rs` 经 `use super::unreachable_city` 取用，于是「城连不上」这句话不会有第二种说法。
- 父模块以 `pub(crate) use enrolment::{enrol, split_reference};` 转出，`main/data.rs` 的两处调用路径一字未改。
- 公开面不涉：两项都在二进制内部。

### 文档同步

本节；`ARCHITECTURE.md` §12 增 `accounting::worker::commanding::entrance` 与两个测试文件的行；
kernel D1 记下 `gate::dedup` 由 `idem::claim` 与 `IdemGuard` 接替；`tools/adversary/Spec.lean` §4 两条发现标注已修。
`docs/operating.md` 增退出码表。公开面：`bin::wire_client` 与 `bin::assembly` 都是二进制内部（`pub(crate)`
以下），`RunWorker::handle` 的签名不变。
-/

/-!
## 8-97 `sprawling gauge --at`：经线协议看监视器（`bin::wire_client::watching`，形状：adapter）

`sprawling gauge [--at host:port] [--token T]`（别名 `top`）与 `call` 一样经 `--at` 找到一座在跑的城（默认同一个 `DEFAULT_AT`），握手后发 `ClientFrame::Monitor(Watch)`，此后每收到一个 `ServerFrame::Monitor` 就输出一次，直到城关闭连接或人按 Ctrl-C。城是由地址找到的，而不是由城名：一座城的地址就是它被 `serve`/`up` 时占的端口，`call` 与 `enrol` 已经这样找城。`bin::main::gauge` 选中 `City` 这个对象之后调本模块（§8-129-4）。

**接口。**

- 输出的形式由 `bin::audience::Audience` 定：`Person`（stdout 是终端）画一屏，`Agent` 每个读数一行。
- `shown(text: &str, history: &mut VecDeque<Sample>, audience: Audience, curve_width: usize) -> Option<String>`：收到的一帧文本变成要写到 stdout 的文字。不是监视读数的帧（欢迎、事件等）返回 `None`。读数先放进 `history`（满 `monitor::CAPACITY` 丢最旧的），`Agent` 返回 `gauge::lines::city_line` 加一个换行；`Person` 返回清屏并把光标移到左上角的 `ESC[H ESC[2J`，接 §8-95 的 `screen(history, curve_width)` 与换行。`city_line` 失败时返回 `None`（§8-95：实际不会出现）。
- `top(at: &str, token: Option<&str>, audience: Audience) -> Result<(), Unheard>`：连接、握手、发 `Watch`、逐帧调用 `shown` 并写出。连不上是 `Unheard::NoCity`（退 4，与 `call` 同一张表）；握手被拒、读帧出错与写 stdout 失败是 `Unheard::Broken`（退 1）；连续 5 s 没有一帧视为城已停，正常返回。
- 曲线宽度：环境变量 `COLUMNS` 能读成数时取它减去标签与读数占的 36 列，否则按 80 列算（44 个点）；标准库不给终端尺寸，不为这一个数引入依赖。

**决定。**

1. 文字由纯函数 `shown` 算出，socket 与 stdout 留在 `top`：一帧变成哪几个字节可以用整串比较来测，而连接只在端到端里测。
2. 沉默 5 s 即结束，而不是永远等：读数每秒一个，5 个空拍说明城已停或已不再发，一个 agent 读到 EOF 比读到永远的阻塞有用。重新考虑的条件：采样的节拍变长。

**测试。** `wire_client::watching::tests`：一帧读数在 `Agent` 下是一行 `city` JSON 加换行，在 `Person` 下是清屏序列接一屏；不是读数的帧什么也不输出、不进历史。
-/
