-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import tools.adversary.spec.Acceptance
import tools.adversary.spec.Answer
import tools.adversary.spec.Check
import tools.adversary.spec.Model

/-! # adversary 的规格

`tools/adversary/` —— 仓外的对抗性性质检验器与验收世界。它不是 crate，不进 workspace，不进发布物；检验器不进 `just check`，本规格进 `just models`。

权威顺序：人的决定 → `ARCHITECTURE.md` §8「the wire is the whole API; a second client writes against it」→ 本规格 → 检验器的代码与它的运行。本规格先于代码改动。

本文件是规格的入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `adversary D<n>`。这份规格在 Markdown 时 §12 没有编号的条目，D1 从迁到 Lean 的那一版起编；号不复用，一条被取代的决定留着它的号，指向取代它的那一条。
-/

/-! ## 1 需求分解

`ARCHITECTURE.md` §8 把线格式定为整个 API，并明说「用任何语言写第二个客户端都是支持的」。本目录行使这一条：它是**第三个**客户端，写在仓外，用来攻击而不是使用。拆成以下可独立验收的最小单元：

| 单元 | 交付物 | 独立验收 |
|---|---|---|
| U1 门 | `Door`：以子进程驱动已构建的二进制，把 stdout 的每一行解成 `Frame` | 一条 init→serve→create_building→city_view 的轨迹被解析成结构化值；未知的帧类当场报错而不是被忽略 |
| U2 场地 | `Ground`：一次性城目录、一个被端起来的进程、一个端口，以及磁盘上的敌意动作 | 场地退出后不留文件也不留进程；`stored` 看到的正是账本目录里的字节 |
| U3 模型 | `World` 与 `runTrace`：动作集合与后置条件 | 随机轨迹全通过；把停摆守卫从模型里摘掉则立即报错 |
| U4 定向对抗 | 任意前缀 ＋ 一次 `Stop` ＋ 任意后缀 ＋ 一次必须被拒的派活 | 该性质在随机轨迹上成立，且停摆后那一次派活被拒且拒得其所 |
| U5 回归 | 两个世界的反例各渲染成 Rust `#[test]`，同一个文件 | 渲染结果与仓内那个 Rust 文件逐字节相同，而该文件由 `cargo test` 编译运行 |
| U6 历史 | 任意轨迹之后，账本离线自证；改一个字节则不能自证 | `replay` 在干净轨迹上恒绿、在翻过一位的轨迹上恒红 |
| U7 供应世界 | `Provider`：挂过 endpoint 的第二种世界——一个 URL 的全部拼法、一个注册模型的上限、两条同时跑的车道 | 等价类的每种拼法经探测与经挂载各落到同一个 `base_url`；注册过的模型带得出上限；messages 派活拿不到「没有输出上限」；不等第一条做完就派出去的活是两个 run |
| U8 配置世界 | `Layer`：写过配置的第三种世界——`configure_building` 任意序列之后，磁盘上的 `CONFIG.toml` 与 `Query::BuildingView` 折出的答案一致 | 任意非空写序列之后：答案等于最后一次写入的值；楼自己那层的文件陈述该值且不再陈述更早的值；上一层一个都不陈述 |
| U9 验收世界 | `Acceptance`：一个陌生人拿发行归档里的二进制，在一个新目录里起城，经门走完第一天、一次进程被杀、第二天早上，以及协作那一串——一栋要评审的楼里，计划的一行被分成两片叶子，两个同时派出的活认同一片叶子而只有一个拿到，活被重派、认下还空着的那一片并提出请求，另一个居民查它而判不通过；provider 是 `just acceptance` 在本目录之外起的替身（§13） | `just acceptance <archive>` 的每一步按序通过，第一处失败报出步名；人的清单写到 `target/acceptance/` 下的 `checklist.md` |

**不负责**：任何规则的再实现（链哈希、`IdemKey` 派生、写域判定、份额守恒）；任何 Rust 侧的构建闸门；任何随产品交付的东西。三者中任何一条被违反，本目录应当被删除而不是被修补。

**本目录是检验器与它自己的规格。** 规定某个 Rust 模块必须守住哪些性质的 Lean 模型住在该 crate 的 `spec/` 下（ARCHITECTURE.md §11「Specifications in Lean」），由 `just models` 证明。Lean 包的三个文件（`lakefile.toml`、`lean-toolchain`、`lake-manifest.json`）在仓库根，检验器是其中三个目标：`Sprawling` 库（`src/`）与 `adversary`、`acceptance` 两个可执行文件（`test/`）。它们只 import `Sprawling.*` 与 Lean 工具链自带的库，不 import 任何规格，所以检验器里没有任何规则的再实现；`just adversary`、`just acceptance` 与夜间任务构建它们，`just models` 不碰它们。检验器自己的规格是本文件与 `spec/` 下的分部，属于 `Spec` 库（`lakefile.toml` 的 `tools.adversary.Spec` 与 `tools.adversary.spec.+` 两个 glob），由 `just models` 证明；它只 import 工具链与自己的分部，不 import 检验器（ARCHITECTURE.md §11），所以分部里写的是参照定义与定理，检验器是它们的实现。

**U9 不在 `just adversary` 的检查树里**：它要一个发行归档和一个会应答的 provider，那一跑两样都没有。它是另一个可执行文件 `acceptance`，只由 `just acceptance <archive>` 调用；树里的世界不挂会应答的 endpoint，U9 挂，这是两者唯一的分界。

**U6 是本目录相对一次性 CLI 检验器的增量**，理由在产品而不在方法：一座城把**一条全序的历史**写在磁盘上，于是「任意轨迹之后历史仍然自洽」是一条可以对着随机轨迹反复问的性质，而不只是一次定点检查。
-/

/-! ## 2 验收标准

1. `just adversary` 全绿：树里每一条检查都通过，`0 failed`。**条数没有第二个家**——它是 `test/Main.lean` 的叶子数，本规格不抄。

   **曾经不是全绿**：U7 的五条里有三条红（等价类一条、上限两条），红在产品而不在检查。两处修复都落在 `crates/` 下——`Entered::resolved` 成为打字地址变成被调用地址的唯一一处，`select_model` 经 `OutputCeiling::resolve` 取上限——三条随之转绿（实测见 §4 第四、第五个发现）。反例按 §9 第 4 步渲染进 `crates/sprawling/tests/from_adversary.rs`，本目录不再留着它们。
2. `just check` 不读本目录的任何文件：删掉 `tools/adversary/` 后 `just check` 的每一步不变。没有 Lean 的机器上 `just adversary` 打印 `skipped: Lean is not installed` 并返回 0。
3. 模型不预测任何哈希、`seq`、时间戳或 `IdemKey`。凡断言只谈**两条轨迹之间的关系**，或**门对调用方的承诺**（稳定错误码）。
4. U5 渲染出的 Rust 源码与 `crates/sprawling/tests/from_adversary.rs` 逐字节相同。渲染器是那个文件的唯一权威：文件里的三条测试、两个辅助函数与每一行注释都从 `Regression.lean` 出，而 URL 的那五种拼法、中转站的名字与那个模型 id 从 `Provider.lean` 出。
5. **咬得动的证据**：把 `Model.lean` 的 `refusal` 里 `work` 那条停摆守卫摘掉后，`a halted city takes no work until it is released` 必须失败。一个永远为真的性质与没有性质等价。

   **已演示。** 摘掉该守卫后该性质报错，收缩 3 次得到两步反例 `Stop City ; Work acme one`，并指出 `refused with Code "E_GATE_DENIED" where Code "E_MODEL_UNCHOSEN" was owed`；恢复后转绿。它咬得动的是**守序**，而不只是「停摆时派活会失败」。这一条同时是对 §13 那套自备机器的验收：生成器、收缩器、极性推导与后置条件四件必须同时工作，才会得到这个最小反例。
6. `just acceptance <archive>` 把归档解进 `target/acceptance/`，按归档的 `skills/` 写出替身的脚本，起替身，然后用归档里的二进制走完 U9 的每一步（§9），每一步打印 `ok` 与耗时，第一处失败以步名开头报出并以退码 1 结束；全部通过才写清单。它不是门，也不进 `just check`：归档要先由 `just package` 造出来，而那是一次发布构建。

   **咬得动，已演示。** 去掉第一天里人准入 skill 的那一步，前四步照常通过，走到 `every shipped skill is pinned, and read by name it reaches the model` 停下，报出 run 钉住的 skill 是空表而书架上有九件；补回那一步转绿。协作那一串同样演示过：脚本里先不写协作的几个 run，前十四步照常通过，走到 `a resident divides the plan's row into two leaves` 停下，报出分计划的 run 以 `cancelled` 结束，替身的记录最后一行是 410 `no_run_left`、`run` 为 `null`；补上那几个 run 转绿。
-/

/-! ## 3 假设与歧义

