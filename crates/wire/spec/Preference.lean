-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::preference

规定 `preference`、`command::shelf`、`answer::model_facts`（`crates/wire/src/` 下同名的文件）。人能编辑的两份文件各得一扇门，以及同一次升版里的几件小事。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `wire::preference::tag::tests` 守住。
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
| `Query::Preferences` → `PreferencesAnswer` | `lang: Option<Lang>`、welcomed、panel、`tier: Option<Tier>`（`zen｜blend｜panorama`）、`Appearance`（含 `glass: Option<Glass>` 与 `blend_percent: Option<u32>`）、`proxying`、改过的和弦、session 的标签（§8-84）；`#[serde(default, deny_unknown_fields)]` | 浏览器曾按行缓存这些：十二个键、三个读取器，各自处理缺省与非法值。整表一扇门，允许值表由本 crate 声明一次并经 schema 传给客户端——**能画出来的选项就是这个 build 装得回的选项** |
| 同上：本类型兼任 `[ui]` 的文法 | `[ui]` 节就是 `PreferencesAnswer` 的序列化；缺席字段取 `PreferencesAnswer::default()`（`panel` 是唯一不同于类型默认的一个：没人关之前它开着）；不认的键即拒 | 文件能写的键与答案能说的字段是**同一份声明**，而不是一边一份的两张表。`lang` 缺席而不是填 `en`：没人选过之前，浏览器自己的语言标是唯一的证据，写死一种语言会在每一台从未打开过该设置的机器上盖掉它 |
| `Command::PutPreferences { patch, idem }` | `PreferencePatch` 闭集：`lang｜welcomed｜panel｜tier｜appearance｜proxying｜chord｜core_priority｜tags`（后两件见 §8-61、§8-84） | 一帧一件事，不是整表写回：两个屏幕各改一件，不得互相覆盖 |
| `Query::Config { addr }` → `ConfigAnswer` | `effort: Option<SettledEffort>` ＋ `second: SettledSecond` ＋ `TuningDefaults`；`SettledEffort` 携 `ConfigLayer`（`default｜city｜building｜resident`，后三个与 `city::Layer` 同拼写，`default` 是没有任何一级文件说过、城的内建值在生效）；`SettledSecond` 同携 `ConfigLayer`，`percent` 为已过 `SecondThreshold` 构造点的整百分数，没有一级说过时是 `kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`，`domain: SecondDomain { min, max }` 是 `SecondThreshold` 构造点的合法域（`CTX_REMINDER_SECOND_MIN`／`_MAX`）（§8-47）；`effort` 缺席仍是一句陈述：没有一级说过时回答的是提供方自己的缺省，城说不出那个值；`TuningDefaults` 为 `from: ConfigLayer`（今天恒为 `default`：梯上没有一级文件说得出这几个数，它们是 `gateway::EndpointTuning::DEFAULTS` 的读出；层随值一起答，页面才不必自己断定「这是内建的」）＋ `timeout_ms` ＋ `request_max_retries: Option<u32>`（`Retries::stated`，缺席即 `UntilHalted`）＋ `stream_idle_timeout_ms: Option<u64>`（缺席即与整通调用同界）＋ `proxying` | **层是答案的一半。** 只给解析值的页面说不出这是本层写的还是继承来的，于是要把三层再读一遍自己爬一次梯子——**一把梯子爬两次就是一个问题两个答案**。层名随 `city::Layer`：线上把楼的文件叫 `resident`、把房的文件叫 `room`，而梯子把房的文件叫 `resident`——读者与被治的那次 run 会对“这是哪一份文件”给出不同的答案；一处穷尽匹配（`accounting::views::lines::rung_of`）把两份拼写钉在一起。`TuningDefaults` 是 `gateway::EndpointTuning::DEFAULTS` 的读出，字段形状也随它：重试上限是 `Retries` 而不是一个数（“直到有人按停”没有数字拼得出来），流的那个界是**闲置界而非整答案的截止**，且可缺席 |
| `Command::PutShelved { shelf, name, text, idem }` | `Shelf { Library, Building(addr) }`；写 `skill_shelved`（kernel D23） | 与 `GovernedDocument` 同一条理由：两处货架都在保留子树里，任何写域都够不着，所以帧里没有路径可拼。`name` 允许子路径，脚本因此留得住自己的文件夹 |

