# city-SPEC.md

> crate：`city`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

本 crate 是城的空间与身份面；每个模块先补齐 §8 的对应子节，再写代码：

| 模块 | 这个模块回答的问题 | §8 |
|---|---|---|
| `resident` | 谁在跑这个 Run（身份从哪来、给 prefix 贡献什么、做过什么） | 8-1 |
| `policy` | 这栋楼里允许什么（confidential 三条、写域、出网） | 8-2 |
| `building` | 一栋楼怎么被建出来，以及一个地址归哪栋楼管 | 8-3 |
| `config_layers` | 三层配置住哪三个文件，又怎么求成一份 `FrozenConfig` | 8-4 |
| `spine_files` | 一栋楼开局有哪几份文档，一件活的 JOB.md 落在哪 | 8-5 |
| `schedule` | 到点发车：谁在什么节奏上自己开始 | 8-6 |
| `archive`、`library` | 东西存哪里、怎么找回来 | 待写 |
| `office`、`wizard` | OFFICE.md；建城向导与搬家 | 待写 |
| `neighbourhood`、`neighbours_tool` | 这座城有哪些地方，我身边站着谁，我该跟谁说话 | 8-15 |

## 2 验收标准

一个 Resident 跨两个 Run 存活，且**两次 Run 的 resident 段字节相同**（`a_resident_crosses_two_runs_with_the_same_identity_segment`，bin 侧从 `model_called` 的 segments 哈希取证）；无 `URBANITE.md` 的地址落为 Ephemeral 且段文本明说这一点。

## 3 假设与歧义

「Resident 住哪」取**地址即目录**：`<city>/<addr>/URBANITE.md`。Building 模板实例化后若改变目录形状，改的是 `urbanite_path` 一处。

## 4 现状分析

`city` 起初是空壳，本 SPEC 描述的是它的第一批代码。

## 5 权威信源

「空间、身份、历史」的语义（Resident 是身份、活跃 Run 才是开销；一个地址决定三件事）；`docs/templates/URBANITE.md`（这份文件长什么样）；`architecture.toml` 里 city 那些条目。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。

Identity（两态）｜Resident｜Ephemeral｜Dossier｜URBANITE.md。**不引入「persona」「角色」「档案」**——概念名一律英文原词，一个概念一个名字。

## 7 模块边界

**三件邻居的活，及它们各自的主人**（写「X 归 Y」而非「不做 X」）：

- 落盘与历史归 `memory`：本模块**读** `URBANITE.md`，写入与备份归 memory 与 checkpoint。
- Building 规则（confidential、写域、阅览室准入）归 `city::policy`：本模块只答「谁」，不答「他能做什么」。
- 身份的**呈现**归 `web`：Dossier 是数值，界面怎么画它是 web 的事。

## 8 接口先行

```rust
pub enum Identity { Resident(Resident), Ephemeral { addr: Address } }   // 穷尽两态
impl Identity {
    pub fn load(city_root: &Path, addr: &Address) -> Result<Identity, AxError>;
    pub fn segment_bytes(&self) -> Vec<u8>;   // prefix 的 resident 段
    pub fn addr(&self) -> &Address;
    pub fn who(&self) -> String;              // Ledger 记的 actor
}
pub struct Resident { /* addr、urbanite、digest —— 私有 */ }
impl Resident { pub fn addr(&self) -> &Address; pub fn digest(&self) -> B3Hash; }
pub fn urbanite_path(city_root: &Path, addr: &Address) -> PathBuf;

pub struct Dossier { /* 计数与位置 —— 私有 */ }        // 形状 7：投影
impl Dossier { pub fn apply(&mut self, who: &str, record: &EventRecord); pub fn is_live(&self) -> bool; /* … */ }
```

- **文件缺失不是错误，读不动才是**：多数房间没有常驻身份，故 `NotFound` 落为 `Ephemeral`；而一个**存在却读不出**的描述必须报错——静默降级成 Ephemeral 会让同一个地址在两次运行中读到两套指令，且没人看得出来。
- **Ephemeral 的段文本明说「你没有常驻身份」**：给它编一个性格，等于让一个用完即弃的执行体以为自己有历史。
- **Dossier 是投影不是文件**：做过什么已经在 Ledger 里；旁边再存一份摘要就是同一段过去的第二个说法。
- **`is_live` 由计数得出而非由标志位**：标志位需要有人清除，而崩溃之后没有人清除标志位。

### 8-2 city::policy（形状 1 判定＋形状 2 值类型）

```rust
pub const RULES_FILE: &str = "RULES.toml";
pub enum ModelPool { Any, LocalOnly }                 // 穷尽，不是 bool
pub enum UserBrowser { Waiting, At(UserBrowserEndpoint) }   // `usersbrowser` 三种读法里去两种
pub struct UserBrowserEndpoint { /* url、host —— 私有 */ }
impl UserBrowserEndpoint {
    pub fn parse(raw: &str) -> Result<UserBrowserEndpoint, AxError>;   // 唯一构造点
    pub fn url(&self) -> &str;
    pub fn host(&self) -> &str;
}
pub struct BuildingRules { /* addr、policy、write_prefixes、browser、usersbrowser —— 私有 */ }
impl BuildingRules {
    pub fn policy(&self) -> &BuildingPolicy;          // 随每次模型调用出行
    pub fn model_pool(&self) -> ModelPool;
    pub fn usersbrowser(&self) -> Option<&UserBrowser>;
    pub fn write_domain(&self) -> Result<WriteDomain, AxError>;
}
pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError>;
pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError>;
pub fn write_rules(city_root: &Path, addr: &Address, text: &str) -> Result<BuildingRules, AxError>;
pub fn rules_path(city_root: &Path, addr: &Address) -> PathBuf;
```

- **规则是一份 TOML，散文没有另起一份文件**：这份文件先前是 Markdown，读者在**任意一行**上匹配 `confidential:`／`write:`／`review:`／`browser:`／`usersbrowser:`／`desktop:`，于是「How work is done here」里一句以 `desktop = true` 开头的话就授予了宿主机的桌面，而一栋没写 `write:` 的楼落到 `Everything`。两处都朝宽松的一侧失败，那是权限读者唯一不许失败的方向。改成 TOML 之后键只在文法给出键的位置成立，`deny_unknown_fields` 让拼错成为一条消息而不是一次静默缺席，`confidential` 与 `write` 都不再有缺省。**散文留在同一份文件里**，作 `does` 与 `conventions` 两个键：拆成两份文档同样能关掉撞键，代价是一栋楼有两种说法且可以互相矛盾。居民拿到的就是这份文件本身的字节，所以城判定的与 agent 读到的是同一串。

- **confidential 三条各有其守处**：模型池锁本地由 `gateway::endpoint` **在会泄漏的那一端**拒（`req.policy.confidential` 即拒，携三段式）；写域止于本楼子树由 `write_domain()` 在构造点拒；数据可入不可出归出网门。**把兜底放在会出事的那一层**，路由错了仍然拦得住。
- **没有 RULES.toml 是普通楼；有而不声明是错误**：把隐私设置的默认值悄悄取成宽松的那一边，正是这整个面存在的理由。拼写不是 `true`／`false` 同样拒——读起来像笔误的隐私设置不得解析成许可。
- **confidential 楼声明越界前缀＝拒而不裁剪**：静默裁剪会让文件说一套、城做另一套；拒绝会指出该改哪一行。
- **无声明写域时默认只写本楼**：一栋楼至少能写自己，且不多。`prefixes` 里一条读不出的地址**传播而不跳过**——先前它被丢在读它的地方，于是一栋楼写得比人授予的少，而这件事没有任何一处说出来。
- **`review = true` 是楼级开关**：开则每个 Run 得一棵自己的 worktree，写的东西在别人检查并 merge 之前对楼不可见。**默认关**，与 confidential 的「不声明即错」相反——隐私的默认值不得惄悄取宽，而审查纪律的默认值不得惄悄取严：一个人派一个 Agent 去改一行字并盯着看，应当看得到文件变化。拼写不是 `true`／`false` 同样拒。
- **`## Egress` 列可达域名**：`BuildingRules::egress()` 交 `kernel::egress_target` 判定。类型经 `kernel::EgressAllowlist` 重导出，住哪一簇文件是 kernel 内政（`gate::egress`，公共拼写不变）。**confidential 楼同时列域名＝矛盾，拒**——「数据可入不可出」是那个设置的含义，域名表写在它下面会逼读者自己去调和两句话。
- **今天的执行点与仍缺的执行点要分清**：provider 路径已被 `endpoint` 的 confidential 拒守住；Agent 自己发起的出网（exec 的 Program／Shell 臂、浏览器）**没有可拦截处**，因为拦截需要 OS sandbox。判定已就位，拦截尚未落地——在那之前不要说「出网已管住」。
- **`usersbrowser` 一键同时是开关与地址**：值 `"ws://127.0.0.1:<port>/session"` 启用并声明地址，`true` 启用而地址未定（工具每次调用都得到门的问题），absent／`false` 即无此工具。**confidential 楼写这一键即拒**（`E_CONFIG_INVALID`）——附着读的是那个人浏览器里全部登录态，本楼的隔离在那一刻失效。地址的**语法**（`ws://`、主机形状）在此读一次并把 `url`／`host` 一起交出；**loopback 与否是 `kernel::gate::attach` 的政策**，语法不替政策作答。
- **`browser` 与 `usersbrowser` 是两个键**：前者是城自己拉起的浏览器（profile 按楼隔离），后者是人已经开着的那个（人的真 profile）。`browser` 不是 `usersbrowser` 的前缀截断——TOML 的键是文法给出的整体，两个设置因此互不误读。
- **`write_rules` 先求值再落盘**：一份写到一半就不再求值的治理文档会把它那栋楼一起带走。且**整份文档才是单位**：confidential 楼不得列域名，故两行可以各自合法而合在一起非法。

### 8-2b city::rules_tool（形状 4 适配器）

```rust
pub struct RulesTool { /* city_root、building、meta —— 私有；op ∈ {read, propose} */ }
impl RulesTool { pub fn new(city_root: &Path, building: Address) -> Result<RulesTool, AxError>; }
// meta.effect = Effect::Govern
```

- **为什么不是 `edit`**：`RULES.toml` 住在楼的保留子树，没有任何写域到得了那里——这不是一个要绕过的障碍，它就是规则本身。故另开一道门（`Effect::Govern`），而那道门的守卫是人。
- **楼是携入的而不是参数**：工具持调用方自己那栋楼的地址，于是一个 Run 无法靠填另一个名字去改别人的规则。
- **人看得到自己在批什么**：`kernel::gate::govern` 把提案正文截前 600 字写进 `action_desc`。

### 8-3 city::building（形状 2 值类型＋一个实例化动作）

```rust
pub enum BuildingTemplate { Minimal, Confidential, Hall }   // 穷尽；新模板＝新臂
impl BuildingTemplate {
    pub const ALL: [BuildingTemplate; 3];                            // 三个模板，按人被提供的顺序
    pub fn parse(name: &str) -> Result<BuildingTemplate, AxError>;   // 经 ALL 与 name 读回；不认即拒，且报出已知集
    pub fn name(self) -> &'static str;                               // 模板名的唯一拼法
}
pub struct Building { /* addr —— 私有 */ }
impl Building {
    pub fn of(addr: &Address) -> Result<Building, AxError>;   // 一个地址归哪栋楼管
    pub fn addr(&self) -> &Address;
    pub fn root(&self, city_root: &Path) -> PathBuf;
    pub fn holds(&self, addr: &Address) -> bool;
}
pub fn create(city_root: &Path, addr: &Address, template: BuildingTemplate)
    -> Result<Building, AxError>;
pub fn created_payload(building: &Building, template: BuildingTemplate)
    -> Result<Payload, AxError>;
pub fn adopt(city_root: &Path, addr: &Address) -> Result<Building, AxError>;
pub fn adopted_payload(building: &Building) -> Result<Payload, AxError>;          // adopted: true
pub fn configured_payload(building: &Building, wrote: Written)
    -> Result<Payload, AxError>;                              // building_configured
```

