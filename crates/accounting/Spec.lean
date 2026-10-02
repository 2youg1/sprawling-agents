-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.accounting.spec.Clock
import crates.accounting.spec.Connectors
import crates.accounting.spec.Effect
import crates.accounting.spec.Guide
import crates.accounting.spec.HeldVault
import crates.accounting.spec.Home
import crates.accounting.spec.Machine
import crates.accounting.spec.Models
import crates.accounting.spec.Person
import crates.accounting.spec.PlanView
import crates.accounting.spec.Playback
import crates.accounting.spec.Playback.Check
import crates.accounting.spec.Playback.Project
import crates.accounting.spec.Playback.Select
import crates.accounting.spec.Playback.Traced
import crates.accounting.spec.Trace
import crates.accounting.spec.Views
import crates.accounting.spec.Views.Answering.Github
import crates.accounting.spec.Views.Document
import crates.accounting.spec.Views.Rounds
import crates.accounting.spec.Views.Snapshot
import crates.accounting.spec.Worker
import crates.accounting.spec.Worker.Attend
import crates.accounting.spec.Worker.Commanding.Saving
import crates.accounting.spec.Worker.Freezing.Naming
import crates.accounting.spec.Worker.Genesis.Lost
import crates.accounting.spec.Worker.Workbench.Tools

/-! # accounting 的规格

`sprawling-accounting`（库名 `accounting`，目录 `crates/accounting`）是 accounting thread 的家：城的唯一写者，它向外伸手时经过的端口，以及页面问的每个问题从折叠里怎样答。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。决定写作 `D<n>`，放在它所管主题的分部里，别处引作 `accounting D<n>`；D26、D38、D40、D43、D44 是空号，§12 末尾列出其余每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：效应先成为账本行、计划的效应按次序重放（`spec/Effect.lean`）、计划的投影与盘上一致（`spec/PlanView.lean`）、只有表里的命令配方问得到端口（`spec/Machine.lean`）、导入 GitHub 身份时视图只判主机（`spec/Views/Answering/Github.lean`）、记账线程的循环（`spec/Worker/Attend.lean`）、playback 的选择与投影（`spec/Playback/Select.lean`、`spec/Playback/Project.lean`）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

本 crate 存在的理由是 ARCHITECTURE.md §11 记下的 V6 缺口：`RunWorker` 自己造模型适配器而不是接收一个，所以 citysim 的脚本只能复现一次 run，复现不了一次 dispatch。把写者移进一个只经端口向外伸手的 crate，端口的第二实现就能从外面把它驱动起来。

| 模块 | 这个模块回答的问题 | 分部 |
|---|---|---|
| `models` | 一次 run 拿什么模型适配器去说话 | `spec/Models.lean` |
| `connectors` | 一栋楼配置里写的 MCP server，怎样连上并变成工具 | `spec/Connectors.lean` |
| `clock` | 现在几点 | `spec/Clock.lean` |
| `machine` | city 所在的主机上有哪些工具，以及装上一个人同意过的那一个 | `spec/Machine.lean` |
| `effect` | 一张桌子留下的效应，怎样先成为账本行、再成为这座城 | `spec/Effect.lean` |
| `plan_view` | 每栋楼的计划，折叠记录而只在记录点到它时重读 | `spec/PlanView.lean` |
| `home` | 这个人的家目录在哪，本产品在它下面放什么 | `spec/Home.lean` |
| `person` | 这个人自己定下的那一层：偏好与核心线程的档位 | `spec/Person.lean` |
| `held_vault`、`toolkit_broker` | 一个锁着的 vault 怎样变成解析器；一个外部应用的 broker 钥匙登记在哪 | `spec/HeldVault.lean` |
| `guide` | 上手指南的进度 | `spec/Guide.lean` |
| `views`、`lineage` | 页面问的每个问题，怎样从折叠的记录里答；每个 run 怎样折成一行 | `spec/Views.lean` 与 `spec/Views/` 下的分部 |
| `worker` | 城的唯一写者 `RunWorker`：它持有的状态、它执行的命令、它驱动的 run | `spec/Worker.lean` 与 `spec/Worker/` 下的分部 |
| `playback` | 一段历史怎样成为一份可以重算、可以复核的 playback bundle；一个 playback page 怎样分五项查过，导出件怎样整份落下；按 UTC 时间选一段，调用的耗时何时是量出来的，一个提交改了什么、出自哪几次调用 | `spec/Playback.lean` 与 `spec/Playback/` 下的分部 |
| `trace` | 一个提交是这个 run 哪几次调用的结果，同一栋楼里还有谁在那一段里调用过工具 | `spec/Trace.lean` |

`RunWorker` 与它的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例住 `worker`；它经构造时收下的一个 `Hands` 碰这台电脑，生产的那一份由二进制的装配根 `bin::assembly::production::hands` 造出（§8-11）。
-/

/-! ## 2 验收标准

