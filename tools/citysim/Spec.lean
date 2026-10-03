-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import tools.citysim.spec.Ablation
import tools.citysim.spec.Bench
import tools.citysim.spec.BenchStartup
import tools.citysim.spec.Executor
import tools.citysim.spec.LongTurn
import tools.citysim.spec.MemLedger
import tools.citysim.spec.Metabolism
import tools.citysim.spec.Nesting
import tools.citysim.spec.RedTeam
import tools.citysim.spec.Suite
import tools.citysim.spec.Throughput
import tools.citysim.spec.WireScript
import tools.citysim.spec.WireScript.Exchange

/-! # citysim 的规格

`citysim`（目录 `tools/citysim`）是 dev-only 的工作区成员、第二个 Main，不占产品拓扑：内存 Ledger 与链检查器、剧本设施与薄执行器、真适配器换入、两个测量二进制、评估仪器与替身 provider。

本文件是规格的入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」，命名空间是 `Citysim.<Path>`。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `tools/citysim/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那一节注释里，别处引作 `citysim D<n>`。D1 到 D15 沿用这份规格在 Markdown 时 §3 里「决定」条目的号，D12、D14 空着不复用；D16 起是那时散在 §8 各节里、没有编号的决定；§12 列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：计数时钟、场景只在回合边界说话与一次工具调用的键（`spec/Executor.lean`）、内存 Ledger 的接法（`spec/MemLedger.lean`）、替身按 run 作答（`spec/WireScript.lean`）与它怎样接上后写的 run、凭据怎样不落盘（`spec/WireScript/Exchange.lean`）、长回合的门为什么咬得住（`spec/LongTurn.lean`）、红队两臂（`spec/RedTeam.lean`）、suite 的计数（`spec/Suite.lean`）、资产的处置次序（`spec/Metabolism.lean`）、夹具的钉子（`spec/Bench.lean`）与读数不说假话的两件（`spec/BenchStartup.lean`）。`spec/Nesting.lean` 与 `spec/Ablation.lean` 只有节注释：它们的规则由 Rust 的穷尽枚举与各自的测试守住（§16）。
-/

/-! ## 1 需求分解

| 件 | 一句话 | 分部 |
|---|---|---|
| `mem_ledger` | kernel Ledger 的第二适配器：全内存、确定性、conformance 的对照实现 | `spec/MemLedger.lean` |
| `checker` | 不变量检查器：链完整且 seq 连续（`check_chain`，复用 `runtime::replay::verify_lines`） | `spec/MemLedger.lean` |
| `script_model` | kernel Model 的第二适配器：脚本驱动的 ModelReturn 序列，确定性 | `spec/Executor.lean` |
| `script_tools` | kernel Tool 的第二适配器：脚本工具加按名分发，全失败模式可注入 | `spec/Executor.lean` |
| `executor` | 薄执行器：把剧本世界（计数时钟、剧本中断、检查点网、工具台）交给 `runtime::run::drive`，驱动到 `run_frozen` | `spec/Executor.lean` |
| `sieving` | 场景里的 `exec` 结果经筛子打包（`SieveWorld`） | `spec/Executor.lean` |
| `red_team` | 有无验证 run 两臂的结论质量（§8-7） | `spec/RedTeam.lean` |
| `suite`、`score`、`metabolism`、`nesting`、`ablation` | 评估仪器（§8-8）；除 `suite` 外只在测试下编译 | `spec/Suite.lean`、`spec/Metabolism.lean`、`spec/Nesting.lean`、`spec/Ablation.lean` |
| `bin/bench`、`fixture_digest` | 负载场景与读数行（§8-6），两族 bench 共用的夹具摘要 | `spec/Bench.lean` |
| `bin/bench_startup` | 四个动作的冷启动测量与首字节（§8-5） | `spec/BenchStartup.lean` |
| 吞吐台（仪表住 `sprawling-accounting`） | N 个并发 run 的吞吐与等待读数（§8-14） | `spec/Throughput.lean` |
| `long_turn`、`bin/long_turn` | 长回合：一个 run 连续读一个在变的文件，逐次记下模型请求的字节上界；内存读数经 `just mem long-turn`（§8-9） | `spec/LongTurn.lean` |
| `wire_script` | provider 线上 JSON 的脚本，与在回环地址上逐条回放它、把每次交换记进文件的替身（§8-10、§8-13） | `spec/WireScript.lean`、`spec/WireScript/Exchange.lean` |
| `bin/provider` | 替身的进程：绑一个回环端口，印出 `SPRAWLING_PROVIDER=<url>`，然后回放（§8-10） | `spec/WireScript.lean` |
-/

/-! ## 2 验收标准

- `MemLedger` 过 kernel conformance 六断言（与 JsonlLedger 同一套——这就是「缝」的兑现）。
- 同一 draft 序列灌 MemLedger 与 JsonlLedger，raw 行逐字节相同（规范字节住 kernel 的实证）。
- checker 对合法序列静默通过；对篡改序列报出首个断点行。
- 跨 OS 字节夹具：`tools/fixtures/golden-s1/` 的脚本化序列重建后与夹具逐字节相同（CI 三平台恒跑同一断言）。
- 同一剧本两次执行，账本逐字节相同：一次失败从它的剧本重放（AGENTS.md *Tests*）。

分部里的定理是模型对性质的证明：

- `spec/Executor.lean`：计数时钟交出的读数只取决于被问了几次（`the_clock_hands_out_its_count`），相邻读数差一（`each_reading_is_one_past_the_last`）；场景在回合内的两个边界上什么都不说（`inside_a_turn_the_scenario_says_nothing`），每个取消点都在它的边界上被答成取消（`every_cancel_point_is_reached`），取消压过 steer（`cancel_wins_over_steer`），steer 只在它那一回合的波边界上递到（`a_steer_is_heard_only_at_its_wave`）；同一 run 里没有两次调用的键输入相同（`no_two_calls_of_a_run_share_a_key`），按一波的时刻取键会把两次同名调用并成一次（`keying_by_the_instant_merges_two_calls_of_one_wave`）；剧本按序作答（`the_script_is_answered_in_order`），用尽之后恒作结（`an_exhausted_script_concludes`）。
- `spec/MemLedger.lean`：一本空账写下一串 draft，seq 连续（`a_fresh_ledger_holds_what_was_written`），每一行接在上一行的摘要上（`every_line_follows_the_one_before`）。
- `spec/WireScript.lean`：合法脚本里一个 id 认出写下它的那一条（`an_id_names_the_reply_that_wrote_it`）；一个续轮的回答与开启过几个 run 无关（`a_continuation_is_answered_whatever_was_opened`），得到它那个 run 的下一条（`every_run_is_answered_from_its_own_replies`）；用尽、交叉被拒（`an_exhausted_run_is_refused`、`a_crossed_request_is_refused`）；第一轮按次序开启（`an_opening_takes_the_next_run`、`no_run_left_changes_nothing`）；接上的脚本不改动握着的 run（`a_grown_replay_answers_every_held_run_alike`、`a_script_that_rewrote_a_held_run_is_refused`）；替身的拒绝都不是城会重发的状态码（`no_refusal_is_one_the_city_sends_again`）。
- `spec/WireScript/Exchange.lean`：凭据头的值不进记录（`a_credential_never_reaches_the_record`）；文件只在 `no_run_left` 时再读（`the_file_is_read_again_only_for_a_run_none_is_left_for`），接不上时什么都不变（`a_reread_that_is_not_taken_changes_nothing`），追加的 run 被下一个第一轮开启（`a_run_appended_after_the_script_ran_out_is_opened`）。
- `spec/LongTurn.lean`：窗口的增量恰是每一步加进去的字节（`the_increments_are_what_each_step_added`），有一步加得不一样多就看得见（`a_step_that_adds_more_shows_as_an_uneven_increment`）。
- `spec/RedTeam.lean`：未验证臂什么都不删（`the_unverified_arm_drops_nothing`）；判定忠实时验证臂不放行缺陷、不误删忠实结论（`the_verified_arm_keeps_exactly_the_faithful`）。
- `spec/Suite.lean`：每个 outcome 恰好记一次（`every_outcome_is_counted_once`），不认识的只进 `unknown`（`an_outcome_nobody_asked_for_is_unknown`）。
- `spec/Metabolism.lean`：没有资产在第一次被注意到的那一轮退场（`nothing_retires_the_round_it_is_first_noticed`），留下当且仅当用得上又付得起（`an_asset_is_kept_exactly_when_it_is_used_and_pays`），警告过仍未改善的下一轮退场（`a_warned_asset_that_did_not_recover_retires`）。
- `spec/Bench.lean`：bench 印出的每条读数都在钉住的字节上量（`every_reading_is_taken_under_the_pinned_fixture`），字节离开钉子就在量之前拒绝（`a_moved_fixture_is_refused_before_any_reading`）。
- `spec/Throughput.lean`：p999 只在样本够多时印出（`a_p999_is_printed_only_over_enough_samples`），不够时印 max（`under_the_floor_the_max_is_printed`），印出的尾部总是一个真实样本（`the_printed_tail_is_a_sample`）。
- `spec/BenchStartup.lean`：每个样本都留在读数里、按中位的倍数标注（`every_sample_is_kept_and_marked_by_the_cut`）；主导子步是中位最大的那一个（`the_dominant_step_has_the_largest_middle`），没有子步时没有主导（`no_steps_have_no_dominant`）。

每个模型都带一个可实现的正常路径（`every_cancel_point_is_reached`、`an_opening_takes_the_next_run`、`a_run_appended_after_the_script_ran_out_is_opened`、`spec/WireScript.lean` 里两个 run 交错作答的 `example`），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

citysim 的模块登记在 `architecture.toml`，但 `modmap` 只判 `crates/` 下的条目，故本 crate 的条目写给读者看、不受那道门判；`specalign` 判它们的 `spec` 锚点（§17）。MPL 头、lexicon、lints 全库同规。`just sim` 的入口是本 crate 测试（固定剧本即测试用例）。**本 crate 没有随机源，也没有种子**：确定性由三件事持有——剧本是写死的、时钟是 `executor` 里的 tick 计数器（`spec/Executor.lean` 的 `Clock`）、执行是单线程；复现一次失败靠的是重跑那个剧本。Dispatch 的「先落 JOB.md 再产事件」在 sim 里以 `checkpoint_committed`（确定性假 oid，由 JOB.md 内容的 B3Hash 派生前 40 个 hex 字符）代文件面——模拟适配器的职责即伪造外部世界，事件序与真城同形。

D1 **没有种子，而不是造一个吃种子的生成器。** 种子此刻没有消费者：剧本是测试用例，随机剧本批要等故障面有东西可随机，现在造一个生成器等于先立一个没有被任何断言驱动的第二权威。故 `justfile` 的 `sim` recipe 不接参数，crate 文档写的是计数时钟与固定剧本。**重开参数**：随机剧本批落地时，种子成为 `Scenario` 的一个字段，由它派生每一处分叉，`sim` recipe 同批接回一个参数。

模型的假设写在定理的假设与参数里，不写成公理：

1. **时钟不溢出**：模型的 `Clock` 是 `Nat`，Rust 的是 `u64`，到顶时以 `E_INVALID_ARGS` 拒（`clock_overflow`）；一条剧本走不到 2⁶⁴ 毫秒，所以模型不写那一档。
2. **摘要单射**：`KeyInput` 是 `IdemKey::derive` 的输入；两把键相同只在输入相同时，这是对 kernel 摘要函数的假设（`crates/kernel/spec/Ledger.lean` 以同样的方式陈述链），模型证的是输入两两不同。内存 Ledger 的行与链摘要也以 kernel 的函数为参数。
3. **判定忠实**：红队验证臂的结论以 `FaithfulJudge` 为假设，那是 `collab::Citation::against` 的性质。
4. **正文里的 id**：`Replay.collect` 怎样在请求的 JSON 里找出字符串值不进模型，模型从找出的字符串起；gateway 的「可重试」状态码集合不进模型，`no_refusal_is_one_the_city_sends_again` 把它写成结论里的三个不等式。

未决：

- **替身的放置规则有两个模型。** 本规格的 `spec/WireScript.lean` 规定 `Replay` 怎样放置一个请求；adversary 的 `tools/adversary/spec/Acceptance.lean` 写了一份验收世界依赖的放置规则（`place`、`placeAll`），它不能 import 本规格（`spec` 门：检验器的规格只 import 自己的分部），本规格也不 import 它（`depmap` 不列 citysim）。两份今天一致：续轮放在它带回那一条之后、第一轮按次序开启、追加不改已有的 run。让一份成为唯一权威的办法有二：给 `spec` 门开一条「检验器的规格可以 import 工具的规格」的边，adversary 改为引用这里的定理；或在 adversary 那份的文档注释里点名这里的定理，由人在两边一起改。证据是哪一份先被改而另一份没跟上。
- **分叉的 run**：一个分叉出来的 run 继承了母 run 的调用 id，替身今天把它读成母 run 的续轮或两个 run 交叉（D11 的重开参数）。
-/

/-! ## 4 现状分析

`just sim` 跑全部场景测试，单线程、无 I/O 等待，耗时由编译主导。测量二进制（§8-5、§8-6）的读数写在 `tools/xtask/budgets.toml` 各自的行里，只入册不入门。

实现与本规格的出入，迁移时对过：`Replay` 只持脚本与开启过几个 run，调用 id 的表住在 `WireScript` 里（§8-10 的接口照代码写）；`Suite::new` 除了同一 id 两次，也拒一个没有 id 的任务（§8-8-1 照代码写）；`bench_startup::samples` 的 `Share` 是 sprawling 的那一个，可疑的倍数是 sprawling 的 `SUSPICIOUS_TIMES`（§8-5 照代码写）。三处都是代码先于文档，接口一节已按代码改，行为没有变。
-/

/-! ## 5 权威信源

- `crates/kernel/Spec.lean` §8-9：Ledger 缝与 conformance；§8-23：`ToolCall::action`，一次工具调用的动作字节。
- `crates/runtime/Spec.lean` §8-1（驱动器与 `verify_lines`，`crates/runtime/spec/Replay.lean`）、§8-15（`run::drive`）与 §8-39（每 run 一条 `prompt_assembled`）。
- `crates/gateway/src/endpoint/failure.rs` 的 `ProviderFailure::retry`：城重发哪些状态码。
- sprawling 的 `monitor::spread`（`crates/sprawling/Spec.lean` §8-129-2）：分位、地板、峰值与可疑倍数 `SUSPICIOUS_TIMES`。
- `tools/xtask/budgets.toml`：测量读数与它们的登记规矩。
- ARCHITECTURE.md §10：确定性七条，本 crate 守其中的计数时钟、无随机源与键不从时钟来。
-/

/-! ## 6 命名统一

MemLedger、checker、Scenario、CancelPoint、ScenarioReport、WireScript、Replay；事件名取 kernel 的 `EventKind`。

Lean 里的名字与 Rust 的对应：

- `Citysim.Executor.Clock`、`Clock.now` ↔ `run_scenario_on` 里的 `tick` 与 `now` 闭包；`answer_at` ↔ `executor::answer_at`；`SafePoint` ↔ `runtime::run::SafePoint`；`KeyInput` ↔ `IdemKey::derive` 的三个参数，`keyed` ↔ `invoke` 闭包里的 `placed` 计数器；`call`、`answers` ↔ `ScriptModel` 的 `Model::call`。
- `Citysim.MemLedger.MemLedger`、`append` ↔ `mem_ledger::MemLedger` 与 `Ledger::append`；`written` 是模型里写出的行，Rust 没有对应。
- `Citysim.WireScript` 的 `Turn`、`WireScript`、`call_ids`、`Replay`、`Asked`、`Refusal`、`Answer`、`Carried` 与 Rust 同名；`located` ↔ `Replay::collect`，`carriedOf` ↔ `Replay::carried`，`Parsed` ↔ `WireScript::parse` 收下的脚本；`Body` 是读成或读不成 JSON 的正文。
- `Citysim.WireScript.Exchange.is_credential`、`answered`、`wants_more_runs` ↔ `exchange::is_credential`、`ScriptedProvider::answered`、`Answer::wants_more_runs`。
- `Citysim.LongTurn.windows` ↔ `TurnReading::windows`；`increments` ↔ 测试里相邻窗口之差。
- `Citysim.RedTeam`、`Citysim.Suite`、`Citysim.Metabolism`、`Citysim.BenchStartup` 里的类型与 Rust 同名；`Metabolism.Limits` 是两个阈值常量合成的参数。
-/

/-! ## 7 模块边界

```
mem_ledger ──▶ kernel（Ledger trait＋event＋conformance feature）
checker    ──▶ runtime::replay（verify_lines 复用，不建第二验证权威）
executor   ──▶ runtime::run::drive（循环只住 runtime，本 crate 只供世界）
wire_script ──▶ gateway::response_from_wire（脚本的每条回复经城读回复的那个函数）
red_team   ──▶ collab::Citation（判定只有一处）
bin/bench、bin/bench_startup ──▶ sprawling、accounting、storage、wire 的公共面
```

**不做什么**：`MemLedger` 不落盘；不采时钟（t 由 tick 计数器给出）；不另写 FaultFs（它住 storage）。

规格本身不加依赖：分部只 import 工具链的库与本规格的分部。`depmap` 块不列 citysim（它在产品图之外），所以本规格不 import 任何 crate 的规格；用到 kernel、runtime 的性质时在注释里引用它们的分部。

D6 **评估仪器住在 citysim，不另立 crate。** 五件仪器（§8-8）没有产品调用点：`suite` 只由 `tests/evaluation.rs` 驱动，其余四件只由自己的测试驱动。为它们在产品拓扑里立一个 crate，换来的是一个不进二进制却占一格依赖图的单元，以及 `sprawling` 为一个交接探针多背一条边。citysim 本就是 dev-only 的第二个 Main，仪器与剧本同住，产品图少一个单元。交接探针有生产调用点，归它的拥有者 `accounting::worker::probing`（`crates/sprawling/Spec.lean` §8-39）。落选方案：独立的 `eval` crate——它唯一的生产面是那个探针。**重开条件**：一件仪器得到生产调用点。
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `tools/citysim/spec/MemLedger.lean` |
| 8-2 | `tools/citysim/spec/Executor.lean` |
| 8-3 | `tools/citysim/spec/Executor.lean` |
| 8-4 | `tools/citysim/spec/Executor.lean` |
| 8-5 | `tools/citysim/spec/BenchStartup.lean` |
| 8-5-1 | `tools/citysim/spec/BenchStartup.lean` |
| 8-6 | `tools/citysim/spec/Bench.lean` |
| 8-7 | `tools/citysim/spec/RedTeam.lean` |
| 8-8 | `tools/citysim/spec/Suite.lean` |
| 8-8-1 | `tools/citysim/spec/Suite.lean` |
| 8-8-2 | `tools/citysim/spec/Metabolism.lean` |
| 8-8-3 | `tools/citysim/spec/Nesting.lean` |
| 8-8-4 | `tools/citysim/spec/Ablation.lean` |
| 8-9 | `tools/citysim/spec/LongTurn.lean` |
| 8-10 | `tools/citysim/spec/WireScript.lean`，一次交换的那一半在 `tools/citysim/spec/WireScript/Exchange.lean` |
| 8-12 | `tools/citysim/spec/Bench.lean` |
| 8-13 | `tools/citysim/spec/WireScript.lean` |
| 8-14 | `tools/citysim/spec/Throughput.lean` |

8-11 没有用过。
-/

/-! ## 9 工作流程

- 测试构造 drafts → MemLedger append → checker／conformance／对拍 JsonlLedger。
- Scenario → `run_scenario` → `runtime::run::drive`（每次问时钟拿下一个计数，每个安全点问一次 `answer_at`，每次工具调用经 `ToolBench` 与信封）→ `ScenarioReport` → `check_chain` 加事件序断言。回合内与 run 的流程是 runtime 的模型（`crates/runtime/spec/Turn.lean`、`crates/runtime/spec/Run.lean`）。
- `just bench`：`bin/bench` 先钉夹具（D8），再依次量负载场景并印读数行；`just bench-startup`：`bin/bench_startup` 先构建被测二进制，再量四个动作与首字节。
- `just provider <script> <record>`：`bin/provider` 绑端口、印 `SPRAWLING_PROVIDER=<url>`，然后一条连接一条连接地回放（§8-10、§8-13）。
- `just mem long-turn <steps>`：`bin/long_turn` 每走若干步停一次，让配方从外面读进程的内存（D10）。
-/

/-! ## 10 实现逻辑

MemLedger 的 append 是 from_draft→canonical_line→chain_hash 推进；无别的逻辑（`spec/MemLedger.lean`）。检查器复用 `runtime::replay`，验证语义一处（D16）。执行器只供世界：循环、回合的四相、事件序与冻结都住 `runtime::run`，执行器给的是计数时钟、`answer_at` 与工具调用的键（`spec/Executor.lean`）。

替身的判定与适配分开：`Replay` 是纯判定，不碰套接字也不碰文件，所以它的规则能写成定理（`spec/WireScript.lean`）；`exchange` 只读字节、写回答、追加记录、在 `no_run_left` 时重读文件（`spec/WireScript/Exchange.lean`）。
-/

/-! ## 11 边界枚举

- 空 Ledger check 通过；单创世行通过。
- 空剧本模型第一次调用就作结，run 以 `Completion::Limit` 冻结（`a_reply_that_says_nothing_freezes_as_limit_rather_than_done`）。
- 取消点写在一个回合数之外：那个边界永远到不了，run 照常走完（`every_cancel_point_is_reached` 只说写在走到的回合上的取消点被答）。
- 替身：空的 run、写了两次的 id、不是最后一条却不调用工具的回复在 `parse` 时拒；正文不是 JSON、id 交叉、用尽、没有剩下的 run 各有自己的码（§8-13）；连接在一个完整请求之前断了，不回答、不记录、不花掉一条（§8-10 续）。
- 时钟到 `u64` 顶时拒（§3 第 1 条）。
-/

/-! ## 12 错误处理

透传 kernel、replay 与产品公面的 `AxError`，不新增码。本 crate 自己的失败用既有码走三段式：时钟溢出与 bench 的零样本是 `E_INVALID_ARGS`；读不懂的替身脚本、接不上的重读是 `E_CONFIG_INVALID`（subject 是键路径）；替身的套接字失败是 `E_TOOL_UNAVAILABLE`；记录写不进去是 `E_STORAGE_FATAL`。替身对城的拒绝不是 `AxError`，是带稳定码的 HTTP 回答（§8-10、§8-13），状态码都不在城会重发的那一集里（`no_refusal_is_one_the_city_sends_again`）。

决定的条目与它们住的地方：

| 决定 | 标题 | 住处 |
|---|---|---|
| D1 | 没有种子，而不是造一个吃种子的生成器 | 本文件 §3 |
| D2 | 计时边界取「动作的可观察端点」，进程动作以退出为端点 | `tools/citysim/spec/BenchStartup.lean` §8-5 |
| D3 | 安装边界含归档摘要校验、不含 PATH 写入 | `tools/citysim/spec/BenchStartup.lean` §8-5 |
| D4 | 计量主语是 Rust measuring Main，不是 tools/adversary/ 也不是 criterion | `tools/citysim/spec/BenchStartup.lean` §8-5 |
| D5 | 被测可执行文件的名字在本 crate 只重述一处，注释点名它的权威 | `tools/citysim/spec/BenchStartup.lean` §8-5 |
| D6 | 评估仪器住在 citysim，不另立 crate | 本文件 §7 |
| D7 | 场景只在回合边界取消 | `tools/citysim/spec/Executor.lean`，`answer_at` 之上 |
| D8 | 读数带着它所量字节的摘要，登记的夹具钉住这个摘要 | `tools/citysim/spec/Bench.lean`，`bench` 之上 |
| D9 | 被量的产品 feature 集就是 `sprawling` 包的默认 feature | `tools/citysim/spec/Bench.lean` §8-6 |
| D10 | 长回合的门是请求窗口的逐步增量，RSS 只作读数 | `tools/citysim/spec/LongTurn.lean` |
| D11 | 进程外的替身 provider 是 citysim 的一个二进制，脚本就是 `ScriptModel` 的那种线上 JSON，按 run 分开作答 | `tools/citysim/spec/WireScript.lean`，`Replay` 之上 |
| D13 | `large_worktree_placement` 量的是领树，备树在计时之外 | `tools/citysim/spec/Bench.lean` §8-12 |
| D15 | 脚本的 run 用完时，替身先把脚本文件再读一遍，再拒一个新开的 run | `tools/citysim/spec/WireScript/Exchange.lean`，`answered` 之上 |
| D16 | 检查器复用 `runtime::replay`，不自写链验证 | `tools/citysim/spec/MemLedger.lean` §8-1 |
| D17 | 读数行一个文法、机器类属进字段，读数不设门 | `tools/citysim/spec/Bench.lean` §8-6 |
| D18 | 多 run 并行不在本 crate 里量 | `tools/citysim/spec/Bench.lean` §8-6 |
| D19 | 红队两臂共用剧本与判定，只差判定是否被调用 | `tools/citysim/spec/RedTeam.lean` |
| D20 | 一次工具调用的键：每跑一个的位次，加上整个动作的字节 | `tools/citysim/spec/Executor.lean` |
| D21 | 仪器只在测试构型里编译 | `tools/citysim/spec/Suite.lean` §8-8 |
| D22 | 过期基线的场景驱动一座真的城，而不是 `run::drive` 加剧本工具台 | 本文件 §16 |
| D23 | 吞吐台住在 `sprawling-accounting` 的仪表里，本规格只规定它 | `tools/citysim/spec/Throughput.lean` §8-14 |
| D24 | 这一段的等待从账本与仪表外侧读，生产代码不加测量点 | `tools/citysim/spec/Throughput.lean` §8-14 |
| D25 | TP3 的协作场景经 `attend` 驱动一座真城，模型按 run 收到的任务认角色 | 本文件 §16 |
-/

/-! ## 13 依赖选型

kernel（features=["conformance"]）、storage（对拍与夹具）、runtime（驱动器与 verify）、gateway（dialect 翻译面）、collab（引文判定）、wire（帧，feature `server`）、sprawling 与 accounting（测量二进制驱动的产品公面）、serde_json、toml（nesting 读它评分的 TOML 形状）、sha2 与 zip（安装动作读发行档）；dev：tempfile、zeroize。版本以 `tools/citysim/Cargo.toml` 与 workspace 清单为准。

规格不加 Lean 依赖：仓库根 `lake-manifest.json` 的包表是空的（adversary D2），分部只用工具链自带的库。
-/

/-! ## 14 硬编码声明

策略常数只在仪器里（`metabolism` 的 `ASSET_IDLE_DAYS`、`ASSET_FLOOR_PER_MILLE`）与测量二进制里（`SAMPLES`、夹具规模、`PINNED_DRAFTS`、轮询间隔与单样本上限），各自注释点名它的理由；它们的值住 Rust，模型把用得到的那几个当参数（`spec/Metabolism.lean` 的 `Limits`），不抄第二份。长回合每一步读的文件长度 `STEP_BYTES` 与替身记录里凭据的写法 `REDACTED` 同理。登记夹具的摘要钉在 `REGISTERED.pinned`，改它的规矩是 D8。
-/

/-! ## 15 影响面

`just sim` 与 `just bench` 的全部场景建于本 crate 之上；`just bench-startup`、`just mem long-turn` 与 `just provider` 拉起本 crate 的三个二进制；`just acceptance` 的替身是 `bin/provider`，它的脚本格式与放置规则是 adversary 验收世界的依靠（adversary D6、D7，`tools/adversary/spec/Acceptance.lean`）。`runtime::run::drive` 的签名、`kernel::Model` 与 `kernel::Tool` 的缝、`gateway::response_from_wire` 变了，执行器与剧本适配器跟着改。

模型体验：零字节。dev-only 设施，恒不进任何 prefix。
-/

/-! ## 16 测试与约束

形式化的义务由证明清偿：`lake build tools.citysim.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查（`cargo nextest run -p citysim`），它们是行为比对，不是精化证明：

