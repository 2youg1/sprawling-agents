-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::config_layers

规定 `config_layers`、`config_layers::write`、`config_layers::settled`、`config_layers::session`、`config_layers::resident`、`config_layers::mcp`、`config_layers::search`、`config_layers::search::section`、`config_layers::context`、`config_layers::cache`、`config_layers::clock`、`config_layers::refuse`（`crates/city/src/` 下同名的文件）。三层配置住哪三个文件、怎么求成一份 `FrozenConfig`、怎么写回去。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-4 city::config_layers（形状 1 判定／求值，兼任文件名权威）

```rust
pub use kernel::layout::CONFIG_FILE;               // 文件名的权威在 kernel::layout
pub enum Layer { City, Building, Resident }        // 穷尽三级，与 kernel::LayeredValue 同形
pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError>;
pub struct ConfigLayer { /* model / harness / effort / sandbox / mcp / search / second_threshold / clock_stamp / keep_warm / shelves / naming / remote —— 私有 */ }
impl ConfigLayer {
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError>;   // 纯函数，无 I/O
    pub fn model(&self) -> Option<&str>;                        // 这一级冻下的模型，照写下的读回
    pub fn effort(&self) -> Option<Effort>;
    pub fn sandbox(&self) -> Option<&SandboxLimits>;   pub fn mcp(&self) -> Option<&[McpServer]>;
    pub fn search(&self) -> Option<&SearchConfiguration>;      // [search] 一节（§8-4c）
    pub fn second_threshold(&self) -> Option<SecondThreshold>;
    pub fn clock_stamp(&self) -> Option<ClockStampGranularity>;   // §8-31
    pub fn keep_warm(&self) -> Option<KeepWarm>;                  // [cache] 一节
    pub fn naming(&self) -> Option<B3Hash>;                       // 会话冻下的命名版本（Identity 分部）
    pub fn shelves(&self) -> Option<&[String]>;                 // 这一级声明挂载的外部书架目录；只有 City 级可以（§8-8）
    pub fn remote(&self) -> Option<&RemoteRoute>;              // 这一级选的远程门通路；只有 City 级可以（§8-39）
}
pub fn load(city_root: &Path, addr: &Address) -> Result<FrozenConfig, AxError>;
pub fn own_layer(city_root: &Path, addr: &Address) -> Result<ConfigLayer, AxError>;
pub fn settled_effort(city_root: &Path, addr: &Address)
    -> Result<Option<(Effort, Layer)>, AxError>;      // 值连同说出它的那一级
pub fn settled_second(city_root: &Path, addr: &Address)
    -> Result<Option<(SecondThreshold, Layer)>, AxError>;  // 第二道阈值，同上
pub fn settled_harness(city_root: &Path, addr: &Address)
    -> Result<Option<(String, Layer)>, AxError>;       // 房间下一段会话交给哪家 harness，连同点名它的那一级
pub fn write_second_threshold(city_root: &Path, addr: &Address, layer: Layer,
    threshold: SecondThreshold) -> Result<(), AxError>;    // 与 write_session 同一扇门
pub fn keep_warm(city_root: &Path, addr: &Address) -> Result<KeepWarm, AxError>;   // [cache] 爬梯，一层不说即 Off
pub fn freeze_naming(city_root: &Path, addr: &Address, version: B3Hash) -> Result<(), AxError>;   // 写地址自己那一层

// config_layers::ladder（crate 内）
impl Layer {
    const ALL: [Layer; 3];                                                // 由远及近；与 Layer 由同一张表 rungs! 声明
    fn file(self, city_root: &Path, addr: &Address) -> Result<PathBuf, AxError>;
}
pub(crate) struct Ladder { /* Vec<(Layer, ConfigLayer)> —— 私有，由远及近 */ }
impl Ladder {
    fn read(city_root: &Path, addr: &Address) -> Result<Ladder, AxError>;
    fn tagged<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<(T, Layer)>;
    fn resolve<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<T>;
}
```

**一条梯子是一个值**：`Ladder::read` 按 `Layer::ALL` 由远及近读一遍，落点重复的一级丢弃；`load` 逐个关切在梯子上 fold，不逐级点名。加一级因此是 `Layer` 多一个臂：`file` 与 `tagged` 两处穷尽匹配同时报编译错，直到新一级被安置，而每个关切一次拿到它；`ALL` 是一个数组字面量，编译器不会因为它漏了一级而报错，新一级要在同一次改动里写进去。`tagged` 是「哪一级填 `LayeredValue` 的哪一格」的唯一一处答案——今天 `kernel::LayeredValue` 只有三格，所以人层（`~/.sprawling/config.toml`）进梯子时，`kernel::config` 与本模块在同一次改动里走完。

**来源与值一起答**：`settled_effort` 与 `load` 爬同一条梯子，区别只在它把说出这个值的那一级留着而不是丢掉。只被告知结果的设置页说不出「这是这间房自己写的」还是「这是全城都有的」，于是它只能把三份文件各读一遍、把同一条梯子再爬一次——**同一个问题两个答案，就是从第二次爬梯开始的**。谁压过谁仍由 `kernel::LayeredValue::resolve` 判：`tagged` 只负责「哪一级填哪一格」，`resolve` 是 `tagged` 去掉那一级，所以这条映射在本 crate 里只有一处。`None` 是整条梯子什么都没说，也就是这座城有意把强度交给供应方，而不是替人填一档。`settled_second` 是同一条路的第二个值：第二道提醒阈值也说得出是那一级写的。

