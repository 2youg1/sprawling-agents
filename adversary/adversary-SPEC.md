# adversary-SPEC

> `adversary/` —— 仓外的对抗性性质检验器。它不是 crate，不进 workspace，不进发布物，不进 `just check`。
>
> 权威顺序：人的决定 → `ARCHITECTURE.md` §8「the wire is the whole API; a second client writes against it」→ 本文 → 代码与测试。本文先于代码改动。

## 1 需求分解

`ARCHITECTURE.md` §8 把线格式定为整个 API，并明说「用任何语言写第二个客户端都是支持的」。本目录行使这一条：它是**第三个**客户端，写在仓外，用来攻击而不是使用。拆成六个可独立验收的最小单元：

| 单元 | 交付物 | 独立验收 |
|---|---|---|
| U1 门 | `Door`：以子进程驱动已构建的二进制，把 stdout 的每一行解成 `Frame` | 一条 init→serve→create_building→city_view 的轨迹被解析成结构化值；未知的帧类当场报错而不是被忽略 |
| U2 场地 | `Ground`：一次性城目录、一个被端起来的进程、一个端口，以及磁盘上的敌意动作 | 场地退出后不留文件也不留进程；`stored` 看到的正是账本目录里的字节 |
| U3 模型 | `World` 与 `runTrace`：动作集合与后置条件 | 随机轨迹全通过；把停摆守卫从模型里摘掉则立即报错 |
| U4 定向对抗 | 任意前缀 ＋ 一次 `Stop` ＋ 任意后缀 ＋ 一次必须被拒的派活 | 该性质在随机轨迹上成立，且停摆后那一次派活被拒且拒得其所 |
| U5 回归 | 反例最小化后渲染成 Rust `#[test]` | 渲染结果与仓内那个 Rust 文件逐字节相同，而该文件由 `cargo test` 编译运行 |
| U6 历史 | 任意轨迹之后，账本离线自证；改一个字节则不能自证 | `replay` 在干净轨迹上恒绿、在翻过一位的轨迹上恒红 |

**不负责**：任何规则的再实现（链哈希、`IdemKey` 派生、写域判定、份额守恒）；任何 Rust 侧的构建闸门；任何随产品交付的东西。三者中任何一条被违反，本目录应当被删除而不是被修补。

**U6 是本目录相对一次性 CLI 检验器的增量**，理由在产品而不在方法：一座城把**一条全序的历史**写在磁盘上，于是「任意轨迹之后历史仍然自洽」是一条可以对着随机轨迹反复问的性质，而不只是一次定点检查。

## 2 验收标准

1. `just adversary` 全绿：13 条检查，`0 failed`。
2. 没有 Lean 的机器上 `just check` 的行为与本目录不存在时**逐字节相同**；`just adversary` 打印 `skipped: Lean is not installed` 并返回 0。
3. 模型不预测任何哈希、`seq`、时间戳或 `IdemKey`。凡断言只谈**两条轨迹之间的关系**，或**门对调用方的承诺**（稳定错误码）。
4. U5 渲染出的 Rust 源码与 `crates/sprawling/tests/from_adversary.rs` 逐字节相同。
5. **咬得动的证据**：把 `Model.lean` 的 `refusal` 里 `work` 那条停摆守卫摘掉后，`a halted city takes no work until it is released` 必须失败。一个永远为真的性质与没有性质等价。

   **已演示。** 摘掉该守卫后该性质报错，收缩 3 次得到两步反例 `Stop City ; Work acme one`，并指出 `refused with Code "E_GATE_DENIED" where Code "E_CONFIG_INVALID" was owed`；恢复后转绿。它咬得动的是**守序**，而不只是「停摆时派活会失败」。这一条同时是对 §13 那套自备机器的验收：生成器、收缩器、极性推导与后置条件四件必须同时工作，才会得到这个最小反例。

## 3 假设与歧义