- 内存 Ledger 与检查器：`tests/ledger_conformance.rs`（`mem_ledger_passes_the_same_conformance_suite_as_jsonl`、`mem_and_jsonl_produce_identical_bytes`、`the_chain_checker_passes_truth_and_bites_tampering`、`golden_fixture_pins_cross_os_bytes`）。
- 确定性与取消：`tests/scenario.rs`（`the_loop_is_byte_deterministic`、三条 `a9_cancel_…`、`s3_14_the_run_is_byte_identical_when_replayed`、`two_runs_interleaved_on_one_ledger_replay_byte_identically`、`two_reads_in_one_wave_are_two_calls`）与 `tests/interventions.rs`（`a_cancel_at_the_same_boundary_beats_a_steer`、`a_steer_advances_the_run_instead_of_stopping_it`）；`script_model` 的 `pops_in_script_order_then_concludes`。
- 驱动一座真城的场景：`tests/proposal_baseline.rs`（`a_proposal_made_on_a_version_the_document_left_is_refused_when_decided`，D22）。
- TP3 的协作场景：`tests/collaboration.rs`（D25）。
- 替身：`wire_script::tests` 的七条（同一请求逐字节同录、用尽、两个 run 交错、追加的 run 被开启、改写了已在答的 run 的脚本被拒、读不懂的回复与分不清或到不了的 run 在解析时拒）。
- 长回合：`long_turn::tests` 的 `a_long_turn_grows_its_window_by_one_step_at_a_time`。
- 仪器：`red_team::tests`、`suite` 与 `tests/evaluation.rs`、`metabolism`、`score`、`nesting::tests`、`ablation::tests`。
- bench：`bench::reading::tests`、`bench::scenarios::tests`（含 `the_registered_fixture_writes_the_bytes_its_digest_pins`）、`bench_startup::samples` 与 `footprint`、`actions` 的测试。