- **模板名只有一个家**：`parse` 不再另列一张字符串表，而是拿 `ALL` 里每一个的 `name()` 去比；拒词里的合法集也由同一趟生成。于是加一个模板只改枚举与 `name()` 两处，而「解析认得的集合」与「拒词列出的集合」在类型上是同一个（Roadmap 7.14）。`Hall` 的名字取 `kernel::consts_policy::HALL_BUILDING`：City Hall 是唯一一栋地址由城而不是由人定的楼，模板名与那个地址是同一个词。

- **楼是顶层地址，房间不是楼**：`create` 拒多段地址（`lab/room1` 是 `lab` 里的一个房间）。嵌套楼会使「这个地址归谁管」多出一个答案，而 `Building::of` 取首段这件事今天已被写域、配置与上报对象三处消费。
- **reserved prefix 下建楼恒拒**：`.sprawling/` 是城自己的账与配置，它在一切写域之外；允许在它下面建楼，就是把一个写域开到账本上。判定用 `Address::is_reserved`，不在本模块重写前缀文法。
- **二次出生恒拒**：已有 `RULES.toml` 即拒（同 `init` 拒第二次创世）。覆写会把一栋已在干活的楼的规则静默换掉，而那份规则可能写着 `confidential = true`。
- **模板字节来自 `docs/templates/RULES.toml`（`include_str!`）**：人读的那份模板与城写出的那份必须是同一串字节，否则两份会各自漂。`Confidential` 与 `Minimal` 只差一行（`confidential` 的值），且该差异由 `policy::evaluate` 读回来断言——换行成功与否不靠阅读，靠测试。
- **先落盘再产事件**：`building_created` 记的是已经发生的事。反过来的顺序会让历史声称一栋目录不存在的楼存在，而重放会把这个谎再说一遍。
- **只写不读的 payload**：`created_payload` 只有写面，因为今天没有读它的投影——`CityView` 的楼列表读盘（assembly 的 `read_spine`）。读面随第一个真正需要它的投影落地，不提前建。
- **adopt（兑现「导入一个已有目录」这件事）**：收编一个已存在的目录为楼。复用 `create` 的全部围栏（房间拒、reserved 拒、二次出生拒），只多一条：**目录不存在即拒并指向 create**——收编不存在的东西是建造，两个动词不共用一个事实。Spine 文档恒不覆写（§8-5 既有约束），故被收编目录的 `Roadmap.md` 保持原主的字节；事件仍是 `building_created`，但 payload 携 `adopted: true`——历史不得声称它建造了它只是找到的东西。CLI 入口 `sprawling adopt <city> <addr>`；城外目录先由人搬入城内再收编，本体不做拷贝。

### 8-4 city::config_layers（形状 1 判定／求值，兼任文件名权威）

```rust
pub use kernel::layout::CONFIG_FILE;               // 文件名的权威在 kernel::layout
pub enum Layer { City, Building, Resident }        // 穷尽三级，与 kernel::LayeredValue 同形
pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError>;
pub struct ConfigLayer { /* model / effort / sandbox / mcp / shelves —— 私有 */ }
impl ConfigLayer {
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError>;   // 纯函数，无 I/O
    pub fn model(&self) -> Option<&str>;                        // 这一级冻下的模型，照写下的读回
    pub fn effort(&self) -> Option<Effort>;
    pub fn shelves(&self) -> Option<&[String]>;                 // 这一级声明挂载的外部书架目录；只有 City 级可以（§8-8）
}
pub fn load(city_root: &Path, addr: &Address) -> Result<FrozenConfig, AxError>;
pub fn own_layer(city_root: &Path, addr: &Address) -> Result<ConfigLayer, AxError>;
pub fn settled_effort(city_root: &Path, addr: &Address)
    -> Result<Option<(Effort, Layer)>, AxError>;      // 值连同说出它的那一级

// config_layers::ladder（crate 内）
impl Layer {
    const ALL: [Layer; 3];                                              // 由远及近
    fn file(self, city_root: &Path, addr: &Address) -> Result<PathBuf, AxError>;
}
pub(crate) struct Ladder { /* Vec<(Layer, ConfigLayer)> —— 私有，由远及近 */ }
impl Ladder {
    fn read(city_root: &Path, addr: &Address) -> Result<Ladder, AxError>;
    fn tagged<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<(T, Layer)>;
    fn resolve<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<T>;
}
```

**一条梯子是一个值**：`Ladder::read` 按 `Layer::ALL` 由远及近读一遍，落点重复的一级丢弃；`load` 逐个关切在梯子上 fold，不再逐级点名。加一级因此是 `Layer` 多一个臂：`ALL`、`file` 与 `resolve` 三处穷尽匹配同时报编译错，直到新一级被安置，而每个关切一次拿到它。`resolve` 是「哪一级填 `LayeredValue` 的哪一格」的唯一一处答案——今天 `kernel::LayeredValue` 只有三格，所以 C 章要加的人层（`~/.sprawling/config.toml`）落地时，`kernel::config` 与本模块在同一次改动里走完。

**来源与值一起答**（叶子 3.3）：`settled_effort` 与 `load` 爬同一条梯子，区别只在它把说出这个值的那一级留着而不是丢掉。只被告知结果的设置页说不出「这是这间房自己写的」还是「这是全城都有的」，于是它只能把三份文件各读一遍、把同一条梯子再爬一次——**同一个问题两个答案，就是从第二次爬梯开始的**。谁压过谁仍由 `kernel::LayeredValue::resolve` 判：`tagged` 只负责「哪一级填哪一格」，`resolve` 是 `tagged` 去掉那一级，所以这条映射在本 crate 里只有一处。`None` 是整条梯子什么都没说，也就是这座城有意把强度交给供应方，而不是替人填一档。

**`own_layer` 答的是另一个问题**：梯子回答「一个 Run 被什么治理」，它回答「这个地址自己写下了什么」。差别正是它存在的理由——城或楼那一级给出的默认值不是这个地址做的选择，所以它不能当作选择的记录。文件是地址自己的那份 `CONFIG.toml`（`Layer::Resident` 的落点，也就是会话写的那一份），哪怕梯子把同一份文件当作两级里更远的级读了一次；地址就是楼时两者是同一个文件，所以那种地址自己就是它的会话（§8-14）。

**门面上换名**（`lib.rs` 按能力组织，不按文件组织）：`load` 已归 `policy`，故本模块对外是 `city::load_config`；`path` 对外是 `city::config_path`；`building::create` 对外是 `city::create_building`，`created_payload` 对外是 `city::building_created_payload`（名字读起来就是它记的那个事件）。

- **文件名只有一份，层级由位置决定**：City 层住 `<city>/.sprawling/CONFIG.toml`（reserved prefix 内，因此任何 Resident 的写域都永远叠不上它——「Agent 改不了自己的配置」因此是判定而非推理）；Building 层住 `<city>/<building>/CONFIG.toml`；Resident 层住 `<city>/<addr>/CONFIG.toml`。三处同名，读者认一次就认得完。
- **地址就是楼时只有两级**：`addr` 与它的 building 相同时，下两级指向同一个文件，只读一次并放在 Building 级。同一份文件在两级各算一次不改变结果，却会让读者以为它能覆盖自己。
- **缺文件不是错，读不动才是**（同 `resident`）：未声明即每级 `None`，落到 `kernel::consts_policy` 的缺省；一份存在却读不出的配置报 `E_STORAGE_FATAL`。
- **不认的键即拒**（`deny_unknown_fields`）：静默忽略一个拼错的键，会产生「我设了 effort 而什么也没发生」这个无从诊断的状态。本版读哪些键，由 `ConfigFile` 的字段给出，此处不复述；拒绝文字也不复述这张键表——serde 的报错点名不认识的键、列出该表接受的键，恢复语只从原文里取出报错所在的那张表头（`[model]`、`[[mcp]]`）并说改哪一节。`[clock]` 等到它在真城里有消费者时再受理，在那之前写它得到的是一句拒绝而不是一份沉默。
- **梯子不在本模块重建**：下层胜上层由 `kernel::LayeredValue::resolve` 给，冻结由 `kernel::freeze` 给；本模块只回答「哪三份文件、怎么读」。一条规则一个权威。
- **effort 属 `FrozenConfig` 而非 `LiveConfig`**：改它会作废 message cache breakpoints，因此改动只影响下一个 Run（理由已写在 `kernel::config`，此处不重述只遵守）。

**`[[mcp]]` 一节**：`CONFIG.toml` 第三节 `[[mcp]]`（表数组）。`label` 解成 `kernel::ServerLabel`，非法即拒并报出是哪一份文件；其余字段按这台服务器怎么够得到分两组：

- **`command`（非空）＋ `args`（缺省空表）＋ `env`（缺省空表，形如 `env = { API_KEY = "secret:mcp/apps" }`）**——城所在的机器上的一个程序。写成表而不是 `NAME=value` 行的列表：文件里一个名字只出现一次，也没有谁要去切一个人写的字符串。
- **`url`（非空）＋ `headers`（缺省空表，写法同上）＋ `transport`（`"http"`｜`"sse"`，缺省 `"http"`）**——一个地址。`transport` 是闭集而不是自由文本，拼错在写它的地方就被拒，而不是变成一台没人够得到的服务器；缺省取 `http`，因为「发一条消息过去」正是一个 url 的本义。

两组各自成行：`command` 与 `url` 同时出现即拒（读者要去猜），两者都不出现也拒。`command` 一行再写 `transport` 同样拒——命令走它自己的管道，再指一条流就是一行说了两种 transport。两张表的值都可以是 `secret:realm/name` 引用，兑付不在本模块（sprawling-SPEC §8-4／§8-15）。两条口径：①**同一层内标签不得重复**——两个同名 server 会让同一个工具名同时指向两个进程，而那是一个路由错误而不是一个偏好；②整表上梯（下层写即替换上层全表），语义住 kernel-SPEC §8-22，此处不复述。**不在本模块的事**：进程怎么起、起不来怎么办、confidential 楼凭什么拒——那三件全在装配层（sprawling-SPEC §8-4），本模块只回答「三份文件说了什么」。

**`[sandbox]` 一节**：`CONFIG.toml` 第二节 `[sandbox]`，字段 `shell`（bool，默认 false）、`fuel`（整数，缺省取 `SANDBOX_FUEL_DEFAULT`）、`mounts`（相对 city root 的路径表，reserved prefix 在解析点即拒）。解析仍是「本版本不读的键即拒」——被写下却什么都不发生是唯一没人能诊断的状态。整节整值上梯，语义住 kernel-SPEC §8-22，此处不复述。

**`[sandbox]` 的第四个字段**：`env_passthrough`（字符串表，缺省空表），逐项解成 `kernel::EnvVarName`。与 `mounts` 逐条同形：同一节、同一次整值上梯、同一个解析点拒——`mounts` 在这里拒保留区，`env_passthrough` 在这里拒凭据形状的名字，而“什么叫凭据形状”是 `kernel::secret` 的答案，本模块不重建它。拒绝文字里带着是哪一份文件、哪一个名字，因为一份配置被拒时人手里只有那句话。