| 歧义 | 假设 | 何时失效 |
|---|---|---|
| 门的形状 | `sprawling call <frame> --at <addr> --quiet-ms <n>`：stdout 每行一枚 JSON 帧，stderr 一行计数，退出码 0／1／2／3 | 线格式换传输时门变成它的 schema，改 `Door.lean` 一处 |
| 城是什么 | 一个本地目录，`init` 造它，`serve` 端起来，账本在 `.sprawling/ledger/` 下按段分文件 | 布局改变时 `Ground.lean` 的敌意动作报错，属预期 |
| 静默 | 门的第三种回答。**不是接受**——见 §10「静默不是接受」 | 若将来 `call` 改为「命令被受理才返回」，`quiet` 这一支变成异常而不是取值 |
| provider | `Model` 的世界一个都不挂，于是每一次派活在配置这道门上被拒，而模型知道这一点；`Provider` 的世界挂一个**这台电脑上没人听的地址**，于是每一次调用停在 socket 上；U9 挂 `just acceptance` 起的替身，调用成功，替身不在本目录里（§13） | 检查树里的世界要一个会应答的 endpoint 时，它们照 U9 的样子从 justfile 接收一个 URL，而不是在本目录里起一个 |
| 替身怎样分 run | 替身按请求带回来的调用 id 认出 run 与它走到哪一条，一个 id 都没带的第一轮按到达次序开启脚本里下一个 run（citysim D11、`tools/citysim/Spec.lean` §8-13；`spec/Acceptance.lean`）。所以 U9 只在第一轮可能同时到达的地方——两个同时派出的认领——把那几个 run 写成一样，其余的活都等前一个 run 冻结再派，第一轮的次序就是脚本的次序（D6） | 两个同时开启、要拿不同回复的 run 进 U9 时，替身要能认第一轮（citysim D11 的重开参数） |
| 杀进程 | `Serving.hangUp` 结束被服务的进程（Lean 的 `IO.Process.Child.kill`：Windows 上是 `TerminateProcess`，Unix 上是 `SIGTERM`；城只把 Ctrl-C、Ctrl-Break 与 `SIGINT` 当作有序关闭，所以 `SIGTERM` 同样不留交接），等被杀的 run 写下 `inFlight` 条 `tool_result` 之后才杀，所以刀落在两次写之间，而不是在最后一次写之后；被杀的 run 若已冻结，那一步报红并说明替身给的调用太少 | 城回来之后怎么处理那个死掉的 run，不是 U9 断言的事：它断言的是历史自证、城再服务、新的活跑到它自己的结尾 |
| 配置写回 | 「写了什么就读得回什么」这条不变量的对象是 **TOML 文件**，不是哪一条帧。人层偏好的那一条（`PutPreferences` / `Query::Preferences`）今天并不存在，而 `configure_building` 写楼自己那层、`Query::BuildingView` 把它折回来，是同一条不变量今天已经承载的地方，所以性质写在那里 | 那一对帧落地后，`Layer` 换成它们驱动，断言一字不改：变的是谁写进文件，不是文件欠谁什么 |
| 时钟 | 只用于超时，从不被预测 | —— |
| 端口 | 从 47100 起向上探，第一个能答 `city_view` 的即用 | 机器上有别的东西占着整段时报错并说明 |
| `just check` 读不读本目录 | `AGENTS.md` 写着 `just check` 不读 `tools/adversary/` 下的任何文件。本规格迁到 Lean 之后，`just models` 构建本文件与 `spec/`（ARCHITECTURE.md §11 为 `tools/` 下的规格定的位置），仍不构建检验器 | 那一句要改成「不构建检验器」；它在 `AGENTS.md`，本次迁移没有改它，这一格等它改了就删 |
| 协作那一串在哪栋楼里走 | 在一栋要评审的楼（`beta`）自己的计划上走：`plan` 只读写派活那栋楼自己的 `Roadmap.md`，市长在 City Hall 分的是 hall 的计划，没有一栋 builder 楼从那里认领，而 City Hall 没有自己的树，提不出请求。所以分计划的是这栋楼里的一个居民（`beta/planner`），市长那一半——City Hall 把一件事分给几栋楼——U9 不走 | 一栋楼的 `plan` 能认领另一栋楼计划里的叶子时，分计划的那一步改由 `hall/mayor` 来做 |
| 计划的第一行谁写 | 人用编辑器写：新立的楼的 `Roadmap.md` 一行都没有，`plan` 的 `split` 只分已有的行，而门（`Door.lean`）不拼写 `put_spine`。人照模板的六列写下一行，与 U9 改阅览室、改 `review` 那两处是同一种手动作 | 门拼写了 `put_spine` 之后，这一步经它写，断言一字不改 |
| 一次可用性验收还该走什么 | U9 走的是一个人第一天能做的事：城答话、挂上 provider、立楼、准入 skill、派活到结尾、历史自证；进程被杀之后城回来接着干；以及一栋楼里的协作：分计划、认领冲突、重派、评审不通过 | 市长跨楼分活、红队消融与多日小镇属 V0.1.0（G4、G8），各自照 U9 的样子从 justfile 接收替身 |
-/

/-! ## 4 现状分析

Rust 侧的验收测试全部是**具体轨迹**：`crates/sprawling/tests/assembly_door.rs` 证明「这一条路走得通」，不证明「任何一条路都走不出去」。本目录的全部增量在后者，以及三件 Rust 侧写不出的事：

1. **门的承诺只在门外才可观测。** 退出码、两个流的分工、静默与拒绝的区别，都是「一个 agent 拿这个二进制写脚本」时遇到的事实，而进程内的测试永远看不见它们。
2. **敌意的磁盘不是被 mock 的磁盘。** `storage::fault_fs` 是一个确定性掉电模型，它回答「我们设想的坏」；本目录直接翻掉账本里的一位，回答「随便一位坏了会怎样」。
3. **任意前缀与任意后缀。** 只会均匀随机生成的东西是 fuzzer；把攻击命名出来再对它前后做全称量化，才是对手。

### 第一个发现：静默与成功无法区分

**`sprawling call` 在拒绝没赶上静默窗口时退出 0。** 实测：`AttachEndpoint` 指向一个连不上的 base URL，产品侧探测超时远长于客户端默认静默窗口。同一次操作，城自己在诊断日志里写下拒绝，并写下「the refusal above reached nobody」。

**诊断**：城是诚实的——它知道拒绝没送到，并且说了出来。缺陷在门：`main.rs` 的 rustdoc 写着 *"Exits 1 when the city refused something"*，而实际语义是「**在静默窗口内没有拒绝到达**」。对一个拿退出码做分支的 agent，这两者的差别是把一次失败读成一次成功。

**已修（「静默有自己的退出码」）。** Rust 侧选了三档中的第二条：`Spoken` 是一个三支穷尽枚举，`Quiet`（帧发出后窗口内一帧未回）退 **3**，而 0 与 1 的含义一个字不改。表写在 `docs/operating.md` 与 `crates/sprawling/Spec.lean` §8-41 里。本目录的 `Door` 因此把静默解成 `quiet` 而不是 `accepted`（§8），并用 `the exit code says what the frames say` 这条检查把它钉住。

### 第二个发现：一次被拒的派活写进了城里

随机轨迹在早期样本上失败，收缩后得到两步：`Raise "acme"`／`Work "gamma"`（一个从没立过的地址）／`Look`——`Look` 看见 `["acme","gamma"]`，而只有 `acme` 被立过。追下去是同一条缝的两个症状：派活到一个没立过的楼答 `E_MODEL_UNCHOSEN`，而磁盘上留下了 `城根/gamma/one/JOB.md`，账本里一条都没有。

**诊断**：`assembly/dispatching.rs` 的 `dispatch_in` 顺序是「判停摆 → 写 JOB.md → 落 CAS → 解析模型 tag」。它开头那句注释把该守的规矩写得一字不差——*"Nothing is written before the city agrees to take the work"*——**停摆那道门守住了它，配置那道门在写之后才判**。这与 `ARCHITECTURE.md` §5 第 4 条（*every effect becomes an event first*）直接冲突。

**边界已量过，不夸大**：保留子树是守住的。`Work ".sprawling/evil"` 得到 `E_INVALID_ARGS` 且一个字节都没落地，所以这不是写域逃逸，而是「判定晚于副作用」。

**已修。** 向没立过的 `gamma` 派活，城答 `E_MODEL_UNCHOSEN`，城根下仍然只有 `City.md`。`a refusal costs nothing` 那一组两条随之转绿并留着——它们此后守的是那次修复确立的**判定先于副作用**这个次序，而不是当时那一行代码。

### 第三个发现：一把每条命令都必须带、而没有人读的钥匙

线格式要求每一个状态变更命令都带 `IdemKey`，当时的 `gate::dedup` 把这道门实现成一个纯函数，而它在自身模块与自身测试之外**找不到任何调用者**。可观测的后果，全程只经门：同一条 `halt` 带同一把钥匙送两次，账本里留下两条 `city_halted`。

**诊断**：`IdemKey` 存在的理由是让**重试无害**。今天这条保证没有承兑人。`gate::dedup` 自身没有错，错在它没有被接到 `assembly` 的命令路径上。

**已修（「重放的命令只做一次」）。** `accounting::worker::commanding::entrance` 持有那个 `seen` 集合，`RunWorker::serve_one`——wire、控制台与 ACP 三条路唯一的汇合点——在任何副作用之前向 `kernel::idem::claim` 问一次，重复的键得到**第一次的答案**，且不再写第二次。钥匙随命令写进它所产生记录的 payload，开城时那一趟已有的账本折叠把它读回来，故重启之后同一把键仍然认得。
**仍未覆盖的一档**：`run_started` 由 `runtime::run::lifecycle` 直接写账本，装配点碰不到它，故一次跑到一半被进程死亡打断的派活，其钥匙不在账本上——那正是重试应当被允许的一档。`keyUsedTwice` 因此单独成一条检查，不并入任何组：一次红只该点出那一档。