**写面与读面同源**：`write_second_threshold` 与 `write_session` 走同一扇门（读—改—写整份 `CONFIG.toml`，别人写的键原样保留），收的值已经是 `SecondThreshold`——域在那一个构造点判定过，写面不再判一次；要值的字符串形状或拒因句式，答案在 `kernel::config`。

**`own_layer` 答的是另一个问题**：梯子回答「一个 Run 被什么治理」，它回答「这个地址自己写下了什么」。差别正是它存在的理由——城或楼那一级给出的默认值不是这个地址做的选择，所以它不能当作选择的记录。文件是地址自己的那份 `CONFIG.toml`（`Layer::Resident` 的落点，也就是会话写的那一份），哪怕梯子把同一份文件当作两级里更远的级读了一次；地址就是楼时两者是同一个文件，所以那种地址自己就是它的会话（§8-14）。

**门面上换名**（`lib.rs` 按能力组织，不按文件组织）：`load` 已归 `policy`，故本模块对外是 `city::load_config`；`path` 对外是 `city::config_path`；`building::create` 对外是 `city::create_building`，`created_payload` 对外是 `city::building_created_payload`（名字读起来就是它记的那个事件）。

- **文件名只有一份，层级由位置决定**：City 层住 `<city>/.sprawling/CONFIG.toml`（reserved prefix 内，因此任何 Resident 的写域都永远叠不上它——「Agent 改不了自己的配置」因此是判定而非推理）；Building 层住 `<city>/<building>/.sprawling/CONFIG.toml`；Resident 层住 `<city>/<addr>/.sprawling/CONFIG.toml`（§8-11）。三处同名，读者认一次就认得完。
- **地址就是楼时只有两级**：`addr` 与它的 building 相同时，下两级指向同一个文件，只读一次并放在 Building 级。同一份文件在两级各算一次不改变结果，却会让读者以为它能覆盖自己。
- **缺文件不是错，读不动才是**（同 `resident`）：未声明即每级 `None`，落到 `kernel::consts_policy` 的缺省；一份存在却读不出的配置报 `E_STORAGE_FATAL`。
- **梯子上的拒词带着文件，其余照解析器说的**：`ladder::stated` 读一份解析不了的文件时，把文件路径加在 subject 前面，码、action、nearby 与恢复语都照 `ConfigLayer::parse` 给的原样交出。恢复语是写拒词的那一处按它的场合写的（`two_residents` 指向 `/new` 或删键，`unreadable` 指向报错所在的那张表），梯子只知道「哪份文件」，不知道「怎么改」（D8 (a)）。
- **不认的键即拒**（`deny_unknown_fields`）：静默忽略一个拼错的键，会产生「我设了 effort 而什么也没发生」这个无从诊断的状态。本版读哪些键，由 `ConfigFile` 的字段给出，此处不复述；拒绝文字也不复述这张键表——serde 的报错点名不认识的键、列出该表接受的键，恢复语只从原文里取出报错所在的那张表头（`[model]`、`[[mcp]]`）并说改哪一节。`[clock]` 只受理 `stamp` 一个键（§8-31）；`zones` 仍拒，写它得到的是 serde 点名这个键的那句拒绝而不是一份沉默。
- **梯子不在本模块重建**：下层胜上层由 `kernel::LayeredValue::resolve` 给，冻结由 `kernel::config::freeze` 给；本模块只回答「哪三份文件、怎么读」。一条规则一个权威。
- **effort 属 `FrozenConfig` 而非 `LiveConfig`**：改它会作废 message cache breakpoints，因此改动只影响下一个 Run（理由已写在 `kernel::config`，此处不重述只遵守）。

**`[[mcp]]` 一节**：`CONFIG.toml` 第三节 `[[mcp]]`（表数组）。`label` 解成 `kernel::ServerLabel`，非法即拒并报出是哪一份文件；其余字段按这台服务器怎么够得到分两组：

- **`command`（非空）＋ `args`（缺省空表）＋ `env`（缺省空表，形如 `env = { API_KEY = "secret:mcp/apps" }`）**——城所在的机器上的一个程序。写成表而不是 `NAME=value` 行的列表：文件里一个名字只出现一次，也没有谁要去切一个人写的字符串。
- **`url`（非空）＋ `headers`（缺省空表，写法同上）＋ `transport`（`"http"`｜`"sse"`，缺省 `"http"`）**——一个地址。`transport` 是闭集而不是自由文本，拼错在写它的地方就被拒，而不是变成一台没人够得到的服务器；缺省取 `http`，因为「发一条消息过去」正是一个 url 的本义。