**`[sandbox]` 的第五个字段**：`trusted`（字符串表，缺省空表），逐项解成 `kernel::ServerLabel`。它答的是「这层楼允许哪一台 connector 服务器去做城里谁都收不回的事」（今天只有运行这座城的机器的桌面），语义与读者住 `kernel::SandboxLimits::trusts`，本节只管它在 TOML 里怎么写、在哪一层写、写错了在解析点怎么拒。与 `mounts`／`env_passthrough` 同形：一名一行、缺省为空、整节整值上梯——收不回的效果按服务器逐台放行，而不是一次放宽给所有人。

### 8-5 city::spine_files（形状 6 数据面＋落盘动作）

```rust
pub const ROADMAP_FILE: &str = "Roadmap.md";
pub const JOB_FILE: &str = "JOB.md";
pub const CITY_FILE: &str = "City.md";
pub(crate) const MEMO_FILE: &str = "Memo.md";      // 尚无外部读者
pub const HANDOFF_FILE: &str = "Handoff.md";                      // 红测要点名它
pub const AGENTS_FILE: &str = "AGENTS.md";                        // 项目自带的约定；城不写也不拥有

pub struct JobBrief<'a> { pub task: &'a str, pub goal: &'a str, pub budget: &'a str }
pub enum RunBrief { Job { text: String }, Principal }          // 穷尽两臂
pub(crate) fn lay_out(building_root: &Path, addr: &Address) -> Result<(), AxError>;  // 唯一调用方是 building::create
pub fn job_path(city_root: &Path, addr: &Address) -> PathBuf;
pub fn roadmap_path(city_root: &Path, building_addr: &Address) -> PathBuf;
pub fn roadmap(city_root: &Path, building_addr: &Address) -> Result<String, AxError>;
pub fn write_job(city_root: &Path, addr: &Address, brief: &JobBrief<'_>) -> Result<String, AxError>;
pub fn write_brief(city_root: &Path, addr: &Address, brief: &JobBrief<'_>) -> Result<RunBrief, AxError>;
pub fn handoff_path(city_root: &Path, building_addr: &Address) -> PathBuf;
pub fn handoff(city_root: &Path, building_addr: &Address) -> Result<Option<String>, AxError>;
pub fn norms(city_root: &Path, addr: &Address) -> Result<Vec<PathBuf>, AxError>;
```

- **四文档三写一不写**：`lay_out` 写 Roadmap／Memo／Handoff；`RULES.toml` 归 `building::create`（它的含义归 `policy`）——同一份文件有两个写入者就是两个权威。
- **已存在的文档恒不覆写**：一栋已在干活的楼的计划不得因为又跑了一次建楼而回到空白。
- **模板的占位行不进新楼的 Roadmap**：`docs/templates/Roadmap.md` 里的两行 `Not started` 是给人看的例子；照抄进去，一栋新楼开局就有两件不存在的待办，而它们会进分母。实例化时删掉 Item 列为空的数据行，断言是「新楼的分母是 0」。
- **JOB.md 先落盘，再产 `run_started`**（模板第一行就这么写）；内容同时进 CAS，于是盘上那份是现场、CAS 那份是历史——Agent 改了 JOB.md 也不会使「当时派的是什么活」不可考。同一个房间再派一件活即覆写它（JOB.md 是本次会话的任务，不是档案）。
- **机器只填它知道的段**：Task／Goal／Budget 三段有事实就写；Background／Delivery 无事实则不写——写一个 `(未知)` 占位，只是让模型每回合读一遍没信息的行。
- **一次会话的 brief 只有两种，且由本次派活决定**：说得出 Goal 的就写 `JOB.md`（`RunBrief::Job`），说不出的就不写（`RunBrief::Principal`）。**依据选 Goal 而不选「盘上有没有 JOB.md」**：一个房间里上周留下的任务书仍在盘上，它可以被读，但不得冒充一次没人派任务的会话的 brief。Goal 是那份表单里唯一不可替代的一栏（什么时候停），它空着就等于告诉 Agent「停不停没定义」。
- **`handoff` 不把空白表单当交接件**：一张没填过的 `Handoff.md` 与一张填过的占同样的 prefix 字节而一个字的信息也不带。识别靠模板自己的括号提示行。
- **第三件事不再被并进 `None`**：原先 `.ok()?` 把「不在」「读不了」「空白表单」三件事归为一个 `None`。现在 `None` 只说「没有值得带走的东西」，读不了则以 `E_STORAGE_FATAL` 上报并带路径——与同模块的 `roadmap` 同形。下一次会话正是从这份文件装配的，静默省略等于告诉它上一次没留下任何东西。
- **计划的路径与读法归本模块**：`roadmap_path` 与 `roadmap` 落在这里，因为 `ROADMAP_FILE` 在这里——在别处拼 `city_root/<addr>/Roadmap.md` 就是第二份「计划在哪里」的权威，它会在真正那份搬家后继续跑得好好的。
- **「还没有」与「读不了」是两件事**：`roadmap` 仅对 `ErrorKind::NotFound` 答空串——一栋还没铺计划的楼确实没有计划；其余任何理由一律以 `E_STORAGE_FATAL` 上报并带上路径。这与同 crate 的 `archive::index` 已有的契约同形（目录不在→`Ok(空)`，真失败→`Err`），不新立一种读法。
- **`handoff` 未改**：它的 `.ok()?` 同属一族，但它把「不在」「读不了」「空白表单」三件归为一个 `None`；这三件在 prefix 里各自应当怎么表现，尚未定。
- **规范类 must-read 由 `norms` 给路径，不给 Locator**：Locator 需要 CAS 或 git oid，而 city 不认识落盘物（拓扑上也依赖不到 memory）。本模块答「哪几份是规范」，装配层把它们入 CAS 变成 Locator。这也是 must-read 最大失败模式的解：不让模型凭记忆重抄规范清单。

### 8-6 city::schedule（形状 1 判定＋形状 6 数据面）

```rust
pub const SCHEDULE_FILE: &str = "SCHEDULE.toml";
pub enum Cadence { EveryMinutes(u64), DailyAt(u64), WeeklyAt(u64) }   // 穷尽
pub struct Entry { /* name、addr、task、goal、cadence —— 私有 */ }
pub struct Schedule { /* entries —— 私有 */ }
impl Schedule {
    pub fn parse(text: &str) -> Result<Schedule, AxError>;
    pub fn load(city_root: &Path) -> Result<Schedule, AxError>;
    pub fn due(&self, after: TimeMs, now: TimeMs) -> Vec<&Entry>;     // 时间只入参
    pub fn due_after(&self, after: TimeMs, now: TimeMs) -> Vec<(Address, String, String)>;
}
```

- **一个窗口里每条最多回一次**：错过八小时的整点活欠一次运行而不是八次。窗口多宽由调用方定——bin 的 `tick` 把起点设在开机那一刻，于是**关机期间的活不在开机第一分钟补跑**；计时源是命令台的有限等待，故不新开线程也不往线格式加 `Tick`。
- **恒 UTC**：节奏按 epoch 分钟数整数运算，无历法依赖。关切时区是呈现面的事（ClockStamp），而一份依赖会动的时区库的日程会在重放时换一个时刻发车。
- **日历形状（day-of-month／month）明拒**：它们需要一部历法，而历法需要一个权威，城里还没有；拒词写明这一点，而不是近似成「每 30 天」。
- **一个 job 只许一个节奏**：写了两个即拒——排名它们等于替用户做一个他没做的决定。
- **`due` 与 `due_after` 并存**：前者是本模块自己的公共面（返回引用，调用方自组装），删它是 breaking；后者是运行级依赖快照的最小形态（返回可直接 dispatch 的三元组，调用方只剩循环）。区间判断一处定义（`due`），`due_after` 只做拥有权转换，不复述窗口语义。

### 8-7 city::watch（形状 6 数据面＋形状 1 判定）

```rust
pub const WATCH_FILE: &str = "WATCH.toml";
pub struct Source { /* name、matches、addr、starts_work 私有 */ }
impl Source { pub fn name(&self) -> &str; pub fn matches(&self) -> &str; pub fn addr(&self) -> &Address;
              pub fn starts_work(&self) -> bool; pub fn building(&self) -> &str; }
pub enum Link { Live { since: TimeMs }, Down { since: TimeMs } }
pub struct Watch { /* sources 私有 */ }
impl Watch {
    pub fn parse(text: &str) -> Result<Watch, AxError>;
    pub fn load(city_root: &Path) -> Result<Watch, AxError>;
    pub fn listening(&self, standing: &[Address]) -> Vec<&Source>;
}
pub fn watch_path(city_root: &Path) -> PathBuf;
```

- **形状同 `schedule`**：盘上一张表、一个纯问题、答案是派活。差别在触发源：日程因时间流逝而响，城自己看得见；watch 因别处发生了事而响，城看不见。
- **本地恒不轮询**：持连接的服务推过来，城只负责接。轮询是一份无人阅读的定时流量，也是第二份「谁先到」的权威。
- **`starts_work` 默认 false**：外来事到达本身不是花一次模型的理由。它为 true 时表示**人事先核过这个来源**——这与 `collab::triage` 的「污染件不自行开工」不冲突，两者答的是不同问题（详见 ARCHITECTURE.md §10）。
- **楼拆即不再听**：`listening` 按现存楼过滤，而不是去改用户写的文件——文件是人的，城替人改文件就是在回答一个没人问的问题。
- **两条边都入 Ledger**：`Link` 的 Live 与 Down 各携时刻，于是「这栋楼何时没在听」是可读事实而非猜测。不做断线补投。

### 8-8 city::library（形状 2 值类型＋形状 1 判定）

```rust
pub use kernel::layout::{BUILDING_SHELF, LIBRARY_DIR};   // 住 reserved prefix 之下
pub enum Shelf {
    Library(Address),
    Building(Address),
    External { index: u32, path: String },
}
impl Shelf { pub fn address(&self) -> Option<&Address>; }
pub struct Holding { pub name, pub section, pub disclosure, pub hash, pub shelf: Shelf }
pub struct Library { /* BTreeMap<ShelfKey, Holding> —— 私有，ShelfKey 由 name 造 */ }
impl Library {
    pub fn scan(city_root: &Path, building: Option<&Address>, home: &Path) -> Result<Library, AxError>;
    pub fn all(&self) -> Vec<&Holding>;
    pub fn sections(&self) -> Vec<&str>;
    pub fn reading_room(&self, admitted: &[String]) -> Vec<&Holding>;
    pub fn missing(&self, admitted: &[String]) -> Vec<String>;
}
// city::config_layers
pub fn city_shelves(city_root: &Path, home: &Path) -> Result<Vec<PathBuf>, AxError>;
```