九条断言。前三条在 `crates/sprawling/tests/model_factory.rs`，都驱动一个由 `bin::assembly::hands` 造出、再接收脚本 `ModelFactory` 的 `accounting::worker::RunWorker`：

| 测试 | 它钉住的事 |
|---|---|
| `a_dispatch_reaches_the_model_the_worker_was_handed` | run 的模型走端口。端点指向一个拒绝连接的 loopback 端口，所以一个绕过端口、自己造适配器的 worker 碰不到脚本模型，历史里也就没有脚本写下的那句回答。 |
| `an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone` | 没有 session 的 dispatch 按规则从任务的词里取房间名（`crates/sprawling/Spec.lean` §8-86），不做命名调用：工厂只被问到 run 的主模型，run 在规则给出的房间里开始。 |
| `a_confidential_building_refuses_before_the_factory_is_asked` | 机密楼的拒绝在 worker 的选择里，不在工厂里：端点不是 loopback 地址时 dispatch 以 `GateDenied` 被拒，脚本工厂一次也没被问到。换掉工厂不能绕开机密。 |

第四条在 `crates/sprawling/tests/connectors.rs`：`a_run_is_offered_the_tools_the_worker_was_handed`。楼的配置写了一个 MCP server，它的命令在任何主机上都不存在；worker 接收了一个脚本 `Connectors`，它给出一个工具。一个绕过端口、自己启动 server 的 worker 起不来这个 server，模型收到的工具表与休眠索引里也就都没有那个工具。

第五条在 `crates/sprawling/tests/clock.rs`：`a_worker_stamps_its_lines_with_the_clock_it_was_handed`。worker 接收一个停在固定时刻的脚本 `Clock` 之后写下的每一行，`t` 都是那个时刻；一个自己读墙钟的写点写下的 `t` 是现在的时间。

第六条在 `crates/sprawling/tests/machine.rs`：`a_refresh_counts_the_items_the_machine_it_was_handed_answered`。worker 接收一个脚本 `Machine`，它的回答只有一个条目；`DoctorRefresh` 之后 worker 在诊断里报的条目数是 1。一个自己去问主机的 worker 报的是需求表的全部条目数。测试从 `DoctorRefresh` 进而不从 `DoctorInstall` 进：对着一个绕过端口自己动手的 worker，后者会在宿主上真的启动包管理器。

第七至第九条是验收覆盖，在 `crates/sprawling/tests/acceptance/`，一个测试二进制（nextest 过滤器 `binary(acceptance)`）。它们同样驱动一个由 `bin::assembly::hands` 造出、再接收脚本 `ModelFactory` 的 `accounting::worker::RunWorker`；脚本只替模型说话，工作台、效果层、检查点、账本与书架都是生产件。

| 测试 | 它钉住的事 |
|---|---|
| `every_tool_a_builder_is_offered_is_called_and_answered` | 一栋 `Builds` 楼里的 run，第一次请求收到的每一件工具都被脚本调用一次；每条 `tool_called` 恰有一条同 id 的 `tool_result`；每件工具的回答与它自己的 SPEC 一致，逐件的判定在 `episodes.rs`。该调用哪些工具不写名单，取模型收到的工具表加上休眠索引里的工具行（`crates/runtime/Spec.lean` §8-60），`describe` 与 `call` 本身各由脚本调一次：工作台多登记一件工具而脚本里没有它的一段，这条测试点名那件工具变红。 |
| `every_tool_city_hall_is_offered_is_called_and_answered` | 同一条验收标准，对 `Plans` 楼（市政厅）。多出的 `city` 和 `rules` 一样在效果层被拒（`crates/city/Spec.lean` §8-2b、§8-23）；判定看的是调用之后城根下的目录与楼的 `RULES.toml` 一字未变，不钉拒绝码。 |
| `every_shipped_skill_a_building_admits_is_read_by_name_and_pinned` | 仓库 `skills/` 下每个技能包经 `city::install_skill` 装进城库、由楼的阅览室按名准入之后：`run_started.skills` 按目录顺序列出每一件，哈希等于装入时 `Installed::holding` 报告的 `SKILL.md` 哈希（整包哈希是 CAS 的键，答的是另一个问题，`crates/city/Spec.lean` §8-28）；`read <名>` 交给模型的就是那份 `SKILL.md`；包内附属文件按 `<名>/<相对路径>` 读得到。技能集合取 `skills/` 目录本身，不另写名单。 |

城外工具（浏览器、MCP、桌面）不在这张覆盖表里：它们经端口交进来的路由各有一条测试（`browsers.rs`、`connectors.rs`、`desktop.rs`），在真实浏览器与真实 MCP server 上的行为不由白盒判定。

分部里的定理是模型对性质的证明：