两组各自成行：`command` 与 `url` 同时出现即拒（读者要去猜），两者都不出现也拒。`command` 一行再写 `transport` 同样拒——命令走它自己的管道，再指一条流就是一行说了两种 transport。两张表的值都可以是 `secret:realm/name` 引用，兑付不在本模块（`crates/sprawling/Spec.lean` §8-4／§8-15）。两条口径：①**同一层内标签不得重复**——两个同名 server 会让同一个工具名同时指向两个进程，而那是一个路由错误而不是一个偏好；②整表上梯（下层写即替换上层全表），语义住 `crates/kernel/Spec.lean` §8-22，此处不复述。**不在本模块的事**：进程怎么起、起不来怎么办、confidential 楼凭什么拒——那三件全在装配层（`crates/sprawling/Spec.lean` §8-4），本模块只回答「三份文件说了什么」。

**`[sandbox]` 一节**：`CONFIG.toml` 第二节 `[sandbox]`，字段 `shell`（bool，默认 false）、`fuel`（整数，缺省取 `SANDBOX_FUEL_DEFAULT`）、`mounts`（相对 city root 的路径表，reserved prefix 在解析点即拒）。解析仍是「本版本不读的键即拒」——被写下却什么都不发生是唯一没人能诊断的状态。整节整值上梯，语义住 `crates/kernel/Spec.lean` §8-22，此处不复述。

**`[sandbox.container]`**：可选子表由 `kernel::ContainerLimits` 的 serde 输入权威直接解析，
不另写镜像或额度文法；字段缺席不选择 container。写面通过 `write_sandbox` 原样写回整份
值，`load_config` 冻结同一份值，运行中的配置不重读。限额未知字段、零值、标签镜像均拒。

**`[sandbox]` 的第四个字段**：`env_passthrough`（字符串表，缺省空表），逐项解成 `kernel::EnvVarName`。与 `mounts` 逐条同形：同一节、同一次整值上梯、同一个解析点拒——`mounts` 在这里拒保留区，`env_passthrough` 在这里拒凭据形状的名字，而“什么叫凭据形状”是 `kernel::secret` 的答案，本模块不重建它。拒绝文字里带着是哪一份文件、哪一个名字，因为一份配置被拒时人手里只有那句话。

**`[sandbox]` 的第五个字段**：`trusted`（字符串表，缺省空表），逐项解成 `kernel::ServerLabel`。它答的是「这层楼允许哪一台 connector 服务器去做城里谁都收不回的事」（今天只有运行这座城的机器的桌面），语义与读者住 `kernel::SandboxLimits::trusts`，本节只管它在 TOML 里怎么写、在哪一层写、写错了在解析点怎么拒。与 `mounts`／`env_passthrough` 同形：一名一行、缺省为空、整节整值上梯——收不回的效果按服务器逐台放行，而不是一次放宽给所有人。

**`[context]` 一节**：`CONFIG.toml` 第四节 `[context]`，一个字段 `second_threshold`（整数百分比）——上下文提醒第二道阈值响在哪一格。值的形状与合法域住 `kernel::config::SecondThreshold`（`crates/kernel/Spec.lean` §8-22），提醒怎么响住 `runtime::reminder`，本节只管它在 TOML 里怎么写、在哪一层写、写错时在哪拒。与 `mounts`／`env_passthrough`／`trusted` 逐条同形：整值上梯、Run 起点冻结、解析点拒——31–90 域外的值在解析点由 `SecondThreshold::parse` 拒（`E_INVALID_ARGS`，恢复语带合法域），不钳位、不读后丢。缺省是「没有一层说话」，而「缺席取 `CTX_REMINDER_SECOND_DEFAULT`」只在 `runtime::reminder` 一处判定。

**`[cache]` 一节**：一个字段 `keep_warm`，取 `off` 或 `five_minute`——这一层要不要在提示缓存到期前续期。值的形状与续期判定住 `kernel::keep_warm`（`crates/kernel/spec/KeepWarm.lean` §8-74），本节只管它在 TOML 里怎么写、在哪一层写。拼写由 serde 按闭集读，拼错的词与未知键同样在解析点拒。`city::keep_warm(city_root, addr) -> Result<KeepWarm, AxError>` 爬同一张梯，下层覆盖上层；一层也没说时答 `KeepWarm::Off`——续期是人付钱的请求，默认必须是不发。与 `[context]` 不同，它不进 `FrozenConfig`：续期发生在两次 run 之间。

**`[resident]` 一节**：一个字段 `harness`（字符串），点名这一层以下的房间由哪家官方 harness 当居民。值照写下的读进来：五个拼写的权威是 `agent_protocols::Harness`，本 crate 只见 `kernel`，认不认得由派活路径判（`crates/sprawling/Spec.lean` §8-4e 第 10 条）。空串在解析点拒，与 `[model] name` 走同一条判定：空值什么也没说，写它是笔误。