- **中央库存与阅览室的分工是常驻上下文不随磁盘膨胀的唯一原因**：盘上躺一千件与本楼的常驻字节无关；进 catalog 的只有 `RULES.toml` 的 `reading_room` 列出的那几件。
- **库存住 reserved prefix**：Agent 读得到、写不了。否则一个 Agent 可以给自己发一件 SKILL，而那正是准入清单存在的理由。
- **一行式条目取作者写的第一行**，不生成摘要：摘要的摘要是消化产物，而消化产物默认可疑。
- **清单上没有的名字不进 catalog、也不报错**，只留一行诊断给写清单的人——承诺一件取不到的技能比它不在还糟。
- **书架按名字建键，section 降为字段**：阅览室按名字准入，所以「同一件」必须也按名字判定。键是类型化的 `ShelfKey`，由 `ShelfKey::of` 一处造出，上架与查清单两端都经它——按 `(section, name)` 建键时同名跨 section 的两份同时在架，「近架盖远架」因而在跨 section 时不成立。楼架后插，于是楼自己的那一份替换城里的那一份，哪怕两者归档在不同 section。
- **身份是名字，hash 不是身份**：`Library` 按名建键、`reading_room` 按名准入，于是每一件都由名字唯一说出；hash 答的是另一个问题——这份文档变了没有、哪一次 run 读的是哪份字节（§8-13）。把两问混为一问，会把「同名改了内容」读成换了一件，把「同一份挂在两处」报成两件。要按 hash 标「亦见于」，先得让 `Holding` 留下被近架盖住的那一份（hash 本身已在 `Holding` 上，缺的是被盖住者的落点与身份），再给答案加一列读它；今天没有这一列，所以「按 hash 判定身份」只是一个没有读者的说法。
- **读架上的失败逐条上报**：目录项读不动、目录名或文件名不是 Unicode，都带路径报 `E_STORAGE_FATAL`。一件静默缺席于每一间阅览室的 skill，是人从 catalog 上看不出来的那一种故障。
- **一格书架加一个落点是一个值，不是两个字段**（`Holding::shelf`）：一个持有只在一格书架上、只在一个落点上，两个字段允许「说 library、指向城外的文件」这个任何书架都进不了的状态。三条臂正好是 skill 能在的三个地方，没有第四条；哪一条由**扫盘时读的那个根**给出，不从地址反推——反推只对「两格书架碰巧落在不同地方」成立，而那是个巧合而不是规则。
- **`Shelf::address()` 答的是 catalog 承诺一件 skill 之前要问的那个问题**：一个条目靠地址打开，城外书架上的文件没有地址。所以**不发明一个假地址**：拼一个看起来像 `Address` 的字符串，会让读者去开一个并不在那儿的文件，且失败发生在第一次 `read` 而不在写下列表的那个时候。
- **外部书架只读挂载，路径由城自己的配置给出**（`[skills] shelves`，城层一份）：`city_shelves` 在城的那一级上读它，而不是走三层梯子——书架是这座城所在的文件系统上的一个目录，它对每一栋楼同时挂上，让楼或房间能自己挂一本就是让一个作用域准入一份没人选过的文件。楼或房间写下它即在读文件处拒（`E_CONFIG_INVALID`，恢复语说把它移进城自己的 `CONFIG.toml`），而不是解析后丢掉——一份被接受却什么都不发生的配置，写它的人无从诊断。
- **外部书架的布局属于写它的那个 harness**：一层目录一件 skill、目录里放 `SKILL.md`（`SKILL_FILE` 是本城写下这条布局的**唯一一处**）；目录名就是 skill 名。**section 为空**：那棵树没有 section 这一级，替它编一个就是本城对一份它不拥有的东西的猜测。不在那里、不是目录的外部路径直接跳过（空架就是空架）；是文件而不是目录则以 `E_CONFIG_INVALID` 拒并报出是哪一条——一句「配置写了却什么都没发生」是没人能诊断的状态。
- **外部路径里的 `~` 指这个人自己的 home，home 以参数传入**：读环境不是本 crate 的活（`bin::assembly` 给出 `Home`），因此测试扫的是测试自己造的目录。committed 的城配置里不放一台机器的绝对路径，这正是 `~` 存在的理由。不是 `~` 开头也不是绝对路径的条目在解释处即拒，恢复语说出这条规则。
- **同一名的优先级是 building > library > external（外部按数组序）**：按最远的架先上、近的盖上去，于是城自己的存货盖过别的程序的目录——城留下一个名字时，那个名字指城的 skill；楼的自己一份又盖过城的。外部条目排到最后，因为它是别人写的：一份目录不是一个权威。
- **目录的架次由 `Shelf` 的臂序给出**：`Library::all` 先按 `Shelf::catalog_position`——城库、楼架、外部架按 `[skills] shelves` 的数组序——再按 `(section, name)`。位置是对三条臂的穷尽 match，不另立一张次序表；加一条臂而不在这里安置它，本 crate 编译不过。外部持有者没有 section，若把全部持有者按 `(section, name)` 排，别人的目录会排到城自己的存货之前，那正是上一条优先级的反面。
- **`Holding` 不再携 `path`**：落点由 `Shelf` 说出（城内的两个臂就是地址），而多一个 `PathBuf` 就是同一件事的第二个家，且两层书架下必有一个是错的（§8-12 对 `holding_address` 的同一条理由）。
### 8-10 city::wizard（形状 1 判定＋形状 2 值类型；含 survey）

```rust
pub enum Standing {
    Empty,
    Work { adoptable: Vec<Address>, loose: usize },
    AlreadyACity,
}
pub fn survey(entries: &[(String, bool)], has_history: bool) -> Standing;
```

- **「目录非空」不是一个人能据以行动的答案**：三个臂各自说清接下来会发生什么，因为指着自己干了一年的文件夹的那个人要知道**会往他的工作旁边放什么、不会动什么**。
- **收列表而不是收路径**：本模块的全部纪律就是「决定是值，落盘是二进制的活」；判断一座城形成在这里会做什么，不该需要一块磁盘。
- **地址语法拼不出的名字算 loose**：一栋 dispatch 不到的楼不是楼。点目录、含 `:` 或 `\` 的名字都归此类；**带空格的名字语法收得下，故照样可作楼**。
- **排序后再答**：两台机器读同一个目录必须得出同一个答案。

```rust
pub struct CityPlan { /* dirs、first 私有 */ }
impl CityPlan {
    pub fn new(first: Option<(&str, &str)>) -> Result<CityPlan, AxError>;   // 一句指令的城
    pub fn dirs(&self) -> &[Address];
    pub fn first(&self) -> Option<&(Address, BuildingTemplate)>;
}
pub struct Relocation { pub from: Address, pub to: Address, pub crosses_building: bool }
pub fn relocate(from: &Address, to: &Address) -> Result<Relocation, AxError>;
```

- **两件都是判定，不是动作**：新城由什么构成、一次搬家蕴含什么，在这里以值给出；建目录与落事件归 bin。这个切分使「一句指令建一座城」可以在不建城的条件下被断言。
- **搬家恒不是改名**：一个 Address 同时决定写域、默认上下文与上报对象，所以搬家是换写域，而**历史留在它发生的地方**。一座会为了配合新地址而改写历史的城，「这件事是在哪儿做的」就没有答案了。
- **恒不得搬到楼根**：住在楼根等于从侧门拿到整栋楼的写域。
- **`city::office` 已并入 `config_layers` 并删行**：三层配置（City／Building／Resident）已是完整的梯子，OFFICE.md 没有任何一条自有规则，第四层只会成为「同一个设置在哪儿写」的第二个答案。

### 8-15 city::neighbourhood（形状 1 判定＋形状 2 值类型）

```rust
pub enum Occupancy { Resident { bring: String }, Empty }   // 穷尽两态
pub struct Neighbour { pub addr: Address, pub name: String, pub occupancy: Occupancy, pub waiting: u32 }
pub struct Neighbourhood { /* building、rooms: Vec<Neighbour>、buildings: Vec<Address> —— 私有 */ }
impl Neighbourhood {
    pub fn scan(city_root: &Path, building: &Address, me: &Address,
                waiting: &dyn Fn(&Address) -> u32) -> Result<Neighbourhood, AxError>;
    pub fn building(&self) -> &Address;
    pub fn here(&self) -> &[Neighbour];        // 本楼，除我之外的每个地址
    pub fn buildings(&self) -> &[Address];     // 全城，只有楼名
    pub fn residents(&self) -> u32;            // here 中真有人站着的个数
}
// 房间与楼的枚举各归其既有权威，本模块只调用：
pub fn room::all(city_root: &Path, building: &Address) -> Vec<Address>;   // city::rooms
pub fn building::all(city_root: &Path) -> Vec<Address>;                   // city::buildings
```

- **这栋楼里有言语，却没有地址簿**：`signal` 的 `to` 只说「the address you are speaking to」，越界拒词只报边界不报住户，于是地址靠猜；而装配层投递时 `.entry(room).or_insert_with(new_inbox)`，**猜错的一句话会当场开出一个没人读的信箱并回 `queued: true`**。本模块存在的第一个理由是让那次猜测消失。
- **`docs/templates/URBANITE.md` 早就承诺了这件事**：模板原话是「other agents and the person read it to know what to expect from them and what to bring to them」。承诺写在模板里，兑现它的代码在本模块。
- **一行取自 `## Bring them`，取不到才退回第一段正文**：这一行要回答的是「我为什么找他」，而模板里正是那一节写「什么样的活属于这位住户」。退回规则跳过标题行与引文行——引文行是模板留给作者的说明，把它显示出来等于让全城住户共用一句自述。**这与 `library::first_line` 不是同一条规则**：书架条目取的是标题，住户名册取的是正文，两种文档、两条规则、两个家。
- **准入判定复用 `Identity::load`，不自读文件**：「一个地址上有没有常住的人」已经有权威，第二次实现必然在某天与第一次分叉。空的 `URBANITE.md` 仍是 Resident（`bring` 为空串），沿用 §11 已记的口径：空描述是作者的选择，不是缺陷。
- **空房间照列，不隐藏**：藏起来的话，模型会把「这里没人」读成「这个地址不存在」，而一间空房恰是可以请人搬进来、或派一件活过去的地方。
- **详略随距离衰减**：本楼给到每个地址的自述，全城只给楼名。这不是新规则，而是 `signal` 的 `reach` 与 `CrossBuildingTransfer` 已经画好的那条界——**看得清的范围与说得着的范围必须是同一个**，否则名册会教模型去够它够不到的人。
- **房间＝楼下一层的非点头目录，且不是 archive 目录**：这条规则本来在装配层 `read_building` 里写着一份、`buildings_of` 与 `read_spine` 又各写了一份城级的同类规则。两条规则现在都住 `city::room::all` 与 `city::building::all`，调用方三处改为调用——一条规则一个权威，页面看到的房间与模型看到的房间从此不可能不同。
- **只到直接子目录**：房间就是这样被造出来的（`room::open` 与 delegate 都建直接子目录）。翻案条件：楼层真的成为目录的那天，改的是 `room::all` 一处。
- **一个活口径接进来了，另一个被判定为噪音**：`waiting`（那间房积压几封信）由装配层以闭包供给——队列是它的，本 crate 看不到那么远。而「谁在跑」**不接**：这座城一次只驱一跑，故答案对除自己以外的每一位恒为「否」，一列恒定的词教不了任何人；`Dossier::is_live` 真正能说的是「某位的上一跑没冻结过」，那是崩溃后的事实，归 `resume` 而不归名册。

### 8-15b city::neighbours_tool（形状 4 适配器）

```rust
pub struct NeighboursTool { /* meta＋Neighbourhood —— 私有 */ }
impl NeighboursTool { pub fn new(neighbourhood: Neighbourhood) -> Result<NeighboursTool, AxError>; }
impl Tool for NeighboursTool { /* name=neighbours、effect=Read、cost=Free、render=Generic、temporal=Timeless */ }
// args：{scope}，`building`（缺省）｜`city`；结果为 {text}，一行一个地址，BTreeMap 序
```

- **`scope` 的两个取值取自配置梯子已有的层名**（`Layer::{City, Building}`），不另造一套远近词。
- **答案是按序渲染的文本而非 JSON 数组**：与 `status` 同一条已被红测试抓出的理由——`serde_json::Map` 对键排序，没有读者可依赖的序；序是模型读到的东西的属性，故落在模型读到的地方。
- **表头把「没列出的名字没有读者」写在第一行**：这是本工具存在的那个缺陷的正面表述，放在模型最先读到的位置。
- **随 Run 冻结，与 catalog 同理**：装配层单线程驱动，一次 drive 之内没有第二个 Run 在跑，且本 Run 发出的 signal 在 drive 结束后才投递——所以「派活那一刻扫到的」与「此刻」在一次 drive 内不可能不同。`Temporal::Timeless` 因此是实话：这份名册没有一个会在回合之间变化的时刻。

### 8-9 city::archive（形状 2 值类型＋形状 7 投影）