| 歧义 | 假设 | 何时失效 |
|---|---|---|
| 门的形状 | `sprawling call <frame> --at <addr> --quiet-ms <n>`：stdout 每行一枚 JSON 帧，stderr 一行计数，退出码 0／1／2／3 | 线格式换传输时门变成它的 schema，改 `Door.lean` 一处 |
| 城是什么 | 一个本地目录，`init` 造它，`serve` 端起来，账本在 `.sprawling/ledger/` 下按段分文件 | 布局改变时 `Ground.lean` 的敌意动作报错，属预期 |
| 静默 | 门的第三种回答。**不是接受**——见 §10「静默不是接受」 | 若将来 `call` 改为「命令被受理才返回」，`quiet` 这一支变成异常而不是取值 |
| provider | 一个都不挂。于是每一次派活在配置这道门上被拒，而模型知道这一点 | 挂上任何真 endpoint 后本目录会把 `E_CONFIG_INVALID` 报成失配，届时模型要学会第二种世界 |
| 时钟 | 只用于超时，从不被预测 | —— |
| 端口 | 从 47100 起向上探，第一个能答 `city_view` 的即用 | 机器上有别的东西占着整段时报错并说明 |

## 4 现状分析

Rust 侧的验收测试全部是**具体轨迹**：`crates/sprawling/tests/assembly_door.rs` 证明「这一条路走得通」，不证明「任何一条路都走不出去」。本目录的全部增量在后者，以及三件 Rust 侧写不出的事：

1. **门的承诺只在门外才可观测。** 退出码、两个流的分工、静默与拒绝的区别，都是「一个 agent 拿这个二进制写脚本」时遇到的事实，而进程内的测试永远看不见它们。
2. **敌意的磁盘不是被 mock 的磁盘。** `memory::fault_fs` 是一个确定性掉电模型，它回答「我们设想的坏」；本目录直接翻掉账本里的一位，回答「随便一位坏了会怎样」。
3. **任意前缀与任意后缀。** 只会均匀随机生成的东西是 fuzzer；把攻击命名出来再对它前后做全称量化，才是对手。

### 第一个发现：静默与成功无法区分

**`sprawling call` 在拒绝没赶上静默窗口时退出 0。** 实测：`AttachEndpoint` 指向一个连不上的 base URL，产品侧探测超时远长于客户端默认静默窗口。同一次操作，城自己在诊断日志里写下拒绝，并写下「the refusal above reached nobody」。

**诊断**：城是诚实的——它知道拒绝没送到，并且说了出来。缺陷在门：`main.rs` 的 rustdoc 写着 *"Exits 1 when the city refused something"*，而实际语义是「**在静默窗口内没有拒绝到达**」。对一个拿退出码做分支的 agent，这两者的差别是把一次失败读成一次成功。

**已修（「静默有自己的退出码」）。** Rust 侧选了三档中的第二条：`Spoken` 是一个三支穷尽枚举，`Quiet`（帧发出后窗口内一帧未回）退 **3**，而 0 与 1 的含义一个字不改。表写在 `docs/operating.md` 与 `crates/sprawling/sprawling-SPEC.md` §8-41 里。本目录的 `Door` 因此把静默解成 `quiet` 而不是 `accepted`（§8），并用 `the exit code says what the frames say` 这条检查把它钉住。

### 第二个发现：一次被拒的派活写进了城里

随机轨迹在早期样本上失败，收缩后得到两步：`Raise "acme"`／`Work "gamma"`（一个从没立过的地址）／`Look`——`Look` 看见 `["acme","gamma"]`，而只有 `acme` 被立过。追下去是同一条缝的两个症状：派活到一个没立过的楼答 `E_CONFIG_INVALID`，而磁盘上留下了 `城根/gamma/one/JOB.md`，账本里一条都没有。

**诊断**：`assembly/dispatching.rs` 的 `dispatch_in` 顺序是「判停摆 → 写 JOB.md → 落 CAS → 解析模型 tag」。它开头那句注释把该守的规矩写得一字不差——*"Nothing is written before the city agrees to take the work"*——**停摆那道门守住了它，配置那道门在写之后才判**。这与 `ARCHITECTURE.md` §5 第 4 条（*every effect becomes an event first*）直接冲突。

**边界已量过，不夸大**：保留子树是守住的。`Work ".sprawling/evil"` 得到 `E_INVALID_ARGS` 且一个字节都没落地，所以这不是写域逃逸，而是「判定晚于副作用」。

**已修。** 向没立过的 `gamma` 派活，城答 `E_CONFIG_INVALID`，城根下仍然只有 `City.md`。`a refusal costs nothing` 那一组两条随之转绿并留着——它们此后守的是那次修复确立的**判定先于副作用**这个次序，而不是当时那一行代码。

### 第三个发现：一把每条命令都必须带、而没有人读的钥匙