- **一层只点名一种居民**：同一层既写 `[model] name` 又写 `[resident] harness`，解析即拒（`E_CONFIG_INVALID`），拒词带两个键和各自的值。居民是模型还是 harness，要读者去猜，就是配置写错了。`ConfigLayer` 的字段私有、`parse` 是唯一构造点，所以两键并存的值构造不出来。地址就是楼时，楼层与房间层是同一个文件；人要在这样一个带着会话记录的文件里写 harness，先 `/new` 清掉会话写下的 `[model] name`。
- **harness 爬梯子，`[model] name` 不爬**：`settled_harness` 与 `settled_effort` 爬同一条梯子，下层胜上层，连同说出它的那一级一起答。`[model] name` 仍只是地址自己那一层的会话记录（`own_layer`，§8-14），不参与求值。
- **会话记录压过梯子**：地址自己那一层有 `[model] name` 时，`settled_harness` 答 `None`。这段会话以模型开场，就以模型走完，理由与 `crates/sprawling/Spec.lean` §8-79「会话的形状只选一次」相同。人在楼层或城层写下的 harness，从 `/new` 开的下一段会话起生效。答案针对「一次 run 在这个地址上接着跑」；派活开新房间时，新房间自己那一层是空的，梯子的答案就是它的答案。
- **城自己的写路径写不出两键并存的文件**：见 §8-4b。
-/

/-!
### 8-4b 两个没人写的配置层长出写面

```rust
pub fn write_sandbox(city_root: &Path, addr: &Address, layer: Layer, limits: &SandboxLimits) -> Result<(), AxError>;
pub fn write_mcp(city_root: &Path, addr: &Address, layer: Layer, servers: &[McpServer]) -> Result<(), AxError>;
```

- **与 `write_session` 同一道门**：梯子（城→楼→房间）本就是「一个 Run 被什么治理」的权威，第二个存储就是第二个答案。其余键原样保留，因为可能是人手写的。
- **写出的字节必须是 `ConfigFile` 读得回来的那种，`change` 在落盘前自己读一遍**：`McpServer` 的 serde 形状是嵌套的，而文件语法是平的（`label` ＋ `command`/`args`/`env` 或 `url`/`headers`/`transport`），写面照文件语法拼，一条往返测试逐支覆盖三种 transport。`Sse` 一行必写出 `transport = "sse"`：缺省是 `http`，不写就会被读回成另一种 transport。拼错之外还有组合错：往一份点名了 harness 的文件里写会话的 `[model] name`，写出的文件每个读者都拒，整栋楼从此派不出活。所以 `change` 把改好的整份文本先交给 `ConfigLayer::parse`，拒了就以 `E_CONFIG_INVALID` 拒这次写，原文件一字不动。读面是文件语法的唯一权威，写面复读一次，就不必在写面另记一份「哪些键不能并存」。
- **空的 `mcp` 表要写出来而不是省略**：省略即继承上一级，而一个人删掉最后一台服务器不是想继承一台。
- **`env` 与 `headers` 逐值判定凭据，落盘之前就拒**：一个值只要不是 `SecretRef::parse` 认得的 `secret:realm/name`，名字命中 `kernel::secret::scan::names_a_credential` 或值命中 `kernel::secret::scan::scan` 即以 `AxCode::ConfigInvalid` 拒，恢复语指向金库。判定在 `write_mcp` 进 `change` 之前逐对做，因此一次被拒的写入一个字节都没落；拒绝文字报出是哪一台服务器、哪一张表、哪一个名字，因为人手里只有那句话。**理由是这份文件进版本库**：楼的 `CONFIG.toml` 由 `city::gitignore` 放行进历史（§8-21），写进去的 key 就在这个项目的每一次克隆里。**判定不重建**：「什么叫凭据」是 `kernel::secret` 的答案，与 `[sandbox] env_passthrough` 走 `EnvVarName::parse` 是同一个权威的两次调用。
- **读面用同一个判定**：`config_layers::mcp` 把一张 `[[mcp]]` 表解成 `McpServer` 之后、交出之前，对每一台调同一个 `validate`（`pub(crate)`，实现只有一处；`write_mcp` 进 `change` 之前调的也是它）。所以手写进 `CONFIG.toml` 的明文 key 在读这份文件时以 `E_CONFIG_INVALID` 拒，拒绝文字与写面那一句相同，只在 subject 前多一个文件（梯子加的，D8 (a)）；`secret:realm/name` 引用与不像凭据的字面值照旧读得回来。理由同写面：这份文件进版本库，读得回来的明文 key 等于这座城认可了它留在每一次克隆里。手写明文的旧配置因此第一次读就被拒，这是一次行为变化，恢复语指向金库：把值存进 vault，在原处写它的引用。
- **url 的判定**：`Http` 与 `Sse` 的 `url` 在同一个 `validate` 里判，读写两面一起。用 `url::Url` 解析，scheme 只收 `http`／`https`；带 userinfo（`user:pass@`）、host 为空、query 里有一个参数的名字命中 `names_a_credential` 或值命中 `scan`（都按 percent 解码之后判）、或 url 原文任何一处命中 `scan`（path 里的 key 也算）即拒。拒绝文字只写服务器、原因和参数名，不写 url 本身，因为被拒的 url 正带着一个 key。这一段判定是 `config_layers::mcp` 的 `check_url`，`validate` 与 `[search]` 的解析都调它。不像凭据的 query 与 fragment 照原字节保留，已有配置里的这类 url 照旧读得回来；凭据要走的路是带引用的 header。用 `url::Url` 而不是自己切字符串的理由见 D25。
- **所有写面只有一条写路径**：各自把要说的话包成 `Change`（穷尽：`Session` / `Forget` / `Sandbox` / `Mcp` / `Search` / `SecondThreshold` / `Naming` / `KeepWarm` / `Effort`），同走内部的 `change`——取 `city::document` 对这份文件的持有、读、改一个键、整份原子换上去。各自读写时，两个会话改同一份 `CONFIG.toml` 会各自从同一份原件出发，后写的那一个抄掉先写的那一个的改动。`[model]` 那一节的三个值由同一条 `table` 找到或建出，免得三处对「该写进哪张表」各有各的说法。
-/