```rust
pub enum Kind { Preference, Decision, Correction, Fact }   // 封闭四类
pub struct Entry { pub kind: Kind, pub day: u64, pub subject: String, pub at: PathBuf }
pub fn day_of(at: TimeMs) -> u64;                          // 时间只入参
pub fn entry(city_root, building, kind, at, subject) -> Result<Entry, AxError>;  // 纯：不碰盘
pub fn file(entry: &Entry, body: &str) -> Result<(), AxError>;                   // 只写盘
pub fn index(city_root, building) -> Result<Vec<Entry>, AxError>;   // 算出来的，不落盘
```

- **四类封闭**：第五类需要理由，而「它不属于前四类」正是让分类表烂掉的那个理由。不合的东西是笔记，笔记住人已经在读的文档里。
- **index 是算出来的**：存一份就是盘上内容的第二份账，而盘是真的那一份。删掉它没有东西可删——这正是投影该有的样子。
- **召回是结构化的**：按类与日期归档，循索引读**原文**。不做向量记忆；翻案条件写死——真实召回率 <90% 才重议。
- **日期取整天**：给人浏览用，精度高过问题所需只会招来没人打算做的比较。

**决定一条记录是什么，与把它写上架，是两步**。原先的 `file(city_root, building, kind, at, subject, body) -> Entry` 把两者合成一步，于是调用方拿到 `Entry`（账本行要的 `kind`／`day`／`subject` 全在里面）时文件已经在架上了；账本行只能后落，而 Ledger 的定义是「Every effect becomes an EventRecord first」。拆开之后：

- `entry` 是纯的：拒空 subject、`day_of(at)` 取整天、按 `<building>/Archive/<kind>/<day>-<slug>.md` 算出落点，全部只读入参。**一条记录是什么，在它到达任何地方之前就已经确定**，所以调用方可以先把它落账再把它写上架。
- `file` 只写：经 `city::document` 把正文整份换上去，建目录也由那一处做。它收一个 `&Entry` 而不是六个参数——落点由 `entry` 算过一次，`file` 不再第二次决定它。
- **仍是一个构造点**：`Entry` 的字段没有对外的写面，`index` 那一支是从盘上读回来的另一种来源（`subject_of(&text)` 而不是入参），两者不共用同一条不变式，因此没有第二个权威。

### 8-4b 两个没人写的配置层长出写面

```rust
pub fn write_sandbox(city_root: &Path, addr: &Address, layer: Layer, limits: &SandboxLimits) -> Result<(), AxError>;
pub fn write_mcp(city_root: &Path, addr: &Address, layer: Layer, servers: &[McpServer]) -> Result<(), AxError>;
```

- **与 `write_effort` 同一道门**：梯子（城→楼→房间）本就是「一个 Run 被什么治理」的权威，第二个存储就是第二个答案。其余键原样保留，因为可能是人手写的。
- **写出的字节必须是 `ConfigFile` 读得回来的那种**：`McpServer` 的 serde 形状是嵌套的，而文件语法是平的（`label` ＋ `command`/`args`/`env` 或 `url`/`headers`/`transport`）。写面照文件语法拼，本 crate 内一处正读一处反写，两者对不上时编译不会说话、测试会——一条往返测试逐支覆盖三种 transport。`Sse` 一行必写出 `transport = "sse"`：缺省是 `http`，不写就会被读回成另一种 transport。
- **空的 `mcp` 表要写出来而不是省略**：省略即继承上一级，而一个人删掉最后一台服务器不是想继承一台。
- **`env` 与 `headers` 逐值判定凭据，落盘之前就拒**（S-07）：一个值只要不是 `SecretRef::parse` 认得的 `secret:realm/name`，名字命中 `kernel::names_a_credential` 或值命中 `kernel::scan` 即以 `AxCode::ConfigInvalid` 拒，恢复语指向金库。判定在 `write_mcp` 进 `change` 之前逐对做，因此一次被拒的写入一个字节都没落；拒绝文字报出是哪一台服务器、哪一张表、哪一个名字，因为人手里只有那句话。**理由是这份文件进版本库**：楼的 `CONFIG.toml` 由 `city::gitignore` 放行进历史（§8-21），写进去的 key 就在这个项目的每一次克隆里。**判定不重建**：「什么叫凭据」是 `kernel::secret` 的答案，与 `[sandbox] env_passthrough` 走 `EnvVarName::parse` 是同一个权威的两次调用。
- **读面今天不判这一条**：手写进 `CONFIG.toml` 的明文 key 仍然读得回来。补齐要让 `ConfigLayer::parse` 调同一个谓词，那时谓词升为 `pub(crate)` 并只有一处实现。
- **三个写面只有一条写路径**：四者各自把要说的话包成 `Change`（穷尽：`Session` / `Effort` / `Sandbox` / `Mcp`），同走内部的 `change`——取 `city::document` 对这份文件的持有、读、改一个键、整份原子换上去。各自读写时，两个会话改同一份 `CONFIG.toml` 会各自从同一份原件出发，后写的那一个抄掉先写的那一个的改动。`[model]` 那一节的三个值由同一条 `table` 找到或建出，免得三处对「该写进哪张表」各有各的说法。

### 8-14 一次会话选一次：模型与思考强度

```rust
pub fn write_session(city_root: &Path, addr: &Address, model: &str, effort: Option<Effort>) -> Result<(), AxError>;
pub fn write_effort(city_root: &Path, addr: &Address, effort: Effort) -> Result<(), AxError>;
```

思考强度放在派活按钮旁边，因为一次会话反正只选一次；而会话选定的模型也写进同一份文件，因为供应的 prompt 缓存对着的正是这两个值。

- **一次写下一个会话冻下的两样东西**：`write_session` 落 `[model] name`，并在人选了强度时落 `[model] effort`。模型总写下（一个模型总在指某个东西），强度只在人说了时写下：缺席不是一个值，而是「让供应方决定」，写出来就是把一个没人做的选择记成记录。
- **写进那一层，而不是另存一份**：选择落到会话自己房间的 `CONFIG.toml`，由已有的 city → building → room 阶梯解析。第二个存处就是第二个答案。
- **这份记录是会话的，不是运行时的设定**：一个 Run 用哪个模型仍由 endpoint book 选，强度仍爬同一条梯子——两者决定会话从哪里开始。房间写下来的只是它当初从哪里开始，所以登记面之后搬了家，是下次派活拒掉的分歧，而不是它默默执行的变更（`sprawling-SPEC §8-79`）。
- **只改 `[model]` 表里的键**：文件里其它键是人写的，读出来、改一个值、写回去，与另外三个写面同走 §8-4b 的那一条写路径。文件读不动或解析不了就**拒绝**，不覆盖——一份本构建看不懂的配置不是可以随手盖掉的配置。
- **`write_effort` 仍在**：它只写强度一个键，供没有会话记录的地址（手搭的房间、`city` 自己的单测）用。派活面走的是 `write_session`，因为那次派活同时也在记下模型。
- **两扇门都只写 Resident 层**，层级不由调用方给：派活按钮旁边选的强度属于这一次会话的房间。需要按层写强度时，签名要多一个 `Layer` 参数，那是一次公开面变更。
- **落点就是地址自己的 `CONFIG.toml`**，所以这个房间跑的 Run 读得到、改不了自己的档位；地址就是楼时它就是楼自己那份文件，也就是楼根上的会话（§8-4 的 `own_layer`）。

### 8-13 city::room（形状 1 判定 ＋ 一个实例化动作）

```rust
pub fn open(city_root: &Path, building: &Address, name: &SessionName) -> Result<Address, AxError>;
// crate 面：`pub use room::open as open_room;`——调用方读到的是 `city::open_room`，
// 因为裸的 `city::open` 在装配层里说不出开的是什么。
```

与 `city::building` 同形：一个判定加一个落盘动作，而不是一个长住的值。

- **同名加后缀，不复用**：`refactor`、`refactor-2`。复用会把两次只是共用一个词的会话放进同一套文件，而那正是本模块要消掉的缺陷。「继续上一次会话」是向它已有的房间地址派活，由界面给出选项，而不是靠拼写撞对。
- **`create_dir` 而非先问后建**：问与答是一个操作，于是同一毫秒里的两次派活不会被同时告知「这个名字空着」。
- **999 个后缀封顶**：一个写错的循环应该停下来，而不是把盘写满。

### 8-12 一个 skill 只有一个家：楼自己的书架

```rust
impl Library {
    pub fn scan(city_root: &Path, building: Option<&Address>, home: &Path) -> Result<Library, AxError>;
}
```

用户要的是「一栋楼就是一个可以 cd 过去的工作区」；已记录的理由是「住户只读得到存货，不得给自己进货」。两者其实不冲突：楼自己的书架放在 `<building>/.sprawling/skills/<section>/<name>.md`，在楼目录里、在写域之外。

- **近的书架盖远的**：同名同 section 时楼的那本胜出。这不是新规则，而是 `config_layers` 已有的那一条（低层胜，高层是回落）在书架上的同一个实例。
- **城的书架只放两栋以上共用的**：一个 skill 只有一个家。代价是找一本 skill 要看两处，换到的是一栋楼拷走就带着它自己的本事。
- **`holding_address` 删掉**：它从 section＋name 拼回一个城级路径，而 `Holding` 本来就握着落点——两个权威，且在两层书架下其中一个必然答错。落点改为扫盘时算一次、存在 `Holding::shelf` 里（§8-8）。

### 8-13 一本书带着它被读到时的样子
```rust
pub struct Holding { …, pub hash: B3Hash }   // 整份文档的 BLAKE3，扫架时算
```

- **它是白得的**：`shelve` 为了取 disclosure 那一行，本来就把整份文档读进了内存；哈希只多走一遍已在手里的字节。
- **为什么存在 `Holding` 而不是让读者自己算**：读者要的答案是「它变了没有」，而那需要**两个时刻各一次读取**；一张只能报当下内容的书架永远答不了这个问题。早一次的那一读由 `run_started` 携走存进账本（runtime-SPEC §8-11），于是比对对的是**这座城自己的历史**，不是一份签名：它只能说「这变了」，永远不说「这安全」。
- **名字不变而字节变了，正是注入的样子**，而在这个改动之前本仓库没有任何东西会发现它。

### 8-11 楼的治理字节搬进它自己的保留子树（沉淀一处路径权威）

```rust
pub fn rules_path(city_root, addr) -> PathBuf;      // <building>/.sprawling/RULES.toml
pub fn agents_path(city_root, addr) -> PathBuf;     // <building>/AGENTS.md（项目的，故在保留区之外）
pub fn config_layers::path(city_root, addr, layer) -> Result<PathBuf, AxError>;
// 三层统一为 <scope>/.sprawling/CONFIG.toml；city 层因此不再是特例
```

不变式已经立起来了，这里把字节搬到它后面。

- **三层一个表达式**：`path()` 先算出 scope 目录（城根、楼根、房间目录），再一律 `.join(RESERVED_PREFIX).join(CONFIG_FILE)`。原先 City 层写死了 `city_root.join(RESERVED_PREFIX)` 而另两层没有，那个不对称正是洞口。
- **旧城必须报错，不得静默降级**：`policy::load` 把「规则不存在」当作默认策略（`confidential = false`）。于是一座旧城的楼会从「机密」静默变成「不机密」——所以 `RULES.toml` 缺失而一份**本版不再读的文件**仍在盘上时**拒绝**，`E_CONFIG_INVALID`，recovery 直接拿出要执行的那一步。这不是兼容适配层（它不读旧文件），是一道不让静默降级发生的门。它问的是一个问题而不是每次搬家加一道守卫：`superseded` 同时看楼根与保留子树下的旧名，因为两次搬家（改位置、改格式）的后果完全相同。
- **楼页仍然看得见规则**：`assembly` 组楼页答案时跳过一切点头目录（房间枚举因此也不会把 `.sprawling` 当房间），故 `RULES.toml` 改为**按路径显式读一次**再入档。人在界面上看得到、改得了；楼里的 agent 读得到、写不了。
- **默认写域仍是整栋楼，这是故意的**：一次 Run 为它那栋楼产出一份楼级产物是正常的；把默认收紧到房间会把那件事一并禁掉，而洞口在于治理文件的位置，不在于写域的宽窄。