线格式要求每一个状态变更命令都带 `IdemKey`，`kernel::gate::dedup` 把这道门实现成一个纯函数，而它在自身模块与自身测试之外**找不到任何调用者**。可观测的后果，全程只经门：同一条 `halt` 带同一把钥匙送两次，账本里留下两条 `city_halted`。

**诊断**：`IdemKey` 存在的理由是让**重试无害**。今天这条保证没有承兑人。`gate::dedup` 自身没有错，错在它没有被接到 `assembly` 的命令路径上。

**已修（「重放的命令只做一次」）。** `bin::assembly::commanding::entrance` 持有那个 `seen` 集合，`RunWorker::serve_one`——wire、控制台与 ACP 三条路唯一的汇合点——在任何副作用之前向 `kernel::gate::dedup` 问一次，重复的键得到**第一次的答案**，且不再写第二次。钥匙随命令写进它所产生记录的 payload，开城时那一趟已有的账本折叠把它读回来，故重启之后同一把键仍然认得。
**仍未覆盖的一档**：`run_started` 由 `runtime::run::lifecycle` 直接写账本，装配点碰不到它，故一次跑到一半被进程死亡打断的派活，其钥匙不在账本上——那正是重试应当被允许的一档。`keyUsedTwice` 因此单独成组留着看守这一档。

**这条承诺反过来约束模型的记账**：被拒的命令同样花掉了它那把钥匙，故 `minted` 必须在负向动作之后照样前进（`failureNextState`）。若不然，下一条命令带着城已经答过的钥匙，读回来的是**上一条命令**的那份拒绝——看上去像门读帧落后一条，实则是钥匙重了。只有查询不花钥匙，故 `look` 是唯一的例外。

`look` 因此不进随机生成器（`Model.lean` 记了理由）：一条随机轨迹里只要出现一次被拒的派活，其后每一个 `look` 都会为同一个原因失败，那会把一个缺陷报成许多个，并把下一个缺陷藏在它后面。

## 5 权威信源

**本节只指位置，不抄数值。** 一个被抄进本文的常量就是同一条规则的第二个权威，而它必然先于产品陈旧：这一点是量出来的——本文曾抄下一个 `WIRE_V`，产品早已走过它许多版，而本文读起来仍然像是对的。

| 事实 | 权威所在 |
|---|---|
| 线格式版本、握手内容 | `crates/channels/src/wire.rs` 的 `WIRE_V` 与 `Welcome` |
| 服务端帧类的全集 | `crates/channels/src/wire.rs` 的 `ServerFrame` |
| Command 与 Query 的全集 | `crates/channels/src/command/kind.rs`、`crates/channels/src/wire/query.rs` |
| 稳定错误码的全集 | `crates/kernel/src/error.rs` 的 `AxCode::ALL` |
| `IdemKey` 与模板名的形状 | 门的拒绝原文，实测 |

**门讲六类帧**（`ServerFrame`）。`Frame.lean` 认得全部六类，并对第七类当场报错——一个未知的帧类意味着线格式变了形，而把新形状当成一次拒绝会把红的测成绿的。其中 `log` 只被解析、不被断言：它没有自己的账本序号，两行可以共用一个位置，漏掉一行什么也没丢；它被解析仅仅因为城在这条通道上**也**叙述它的拒绝，而一个读失败报告的人想看到那句话。

## 6 命名统一

`Address`、`Building`、`Run`、`Ledger`、`Seq`、`Refusal` 一律沿用 `docs/glossary.md` 与 `ARCHITECTURE.md` 的词表，本目录不得另起名字。本目录只新增三个词，各自只指一件事：

| 词 | 它是什么 |
|---|---|
| **Door** | 已构建二进制的 `call` 门面，唯一知道有个可执行文件存在的地方 |
| **Ground** | 一次性的场地：一座被端起来的城、它的端口、它的账本目录，以及磁盘可以施加的敌意 |
| **Trace** | 一串动作及其观察结果，是本目录唯一的断言对象 |

## 7 模块边界

```
lakefile.toml                工程定义；不被任何 Rust 构建读到
lean-toolchain               编译它的工具链版本
lake-manifest.json           依赖清单，`"packages": []`
src/Sprawling/Frame.lean     线格式的代数镜像。只解析，不判断
src/Sprawling/Door.lean      唯一知道二进制存在的地方
src/Sprawling/Ground.lean    一次性场地，以及磁盘的敌意
src/Sprawling/Check.lean     抽样、收缩、检查树。不知道城是什么
src/Sprawling/Model.lean     状态模型、后置条件、定向对抗场景
src/Sprawling/Regression.lean 反例 → Rust #[test]
test/Main.lean               入口与检查树
```