**这条承诺反过来约束模型的记账**：被拒的命令同样花掉了它那把钥匙，故 `minted` 必须在负向动作之后照样前进（`failureNextState`）。若不然，下一条命令带着城已经答过的钥匙，读回来的是**上一条命令**的那份拒绝——看上去像门读帧落后一条，实则是钥匙重了。只有查询不花钥匙，故 `look` 是唯一的例外。

`look` 因此不进随机生成器（`Model.lean` 记了理由）：一条随机轨迹里只要出现一次被拒的派活，其后每一个 `look` 都会为同一个原因失败，那会把一个缺陷报成许多个，并把下一个缺陷藏在它后面。

### 第四个发现：一条 URL 归一化规则，写了但没有调用者

一个 endpoint 的五种拼法经 `ProbeEndpoint` 进去，`endpoint_probed` 里出来的是五个不同的 `base_url`。实测（这台电脑，debug 二进制）：

```
owed: http://127.0.0.1:47199/v1
saw:  http://127.0.0.1:47199/v1/
```

**诊断**：`gateway::normalise_entered` 存在、有自己的 proptest、也有主机预设表，而全仓找不到一个生产调用者：`accounting::worker::commanding::routing` 把帧里的 `base_url` 原样装进 `Entered`。于是算法是对的，城却从来没问过它。这正是“门外才看得见”的那一类缺陷：仓内测试测的是函数，而一个 agent 看的是城写下的那个 URL。

**已修。** `Entered::resolved` 是打字地址变成被调用地址的唯一一处，探测与挂载共用那一次归一化的结果。五种拼法现在落到同一个 `base_url`，实测 37 s（探测那条）与 17 s（挂载那条）。

**挂载那一侧此前没有人问过**，故随修复补了第二条检查：探测回答「这个端点供应什么」，挂载决定「这座城会调用哪里」，只有后者是派活真正走下去的那条路——一个探测归一化了而挂载没有的实现，会在设置页上显示对的端点、把活派到别处。反例渲染成 `every_spelling_of_one_endpoint_is_registered_as_one_url`。

### 第五个发现：事实梯的最后一级没有接在注册上

挂一个 messages 兼容格式的 endpoint，人指名一个目录不认得的模型 id，上限框留空，然后派活。实测两条：

* `model_selected` 的 `max_output_tokens` 是 `null`——注册时没有人问过事实梯。
* 派活当场被拒：`E_CONFIG_INVALID on opus-nine states no output ceiling`。

**诊断**：`gateway::provider::ceiling::OutputCeiling::resolve` 把四档梯子写齐了（人填 > 上游陈述 > 预设表 > 策略缺省），并且有自己的测试；而 `select_model` 走的是另一条链（人填 → 已注册 → 内置目录），没有第四档，也不写 `ceiling_from`。一个事实两个家，其中一个没有调用者。

**已修。** `select_model` 经 `OutputCeiling::resolve` 取上限，并把 `ceiling_from` 写进 `model_selected`；本地那条链删掉了。目录不认得的 id 现在注册着一个数字，梯子的最后一级（`policy`）在账本上署名，派活不再为上限被拒。反例渲染成 `a_model_no_catalogue_prices_is_registered_with_a_ceiling`，它同时钉住那个数字与那个署名。

### 第六个发现：门看得见两条车道，看不见 Inbox

B-24 要钉的是「并发两 run 的审批 id 不相等」。审批项的 id 是 `ap-<run>-<seq>`，故 run 一旦分开，id 就分开；而**一条审批项只由一次工具波中的门抬起来**，工具波要一个会应答的 provider，本目录不许自带一个（§13）。于是这条性质在门外只有一半可观测，本目录只写它能观测的那一半：两条不等第一条做完就派出去的活是两个 run，而两条车道同时追加之后历史仍然是一条链（`two dispatches at once are two runs`，实测 11 s）。

另一半——两个问题在 Inbox 里占两行而不是一行——欠在 `crates/sprawling/tests/e2e.rs`：那里有一个真端点可以问。把它写进本目录就要在本目录里起一个假 provider，而那是本目录被删除的三个理由之一。

### 第七个发现：链的头一格谁也没哈希过

**第七个发现是量出来的，而且它不是缺陷。** 把一条四行账本记为 0…3，逐格改掉中间的一位（每次先把城自己那份字节写回去，所以每次问的都是干净的账本），问 `sprawling replay`：记录 0 到 2 的改动每一次都被拒，改记录 3 时 `replay` 报 `chain verified`（实测：`a change inside record 4 of 4 was believed: the chain still verified, with tail seq 3`）。

**为什么这不奇怪，以及为什么值得写下来。** 每条记录的 `prev` 是上一行的摘要，所以被改的那一格由**它的下一条**作证；最后一条后面没有记录，便没有东西哈希过它——校验的射程正好比文件短一格。这是追加式哈希链本身的样子，不是 `open.rs` 少写了一句：要覆盖头一格，锚必须住在账本**之外**（本仓已经有这样的锚：`storage::bundle` 的清单把链头写进 `head`，`crates/storage/src/bundle/manifest.rs:20,49`），而一个只会读账本目录的 `replay` 看不到它。改动的可观测面还要窄一层：命中引号、花括号这类字节时，拒绝来自解析（`verify_lines:90`）或来自“字节不是写者规范拼写”（`:127`），“被相信”这一档要求改动后的行仍然可解析且仍是规范拼写。

**它对检查的约束有两条，都是本目录自己欠的账。** 一是测**检测**的检查必须落在被覆盖的记录上：`Ground.corrupt` 今天挑最老那条是为了确定性（没有城进程在竞写它），而 `a change to any record but the last is refused` 把整段走完，因为一格通过只说明一格；二是**不许把这一格写成需要修的东西**：谁若把「改一条记录必被拒」写成对所有记录成立，他写的是一条产品不欠的断言，而修它的唯一办法是在文件里放一个自指的摘要——那是伪证，不是校验。

**kernel 的规格 `crates/kernel/spec/Ledger.lean` 把这四件事写成了定理，连同它们各自的价格**：字节 ⇒ 声索是免费的（同余，不假设摘要函数）；声索 ⇒ 字节（头一格除外）要买，价钱是摘函数的单射性，而那条假设写在定理自己的语句里，并有一个反模型（常函数）证明它不省得掉；头一格落在所有这一切之外，且不需要任何假设就能证。

### 第八个发现：分一行没人认领的计划，工具答应了，落地时丢了

协作那一串第一次走时，分计划的 run 只调了一次 `plan` 的 `split`（没有先 `claim`）。工具的回答是成功（`{"children":["wire the kiln","glaze tests"],"node":"1","unfinished":2}`），run 以自己的最后一句结束，而历史在一分钟里没有出现 `roadmap_split`。

**诊断**：一件事两处判。`collab::ClaimDesk::split` 不要求 run 握着那一行，照分不误，并把这次分记成要落地的效果；落地时 `accounting::effect::Claims::of` 用 `collab::still_true` 问盘上的那一行是否仍是效果期待的状态，而 `ClaimEffect::expected_before` 对 `Split` 答的是 `In progress`——一行没人认领的计划是 `Not started`，于是整次落地被判为过时（`Claims::Stale`），不写一行、不改文件，而模型已被告知计划分好了。

**已修。** 「分一行要不要先握着它」现在只由书桌判（collab D6）：`plan` 的 `split` 只分本 run 握着的那一行，没握着就当场以 `E_INVALID_ARGS` 拒绝，恢复语叫它先认领那一行，书桌不排效应；落地不再为拆分另判状态，只核认领，效应按次序重放（`crates/accounting/Spec.lean` §8-27）。被否的另一条路——落地按书桌看到的状态判、让没人认领的一行也能分——会让两个 run 把同一行各分一次，第二组子行在第一组之后不报错地长出来（`crates/collab/spec/Claim.lean` 的 `withoutHold_splits_twice`）。反例在仓内钉住：`a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call`（`crates/collab/src/claim_tool/split_tests.rs`）。修的时候还找到同一类的第二条路：一个 run 分了自己握着的一行、再认领其中一片叶子，旧的落地拿盘上原文核那片叶子，叶子还不存在，整次落地被判过时；钉在 `a_run_that_splits_its_row_and_claims_a_leaf_lands_both`（`crates/accounting/src/effect.rs`）。

U9 的分计划的 run 照旧先认领再分（`Acceptance/Script.lean` 的 `plannerRun`）：那已经不是绕路，而是产品唯一接受的走法。走法里不加一步去看拒绝：拒词与「书桌不排效应」由上面那条仓内测试经生产入口钉住，U9 要看的是一个人第一天能走通的那一串（§3「一次可用性验收还该走什么」）。
-/

/-! ## 5 权威信源

**本节只指位置，不抄数值。** 一个被抄进本文的常量就是同一条规则的第二个权威，而它必然先于产品陈旧：这一点是量出来的——本文曾抄下一个 `WIRE_V`，产品早已走过它许多版，而本文读起来仍然像是对的。