## 8.5 两个设计

**A（选中）：`Identity` 两态枚举**。调用方拿到的东西自己会说自己是谁，段字节由它给出。
**B（落选）：`Option<Resident>`**。少一个类型，但每个调用方都要自己决定「None 时该给 prefix 什么」——那是一条散落在每个调用点的策略，且第一个忘记写的人会得到一个空的 resident 段。翻案条件：出现第三种身份（例如代表某人的临时身份），届时枚举照常扩，Option 则无法扩。

**另两个设计（配置文件名）**

**A（选中）：三层同名 `CONFIG.toml`，层级由位置决定**。读者记一个名字；把一份配置放错层是一个**位置**错误，而位置在目录树里看得见。
**B（落选）：每层各起一名**（`CITY.toml`／`BUILDING.toml`／`RESIDENT.toml`）。文件名自带层级信息，代价是三个名字要同时被记住，且放错层变成一个**拼写**错误——拼写错误要靠逐字比对才看得出。翻案条件：出现同一目录下共存两级配置的需求（例如一栋楼的默认与它自己作为房间的默认同处），届时位置不再能区分层级。

## 9 工作流程

bin `RunWorker::dispatch` → `Identity::load(city_root, addr)` → `segment_bytes()` 进 `FrozenPrefix` 的 resident 槽 → `who()` 成为 Ledger 的 actor。

## 10 实现逻辑

纯 std：一次 `fs::read` 与一次 `B3Hash::digest`。`urbanite_path` 按地址分段 push，故不做字符串拼接，Windows 上也无分隔符问题。

## 11 边界枚举

无文件（→Ephemeral）｜文件存在但无读权限（→报错）｜空文件（→Resident，段为空字节；空描述是作者的选择，不是缺陷）｜地址含多段（`lab/room1`，逐段 push）。

## 12 错误处理

`E_STORAGE_FATAL`（读不动一个存在的描述）：不可定义掉——文件系统权限是外部世界的事实，而静默降级是被明拒的替代。

## 13 依赖选型

只依赖 `kernel`（拓扑硬约束）＋ std。dev 依赖 `tempfile`。

另有两件依赖，均已在 workspace 钉版（不新增版本权威）：`toml` 与 `serde`（derive）。理由：三层配置的格式是 TOML，而 `toml` 已被 `xtask` 消费（budgets.toml／lexicon.toml）；解析走 serde derive 加 `deny_unknown_fields`，使「写错的键」在反序列化那一刻失败。手写一个 TOML 子集解析器是可行的另一条路，已落选：它会把一个已有权威的格式变成本库自己的私有变体。

## 14 硬编码声明

城里每一份文件叫什么、落在哪，权威是 `kernel::layout`：`ARCHIVE_DIR`、`BUILDING_SHELF`、`CONFIG_FILE`、`LIBRARY_DIR`、`URBANITE_FILE` 由本 crate `pub use` 转出而不复述，路径由 `CityLayout` 的十一个方法给出而不逐段 push。`spine_files` 的 `HANDOFF_FILE` 与 `JOB_FILE` 同样只是 `kernel::layout` 那一份的别名。本 crate 自己定义的文件名只剩没进布局表的那几份：`RULES_FILE`、`DESKTOP_SCOPE_FILE`、`PREFERENCES_FILE`、`SCHEDULE_FILE`、`WATCH_FILE`、`GITIGNORE_FILE` 与 spine 剩下的那一组。

Ephemeral 段文本（私有常量，改它即改一个 Ephemeral 读到的第一句话）。

`CONFIG_FILE`（三层同名，理由见 §8.5）；新楼的 `RULES.toml` 字节不写在代码里，而是 `include_str!("../../../docs/templates/RULES.toml")`——它的权威是那份模板，路径写错在编译期就会被堵住；`Confidential` 模板对该字串做一处行替换（`confidential = false` → `true`），替换是否真的生效由 `policy::evaluate` 读回来断言。

## 15 影响面

bin 装配层的 prefix 组装随之改；`docs/templates/URBANITE.md` 是这份文件的模板，两者改动须同期。

建楼与三层配置波及 bin 装配层三处：`run_command` 增 `CreateBuilding` 臂；`dispatch` 里的本地函数 `building_of` **删除**，改用 `city::Building::of`（一条规则一个权威）；`CallShape.effort` 不再恒为 `None`，改由 `config_layers::load` 供给——接线台账里「Effort 值的生产者待接」那一行到此为止。

邻里名册波及四处：装配层本地函数 `buildings_of` **删除**，`read_spine` 的内联同类规则与 `read_building` 的房间枚举一并改调 `city::buildings`／`city::rooms`；`dispatch` 增一次 `Neighbourhood::scan`，其结果既供 `NeighboursTool` 也供 `status` 的第十三字段；`runtime::StatusSnapshot` 增 `neighbours: u32`（runtime-SPEC §8-14 同期改）；工具表增一件，故 `ChatRequest.tools` 每回合多一条 disclosure 与一份 schema。

## 16 测试与约束

三条：身份两态各一条；**段字节跨两次加载稳定**；Dossier 只计本人的 Run 且跨 Run 累加。bin 侧另有一条端到端断言（两次 Dispatch 的 resident 段哈希相同、run 段不同）。

建楼与配置八条：新建的楼被 `policy::load` 读回且 confidential 模板真的锁本地模型池｜二次出生恒拒｜reserved prefix 下建楼恒拒｜房间地址建楼恒拒且拒词指出该建哪栋｜下层配置盖上层｜不认的键即拒｜**写在 `CONFIG.toml` 里的 effort 出现在真实出线请求体里**（bin 侧端到端，假 provider 录下请求体）｜**`usersbrowser` 的三种值各读回各的形状，`browser` 与它互不影响，confidential 楼写它即拒**。

邻里名册六条：扫到的名册**不含我自己**且有人的与空的各自落在对的臂上｜`## Bring them` 在场时取它、缺席时退回第一段正文且跳过标题与引文｜同一座城扫两次字节相同（`read_dir` 序不得泄漏到答案里）｜`scope=city` 只交出楼名、不交出任何住户｜`.sprawling` 与 archive 目录都不是房间｜**模板仍然带着 `## Bring them` 这一节**（对 `docs/templates/URBANITE.md` 的 `include_str!` 断言；模板改名而代码不改，就是一份永远退回正文的名册）。

## 17 模型体验

三层配置入窗零字节：`CONFIG.toml` 改的是请求字段（effort），不是模型读到的文本；新楼的 `RULES.toml` 经 `city::policy` 进判定面，其字节另经 building 段整份入窗——判定与阅读读的是同一串字节。

resident 段是模型每回合都读到的四段之一。`URBANITE.md` 建议 30 行以内：长的描述不会让 Resident 更能干，只会让每个回合更贵——这句话写在模板里，因为模板在场即教学。

邻里名册的常驻代价是一件工具的 disclosure 与 schema，名册本身**不进 prefix**：一栋楼的住户数会长，而每回合都付的字节不该随人口增长。模型读到的常驻新增只有 `status` 的一行 `neighbours: N`——它回答的是「值不值得问」，问出来的详情由工具在需要时交付，与 catalog 对 skill 用的是同一条渐进披露。

## 18 文档同步

新增模块登记 ARCHITECTURE.md §6 与接线台账；`Dossier` 的生产消费者（Resident 视图）到位时更新台账行。

建楼与配置同期四处：§6 模块表两行翻 `已建`；§6 接线台账的 `kernel::config`（freeze 面）与 Effort 两行改成已接线；`xtask/api-baselines/city.txt` 随公开面重算；`docs/templates/RULES.toml` 从此是被实例化的那串字节，改它即改新楼的第一句话。

邻里名册同期五处：`architecture.toml` 增 city 两条、`runtime::tools::status` 一行由十二字段改十三；`docs/glossary.md` 增 **Neighbourhood** 与 **neighbours** 两行（一个概念一个名字，且 `directory` 因与文件系统目录同音而被明确弃用）；`crates/runtime/runtime-SPEC.md` §8-14 的 status 接口块；`xtask/api-baselines/` 的 `city.txt` 与 `runtime.txt`；`docs/templates/URBANITE.md` 的 `## Bring them` 从此是被读取的一节，改它即改全城名册显示的那一行。

### 8-16 city::config_layers 目录化

681 行一份文件切成三份，读面与写面各占一份，测试单独一份：

- `config_layers.rs`（300 行）：`CONFIG_FILE`、`Layer`、`path`，以及读面 `ConfigLayer::parse`／`load`／`read_layer`／`refuse` 与四个 `serde` 段落类型。它回答「哪三份文件、怎么读」，仍是本节开头那份接口块的家。
- `config_layers/write.rs`（160 行）：`write_effort`、`write_mcp`、`write_sandbox` 与它们私有的 `read_document`／`write_document`／`refuse_file`。写面自成一簇的缝在于它只经 `path` 与 `Layer` 回到读面，不碰 `ConfigLayer` 的任何字段。`config_layers.rs` 以 `pub use write::{write_effort, write_mcp, write_sandbox};` 重导出，`lib.rs` 的三行门面与 crate 内所有 `use` 一字未改。
- `config_layers/tests.rs`（245 行）：原内联 `mod tests` 整体迁出，11 个 `#[test]` 与其断言逐字不动。

**无字段开放**：没有为跨文件引用把任何私有字段升成 `pub(crate)`／`pub(super)`；`write.rs` 用到的 `path` 与 `Layer` 本来就是公开面。

**apisync 未重写基线**：`cargo public-api -p city` 与基线的差异只有三行，全部是 `kernel::model::Effort` → `kernel::model::wire::Effort` 一类的 kernel 侧规范路径重拼，与本次切分无关（本次切分的公开面逐字节不变）；基线随 kernel 那一侧的改动一并重算。

### 8-17 city::spine_files 目录化

591 行一份文件切成两份，生产代码与测试各占一份：

- `spine_files.rs`（371 行）：五个文件名常量、三份模板、`JobBrief`／`RunBrief`、`lay_out`、`write_brief`／`write_job`、`job_path`／`roadmap_path`／`handoff_path`、`roadmap`／`handoff`／`norms`，以及私有的 `is_blank_form`／`empty_roadmap`／`is_placeholder_row`／`write_new`／`storage`。它仍是 §8-5 那份接口块的家，公开面逐字节不变。
- `spine_files/tests.rs`（224 行）：原内联 `mod tests` 整体迁出，10 个 `#[test]` 与其断言、名字逐字不动；`use super::*;` 保持，`super` 仍指 `spine_files`。父文件尾部保留原样的 `#[allow(...)]` 属性列表加一行 `mod tests;`。

**无字段开放**：没有为跨文件引用把任何私有字段升成 `pub(crate)`／`pub(super)`；测试经 `super::*` 看到的私有项与迁出前相同。

**apisync 未重写基线**：公开面不受本次切分影响。

**第二次切分（`building/template.rs`，126 行）**：`BuildingTemplate`、它的四个模板常量与 `NAME_PLACEHOLDER` 迁入 `building/template.rs`，`building.rs` 因此回到 312 行（单文件 400 行上限）。`rules` 升为 `pub(super)`、`NAME_PLACEHOLDER` 升为 `pub(crate)`——后者是为了收掉 `spine_files.rs` 里那份同值的第二份定义：楼的规则与它的计划、备忘、交接读同一个占位符，两份拼法会让其中一份文件永远写着 `<building name>`。公开面仍由 `building.rs` 的 `pub use template::BuildingTemplate;` 给出，逐字节不变。