/-!
### 8-4c `[search]`：网络搜索接哪一家（`config_layers::search`，形状 1 判定／求值）

```rust
pub fn search_supplier(configuration: &SearchConfiguration) -> Result<Option<SearchSupplier>, AxError>;
pub fn default_search_supplier() -> Result<SearchSupplier, AxError>;   // 缺省那一家的唯一声明
pub fn settled_search(city_root: &Path, addr: &Address)
    -> Result<Option<(SearchConfiguration, Layer)>, AxError>;         // 值连同说出它的那一级
pub fn city_search(city_root: &Path) -> Result<Option<SearchConfiguration>, AxError>;   // 只读城那一级
pub fn write_search(city_root: &Path, configuration: &SearchConfiguration) -> Result<(), AxError>;
```

文件里这样写（`choice` 三选一；`selected` 与 `suppliers` 只跟 `"custom"` 出现）：

```toml
[search]
choice = "custom"                # "default" | "custom" | "off"
selected = "brave"
[[search.suppliers]]
id = "brave"
url = "https://search.example/mcp"
remote = "brave_web_search"
query = "query"
count = "count"                  # 可缺；objective 同样可缺
[[search.suppliers.accounts]]
id = "main"
reference = "secret:search/brave.main"
header = "x-api-key"
```

- **求值**：`search_supplier` 是「一个 run 的 `web_search` 接哪一家」的唯一答案。`Default` 答 `default_search_supplier()`，`Custom` 答 `id` 等于 `selected` 的那一家，`Off` 答 `None`（这里不提供 `web_search`）。`Custom` 的答案恒不是缺省那一家，除非人把它列进了 `suppliers` 并选了它；`selected` 在列表里找不到时以 `E_CONFIG_INVALID` 拒，不回落（D24）。冻结的值是 `FrozenConfig.search`（`crates/kernel/spec/Config.lean` §8-22），accounting 在 Run 起点对它调一次本函数（`crates/accounting/spec/Connectors.lean` §8-35）。
- **缺省那一家只在这里声明**：`id = "exa"`，`url = "https://mcp.exa.ai/mcp"`，`remote = "web_search_exa"`，`query = "query"`，`objective = "objective"`，`count = "numResults"`，一个匿名账号 `anonymous`。设置页显示的缺省地址由 `Query::Config` 从这个函数读回，页面与 kernel 都不写第二份。这组参数名的出处见 D24。
- **解析时拒**（`E_CONFIG_INVALID`，梯子在 subject 前加上文件）：`choice` 不是三种拼写之一；`"default"`／`"off"` 带着 `selected` 或 `suppliers`；`"custom"` 缺 `selected`、`suppliers` 为空或 `selected` 不在其中；两家同 `id`；`url` 过不了 `[[mcp]]` 的 url 判定（§8-4b，D25）；`remote` 或 `query` 为空；写了的 `objective`／`count` 为空，或三个参数名里有两个相同；`accounts` 过不了 `ProviderAccount` 的校验（空表、重复 id 或引用、匿名账号带 header）；带引用的账号没写 `header`；任何不认的键。
- **带引用的账号必须写 `header`**：MCP 的 header 值按整个值兑付（一个 `secret:realm/name` 换成整份密文），所以 Key 只能原样放进一个具名 header；`Authorization: Bearer` 要一个兑付者不加的前缀。缺省那一家收 `x-api-key`。重开参数：出现一家只收 Bearer 的供应方。
- **只写在城与楼两级**：房间那一级写 `[search]` 由梯子拒（`crates/city/spec/ConfigLayers/Ladder.lean` 的 `Confined.Search`），恢复语指向楼或城的 `CONFIG.toml`。理由：设置页编辑城那一级、只读列出各楼的覆盖，房间那一级的值会治理一个 run 却不出现在页面上；房间的文件又是会话写记录的地方（§8-14）。楼一级整值覆盖城一级，同 `[[mcp]]`。
- **写面**：`write_search` 只写城那一级（设置页只编辑那一级），把值包成 `Change::Search`，先调读面同一个校验，过了才进 `change`，`change` 再复读整份文本（§8-4b）。`Default` 也写出 `choice = "default"`，读回的就是写下的。
- **读回来源**：`settled_search` 与 `settled_effort` 爬同一条梯子，连同说出值的那一级答：在某栋楼（或它的房间）的地址上问，答的是这里被什么治理。没有地址指城根，所以设置页要编辑的城那一级由 `city_search` 单独读：它只读城自己那份 `CONFIG.toml`，与梯子读那一级用的是同一个 `stated`，一栋楼自己写了 `[search]` 也不会把楼的值当成城的值答出来（`crates/wire/spec/Answer/Config.lean` §8-86）。没有 `[search]` 的旧 `CONFIG.toml` 照旧读得回来，两者都答 `None`，求值为 `Default`。
- **不在本模块的事**：机密楼不提供 `web_search`、账号怎么选、失败怎么恢复、连接怎么复用，都在 `crates/accounting/spec/Connectors.lean` §8-35。本模块只回答文件说了什么、它指哪一家。
-/