| 事实 | 权威所在 |
|---|---|
| 线格式版本、握手内容 | `crates/wire/src/frames.rs` 的 `WIRE_V` 与 `Welcome` |
| 服务端帧类的全集 | `crates/wire/src/frames.rs` 的 `ServerFrame` |
| Command 与 Query 的全集 | `crates/wire/src/command/kind.rs`、`crates/wire/src/frames/query.rs` |
| 稳定错误码的全集 | `crates/kernel/src/error.rs` 的 `AxCode::ALL` |
| `IdemKey` 与模板名的形状 | 门的拒绝原文，实测 |
| 链的链接规则（每条记录携带的 `prev` 就是上一行的摘要） | `crates/kernel/src/ledger.rs:31` 的 `chain_hash` 与 `:26` 的 `GENESIS_PREV` 决定值；写的一侧在 `crates/storage/src/jsonl/append.rs:74`，读的一侧在 `crates/runtime/src/replay.rs:106-118`、`crates/storage/src/jsonl/open.rs:254` 与 `crates/storage/src/bundle/files.rs:30-40` | 本目录比较读数，从不计算摘要：链的规则由 `crates/kernel/spec/Ledger.lean` 以摘要函数为参数陈述，名字都不提 blake3。仓库改成别的摘要函数时，那里每一条语句一字不变 |
| 摘要函数的**单射性**（碰撞抵抗） | 不是本仓的事实，也不是本目录能证的事实：它是对所依赖摘要函数的假设 | `crates/kernel/spec/Ledger.lean` 把它作为定理假设写在语句里，并用一个反模型（常函数）证明去掉它结论就假；需要一个比它更强的保证的人，从这里知道自己在换什么 |

**门讲六类帧**（`ServerFrame`）。`Frame.lean` 认得全部六类，并对第七类当场报错——一个未知的帧类意味着线格式变了形，而把新形状当成一次拒绝会把红的测成绿的。其中 `log` 只被解析、不被断言：它没有自己的账本序号，两行可以共用一个位置，漏掉一行什么也没丢；它被解析仅仅因为城在这条通道上**也**叙述它的拒绝，而一个读失败报告的人想看到那句话。
-/

/-! ## 6 命名统一

`Address`、`Building`、`Run`、`Ledger`、`Seq`、`Refusal` 一律沿用 `docs/glossary.md` 与 `ARCHITECTURE.md` 的词表，本目录不得另起名字。本目录只新增四个词，各自只指一件事：

| 词 | 它是什么 |
|---|---|
| **Door** | 已构建二进制的 `call` 门面，唯一知道有个可执行文件存在的地方 |
| **Ground** | 一次性的场地：一座被端起来的城、它的端口、它的账本目录，以及磁盘可以施加的敌意 |
| **Trace** | 一串动作及其观察结果，是本目录唯一的断言对象 |
| **putBack** | 把一个账本目录按一份读数写回去：本目录撤掉自己造成的伤，好让同一个场地回答第二个位置的问题 |

第三种世界的模块叫 `Layer`，这个词不是本目录新起的：它就是 `city::config_layers::Layer`——配置梯子上的一级。
-/

/-! ## 7 模块边界

```
src/Sprawling/Frame.lean     线格式的代数镜像。只解析，不判断
src/Sprawling/Door.lean      唯一知道二进制存在的地方
src/Sprawling/Ground.lean    一次性场地，以及磁盘的敌意
src/Sprawling/Check.lean     抽样、收缩、检查树。不知道城是什么
src/Sprawling/Model.lean     状态模型、后置条件、定向对抗场景
src/Sprawling/Provider.lean  挂过 endpoint 的第二种世界：URL 等价类与上限
src/Sprawling/Layer.lean     写过配置的第三种世界：磁盘上的那份与答案里的那份
src/Sprawling/Person.lean    人自己那一层的第四种世界：住在城外的那份与答案里的那份
src/Sprawling/Regression.lean 两个世界的反例 → 一个 Rust 测试文件
src/Sprawling/Acceptance/Script.lean 替身要回放的脚本：从归档的 skills 写出
src/Sprawling/Acceptance/Stage.lean  陌生人的目录，以及他用编辑器改的两份文件
src/Sprawling/Acceptance/Walk.lean   第一天、进程被杀、第二天早上，以及各段共用的步与读法
src/Sprawling/Acceptance/Collaboration.lean 协作那一串：分计划、两次认领、重派、查而不过
src/Sprawling/Acceptance/Servings.lean 一个目录的三次服务，按序：走哪几段、步名的次序
src/Sprawling/Acceptance/Checklist.lean 人手测的清单：每一节从决定它的那一处读出
test/Main.lean               入口与检查树
test/Acceptance.lean         `acceptance` 可执行文件的两个命令：写脚本、走一遍
Spec.lean                    本规格的入口：十七节与决定
spec/Answer.lean             门说了什么：接受、拒绝、静默
spec/Model.lean              欠哪一种拒绝：守序，以及被拒的命令也花掉它的键
spec/Check.lean              收缩的候选更短；一个端口一次只借给一个场地
spec/Acceptance.lean         替身把一个请求放进哪个 run 的哪一条；按序走、停在第一处失败
```

依赖单向：`Model` → `Door` → `Frame`，`Model` → `Ground` → `Door`，`Model` → `Check`，`Provider` → `Ground`，`Layer` → `Ground` 与 `Check`，`Person` → `Layer`，`Regression` → `Model` 与 `Provider`。**`Person` 读 `Layer` 而不自立一套**：两个世界问的是同一件事（一份人也手改的文件与一个折出来的答案会不会分岔），差在那份文件在不在城里；序列生成器、收缩器、“这份读数陈述了某个值吗”那一个子串探针、以及七个互不为子串的四位数，全部只有 `Layer` 一个家。`Layer` 不被 `Regression` 读：它至今没有找到反例，而一条没有反例的性质不向 Rust 侧交付任何东西。**`Regression` 依赖两个世界，因为交付物是一个文件**：轨迹那条测试的每一步与极性从 `Model` 读，供应世界那两条测试的拼法、中转站名字与模型 id 从 `Provider` 读，于是演员表在本目录里仍然只有一个家。`Provider` 不 import `Model`：那是另一种世界，两边共用的只有门与场地。`Frame` 不 import 任何本工程模块；`Check` 也不，且它**不 import `Door`**——抽样与收缩不允许知道有一座城存在。

`Ground` 依赖 `Door` 而不是自己起进程：**「二进制在哪」只允许有一个答案**，而场地要用它做三件事（`init`、`serve`、探活）。

**U9 读 `Provider` 与 `Layer` 而不自立一套**：等一条记录、读整段历史、发一条不许被拒的命令在 `Provider`，城自己那一层配置的路径在 `Layer`，服务一个已经起好的目录在 `Ground.servingAt`（`withGround` 就是它加一个一次性目录）。`Script` 不 import 任何本工程模块：脚本是数据，它只知道替身读的那种线上 JSON。

**检验器不 import 规格，规格也不 import 检验器。** `src/` 与 `test/` 下每一条 `import` 只指向 `Sprawling.*` 或 Lean 工具链自带的库；本文件与 `spec/` 下的分部只 import 工具链与本规格的分部（`xtask spec` 门判两侧，tools/xtask/Spec.lean §8-47）。链与快照的定理在 kernel 与 storage 的规格里（`crates/kernel/spec/Ledger.lean`、`crates/storage/spec/Snapshot.lean`），检验器只在注释里引用它们。一条规则因此只有一处权威：规格陈述它，产品实现它，检验器从门外问产品守没守住。
-/

/-! ## 8 接口先行

检验器的接口如下；它们必须守的性质，凡能写成定理的，在 `spec/` 的分部里以参照定义与定理陈述：`Adversary.Answer`（`classify` 与「静默不满足任何期待」）、`Adversary.Model`（`owedOnRaise`、`owedOnWork` 与三条守序、`spent` 与「被拒的命令也花掉它的键」）、`Adversary.Check`（`removeAt` 与「候选更短」、端口池 `claim`／`release`）、`Adversary.Acceptance`（`place` 与「每个 run 答它自己的下一条」「第一轮按次序开启 run」「追加 run 不改已有的回答」、`firstBroken` 与「报出的那一步失败而之前的都通过」）。