依赖单向：`Model` → `Door` → `Frame`，`Model` → `Ground` → `Door`，`Model` → `Check`，`Regression` 只依赖 `Model`。`Frame` 不 import 任何本工程模块；`Check` 也不，且它**不 import `Door`**——抽样与收缩不允许知道有一座城存在。

`Ground` 依赖 `Door` 而不是自己起进程：**「二进制在哪」只允许有一个答案**，而场地要用它做三件事（`init`、`serve`、探活）。

## 8 接口先行

```lean
-- Frame.lean —— 线上说了什么
inductive Frame
  | welcomed (welcome : Welcome) | happened (record : Record)
  | answered (name : String) (body : Json) | refused (complaint : Complaint)
  | streamed (run : String) (text : String) | logged (level : String) (line : String)
structure Complaint where code : Code; action, subject, recovery : String; retriable : Bool
def decodeFrame  : String → Except String Frame
def cityBuildings : Json → Option (List String)

-- Door.lean —— 怎么问
structure Door where binary : System.FilePath
inductive Answer | accepted (frames : List Frame) | denied (complaint : Complaint) | quiet
def discover     : IO (Option Door)
def Door.raise   : Door → System.FilePath → IO Unit
def Door.serve   : Door → System.FilePath → Port → IO Serving
def Door.ask     : Door → Port → Verb → IO Answer
def Door.verify  : Door → System.FilePath → IO (Except String Nat)
def idemKey      : Nat → IdemKey

-- Ground.lean —— 在哪里问，以及磁盘怎么撒谎
def withGround      : Door → (Ground → IO α) → IO α
def Ground.ledger   : Ground → System.FilePath
def Ground.stored   : Ground → IO (List (String × ByteArray))
def Ground.tree     : Ground → IO (List String)
def Ground.corrupt  : Ground → IO Unit   -- 翻一位：最老那条记录的中点
def Ground.tear     : Ground → IO Unit   -- 截尾：最新那段的末 20 字节
def Ground.duplicate : Ground → IO Unit  -- 重放：把最老那条记录再写一遍

-- 三个敌意动作各自回答一件事，不得合并：`corrupt` 问检测，`tear` 问恢复
-- （断尾修复是产品**故意**支持的路径），`duplicate` 回到检测。

-- Check.lean —— 怎么抽、怎么缩、怎么跑
structure Gen (α : Type) where draw : StdGen → α × StdGen
def shrinkList : List α → List (List α)
def forAll : Nat → Nat → Gen α → (α → String) → (α → List α) → (α → IO Verdict) → IO (Option Finding)
inductive Tree | leaf (name : String) (check : IO Unit) | group (name : String) (children : List Tree)

-- Model.lean —— 断言什么
inductive Yield | nothing | addresses
inductive Action : Yield → Type
def refusal   : World → Action y → Option Code          -- 纯
def runTrace  : World → Trace → Attempt Verdict          -- 带 IO
def haltIsHonoured : Nat → Gen Trace

-- Regression.lean —— 交付什么
def render : String → Trace → String
```

`ask` 返回 `Answer` 而不是抛异常：被拒绝是产品的正常输出，而**解析失败**才是异常——门的形状变了，检查应当当场停下，而不是把新形状当成一次拒绝。

**黑盒边界由类型划定。** `World` 到 `refusal` 全段没有一个签名提到 `IO`，`Gen` 是种子的纯函数；于是决定「欠哪一种失败」的那一半，和抽取轨迹的那一半，都够不到它们正在审判的那座城。一个能先看后判的模型会按构造与产品一致，那是本目录唯一可能在什么都没检查的情况下报绿的路。

`Action` 以 `Yield` 这个有限标签为索引而不是以结果类型本身为索引：后者会把存在包装推到更高的宇宙，进而把宇宙多态传染给 `Gen`、`shrinkList` 与整棵检查树。以标签为索引，`look` 是唯一答地址表的动作这一点仍由类型保证，而别处一分钱不花。

## 9 工作流程