/-!
### 8-14 一次会话选一次：模型与思考强度

```rust
pub fn write_session(city_root: &Path, addr: &Address, model: &str, effort: Option<Effort>) -> Result<(), AxError>;
```

思考强度放在派活按钮旁边，因为一次会话反正只选一次；而会话选定的模型也写进同一份文件，因为供应的 prompt 缓存对着的正是这两个值。

- **一次写下一个会话冻下的两样东西**：`write_session` 落 `[model] name`，并在人选了强度时落 `[model] effort`。模型总写下（一个模型总在指某个东西），强度只在人说了时写下：缺席不是一个值，而是「让供应方决定」，写出来就是把一个没人做的选择记成记录。
- **写进那一层，而不是另存一份**：选择落到会话自己房间的 `CONFIG.toml`，由已有的 city → building → room 阶梯解析。第二个存处就是第二个答案。
- **这份记录是会话的，不是运行时的设定**：一个 Run 用哪个模型仍由 endpoint book 选，强度仍爬同一条梯子——两者决定会话从哪里开始。房间写下来的只是它当初从哪里开始，所以登记面之后搬了家，是下次派活拒掉的分歧，而不是它默默执行的变更（`crates/sprawling/Spec.lean` §8-79`）。
- **只改 `[model]` 表里的键**：文件里其它键是人写的，读出来、改一个值、写回去，与其余写面同走 §8-4b 的那一条写路径。文件读不动或解析不了就**拒绝**，不覆盖——一份本构建看不懂的配置不是可以随手盖掉的配置。
- **只写 Resident 层**，层级不由调用方给：派活按钮旁边选的强度属于这一次会话的房间。需要按层写强度时，签名要多一个 `Layer` 参数，那是一次公开面变更。
- **落点就是地址自己的 `CONFIG.toml`**，所以这个房间跑的 Run 读得到、改不了自己的档位；地址就是楼时它就是楼自己那份文件，也就是楼根上的会话（§8-4 的 `own_layer`）。
-/

/-!
### 8-31 `[clock]`：一层说结果多久带一次时钟行（`config_layers::clock`，形状 1 判定）

**接口**：`CONFIG.toml` 的 `[clock]` 表，一个键：

```toml
[clock]
stamp = "minute"   # "off" | "minute" | "five_minute" | "hour"
```

值的拼法是 `kernel::ClockStampGranularity` 的 serde 拼法，本模块不另写一份；`ConfigLayer::clock_stamp() -> Option<ClockStampGranularity>` 报这一层说了什么，`load` 用 `ladder.resolve(ConfigLayer::clock_stamp)` 把三级求成 `FrozenConfig.clock_stamp`，谁也没说时落到 `kernel::consts_policy::CLOCK_STAMP_DEFAULT`（`Minute`）。时区梯仍传空的 `LayeredValue`。

**拒什么**：`[clock]` 表 `deny_unknown_fields`。`zones` 与任何别的键、拼不出的值（`"minutes"`）都在解析时拒，走本模块既有的那一种拒法（`refuse::unreadable`）：主体是 serde 点名的键或值与它接受的集合，恢复语是「under `[clock]`, change the value the message names, or take that key out」。

**读者**：粒度只在 `runtime::clock::StampGate` 里起作用（`crates/runtime/Spec.lean` §8-10）；生产的每一跑由装配层按冻结下来的值造一个 `StampGate`（`crates/sprawling/Spec.lean` §8-125）。
-/

/-! D6 定规：`[resident] harness` 上梯子，会话记录压过它

**决定**：`[resident] harness` 按城／楼／房间的梯子取最近一级（`settled_harness`），`[model] name` 仍只是地址自己那一层的会话记录。同一层两键并存在解析时拒。地址自己那一层有会话记录时，`settled_harness` 答 `None`，梯子上的 harness 从 `/new` 开的下一段会话起生效。`change` 落盘前用 `ConfigLayer::parse` 读一遍自己要写下的字节，读不回的不写。

**理由**：房间在派活时才开，人事先只能把 harness 写在楼层或城层，所以它必须上梯子；`[model] name` 是城在会话第一次 run 时写下的记录，别的房间继承它就把一段会话的选择变成了别人的默认。会话记录压过梯子，一段会话就只有一个居民，provider 对这段会话缓存的前缀也不会在中途失效（`crates/sprawling/Spec.lean` §8-79）。写路径复读一次，是因为地址就是楼时楼层与房间层是同一个文件：派活路径若在别处漏了分流，把 `[model] name` 写进点名了 harness 的楼，每个读者都会拒这份文件，整栋楼派不出活；复读让这件事停在写之前，而判定仍只有 `parse` 一处。

**被否**：①梯子上的 harness 压过会话记录：改了楼层下一次派活就换居民，一段会话前后两截的记录说的是两种居民；②把两个键收成一个枚举字段：一个是城写下的记录、只读本层，一个是人写下的设定、爬梯子，合成一个值会暗示两者按同一条规则求值，而私有字段加唯一构造点已经让两键并存的值构造不出来；③写路径只靠派活路径先分流、自己不复读：分流住在另一个 crate，一处疏漏的代价是一整栋楼。

**重开参数**：harness 的会话也要冻下一条房间层的记录时（例如一段 harness 会话要跨 run 续上），房间层的记录要能说 harness，同层互斥与记录优先要一起重议。
-/

/-! D24 `[search]` 是三臂的值，`Custom` 不回落到缺省那一家，缺省那一家只在 `config_layers::search` 声明

**决定**：`SearchConfiguration` 是 `Default`／`Custom { selected, suppliers }`／`Off` 三臂；`Custom` 只接 `selected` 点名的那一家，这一家连不上、`tools/list` 里没有它的远端工具或调用失败时，`web_search` 交回失败的工具结果，不换到缺省那一家，也不换到列表里的另一家。缺省那一家（Exa）的地址、远端工具名与参数映射只在 `default_search_supplier` 写一次，设置页经 `Query::Config` 读它。

**理由**：一次搜索把 run 写下的查询文字交给一个城外的服务。人选了一家，换到另一家就是悄悄换掉了数据的接收方，而且没有一行记录告诉人这件事。`selected` 加 `Off` 让三种意图各有一个拼写：要缺省、要这一家、不要。缺省那一家的参数名照实测写：对 `https://mcp.exa.ai/mcp` 做只读的 `initialize` 与 `tools/list`，服务自报 `exa-search-server` 3.2.1、协议版本 `2025-06-18`；`web_search_exa` 的 `inputSchema` 必填 `query`（string，至少 1 字符）与 `objective`（string，1–4096 字符），可选 `numResults`（number），`additionalProperties` 为 false。所以缺省映射带 `objective`，`web_search` 对这一家把 `objective` 列为必填参数（§8-35）。服务现在也收下不带 `objective` 的 `tools/call` 并给出结果，但这只是它眼下的宽容，不是它写下的契约；映射照 `inputSchema` 写，因为 `judge_mapping` 每次调用前对照的正是它。`accounting` 里按名字运行的 `web_search_reaches_the_default_supplier_live` 走生产的 `web_search` 把这三个参数发给真实的服务，并断言答复是非空且不带 `isError` 的结果；它要外网，所以不在必跑的套件里。