- `spec/Effect.lean`：交出 `Then` 时每一行都已按次序落下，有一行没落下就不交出（`then_comes_after_every_line`、`a_refused_line_withholds_then`）；落下的文本就是按次序重放的结果，只有认领会对不上，拆了自己的行再认领其中一片叶子两条都落下（`landed_text_is_the_ordered_replay`、`only_a_claim_is_stale`、`a_run_that_splits_its_row_and_claims_a_leaf_lands_both`）。
- `spec/PlanView.lean`：盘上只在被点到的楼变时，投影答的就是盘上此刻的计划（`apply_keeps_coherence`、`of_answers_the_disk`、`of_keeps_coherence`），不把工具波当作点到楼的投影答旧计划（`a_view_that_ignores_tool_waves_answers_the_old_plan`）。
- `spec/Machine.lean`：一个 `Runnable` 只来自命令配方，端口只被问到表里有、配方是命令的名字（`a_runnable_comes_from_a_command`、`the_port_is_asked_only_for_a_listed_command`）。
- `spec/Views/Answering/Github.lean`：不是主机名的串到不了 `gh`，没人交读者的视图答 `Unavailable`（`a_string_that_is_no_host_never_reaches_gh`、`views_nobody_served_answer_unavailable`）。
- `spec/Worker/Attend.lean`：每条消息在有限步内被服务、追加次序即 seq 次序、不忙等，证明之前与链断之后不写。
- `spec/Playback/Select.lean` 与 `spec/Playback/Project.lean`：选择是交集、cutoff 不读未来、读不到的行不流进派生表、真实关闭单调、量过的调用才有耗时、提交的证据不读 cutoff 之后的行。

每个模型都带一个可实现的正常路径（两行落下再交出变化、一条命令配方问得到端口、不给主机时问 `github.com`、一份非空的选择），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! D23 验收覆盖从模型收到的工具表与休眠索引的工具行算出应当调用的集合；城外工具不进这张表；效果层拒绝的工具按「没有东西变」判定；技能经城库装入

理由：哪些工具存在，唯一的权威是工作台的那一次登记（`crates/sprawling/Spec.lean` §8-27），模型第一次请求里的工具表加上休眠索引的工具行就是它的输出（不常驻的工具只在索引里留一行，`crates/runtime/Spec.lean` §8-60）。测试若照抄一份名单，下一次加工具时名单会悄悄漏掉那一件；从工具表算，漏掉的那件会被点名。城外工具的集合随楼的配置与主机而变，放进来就要在测试里配一台浏览器或一台 MCP server，而它们的路由已经各有一条端口测试。`rules` 与 `city` 声明 `Effect::Govern`，拒绝码取决于工具是否给出自己的 `subject`（`crates/city/Spec.lean` §8-2b 写了两种拒词）；钉住拒绝码，补上 `subject` 的那次改动就会打红验收，而验收要守的规矩——run 不改写审判它的规则、不立楼——在那次改动前后都成立。技能经 `city::install_skill` 装进城库：那条路把一个包的每个文件落在城内，所以验收连包里附属文件的按名读取一起判；城外书架上的一件由 catalog 携着正文交给 run（`crates/runtime/Spec.lean` §8-29-6），它的验收是另一条测试，判的是同一条阅览室、catalog、`read` 与 `SkillPin` 的链。被否决的做法：手写工具名单再逐件断言（第二个权威）；把城外工具一并覆盖（重复端口测试，并让验收依赖主机）；按拒绝码断言效果层的拒绝（钉死一个 SPEC 已说明会变的细节）。
-/

/-! ## 3 假设与歧义

- citysim 有一个场景经脚本的 `Hands`（计数时钟、内存里的 vault）与 `ModelFactory` 驱动一次 dispatch：`tests/proposal_baseline.rs`（citysim D22）。它判的是结局，不判同一场景跑两遍账本逐字节相同，所以 ARCHITECTURE.md §11 的 V6 缺口还差这一步；能定下它的证据是那个场景在 citysim 里逐字节重放。
- `views::mcp_health` 自己用 `agent_protocols::McpLink` 启动一个 MCP server 去问它的健康，不经 `Connectors`。未定的是这次读要不要也经端口：`views` 搬进本 crate 时它照原样搬（`agent_protocols` 本来就是本 crate 的依赖）；能定下它的证据是一个脚本场景需不需要回答 MCP 健康查询。
- `playback` 的导出在 40 万行的城上要 27 s，分段计时里约七成是投影对 cutoff 以内每一行做的 credential 扫描（`kernel::secret::scan`，每行约 60 µs，在 `opt-level = "z"` 下量；读数见 §8-25）。范围外的行也要扫，因为配对表（`links`）、`hidden_calls` 与 `policies` 都读一行对读者是否可见。未定的是走哪条路：让 `kernel::secret::scan` 本身变快（它是「什么像凭证」的唯一权威，每个扫描者都受益），还是让投影只扫可见性会被某张表读到的行（判定从一处变成两处）。能定下它的证据是 `kernel::secret::scan` 在这类行上的单行读数：降到几 µs，导出就只剩逐行核对的 5.5 s 与视图折叠，投影不必拆开判定。
- `playback`（§8-12、§8-13）的两个上限是待测初值：`BUNDLE_MAX_BYTES` 与 `PAGE_MAX_BYTES` 要在多日夹具上量过导出峰值、页面解析与首屏成本才定值，定值的证据是 citysim 的多日场景读数。居民导出位置（§8-13）里崩溃留下的暂存文件 `<名>.partial-<pid>` 没有人收走：它不会被当成导出件读（`check` 只认 `.json`/`.html` 的名字），能定下要不要收的是这类文件在真实城里出现的频率。
- **本 crate 有一百九十五个模块的接口仍写在 sprawling 的规格里。** 模块从 `sprawling` 搬过来时，它在 `crates/sprawling/Spec.lean` 里的那一节留在原处，只把模块路径改成新的拼写（`bin::views::x` 写作 `accounting::views::x`，`bin::assembly::x` 写作 `accounting::worker::x`）。`architecture.toml` 里这些行指向本 crate 的分部（`spec/Views.lean`、`spec/Views/Snapshot.lean`、`spec/Views/Rounds.lean`、`spec/PlanView.lean`、`spec/Worker.lean`、`spec/Worker/Attend.lean`、`spec/Worker/Workbench/Tools.lean`），每个分部末尾一张表按 sprawling 的标签列出它们；搬法见 D15。
-/