```lean
-- Frame.lean —— 线上说了什么
inductive Frame
  | welcomed (welcome : Welcome) | happened (record : Record)
  | answered (name : String) (body : Json) | refused (complaint : Complaint)
  | streamed (run : String) (text : String) | logged (level : String) (line : String)
structure Complaint where code : Code; action, subject, recovery, retry : String
def decodeFrame  : String → Except String Frame
def cityBuildings : Json → Option (List String)

-- Door.lean —— 怎么问
structure Door where binary : System.FilePath
inductive Answer | accepted (frames : List Frame) | denied (complaint : Complaint) | quiet
def discover     : IO (Option Door)
def Door.raise   : Door → System.FilePath → IO Unit
def Door.serve   : Door → System.FilePath → Port → System.FilePath → IO Serving  -- 城、端口、给城的 home
def Door.ask     : Door → Port → Verb → IO Answer
def Door.verify  : Door → System.FilePath → IO (Except String Nat)
def idemKey      : Nat → IdemKey

-- Ground.lean —— 在哪里问，以及磁盘怎么撒谎
def withGround      : Door → (Ground → IO α) → IO α
def Ground.ledger   : Ground → System.FilePath
def Ground.stored   : Ground → IO (List (String × ByteArray))
def Ground.tree     : Ground → IO (List String)
def Ground.corrupt  : Ground → Nat → IO Unit  -- 翻一位：第 index 条完整记录的中点
  -- `index` 是一个问题，不是细节：测检测挑最老那条（没有城进程竞写它），
  -- 测恢复挑最新那条并改叫 `tear`
def Ground.putBack  : Ground → List (String × ByteArray) → IO Unit  -- 按一份读数把账本写回去
def Ground.tear     : Ground → IO Unit   -- 截尾：最新那段的末 20 字节
def Ground.duplicate : Ground → IO Unit  -- 重放：把最老那条记录再写一遍

-- 三个敌意动作各自回答一件事，不得合并：`corrupt` 问检测，`tear` 问恢复
-- （断尾修复是产品**故意**支持的路径），`duplicate` 回到检测。`putBack`
-- 不是第四个敌意动作：它撤掉本目录自己造成的伤，好让同一个场地回答第二个位置
-- 的问题——上一次的伤还留在文件里，下一次问出的拒绝就会是上一个原因，而那个答案
-- 会被当成关于一条从未被改过的记录的结论。

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

```lean
-- Ground.lean —— 服务一个已经起好的目录；withGround 是它加一个一次性目录
inductive Leaving | killed | closedInOrder        -- 服务结束时城怎样离开（D8）
def Serving.leave : Serving → Leaving → IO Unit  -- 有序关闭不成就结束进程，并印出退回
def servingAt : Door → System.FilePath → System.FilePath → Leaving → (Ground → IO α) → IO α

-- Acceptance/Script.lean —— 替身回放什么
def standInModel : String                          -- 替身列出的唯一模型
def building : String                              -- U9 立的那栋楼
def notesPath : String                             -- 第一个 run 写的文件
def notesText : String
def statusCalls : Nat                              -- 被杀的 run 拿到的只读调用数
def callId  : Nat → Nat → String                   -- 第 run 个 run 第 turn 条回复的调用 id，全脚本唯一
def reviewed : String                              -- 协作那一串的楼
def planner left right : String                    -- 它的三个房间
def planItem : String                              -- 人写下的那一行
def leaves  : List String                          -- 分出来的两片叶子
def firstLeaf secondLeaf : String                 -- 它们的编号 `1.1`、`1.2`
def holdCalls : Nat                               -- 每个认领的 run 认领之后再调几次只读工具
def offeredPath offeredText refusedWhy : String    -- 提出评审的文件，与查它的人说的理由
def runsFor : List String → List (Nat → List Json) -- 按 run 开启的次序；每个 run 拿到自己的序号
def script : List String → Json                    -- 由归档的 skill 名写出整份脚本
def scriptWithChecker : List String → String → Json -- 同一份脚本，后面接上查那条分支的 run（D7）

-- Acceptance/Stage.lean —— 陌生人的目录
structure Stage where root city home : System.FilePath
def Stage.raise   : Door → IO Stage
def Stage.serving : Stage → Door → Leaving → (Ground → IO α) → IO α
def shipped    : System.FilePath → IO (List String) -- 书架上带 SKILL.md 的目录名，排序
def mountShelf : System.FilePath → System.FilePath → IO Unit  -- 把书架写进城那一层
def admit      : System.FilePath → String → List String → IO Unit  -- 改阅览室那一行
def askForReview : System.FilePath → String → IO Unit              -- 把 `review = false` 改成 true
def layPlan    : System.FilePath → String → String → IO Unit       -- 在楼的计划表里写下第一行

-- Acceptance/Walk.lean —— 走什么
structure Setting where door : Door; url : String; shelf : System.FilePath
                        skills : List String; script record : System.FilePath
structure Step where name : String; walk : Ground → IO Unit
def firstDay     : Setting → List Step
def interrupted  : Setting → Stage → IO Unit
def morningAfter : Setting → List Step

-- Acceptance/Collaboration.lean —— 协作那一串，在第三次服务里、第二天早上之后走
def collaboration : Setting → List Step

-- Acceptance/Servings.lean —— 整个一遍
def walkedSteps  : Setting → List String           -- 四段的步名，按走的次序
def walk         : Setting → (Ground → IO α) → IO α  -- 全部通过后在第三次服务里问一次