只有节注释的分部（`spec/Nesting.lean`、`spec/Ablation.lean`）由穷尽枚举与各自的测试守住。挂钟读数不是任何断言的对象：它们入册不入门（D17）。

D25 **TP3 的协作场景经 `attend` 驱动一座真城，模型按 run 收到的任务认角色。** 三条场景：(1) `tp3_three_delegated_children_start_at_the_call_and_run_side_by_side`：`lab/lead` 在三个回合里各调一次 `delegate`（`lab/kiln`、`lab/glaze`、`lab/clay`），判每个子 run 以 lead 为 parent、`run_started` 的 seq 在它那次 `tool_called` 之后，且三个子 run 同时在跑（每个子 run 的第一次模型调用等另两个到齐，等不到 20 s 就记为没到齐）；子 run 的 `run_started` 可以落在 lead 冻结之后，因为「在调用时生效」说的是调用时派出，子 lane 写下第一行的时刻不归调用方。(2)(3) 测试城第二局海龟汤的形状，三轮：`hall/mayor` 主持、`hall/clerk` 猜，每个 run 开头读一次城里 hall 楼的手册 soup.md。异步：主持第一个 run 发问后给自己发一次 mention（测试城里「等一会儿再看」的绕行），期望 7 个 run、开头读 7 次、21 次模型调用；一个回答到时主持的上一个 run 还在跑，它就落在那个 run 的下一个安全点、不开新 run（TP3 的 B），所以少一个 run 也是 lane 可能走的次序，断言收的是 6 或 7 个 run、run 数等于开头读的次数、至多 21 次调用；同步（`wait: true`）：期望 4 个 run、开头读 4 次、14 次模型调用。读数还印出计数时钟从第一个 `run_started` 到最后一个 `run_frozen` 被读了几次与 seq 跨度，这两个不进断言。理由：要判的事跨发信方、收信方与 lane 三个写者，只在 worker 里会合（同 D22）。并发 lane 写进账本的先后不固定，所以断言只读计数与每个 run 自己的先后，不读整本账的字节。被否决的做法：按 factory 被调用的次序分角色——并发的 lane 让这个次序不固定。重开参数：替身 provider 能按房间作答时，角色改由脚本给出。