**八、`DialectKind::OpenAiResponses`。** kernel 的兼容格式集由二变三，`gateway::dialect` 的五个入口各多一条臂。登记与调用从此说同一句话：人粘贴 responses URL，`ConnectionKind::Responses` 记住了，而 `wire()` 从前仍答 `OpenAi`——**记对了、调错了**。形状的出处归 `crates/gateway/Spec.lean` §8-20。
-/

/-! D14 档位、玻璃与混合档透明度随偏好进城，三者都可缺

**决定**：`PreferencesAnswer` 加 `tier: Option<Tier>`，`PreferencePatch` 加 `Tier(Tier)` 一件；`Appearance` 加 `glass: Option<Glass>` 与 `blend_percent: Option<u32>`，两者带 `#[serde(default)]`，随整份外观一起写。三者缺席都是「这个人没说过」：档位缺席时页面开在它自己的首档，玻璃缺席时页面画它自己的姿态，透明度缺席时画样式表自己的值。滑块的取值域只由页面声明（`client/src/core/appearance.ts` 的 `BLEND_PERCENT`），城照存，页面读回时把域外的数读成缺席。`WIRE_V` 不动（D13 同理：新字段可缺；新问题 `OpenProposals` 本身已改动握手哈希）。

**理由**：这三件原先只在一个浏览器里，换一个浏览器或清掉缓存就丢，而人的文件本来就是偏好的权威（§8-39 七）。档位是一件自己的事而不是外观的一格：◐ 键一按就写，单独一件补丁不会把外观整份再送一次。缺席而不是一个本 build 挑的值：与 `lang` 同理，城里写死一个首档会在每一台没按过那个键的机器上盖掉页面的首档，而页面的首档只该有一个家。

**被否**：①把档位放进 `Appearance`——每按一次 ◐ 都要整份外观往返，两个屏幕一个改字体一个切档时后到的会盖掉先到的；②在城里也声明透明度的域——城不画它，也不拒它，两份域只会各自漂开；③给三者写非可缺的默认值——那是首档与玻璃姿态的第二个家。

**重开参数**：城开始按档位或透明度做决定（例如服务端渲染一屏）时，域与默认值搬进本 crate。
-/

/-!
### 8-84 人给 session 的标签：`PreferencesAnswer::tags`、`PreferencePatch::Tags`

```rust
pub const TAG_MAX: usize = 24;
pub struct Tag(String);                // 唯一构造点 `Tag::parse`
pub struct SessionTags {
    pub city: Address,                 // 握手 `Welcome::city` 说出的那座城
    pub room: Address,                 // 这一段所在的房间
    pub began: Seq,                    // 这一段的第一行，即 `SessionLine::began`（§8-71）
    pub tags: Vec<Tag>,                // 人给它的标签，按字典序、无重复
}
pub struct PreferencesAnswer { /* … §8-39 的各字段 … */ pub tags: Vec<SessionTags> }
pub enum PreferencePatch { /* … */ Tags(SessionTags) }
```