-- Acceptance/Checklist.lean —— 人手测什么
def routeTable : String                            -- client/src/core/route.ts
def slashTable : String                            -- client/src/core/slash.ts
inductive Takes | written (grammar : String) | computed
def pagesIn    : String → List String              -- `BARE` 的键，去掉空键
def commandsIn : String → List (String × Takes)    -- `SLASH` 每条的拼法与它后面跟什么
def facesIn    : Json → List (String × String × String)  -- known_hosts 的主机、兼容格式、地址
def knownHosts : Door → Ground → IO Json
def render     : Gathered → String
def writeChecklist : Setting → Json → System.FilePath → IO Unit
```

**清单的每一节从决定它的那一处读出，本目录不列任何一项**：页面是客户端 `BARE` 表的键（与 `cargo xtask shots` 读的是同一张表、同一种读法），命令是 `SLASH` 表的拼法，工具是模型第一次请求里拿到的目录（替身的记录），skill 是归档的书架，provider 是城自己对 `known_hosts` 的回答。一份写在这里的名单会是每一项的第二个家，而它会在有人加一页的那天成为错的那一份。`SLASH` 里由别的表算出来的语法（强度的词、页面的名字）记作 `computed`，不在这里求值：求值就是那张表的第二份读法。清单只在走完之后写，第一节把走过的每一步列成已勾选。

`walk` 按序走、停在第一处失败：每一步站在前面几步留下的城上，一栋没立起来的楼没法派活，接着走只会把一个原因报成许多个。每一处断言都从城写下它的地方读回——盘上的文件、历史里的记录、查询的答案、替身记下的请求——不重算任何东西。

```lean
-- Provider.lean —— 挂过 endpoint 的第二种世界
def deadAuthority : String                       -- 这台电脑上没人听的那个地址
def spellings : String → List String             -- 一个 endpoint 的等价类，穷举而非抽样
def unreadable : List String                     -- 本构建读不懂的三种帧
def Ground.text        : Ground → IO String
def Ground.stillText   : Ground → Nat → IO String  -- 等城停笔，不等任何一条记录
def Ground.records     : Ground → IO (List Record)
def Ground.awaiting    : Ground → String → Nat → Nat → IO (List Record)
def Door.send          : Door → Ground → Verb → IO Unit
def nthName      : Nat → String                  -- 等价类第 n 种拼法注册用的名字
def attachedUrl  : String                        -- 这个世界挂载用的那一种拼法
def probedUrls   : Door → Ground → List String → Nat → IO (List String)
def attachedUrls : Door → Ground → List String → IO (List String)
def Ground.runsStarted : Ground → Nat → IO (List String)
def givenAProvider : Door → Ground → Option Nat → IO Record
def statedCeiling  : Record → Option Nat
```

```lean
-- Layer.lean —— 写过配置的第三种世界
def configured     : String                      -- 这个世界写的那栋楼
def buildingLayer  : String                      -- 楼自己那层的 CONFIG.toml
def cityLayer      : String                      -- 它上面那层
def budgets        : List Nat                    -- 互不为子串的七个指令预算
def writeSequence  : Nat → Gen (List Nat)        -- 非空的写序列
def shrinkSequence : List Nat → List (List Nat)  -- 收缩后仍非空
def states         : String → Nat → Bool         -- 这份读数陈述了这个数字吗
def documentText   : Json → Option String       -- 缺失或空的文件读作空串，有正文的读第一个窗口
def Door.readDocument : Door → Ground → String → IO String
def Door.readBudget   : Door → Ground → String → IO (Option Nat)
def Door.writeBudget  : Door → Ground → Nat → Nat → IO (Option String)
def writtenReadsBack  : Door → List Nat → IO Verdict
```

`states` 是子串判断而不是一次解析：TOML 的文法权威在 `city::config_layers`，本目录再写一个读者就是第二个权威（§1）。子串够用的前提写在 `budgets` 里——七个四位数互不为子串，于是「文件还陈述着更早那次写入」这条断言不会被两个数字的包含关系伪造。

`Door.readDocument` 把「这一层没有文件」与空文件读成空字符串，把有正文的文件读成它的第一个窗口，而把任何别的形状抛出去。这条分界是必须的：一个把读不懂的答案也当成空文件的读者，会让下面每一条「文件没有陈述什么」的断言无条件成立。

`ask` 返回 `Answer` 而不是抛异常：被拒绝是产品的正常输出，而**解析失败**才是异常——门的形状变了，检查应当当场停下，而不是把新形状当成一次拒绝。

**黑盒边界由类型划定。** `World` 到 `refusal` 全段没有一个签名提到 `IO`，`Gen` 是种子的纯函数；于是决定「欠哪一种失败」的那一半，和抽取轨迹的那一半，都够不到它们正在审判的那座城。一个能先看后判的模型会按构造与产品一致，那是本目录唯一可能在什么都没检查的情况下报绿的路。

`Action` 以 `Yield` 这个有限标签为索引而不是以结果类型本身为索引：后者会把存在包装推到更高的宇宙，进而把宇宙多态传染给 `Gen`、`shrinkList` 与整棵检查树。以标签为索引，`look` 是唯一答地址表的动作这一点仍由类型保证，而别处一分钱不花。
-/

/-! ## 9 工作流程

1. `just adversary` 先 `cargo build -p sprawling`，把二进制路径经 `SPRAWLING_BIN` 传给检查器。**唯一一处知道二进制在哪的地方是 justfile**。
2. `lake exe adversary` 按树跑：先跑 U5 的渲染对拍（毫秒级，先失败先止损），再跑 U1 的门契约，再跑 U3 的随机轨迹，然后 U4 的定向场景，再是 U6 的历史自洽与磁盘敌意，最后是按缺陷命名的那一组与那一条。
3. 反例出现时，报文给出种子、收缩次数、最小化后的轨迹与破掉的那条承诺。把种子经 `SPRAWLING_SEED` 传回去可以原样复现。
4. 人把最小反例经 `SPRAWLING_ACCEPT=1` 渲染成 Rust 源码，提交到 `crates/sprawling/tests/`。**知识就此迁移到 Rust，本目录不保留它。**
5. `just adversary --select <文字>` 与 `--reject <文字>` 按检查的完整路径筛选。它们为红之后的那几分钟而存：单条检查几秒就重跑完，而整棵树要几分钟。定时任务不用它们——它把整棵树一次跑完，因为树里每一条都是**必须通过**的。
6. `just acceptance <archive>`：把归档解进 `target/acceptance/`，构建替身与 `acceptance`；`lake exe acceptance script <书架> <脚本>` 按归档的 `skills/` 写出脚本；配方起替身，从它印出的第一行读 `SPRAWLING_PROVIDER`；`lake exe acceptance walk <书架> <脚本> <记录> <清单>` 在 `SPRAWLING_BIN` 指着归档里的二进制时走完四段：
   - **第一天**（一次服务）：城答出它起城时的那栋 hall；替身被挂上、它的模型被选中；立一栋楼并被列出；人在楼的阅览室里准入每一件 skill；派活跑到脚本给的结尾并在盘上留下文件；run 钉住的 skill 恰是书架上的那些，按名读到的每一件以它自己的正文到达模型；模型拿到的目录里有脚本调用的每件工具；城列出的楼恰是历史创建过的楼；历史自证。
   - **进程被杀**（第二次服务）：派活，等那个 run 写下几条工具结果，然后结束进程。

   第一次与第三次服务结束时城被有序关闭（D8，`Leaving.closedInOrder`）：macOS 与 Linux 上送 `SIGINT`，等进程自己退出；Windows 上、或十秒内没退出时，退回到结束进程，并在输出里印一行 `note` 说出退回与原因。只有第二次服务是被杀的。
   - **第二天早上**（第三次服务）：被杀的城留下的历史自证；城再服务，新派的活跑到它自己的结尾；历史再自证。
   - **协作**（仍是第三次服务）：立第二栋楼 `beta`，人把它的 `review` 改成 true、在它的计划表里写下一行；`beta/planner` 认领那一行并把它分成两片叶子（分一行要先握着它，collab D6；§4 第八个发现）；`beta/left` 与 `beta/right` 同时派活、都去认第一片叶子，历史里只有一条认领；活重派到 `beta/left`，它认下第二片叶子、写一个文件、提出评审，文件不在城里；检查从历史读出那条请求的分支，把查它的 run 接到脚本后面（D7），派给 `beta/right`，它判不通过，历史里有一条以同一个分支、同一句理由被拒的记录，文件仍不在城里；历史再自证；最后问城 `known_hosts`。

   全部通过后，`walk` 把清单写到 `target/acceptance/` 下的 `checklist.md`（§8 `Acceptance/Checklist.lean`）。

   配方在结束时停掉替身，无论走没走完。
-/

/-! ## 10 实现逻辑

**门**：`IO.Process.output` 在等待之前把两个管道读空——这是手写版本必须记住的死锁：一个没被读的管道会让子进程活着，于是先等待就会在输出超过一个缓冲区时把两边挂住。退出码 0／1／3 都要读 stdout；退出码 2 是 `sprawling call` 的命令行被拒，而帧是命令行的一部分——客户端在开套接字之前先解析帧。所以退出码 2 按帧的作者分两种读法：从 `Verb` 编码出来的帧本该可读，被拒就是本目录自己的错，抛出；为了被拒而手写的帧（`askRaw`，如 `put_secret` 与读不懂的三种帧），stderr 上带 `E_` 码的那行拒绝就是答复。

**慢路径的动词从历史里读回答案。** `ProbeEndpoint` 与 `AttachEndpoint` 各开一条到 provider 的 socket，而门要等满静默窗口才返回：在连接上等它们的记录，等于在一次往返之外再付一个窗口（实测：窗口 20 s 时一次探测花 32 s）。所以这两个动词发出去后，**拒绝仍然从 socket 上读**，而它们产生的记录从账本里读（`Ground.awaiting`）。等的是「数量到了」而不是「内容对了」，因此不预测任何一条记录；这是对下一段那条规则的唯一例外，而它把「静默」读成它本来的意思——城还没完工，而不是城接受了。

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
| `work` 而未停摆且无 provider | 一定被拒，且 `E_MODEL_UNCHOSEN` |
| `batch`（一条拼得出、执行不了的帧） | 一定被拒，且 `E_WIRE_MISMATCH`，且 `recovery` 非空 |
| `look` | 答里的楼集合恰是模型记的那一套 |
| 每一次拒绝 | `recovery` 非空——三段式承诺的第三段 |

**三条守序（guard order）被显式钉住**，因为它们是用户看得见的差别：停摆压过配置（`E_GATE_DENIED` 而不是 `E_MODEL_UNCHOSEN`），地址良构压过占用，以及——**对派活而言**——停摆压过地址良构。一个把前两条调换了的实现会在城停摆时叫人去挂 provider——恢复建议指向一件与真实原因无关的事。

第三条是**量出来的，不是想出来的**，而且它纠正的是模型而不是产品。四次直接测量说明产品是自洽的：

| 条件 | 码 |
|---|---|
| 派活，未停摆，保留地址 | `E_INVALID_ARGS` |
| 派活，未停摆，普通地址 | `E_MODEL_UNCHOSEN` |
| 派活，已停摆，保留地址 | `E_GATE_DENIED` |
| 立楼，已停摆，保留地址 | `E_INVALID_ARGS` |

停摆是**派活**这件事最外层的那道门，而立楼不是派活，所以停摆盖不住它。两个次序都有道理，产品选了其中一个并且到处一致；模型断言一个它从未量过的次序，那是模型在发明规则。

**定向对抗**（U4）：先用一个具体动作把世界推到「有一栋楼」，再抽任意前缀，插入那一次 `Stop City`，再抽任意后缀，最后在**读回模型状态确认仍然停摆之后**补上一次派活。那一步的极性同样由 `refusal` 决定，于是「它必须失败、且必须以 `E_GATE_DENIED` 失败」是模型自己的断言，而不是此处另写的第二条。后缀里可能出现 `resume`，所以那一次读回是必须的：断言一个产品不欠的拒绝，就是本目录在发明规则。

**「拒绝」不是「期望输出」**。模型断言的是**哪一种失败**，从不断言任何被计算出来的值：错误码由 `AGENTS.md` 定为门的契约的一部分，钉住它钉的是产品对调用方的承诺，不是对某条规则的重算。

D5 由 D6 取代。

D6 **验收世界在协作那一串里并发派活，仍走到第一处失败就停；它取代 D5。** 替身按 run 分开作答（citysim D11）：一个续轮由它带回的调用 id 放进它自己的 run，与别的 run 怎么交错无关（`spec/Acceptance.lean` 的 `every_run_is_answered_from_its_own_replies`）；第一轮按到达次序开启脚本里的下一个 run（`openings_take_the_runs_in_order`）。所以只有第一轮可能同时到达的 run 要写成一样（`alike_runs_answer_alike`）——两个同时派出的认领正是这样：两个 run 都去认同一片叶子，谁先到替身都答同一条，谁拿到叶子由城的认领决定，而那正是这一步要看的。其余的活仍等前一个 run 冻结再派，第一轮的次序就是脚本的次序。被杀的 run 不再与之后的活分一段回复：城回来之后若接着送它的对话，它续自己的那一段，新派的活开启自己的那一段。每一步站在前面几步留下的城上，一处失败之后接着走只会把一个原因报成许多个，所以仍停在第一处失败（`the_reported_step_broke_and_every_earlier_one_held`）。被否：D5 的做法——一次只派一个 run，并发认领就走不了，认领冲突只有白盒的 `crates/accounting/src/worker/plans/tests/rows.rs` 守着；给两个认领的 run 写不同的回复——它们第一轮同时到达，替身分不出谁是谁。

D7 **城才知道的东西，检查从历史里读出来，再把要用它的 run 接到脚本后面。** 查一条请求要说出它的分支，分支名由城按房间地址的摘要取，而本目录不预测任何摘要（§2 第 3 条）。所以 `beta/left` 提出评审之后，检查从 `pr_opened` 读出分支，把整份脚本连同查它的那个 run 写回脚本文件，再派活给 `beta/right`；替身在新开的 run 找不到还没开启的 run 时重读这个文件（citysim D15），接上的 run 不改动已经在答的那些（`a_grown_script_answers_the_runs_it_held_alike`）。写回时没有别的 run 在开启：前一个 run 已经冻结，下一个活还没派。被否：把分支名写死在脚本里——那是在预测一个摘要；让替身从上一次工具结果里抄出分支——替身就在写自己的文字（citysim D11）；用 `pr list` 让模型自己看——脚本写好的回复不会读它拿到的结果。

D8 **U9 只在第二次服务里杀城；第一次与第三次服务按人在键盘前的做法有序关闭，关不了才杀，并说出来。** 有序关闭让城写下交接、进程正常退出，于是交接那条路径被走到，而插桩的发行件（`just pgo-train`）只有正常退出才写出 profile：一个全被杀的走查对 PGO 一份都不贡献。macOS 与 Linux 上是对子进程 `kill -s INT <pid>`；之后每 100 毫秒问一次是否退出，十秒为限。Windows 上城只认它自己控制台上的 Ctrl-Break，而 Lean 的 `IO.Process.spawn` 不能让城另起一个进程组（`SpawnArgs.setsid` 在 POSIX 之外不起作用），在共享的控制台上发 Ctrl-Break 会连同走查本身与它上面的 `lake`、`just` 一起关掉；所以 Windows 上直接退回到结束进程，`note` 一行写明原因。这一条在 `Door.serve` 能把城起在自己的进程组里时重开（例如经一个用 `CommandExt::creation_flags` 设 `CREATE_NEW_PROCESS_GROUP` 的启动器，再向那个组发 Ctrl-Break）。退回只印一行而不报红：U9 判的是一个人第一天的路径，城怎样被这个检查器关掉不在那条路径上。被否：三次服务都有序关闭——第二次服务要的正是一次崩溃；关不了就报红——Windows 上每一跑都会红在检查器自己的缺口上。
-/

/-! ## 11 边界枚举

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
| 替身答一个 run 时拒了（`no_run_left`、`script_exhausted`、`runs_crossed`） | 城把这次模型调用读成一次失败，那个 run 不以自己的最后一句结束，等它的那一步报红并说出它怎样结束；替身的记录里那一行的 `run` 是 `null`，读记录的人从那里看出是哪一种 |
| U9 没有 `SPRAWLING_BIN` 或 `SPRAWLING_PROVIDER` | 抛出并说明该经 `just acceptance <archive>` 运行。与检查树不同，U9 不跳过：人点名了一个归档，一跑什么都没走的绿就是在报一次没发生的验收 |
| 归档的书架上一件 skill 都没有 | `acceptance script` 拒绝写脚本：一次什么都不钉、什么都不读的验收会让 skill 那一步平凡成立 |
| 人要改的那一行不在模板里 | `admit` 拒绝而不追加：第二个 `reading_room` 键会让城以本目录造成的原因拒这份文件；`mountShelf` 对已经有 `[skills]` 的配置同理 |
-/

/-! ## 12 错误处理

D1 断言不成立就是发现，不是一个要恢复的错误。

本目录没有「恢复」这个概念：断言不成立就是发现，发现就该停下并交付一个 Rust 回归测试。三件被当作错误处理的事：**门的形状变了**（JSON 解析失败、未知帧类）、**场地起不来**（端口或进程）、**用法错**（从 `Verb` 编码的帧得到退出码 2）。它们都抛异常，因为继续跑只会把新形状当成拒绝，从而把红的测成绿的。

被否：遇到读不懂的形状时记一条警告接着跑——那会把一个变了形的门当成一次拒绝，把红的测成绿的。
-/

/-! ## 13 依赖选型

D2 **一个外部依赖都不引入。** 仓库根 `lake-manifest.json` 的 `"packages": []` 是这条的机器形式。

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

D3 **一个会应答的 provider 从 justfile 接收，不在本目录里起。** U9 要一座调用成功的城，替身是 `tools/citysim` 的 `provider` 二进制（`tools/citysim/Spec.lean` §8-10），由 `just acceptance` 起在本目录之外，URL 经 `SPRAWLING_PROVIDER` 交进来——与二进制的路径经 `SPRAWLING_BIN` 交进来是同一个形状。一个接收来的 URL 不让本目录变成第二个 gateway 实现：替身用城自己的翻译读脚本里的每一条回复，HTTP 那一面是 citysim 的，本目录只写脚本，而脚本是 U9 把期待写成数据。**被否**：在 Lean 里起一个 HTTP 服务端——本目录会多出一个 provider 的实现，并为一个注册表之外的协议栈负责；用 Rust 写一个起子进程的测试去驱动发行件——从门外进城的检查是 Lean 的（`xtask boundary` 守着的那条边界），换成 Rust 就是把黑盒写回白盒的那一侧。
-/

/-! ## 14 硬编码声明

| 硬编码 | 意图 | 后续影响 |
|---|---|---|
| 账本布局 `.sprawling/ledger/*.jsonl` | 敌意动作要按它找到文件 | 该布局改变时本目录报错，属预期 |
| 端口 47100–47115 | 避开常用段，又不需要网络库依赖；整棵树串行跑，十六个是给外部占用留的余地 | 冲突时报环境问题 |
| 静默窗口 250 ms | 模型驱动的每个动词都答在 1 ms 内（实测），250 ms 是三个数量级的余量 | 模型开始走慢路径（挂 endpoint）时必须同步改 |
| 种子 `<!-- xtask:begin adversary_seed -->20260912<!-- xtask:end -->` | 本地运行必须可复现：一次反例只有在它能被重跑时才值得渲染成 Rust 测试。**`SPRAWLING_SEED` 读成三态**：给了且能解成数、没给、给了但解不成。第三态以退码 `2` 在起城之前停住，而不是静默换成默认值：拼错的种子不是关于产品的证据，而报告里那句复现命令必须是这一跑真用过的那一个（B-81）。每一跑开头打印 `seed <值>` | 定时任务经 `SPRAWLING_SEED` 用会变的种子，于是「每晚探索新轨迹」与「本地可复现」各得其所。**这一格的数是受管区段**：`cargo xtask docnum` 每次都去 `test/Main.lean` 的 `defaultSeed` 重读一遍，两处不同即红，改法只有 `cargo xtask docnum --write` |
| 人那一层的路径 `<home>/.sprawling/config.toml` | 第四种世界要按它找到那份磁盘上的读数，而它不在任何一座城里，`document` 查询读不到 | 布局改变时本目录报错，属预期 |
| 环境变量 `USERPROFILE` 与 `HOME` | 被服务的城按这两个变量找家目录，两个都指向一次性目录；**只覆写其中一个就把答案交给了平台**，而一跑对抗不得动跑它的人自己的偏好 | 产品改读家目录的方式时同步改 `Door.serve` |
| 指令预算 `7001` 等七个四位数 | 写回性质靠子串判断问文件陈述了什么，互不为子串是这一判断成立的前提 | 加值时要保持该性质；否则「旧值还在文件里」会被包含关系伪造 |
| 两个层的路径 一个楼自己的 `.sprawling/CONFIG.toml` 与 `.sprawling/CONFIG.toml` | 写回性质要按层分别读 | 布局改变时本目录报错，属预期 |
| 三个地址 `acme` / `beta` / `gamma` | 固定的演员表，让反例可读 | 加人时同步改 `Regression.lean` 的模板 |
| 模板 `minimal` | 两个模板里不带保密约束的那个 | 要测 `confidential` 时它进模型 |
| 环境变量 `SPRAWLING_BIN` | 二进制位置的唯一入口 | 由 justfile 提供 |
| 地址 `127.0.0.1:47199` | 第二种世界指向的地方：这台电脑上没人听、在端口池（47100–47115）之外，于是一次探测永远碰不到本套件自己的城 | 外面的进程占住它时，探测的读数变成另一个程序的 |
| 演员表 `relay` / `opus-nine` | 一个中转站，一个内置目录与预设表都不认得的模型 id——事实梯最后一级正是为这一格而存在 | 预设表有了这个 id 的行时，换一个它不认得的 |
| 渲染出的一行 `attach(...)` 不折行 | 最长的那种拼法仍在 rustfmt 的宽度以内，于是渲染器写一行、rustfmt 不动它 | 换一个更长的地址会让 rustfmt 折行，逐字节对拍当场报红——这正是它该报的 |
| 记录预算 240 × 250 ms | 一条命令等自己那条记录的上限。只在城真的还在干活时花掉；一次探测在 debug 二进制上的主要开销是构造 HTTP 客户端，不是那次被拒的连接 | 探测变快后可以调小；调小前要先量 |
| 替身的模型 `stand-in-1` 与 U9 的楼 `acme` | 一个替身列出的 id，一栋演员表里的楼，让一次红读起来与本目录别处的报告一样 | —— |
| `holdCalls` 60 | 两个认领的 run 同时派出，但各自先等城给它放一棵树；先认领的那个要在另一个认领时仍在跑，第二次认领才由在途的 run 拒，而不是由一个已经回家的 run 放过去（`crates/accounting/src/worker/booking.rs`：预订只持续到 run 回家）。实测（debug 二进制）两个第一轮相邻到达替身，认领随即跟上；六十次让先认领的那个再跑约两秒，给一次晚到的放树留余地 | 放树变快或变慢时跟着量；第二次认领若落在第一个 run 回家之后，那一步报红并点名这个数 |
| `statusCalls` 120、`inFlight` 3 | 被杀的 run 要在被杀时仍在调用：一次调用在 debug 二进制上约十毫秒，120 次是一秒多的在途，等到 3 条工具结果再杀；它的最后一条是一句收尾的话，城回来时若接着送它的对话，它以自己的那一句结束 | 发布二进制更快；被杀的 run 若在被杀前就冻结，那一步报红并点名这个数 |
| 幂等键 400–411 | U9 每条命令一把，与检查树的 0–323 不相交，一份报告里不会有两条命令共用一个数 | —— |
| 调用 id `call-<run>-<turn>` | 替身靠它认 run（citysim D11），所以全脚本唯一；带上 run 的序号，一份记录里的 id 读得出是哪个 run 的 | 替身认 id 的办法变了时，这里跟着变 |
| 楼 `beta`，房间 `beta/planner`、`beta/left`、`beta/right`，计划的一行 `glaze the kiln` 与两片叶子 `1.1`、`1.2` | 演员表里的第二栋楼；叶子的编号是计划表的文法（`crates/city/templates/Roadmap.md`：分出来的子行从父行往下编号），一个人读计划时看到的就是它 | 计划表的编号规则变了时，认领那两步报红 |
| 计划文件 `Roadmap.md`、表头下的分隔行以 `|---` 开头、`review = false` 那一行 | 人用编辑器改的两处，照模板的字样找；找不到恰好一处就拒，不追加 | 模板变了时这两步报红，并说出找的是哪一行 |
| 目录 `target/acceptance/` | 解开的归档、脚本、替身的记录、清单都在这里，配方每次先清空它 | 由 justfile 提供 |
| 客户端的两张表 `client/src/core/route.ts`、`client/src/core/slash.ts`，以及它们的开头行 `const BARE`、`export const SLASH` | 清单的页面与命令两节从这里读；`just acceptance` 在仓库根运行，路径相对于根 | 表搬家或改了开头行时，那一节读成空，清单把空节写成一行要人先查原因的条目，而不是一个空标题 |
-/

/-! ## 15 影响面

对 Rust 侧的影响**必须**恰好为零：不改 `Cargo.toml` 的 members，不进 `cargo deny` 的依赖图，不参与 `xtask length` 的行数，不进 `xtask modmap` 的模块表。唯一的交汇点是 `crates/sprawling/tests/from_adversary.rs`——它由本目录渲染、由 `cargo test` 编译，两侧任何一方漂移都会让某一侧变红。

`xtask` 的扫描不进 `.lake`（`walk::SKIP_DIRS`），因为那是构建产物，而一道门为已提交的对象作证。`xtask release` 拒绝「引用了一台机器自己的文件的文件」，`xtask header` 要求每个 `.rs` 带 MPL 抬头；本目录不含 `.rs`，且只引用仓内相对路径与环境变量名，两道门都不适用。
-/

/-! ## 16 测试与约束

按「坏得越早越省时间」排序：渲染对拍（U5，毫秒级）、门的契约三条（U1，含 §4 那条退出码性质）、人填进去的四条（U7：探测的等价类、挂载的等价类、幂等、读不懂的帧）、挂过 provider 的三条（U7：注册带上限、派活不为上限被拒、两条车道是两个 run）、配置写回（U8，实测 26 s）、随机轨迹（U3）、定向停摆（U4）、账本自洽（U6）、磁盘的四句（U2，含 §4 第七个发现之后补上的那一条：改动落在被覆盖的每一格上）、最后是按缺陷命名的那一组与那一条。

**U8 同样被演示过咬得动**（§2 第 5 条）：把“答案等于最后一次写入”改成“等于第一次写入”后，该条报错，收缩 1 次得到两步反例 `[6556, 9223]`；恢复后转绿。**磁盘那一条同样被演示过**（§4 第七个发现）：把被改的那一格从「除最后一条之外」改成「包括最后一条」，该条报错并指名 `a change inside record 4 of 4 was believed: the chain still verified, with tail seq 3`；改回来转绿。它咬得住的是产品欠的那件事——被覆盖的改动必被读出——而不仅仅是「改一位就会红」。U7 那七条各自实测为 7–37 s（debug 二进制，四核 Windows），其中的时间几乎全在城构造 HTTP 客户端上；其余十三条一整套 2 min 35 s（实测，四核 Windows，热缓存）；同一棵树在两核 Linux 上 51 s，差别在起进程的价钱而不在核数。**新增的那一条**（`a change to any record but the last is refused`）单独实测 18.2 s 首跑、2.5 s 暖盘（同机）：它贵在每问一次都起一个 `replay` 子进程，而不在计算。约束是 §2 第 5 条——**咬得动**必须被演示过，而不是被相信。

U9 不在这棵树里，它由 `just acceptance` 单独跑：第一天约 1.8 s，被杀那一步约 0.75 s，第二天早上约 2 s（debug 二进制，同一台四核 Windows）；协作那一串约 3.4 s，其中两个认领的 run 1.3 s，多半花在 `holdCalls` 上（debug 二进制，十六核 Windows）。

**全新机器上的 U9 还没有读数。** `.github/workflows/on-demand.yml` 的 `fresh` 作业在 windows、macos、ubuntu 三个全新 runner 上各走一遍：打出发行归档，解进一个别的东西都没写过的目录，先在只有系统目录与归档目录的 PATH 上跑 `sprawling doctor`、记下它的退出码，再跑 `just acceptance <archive>`；runner 的账号有管理员权限，于是「没有管理员权限」只由那条窄 PATH 与空的 HOME 模拟。第一次运行里三行都停在 doctor 那一步（windows 那一行报告账号是提升过的管理员）：步骤写的是 `set -uo pipefail`，而 `shell: bash` 本来就带 `-e`，doctor 报出缺一件开发工具、以非零退出时，步骤在记下退出码之前就结束了，`just acceptance` 一步都没走。作业现在写 `set +e`；能定下这一条的是下一次运行里每一行的 `checklist.md` 与 `doctor.exit`。

**树里没有一条是被期待失败的。** 一条因为预期会红而被留下的检查，教会每一个看到它的人把红当成常态，于是下一个真的发现落进一次没人读的运行里。

曾经红的那三条现在是绿的，两处修复都在 `crates/` 下（§4 第四、第五个发现）。当时留着它们而不是摘掉，理由在期限：被期待失败的检查，是没有修复日期的那一条；那三条点名了要改的那一处，红只活到修复落地为止。

D4 **整棵树串行跑。** 一座被端起来的城占着一个端口、一个目录与一条历史，两组同时跑就三样都争。实测过的后果不是变慢而是**换城**：输的那一边城绑不上端口退了出去，它自己的探活却在同一个口上接到了赢的那一边的城，于是一整条轨迹跑在别人的历史上。它把当时还开着的那个缺陷测成了绿的——一个答案取决于哪个线程赢了的对手，比没有对手更坏。

**一条检查失败不中止整棵树。** 检验器存在的理由是把每一条破掉的承诺都报出来，停在第一条会把其余的藏在它后面。

**证明的与检查的，分开说。** `spec/` 的分部由 `just models` 证明，没有 `sorry`、`admit`、`axiom`；它们证的是参照定义的性质，不是检验器那几行 Lean 的性质——检验器不 import 规格，规格也不 import 检验器（§7）。检验器与参照定义一致，由从门外跑的两件事作证：`just adversary` 的树（门契约、守序、磁盘、钥匙）与 `just acceptance <archive>` 的那一遍（被杀之后的活跑到它自己的结尾）。一处参照定义与检验器分岔，红出现在这两次运行里，而不在证明里。
-/

/-! ## 17 文档关系

1. 本文件与 `spec/` 下的分部。
2. `ARCHITECTURE.md` §11 的验证层表——本目录是 V9 之外的一层，记为 V10，并写明它不是门。
3. `AGENTS.md` 的命令表（`just adversary` 一行）与边界那一节。
4. `justfile` 的 `adversary` 配方。
5. `.gitignore` 的 `/.lake`（Lean 包在仓库根），以及 `tools/xtask/src/walk.rs` 的 `SKIP_DIRS`。
6. `.github/workflows/adversary.yml`——定时任务，永远不进 `check`。
7. 仓库根的 `lakefile.toml`：检验器的三个目标在那里定义（`Sprawling` 库、`adversary` 与 `acceptance` 两个可执行文件），本规格经 `Spec` 库的两个 glob 进 `just models`。
9. `tools/xtask/Spec.lean` §8-47：`spec` 门怎样认出本规格。
8. `justfile` 的 `acceptance` 配方，以及 `tools/citysim/Spec.lean` §8-10、§8-13 与 citysim D11、D15：替身的脚本格式、它印出的那一行、它怎样按 run 作答与怎样接上后写的 run。
-/