/-! ## 4 现状分析

本 crate 有四个端口。`ModelFactory` 的生产适配器是 `accounting::worker::models::GatewayModels`，`Connectors` 的是 `accounting::worker::mcp::Residents`，两者都由 `RunWorker` 的构造器装上（D18）；`Clock` 的生产适配器是 `bin::assembly::production::SystemClock`，经 `Hands.clock` 交进来；`Machine` 的是 `bin::doctor::ThisMachine`，经 `Hands.machine` 交进来。生产的 `Hands` 只由 `bin::assembly::production::hands` 一处造出（§8-11）。四个端口的第二实现依次在 `crates/sprawling/tests/model_factory.rs`、`connectors.rs`、`clock.rs`、`machine.rs`。

生产消费者是 `crates/sprawling`：装配根造 `Hands`、起写者线程与视图折叠线程，服务面与 CLI 经 §8-10、§8-11 列出的 `pub` 面读本 crate；citysim 经同一组端口驱动一次 dispatch（§3）。
-/

/-! ## 5 权威信源

ARCHITECTURE.md §3（依赖律与 `depmap`）、§4（端口表）、§11（V6 缺口）；`gateway::adapter_for`（一个 `Chosen` 怎样变成一个适配器）。
-/

/-! ## 6 命名统一

ModelFactory｜Connectors｜Clock｜Machine｜Recipe｜Runnable｜accounting thread｜Chosen｜Redemption。「适配器」专指 `kernel::Model` 的一个实现；「端口」专指本 crate 声明、外层实现的 trait。

Lean 里的名字与 Rust 的对应：

- `Accounting.Effect.record` ↔ `Landing::record`；`Plan`、`replay`、`Claims.Landed`／`Claims.Stale` ↔ `Claims::of` 与 `Claims` 的两臂，`Plan.holds`／`Plan.apply` ↔ `collab::still_true`、`ClaimEffect::apply`。
- `Accounting.PlanView.View.apply`／`View.of` ↔ `PlanView::apply`、`PlanView::of`，`moved` ↔ `plan_view::reach` 的判定。
- `Accounting.Machine.Recipe`／`Runnable`／`Recipe.command` 与 Rust 同名；`doctorInstall` ↔ `doctor_install` 在问端口之前的两步。
- `Accounting.Views.Answering.Github.isHost`／`githubAnswer`／`defaultHost` ↔ `is_host`、`github_answer`、`DEFAULT_HOST`。
-/

/-! ## 7 模块边界

- 怎样按 endpoint、dialect、凭据造出一个适配器，归 `gateway::adapter_for`：本 crate 只声明「造一个」这个动作。
- 挑哪个模型（`EndpointBook::select`）与何时续期凭据，归 `RunWorker`：端口拿到的是已经选好的 `Chosen` 与已经兑换好的 `Redemption`。
- 把生产适配器接到 worker 上，归装配根 `bin::assembly`。
- MCP 的生命周期（`initialize`、`notifications/initialized`、`tools/list`）怎样说，归 `agent_protocols`；一个工具能不能在机密楼里存在，归 `agent_protocols::McpTool::new`。端口只声明「连上一个 server，交回它的工具」。
- 机密楼根本不启动 server，这一步在 worker 的 `mcp_tools` 里、端口被问到之前。
- 主机上有什么、每一项怎样判定、怎样折叠成一页，归 `bin::doctor`（city 所在主机有什么，它是唯一权威）；一条安装配方能不能跑，归 `Recipe::command`；worker 只决定一个名字在不在需求表里、这个平台有没有配方。

**worker 用到的每个 `bin` 模块归哪一边。** 规则是 D9：worker 的决定与读面搬进本 crate；通往主机、网络或终端的做法留在 `sprawling`，经一个端口交进来。