- **标签是人的分类，不是城的历史。** 它不改变任何 run 能观察到的东西，所以不进账本、没有事件，住在人自己的 `~/.sprawling/config.toml` 的 `[ui]` 节里，与和弦同一扇门（§8-39 七）。手机连到同一座城，问 `Query::Preferences` 得到的是同一份。
- **一段 session 的身份是 `(city, room, began)`。** `began` 是账本给这一段第一行的序号，由 `Query::Sessions` 答出（§8-71），一座城里一行一个序号，所以 `(room, began)` 在一座城里唯一，且这一段后来再长多少行都不变。`city` 不能省：人的文件在他所有的城之间共用，而每座城的 `hall/mayor` 第一段都从很小的序号开始，只按 `(room, began)` 存，两座城的标签会落到彼此身上。
- **`Tags` 是整段的标签集，不是加一个或减一个。** 它把这一段的标签换成帧里那一组；空组删除这一条。与 `Chord` 同一个理由：一帧说一件事，这件事是「这一段的标签」。`apply` 之后整表按 `(city, room, began)` 排序，每段的标签按字典序且去重，所以两台机器以不同顺序打上同样的标签，写出同一份文件。
- **`Tag` 的文法：1 到 `TAG_MAX` 个字符，每个字符是字母（Unicode `Alphabetic`，任何文字）、数字（`\p{N}`）、`-` 或 `_`。** 没有空白，所以 `/tag <name>` 读一个词，一个标签在窄窗格里画成一枚不折行的小签；24 个字符放得下一个英文复合词或十来个汉字，再长就是一句话而不是一个分类。大小写不在文法里：页面在人输入时把它折成小写（`client/src/core/tags.ts`），所以 `Bug` 与 `bug` 是同一个标签；手写进文件的大写标签照读，只是另一个标签。模式（`^[-_\p{Alphabetic}\p{N}]{1,24}$`）与构造点逐字符同义：`char::is_alphabetic` 即 `Alphabetic`，`char::is_numeric` 即 `\p{N}`，两边都按码点计长度——客户端读得懂城写下的每一个标签，城不会收下客户端读不懂的。
- **`pin` 是页面的保留词，不是线上的。** 置顶是页面怎么排这些行的事（`client/src/core/tags.ts`）；线上的 `pin` 只是一个合法的标签。Mayor 当前那一段的置顶由页面推出、不存储。
- **线形变了，`WIRE_V` 升一（D1）。** `PreferencesAnswer` 多一个字段，`PreferencePatch` 多一个变体；旧客户端读不懂带 `tags` 的答案（`deny_unknown_fields` 的对面是生成的严格 schema）。
-/

/-! D21 session 的标签住在人的偏好文件里，按 `(city, room, began)` 存

**决定**：标签是 `PreferencesAnswer::tags`，经 `PutPreferences` 写入 `config.toml` 的 `[ui]`，一段 session 以 `(city, room, began)` 命名（§8-84）。

**理由**：标签是人怎么归类自己的工作，不是城里发生过的事；放进账本就要一个事件、一次折叠与一份视图，而它改变不了任何 run 的行为。偏好文件已经是「人的那一层」（`crates/accounting/spec/Person.lean` §8-8），浏览器与手机都从同一个答案读它。`began` 是 `Query::Sessions` 已经答出的稳定序号，不必为 session 另造一个 id。

**被否**：①账本事件 `session_tagged`——让人的分类变成城的历史，导出一座城会把人的标签带给下一个人；②存在浏览器里——手机看不到；③以 `(room, began)` 为键——人的文件在多座城之间共用，各城 `hall/mayor` 的第一段序号相近，会相撞。

**重开参数**：一个人在同一台机器上有两座同名的城、并且都打标签时，`city` 不再能区分它们，那时改用城的密钥指纹做键。
-/

/-! D29 配色的覆盖是人的偏好，经 `PutPreferences` 写进偏好文件的 `[ui]`

**决定**：`PreferencePatch` 多一臂 `theme`，`PreferencesAnswer` 多一件 `theme: ThemeOverride`：

```rust
pub struct ThemeOverride { pub tokens: BTreeMap<String, String>, pub css: Option<String> }
```

`tokens` 的键是 `client/src/theme.css` 的 `@theme` 块里的 CSS 变量名，值是一段 CSS 颜色；`css` 是人在 RefRain 里写的整段覆盖文本。空的 `ThemeOverride` 即「恢复默认」。覆盖与其余偏好一样存在人的偏好文件 `config.toml` 的 `[ui]` 里（D21 说的「人的那一层」），所以换浏览器、换设备也在；页面把 `tokens` 逐个写在根元素的内联样式上（`document.documentElement.style`），内联样式压过 `@theme` 块给的值；只有 `css` 放进一个追加在内置主题之后的 `<style>` 元素（`client/src/views/setup/colours.ts`）。两处都在页面里改，改动立刻生效。变量放在内联样式上，是因为一个变量的覆盖不必与内置样式表比先后，追加的 `<style>` 只留给人自己写的整段规则。易读性按 `xtask color` 的规则在页面上照算，不合格只提示，不拒写。随 V0.0.9 的那一次 `WIRE_V` 进位（D22）。没有账本事件：与 D21 同一个理由，配色是人的偏好，不是城里发生过的事。