**被否**：①每家一个 `enabled` 开关、取第一个启用的：两家都启用时谁说了算要读者去猜，全都关掉与 `Off` 又是同一件事的两种拼法；②`Custom` 失败时回落到 Exa：见理由，数据接收方被换掉；③把缺省地址写进 kernel 常量或页面：同一个事实两个家，页面那份会在服务地址变化时与实际调用分叉。

**重开参数**：缺省那一家的 `tools/list` 不再列出 `web_search_exa` 或改了这几个参数，`web_search` 的映射校验会在调用前拒（§8-35），上面那条按名字运行的测试也会变红，那时改 `default_search_supplier` 一处；人要求在多家之间按顺序回落时，重开的是本决定的第一条。
-/

/-! D25 MCP url 用 `url::Url` 解析，凭据的名字与值仍由 `kernel::secret` 判

**决定**：§8-4b 的 url 判定用 workspace 钉版的 `url` crate 解析 scheme、userinfo、host 与 query，并按 percent 解码之后把每个 query 参数的名字交给 `names_a_credential`、值交给 `scan`。

**理由**：url 的语法有 percent 编码、`+`、重复的 `?` 与 IPv6 字面量，自己切字符串会漏掉 `%61pi_key=` 这样编码过的参数名，也会把一个正常的 query 当作凭据而拒掉已有的 MCP 设置。「什么叫凭据」已经有 `kernel::secret` 一处答案，这里只缺一个按标准读 url 的解析器。

**被否**：①手写 url 切分：见理由；②一律拒带 query 的 url：许多服务把非凭据的选项放在 query 里，已有配置会无故被拒。

**重开参数**：`url` crate 的解析与 WHATWG URL 标准在本判定用到的部分出现分歧时。
-/

/-!
## 模型：每一次写都先让读者读一遍

所有写面（`write_session`、`forget_session`、`write_sandbox`、`write_mcp`、`write_search`、`write_second_threshold`、`write_city_setting`、`freeze_naming`）都把要说的话包成一个 `Change`，走同一个 `change`：在文档锁里读出整份文件，读不成一份 TOML 文档就拒；改它要改的键；把改好的整份文本交给 `ConfigLayer::parse`，读者拒了就拒这次写，什么都不落；读者收下才整份换上（§8-4b、D6）。锁与整份换上是 `crates/city/spec/Document.lean` 的模型，这里只说「写下的总是读者收得下的」。