1. `just adversary` 先 `cargo build -p sprawling`，把二进制路径经 `SPRAWLING_BIN` 传给检查器。**唯一一处知道二进制在哪的地方是 justfile**。
2. `lake exe adversary` 按树跑：先跑 U5 的渲染对拍（毫秒级，先失败先止损），再跑 U1 的门契约，再跑 U3 的随机轨迹，然后 U4 的定向场景，再是 U6 的历史自洽与磁盘敌意，最后是两个按缺陷命名的组。
3. 反例出现时，报文给出种子、收缩次数、最小化后的轨迹与破掉的那条承诺。把种子经 `SPRAWLING_SEED` 传回去可以原样复现。
4. 人把最小反例经 `SPRAWLING_ACCEPT=1` 渲染成 Rust 源码，提交到 `crates/sprawling/tests/`。**知识就此迁移到 Rust，本目录不保留它。**
5. `just adversary --select <文字>` 与 `--reject <文字>` 按检查的完整路径筛选，定时任务用它把「open finding」那一组与其余分开跑。

## 10 实现逻辑

**门**：`IO.Process.output` 在等待之前把两个管道读空——这是手写版本必须记住的死锁：一个没被读的管道会让子进程活着，于是先等待就会在输出超过一个缓冲区时把两边挂住。退出码 0／1／3 都要读 stdout；退出码 2 是用法错，抛出。

**静默不是接受。** `ask` 在拿到 `welcome` 之后若一个帧都没再来，返回 `quiet`。模型对每个动作声明它期待哪一种回答，**`quiet` 从不满足任何期待**。`log` 与 `welcome` 同样不算回答：城在那条通道上**也**叙述它的拒绝，把叙述算成回答就会把一条城根本没受理的命令报成受理了。

静默窗口取 250 ms。`sprawling call` **要等满静默窗口才返回**，于是窗口是每一个动作都要付满的价钱。短窗口在这里是诚实的，理由在模型而不在计时：它一个 endpoint 都不挂，每条命令都在本地磁盘与回环上答完，实测均在 1 ms 以内。慢路径自身的危险由「模型里没有一个动作走它」来保证，而不是靠等。

**模型**：`World` 只记一个用户记得住的东西——哪些楼立着、哪些范围停摆着、以及有没有挂过 provider。它**不记** `seq`、哈希、时间戳；`seq` 只作为 U6 里「逐行读账本」时的相邻关系间接出现。

**极性由模型算，不由轨迹带。** 一条轨迹只是一串动作；某一步是正向还是负向，由 `refusal` 在那一刻的世界上回答。于是「这一步应当成功」这件事只有一个权威，手写轨迹也无法声称一件模型说必须失败的事会成功。模型变了而手写轨迹没跟上时，红出现在 U5 的逐字节对拍上——那是同一个信号的更早、更具体的一次出现。

**后置条件**，逐条都是关系或承诺而非期望值：

| 动作 | 断言 |
|---|---|
| `raise` 一个没人占的良构地址 | 一定被接受 |
| `raise` 一个已被占的地址 | 一定被拒，且 `E_INVALID_ARGS` |
| `raise` 一个含 `.sprawling` 段的地址 | 一定被拒，且 `E_INVALID_ARGS` |
| `work` 而城或该楼停摆 | 一定被拒，且 `E_GATE_DENIED` |
| `work` 而未停摆且无 provider | 一定被拒，且 `E_CONFIG_INVALID` |
| `seize` | 一定被拒，且 `E_WIRE_MISMATCH`，且 `recovery` 非空 |
| `look` | 答里的楼集合恰是模型记的那一套 |
| 每一次拒绝 | `recovery` 非空——三段式承诺的第三段 |

**三条守序（guard order）被显式钉住**，因为它们是用户看得见的差别：停摆压过配置（`E_GATE_DENIED` 而不是 `E_CONFIG_INVALID`），地址良构压过占用，以及——**对派活而言**——停摆压过地址良构。一个把前两条调换了的实现会在城停摆时叫人去挂 provider——恢复建议指向一件与真实原因无关的事。

第三条是**量出来的，不是想出来的**，而且它纠正的是模型而不是产品。四次直接测量说明产品是自洽的：

| 条件 | 码 |
|---|---|
| 派活，未停摆，保留地址 | `E_INVALID_ARGS` |
| 派活，未停摆，普通地址 | `E_CONFIG_INVALID` |
| 派活，已停摆，保留地址 | `E_GATE_DENIED` |
| 立楼，已停摆，保留地址 | `E_INVALID_ARGS` |