### 8-18 city::policy 目录化

517 行一份文件切成两份，生产代码与测试各占一份：

- `policy.rs`（360 行）：`RULES_FILE`、`SUPERSEDED_FILE`、`ModelPool`、`BuildingRules` 及其全部方法（`policy`／`addr`／`egress`／`review`／`reading_room`／`model_pool`／`write_domain`）、`rules_path`／`superseded`／`scope_path`、`load`／`write_rules`／`evaluate`。它仍是 §8-2 那份接口块的家，公开面逐字节不变。
- `policy/tests.rs`（150 行）：原内联 `mod tests` 整体迁出，11 个 `#[test]` 与其断言、名字、夹具 `addr` 逐字不动；`use super::*;` 保持，`super` 仍指 `policy`。父文件尾部保留原样的 `#[allow(...)]` 属性列表加一行 `mod tests;`。

**无字段开放**：没有为跨文件引用把任何私有字段升成 `pub(crate)`／`pub(super)`；测试经 `super::*` 看到的私有项（`superseded`、`BuildingRules` 的字段）与迁出前相同。

**apisync 未重写基线**：公开面不受本次切分影响。

**第二次切分（`building/template.rs`，126 行）**：`BuildingTemplate`、它的四个模板常量与 `NAME_PLACEHOLDER` 迁入 `building/template.rs`，`building.rs` 因此回到 312 行（单文件 400 行上限）。`rules` 升为 `pub(super)`、`NAME_PLACEHOLDER` 升为 `pub(crate)`——后者是为了收掉 `spine_files.rs` 里那份同值的第二份定义：楼的规则与它的计划、备忘、交接读同一个占位符，两份拼法会让其中一份文件永远写着 `<building name>`。公开面仍由 `building.rs` 的 `pub use template::BuildingTemplate;` 给出，逐字节不变。


### 8-19 city::building 目录化

502 行一份文件切成两份，生产代码与测试各占一份：

- `building.rs`（339 行）：`TEMPLATE_RULES` 等四个模板常量、`BuildingTemplate`（`parse`／`name`／私有 `rules`）、`Building`（`of`／`addr`／`root`／`holds`）、`all`／`create`／`adopt`、`created_payload`／`adopted_payload`，以及私有的 `storage`。它仍是 §8-3 那份接口块的家，公开面逐字节不变。
- `building/tests.rs`（167 行）：原内联 `mod tests` 整体迁出，9 个 `#[test]` 与其断言、名字、夹具 `addr` 逐字不动；`use super::*;` 保持，`super` 仍指 `building`。父文件尾部保留原样的 `#[allow(...)]` 属性列表加一行 `mod tests;`。

**无字段开放**：没有为跨文件引用把任何私有字段升成 `pub(crate)`／`pub(super)`；测试经 `super::*` 看到的私有项（`NAME_PLACEHOLDER`、`BuildingTemplate::rules`）与迁出前相同。

**apisync 未重写基线**：公开面不受本次切分影响。

**第二次切分（`building/template.rs`，126 行）**：`BuildingTemplate`、它的四个模板常量与 `NAME_PLACEHOLDER` 迁入 `building/template.rs`，`building.rs` 因此回到 312 行（单文件 400 行上限）。`rules` 升为 `pub(super)`、`NAME_PLACEHOLDER` 升为 `pub(crate)`——后者是为了收掉 `spine_files.rs` 里那份同值的第二份定义：楼的规则与它的计划、备忘、交接读同一个占位符，两份拼法会让其中一份文件永远写着 `<building name>`。公开面仍由 `building.rs` 的 `pub use template::BuildingTemplate;` 给出，逐字节不变。

### 8-20 City Hall：随城市立起的那栋楼，和住在里面的两个人

**需求**：城市需要一个规划者和一个代答者。它们服务每一栋楼，因此不属于任何一栋楼；它们写 Markdown 和计划，不建造。

**接口**：

```rust
// city::building
pub enum BuildingTemplate { Minimal, Confidential, Hall }   // Hall 的字节是固定的一份模板

// city::policy
pub enum DomainReach { Everything, Documents }              // RULES.toml 的 `write` 一键
impl BuildingRules { pub fn reach(&self) -> DomainReach; }
// write_domain()：Everything → WriteDomain::new，Documents → WriteDomain::documents

// city::spine_files
pub const MAYOR_FILE: &str = "MAYOR.md";
pub const CLERK_FILE: &str = "CLERK.md";
/// hall/mayor 与 hall/clerk 的身份文件位置；别的地址返回 None。
pub fn hall_identity_path(city_root: &Path, addr: &Address) -> Option<PathBuf>;
/// 把两份模板写进 <city>/.sprawling/，已存在的不覆盖。
pub fn lay_out_hall_identities(city_root: &Path) -> Result<(), AxError>;

// city::wizard
impl CityPlan { pub fn hall(&self) -> &(Address, BuildingTemplate); }   // 恒存在，非 Option
```

- **为什么 `hall` 在 `CityPlan` 里是恒存在的字段而不是 `Option`**：一座没有 City Hall 的城市不是这个版本能形成的东西。可选性会让「城市有没有市政厅」变成调用点每次都要答一遍的问题，而它只有一个答案。
- **两份身份文件住 `<city>/.sprawling/`**：写域碰不到保留子树，所以 Mayor 改不了自己是谁，clerk 改不了自己按什么答。这是 `URBANITE.md` 住在居民自己地址下时拿不到的性质，也是这两位与普通居民唯一的结构差别。
- **路径权威仍只有一个**：`city::resident::urbanite_path` 先问 `spine_files::hall_identity_path`，无答再拼 `<addr>/URBANITE.md`。`Identity::load` 一字不改，因此「有身份文件即居民」这条规则对市政厅与对普通房间是同一条。
- `write = "documents"` 由 `policy::evaluate` 读成 `DomainReach`；**缺这一键即拒**，与 `confidential` 同——先前它读作 `Everything`，于是没见过这个设置的人得到最宽的那一档。值既不是 `everything` 也不是 `documents` 时同样拒绝：读成打字错误的权限设置不能落到宽松那一侧。
- 被否：给 Mayor 一个覆盖全城的 `Everything` 写域，靠 `MAYOR.md` 的措辞请它别碰代码——把不变量交给提示词，等于没有不变量。

### 8-21 city::gitignore：一栋楼承诺的东西进历史，一次会话在想的东西不进

**需求**：`building::create`（raise）与 `building::adopt` 立起一栋楼时，同时放下两样东西：一份 `SPEC.md`，和一份 `.gitignore`。

**接口**：

```rust
// city::gitignore（一张从各模块取名的规则表＋一个幂等落盘动作）
pub const GITIGNORE_FILE: &str = ".gitignore";
/// 把本城的忽略规则补进这栋楼的 .gitignore。已有的字节一行不删，
/// 缺哪行补哪行；文件不存在则整份写出。
pub(crate) fn place(building_root: &Path) -> Result<(), AxError>;

// city::spine_files
pub const SPEC_FILE: &str = "SPEC.md";   // 字节来自 docs/templates/SPEC.md（include_str!）
```

- **不对称本身就是这一条的全部内容**：`SPEC.md` 与这栋楼自己保留子树里的五份承诺进版本库，`Roadmap.md`、`Memo.md`、`Handoff.md` 与各个房间不进。一栋楼向外承诺的东西必须在历史里，任何一次克隆都读得到；一次会话当时在想什么不是承诺，它留在运行中的机器上。
- **`SPEC.md` 是十七节 crate SPEC 的压缩式，不是第二种形状**：`docs/templates/SPEC.md` 的十二节逐节对应 crate SPEC 的节次（需求／验收／假设／权威／命名／边界／接口／错误／依赖／硬编码／测试／决策），只是把「现状分析、工作流程、实现逻辑、影响面、模型体验、文档同步」这几节留给 crate 自己。它压缩，不另起。
- **`place` 只追加，从不重写**：被收编的目录往往已经有一份 `.gitignore`，里面写着这个项目自己的东西。整份覆盖会把它们冲掉，而那正是 adopt 承诺不会碰的字节。依据是逐行比对（去空白后相等即视为已有），因此重复 raise 不会把同一段追加两次。
- **显式的反忽略**：`!SPEC.md` 与保留子树的放行行写进块里，而不是靠「没人忽略它们」这个默认。被收编的仓库可能已经忽略了 `*.md` 或一切点开头的目录；那时「这栋楼的承诺在历史里」就是假的，而没有人会发现。
- **保留子树逐文件放行，不整棵放行**（S-07）：块里先 `.sprawling/` 忽略任意深度的保留子树，再 `!/.sprawling/` 只把这栋楼自己的那一棵放回来，`/.sprawling/*` 把它清空，最后逐行放行五份承诺——`RULES.toml`、`CONFIG.toml`、`FILTERS.toml`、`DESKTOP.toml` 与 `skills/`。三个理由：①城自己的保留子树同名，账本、对象库与金库引用住在那里，一行 `!.sprawling/` 把它们一并放回版本控制的可见面；②那一行不带斜杠，因此对楼下每一个居民、每一个房间的保留子树同样生效，而那些是机器上的东西，不是这栋楼的承诺；③`CONFIG.toml` 正是 MCP 凭据的落点，它进历史的前提是 §8-4b 的逐值判定同时成立——两件事是同一次改动。
- **五个名字都从写它的模块取**：`kernel::layout` 的 `CONFIG_FILE`／`FILTERS_FILE`／`BUILDING_SHELF`、`policy` 的 `RULES_FILE`／`DESKTOP_SCOPE_FILE`、`spine_files` 的四份脊柱文档名。因此 `BLOCK` 由 `&[&str]` 常量改为 `block() -> Vec<String>`：一个改了名的文档不会在这里留下一条谁都不写的规则。
- **顺序就是文法**：git 认最后一条命中的规则，所以「忽略—放回目录—清空—逐行放行」这四步不能重排。这一条由 git 自己验过：外层再写 `.*` 与 `*.md`，`SPEC.md` 与 `.sprawling/CONFIG.toml` 仍然进历史，`.sprawling/ledger/` 仍然不进，一个嵌套目录自己的 `.sprawling/CONFIG.toml` 也不进。
- **房间由房间自己忽略**：`room::open` 在新开的房间里放一份只有 `*` 一行的 `.gitignore`。楼这一层的 `.gitignore` 写不出「房间」——房间是人当场命名的普通子目录，立楼时它们还不存在，而在被收编的仓库里按通配符去猜哪个子目录是房间会误伤源码目录。
- 被否：在楼的 `.gitignore` 里写 `*/JOB.md`、`*/URBANITE.md` 一类通配。它只忽略房间里的某几个文件名，会让一次会话的其余产物照样进历史，等于把这条规则写成一半。

### 8-22 city::vocation：一个地址上的居民是来建造的还是来规划的

**接口**：

```rust
// city::vocation（形状 1 判定；无 I/O、无时钟）
pub enum Vocation { Builds, Plans }   // 穷尽；第三种职分＝第三个臂
pub fn vocation_of(building: &Address) -> Vocation;
```

- **按地址判，不按提示词判**：City Hall 的居民写 Markdown 和计划，不建造。装配层据此为它组工具台：没有 `exec`、没有 `delegate`、没有 `workshop`。把这件事交给 `MAYOR.md` 的措辞，就是把不变量交给提示词。
- **返回穷尽枚举而不是 bool**：`is_hall()` 只答得出「是不是市政厅」，而调用点要问的是「这个地址上的人是干什么的」。第三种职分出现时，缺臂是编译错误，不是一个悄悄落到 `else` 里的新楼。
- **依据是首段等于 `kernel::consts_policy::HALL_BUILDING`**：`hall` 这个名字只有一处权威，本模块不重抄字面量。