| `bin` 里的东西 | 归属 | worker 或 `views` 用它做什么 | 依据 |
|---|---|---|---|
| `views` | 搬进本 crate | `Views`、`Published`、`Governance`、`pursued`、`snapshot::start` | worker 的读面；`Governance` 由读侧拥有，写侧从那里取用（`crates/sprawling/Spec.lean` §8-92） |
| `lineage` | 搬进本 crate | `sprawling view` 的 run 列表，以及 playback 的共享投影 | 它只折记录，与 `views` 同形（形状 7）；二进制的 `main::view` 与本 crate 的读面都够得到它 |
| `home` | 搬进本 crate | 阅览室与 `views::skills` 取这个人的家目录 | 只读一个环境变量、拼路径，不启动任何东西；`person` 与 `views` 都从它取路径 |
| `person` | 搬进本 crate | `PutPreferences` 写、`Preferences` 查询读 | 人的那一层是一份文件，读写它和读写城的文件同类，不是主机的能力 |
| `serving::standing::CorePriority` | 随 `person` 搬进本 crate | 偏好里核心线程抬不抬高的那个值 | 它是 `person` 读出来的值；真去抬高线程的 `raise_this_thread` 留在 `serving` |
| `held_vault` | 搬进本 crate | 把一个锁着的 vault 变成解析器，锁中毒时的拒绝 | 纯函数，只碰已经打开的 vault |
| `toolkit_broker` | 搬进本 crate | 一个外部应用的 broker 钥匙登记在哪 | 纯函数，`views::toolkits` 与连接动作读同一组事实 |
| `doctor`（`REQUIREMENTS`、`Platform`、`host`、`Presence`、`PATIENCE`、`ThisMachine`） | 经端口：看与装经 `Machine`，需求表的查法经 `RunWorker.recipe_for`（`crates/sprawling/Spec.lean` 中 `doctor_install` 那一节） | 需求表查找、执行引擎的路径 | 主机上有什么，`bin::doctor` 是唯一权威（本节上文）；`views::lines::harnesses_answer` 找一条命令的程序经 `Views.programs`，生产交的是 `bin::doctor::host::find_program`（§8-10）；exec 的主机半经 `Hands.exec_host`（§8-11） |
| `monitor::memory::read`、`monitor::volume::read` | 经 `Hands`：`read_memory` 与 `read_volume`，都是 `fn` 指针（`crates/sprawling/Spec.lean` §8-46-3、§8-94；§8-11） | 新工作进门时读内存与卷的余量 | 读主机的计数器；读数的类型 `Memory` 与记账线程的计数 `Health` 随 worker 搬进本 crate，读数的做法留在 `bin::monitor` |
| `revealing` | 经端口：`RunWorker` 的 `reveal` 字段，一个 `fn` 指针（`crates/sprawling/Spec.lean` §8-60） | `Reveal` 在主机的文件管理器里打开一个地址 | 启动主机的一个程序 |
| `browser_tool` | 经端口：`RunWorker::with_browsers` 交进来的 `fn` 指针（`crates/sprawling/Spec.lean` §8-45-2） | 按楼的规则给 run 的浏览器工具 | 启动浏览器，经 BiDi 说话 |
| `release` | 经端口：`Views.registry`，一个由 `views::served` 放进来的 `fn` 指针（`crates/sprawling/Spec.lean` 中 `Views.machine` 旁的那一条） | `views` 回答 `NewestRelease` 查询 | 向 npm 注册表发请求 |
| `console` | 留在装配根 | — | 只有 `listening` 用它；它是终端，不是 worker |
| `serving` 的其余部分（`folding`、`output_ring`、`Serving`、`open_vault`） | 留在装配根 | — | 只有 `attending`、`listening` 与 `production` 用它们 |
| `assembly` 的 `production`（`SystemClock`、`hands`、`init_city`、`form_city`）、`listening`、`attending` 的起线程那一半（`spawn_worker`）、`chain_watch` 的起审计线程那一半、`dropping` | 留在装配根 | — | 起线程、绑端口、造生产的手：D17、D18 |
| `models::GatewayModels`、`mcp::Residents` | 随 worker 搬进本 crate | 新 worker 默认的模型工厂与 MCP 连接表 | 它们只经 `gateway` 与 `agent_protocols` 伸手，而 worker 本来就经这两个 crate 探端点、读 MCP 健康（D18） |

workspace 内的依赖由 ARCHITECTURE.md §3 的 `depmap` 定；规格只 import 工具链的库与本 crate 的分部。
-/

/-! D9 worker 的决定与读面搬进本 crate，通往主机、网络或终端的做法留在 `sprawling`、经端口交进来（§7 归属表）

理由：端口正是 citysim 插第二实现的地方；把一个适配器搬进来，它碰主机的那一步就跟着进了 citysim 驱动的 crate，脚本场景会真的起浏览器、读内存、打开文件管理器。被否决的做法：全部搬进来——本 crate 就要依赖 `thread-priority`、`sysinfo`、浏览器与终端，citysim 换不掉其中任何一个；把 `views` 留在 `sprawling`、经端口交给 worker——`Governance` 由读侧拥有，写侧在写下记录之前就要同步地从它作决定（`crates/sprawling/Spec.lean` §8-92），端口会把一个 trait 放到决定路径上，并把一份折叠的权威分到两个 crate。
-/