停摆是**派活**这件事最外层的那道门，而立楼不是派活，所以停摆盖不住它。两个次序都有道理，产品选了其中一个并且到处一致；模型断言一个它从未量过的次序，那是模型在发明规则。

**定向对抗**（U4）：先用一个具体动作把世界推到「有一栋楼」，再抽任意前缀，插入那一次 `Stop City`，再抽任意后缀，最后在**读回模型状态确认仍然停摆之后**补上一次派活。那一步的极性同样由 `refusal` 决定，于是「它必须失败、且必须以 `E_GATE_DENIED` 失败」是模型自己的断言，而不是此处另写的第二条。后缀里可能出现 `resume`，所以那一次读回是必须的：断言一个产品不欠的拒绝，就是本目录在发明规则。

**「拒绝」不是「期望输出」**。模型断言的是**哪一种失败**，从不断言任何被计算出来的值：错误码由 `AGENTS.md` 定为门的契约的一部分，钉住它钉的是产品对调用方的承诺，不是对某条规则的重算。

## 11 边界枚举

| 边界 | 处理 |
|---|---|
| 二进制不存在 | `discover` 返回 `none`，整棵树跳过，退出码 0 |
| `SPRAWLING_BIN` 指向不存在的文件 | 抛出。调用方点了名就说明它已经构建过，空答案意味着路径在传入时被改坏了，而把它报成绿等于把一次什么都没测的运行报成通过 |
| 端口全被占 | 47100–47115 十六个口由一个 `IO.Ref` 池独占出借；连四次都被外面的进程占住才抛出并说明——这是环境问题，不是产品缺陷 |
| `serve` 起不来 | 探活轮询到预算用尽为止。预算放得宽：答得出来的城第一次就返回，于是次数只在城真的慢时才被花掉——而一个绑得太紧的预算会把冷缓存、刚链好的二进制、负载重的 runner 全部报成产品起不来，并把城最后写到错误流的任意一句告警当成原因印出来 |
| 账本只有创世一行 | U6 的相邻断言在少于两行时平凡成立 |
| 轨迹里出现 Windows 路径分隔符 | 全程走 `System.FilePath`，不手拼字符串 |
| 进程没被杀干净 | `withGround` 用 `try … finally`，异常路径也走 `hangUp` 并等它被回收 |
| 检查自身抛出 | 原样传上去。只有端口在出口处被收回——一个把每种失败都改名的检验器，会在产品欠着答案时报出自己的脚手架 |
| 同一 `IdemKey` 用两次 | 模型每个动作各铸一个新的；重放同一个由 `keyUsedTwice` 单独断言，见 §4 第三个发现 |

## 12 错误处理

本目录没有「恢复」这个概念：断言不成立就是发现，发现就该停下并交付一个 Rust 回归测试。三件被当作错误处理的事：**门的形状变了**（JSON 解析失败、未知帧类）、**场地起不来**（端口或进程）、**用法错**（退出码 2）。它们都抛异常，因为继续跑只会把新形状当成拒绝，从而把红的测成绿的。

## 13 依赖选型

**一个外部依赖都不引入。** `lake-manifest.json` 的 `"packages": []` 是这条的机器形式。

| 需要的东西 | 由谁提供 |
|---|---|
| 读线上的 JSON | `Lean.Data.Json` |
| 起进程、读两个流 | `IO.Process` |
| 造场地、翻字节 | `IO.FS`、`ByteArray` |
| 抽样的伪随机源 | `mkStdGen` / `randNat` |

这么选的理由有两条，第二条是量出来的。其一，本目录由定时任务驱动，而一次**需要先解析依赖图才能开跑**的检验会为着一个注册表的状态变红，而一个意义不明的红比没有这次运行更坏。其二，构建可复现：清单锁死为空集，今晚与昨晚之间唯一会变的输入是种子，那正是定时任务应当探索的那个维度。

**代价要写清楚**：抽样、收缩与检查树没有现成件可用，于是它们写在 `Check.lean` 里，约二百行。这是本目录相对「拿一个成熟性质测试库」多付的钱。它买回三样：轨迹的极性由模型算而不是由库的约定决定（§10）；收缩按「先整块后单点」走，因为这里每个候选要花掉一整座城，而按元素逐个缩会在四十步的轨迹上付三十六次；以及定向对抗写成一个普通的生成器，而不是一套只有一个使用者的组合子语言。