**理由**：主题在构建时编进页面包，运行时没有一个 theme.css 文件可以打开（roadmap CT 已更正这条前提）；覆盖的变量集就是 `@theme` 那一组，所以键用变量名。键是否在 `@theme` 名单里、值是不是一段颜色，都由页面校验：`theme.css` 是客户端的文件，`wire` 包不能编进包目录以外的文件（`xtask packaged`），在服务端另抄一份名单就是这份名单的第二个权威；服务端只按 `deny_unknown_fields` 守住 `ThemeOverride` 自己的两个字段。

**被否**：①把覆盖存在浏览器里：换浏览器就丢；②存在城的保留子树里（roadmap CT 的原文）：偏好文件已经是设置之门写入的地方，配色跟着人走而不是跟着城走，与标签同理；③服务端改写页面包：一份内置资源有了两个版本。

**重开参数**：要按楼或按设备分主题时，重议覆盖的作用域。
-/

/-! D30 性能读回携带 core，而不在 ui 中再保存

`PreferencesAnswer.core: CorePreferences` 答 `[core]` 的 placement、priority 与
可缺席非零 memory_bytes；该类型的默认值为 Soft、Raised、None。
`PreferencePatch::CorePlacement(CorePlacement)` 与 `RunMemory(Option<NonZeroU64>)`
同现有 CorePriority 一样逐键写入 `[core]`。CorePlacement 四个 serde 拼写是
none、soft、soft_shares、pinned；配置读者与客户端 schema 共用此声明。
新字段及补丁改变线形，WIRE_V 升一并重生 wire.ts。
-/

/-! 设置覆盖：记录的是声明字段；TOML 拼写仍由 serde 类型决定。
settings-control PreferencesAnswer.lang client/src/views/setup.svelte
settings-control PreferencesAnswer.core client/src/views/settings/performance.svelte
settings-control PreferencesAnswer.panel client/src/views/right.svelte
settings-control PreferencesAnswer.appearance client/src/views/setup/appearance.svelte
settings-control PreferencesAnswer.proxying client/src/views/setup.svelte
settings-control PreferencesAnswer.chords client/src/views/setup/keys.svelte
settings-control PreferencesAnswer.tags client/src/views/world/session_menu.svelte
settings-control PreferencesAnswer.theme client/src/views/setup/colours.svelte
settings-control CorePreferences.placement client/src/views/settings/performance.svelte
settings-control CorePreferences.priority client/src/views/settings/performance.svelte
settings-control CorePreferences.memory_bytes client/src/views/settings/performance.svelte
settings-control ThemeOverride.tokens client/src/views/setup/colours.svelte
settings-control ThemeOverride.css client/src/views/setup/colours.svelte
settings-control Chord.action client/src/views/setup/keys.svelte
settings-control Chord.spelled client/src/views/setup/keys.svelte
settings-control Appearance.lighting client/src/views/setup/appearance.svelte
settings-control Appearance.sans client/src/views/setup/appearance.svelte
settings-control Appearance.mono client/src/views/setup/appearance.svelte
settings-control Appearance.sans_stack client/src/views/setup/appearance.svelte
settings-control Appearance.mono_stack client/src/views/setup/appearance.svelte
settings-control Appearance.body_px client/src/views/setup/appearance.svelte
settings-control Appearance.density client/src/views/setup/appearance.svelte
settings-control Appearance.chroma client/src/views/setup/appearance.svelte
settings-control Appearance.motion client/src/views/setup/appearance.svelte
settings-control Appearance.glass client/src/views/setup/appearance.svelte
settings-control Appearance.blend_percent client/src/views/setup/appearance.svelte
settings-reason PreferencesAnswer.welcomed 由 welcome 流程记录是否已走过，不提供任意改写历史的开关；设置中的 welcome 入口可再次打开。
settings-reason PreferencesAnswer.tier tier 只属于当前 tab；持久文件字段仍可读但不覆盖每次启动的 zen，层键提供当前 tab 控制。
settings-reason SessionTags.city 标签通过 session 控件整体设置，定位字段来自 session 的身份，不另提供原始字段编辑。
settings-reason SessionTags.room 标签通过 session 控件整体设置，定位字段来自 session 的身份，不另提供原始字段编辑。
settings-reason SessionTags.began 标签通过 session 控件整体设置，定位字段来自 session 的身份，不另提供原始字段编辑。
settings-reason SessionTags.tags 标签通过 session 控件整体设置，定位字段来自 session 的身份，不另提供原始字段编辑。
-/