/-! D8 `effect` 与 `plan_view` 先于 `RunWorker` 搬进本 crate

理由：它们只依赖 `kernel`、`city` 与 `collab`，不碰 §7 归属表里的任何一个 `bin` 模块，搬它们不需要新端口，而 worker 搬过来时它们必须已经在这里。被否决的做法：等 worker 整体搬迁时一起搬——那一次改动就同时背着机械的搬移与端口设计，审的人分不开两者。
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/accounting/spec/Models.lean` |
| 8-2 | `crates/accounting/spec/Connectors.lean` |
| 8-3 | `crates/accounting/spec/Clock.lean` |
| 8-4 | `crates/accounting/spec/Machine.lean` |
| 8-5 | `crates/accounting/spec/Effect.lean` |
| 8-6 | `crates/accounting/spec/PlanView.lean` |
| 8-7 | `crates/accounting/spec/Home.lean` |
| 8-8 | `crates/accounting/spec/Person.lean` |
| 8-9 | `crates/accounting/spec/HeldVault.lean` |
| 8-10 | `crates/accounting/spec/Views.lean` |
| 8-11 | `crates/accounting/spec/Worker.lean` |
| 8-12 | `crates/accounting/spec/Playback.lean` |
| 8-13 | `crates/accounting/spec/Playback/Check.lean` |
| 8-15 | `crates/accounting/spec/Worker/Freezing/Naming.lean` |
| 8-16 | `crates/accounting/spec/Trace.lean` |
| 8-17 | `crates/accounting/spec/Playback/Traced.lean` |
| 8-18-1 | `crates/accounting/spec/Worker/Genesis/Lost.lean` |
| 8-18-2 | `crates/accounting/spec/Guide.lean` |
| 8-18-3 | `crates/accounting/spec/Views/Answering/Github.lean` |
| 8-19 | `crates/accounting/spec/Views/Snapshot.lean` |
| 8-20 | `crates/accounting/spec/Worker/Workbench/Tools.lean` |
| 8-21 | `crates/accounting/spec/Views/Document.lean` |
| 8-22 | `crates/accounting/spec/Worker/Commanding/Saving.lean` |
| 8-23 | `crates/accounting/spec/Views/Document.lean` |
| 8-24 | `crates/accounting/spec/Views/Snapshot.lean` |
| 8-25 | `crates/accounting/spec/Playback/Traced.lean` |
| 8-27 | `crates/accounting/spec/Effect.lean` |
| 8-29 | `crates/accounting/spec/Views/Document.lean` |
| 8-30 | `crates/accounting/spec/Worker/Workbench/Tools.lean` |
| 8-33 | `crates/accounting/spec/Views/Rounds.lean` |

§8-18 分成三小节，各住规定它的那个模块的分部：§8-18-1、§8-18-2、§8-18-3。`spec/Playback/Select.lean`、`spec/Playback/Project.lean` 与 `spec/Worker/Attend.lean` 没有标签：前两个是 §8-12、§8-17、§8-25 的性质，后一个是记账线程循环的模型。
-/

/-! D15 sprawling 规格里写本 crate 模块的节，等两份规格都在 Lean 里再搬

**决定**：模块从 `sprawling` 搬过来时，它在 sprawling 规格里的那一节留在原处，只改模块路径的拼写。本 crate 迁到 `Spec.lean` 时，`architecture.toml` 里这些模块的行指向本 crate 的分部，分部末尾的表按 sprawling 的标签把读者带到那一节（§3）；这些节等 sprawling 的规格也迁到 Lean 之后，按模块搬进本 crate 的分部，一节一个改动，标签照搬或在表里记下新旧对应。

**理由**：两份 Markdown 时，搬一次、迁 Lean 时再改写一次，是两遍约两千行的重写；两份规格的节号相撞（sprawling 的 8-3、8-6、8-17、8-20 与本规格的同名节），sprawling 的规格自己也有重号，逐节搬要先重新编号，而代码与文档里引用 sprawling 那几节的地方都得跟着改。本 crate 与 sprawling 同期迁 Lean，一边搬出、一边整份改写，同一段文字会在两个变更集里各成一份；先让两边各自迁完，搬的那一步就是 Lean 到 Lean、标签已经各自唯一的一次移动。

**被否**：①逐节搬进本 crate 的 Markdown 规格（几十节，节号相撞）；②本 crate 迁 Lean 时同时把约三千三百行的那些节一起搬来（与 sprawling 的迁移改同一段文字，合并时要在两份 Lean 里删重）。

**重开参数**：sprawling 的规格迁到 Lean 之后，这一条变成搬的那一步本身。
-/

/-! ## 9 工作流程

一次 dispatch 从装配根走到账本：服务面把 `wire::Command` 交给 `accounting::worker::attend` 的收件队列；记账线程判定、写下 `run_started` 之前的每一步不写账本（`crates/sprawling/Spec.lean` §8-40「先判定后动手」）；lane 线程在 `accounting::worker::pool` 里驾驶 run，经 `relay` 写回；run 结束后桌子的效应经 `effect::Landing::record` 先成为账本行，再成为这座城（§8-5）。读的一侧：视图折叠线程把每一行交给 `Views::apply`，查询在锁内取小数据、在锁外读盘（§8-10）。
-/

/-! ## 10 实现逻辑

每个模块的参照定义与取舍写在它的分部里：端口为什么交整页、交整条连接（D4–D7）；`Hands` 为什么是一个值（D20）；效应的先后为什么是类型的性质（§8-5）；playback 为什么逐字节携带账本行、复核靠重算（D24、D25）。

代价的读数随它所在的节：快照的克隆与编码（§8-19，40 万行夹具城上克隆 22 ms、编码再解码 350 ms）；playback 在 40 万行的城上的导出（§8-25，约 27 s，其中约七成是 credential 扫描，§3）；提交证据随提交数线性增长（§8-25 的 `instrument_evidence_cost`）。复杂度的计量都按账本行数与区间长度，不按墙钟；墙钟只进 `tools/xtask/budgets.toml`。
-/

/-! ## 11 边界枚举

各分部列出它的边界：效应的一行被拒（§8-5）、认领在重放中途对不上（§8-27）、计划文件读不了（§8-6）、家目录不在（§8-7）、人层文件读不懂（§8-8）、vault 的锁中毒（§8-9）、没人 serve 的视图（§8-10）、快照解不开（§8-19）、死掉的 run 只写过 `run_started`（§8-18-1）、不是主机名的串（§8-18-3）、矛盾的时间区间与 `t` 回退的行（§8-17）、cutoff 之后坏掉的历史（§8-25）、版本不在内容库（§8-21、§8-23）、提案引文出现零次或多次（§8-30）。
-/

/-! ## 12 错误处理

本 crate 不另造错误码：端口原样传 `gateway`、`agent_protocols` 与 `bin::doctor` 的 `AxError`（§8-1、§8-2、§8-4），构造器原样传账本、CAS、`city` 与 `Standing::fold` 的 `AxError`（§8-11）。每个模块的拒绝、它的码与恢复语写在规定它的那一节里；失败之后什么保持不变是接口的一部分：`Landing::record` 一行被拒时变化不交出（§8-5，`spec/Effect.lean`），`Claims::of` 过时时计划一字不写（§8-27），`person::put` 读不懂文件时不覆写（§8-8），被拒的保存账上没有它（§8-22），`playback::export` 不交回部分的 bundle、`land` 不留半成品（§8-12、§8-13），`check` 不返回错误、每一项把读不下去的原因写在自己的结论里（§8-13）。

决定的条目与它们住的地方：

| 决定 | 住处 |
|---|---|
| D1 | `crates/accounting/spec/Models.lean` |
| D2 | `crates/accounting/spec/Models.lean` |
| D3 | `crates/accounting/spec/Models.lean` |
| D4 | `crates/accounting/spec/Connectors.lean` |
| D5 | `crates/accounting/spec/Connectors.lean` |
| D6 | `crates/accounting/spec/Machine.lean` |
| D7 | `crates/accounting/spec/Machine.lean` |
| D8 | `crates/accounting/Spec.lean` |
| D9 | `crates/accounting/Spec.lean` |
| D10 | `crates/accounting/spec/Worker.lean` |
| D11 | `crates/accounting/spec/Worker.lean` |
| D12 | `crates/accounting/spec/Worker.lean` |
| D13 | `crates/accounting/spec/Views.lean` |
| D14 | `crates/accounting/spec/Views.lean` |
| D15 | `crates/accounting/Spec.lean` |
| D16 | `crates/accounting/spec/Views.lean` |
| D17 | `crates/accounting/spec/Worker.lean` |
| D18 | `crates/accounting/spec/Worker.lean` |
| D19 | `crates/accounting/spec/Worker.lean` |
| D20 | `crates/accounting/spec/Worker.lean` |
| D21 | `crates/accounting/spec/Worker.lean` |
| D22 | `crates/accounting/spec/Worker.lean` |
| D23 | `crates/accounting/Spec.lean` |
| D24 | `crates/accounting/spec/Playback.lean` |
| D25 | `crates/accounting/spec/Playback/Check.lean` |
| D27 | `crates/accounting/spec/Worker/Freezing/Naming.lean` |
| D28 | `crates/accounting/spec/Trace.lean` |
| D29 | `crates/accounting/spec/Playback/Traced.lean` |
| D30 | `crates/accounting/spec/Worker/Genesis/Lost.lean` |
| D31 | `crates/accounting/spec/Views/Snapshot.lean` |
| D32 | `crates/accounting/spec/Worker/Workbench/Tools.lean` |
| D33 | `crates/accounting/spec/Views/Document.lean` |
| D34 | `crates/accounting/spec/Worker/Commanding/Saving.lean` |
| D35 | `crates/accounting/spec/Views/Document.lean` |
| D36 | `crates/accounting/spec/Views/Snapshot.lean` |
| D37 | `crates/accounting/spec/Playback/Traced.lean` |
| D39 | `crates/accounting/spec/Effect.lean` |
| D41 | `crates/accounting/spec/Views/Document.lean` |
| D42 | `crates/accounting/spec/Worker/Workbench/Tools.lean` |
| D45 | `crates/accounting/spec/Views/Rounds.lean` |
-/

/-! ## 13 依赖选型

workspace 内的依赖由 ARCHITECTURE.md §3 的 `depmap` 定，每一条边的理由写在用到它的那一节：`gateway`（§8-1）、`agent_protocols`（§8-2）、`wire`（§8-4）、`city` 与 `collab`（§8-5）、`storage` 与 `runtime`（§8-10、D14）。外部依赖都在 workspace 钉版：`html5ever` 与 `cssparser`（D25 (a)、(b)），`git2`（D25 (h)、D34 (b)），`blake3` 经 `kernel::B3Hash`。

规格本身只 import 工具链的库与本 crate 的分部。
-/

/-! ## 14 硬编码声明

每个固定值写在定义它的那一节，连同它的来源与改它的后果：`SCHEMA`、`PROJECTION_RULES`、`BUNDLE_MAX_BYTES`（§8-12）、`PAGE_MAX_BYTES`、`BUNDLE_BLOCK`（§8-13）、`DIFF_MAX_BYTES`（§8-17）、`VIEWS_FOLD_RULES`、`STANDING_FOLD_RULES`（§8-24，由夹具摘要进位）、`DEFAULT_HOST`（§8-18-3）、`CITY_DIR` 与 `NO_HOME`（§8-7）。两个上限是待测的初值（§3）。
-/

/-! ## 15 影响面

改端口的签名，波及 `crates/sprawling` 的装配根（`bin::assembly::production`）与四个端口测试（§4）；改 `views` 的公开面，波及服务面、CLI 与视图折叠线程；改 bundle 的形状，`SCHEMA` 与 `PROJECTION_RULES` 一起进位（D29 (f)），波及 `skills/playback/` 的参考页面；改快照里出现的类型，`VIEWS_FOLD_RULES` 由夹具摘要进位（§8-24）。
-/

/-! ## 16 测试与约束

§2 的九条断言与各分部「验收」一段列出的模块测试是生产实现与模型的对应，它们是行为比对，不是精化证明：

- 效应：`accounting::worker::driving::tests::ledger` 的 `what_a_run_changes_is_changed_after_the_line_that_announces_it`、`accounting::effect::tests` 的 `a_run_that_splits_its_row_and_claims_a_leaf_lands_both`。
- 计划的投影：`accounting::plan_view::tests`。
- 主机与安装：`crates/sprawling/tests/machine.rs`、`accounting::worker::commanding` 的安装测试。
- GitHub：`views::answering::github` 的 `a_github_login_is_asked_of_the_reader_the_views_were_handed`。
- playback：`accounting::playback::tests::model` 从 `spec/Playback/Select.lean` 逐行读出场景表，跑生产的 `export`（D25 (i)）。

形式化的义务由证明清偿：`lake build crates.accounting.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。没有 Lean 模型的分部，其要求由类型与 `cargo nextest run -p sprawling-accounting` 的各模块测试守住。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；§3 的 `depmap` 与 §4 的端口表：本 crate 的依赖与端口。它们改了，分部的路径、`architecture.toml` 里 accounting 各行的 `spec` 锚点与 §7 一起重看。
- `architecture.toml` 的模块图：accounting 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词（accounting thread、Ledger、playback bundle 等），`cargo xtask gates lexicon` 检查。
- sprawling 的规格（`crates/sprawling/Spec.lean`）：装配根、CLI 与服务面怎样调本 crate，以及 §3 那一百九十五个模块的接口；D15 说那些节怎样搬进来。
- 别的 crate 的规格：`crates/kernel/Spec.lean`（事件、地址、读界）、`crates/wire/Spec.lean`（查询与命令的答）、`crates/city/Spec.lean`（文档、楼规、归档）、`crates/collab/Spec.lean`（认领）、`crates/documents/Spec.lean`（窗口、预览、提案）、`crates/storage/Spec.lean`（账本索引与内容库）、`crates/runtime/Spec.lean`（读界与回合）。它们改了，引用它们的那一节一起重看。
- `skills/playback/SKILL.md`：playback page 的数据契约与验证要求，与 §8-12、§8-13 同期改。
- 引本规格的其他规格与 rustdoc 写 `crates/accounting/Spec.lean §8-n` 或 `accounting D<n>`；本 crate 的 rustdoc 写规定它的分部。一节换了分部，它的标签不变，引用不必改。
-/