D22 **过期基线的场景驱动一座真的城，而不是 `run::drive` 加剧本工具台。** 场景经 `accounting::worker::genesis::form` 造城、经 `accounting::worker::RunWorker` 派一次活：`Hands` 换上计数时钟与内存里的 vault，模型是剧本的 `ScriptModel`，工具台、`proposal` 工具、relay、治理折叠与人的决定都是生产件（`crates/accounting/Spec.lean` §8-30）。理由：要判的事跨三个写者——run 的工具写下卡，城外的写者（人的编辑器）挪动文档，人的决定判基线（documents D17、D35）——三者只在 worker 里会合；在剧本工具台上用 `ScriptTool` 写一行 `proposal_offered`，判的是剧本自己，生产的工具与 relay 都不在路上。它判结局：拒绝码是 `E_VERSION_CONFLICT`，文档留着挪动之后的字节，卡仍开着；不判同一场景跑两遍账本逐字节相同，那是 `crates/accounting/Spec.lean` §3 第一条还差的一步。代价：场景经 git 与文件系统，比只经 `run::drive` 的场景慢。被否决的做法：写进 `crates/sprawling/tests/acceptance/`——那里判每件工具答得对，过期是一个跨写者的场景，refrain 路线图 §4-10 的验收要的正是一个 citysim 场景。重开参数：场景库要逐字节重放一次 dispatch 时，本场景是它的起点。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；仓库根 `lakefile.toml` 的 `Spec` 库以 `tools.citysim.Spec` 与 `tools.citysim.spec.+` 两个 glob 收进它。布局改了，分部的路径、两个 glob 与 `architecture.toml` 里 citysim 各行的 `spec` 锚点一起重看。
- `architecture.toml` 的模块图：citysim 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- ARCHITECTURE.md §3（citysim 在产品图之外、驱动回合循环的第二个 Main）、§10（确定性七条）、§11 的 V6 一行（citysim 的场景数由 `cargo xtask docnum` 管）。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- `justfile` 的 `sim`、`bench`、`bench-startup`、`mem`、`provider`、`acceptance` 配方，与 `crates/sprawling/Cargo.toml` 的默认 feature（D9）。
- adversary 的规格：`tools/adversary/Spec.lean` D3、D6、D7 与 `tools/adversary/spec/Acceptance.lean` 依靠替身的脚本格式与放置规则（§3 未决的第一条）。
- runtime 的规格（`crates/runtime/Spec.lean`）：驱动器、回合、`SafePoint` 与 `verify_lines`；它们改了，`spec/Executor.lean` 与 `spec/MemLedger.lean` 一起重看。
- 增一个场景、测量或仪器时，本规格同一变更集增一节（在规定它的模块的分部里）；夹具更新须与 storage、kernel 的字节规范同一变更集。
- 引本规格的其他文档与 rustdoc 写 `tools/citysim/Spec.lean §8-n` 或 `citysim D<n>`；一节换了分部，它的标签不变，引用不必改。
-/
