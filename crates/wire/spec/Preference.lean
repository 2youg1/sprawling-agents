-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::preference

规定 `preference`、`command::shelf`、`answer::model_facts`（`crates/wire/src/` 下同名的文件）。人能编辑的两份文件各得一扇门，以及同一次升版里的几件小事。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-39 七件线上的小事：闭集、缺席、两个文件与一个兼容格式

**升版的代价是 `wire.ts` 重生与客户端同改，与改动数量无关，分两次就是付两次**，所以能同时落地的线上改动放进同一次升版。以下各件与 §8-38 同属一次升版。

**一、`mode` 是 `kernel::model::Mode`，不是自由文本。** 自由文本的 mode 要由上游把认不出的词落到某个默认值上，于是拼错一个词得到一个没人要的 run 和零句话。`Mode` 是闭集（今天 `chat｜work`，§8-57），未知词在反序列化处即拒。**定义落在 kernel 而不是 wire**：与 `DialectKind` 同一条依赖倒置，wire 携带它、`runtime` 求值它，两边都不得指名对方。`carried_name` 因此只剩三个真正开放的名字（provider／template／toolkit）——**值集开放才进那个宏，闭集不进**。

**二、`context_tokens` 是 `Option<Window>`。** `Window` 与 `Ceiling` 同形（非零新类型）而**不是同一个类型**：一个界定模型能读多少，一个界定它能写多少，互换仍能编译的两个数不该共用一个名字。零在类型上不存在，缺席是 `null`；旧编码把「没人填」写成 `0`，于是上下文提醒拿一段对话去比对一个没人给过的数。

**三、`HandOff`；没有 `CreatePolicy`。** `HandOff { item, to, idem }` 把一条等待中的设计问题交给一位居民，写 `question_handed`；`SetAutonomy` 仍然回答「谁答全部」，两者是不同射程的两个决定，故是两条帧。`CreatePolicy` 随升级机制一起删——没有升级就没有可豁免的。

**四、没有上传链条。** 线上没有 `Attach`、`/upload` 与 `UploadId`：一条只写不读的链会把字节永久落进城目录，无保留期、无清理，而消费它的路由臂只能拒为 `not_built`。

**五、`agent_protocols::Incoming` 是唯一入站文法。** 若路由把请求反序列化成另一个结构再逐字段搬进 `Incoming`，`Incoming::parse`——那个拒绝空 task／空 goal 的构造器——就**只有测试在调**，rustdoc 承诺的「没有完成定义的 run 报不出自己完成了」在真实编辑器路径上不成立；同一次搬运还把明文令牌抄进一个 `derive(Debug)` 的结构。现在 `AcpSink` 收 `&serde_json::Value` 与 `Pairing`，门只读 `token` 一个键用于判定，其余的键由 `parse` 读——**一个文法一个家**。`Pairing` 是枚举而不是 `bool`：传反了不该还能编译。

**六、`EndpointSummary` 说得出自己是怎么连的，也说得出它服务的模型。** 它携 `connection_kind: String`，取 `ConnectionKind::as_str` 的七个扁平词之一；`models: Vec<String>` 升为 `Vec<ModelFactsSummary>`（上限、模态、价格原文）。`dialect` 留在旁边，它答的是更窄的一问——**哪支笔写请求**；两种连接可以共用一支笔而仍是两次不同的登记，只显示笔的页面说不出人当初设的是哪一个。`ModelFactsSummary` 每个字段都是**上游说过的话，不是本城的结论**：缺席就是那一行没说，不在此处补预设表，因为事实梯要在调用处爬一次，答案已经爬过的摘要就是第二个答案。

**七、人能编辑的两个文件各得一扇门。**