* `change` 交出的文本读者一定收下（`a_change_lands_only_what_the_reader_accepts`）。
* 被拒的一次写不动文件（`a_refused_change_leaves_the_file`）。
* 所以一份读者收得下的配置，经过任意多次写之后仍然收得下（`the_layer_on_disk_stays_readable`）：写面不需要另记一份「哪些键不能并存」，读面是文件语法唯一的权威。
-/

namespace City.ConfigLayers

/-- `change` 的拒绝：原文读不成文档，或改完之后读者不收。 -/
inductive Refused where
  | Unreadable
  | ParserRefused
  deriving DecidableEq, Repr

/-- `change_at`：读出原文，改，交给读者，收下才交出要换上的文本。 -/
def change {Text : Type} (readable parses : Text → Bool) (edit : Text → Text) (onDisk : Text) :
    Except Refused Text :=
  if readable onDisk then
    if parses (edit onDisk) then .ok (edit onDisk) else .error .ParserRefused
  else .error .Unreadable

/-- 一次写之后盘上的文本：收下的换上，拒了的不动。 -/
def land {Text : Type} (readable parses : Text → Bool) (edit : Text → Text) (onDisk : Text) : Text :=
  match change readable parses edit onDisk with
  | .ok written => written
  | .error _ => onDisk

theorem a_change_lands_only_what_the_reader_accepts {Text : Type} (readable parses : Text → Bool)
    (edit : Text → Text) (onDisk written : Text)
    (landed : change readable parses edit onDisk = .ok written) : parses written = true := by
  unfold change at landed
  split at landed
  · split at landed
    · cases landed
      assumption
    · cases landed
  · cases landed

theorem a_refused_change_leaves_the_file {Text : Type} (readable parses : Text → Bool)
    (edit : Text → Text) (onDisk : Text) (refused : Refused)
    (refusal : change readable parses edit onDisk = .error refused) :
    land readable parses edit onDisk = onDisk := by
  simp [land, refusal]

theorem the_layer_on_disk_stays_readable {Text : Type} (readable parses : Text → Bool)
    (edits : List (Text → Text)) (onDisk : Text) (reads : parses onDisk = true) :
    parses (edits.foldl (fun held edit => land readable parses edit held) onDisk) = true := by
  induction edits generalizing onDisk with
  | nil => exact reads
  | cons edit rest later =>
    apply later
    show parses (land readable parses edit onDisk) = true
    cases landed : change readable parses edit onDisk with
    | ok written =>
      simp only [land, landed]
      exact a_change_lands_only_what_the_reader_accepts readable parses edit onDisk written landed
    | error refused =>
      simp only [land, landed]
      exact reads

end City.ConfigLayers

/-!
## 模型：`Custom` 只接它选的那一家

`search_supplier` 的 Lean 读法（§8-4c，D24）。`S` 是一家供应方，`id` 读出它的名字，`exa` 是 `default_search_supplier` 的那一份声明。

* `Custom` 接到的那一家一定在它自己的列表里、名字就是 `selected`（`a_custom_choice_reaches_only_its_selection`）。
* `selected` 不在列表里时答 `none`，不回落到缺省那一家（`a_custom_choice_never_falls_back`）；解析把这种值拒在文件那一格。
* `selected` 在列表里时一定接得到（`a_listed_selection_is_reached`）。

由它派生的 Rust 检查在 `config_layers::search` 的测试里：缺席读作缺省那一家，`Custom` 只取 `selected` 并在它缺席时拒，`Off` 没有供应方，楼一级覆盖城一级。
-/

namespace City.ConfigLayers.Search

/-- `SearchConfiguration`：要缺省、要这一家、不要。 -/
inductive Configuration (S : Type) where
  | Default
  | Custom (selected : String) (suppliers : List S)
  | Off

/-- `search_supplier`：一个 run 的 `web_search` 接哪一家。 -/
def supplier {S : Type} (id : S → String) (exa : S) : Configuration S → Option S
  | .Default => some exa
  | .Custom selected suppliers => suppliers.find? (fun one => id one == selected)
  | .Off => none

theorem a_custom_choice_reaches_only_its_selection {S : Type} (id : S → String) (exa : S)
    (selected : String) (suppliers : List S) (reached : S)
    (answer : supplier id exa (.Custom selected suppliers) = some reached) :
    reached ∈ suppliers ∧ id reached = selected := by
  simp only [supplier] at answer
  exact ⟨List.mem_of_find?_eq_some answer, by simpa using List.find?_some answer⟩

theorem a_custom_choice_never_falls_back {S : Type} (id : S → String) (exa : S)
    (selected : String) (suppliers : List S) (missing : ∀ one ∈ suppliers, id one ≠ selected) :
    supplier id exa (.Custom selected suppliers) = none := by
  simp only [supplier]
  exact List.find?_eq_none.mpr fun one listed => by simpa using missing one listed

theorem a_listed_selection_is_reached {S : Type} (id : S → String) (exa : S)
    (selected : String) (suppliers : List S) (listed : ∃ one ∈ suppliers, id one = selected) :
    (supplier id exa (.Custom selected suppliers)).isSome := by
  obtain ⟨one, member, named⟩ := listed
  simp only [supplier, List.find?_isSome]
  exact ⟨one, member, by simpa using named⟩

end City.ConfigLayers.Search