**被否决的替代**：把 `Check.lean` 写成对任意状态模型通用的一层。本仓只有一个模型，`AGENTS.md` 的「只在已经有第二个实现的缝上引入 trait」直接适用——通用化会换来一层没有第二个实现的抽象，而具体写法让 `World` 与 `Action` 直接出现在签名里，读的人少走一跳。

**不引入**：任何 FFI、任何绑定 Rust 类型的东西、任何需要改 Rust 代码才能工作的东西、任何 HTTP 服务端（假 provider 会让本目录变成第二个 gateway 实现）。

## 14 硬编码声明

| 硬编码 | 意图 | 后续影响 |
|---|---|---|
| 账本布局 `.sprawling/ledger/*.jsonl` | 敌意动作要按它找到文件 | 该布局改变时本目录报错，属预期 |
| 端口 47100–47115 | 避开常用段，又不需要网络库依赖；整棵树串行跑，十六个是给外部占用留的余地 | 冲突时报环境问题 |
| 静默窗口 250 ms | 模型驱动的每个动词都答在 1 ms 内（实测），250 ms 是三个数量级的余量 | 模型开始走慢路径（挂 endpoint）时必须同步改 |
| 种子 `20260912` | 本地运行必须可复现：一次反例只有在它能被重跑时才值得渲染成 Rust 测试 | 定时任务经 `SPRAWLING_SEED` 用会变的种子，于是「每晚探索新轨迹」与「本地可复现」各得其所 |
| 三个地址 `acme` / `beta` / `gamma` | 固定的演员表，让反例可读 | 加人时同步改 `Regression.lean` 的模板 |
| 模板 `minimal` | 两个模板里不带保密约束的那个 | 要测 `confidential` 时它进模型 |
| 环境变量 `SPRAWLING_BIN` | 二进制位置的唯一入口 | 由 justfile 提供 |

## 15 影响面

对 Rust 侧的影响**必须**恰好为零：不改 `Cargo.toml` 的 members，不进 `cargo deny` 的依赖图，不参与 `xtask length` 的行数，不进 `xtask modmap` 的模块表。唯一的交汇点是 `crates/sprawling/tests/from_adversary.rs`——它由本目录渲染、由 `cargo test` 编译，两侧任何一方漂移都会让某一侧变红。

`xtask` 的扫描不进 `.lake`（`walk::SKIP_DIRS`），因为那是构建产物，而一道门为已提交的对象作证。`xtask release` 拒绝「引用了一台机器自己的文件的文件」，`xtask header` 要求每个 `.rs` 带 MPL 抬头；本目录不含 `.rs`，且只引用仓内相对路径与环境变量名，两道门都不适用。

## 16 测试与约束

按「坏得越早越省时间」排序，共 13 条：渲染对拍（U5，毫秒级）、门的契约三条（U1，含 §4 那条退出码性质）、随机轨迹（U3）、定向停摆（U4）、账本自洽（U6）、磁盘的三句谎话（U2），最后是两个按缺陷命名的组。一整套 2 min 36 s（实测，四核，热缓存）。约束是 §2 第 5 条——**咬得动**必须被演示过，而不是被相信。

**整棵树串行跑。** 一座被端起来的城占着一个端口、一个目录与一条历史，两组同时跑就三样都争。实测过的后果不是变慢而是**换城**：输的那一边城绑不上端口退了出去，它自己的探活却在同一个口上接到了赢的那一边的城，于是一整条轨迹跑在别人的历史上。它把当时还开着的那个缺陷测成了绿的——一个答案取决于哪个线程赢了的对手，比没有对手更坏。

**一条检查失败不中止整棵树。** 检验器存在的理由是把每一条破掉的承诺都报出来，停在第一条会把其余的藏在它后面。

## 17 文档同步

1. 本文。
2. `ARCHITECTURE.md` §11 的验证层表——本目录是 V9 之外的一层，记为 V10，并写明它不是门。
3. `AGENTS.md` 的命令表（`just adversary` 一行）与边界那一节。
4. `justfile` 的 `adversary` 配方。
5. `.gitignore` 的 `/adversary/.lake`，以及 `xtask/src/walk.rs` 的 `SKIP_DIRS`。
6. `.github/workflows/adversary.yml`——定时任务，永远不进 `check`。

---

*本文档采用 MPL-2.0。*