### 8-23 city::city_tool：市政厅对城市本身的那一扇门

**接口**：

```rust
// city::city_tool（形状 4 适配器）
pub struct CityTool { /* city_root —— 私有 */ }
impl CityTool { pub fn new(city_root: &Path) -> Result<CityTool, AxError>; }
// 工具名 `city`；action ∈ { raise, adopt, list }
// raise:  { name, template }  —— template 缺省 `minimal`
// adopt:  { name }
// list:   —— 无参，答本城的楼与每栋楼是否已有 RULES.toml
```

- **`Effect::Govern`，不是 `Effect::Write`**：立一栋楼是在城根下造目录，任何写域都够不到那里，理由与 `rules_tool` 同——这个决定是人的。门本身把它交给人，而不是靠写域的拒绝去兜。
- **三个动作一条目录行**：`list` 是 `raise` 与 `adopt` 的前提（叫什么名字、哪个目录已经在那儿），拆成第二个工具只会多一行给模型读。
- **动作不认即拒并报出已知集**：猜错这里意味着把「收编一个已有目录」执行成「新建一栋空楼」，而后者会在人的工作目录旁边多出一份不属于它的模板。
- **本工具只装给市政厅的居民**（见 8-22）。别的楼要新增一栋楼，走人的控制面。

### 8-24 Handoff 从楼搬到房间

> 权威在 runtime-SPEC §8-33；本节只记 city 这一侧怎么变。

`handoff_path(city_root, room)` 与 `handoff(city_root, room)` 的第二个参数从楼地址改为**房间地址**：`<city>/<room>/Handoff.md`。签名一字不变，变的是调用方递什么——装配层的 `run_segment` 递本跑的地址。模板由 `room::open` 在打开房间时经 `spine_files::lay_out_handoff` 铺下；楼级 `lay_out` 不再铺 `Handoff.md`。理由是同楼并发：两个房间同时冻结，一份楼级文件就是两份内容抢一个名字。没有房间的地址（直接派到楼根的跑）读到 `None`，与从前空表单的读法一致。

### 8-24 city::governed：治理这座城的三份文件（形状 4 adapter）

```rust
pub const PREFERENCES_FILE: &str = "PREFERENCES.md";
pub enum Governed { Mayor, Clerk, Preferences }
impl Governed { pub fn file(self) -> &'static str; pub fn path(self, city_root: &Path) -> PathBuf; }
pub fn write_governed(city_root: &Path, which: Governed, body: &str) -> Result<PathBuf, AxError>;
```

三份文件都住 `<city>/.sprawling/`——没有任何写域够得到的地方（前两份也在那里）。**本模块之所以是一扇门而不是三个调用方各自拼一条路径**：能自己拼路径的调用方就能拼出一条走出保留子树的路径，那样「居民改不了治自己的东西」就成了靠习惯成立而不是靠构造成立。

- **`Preferences` 是第三份而不是第三个居民**：市长与文书各有身份文件，而「这个人怎么喜欢这座城办事」不属于任何一个居民，它属于城；它与前两者被同一条规矩治理，所以住同一处、走同一扇门。
- **整份覆写**：这是人在一个框里编辑、按一次保存的文件，写一半会让这座城被半句话治理。旧内容不留在这里——账本上那行 `governed_document_written` 才是回头看的地方。
- **枚举而不是文件名字符串**：文件名是城的答案，不是发帧的人的答案（channels-SPEC §8-19 同一条理由，两侧各说一次）。

### 8-25 `desktop`：这栋楼把桌面交出去了吗（`policy` 内，形状 1 判定）

`RULES.toml` 多一位开关，读法与 `browser` 逐字同形：

```rust
impl BuildingRules {
    pub fn desktop(&self) -> bool;   // 缺这一行读作 false
}
```

- **缺省是关，与 `browser` 同、与 `confidential` 反**。分野是两者各自往哪一侧失手：把隐私设置读成宽松那一侧是一次事故，而少给一件工具只是少一件工具。桌面比浏览器更该守这一条——`desktop.act` 会在这个人自己的机器上按下按键，而按下去的东西没有 restoration。
- **机密楼恒不给桌面**，理由与 `browser` 那条同构而更强：一台桌面上有别的程序、别的窗口、一整块剪贴板，`desktop.screenshot` 读到的东西没有一样是这栋楼的。「数据可入不可出」与「这栋楼可以截屏运行中的机器」是同一句话的两半，不能同时为真，故在 `evaluate` 里即拒，`E_CONFIG_INVALID`，拒词指出删哪一行。
- **它开的是「准不准接」，不是「准不准做」。**准不准做归 `DESKTOP.toml`，那是 server 那一侧、按窗口逐条写的 allowlist（`desktop/desktop-SPEC.md` §8-4）。两道门叠着，且**次序固定**：楼说不，连进程都不起；楼说是，仍要那份 allowlist 逐窗口点头。一道门管「这栋楼是干这个的吗」，另一道管「运行中的机器上的哪几个窗口」，把它们合成一道都会让其中一个问题没人问。

### 8-26 `DESKTOP.toml` 落在哪（`policy` 内，形状 4 adapter）

```rust
pub const DESKTOP_SCOPE_FILE: &str = "DESKTOP.toml";
pub fn desktop_scope_path(city_root: &Path, addr: &Address) -> PathBuf;
pub fn write_desktop_scope(city_root: &Path, addr: &Address, text: &str) -> Result<PathBuf, AxError>;
```

**先问 `DomainReach` 立下的那条读法。**`DomainReach` 把「residents 能写什么」变成了一份可判定的东西，而它成立的前提是：**决定这件事的那份文件，恒不由被它决定的人来写**。`DESKTOP.toml` 恰恰是这一类——它逐窗口地说这栋楼的 runs 能碰运行中的机器上的什么。故它不是产物，是治理文件。

**落点因此是 `<city>/<building>/.sprawling/DESKTOP.toml`**，与 `RULES.toml`、`CONFIG.toml` 同处，在 reserved prefix 之下——`is_reserved` 对任何含 `.sprawling` 段的地址为真，故**任何写域都够不到它**，包括 `DomainReach::Everything` 的楼。一个 agent 改不了自己被判的那把尺子，这一条在这里是由构造成立的，不是由记得成立的。

**写它的是一扇门，不是一个能拼路径的调用方**（`city::governed` §8-24 同一条理由，此处第二次适用而不是第二个权威）：设置页递「哪一栋楼」与「整份文本」，路径由本模块算。能自己拼路径的调用方就能拼出一条走出保留子树的路径。

**整份覆写，且恒不在此校验内容**：`DESKTOP.toml` 的语法权威在 server 那一侧（`desktop/src/scope.rs`），且它 fail closed——读不出来的文件关成全拒。城里再抄一份解析器就是第二个权威，而两个权威里迟早有一个会把某份文件读成另一种意思。城这一侧只保证「写进去的字节就是人给的字节」，剩下的由那台 server 在启动时读，读不动就什么都不做。

**恒不复用 `Governed`**：那三份是**城**的文件（`<city>/.sprawling/`），这一份是**楼**的。把楼级路径塞进一个按 city_root 取路径的枚举里，会让那个枚举需要一个只有部分变体用得上的参数。

**线上它走 `ConfigureBuilding` 的第三个可选字段**（channels-SPEC §8-26）：那条帧问的就是「这栋楼的 runs 够得到什么」，沙箱、外部服务器与运行中的机器上的窗口是同一个问题的三面。`configured_payload` 因此收一个 `Written { sandbox, mcp, desktop }` 而不是三个裸布尔——一个调用点写 `(true, false, true)` 说不出哪一位是哪一面。

### 8-27 city::document：一份文档整个换上去，或者旧的留着（形状 4 adapter）

```rust
pub(crate) fn replace(path: &Path, body: &[u8]) -> Result<(), AxError>;
pub fn edit<T>(path: &Path, act: impl FnOnce(&Held<'_>) -> Result<T, AxError>)
    -> Result<T, AxError>;                        // 门面上是 city::edit_document
pub struct Held<'a> { /* 私有 */ }
impl Held<'_> { pub fn replace(&self, body: &[u8]) -> Result<(), AxError>; }
```

**`edit` 与 `Held` 对外开放，`replace` 不**（叶子 3.1 改了这一条记录，理由在此）：人层 `<home>/.sprawling/config.toml` 不在任何一座城里，却与一份 `CONFIG.toml` 同性质——有人手工编辑它，有解析器把它读回来，同一条命令流写它。它要的正是本模块那两条性质，而**再写一份「要么整份要么不动」就是给 B-49 立第二个权威**，两份实现里迟早有一份漏掉 `sync_all` 或漏掉锁。开放的是读-改-写那扇门（`edit` 与它给出的 `Held`），不是整份覆写那条捷径：`replace` 留在 crate 内，因为城外唯一的调用方做的是读-改-写，而一个能整份覆写的外部调用方就能不读就写。

**城里写下的每一份文件都有人拿解析器读回来**：配置层、楼的规则、一次会话的 JOB.md。就地截断再流式写入，中间有一段时间盘上既不是旧版也不是新版；断电后那段时间不会结束，于是那间房、那栋楼乃至整座城的每一次派活都失败，直到有人手工改那份文件。两条性质把这扇窗关上，而两条都只写在本模块：

- **要么整份，要么不动**：字节先落到目标同目录的暂存文件，`sync_all` 把它交到设备上，再由一次 `rename` 把它放到位。覆盖式 `rename` 在本项目支持的每一种文件系统上是一个操作，所以读者拿到的是旧版或新版，没有第三种。
- **同一刻只有一个写者**：一份文档的读-改-写在本进程内互斥。`Held` 只能从 `edit` 里拿到，于是「改写期间锁是持有的」由类型成立，而不是由每个调用点记得成立。

**暂存文件名固定为 `.<文件名>.staging`**：写者在 flush 与 rename 之间被杀会留下它，固定名让下一次写复用同一个位置，而不是攒出一目录谁也说不清归属的碎片；点前缀使城里每一处扫描都跳过它（扫描一律跳过点开头的项）。

**父目录的 `sync_all` 只在 unix 上做**：目录项住在目录里，光刷文件不够；Windows 没有可供进程打开的目录句柄，也不需要——`rename` 调到的 `MoveFileEx`（`REPLACE_EXISTING`）由文件系统自己记日志。

**锁是本进程的**：一个人在编辑器里改同一份文件不受它约束，而在本仓库现有依赖下也无法约束（跨进程文件锁需要新依赖，`crates/city/Cargo.toml` 只认 `kernel` + std + `toml`/`serde`）。挡住那个人的是原子替换：他的编辑器永远读不到半份文档。

**`create_new` 那一族不归本模块**：`spine_files::write_new`、`building::create`、`gitignore::seal_room` 要的是「独占地认领一个名字」，而 `OpenOptions::create_new` 已经把认领与拒绝合成一个操作。把它们改道本模块只会让一条已经成立的规则多一个家。

**错误面**：`E_STORAGE_FATAL`，主题是失败的那条路径与操作系统的原话，恢复语一句——把目录改成可写、确认磁盘有空间，然后重存。八个写面此前各写一遍这句话，现在是一份。

**关门条件**：断电模拟——任意时刻杀进程，`CONFIG.toml` 要么是旧版要么是新版。逼近它的是四条测试：一个读者在另一线程反复替换 512 KiB 文档时每次都读到完整的旧版或新版；被杀的写者留下的暂存文件既不是那份文档、也不挡下一次写；两个线程各二百次读-改-写之后计数是四百；一份文档把它上面的目录一并带来。