| 帧 | 形状 | 为什么是这个形状 |
|---|---|---|
| `Query::Preferences` → `PreferencesAnswer` | `lang: Option<Lang>`、welcomed、panel、`Appearance`、`proxying`、改过的和弦；`#[serde(default, deny_unknown_fields)]` | 浏览器曾按行缓存这些：十二个键、三个读取器，各自处理缺省与非法值。整表一扇门，允许值表由本 crate 声明一次并经 schema 传给客户端——**能画出来的选项就是这个 build 装得回的选项** |
| 同上：本类型兼任 `[ui]` 的文法 | `[ui]` 节就是 `PreferencesAnswer` 的序列化；缺席字段取 `PreferencesAnswer::default()`（`panel` 是唯一不同于类型默认的一个：没人关之前它开着）；不认的键即拒 | 文件能写的键与答案能说的字段是**同一份声明**，而不是一边一份的两张表。`lang` 缺席而不是填 `en`：没人选过之前，浏览器自己的语言标是唯一的证据，写死一种语言会在每一台从未打开过该设置的机器上盖掉它 |
| `Command::PutPreferences { patch, idem }` | `PreferencePatch` 闭集：`lang｜welcomed｜panel｜appearance｜proxying｜chord` | 一帧一件事，不是整表写回：两个屏幕各改一件，不得互相覆盖 |
| `Query::Config { addr }` → `ConfigAnswer` | `effort: Option<SettledEffort>` ＋ `second: SettledSecond` ＋ `TuningDefaults`；`SettledEffort` 携 `ConfigLayer`（`default｜city｜building｜resident`，后三个与 `city::Layer` 同拼写，`default` 是没有任何一级文件说过、城的内建值在生效）；`SettledSecond` 同携 `ConfigLayer`，`percent` 为已过 `SecondThreshold` 构造点的整百分数，没有一级说过时是 `kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`，`domain: SecondDomain { min, max }` 是 `SecondThreshold` 构造点的合法域（`CTX_REMINDER_SECOND_MIN`／`_MAX`）（§8-47）；`effort` 缺席仍是一句陈述：没有一级说过时回答的是提供方自己的缺省，城说不出那个值；`TuningDefaults` 为 `from: ConfigLayer`（今天恒为 `default`：梯上没有一级文件说得出这几个数，它们是 `gateway::EndpointTuning::DEFAULTS` 的读出；层随值一起答，页面才不必自己断定「这是内建的」）＋ `timeout_ms` ＋ `request_max_retries: Option<u32>`（`Retries::stated`，缺席即 `UntilHalted`）＋ `stream_idle_timeout_ms: Option<u64>`（缺席即与整通调用同界）＋ `proxying` | **层是答案的一半。** 只给解析值的页面说不出这是本层写的还是继承来的，于是要把三层再读一遍自己爬一次梯子——**一把梯子爬两次就是一个问题两个答案**。层名随 `city::Layer`：线上把楼的文件叫 `resident`、把房的文件叫 `room`，而梯子把房的文件叫 `resident`——读者与被治的那次 run 会对“这是哪一份文件”给出不同的答案；一处穷尽匹配（`accounting::views::lines::rung_of`）把两份拼写钉在一起。`TuningDefaults` 是 `gateway::EndpointTuning::DEFAULTS` 的读出，字段形状也随它：重试上限是 `Retries` 而不是一个数（“直到有人按停”没有数字拼得出来），流的那个界是**闲置界而非整答案的截止**，且可缺席 |
| `Command::PutShelved { shelf, name, text, idem }` | `Shelf { Library, Building(addr) }`；写 `shelved_document_written` | 与 `GovernedDocument` 同一条理由：两处货架都在保留子树里，任何写域都够不着，所以帧里没有路径可拼。`name` 允许子路径，脚本因此留得住自己的文件夹 |

**八、`DialectKind::OpenAiResponses`。** kernel 的兼容格式集由二变三，`gateway::dialect` 的五个入口各多一条臂。登记与调用从此说同一句话：人粘贴 responses URL，`ConnectionKind::Responses` 记住了，而 `wire()` 从前仍答 `OpenAi`——**记对了、调错了**。形状的出处归 `crates/gateway/Spec.lean` §8-20。
-/
