# city-SPEC.md

> crate：`city`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

本 crate 是城的空间与身份面：

| 模块 | 这个模块回答的问题 | §8 |
|---|---|---|
| `resident` | 谁在跑这个 Run（身份从哪来、给 prefix 贡献什么、做过什么） | 8-1 |
| `policy` | 这栋楼里允许什么（confidential 四条、写域、出网） | 8-2 |
| `building` | 一栋楼怎么被建出来，以及一个地址归哪栋楼管 | 8-3 |
| `config_layers` | 三层配置住哪三个文件，又怎么求成一份 `FrozenConfig` | 8-4 |
| `spine_files` | 一栋楼开局有哪几份文档，一件活的 JOB.md 落在哪 | 8-5 |
| `schedule` | 到点发车：谁在什么节奏上自己开始 | 8-6 |
| `watch` | 盘上的文件变了，谁该知道 | 8-7 |
| `library`、`archive` | 书架上有什么、怎么装上去；一条记录存哪里、怎么找回来 | 8-8、8-28、8-9 |
| `wizard` | 建城向导 | 8-10 |
| `room` | 一个地址是不是房间，会话没指名时开哪一间 | 8-13 |
| `session` | 一段会话开始时清掉什么 | 8-14b |
| `neighbourhood`、`neighbours_tool` | 这座城有哪些地方，我身边站着谁，我该跟谁说话 | 8-15 |
| `rules_tool` | 居民怎么读写自己楼的规则 | 8-2b、8-36 |
| `gitignore` | 一栋楼的哪些字节进历史 | 8-21 |
| `vocation` | 一个地址上的居民是来建造的还是来规划的 | 8-22 |
| `city_tool` | 市政厅对城市本身的那一扇门 | 8-23、8-36 |
| `governed` | 治理这座城的三份文件 | 8-24b |
| `document` | 一份文档整个换上去，或者旧的留着 | 8-27 |
| `handoff_form` | 交接表单的四节怎么读 | 8-5 |
| `check` | 一座城的每份 TOML，错处落到行列 | 8-29 |
| `history` | 这个目录有没有城的历史 | 8-30 |

## 2 验收标准

每个模块的验收是它 §8 小节里的规则，各由模块旁的 `tests.rs` 经生产入口断言。身份这一面的验收：一个 Resident 跨两个 Run 存活，且**两次 Run 的 resident 段字节相同**（`a_resident_crosses_two_runs_with_the_same_identity_segment`，bin 侧从 `model_called` 的 segments 哈希取证）；无 `URBANITE.md` 的地址落为 Ephemeral 且段文本明说这一点。

## 3 假设与歧义

「Resident 住哪」取**地址即目录**：`<city>/<addr>/URBANITE.md`。Building 模板实例化后若改变目录形状，改的是 `urbanite_path` 一处。

**书架文档由页面写入（`PutShelved`）尚未实现，规则已定**：页面写的书架文档只经 `library::install`（§8-28，上架唯一的门），作为单文档包安装；写进已安装的包时生成新版本——暂存后整体替换（§8-27），不在原地改；每次写入在账本记一行新事件 `shelved_document_written`（kernel 事件表）。wire 的 `PutShelved` 分支此刻仍答 `not_built`，客户端没有控件。

**升级后的模板过期检查尚未实现，规则已定**：`init` 布置的城文件（`crates/city/templates/` 下的模板，`City.md` 在内）带上布置时所用模板的标记（模板内容哈希）；新二进制打开旧城的路径上挂一个检查，标记与二进制所带模板不符即如实说出哪份文件过期，并给出更新命令。标记与检查的挂点都还没有代码。

## 4 现状分析

二十三个模块（§1）。生产消费者是 `crates/sprawling`：装配层在派活时读身份、规则与配置梯子，在建楼、开房间、写会话时调本 crate 的写面；`city_tool`、`rules_tool`、`neighbours_tool` 注册进 bench。

## 5 权威信源

「空间、身份、历史」的语义（Resident 是身份、活跃 Run 才是开销；一个地址决定三件事）；`crates/city/templates/URBANITE.md`（这份文件长什么样）；`architecture.toml` 里 city 那些条目。

## 6 命名统一

Identity（两态）｜Resident｜Ephemeral｜Dossier｜URBANITE.md。**不引入「persona」「角色」「档案」**——概念名一律英文原词，一个概念一个名字。

## 7 模块边界

**三件邻居的活，及它们各自的主人**（写「X 归 Y」而非「不做 X」）：

- 落盘与历史归 `storage`：本模块**读** `URBANITE.md`，写入与备份归 storage 与 checkpoint。
- Building 规则（confidential、写域、阅览室准入）归 `city::policy`：本模块只答「谁」，不答「他能做什么」。
- 身份的**呈现**归客户端（`client/`）：Dossier 是数值，界面怎么画它是客户端的事。

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
    pub fn harness_minutes(&self) -> u32;             // harness run 的墙钟上限；缺这一行读作 60
}
pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError>;
pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError>;
pub fn write_rules(city_root: &Path, addr: &Address, text: &str) -> Result<BuildingRules, AxError>;
pub fn rules_path(city_root: &Path, addr: &Address) -> PathBuf;
pub struct RulesCache { /* city_root、按楼存的 (mtime, len) 与规则 —— 私有 */ }
impl RulesCache {
    pub fn new(city_root: &Path) -> RulesCache;
    pub fn load(&self, addr: &Address) -> Result<Arc<BuildingRules>, AxError>;
}
```

**`harness_minutes`**：一次 harness run 在这栋楼里最多跑多少分钟（sprawling-SPEC §8-124），正整数；缺这一行读作 `HARNESS_MINUTES_DEFAULT`（60）。`0` 由 serde 在解析时拒（`NonZeroU32`）：一个到点即停的上限是笔误。它只管 harness：模型 run 的每一回合都经城自己的安全点，停一件事是 `cancel`、停一片是 `halt`，而 harness 的回合在城之外走完，一个既不说话也不结束的 harness 没有别的路让出车道（§12.8 (b)）。

`RulesCache`（`policy/cache.rs`，形状 1 判定）：一个 run 一份，读界的闭包持有它。`load` 先对 `RULES.toml` 做一次 stat，(mtime, len) 与上次读到的相同就交回留着的规则，不同或第一次就走 `load` 读盘求值并按这次 stat 的戳留下。文件不存在或 stat 失败时不留任何东西、每次都走 `load`，于是「没有 RULES.toml」与「被替代的旧文档」两条的答案与不缓存时逐字相同。锁中毒时同样退回 `load`。失败与 `load` 相同，失败不留。决定见 §12.3。

- **规则是一份 TOML，散文没有另起一份文件**：若是 Markdown，读者就得在**任意一行**上匹配 `confidential:`／`write:`／`review:`／`browser:`／`usersbrowser:`／`desktop:`，于是「How work is done here」里一句以 `desktop = true` 开头的话就授予了宿主机的桌面，而一栋没写 `write:` 的楼落到 `Everything`。两处都朝宽松的一侧失败，那是权限读者唯一不许失败的方向。改成 TOML 之后键只在文法给出键的位置成立，`deny_unknown_fields` 让拼错成为一条消息而不是一次静默缺席，`confidential` 与 `write` 都不再有缺省。**散文留在同一份文件里**，作 `does` 与 `conventions` 两个键：拆成两份文档同样能关掉撞键，代价是一栋楼有两种说法且可以互相矛盾。居民拿到的就是这份文件本身的字节，所以城判定的与 agent 读到的是同一串。

- **confidential 四条各有其守处**：模型池锁本地由 `gateway::endpoint` **在会泄漏的那一端**拒（`req.policy.confidential` 即拒，携三段式）；写域止于本楼子树由 `write_domain()` 在构造点拒；数据可入不可出归出网门；**楼里的字节楼外读不到**，归读界（下一条）。**把兜底放在会出事的那一层**，路由错了仍然拦得住。
- **读界：本楼全开，他楼非机密可读，机密楼对楼外全关**。判定只有一处：`kernel::address::may_read(reader_building, target, rules) -> ReadVerdict`（kernel-SPEC §8-2），三臂 `Open`／`Confidential`／`RulesUnreadable(AxError)`。问它的是模型选路的唯一判定处 `runtime::tools::chosen_path`（`crates/runtime/Spec.lean` §8-30-1）：`read` 的路径、`search` 的起点、`search` 不带路径时在城根下走进的每一栋楼，以及经这两件工具读到的 transcript（`crates/runtime/Spec.lean` §8-32），都先过它。`rules` 是目标所在楼此刻的规则：装配层把 `city::Building::of(target)` 与 `policy::load` 接成一个闭包交给 `may_read`，**只在目标出了本楼时才调用**——本楼的读不读盘，他楼的读每次现读规则。规则读不出＝`RulesUnreadable`，一样关：读不出的那份规则可能写着 `confidential = true`，而隐私设置不得朝宽松的一侧失败（本节「没有 RULES.toml」那条同理）。**读界只管模型选的路径**：catalog 名是人在阅览室里准入的（`crates/runtime/Spec.lean` §8-29 起首），不经它；`exec` 在宿主机上跑的命令读得到盘上任何文件，那道墙要 OS sandbox，与「出网」那条的缺口是同一个缺口。
- **没有 RULES.toml 是普通楼；有而不声明是错误**：把隐私设置的默认值悄悄取成宽松的那一边，正是这整个面存在的理由。拼写不是 `true`／`false` 同样拒——读起来像笔误的隐私设置不得解析成许可。
- **confidential 楼声明越界前缀＝拒而不裁剪**：静默裁剪会让文件说一套、城做另一套；拒绝会指出该改哪一行。
- **无声明写域时默认只写本楼**：一栋楼至少能写自己，且不多。`prefixes` 里一条读不出的地址**传播而不跳过**——在读它的地方丢掉，一栋楼就写得比人授予的少，而这件事没有任何一处说出来。
- **`review = true` 是楼级开关**：开则每个 Run 得一棵自己的 worktree，写的东西在别人检查并 merge 之前对楼不可见。**默认关**，与 confidential 的「不声明即错」相反——隐私的默认值不得惄悄取宽，而审查纪律的默认值不得惄悄取严：一个人派一个 Agent 去改一行字并盯着看，应当看得到文件变化。拼写不是 `true`／`false` 同样拒。
- **`## Egress` 列可达域名**：`BuildingRules::egress()` 交 `kernel::egress_target` 判定。类型经 `kernel::EgressAllowlist` 重导出，住哪一簇文件是 kernel 内政（`gate::egress`，公共拼写不变）。**confidential 楼同时列域名＝矛盾，拒**——「数据可入不可出」是那个设置的含义，域名表写在它下面会逼读者自己去调和两句话。
- **执行点与缺口要分清**：provider 路径由 `endpoint` 的 confidential 拒守住；Agent 自己发起的出网是否被拦，取决于 exec 所在的那一臂 `runtime::tools::exec::Confinement` 是否承诺关网（`Guarantee::Network`，每一臂在工具描述里说出自己不承诺什么），浏览器则没有拦截处。判定在这里，拦截不在——不承诺关网的地方不要说「出网已管住」。
- **`usersbrowser` 一键同时是开关与地址**：值 `"ws://127.0.0.1:<port>/session"` 启用并声明地址，`true` 启用而地址未定（工具每次调用都得到门的问题），absent／`false` 即无此工具。**confidential 楼写这一键即拒**（`E_CONFIG_INVALID`）——附着读的是那个人浏览器里全部登录态，本楼的隔离在那一刻失效。地址的**语法**（`ws://`、主机形状）在此读一次并把 `url`／`host` 一起交出；**loopback 与否是 `kernel::gate::attach` 的政策**，语法不替政策作答。
- **`browser` 与 `usersbrowser` 是两个键**：前者是城自己拉起的浏览器（profile 按楼隔离），后者是人已经开着的那个（人的真 profile）。`browser` 不是 `usersbrowser` 的前缀截断——TOML 的键是文法给出的整体，两个设置因此互不误读。
- **`write_rules` 先求值再落盘**：一份写到一半就不再求值的治理文档会把它那栋楼一起带走。且**整份文档才是单位**：confidential 楼不得列域名，故两行可以各自合法而合在一起非法。

### 8-2b city::rules_tool（形状 4 适配器）

```rust
pub struct RulesTool { /* city_root、building、meta —— 私有；op ∈ {read, propose} */ }
impl RulesTool { pub fn new(city_root: &Path, building: Address) -> Result<RulesTool, AxError>; }
// meta.effect = Effect::Govern
```

- **每一次经工具台的调用都在效果层被拒，这是定规而不是漏接**：一个 run 不改写审判它自己的规则（§12.1 定规；kernel-SPEC §8-27 的 `Governance` 行）。`Effect::Govern` 无门、无审批、无 `ApprovalItem`：`runtime::bench::admit` 在 `invoke` 之前就把调用拒掉，拒绝以 tool result 回到模型而回合不终止（`runtime::turn::wave`「A tool Err is not a turn Err」那条）。规则要变只有人改文件这一条路——`RULES.toml` 的唯一写者是人，下一个 run 按改后的字节受审。
- **拒词说出被治理的 scope**：本工具的 `subject` 答 `GateSubject::Scope`（§8-36），效果层因此给出 `E_GATE_DENIED` 的「一个 run 不得改写审判它自己的规则」，恢复语指向人改的 `CONFIG.toml` 与 `RULES.toml`。参数读不懂的调用在同一处被拒，拒词与 `invoke` 读到同样参数时给出的相同；两种拒都出自效果层，run 都到不了 `invoke`。
- **为什么不是 `edit`**：`RULES.toml` 住在楼的保留子树，没有任何写域到得了那里——这不是一个要绕过的障碍，它就是规则本身。写面（`policy::write_rules`）因此只有本工具的 `invoke` 一个调用方，形状是整份提案、先求值后落盘（§8-2 末条）；run 到不了它，人改文件也不经它。
- **楼是携入的而不是参数**：工具持调用方自己那栋楼的地址，于是一个 Run 无法靠填另一个名字去改别人的规则。

### 8-36 两件治理工具说出自己治理的 scope（`rules_tool`、`city_tool` 的 `Tool::subject`，形状 4 适配器）

```rust
impl Tool for RulesTool {
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError>;
    // Ok(GateSubject::Scope("building:<本楼地址>"))，op 读不懂即 Err
}
impl Tool for CityTool {
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError>;
    // Ok(GateSubject::Scope("city"))，action、name、template 读不懂即 Err
}
```

- **scope 的拼法取 `kernel::event::Scope` 的 `Display`**：`rules` 答本楼的 `Scope::Building`，`city` 答 `Scope::City`，`GateSubject::Scope` 里的字符串就是账本上 `rules_changed` 写 scope 的那一种文字（§12.13）。
- **`city` 三个动作答同一个 scope**：`list` 读、`raise` 与 `adopt` 改的都是城的形状，被点名的那栋楼在 `raise` 时还不存在，治理它的不是它自己的规则。
- **参数只有一处文法**：`subject` 与 `invoke` 经同一个读参函数（`rules` 的 `Op::read`、`city` 的 `Request::read`），所以 `subject` 的拒词就是 `invoke` 读同样参数时的拒词（kernel-SPEC 讲 `Tool::subject` 的那一段：文法读不出的调用返回 `Err`，bench 原样拒收）。
- **验收**：两件工具对一条合法调用答出上述 `Scope`，对一条读不懂的调用答出与 `invoke` 相同的码（`city` crate 内两件工具旁的测试）；经工具台的拒词由 `runtime::bench::admit` 的 Govern 臂给出，它的测试已钉住恢复语。

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

// building/removal.rs（形状 2 值类型＋一个实例化动作）
pub struct Removed { /* addr、kept —— 私有 */ }
impl Removed {
    pub fn addr(&self) -> &Address;
    pub fn kept(&self) -> &str;          // 楼的文件现在所在处，相对城根、以 / 分段
}
pub fn remove(city_root: &Path, addr: &Address) -> Result<Removed, AxError>;   // city::remove_building
pub fn removed_payload(removed: &Removed) -> Result<Payload, AxError>;         // building_removed
```

- **移走楼＝把目录整个搬进 `.sprawling/removed/<名>`，不删一个字节**：人造的东西一样不丢，楼的历史仍在 Ledger 里，`building_removed` 记下它搬去了哪里。同名的楼第二次被移走时落在 `<名>-2`、`<名>-3`……第一个空位，已搬走的那份不被覆盖。搬用 `std::fs::rename`：同一卷上是一步，半途失败时楼要么还在原处、要么已整个到位。放在 reserved prefix 下，是因为 `all` 不列点开头的目录，而写域够不到那里——被移走的楼不再是楼，也不能被居民改动。找回＝人把目录搬回城根，再 `adopt`。
- **拒绝**：房间地址（`lab/room1` 不是楼，`AxCode::InvalidArgs`）；City Hall（`HALL_BUILDING`，城自己的楼，地址由城定，`InvalidArgs`）；没有目录的地址（`InvalidArgs`）；搬不动（文件被占用等，`StorageFatal`，恢复：关掉占用它的程序再试）。有活跃 run 的楼由 sprawling 在调用前拒绝——本模块不知道哪些 run 在跑。

- **模板名只有一个家**：`parse` 不再另列一张字符串表，而是拿 `ALL` 里每一个的 `name()` 去比；拒词里的合法集也由同一趟生成。于是加一个模板只改枚举与 `name()` 两处，而「解析认得的集合」与「拒词列出的集合」在类型上是同一个。`Hall` 的名字取 `kernel::consts_policy::HALL_BUILDING`：City Hall 是唯一一栋地址由城而不是由人定的楼，模板名与那个地址是同一个词。

- **楼是顶层地址，房间不是楼**：`create` 拒多段地址（`lab/room1` 是 `lab` 里的一个房间）。嵌套楼会使「这个地址归谁管」多出一个答案，而 `Building::of` 取首段这件事被写域、配置与上报对象三处消费。
- **reserved prefix 下建楼恒拒**：`.sprawling/` 是城自己的账与配置，它在一切写域之外；允许在它下面建楼，就是把一个写域开到账本上。判定用 `Address::is_reserved`，不在本模块重写前缀文法。
- **二次出生恒拒**：已有 `RULES.toml` 即拒（同 `init` 拒第二次创世）。覆写会把一栋已在干活的楼的规则静默换掉，而那份规则可能写着 `confidential = true`。
- **模板字节来自 `crates/city/templates/RULES.toml`（`include_str!`）**：人读的那份模板与城写出的那份必须是同一串字节，否则两份会各自漂。`Confidential` 与 `Minimal` 只差一行（`confidential` 的值），且该差异由 `policy::evaluate` 读回来断言——换行成功与否不靠阅读，靠测试。
- **先落盘再产事件**：`building_created` 记的是已经发生的事。反过来的顺序会让历史声称一栋目录不存在的楼存在，而重放会把这个谎再说一遍。
- **只写不读的 payload**：`created_payload` 只有写面，因为没有读它的投影——楼列表读盘（`city::buildings`）。读面随第一个真正需要它的投影落地，不提前建。
- **占位符只有一个家**：`NAME_PLACEHOLDER` 住 `building::template`，`pub(crate)`；楼的规则与它的计划、备忘、交接读同一个占位符，两份拼法会让其中一份文件永远写着 `<building name>`。
- **adopt（兑现「导入一个已有目录」这件事）**：收编一个已存在的目录为楼。复用 `create` 的全部围栏（房间拒、reserved 拒、二次出生拒），只多一条：**目录不存在即拒并指向 create**——收编不存在的东西是建造，两个动词不共用一个事实。Spine 文档恒不覆写（§8-5 既有约束），故被收编目录的 `Roadmap.md` 保持原主的字节；事件仍是 `building_created`，但 payload 携 `adopted: true`——历史不得声称它建造了它只是找到的东西。CLI 入口 `sprawling adopt <city> <addr>`；城外目录先由人搬入城内再收编，本体不做拷贝。

### 8-4 city::config_layers（形状 1 判定／求值，兼任文件名权威）

```rust
pub use kernel::layout::CONFIG_FILE;               // 文件名的权威在 kernel::layout
pub enum Layer { City, Building, Resident }        // 穷尽三级，与 kernel::LayeredValue 同形
pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError>;
pub struct ConfigLayer { /* model / harness / effort / sandbox / mcp / shelves / remote —— 私有 */ }
impl ConfigLayer {
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError>;   // 纯函数，无 I/O
    pub fn model(&self) -> Option<&str>;                        // 这一级冻下的模型，照写下的读回
    pub fn effort(&self) -> Option<Effort>;
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

**一条梯子是一个值**：`Ladder::read` 按 `Layer::ALL` 由远及近读一遍，落点重复的一级丢弃；`load` 逐个关切在梯子上 fold，不再逐级点名。加一级因此是 `Layer` 多一个臂：`ALL`、`file` 与 `resolve` 三处穷尽匹配同时报编译错，直到新一级被安置，而每个关切一次拿到它。`resolve` 是「哪一级填 `LayeredValue` 的哪一格」的唯一一处答案——今天 `kernel::LayeredValue` 只有三格，所以人层（`~/.sprawling/config.toml`）进梯子时，`kernel::config` 与本模块在同一次改动里走完。

**来源与值一起答**：`settled_effort` 与 `load` 爬同一条梯子，区别只在它把说出这个值的那一级留着而不是丢掉。只被告知结果的设置页说不出「这是这间房自己写的」还是「这是全城都有的」，于是它只能把三份文件各读一遍、把同一条梯子再爬一次——**同一个问题两个答案，就是从第二次爬梯开始的**。谁压过谁仍由 `kernel::LayeredValue::resolve` 判：`tagged` 只负责「哪一级填哪一格」，`resolve` 是 `tagged` 去掉那一级，所以这条映射在本 crate 里只有一处。`None` 是整条梯子什么都没说，也就是这座城有意把强度交给供应方，而不是替人填一档。`settled_second` 是同一条路的第二个值：第二道提醒阈值也说得出是那一级写的。

**写面与读面同源**：`write_second_threshold` 与 `write_session` 走同一扇门（读—改—写整份 `CONFIG.toml`，别人写的键原样保留），收的值已经是 `SecondThreshold`——域在那一个构造点判定过，写面不再判一次；要值的字符串形状或拒因句式，答案在 `kernel::config`。

**`own_layer` 答的是另一个问题**：梯子回答「一个 Run 被什么治理」，它回答「这个地址自己写下了什么」。差别正是它存在的理由——城或楼那一级给出的默认值不是这个地址做的选择，所以它不能当作选择的记录。文件是地址自己的那份 `CONFIG.toml`（`Layer::Resident` 的落点，也就是会话写的那一份），哪怕梯子把同一份文件当作两级里更远的级读了一次；地址就是楼时两者是同一个文件，所以那种地址自己就是它的会话（§8-14）。

**门面上换名**（`lib.rs` 按能力组织，不按文件组织）：`load` 已归 `policy`，故本模块对外是 `city::load_config`；`path` 对外是 `city::config_path`；`building::create` 对外是 `city::create_building`，`created_payload` 对外是 `city::building_created_payload`（名字读起来就是它记的那个事件）。

- **文件名只有一份，层级由位置决定**：City 层住 `<city>/.sprawling/CONFIG.toml`（reserved prefix 内，因此任何 Resident 的写域都永远叠不上它——「Agent 改不了自己的配置」因此是判定而非推理）；Building 层住 `<city>/<building>/.sprawling/CONFIG.toml`；Resident 层住 `<city>/<addr>/.sprawling/CONFIG.toml`（§8-11）。三处同名，读者认一次就认得完。
- **地址就是楼时只有两级**：`addr` 与它的 building 相同时，下两级指向同一个文件，只读一次并放在 Building 级。同一份文件在两级各算一次不改变结果，却会让读者以为它能覆盖自己。
- **缺文件不是错，读不动才是**（同 `resident`）：未声明即每级 `None`，落到 `kernel::consts_policy` 的缺省；一份存在却读不出的配置报 `E_STORAGE_FATAL`。
- **梯子上的拒词带着文件，其余照解析器说的**：`ladder::stated` 读一份解析不了的文件时，把文件路径加在 subject 前面，码、action、nearby 与恢复语都照 `ConfigLayer::parse` 给的原样交出。恢复语是写拒词的那一处按它的场合写的（`two_residents` 指向 `/new` 或删键，`unreadable` 指向报错所在的那张表），梯子只知道「哪份文件」，不知道「怎么改」（§12.8 (a)）。
- **不认的键即拒**（`deny_unknown_fields`）：静默忽略一个拼错的键，会产生「我设了 effort 而什么也没发生」这个无从诊断的状态。本版读哪些键，由 `ConfigFile` 的字段给出，此处不复述；拒绝文字也不复述这张键表——serde 的报错点名不认识的键、列出该表接受的键，恢复语只从原文里取出报错所在的那张表头（`[model]`、`[[mcp]]`）并说改哪一节。`[clock]` 只受理 `stamp` 一个键（§8-31）；`zones` 仍拒，写它得到的是 serde 点名这个键的那句拒绝而不是一份沉默。
- **梯子不在本模块重建**：下层胜上层由 `kernel::LayeredValue::resolve` 给，冻结由 `kernel::freeze` 给；本模块只回答「哪三份文件、怎么读」。一条规则一个权威。
- **effort 属 `FrozenConfig` 而非 `LiveConfig`**：改它会作废 message cache breakpoints，因此改动只影响下一个 Run（理由已写在 `kernel::config`，此处不重述只遵守）。

**`[[mcp]]` 一节**：`CONFIG.toml` 第三节 `[[mcp]]`（表数组）。`label` 解成 `kernel::ServerLabel`，非法即拒并报出是哪一份文件；其余字段按这台服务器怎么够得到分两组：

- **`command`（非空）＋ `args`（缺省空表）＋ `env`（缺省空表，形如 `env = { API_KEY = "secret:mcp/apps" }`）**——城所在的机器上的一个程序。写成表而不是 `NAME=value` 行的列表：文件里一个名字只出现一次，也没有谁要去切一个人写的字符串。
- **`url`（非空）＋ `headers`（缺省空表，写法同上）＋ `transport`（`"http"`｜`"sse"`，缺省 `"http"`）**——一个地址。`transport` 是闭集而不是自由文本，拼错在写它的地方就被拒，而不是变成一台没人够得到的服务器；缺省取 `http`，因为「发一条消息过去」正是一个 url 的本义。

两组各自成行：`command` 与 `url` 同时出现即拒（读者要去猜），两者都不出现也拒。`command` 一行再写 `transport` 同样拒——命令走它自己的管道，再指一条流就是一行说了两种 transport。两张表的值都可以是 `secret:realm/name` 引用，兑付不在本模块（sprawling-SPEC §8-4／§8-15）。两条口径：①**同一层内标签不得重复**——两个同名 server 会让同一个工具名同时指向两个进程，而那是一个路由错误而不是一个偏好；②整表上梯（下层写即替换上层全表），语义住 kernel-SPEC §8-22，此处不复述。**不在本模块的事**：进程怎么起、起不来怎么办、confidential 楼凭什么拒——那三件全在装配层（sprawling-SPEC §8-4），本模块只回答「三份文件说了什么」。

**`[sandbox]` 一节**：`CONFIG.toml` 第二节 `[sandbox]`，字段 `shell`（bool，默认 false）、`fuel`（整数，缺省取 `SANDBOX_FUEL_DEFAULT`）、`mounts`（相对 city root 的路径表，reserved prefix 在解析点即拒）。解析仍是「本版本不读的键即拒」——被写下却什么都不发生是唯一没人能诊断的状态。整节整值上梯，语义住 kernel-SPEC §8-22，此处不复述。

**`[sandbox]` 的第四个字段**：`env_passthrough`（字符串表，缺省空表），逐项解成 `kernel::EnvVarName`。与 `mounts` 逐条同形：同一节、同一次整值上梯、同一个解析点拒——`mounts` 在这里拒保留区，`env_passthrough` 在这里拒凭据形状的名字，而“什么叫凭据形状”是 `kernel::secret` 的答案，本模块不重建它。拒绝文字里带着是哪一份文件、哪一个名字，因为一份配置被拒时人手里只有那句话。

**`[sandbox]` 的第五个字段**：`trusted`（字符串表，缺省空表），逐项解成 `kernel::ServerLabel`。它答的是「这层楼允许哪一台 connector 服务器去做城里谁都收不回的事」（今天只有运行这座城的机器的桌面），语义与读者住 `kernel::SandboxLimits::trusts`，本节只管它在 TOML 里怎么写、在哪一层写、写错了在解析点怎么拒。与 `mounts`／`env_passthrough` 同形：一名一行、缺省为空、整节整值上梯——收不回的效果按服务器逐台放行，而不是一次放宽给所有人。

**`[context]` 一节**：`CONFIG.toml` 第四节 `[context]`，一个字段 `second_threshold`（整数百分比）——上下文提醒第二道阈值响在哪一格。值的形状与合法域住 `kernel::config::SecondThreshold`（kernel-SPEC §8-22），提醒怎么响住 `runtime::reminder`，本节只管它在 TOML 里怎么写、在哪一层写、写错时在哪拒。与 `mounts`／`env_passthrough`／`trusted` 逐条同形：整值上梯、Run 起点冻结、解析点拒——30–90 域外的值在解析点由 `SecondThreshold::parse` 拒（`E_INVALID_ARGS`，恢复语带合法域），不钳位、不读后丢。缺省是「没有一层说话」，而「缺席取 `CTX_REMINDER_SECOND_DEFAULT`」只在 `runtime::reminder` 一处判定。

**`[cache]` 一节**：一个字段 `keep_warm`，取 `off` 或 `five_minute`——这一层要不要在提示缓存到期前续期。值的形状与续期判定住 `kernel::keep_warm`（kernel-SPEC §8-74），本节只管它在 TOML 里怎么写、在哪一层写。拼写由 serde 按闭集读，拼错的词与未知键同样在解析点拒。`city::keep_warm(city_root, addr) -> Result<KeepWarm, AxError>` 爬同一张梯，下层覆盖上层；一层也没说时答 `KeepWarm::Off`——续期是人付钱的请求，默认必须是不发。与 `[context]` 不同，它不进 `FrozenConfig`：续期发生在两次 run 之间。

**`[resident]` 一节**：一个字段 `harness`（字符串），点名这一层以下的房间由哪家官方 harness 当居民。值照写下的读进来：五个拼写的权威是 `agent_protocols::Harness`，本 crate 只见 `kernel`，认不认得由派活路径判（sprawling-SPEC §8-4e 第 10 条）。空串在解析点拒，与 `[model] name` 走同一条判定：空值什么也没说，写它是笔误。

- **一层只点名一种居民**：同一层既写 `[model] name` 又写 `[resident] harness`，解析即拒（`E_CONFIG_INVALID`），拒词带两个键和各自的值。居民是模型还是 harness，要读者去猜，就是配置写错了。`ConfigLayer` 的字段私有、`parse` 是唯一构造点，所以两键并存的值构造不出来。地址就是楼时，楼层与房间层是同一个文件；人要在这样一个带着会话记录的文件里写 harness，先 `/new` 清掉会话写下的 `[model] name`。
- **harness 爬梯子，`[model] name` 不爬**：`settled_harness` 与 `settled_effort` 爬同一条梯子，下层胜上层，连同说出它的那一级一起答。`[model] name` 仍只是地址自己那一层的会话记录（`own_layer`，§8-14），不参与求值。
- **会话记录压过梯子**：地址自己那一层有 `[model] name` 时，`settled_harness` 答 `None`。这段会话以模型开场，就以模型走完，理由与 sprawling-SPEC §8-79「会话的形状只选一次」相同。人在楼层或城层写下的 harness，从 `/new` 开的下一段会话起生效。答案针对「一次 run 在这个地址上接着跑」；派活开新房间时，新房间自己那一层是空的，梯子的答案就是它的答案。
- **城自己的写路径写不出两键并存的文件**：见 §8-4b。

### 8-5 city::spine_files（形状 6 数据面＋落盘动作）

```rust
pub const ROADMAP_FILE: &str = kernel::ROADMAP_FILE;
pub const CITY_FILE: &str = "City.md";
pub const MEMO_FILE: &str = "Memo.md";
pub const HANDOFF_FILE: &str = kernel::layout::HANDOFF_FILE;      // 红测要点名它
pub const AGENTS_FILE: &str = "AGENTS.md";                        // 项目自带的约定；城不写也不拥有

pub struct JobBrief<'a> { pub task: &'a str, pub goal: &'a str }
pub enum RunBrief { Job { text: String }, Principal }          // 穷尽两臂
pub(crate) fn lay_out(building_root: &Path, addr: &Address) -> Result<(), AxError>;  // 唯一调用方是 building::create
pub fn job_path(city_root: &Path, addr: &Address) -> PathBuf;
pub fn roadmap_path(city_root: &Path, building_addr: &Address) -> PathBuf;
pub fn roadmap(city_root: &Path, building_addr: &Address) -> Result<String, AxError>;
pub fn write_job(city_root: &Path, addr: &Address, brief: &JobBrief<'_>) -> Result<String, AxError>;
pub fn write_brief(city_root: &Path, addr: &Address, brief: &JobBrief<'_>) -> Result<RunBrief, AxError>;
pub fn handoff_path(city_root: &Path, room: &Address) -> PathBuf;           // 交接住房间（§8-24）
pub fn handoff(city_root: &Path, room: &Address) -> Result<Option<String>, AxError>;
pub struct HandoffSections { pub overall: Option<String>, pub progress: Option<String>, pub context: Option<String>, pub next_step: Option<String> }
pub fn handoff_sections(text: &str) -> HandoffSections;           // city::handoff_form
pub fn norms(city_root: &Path, addr: &Address) -> Result<Vec<PathBuf>, AxError>;
```

- **四文档三写一不写**：`lay_out` 写 Roadmap／Memo／Handoff；`RULES.toml` 归 `building::create`（它的含义归 `policy`）——同一份文件有两个写入者就是两个权威。
- **已存在的文档恒不覆写**：一栋已在干活的楼的计划不得因为又跑了一次建楼而回到空白。
- **模板的占位行不进新楼的 Roadmap**：`crates/city/templates/Roadmap.md` 里的两行 `Not started` 是给人看的例子；照抄进去，一栋新楼开局就有两件不存在的待办，而它们会进分母。实例化时删掉 Item 列为空的数据行，断言是「新楼的分母是 0」。
- **JOB.md 先落盘，再产 `run_started`**（模板第一行就这么写）；内容同时进 CAS，于是盘上那份是现场、CAS 那份是历史——Agent 改了 JOB.md 也不会使「当时派的是什么活」不可考。同一个房间再派一件活即覆写它（JOB.md 是本次会话的任务，不是档案）。**人那句话在表单里只出现一次**：标题只写 `# JOB.md`，任务正文只进 `<task>` 节——标题再插一遍，一段粘贴每次请求就多付一遍。
- **机器只填它知道的段**：Task／Goal 两段有事实就写；Background／Delivery 无事实则不写——写一个 `(未知)` 占位，只是让模型每回合读一遍没信息的行。
- **一次会话的 brief 只有两种，且由本次派活决定**：说得出 Goal 的就写 `JOB.md`（`RunBrief::Job`），说不出的就不写（`RunBrief::Principal`）。**依据选 Goal 而不选「盘上有没有 JOB.md」**：一个房间里上周留下的任务书仍在盘上，它可以被读，但不得冒充一次没人派任务的会话的 brief。Goal 是那份表单里唯一不可替代的一栏（什么时候停），它空着就等于告诉 Agent「停不停没定义」。
- **`handoff` 不把空白表单当交接件**：一张没填过的 `Handoff.md` 与一张填过的占同样的 prefix 字节而一个字的信息也不带。识别靠模板自己的括号提示行。
- **「读不了」不并进 `None`**：`None` 只说「没有值得带走的东西」（不在，或空白表单），读不了则以 `E_STORAGE_FATAL` 上报并带路径——与同模块的 `roadmap` 同形。下一次会话正是从这份文件装配的，静默省略等于告诉它上一次没留下任何东西。
- **计划的路径与读法归本模块**：`roadmap_path` 与 `roadmap` 落在这里，因为 `ROADMAP_FILE` 在这里——在别处拼 `city_root/<addr>/Roadmap.md` 就是第二份「计划在哪里」的权威，它会在真正那份搬家后继续跑得好好的。
- **「还没有」与「读不了」是两件事**：`roadmap` 仅对 `ErrorKind::NotFound` 答空串——一栋还没铺计划的楼确实没有计划；其余任何理由一律以 `E_STORAGE_FATAL` 上报并带上路径。这与同 crate 的 `archive::index` 已有的契约同形（目录不在→`Ok(空)`，真失败→`Err`），不新立一种读法。
- **交接表单的读法归本模块**：`handoff_sections` 把 `<overall>`、`<current-progress>`、`<context>`、`<next-step>` 四节各读成一段正文；一节缺席、或只剩模板的括号提示行，即 `None`。括号提示行的判断与 `is_blank_form` 共用 `blank::is_guidance` 一处，因为「这一行是不是模板自己的话」只能有一个答案。`<must-read>` 节不读：它是写给下一个 Agent 的散文而不是 Locator，装配层把整份文件入 CAS，作为 must-read 的一条。被否决的备选：在装配层按标签切字符串——那是模板格式的第二个读者，模板改一个标签它就静默读到空。
- **规范类 must-read 由 `norms` 给路径，不给 Locator**：Locator 需要 CAS 或 git oid，而 city 不认识落盘物（拓扑上也依赖不到 storage）。本模块答「哪几份是规范」，装配层把它们入 CAS 变成 Locator。这也是 must-read 最大失败模式的解：不让模型凭记忆重抄规范清单。

### 8-41 城写下的模板住在 city 自己的包里（`crates/city/templates/`、`city::CITY_TEMPLATE`，形状 6 数据面）

```rust
// city::spine_files
pub const CITY_TEMPLATE: &str = include_str!("../templates/City.md");   // 立城时写下的 City.md；accounting 的创世读它
// 其余模板仍是本模块与 building::template 的私有常量，各自 include_str!("../templates/<文件>") 或 ("../../templates/<文件>")
```

- **一个目录装城写下的全部第一批字节**：`crates/city/templates/` 下是 `RULES.toml`、`RULES-hall.toml`、`Roadmap.md`、`Memo.md`、`Handoff.md`、`SPEC.md`、`JOB.md`、`MAYOR.md`、`CLERK.md`、`URBANITE.md`、`City.md`，以及说明每份文件谁写、谁读的 `README.md`。人读的那份与城写出的那份仍是同一串字节（§8-3、§8-5），只是这串字节现在住在把它编进去的包里。
- **为什么在包里**：crates.io 上的 `.crate` 只装包目录里的文件，验证构建与 `cargo install` 都在解开的包里编译；`include_str!` 指向包外的 `docs/` 时，那里没有这些文件，city 编不出来（sprawling-SPEC 8-157）。`packaged` 门判这一条（xtask-SPEC §8-49）。
- **`City.md` 由 city 交出，accounting 来读**：立城时写下 `City.md` 的是 `accounting::worker::genesis`，但这份文件的名字（`CITY_FILE`）与它同目录的模板都归本模块。accounting 的 `CITY_MD` 是 `city::CITY_TEMPLATE`，一份字节、一个家；accounting 不能 `include_str!` 别的包目录里的文件，那在包里同样落空。
- **失败**：没有运行期失败。模板改名或挪走，`include_str!` 在编译期就红。

**验收**：`cargo xtask gates packaged` 对 city 与 accounting 为绿；§8-3、§8-5、§8-15、§8-20 现有的模板测试照旧通过（它们读的是同一串字节）。

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
pub struct Holding { pub name, pub section, pub disclosure, pub hash, pub shelf: Shelf, pub package: Option<Address>,
                     pub carried: Option<String> }   // 城外书架上的一件：扫描读到的正文；城内书架为 None
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
- **`Shelf::address()` 答的是 catalog 承诺一件 skill 之前要问的那个问题**：城内书架上的一件靠地址打开，城外书架上的文件没有地址。所以**不发明一个假地址**：拼一个看起来像 `Address` 的字符串，会让读者去开一个并不在那儿的文件，且失败发生在第一次 `read` 而不在写下列表的那个时候。
- **城外书架上的一件带着它的正文**（`Holding::carried`）：扫描为了取一行披露与哈希本来就读了每个字节，留下这份正文，阅览室就能把它整份交给 catalog（`crates/runtime/Spec.lean` §8-29-6），run 按名读到的字节与 `hash` 出自同一次读入。只在 `Holding::of` 一处派生：`shelf.address()` 为 `None` 时为 `Some(正文)`，否则为 `None`——有地址的一件由 `read` 到地址去开，再带一份正文就是同一份文档的第二个家（§12.9）。
- **外部书架只读挂载，路径由城自己的配置给出**（`[skills] shelves`，城层一份）：`city_shelves` 在城的那一级上读它，而不是走三层梯子——书架是这座城所在的文件系统上的一个目录，它对每一栋楼同时挂上，让楼或房间能自己挂一本就是让一个作用域准入一份没人选过的文件。楼或房间写下它即在读文件处拒（`E_CONFIG_INVALID`，恢复语说把它移进城自己的 `CONFIG.toml`），而不是解析后丢掉——一份被接受却什么都不发生的配置，写它的人无从诊断。
- **外部书架的布局属于写它的那个 harness**：一层目录一件 skill、目录里放 `SKILL.md`（`SKILL_FILE` 是本城写下这条布局的**唯一一处**）；目录名就是 skill 名。**section 为空**：那棵树没有 section 这一级，替它编一个就是本城对一份它不拥有的东西的猜测。不在那里、不是目录的外部路径直接跳过（空架就是空架）；是文件而不是目录则以 `E_CONFIG_INVALID` 拒并报出是哪一条——一句「配置写了却什么都没发生」是没人能诊断的状态。
- **外部路径里的 `~` 指这个人自己的 home，home 以参数传入**：读环境不是本 crate 的活（`bin::assembly` 给出 `Home`），因此测试扫的是测试自己造的目录。committed 的城配置里不放一台机器的绝对路径，这正是 `~` 存在的理由。不是 `~` 开头也不是绝对路径的条目在解释处即拒，恢复语说出这条规则。
- **同一名的优先级是 building > library > external（外部按数组序）**：按最远的架先上、近的盖上去，于是城自己的存货盖过别的程序的目录——城留下一个名字时，那个名字指城的 skill；楼的自己一份又盖过城的。外部条目排到最后，因为它是别人写的：一份目录不是一个权威。
- **目录的架次由 `Shelf` 的臂序给出**：`Library::all` 先按 `Shelf::catalog_position`——城库、楼架、外部架按 `[skills] shelves` 的数组序——再按 `(section, name)`。位置是对三条臂的穷尽 match，不另立一张次序表；加一条臂而不在这里安置它，本 crate 编译不过。外部持有者没有 section，若把全部持有者按 `(section, name)` 排，别人的目录会排到城自己的存货之前，那正是上一条优先级的反面。
- **一件藏品可以是一个目录**：section 书架上的一项是 `<name>.md` 一份文档，或 `<name>/` 一个包——包里的 `SKILL.md`（`SKILL_FILE`，与外部书架同一布局）就是这件持有：catalog 只列它的第一行，`Holding::shelf` 指向它，`hash` 是它的字节，`Holding::package` 是包目录的地址——一件藏品是不是包由扫描在这里说出，读包的一方不从地址的写法去猜：一份恰好叫 `SKILL.md` 的单文档会让整个 section 被当成包，把阅览室没准入的藏品一并交出去。单文档与城外书架上的持有 `package` 为 `None`（后者没有地址）。包里其余文件不是持有，留在架上由 `read` 按 `<名>/<相对路径>` 打开（`crates/runtime/Spec.lean` §8-29-3）；只列一行是常驻上下文不随包膨胀的原因。没有 `SKILL.md` 的目录不是藏品，跳过而不报——与外部书架同形。
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
```

- **是判定，不是动作**：新城由什么构成，在这里以值给出；建目录与落事件归 bin。这个切分使「一句指令建一座城」可以在不建城的条件下被断言。
- **没有 `city::office`**：三层配置（City／Building／Resident）已是完整的梯子，OFFICE.md 没有任何一条自有规则，第四层只会成为「同一个设置在哪儿写」的第二个答案。

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
- **`crates/city/templates/URBANITE.md` 早就承诺了这件事**：模板原话是「other agents and the person read it to know what to expect from them and what to bring to them」。承诺写在模板里，兑现它的代码在本模块。
- **一行取自 `## Bring them`，取不到才退回第一段正文**：这一行要回答的是「我为什么找他」，而模板里正是那一节写「什么样的活属于这位住户」。退回规则跳过标题行与引文行——引文行是模板留给作者的说明，把它显示出来等于让全城住户共用一句自述。**这与 `library::first_line` 不是同一条规则**：书架条目取的是标题，住户名册取的是正文，两种文档、两条规则、两个家。
- **准入判定复用 `Identity::load`，不自读文件**：「一个地址上有没有常住的人」已经有权威，第二次实现必然在某天与第一次分叉。空的 `URBANITE.md` 仍是 Resident（`bring` 为空串），沿用 §11 已记的口径：空描述是作者的选择，不是缺陷。
- **空房间照列，不隐藏**：藏起来的话，模型会把「这里没人」读成「这个地址不存在」，而一间空房恰是可以请人搬进来、或派一件活过去的地方。
- **详略随距离衰减**：本楼给到每个地址的自述，全城只给楼名。这不是新规则，而是 `signal` 的 `reach` 与 `CrossBuildingTransfer` 已经画好的那条界——**看得清的范围与说得着的范围必须是同一个**，否则名册会教模型去够它够不到的人。
- **房间＝楼下一层的非点头目录，且不是 archive 目录**：这条规则与城级的同类规则分别只住 `city::room::all` 与 `city::building::all`，装配层只调用它们——页面看到的房间与模型看到的房间因此不可能不同。
- **只到直接子目录**：房间就是这样被造出来的（`room::open` 与 delegate 都建直接子目录）。翻案条件：楼层真的成为目录的那天，改的是 `room::all` 一处。
- **一个活口径接进来了，另一个被判定为噪音**：`waiting`（那间房积压几封信）由装配层以闭包供给——队列是它的，本 crate 看不到那么远。而「谁在跑」**不接**：多条 lane 同时驱动，「在跑」在一次 drive 之内就会变，而名册随 Run 冻结（§8-15b），冻下来的一列「在跑」一回合后就可能是错的；`Dossier::is_live` 真正能说的是「某位的上一跑没冻结过」，那是崩溃后的事实，归 `resume` 而不归名册。

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
- **随 Run 冻结，与 catalog 同理**：名册是派活那一刻扫到的地址与自述。别的 lane 上的 run 可能在这一次 drive 之内开出新房间，这份名册要到下一次派活才看得见它；本 Run 发出的 signal 在 drive 结束后才投递。`Temporal::Timeless` 说的是这份冻结的名册在回合之间不变，而不是城在回合之间不变。

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

**决定一条记录是什么，与把它写上架，是两步**。合成一步的 `file(..) -> Entry` 会让调用方拿到 `Entry`（账本行要的 `kind`／`day`／`subject` 全在里面）时文件已经在架上了；账本行只能后落，而 Ledger 的定义是「Every effect becomes an EventRecord first」。拆开之后：

- `entry` 是纯的：拒空 subject、`day_of(at)` 取整天、按 `<building>/Archive/<kind>/<day>-<slug>.md` 算出落点，全部只读入参。**一条记录是什么，在它到达任何地方之前就已经确定**，所以调用方可以先把它落账再把它写上架。
- `file` 只写：经 `city::document` 把正文整份换上去，建目录也由那一处做。它收一个 `&Entry` 而不是六个参数——落点由 `entry` 算过一次，`file` 不再第二次决定它。
- **仍是一个构造点**：`Entry` 的字段没有对外的写面，`index` 那一支是从盘上读回来的另一种来源（`subject_of(&text)` 而不是入参），两者不共用同一条不变式，因此没有第二个权威。

### 8-4b 两个没人写的配置层长出写面

```rust
pub fn write_sandbox(city_root: &Path, addr: &Address, layer: Layer, limits: &SandboxLimits) -> Result<(), AxError>;
pub fn write_mcp(city_root: &Path, addr: &Address, layer: Layer, servers: &[McpServer]) -> Result<(), AxError>;
```

- **与 `write_session` 同一道门**：梯子（城→楼→房间）本就是「一个 Run 被什么治理」的权威，第二个存储就是第二个答案。其余键原样保留，因为可能是人手写的。
- **写出的字节必须是 `ConfigFile` 读得回来的那种，`change` 在落盘前自己读一遍**：`McpServer` 的 serde 形状是嵌套的，而文件语法是平的（`label` ＋ `command`/`args`/`env` 或 `url`/`headers`/`transport`），写面照文件语法拼，一条往返测试逐支覆盖三种 transport。`Sse` 一行必写出 `transport = "sse"`：缺省是 `http`，不写就会被读回成另一种 transport。拼错之外还有组合错：往一份点名了 harness 的文件里写会话的 `[model] name`，写出的文件每个读者都拒，整栋楼从此派不出活。所以 `change` 把改好的整份文本先交给 `ConfigLayer::parse`，拒了就以 `E_CONFIG_INVALID` 拒这次写，原文件一字不动。读面是文件语法的唯一权威，写面复读一次，就不必在写面另记一份「哪些键不能并存」。
- **空的 `mcp` 表要写出来而不是省略**：省略即继承上一级，而一个人删掉最后一台服务器不是想继承一台。
- **`env` 与 `headers` 逐值判定凭据，落盘之前就拒**：一个值只要不是 `SecretRef::parse` 认得的 `secret:realm/name`，名字命中 `kernel::names_a_credential` 或值命中 `kernel::scan` 即以 `AxCode::ConfigInvalid` 拒，恢复语指向金库。判定在 `write_mcp` 进 `change` 之前逐对做，因此一次被拒的写入一个字节都没落；拒绝文字报出是哪一台服务器、哪一张表、哪一个名字，因为人手里只有那句话。**理由是这份文件进版本库**：楼的 `CONFIG.toml` 由 `city::gitignore` 放行进历史（§8-21），写进去的 key 就在这个项目的每一次克隆里。**判定不重建**：「什么叫凭据」是 `kernel::secret` 的答案，与 `[sandbox] env_passthrough` 走 `EnvVarName::parse` 是同一个权威的两次调用。
- **读面不判这一条**：手写进 `CONFIG.toml` 的明文 key 仍然读得回来（`ConfigLayer::parse` 不调凭据谓词）。补齐要让 `ConfigLayer::parse` 调同一个谓词，那时谓词升为 `pub(crate)` 并只有一处实现。
- **所有写面只有一条写路径**：各自把要说的话包成 `Change`（穷尽：`Session` / `Forget` / `Sandbox` / `Mcp` / `SecondThreshold`），同走内部的 `change`——取 `city::document` 对这份文件的持有、读、改一个键、整份原子换上去。各自读写时，两个会话改同一份 `CONFIG.toml` 会各自从同一份原件出发，后写的那一个抄掉先写的那一个的改动。`[model]` 那一节的三个值由同一条 `table` 找到或建出，免得三处对「该写进哪张表」各有各的说法。

### 8-14 一次会话选一次：模型与思考强度

```rust
pub fn write_session(city_root: &Path, addr: &Address, model: &str, effort: Option<Effort>) -> Result<(), AxError>;
```

思考强度放在派活按钮旁边，因为一次会话反正只选一次；而会话选定的模型也写进同一份文件，因为供应的 prompt 缓存对着的正是这两个值。

- **一次写下一个会话冻下的两样东西**：`write_session` 落 `[model] name`，并在人选了强度时落 `[model] effort`。模型总写下（一个模型总在指某个东西），强度只在人说了时写下：缺席不是一个值，而是「让供应方决定」，写出来就是把一个没人做的选择记成记录。
- **写进那一层，而不是另存一份**：选择落到会话自己房间的 `CONFIG.toml`，由已有的 city → building → room 阶梯解析。第二个存处就是第二个答案。
- **这份记录是会话的，不是运行时的设定**：一个 Run 用哪个模型仍由 endpoint book 选，强度仍爬同一条梯子——两者决定会话从哪里开始。房间写下来的只是它当初从哪里开始，所以登记面之后搬了家，是下次派活拒掉的分歧，而不是它默默执行的变更（`sprawling-SPEC §8-79`）。
- **只改 `[model]` 表里的键**：文件里其它键是人写的，读出来、改一个值、写回去，与其余写面同走 §8-4b 的那一条写路径。文件读不动或解析不了就**拒绝**，不覆盖——一份本构建看不懂的配置不是可以随手盖掉的配置。
- **只写 Resident 层**，层级不由调用方给：派活按钮旁边选的强度属于这一次会话的房间。需要按层写强度时，签名要多一个 `Layer` 参数，那是一次公开面变更。
- **落点就是地址自己的 `CONFIG.toml`**，所以这个房间跑的 Run 读得到、改不了自己的档位；地址就是楼时它就是楼自己那份文件，也就是楼根上的会话（§8-4 的 `own_layer`）。

### 8-14b 一段会话开始：清掉冻下的形状与交接槽位（`city::session`）

```rust
pub fn clear_session(city_root: &Path, addr: &Address) -> Result<(), AxError>;
pub fn forget_shape(city_root: &Path, addr: &Address) -> Result<(), AxError>;
```
两个名字而不是一个带开关的名字：`clear_session` 是 `Carry::Nothing` 的整件事（形状与交接槽位都清），
`forget_shape` 是 `Carry::Handoff` 要的那一半（形状清、交接留）。**两者都不是「可选参数」**：
命令行与线协议各自都有它们要的那个动词（sprawling-SPEC §8-82），而传一个 `bool` 到这里会让「带不带」
在城的接口上多出一种拼法。

**原因**：房间的第一个 run 把 `[model]` 与 `effort` 写进它自己的 `CONFIG.toml`，此后形状不同的派活全被拒（§8-14），没有逆操作，换过主模型的人就永远派不出去。这个函数就是那个出口的城侧一半（动词在 sprawling-SPEC §8-82）。

- **两条写，一个决定**：新的一段开始时，房间自己写下的 `[model] name` 与 `[model] effort` 删掉（`write_session` 的逆操作），房间的 `Handoff.md` 同时清空。放在一个函数里，是因为「这一段从这里开始」是一个判断：拆成两个调用，就有一个可能没被调到，而两种半清理的状态都是假话。
- **交接槽位必须清，否则「不带」是假话**：`accounting::worker::freezing` 无条件读 `city::handoff(root, room)` 并把它折进下一个 run 的 prompt；只清配置而留文件，新一段仍会继承上一段的摘要，于是开关不起作用。
- **清的是槽位，不是内容**：`Handoff.md` 不在城的 git 里（`gitignore.rs` 列了它），账本的 `handoff_written` 记的是派活前填好的交接而不是这个文件，所以删掉的是上一段会话写的唯一一份；要带走它就用 `/new --carry`。**删文件而不写一张空白表**：`handoff` 对两者都答 `None`，而删掉少一个可被读到的中间状态。被否决的备选：保留文件、让装配器忽略「比本段起点更早」的那一份——那要求会话记住自己的起点，等于给「这份交接是不是我的」立第二个家。
- **`[model]` 之外一个键都不动**：文件里其它键是人写的，读出来、改这几处、写回去，与 §8-14 同走那一条写路径；读不动或解析不了即拒绝，不覆盖。
- **它不另立「有没有交接」的判断**：`city::handoff` 的「文件缺席，或是一张空白表，即 `None`」就是那条判断的唯一权威，`clear_session` 只把槽位清空，不重判它。

**本章测试**：`session::tests`——清后 `own_layer` 答不出模型与强度，而同一份文件里人写的其它键一个字节未动；清后 `city::handoff` 答 `None`。

### 8-13 city::room（形状 1 判定 ＋ 一个实例化动作）

```rust
pub fn open(city_root: &Path, building: &Address, name: &SessionName) -> Result<Address, AxError>;
pub fn claim(city_root: &Path, room: &Address) -> Result<(), AxError>;   // 门面上叫 `city::claim_room`；城替派活新建的房间：建出并封上；已存在的目录不动（§8-21）
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
- **落点只在 `Holding::shelf`**：扫盘时算一次（§8-8）。从 section＋name 拼回一个城级路径就是第二个权威，且在两层书架下其中一个必然答错。

### 8-12b 一本书带着它被读到时的样子
```rust
pub struct Holding { …, pub hash: B3Hash }   // 整份文档的 BLAKE3，扫架时算
```

- **它是白得的**：`shelve` 为了取 disclosure 那一行，本来就把整份文档读进了内存；哈希只多走一遍已在手里的字节。
- **为什么存在 `Holding` 而不是让读者自己算**：读者要的答案是「它变了没有」，而那需要**两个时刻各一次读取**；一张只能报当下内容的书架永远答不了这个问题。早一次的那一读由 `run_started` 携走存进账本（`crates/runtime/Spec.lean` §8-11），于是比对对的是**这座城自己的历史**，不是一份签名：它只能说「这变了」，永远不说「这安全」。
- **名字不变而字节变了，正是注入的样子**，而只按名字核对的读者发现不了它。

### 8-11 楼的治理字节搬进它自己的保留子树（沉淀一处路径权威）

```rust
pub fn rules_path(city_root, addr) -> PathBuf;      // <building>/.sprawling/RULES.toml
pub fn agents_path(city_root, addr) -> PathBuf;     // <building>/AGENTS.md（项目的，故在保留区之外）
pub fn city_agents_path(city_root) -> PathBuf;      // <city>/AGENTS.md：城围起来的工作区自己的那份
pub fn config_layers::path(city_root, addr, layer) -> Result<PathBuf, AxError>;
// 三层统一为 <scope>/.sprawling/CONFIG.toml；city 层因此不再是特例
```

- **三层一个表达式**：`path()` 先算出 scope 目录（城根、楼根、房间目录），再一律 `.join(RESERVED_PREFIX).join(CONFIG_FILE)`。只有 City 层在保留区里、另两层不在，就是一个 Agent 写域够得着自己配置的洞口。
- **旧城必须报错，不得静默降级**：`policy::load` 把「规则不存在」当作默认策略（`confidential = false`）。于是一座旧城的楼会从「机密」静默变成「不机密」——所以 `RULES.toml` 缺失而一份**本版不再读的文件**仍在盘上时**拒绝**，`E_CONFIG_INVALID`，recovery 直接拿出要执行的那一步。这不是兼容适配层（它不读旧文件），是一道不让静默降级发生的门。它问的是一个问题而不是每次搬家加一道守卫：`superseded` 同时看楼根与保留子树下的旧名，因为两次搬家（改位置、改格式）的后果完全相同。
- **楼页仍然看得见规则**：`assembly` 组楼页答案时跳过一切点头目录（房间枚举因此也不会把 `.sprawling` 当房间），故 `RULES.toml` **按路径显式读一次**再入档。人在界面上看得到、改得了；楼里的 agent 读得到、写不了。
- **默认写域仍是整栋楼，这是故意的**：一次 Run 为它那栋楼产出一份楼级产物是正常的；把默认收紧到房间会把那件事一并禁掉，而洞口在于治理文件的位置，不在于写域的宽窄。

## 8.5 两个设计

**A（选中）：`Identity` 两态枚举**。调用方拿到的东西自己会说自己是谁，段字节由它给出。
**B（落选）：`Option<Resident>`**。少一个类型，但每个调用方都要自己决定「None 时该给 prefix 什么」——那是一条散落在每个调用点的策略，且第一个忘记写的人会得到一个空的 resident 段。翻案条件：出现第三种身份（例如代表某人的临时身份），届时枚举照常扩，Option 则无法扩。

**安装的决定与动作（`library::install`）**

**A（选中）：`plan_install` → `PlannedInstall::apply` 拆两步，`install` 是两者的合成。** 全部拒绝发生在决定里，动作只能从决定里拿到（`storage::worktree` 的 `plan_merge` 同形）。TOCTOU 复查问的正是「决定之后、落位之前，世界动了没有」——这道缝不打开，复查就无处可查；批准面也在这道缝上：人按 `PlannedInstall::hash()` 批准一个具体哈希再让它落地。
**B（落选）：单一同步 `install` 四步全吞。** 入口最窄，但「安装中途目录被换」在单扇门内无法被测试模拟，除非往产品代码塞一个测试钩子；而钩子进产品代码正是本仓库明拒的东西。翻案条件：落位改由携带代际计数的文件系统原语完成（复查不再需要人为制造的间隙），届时合成回单扇门。

**另两个设计（配置文件名）**

**A（选中）：三层同名 `CONFIG.toml`，层级由位置决定**。读者记一个名字；把一份配置放错层是一个**位置**错误，而位置在目录树里看得见。
**B（落选）：每层各起一名**（`CITY.toml`／`BUILDING.toml`／`RESIDENT.toml`）。文件名自带层级信息，代价是三个名字要同时被记住，且放错层变成一个**拼写**错误——拼写错误要靠逐字比对才看得出。翻案条件：出现同一目录下共存两级配置的需求（例如一栋楼的默认与它自己作为房间的默认同处），届时位置不再能区分层级。

## 9 工作流程

装配层派活（`accounting::worker::workbench::standing`；唤醒时 `accounting::worker::waking`）→ `Identity::load(city_root, addr)` → `segment_bytes()` 进 `FrozenPrefix` 的 resident 槽 → `who()` 成为 Ledger 的 actor。

## 10 实现逻辑

纯 std：一次 `fs::read` 与一次 `B3Hash::digest`。`urbanite_path` 按地址分段 push，故不做字符串拼接，Windows 上也无分隔符问题。

## 11 边界枚举

无文件（→Ephemeral）｜文件存在但无读权限（→报错）｜空文件（→Resident，段为空字节；空描述是作者的选择，不是缺陷）｜地址含多段（`lab/room1`，逐段 push）。

## 12 Decisions

`E_STORAGE_FATAL`（读不动一个存在的描述）：不可定义掉——文件系统权限是外部世界的事实，而静默降级是被明拒的替代。

`E_INVALID_ARGS`（`[context] second_threshold` 域外）：不可定义掉——值是人写的输入，类型把「构造后非法」定义掉了，「构造时非法」必须留码；钳位是被明拒的替代。
### 12.1 定规：一个 run 不改写审判它自己的规则

`Verdict: user-approved`（kernel-SPEC §12.1 那条定规的城侧应用；六类升级的固定答案表见 kernel-SPEC §8-27）

**决定**：`rules`／`city` 两件工具保留 `Effect::Govern`，而这个效果在效果层恒拒：run 提不出提案、等不到审批、写不了规则。规则要变只有人改文件这一条路——`RULES.toml` 的唯一写者是人；建楼走线上命令 `CreateBuilding`，收编走 CLI 的 `sprawling adopt`。

**理由**：规则是审判一个 run 的尺子，而提案-审批形把改尺子的手留给被审判者、把「批准」放进一个 agent 循环——默认答案会被点过去的门等于没有门（kernel-SPEC §12.1 同一理由，此处第二次适用而不是第二个权威）。规则的正确位置是文件本身：diff、历史与回退都在版本库里，而一份获批的提案正文只在一次对话里活过一回。

**被否**：govern 存在形——提案正文由 `kernel::gate::govern` 截一段写进 `action_desc` 供人过目，门问人、批后落盘。它与 `GateOutcome::Escalate` 在 kernel 侧同集删净（kernel-SPEC §12.1 的同集删净名单列着 `gate::govern`）；`rules_tool` 的 `op=propose` 只保留「整份文档、先求值后落盘」这个形状（§8-2b），通向它的判定是拒而不是问。

**重开参数**：`attach` 是唯一会问人的门，理由是人的动作本身就是答案、没有可以点过去的默认（kernel-SPEC §8-27）。治理审批只有取得同样的性质——人在 run 之外对整份 diff 作答，且不存在「全批」的默认——才需要重新论证这一条；参数不动，定规不动。

### 12.2 定规：读界在调用时按目标所在楼现读规则

**决定**：读界的规则不在派活时冻结。`may_read` 的第三个参数是一个闭包，只在目标出了读者本楼时调用，调用时读目标所在楼的 `RULES.toml`；一次 `search` 不带路径时，城根下每走进一栋他楼调一次。

**理由**：两条轴都站在这一边。延迟：派活时冻结一张全城表要为每一栋楼读一次规则，代价随楼数线性增长，而绝大多数 run 从不读他楼；现读把代价放到真的跨楼的那一次读上，本楼的读零次读盘。安全：派活之后才建起来的机密楼、派活之后才改成 `confidential = true` 的楼，现读立刻关上；冻结表在 run 冻结之前一直按旧答案放行，而那正是朝宽松一侧失败。

**被否**：派活时冻结一张「哪些楼机密」的表，与写域、工具表一同冻结。它让 `may_read` 成为只吃值的纯函数，代价是上面两条；若冻结的是「哪些楼开放」，新楼会被关上，但派活代价不变。

**重开参数**：一次跨楼读的规则读取（一个小文件加一次 TOML 求值）相对该次 `read` 本身的读盘不再可忽略时——例如城的规则搬进一张随账本折叠的内存表，读规则不再触盘——冻结与现读的延迟差消失，可以重议；安全那一条不随它消失，重议时须给出新楼与改规则两种情形下仍然关上的办法。

### 12.3 定规：一个 run 内按 (mtime, len) 留住他楼的规则

**决定**：读界的闭包持有一份 `RulesCache`，一个 run 一份。同一栋他楼的规则，文件的 mtime 与长度都没变时不再读盘求值；任一变了就重读。

**理由**：一次不带路径的 `search` 在城根下走进每一栋他楼，一个 run 反复读同一栋楼，每次都读一个小文件再做一次 TOML 求值；stat 一次的代价低于读加求值。§12.2 的安全一条仍然成立：改规则就改了文件，mtime 随之前进，下一次 stat 就重读；把 `confidential = false` 改成 `true` 连长度也变了，所以即便 mtime 的粒度粗到同一刻内写两次，这一次翻转也逃不过长度。

**被否**：按内容哈希作键——要先读完整个文件，省下的只剩求值；run 之间共享一份——run 的寿命是缓存失效的天然边界，跨 run 的表要自己回答何时丢，而多出来的只有第一次读。

**重开参数**：若某个文件系统的 mtime 粒度粗到同一刻内能写出等长而意义不同的规则并且真有人这样改，键要加上内容哈希或 inode 变更计数。

### 12.4 没有生产调用者的公开面不留

**决定**：本 crate 的每个公开函数都有一个生产调用者。所以没有单写强度的门（派活与会话都经 `write_session` 写强度），也没有搬家的判定（全仓没有一个动作会搬家）。

**理由**：一个只有测试在调的公开函数，是一条没有人在生产里执行的规则：它的测试证明的是一段不会跑的代码，而它的签名让读者以为那扇门是开的。单写强度的门还会让「强度写在哪一张表里」有第二个写者，与 `write_session` 各自拼 `[model] effort`。

**被否**：为将来的功能先立判定——搬家真的上线时，判定连同它的四条规矩（不是改名、历史留在原址、不落楼根、不碰保留子树）从 git 取回，那时它有第一个调用者，也就有第一个能判它对不对的测试。

**重开参数**：出现一个会搬家的动作，或一个没有会话记录却要写强度的生产调用点。

### 12.5 定规：项目文件夹里的工作文档与对话记录恒不进 git

**决定**：城在项目文件夹里写下的工作文档与对话记录——计划、备忘、交接、任务单、居民身份、归档的偏好与决定、房间里的一切、城根保留子树里的账本与对象库——在城写过的每一个 git 边界上都被忽略：城根一块（§8-21 `place_city`），每栋楼一块（`place`），每个城建的房间一张封条（`seal_room`）。进历史的只有一栋楼的承诺：`SPEC.md` 与它保留子树里的五份治理文件。

**理由**：这些文件写的是人与居民之间说过的话和人的偏好，是隐私；项目的历史会被推送、克隆、分享，一旦进去就收不回来。城的检查点提交（storage-SPEC 8-8）经 libgit2 的 `add_all` 与 `is_path_ignored`，同样遵守这些规则，所以忽略规则同时是人的提交与城自己的检查点的边界。

**为什么不合成一份文件**：git 只读一个仓库工作树之内的 `.gitignore`，一个嵌套的仓库不读它父目录的那一份（在一个父仓库里写 `.sprawling/`、在子仓库里建 `.sprawling/ledger`，`git -C 子仓库 status` 照样列出它）；而一份文件里的不带斜杠的规则对它所在目录之下的任意深度生效，写在城根的 `Roadmap.md` 会连带忽略项目自己根上的同名文件。工作区是装着几个项目的父目录时，每个项目是它自己的仓库，规则只能写在项目自己的根上。所以规则只有一张表（`gitignore` 模块），写进每一个它必须生效的位置。

**重开参数**：git 若允许一个仓库读取其外的忽略文件，或城改为只在城自己的仓库里工作，这一条的落点可以收成一处。

### 12.6 定规：`[resident] harness` 上梯子，会话记录压过它

**决定**：`[resident] harness` 按城／楼／房间的梯子取最近一级（`settled_harness`），`[model] name` 仍只是地址自己那一层的会话记录。同一层两键并存在解析时拒。地址自己那一层有会话记录时，`settled_harness` 答 `None`，梯子上的 harness 从 `/new` 开的下一段会话起生效。`change` 落盘前用 `ConfigLayer::parse` 读一遍自己要写下的字节，读不回的不写。

**理由**：房间在派活时才开，人事先只能把 harness 写在楼层或城层，所以它必须上梯子；`[model] name` 是城在会话第一次 run 时写下的记录，别的房间继承它就把一段会话的选择变成了别人的默认。会话记录压过梯子，一段会话就只有一个居民，provider 对这段会话缓存的前缀也不会在中途失效（sprawling-SPEC §8-79）。写路径复读一次，是因为地址就是楼时楼层与房间层是同一个文件：派活路径若在别处漏了分流，把 `[model] name` 写进点名了 harness 的楼，每个读者都会拒这份文件，整栋楼派不出活；复读让这件事停在写之前，而判定仍只有 `parse` 一处。

**被否**：①梯子上的 harness 压过会话记录：改了楼层下一次派活就换居民，一段会话前后两截的记录说的是两种居民；②把两个键收成一个枚举字段：一个是城写下的记录、只读本层，一个是人写下的设定、爬梯子，合成一个值会暗示两者按同一条规则求值，而私有字段加唯一构造点已经让两键并存的值构造不出来；③写路径只靠派活路径先分流、自己不复读：分流住在另一个 crate，一处疏漏的代价是一整栋楼。

**重开参数**：harness 的会话也要冻下一条房间层的记录时（例如一段 harness 会话要跨 run 续上），房间层的记录要能说 harness，同层互斥与记录优先要一起重议。

### 12.8 定规：拒词的恢复语归写拒词的那一处

**(a) 梯子只加文件，不改恢复语。**

**决定**：`ladder::stated` 包一个解析拒词时只在 subject 前加上文件路径；码、action、nearby 与恢复语照 `ConfigLayer::parse` 交出的原样保留。

**理由**：派活、设置页与 `sprawling check` 都经梯子读配置，而恢复语是写拒词的那一处按场合写的：一层同时写了 `[model] name` 与 `[resident] harness` 时，`two_residents` 说「`/new` 忘掉会话写下的模型，或删掉 harness 那一行」，这正是人要做的那一步。梯子把它换成通用的「改掉消息点名的值，或删掉那个键」，派活路径上的人就读不到 `/new` 这条出口，而 `sprawling check` 与写路径读到的却是完整的拒词：同一份文件的同一个错，两条路给出两句话。

**被否**：梯子自己按错误种类挑恢复语——那是恢复语的第二个家，拒词加一种它就要跟一种。

**重开参数**：梯子开始读 `ConfigLayer::parse` 以外的来源（例如人层 `~/.sprawling/config.toml`），而那个来源的拒词不带恢复语时。

**(b) harness 的墙钟上限是楼规的一键，缺省 60 分钟。**

**决定**：`RULES.toml` 的 `harness_minutes` 给一栋楼里每次 harness run 的墙钟上限，缺省 `HARNESS_MINUTES_DEFAULT = 60`，`0` 在解析时拒。

**理由**：上限回答的是「这栋楼肯让一个外来居民占一条车道多久」，与 `review`、`confidential` 同是楼对它的居民立的规矩，而楼规由人写、run 改不了（§12.1）。放在 `CONFIG.toml` 的 `[resident]` 表里，它会爬城／楼／房间的梯子，一间房自己的那一层就能把上限写大，而那一层是会话写记录的地方。缺省给一个值而不是「不限」：不限时一个卡住的 harness 占着车道直到人发现，60 分钟够一次大的改动做完。

**被否**：①`[resident] minutes` 与 `harness` 并列：见上，房间一层能改楼的上限；②缺省不限、只靠停摆：卡住的 harness 要人来发现。

**重开参数**：车道数（sprawling-SPEC §8-46-3）变得不再稀缺，或 harness 能在回合中间报告进度、城能分辨「在做事」与「卡住了」时。

### 12.9 城外书架上的持有带着扫描读到的正文

**决定**：`Holding` 多一个字段 `carried: Option<String>`，城外书架上的一件是 `Some(SKILL.md 正文)`，城内书架上的是 `None`；只由 `Holding::of` 按 `Shelf::address()` 派生。

**理由**：阅览室要把城外的一件交给 run，而 run 打不开城外的路径；正文是唯一能交的东西（runtime D9）。扫描已经读了这些字节去取披露行与哈希，留下它们就保证交出去的字节与 `hash` 是同一次读入——在阅览室再读一次文件，两次读之间文件可能被它的主人改掉，pin 说的就不再是 run 读到的那份。

**被否**：①把正文放进 `Shelf::External` 臂——`Shelf` 答的是「在哪里」，而且 `accounting::views::skills` 按字段解构这一臂、把它映射到线上，正文会跟着进页面的那一侧；②阅览室准入时再读一次并比哈希——多一次读盘，比不上时还要另造一个拒词；③每件持有都带正文——城内的一件由 `read` 到地址去开，正文是第二个家，也让一千件的城库多占一千份内存。

**重开参数**：城外书架上的包里附属文件也要可读（runtime D9 的重开参数）时，`carried` 换成整包的字节或一份城内镜像的地址。

### 12.10 写入限制随派活走，不进 `RULES.toml`

**决定**：「只读可新建」是一次派活的选择（`RunPolicy.write`），楼的 `RULES.toml` 不加对应的键；楼的写域是一次 run 的上限，派活只能收窄它（§8-32）。

**理由**：今天要「只新建」的是人对某一次活的谨慎——让一个 agent 起草新文件而不碰已有的——不是一栋楼的性质。若楼与派活各有一个限制，就要再定一条「两者取更严」的规则，并让页面、状态工具与回放都说清楚一次 run 的限制来自哪一处；只有一个来源时，账上的 `run_started.policy` 就是全部答案。

**被否**：`RULES.toml` 加 `limit = "full" | "create"`，与派活取更严的一个：一个事实有两个来源，而今天没有一栋楼需要它。

**重开参数**：出现一栋每次 run 都必须只新建的楼（例如只收稿件的投稿楼），且人要求不依赖每次派活都选对时。

### 12.11 身份住在两份治理文档的身份区里，一个 session 在房间那一层冻一版

**决定**：用户 ID 与导入来源写进 `PREFERENCES.md` 开头以 `+++` 围起的 TOML 身份区，主 Agent 的名字写进 `MAYOR.md` 的同类身份区；正文分别是「关于你」与这位主 Agent 是谁（§8-33）。一个 session 的第一次 run 把此刻的身份冻进内容库，摘要记在房间自己那一层 `CONFIG.toml` 的 `[identity] version`。

**理由**：人已经在这两份文件里写自己的背景与主 Agent 的样子，名字放在同一份文件里，原文编辑器与设置卡片改的就是同一份东西，没有第二份可编辑的配置要同步。`+++` 是 TOML 的惯用围栏，与 Markdown 正文分得开，一个只想改正文的人不会误碰它。冻在房间那一层，是因为一个 session 的形状（模型与强度）已经记在那里，`/new` 清掉的也正是这一层；身份跟着同一个边界走，就不必再定一条「哪一次清身份」的规则。

**被否**：①另起一份 `IDENTITY.toml`：同一个人写的同一件事分在两个文件里，原文编辑器里看到的正文与卡片上的名字各说各的；②身份区用 YAML（`---`）：本 crate 只依赖 TOML 的解析器，而 YAML 的隐式类型会把 `no`、`on` 这样的名字读成布尔值；③session 开始时把身份写进 `session_opened`：经 `Dispatch` 打开的房间没有那一行，而第一次 run 是每个 session 都有的那一刻。

**重开参数**：身份要跨城复用（人明确导入另一座城的身份）时，重议身份是否搬到人自己那一层（`~/.sprawling/config.toml`）。

### 12.13 治理工具的 scope 用账本上的 scope 文字

**决定**：`rules` 与 `city` 的 `subject` 答 `GateSubject::Scope`，字符串是 `kernel::event::Scope` 的 `Display`：`building:<地址>` 或 `city`（§8-36）。

**理由**：scope 在这座城里已经有一种文字，`rules_changed` 与 `city_halted` 都按它写进账本，`Scope::parse` 读回它。拒词里的 scope 用同一种文字，模型与人读到的是账本上会出现的那个词；`city` 与一栋叫 `city` 的楼在这种写法里也分得开。补上 `subject` 之前，两件工具答 trait 默认的 `GateSubject::None`，效果层按「主体与效果不一致」拒，恢复语让模型去报告工具缺陷，而工具并没有缺陷。

**被否**：①只写楼的地址（`lab`）：`city` 要另造一个词，而这个词可能就是某栋楼的名字；②`city` 的 `raise`／`adopt` 答被点名的楼：那栋楼还不存在，改的是城的形状，治理它的不是它自己的规则；③`subject` 不读参数、只答 scope：一条读不懂的调用得到的是「不得改写规则」而不是「这个动作不存在」，模型下一步改不对。

**重开参数**：`GateSubject::Scope` 改成携带类型化的 `kernel::event::Scope`（kernel 一侧的改形），这时两件工具交值而不交文字。

### 12.16 `[remote]` 只在城那一层，值照写下的读，判在开门时

**决定**：`[remote]` 是城那一层独有的表，`route` 选臂；隧道名、地址与程序以字符串交给装配层，在每次 `/remote open` 时由 `remote_access` 的类型判（§8-39）。

**理由**：远程门开在整座城上，选通路的只能是管这座城的那份文件，下层写它与下层挂外部书架是同一种越权，所以两张表共用一条判定与一个拒法，而不是各写一份。本 crate 在拓扑上只依赖 `kernel`；隧道名与 `https://` 地址的规则已在 `remote_access` 有唯一的家，在这里再写一遍，就是同一条规则两个权威，两份迟早说法不一。

**被否**：①`[remote]` 上三层梯子：一栋楼能替全城开一条外面进来的路；②在本 crate 复写 `TunnelName`、`PublicUrl` 的判定，让写错的值在解析点就被拒：两个权威；③`permanence` 缺省为 `fixed`：见 §8-39，错的缺省比没有缺省更难诊断。

**重开参数**：本 crate 被允许依赖 `remote_access`（拓扑改动）时，解析点直接造出类型化的值，写错的地址在读文件时就被拒。

### 12.17 读-判-换的门把此刻的字节交给判定，判定留在调用方

**决定**：`city::document` 多一扇门 `revise`：在一份文档的锁里读出它此刻的全部字节，交给调用方的判定，判定经 `Held::replace` 整份换上（§8-40）。`edit_against` 改由它实现。本 crate 不依赖 `documents`。

**理由**：页面的保存与修改提案的决定都是「读此刻、判、换上」，判定是 `documents` 的，锁与整份换上是本模块的，两件事各有一个家。`edit` 已经开放了锁与 `Held`，但让调用方自己读文件，就会出现第二处「读不到就是空」的读法；把读放进门里，`edit_against` 与保存读的是同一次读。

**被否**：①`city` 依赖 `documents`、门面上给一个 `save_document(path, baseline, edits)`：多一条 crate 边，而本 crate 不需要知道编辑长什么样；②`accounting` 在 `edit` 里自己读文件：理由见上；③另开一把锁：一份文档两把锁，`PutSpine` 与 `PutRange` 写同一份 `Roadmap.md` 时互相看不见。

**重开参数**：出现一个要在锁里读别的东西（例如同目录的另一份文件）的写者时，重议门交出的是字节还是 `Held` 加一个读的方法。

### 12.18 模板与 `City.md` 住进 city 的包目录，不留在 `docs/`

**决定**：城写下的模板与 `City.md` 住在 `crates/city/templates/`（§8-41）；`City.md` 由 `city::CITY_TEMPLATE` 交给 accounting。

**理由**：模板的字节是 city 的产品：`building::create`、`spine_files::lay_out`、`write_job` 与创世都按它们写盘，编译期 `include_str!` 它们。一个包要发布，它编译时读的每个文件都得在它自己的目录里（sprawling-SPEC 8-157），而 `docs/` 这样的仓库目录不属于任何包。放进 city 而不是 accounting，因为这些文件的名字、读法与「谁写哪一份」本来就在本模块。

**被否**：①模板放在 `docs/` 下给人读，发布前由脚本复制进包——仓库里的包与发布出去的包不再是同一组文件，复制那一步要自己的检查；②模板放在 `docs/` 下，city 里放一个指向它的符号链接——Windows 上建符号链接要开发者模式或管理员权限，git 在那里默认把链接检出成一个写着路径的普通文件；③`City.md` 搬进 accounting——它的文件名与同族模板都在 city，搬过去就把一族文件拆进两个包。

**重开参数**：模板要给 city 之外的读者在运行期按文件读（而不只是编译进二进制），那时它们的位置成为一个运行期的事实，要另议。

## 13 依赖选型

workspace 内只依赖 `kernel`（拓扑硬约束）。dev 依赖 `tempfile`。外部依赖如下，均在 workspace 钉版（不新增版本权威）。

`toml` 与 `serde`（derive）。理由：三层配置的格式是 TOML，而 `toml` 已被 `xtask` 消费（budgets.toml／lexicon.toml）；解析走 serde derive 加 `deny_unknown_fields`，使「写错的键」在反序列化那一刻失败。手写一个 TOML 子集解析器是可行的另一条路，已落选：它会把一个已有权威的格式变成本库自己的私有变体。

`cap-std` 与 `cap-fs-ext`（workspace 钉版）：技能包预检要按打开的目录句柄相对地列举与打开（§8-28），标准库只按路径打开。它们是安全 Rust 里的那一层 `openat`；其下的 `cap-primitives` 已在锁文件里（`wasmtime-wasi` 之下），许可相同。落选的另一条路是只收城自己控制的暂存区里的来源：它把「来源是谁的目录」变成一条安装方要遵守的约定，而不是由读法成立。

## 14 硬编码声明

城里每一份文件叫什么、落在哪，权威是 `kernel::layout`：`ARCHIVE_DIR`、`BUILDING_SHELF`、`CONFIG_FILE`、`LIBRARY_DIR`、`URBANITE_FILE` 由本 crate `pub use` 转出而不复述，路径由 `CityLayout` 的十一个方法给出而不逐段 push。`spine_files` 的 `HANDOFF_FILE` 同样只是 `kernel::layout` 那一份的别名；房间的 `JOB.md` 只经 `job_path` 取，文件名留在 `kernel::layout::JOB_FILE` 一处。本 crate 自己定义的文件名只剩没进布局表的那几份：`RULES_FILE`、`DESKTOP_SCOPE_FILE`、`PREFERENCES_FILE`、`SCHEDULE_FILE`、`WATCH_FILE`、`GITIGNORE_FILE` 与 spine 剩下的那一组。

Ephemeral 段文本（私有常量，改它即改一个 Ephemeral 读到的第一句话）。

`PACKAGE_BYTES_LIMIT`（32 MiB，`library::install::precheck::walk`）：一件技能包文件字节的上限，理由见 §8-28。它是产品的界，不是按某一类机器调出来的数：包的字节要在内存里握两份，而技能的本分是文本和小脚本。

`CONFIG_FILE`（三层同名，理由见 §8.5）；新楼的 `RULES.toml` 字节不写在代码里，而是 `include_str!("../../templates/RULES.toml")`——它的权威是那份模板，路径写错在编译期就会被堵住；`Confidential` 模板对该字串做一处行替换（`confidential = false` → `true`），替换是否真的生效由 `policy::evaluate` 读回来断言。

## 15 影响面

改身份、规则、配置梯子或书架的公开面，波及 `crates/sprawling` 的装配层（派活、建楼、开房间、写会话）与视图；改 `crates/city/templates/` 下被 `include_str!` 的模板即改新楼与新城的第一批字节；加一件工具即 `ChatRequest.tools` 每回合多一条 disclosure 与一份 schema。

## 16 测试与约束

三条：身份两态各一条；**段字节跨两次加载稳定**；Dossier 只计本人的 Run 且跨 Run 累加。bin 侧另有一条端到端断言（两次 Dispatch 的 resident 段哈希相同、run 段不同）。

建楼与配置九条：新建的楼被 `policy::load` 读回且 confidential 模板真的锁本地模型池｜二次出生恒拒｜reserved prefix 下建楼恒拒｜房间地址建楼恒拒且拒词指出该建哪栋｜下层配置盖上层｜**同一层写模型又写 harness 即拒，城自己的写路径也写不出这样的文件；房间有会话记录时梯子上的 harness 不生效，`/new` 之后生效**｜不认的键即拒｜**写在 `CONFIG.toml` 里的 effort 出现在真实出线请求体里**（bin 侧端到端，假 provider 录下请求体）｜**`usersbrowser` 的三种值各读回各的形状，`browser` 与它互不影响，confidential 楼写它即拒**。

技能安装六条（`library::install::tests`，CAS 绑定一例在 `crates/sprawling/tests/skill_install.rs`）：装上架的字节与来源一致且扫描读回的 `Holding` 整体相等｜同哈希重装幂等且盘上字节不变｜**plan 与 apply 之间来源目录被换即整体拒收**（盘上无文件、登记零调用）｜经符号链接的包（包目录本身或 `SKILL.md`）恒拒｜同格异哈希占名拒｜跨 section 同名拒。

读界两条：`kernel::address::tests` 的三类读者矩阵（本楼读本楼、他楼读非机密楼、楼外读机密楼，外加机密楼读自己、规则读不出）逐格判出 `ReadVerdict`，且本楼的读从不调用规则闭包｜bin 侧 `a_run_in_another_building_reads_nothing_of_a_confidential_one`：普通楼里的 run 按路径 `read`、再不带路径 `search` 机密楼里的文件，假 provider 录下的每一份请求体里都没有那份文件的字节，且最后一份带着 `E_GATE_DENIED`。

邻里名册六条：扫到的名册**不含我自己**且有人的与空的各自落在对的臂上｜`## Bring them` 在场时取它、缺席时退回第一段正文且跳过标题与引文｜同一座城扫两次字节相同（`read_dir` 序不得泄漏到答案里）｜`scope=city` 只交出楼名、不交出任何住户｜`.sprawling` 与 archive 目录都不是房间｜**模板仍然带着 `## Bring them` 这一节**（对 `crates/city/templates/URBANITE.md` 的 `include_str!` 断言；模板改名而代码不改，就是一份永远退回正文的名册）。

## 17 模型体验

三层配置入窗零字节：`CONFIG.toml` 改的是请求字段（effort），不是模型读到的文本；新楼的 `RULES.toml` 经 `city::policy` 进判定面，其字节另经 building 段整份入窗——判定与阅读读的是同一串字节。

resident 段是模型每回合都读到的四段之一。`URBANITE.md` 建议 30 行以内：长的描述不会让 Resident 更能干，只会让每个回合更贵——这句话写在模板里，因为模板在场即教学。

邻里名册的常驻代价是一件工具的 disclosure 与 schema，名册本身**不进 prefix**：一栋楼的住户数会长，而每回合都付的字节不该随人口增长。模型读到的常驻新增只有 `status` 的一行 `neighbours: N`——它回答的是「值不值得问」，问出来的详情由工具在需要时交付，与 catalog 对 skill 用的是同一条渐进披露。

## 18 文档同步

`ARCHITECTURE.md` 模块表的 city 各行｜`docs/glossary.md` 的 Resident、Neighbourhood 等词条｜`crates/city/templates/` 下被实例化的模板与本文 §8-3、§8-5 同期改。

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
- `write = "documents"` 由 `policy::evaluate` 读成 `DomainReach`；**缺这一键即拒**，与 `confidential` 同——读作 `Everything` 会让没见过这个设置的人得到最宽的那一档。值既不是 `everything` 也不是 `documents` 时同样拒绝：读成打字错误的权限设置不能落到宽松那一侧。
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
/// 把城自己的保留子树补进城根的 .gitignore，同一条只追加的路。
/// 门面上叫 `city::ignore_city_records`：裸的 `place_city` 在装配层里说不出放下的是什么。
pub fn place_city(city_root: &Path) -> Result<(), AxError>;
/// 城根一块，加上每栋楼一块，缺哪行补哪行。开城时调；门面上叫 `city::keep_records_out_of_git`。
pub fn place_everywhere(city_root: &Path) -> Result<(), AxError>;

// city::spine_files
pub const SPEC_FILE: &str = "SPEC.md";   // 字节来自 crates/city/templates/SPEC.md（include_str!）
```

- **不对称本身就是这一条的全部内容**：`SPEC.md` 与这栋楼自己保留子树里的五份承诺进版本库；工作文档与对话记录一份都不进（§12.5 定规）——`Roadmap.md`、`Memo.md`、`Handoff.md`、`JOB.md`、`URBANITE.md`、`Archive/` 与各个房间。一栋楼向外承诺的东西必须在历史里，任何一次克隆都读得到；一次会话当时在想什么不是承诺，它留在运行中的机器上。
- **`SPEC.md` 是十七节 crate SPEC 的压缩式，不是第二种形状**：`crates/city/templates/SPEC.md` 的十二节逐节对应 crate SPEC 的节次（需求／验收／假设／权威／命名／边界／接口／错误／依赖／硬编码／测试／决策），只是把「现状分析、工作流程、实现逻辑、影响面、模型体验、文档同步」这几节留给 crate 自己。它压缩，不另起。
- **`place` 只追加，从不重写**：被收编的目录往往已经有一份 `.gitignore`，里面写着这个项目自己的东西。整份覆盖会把它们冲掉，而那正是 adopt 承诺不会碰的字节。依据是逐行比对（去空白后相等即视为已有），因此重复 raise 不会把同一段追加两次。
- **显式的反忽略**：`!SPEC.md` 与保留子树的放行行写进块里，而不是靠「没人忽略它们」这个默认。被收编的仓库可能已经忽略了 `*.md` 或一切点开头的目录；那时「这栋楼的承诺在历史里」就是假的，而没有人会发现。
- **保留子树逐文件放行，不整棵放行**：块里先 `.sprawling/` 忽略任意深度的保留子树，再 `!/.sprawling/` 只把这栋楼自己的那一棵放回来，`/.sprawling/*` 把它清空，最后逐行放行五份承诺——`RULES.toml`、`CONFIG.toml`、`FILTERS.toml`、`DESKTOP.toml` 与 `skills/`。三个理由：①城自己的保留子树同名，账本、对象库与金库引用住在那里，一行 `!.sprawling/` 把它们一并放回版本控制的可见面；②那一行不带斜杠，因此对楼下每一个居民、每一个房间的保留子树同样生效，而那些是机器上的东西，不是这栋楼的承诺；③`CONFIG.toml` 正是 MCP 凭据的落点，它进历史的前提是 §8-4b 的逐值判定同时成立——两件事是同一次改动。
- **五个名字都从写它的模块取**：`kernel::layout` 的 `CONFIG_FILE`／`FILTERS_FILE`／`BUILDING_SHELF`、`policy` 的 `RULES_FILE`／`DESKTOP_SCOPE_FILE`、`spine_files` 的四份脊柱文档名。因此 `BLOCK` 由 `&[&str]` 常量改为 `block() -> Vec<String>`：一个改了名的文档不会在这里留下一条谁都不写的规则。
- **顺序就是文法**：git 认最后一条命中的规则，所以「忽略—放回目录—清空—逐行放行」这四步不能重排。这一条由 git 自己验过：外层再写 `.*` 与 `*.md`，`SPEC.md` 与 `.sprawling/CONFIG.toml` 仍然进历史，`.sprawling/ledger/` 仍然不进，一个嵌套目录自己的 `.sprawling/CONFIG.toml` 也不进。
- **城根也有一块，只有一行 `/.sprawling/`**：城根的保留子树装着账本、对象库、工作树与金库引用，全是城运行时留下的记录，不是项目的内容。人让城围着一个工作区立起来时，那个工作区常常就是一个 git 仓库（项目本身），而楼这一层的块只管楼自己的保留子树，城根的那一棵就会以几百兆的未跟踪目录出现在项目的 `git status` 里。`place_city` 在立城时（`form_city`，`init`、`up` 与「用一个已有的文件夹」三条路都经过它）把这一行补进城根的 `.gitignore`，与 `place` 同一个只追加、逐行比对的规矩；锚在根上（带前导斜杠），因此楼与房间自己的保留子树仍由楼那一块逐文件放行。城根的 `City.md` 与 `hall/` 不在这一行里：前者是人要改的城规，后者是一栋楼，它自己的块已经说了它哪些进历史。
- **工作文档按名忽略，不靠房间封条**：块里 `JOB.md`、`URBANITE.md`、`Handoff.md`、`Roadmap.md`、`Memo.md` 与一次 run 的对话记录（`runtime::transcript` 写在房间里的 `<run>.jsonl`，按 `kernel::layout::RUN_ID_PATTERN` 与 `TRANSCRIPT_EXT` 拼成 `????????-????-????-????-????????????.jsonl`）不带斜杠，在楼下任意深度都不进历史；`/Archive/` 锚在楼根，那里存的是人说过的偏好、做过的决定与纠正。按名忽略是必要的，因为房间封条只盖在城自己建的目录上：人把活派到一个项目里本来就有的子目录（`proj/src`），城不能往那里放一个 `*`——那会让这个源码目录里此后的每一个新文件都悄悄进不了历史。
- **城替派活新建的房间都封上**：派活开房间的那一步（sprawling-SPEC §8-40 的 `room_for`，城为一次派活写下的第一件东西）遇到一个没经过 `open` 的房间地址时调 `city::claim_room`：房间还不存在就建出它并封上，与 `room::open` 同一个 `seal_room`。放在这一步而不是写 `JOB.md` 时，是因为会话冻下的形状（`write_session`）在任务单之前就写进房间自己的 `.sprawling/CONFIG.toml`，那一写会先把目录建出来。一个直接派到 `building/room` 地址、没经过 `open` 的派活因此不再留下一个没封的房间。已存在的目录不封（理由同上一条），它里面的城文件由按名的规则挡住。
- **每次开城补一遍**：`place_everywhere` 在城的写者打开时（sprawling `RunWorker::holding`，serve、resume 与立城都经过它）对城根与 `building::all` 列出的每栋楼各补一遍缺的行。规则表是会长的：它长出一行时，早先立起的楼要在下一次开城就拿到这一行，而不是只有新立的楼才有——§12.5 的定规对一座已经在跑的城同样成立。只追加、逐行比对，所以一栋规则齐全的楼一个字节都不会被写。一份写不进去的 `.gitignore` 让开城以 `E_STORAGE_FATAL` 拒绝：城说不出它的对话记录会不会进 git 时，不接活。
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

- **`Effect::Govern`，不是 `Effect::Write`**：立一栋楼是在城根下造目录，任何写域都够不到那里，理由与 `rules_tool` 同——这个决定是人的。判定与 §8-2b 同一条（此处第二次适用而不是第二个权威）：调用在效果层被拒，run 里的 `raise`／`adopt` 到不了盘。
- **三个动作一条目录行**：`list` 是 `raise` 与 `adopt` 的前提（叫什么名字、哪个目录已经在那儿），拆成第二个工具只会多一行给模型读。
- **动作不认即拒并报出已知集**：猜错这里意味着把「收编一个已有目录」执行成「新建一栋空楼」，而后者会在人的工作目录旁边多出一份不属于它的模板。
- **本工具只装给市政厅的居民**（见 8-22），但装上不改变判定——每一次调用都在效果层被拒（§8-2b）。建楼与收编恒走人的手：线上命令 `CreateBuilding`，或 CLI 的 `sprawling adopt <city> <addr>`（§8-3），无论哪个地址在问。

### 8-24 Handoff 从楼搬到房间

> 权威在 `crates/runtime/Spec.lean` §8-33；本节只记 city 这一侧怎么变。

`handoff_path(city_root, room)` 与 `handoff(city_root, room)` 的第二个参数从楼地址改为**房间地址**：`<city>/<room>/Handoff.md`。签名一字不变，变的是调用方递什么——装配层的 `run_segment` 递本跑的地址。模板由 `room::open` 在打开房间时经 `spine_files::lay_out_handoff` 铺下；楼级 `lay_out` 不再铺 `Handoff.md`。理由是同楼并发：两个房间同时冻结，一份楼级文件就是两份内容抢一个名字。没有房间的地址（直接派到楼根的跑）读到 `None`，与从前空表单的读法一致。

### 8-24b city::governed：治理这座城的三份文件（形状 4 adapter）

```rust
pub const PREFERENCES_FILE: &str = "PREFERENCES.md";
pub enum Governed { Mayor, Clerk, Preferences }
impl Governed { pub fn file(self) -> &'static str; pub fn path(self, city_root: &Path) -> PathBuf; }
pub fn write_governed(city_root: &Path, which: Governed, base: &str, body: &str) -> Result<PathBuf, AxError>;
```

三份文件都住 `<city>/.sprawling/`——没有任何写域够得到的地方（前两份也在那里）。**本模块之所以是一扇门而不是三个调用方各自拼一条路径**：能自己拼路径的调用方就能拼出一条走出保留子树的路径，那样「居民改不了治自己的东西」就成了靠习惯成立而不是靠构造成立。

- **`Preferences` 是第三份而不是第三个居民**：市长与文书各有身份文件，而「这个人怎么喜欢这座城办事」不属于任何一个居民，它属于城；它与前两者被同一条规矩治理，所以住同一处、走同一扇门。
- **整份覆写，带基线**：这是人在一个框里编辑、按一次保存的文件，写一半会让这座城被半句话治理。写者有两个——原文编辑器与设置页上的卡片（§8-33），或两个开着的页面——所以一次保存携它起手时的全文，文件已经变了就拒。旧内容不留在这里——账本上那行 `governed_document_written` 才是回头看的地方。
- **枚举而不是文件名字符串**：文件名是城的答案，不是发帧的人的答案（wire-SPEC §8-19 同一条理由，两侧各说一次）。

### 8-25 `desktop`：这栋楼把桌面交出去了吗（`policy` 内，形状 1 判定）

`RULES.toml` 多一位开关，读法与 `browser` 逐字同形：

```rust
impl BuildingRules {
    pub fn desktop(&self) -> bool;   // 缺这一行读作 false
}
```

- **缺省是关，与 `browser` 同、与 `confidential` 反**。分野是两者各自往哪一侧失手：把隐私设置读成宽松那一侧是一次事故，而少给一件工具只是少一件工具。桌面比浏览器更该守这一条——`desktop.act` 会在这个人自己的机器上按下按键，而按下去的东西没有 restoration。
- **机密楼恒不给桌面**，理由与 `browser` 那条同构而更强：一台桌面上有别的程序、别的窗口、一整块剪贴板，`desktop.screenshot` 读到的东西没有一样是这栋楼的。「数据可入不可出」与「这栋楼可以截屏运行中的机器」是同一句话的两半，不能同时为真，故在 `evaluate` 里即拒，`E_CONFIG_INVALID`，拒词指出删哪一行。
- **它开的是「准不准接」，不是「准不准做」。**准不准做归 `DESKTOP.toml`，那是 server 那一侧、按窗口逐条写的 allowlist（`crates/desktop/Spec.lean` §8-4）。两道门叠着，且**次序固定**：楼说不，连进程都不起；楼说是，仍要那份 allowlist 逐窗口点头。一道门管「这栋楼是干这个的吗」，另一道管「运行中的机器上的哪几个窗口」，把它们合成一道都会让其中一个问题没人问。

### 8-26 `DESKTOP.toml` 落在哪（`policy` 内，形状 4 adapter）

```rust
pub const DESKTOP_SCOPE_FILE: &str = "DESKTOP.toml";
pub fn desktop_scope_path(city_root: &Path, addr: &Address) -> PathBuf;
pub fn write_desktop_scope(city_root: &Path, addr: &Address, text: &str) -> Result<PathBuf, AxError>;
```

**先问 `DomainReach` 立下的那条读法。**`DomainReach` 把「residents 能写什么」变成了一份可判定的东西，而它成立的前提是：**决定这件事的那份文件，恒不由被它决定的人来写**。`DESKTOP.toml` 恰恰是这一类——它逐窗口地说这栋楼的 runs 能碰运行中的机器上的什么。故它不是产物，是治理文件。

**落点因此是 `<city>/<building>/.sprawling/DESKTOP.toml`**，与 `RULES.toml`、`CONFIG.toml` 同处，在 reserved prefix 之下——`is_reserved` 对任何含 `.sprawling` 段的地址为真，故**任何写域都够不到它**，包括 `DomainReach::Everything` 的楼。一个 agent 改不了自己被判的那把尺子，这一条在这里是由构造成立的，不是由记得成立的。

**写它的是一扇门，不是一个能拼路径的调用方**（`city::governed` §8-24b 同一条理由，此处第二次适用而不是第二个权威）：设置页递「哪一栋楼」与「整份文本」，路径由本模块算。能自己拼路径的调用方就能拼出一条走出保留子树的路径。

**整份覆写，且恒不在此校验内容**：`DESKTOP.toml` 的语法权威在 server 那一侧（`crates/desktop/src/scope.rs`），且它 fail closed——读不出来的文件关成全拒。城里再抄一份解析器就是第二个权威，而两个权威里迟早有一个会把某份文件读成另一种意思。城这一侧只保证「写进去的字节就是人给的字节」，剩下的由那台 server 在启动时读，读不动就什么都不做。

**恒不复用 `Governed`**：那三份是**城**的文件（`<city>/.sprawling/`），这一份是**楼**的。把楼级路径塞进一个按 city_root 取路径的枚举里，会让那个枚举需要一个只有部分变体用得上的参数。

**线上它走 `ConfigureBuilding` 的第四个可选字段**（wire-SPEC §8-40）：那条帧问的就是「这栋楼的 runs 按什么规矩来」，沙箱、外部服务器、运行中的机器上的窗口与第二级提醒落哪一层是同一个问题的四面。`configured_payload` 因此收一个 `Written { sandbox, mcp, desktop, context }` 而不是四个裸布尔——一个调用点写 `(true, false, true, false)` 说不出哪一位是哪一面。

### 8-27 city::document：一份文档整个换上去，或者旧的留着（形状 4 adapter）

```rust
pub(crate) fn replace(path: &Path, body: &[u8]) -> Result<(), AxError>;
pub fn edit<T>(path: &Path, act: impl FnOnce(&Held<'_>) -> Result<T, AxError>)
    -> Result<T, AxError>;                        // 门面上是 city::edit_document
pub fn edit_against(path: &Path, base: &[u8], body: &[u8]) -> Result<(), AxError>;
pub struct Held<'a> { /* 私有 */ }
impl Held<'_> { pub fn replace(&self, body: &[u8]) -> Result<(), AxError>; }
pub(crate) enum TreeEntry<'a> { Directory(&'a str), File(&'a str, &'a [u8]) }
pub(crate) fn place_tree(target: &Path, entries: &[TreeEntry<'_>]) -> Result<(), AxError>;
```

- **一棵树同样要么整棵、要么没有**：`place_tree` 把一个此刻不存在的目录整棵放上去（§8-28 的整包安装是它唯一的调用者）——每一项在目标旁一个点开头的暂存目录里写好并 `sync_all`，暂存目录里的每一层目录（嵌套的子目录和暂存根）在 unix 上各自 `sync_all`——只刷根时，换入可能先于某个子目录的目录项落盘，崩溃后读者会看到半个包——再一次 `rename` 把整个目录换到位；读者看到的是没有这个目录或完整的它。目标已在即拒（`E_STORAGE_FATAL`），判在暂存之前、判的是路径本身（一个链接也算「已在」），所以被拒时目标旁什么也没暂存：这扇门只放置，不覆盖一棵树——标准库的 `rename` 在 unix 与 Windows 上都会把一个空目录静默换掉，覆盖非空目录则不是一次操作。上次被杀的写者留下的暂存目录从未换到位、没有读者见过，先清掉再写。暂存目录里的文件不再各自经一次暂存改名：整棵树在换到位之前对谁都不可见，逐个文件的改名只多花系统调用。

**`edit` 与 `Held` 对外开放，`replace` 不**：人层 `<home>/.sprawling/config.toml` 不在任何一座城里，却与一份 `CONFIG.toml` 同性质——有人手工编辑它，有解析器把它读回来，同一条命令流写它。它要的正是本模块那两条性质，而**再写一份「要么整份要么不动」就是给 B-49 立第二个权威**，两份实现里迟早有一份漏掉 `sync_all` 或漏掉锁。开放的是读-改-写那扇门（`edit` 与它给出的 `Held`），不是整份覆写那条捷径：`replace` 留在 crate 内，因为城外唯一的调用方做的是读-改-写，而一个能整份覆写的外部调用方就能不读就写。

**城里写下的每一份文件都有人拿解析器读回来**：配置层、楼的规则、一次会话的 JOB.md。就地截断再流式写入，中间有一段时间盘上既不是旧版也不是新版；断电后那段时间不会结束，于是那间房、那栋楼乃至整座城的每一次派活都失败，直到有人手工改那份文件。两条性质把这扇窗关上，而两条都只写在本模块：

- **要么整份，要么不动**：字节先落到目标同目录的暂存文件，`sync_all` 把它交到设备上，再由一次 `rename` 把它放到位。覆盖式 `rename` 在本项目支持的每一种文件系统上是一个操作，所以读者拿到的是旧版或新版，没有第三种。
- **同一刻只有一个写者**：一份文档的读-改-写在本进程内互斥。`Held` 只能从 `edit` 里拿到，于是「改写期间锁是持有的」由类型成立，而不是由每个调用点记得成立。
- **`edit_against` 是有第二个写者那份文档的门**：四份 spine 文档（以及楼的规则），人手在编辑器里与城的命令流同时在写，而本进程的锁对编辑器毫无约束。写者有权替换的只是它起手时读到的那份正文（`base`），文件已经变了就拒而不覆写，报 `E_VERSION_CONFLICT`；`base` 为空即「这份文档本不该存在」，同一条规则读在它的起点。「文件被人动过就拒」这条规则全城只在这里判定，`put_spine` 经它写。

**暂存文件名固定为 `.<文件名>.staging`**：写者在 flush 与 rename 之间被杀会留下它，固定名让下一次写复用同一个位置，而不是攒出一目录谁也说不清归属的碎片；点前缀使城里每一处扫描都跳过它（扫描一律跳过点开头的项）。

**父目录的 `sync_all` 只在 unix 上做**：目录项住在目录里，光刷文件不够；Windows 没有可供进程打开的目录句柄，也不需要——`rename` 调到的 `MoveFileEx`（`REPLACE_EXISTING`）由文件系统自己记日志。

**锁是本进程的**：一个人在编辑器里改同一份文件不受它约束，而在本仓库现有依赖下也无法约束（跨进程文件锁需要新依赖，`crates/city/Cargo.toml` 只认 `kernel` + std + `toml`/`serde`）。挡住那个人的是原子替换：他的编辑器永远读不到半份文档。

**`create_new` 那一族不归本模块**：`spine_files::write_new`、`building::create`、`gitignore::seal_room` 要的是「独占地认领一个名字」，而 `OpenOptions::create_new` 已经把认领与拒绝合成一个操作。把它们改道本模块只会让一条已经成立的规则多一个家。

**错误面**：`E_STORAGE_FATAL`，主题是失败的那条路径与操作系统的原话，恢复语一句——把目录改成可写、确认磁盘有空间，然后重存。八个写面共用这一句。经 `edit_against` 另有一个码：`E_VERSION_CONFLICT`，含义是「文件不再是你起手时的那份」，恢复语是重读再发。它与 `runtime::tools::edit`、`library::install` 报同一件事的码相同，客户端已有它的词条，故不是新开的一种失败。

**关门条件**：断电模拟——任意时刻杀进程，`CONFIG.toml` 要么是旧版要么是新版。逼近它的是四条测试：一个读者在另一线程反复替换 512 KiB 文档时每次都读到完整的旧版或新版；被杀的写者留下的暂存文件既不是那份文档、也不挡下一次写；两个线程各二百次读-改-写之后计数是四百；一份文档把它上面的目录一并带来。

### 8-40 city::document 的第三扇门：读出此刻的字节、判、整份换上（`revise`，形状 4 adapter）

```rust
pub fn revise<T>(path: &Path, act: impl FnOnce(&Held<'_>, &[u8]) -> Result<T, AxError>)
    -> Result<T, AxError>;                        // 门面上是 city::revise_document
```

- **一次保存要的是「此刻的字节」，不是「我起手时的正文」。** `edit_against` 拿调用方给的整份正文与盘上的比较；页面对任意一份文档的保存（wire-SPEC §8-72）带的是 32 字节的版本摘要与几段编辑，判它的是 `documents::save`，它要读的是盘上此刻的全部字节。`revise` 在 `edit` 的锁里把这份文件读出来交给 `act`，`act` 判定、经 `Held::replace` 整份换上，锁在 `act` 返回之前一直持有，所以两个从同一版出发的保存只有先到的那个落下，第二个读到的已经是第一个的字节。
- **没有文件读作空字节**，与 `edit_against` 同一条规则；`edit_against` 就是 `revise` 加一次逐字节比较，「读不到就是空」这条读法只写在 `revise` 一处。别的读错（目录、无权限）是 `E_STORAGE_FATAL`，与本模块其余的写面同一句恢复语。
- **本 crate 不依赖 `documents`。** 判定由调用方交进来：`accounting::worker::commanding::saving` 交的是 `documents::save` 与 `documents::decide`（accounting-SPEC §8-22）。这扇门只拥有锁、读与整份换上——就是 §8-27 的两条性质。
- **当前状态：修改提案的提出与收回还没有写者。** 提案由 run 提出（kernel-SPEC §8-83 的 `proposal_offered`），这需要一件工作台工具：它读那一版、切出原文、经 `documents::Offer::of` 判长度，把一行 `proposal_offered` 记在这次 run 名下，收回时记 `proposal_withdrawn`。这件工具还没有落地，所以今天账本上的提案只来自测试。refrain 路线图 §4-10 说「文稿审阅期间 `edit`、`exec` 对该文稿的写入受同一边界约束，否则只在候选工作树里操作」：本轮按后一条走——提案只是账本上的一行，run 不改那份文档，接受时由人的决定经 `revise` 落下。前一条若要成立，要由 runtime 的写门判「这份文档有开着的提案」，那是 runtime 规格的事，决定它的证据是那件工具落地时一次 run 同时提案又直接改同一份文档的情形。
- 验收：`document::tests::a_revision_reads_the_bytes_on_disk_and_holds_the_lock_while_it_decides`——两个线程各从同一份文件出发做两百次读-判-换，计数是四百；没有文件时交给 `act` 的是空字节。

### 8-28 技能安装：静态预检、原子落位、内容哈希入 CAS（`library::install`，形状 2 值＋一个落盘动作）

技能包与居民自建技能进书架前的唯一入口。四步都在这一扇门里：静态预检（不执行代码、禁符号链接、名称冲突校验）、staging 原子换入、落位前 TOCTOU 复查、内容哈希登记。

```rust
// city::library::install
pub struct Slot { /* shelf、section —— 私有 */ }
impl Slot {
    pub fn library(section: &str) -> Result<Slot, AxError>;
    pub fn building(building: &Address, section: &str) -> Result<Slot, AxError>;
}
pub enum Placed { Fresh, AlreadyShelved }                  // 穷尽两态
pub struct Installed { pub holding: Holding, pub hash: B3Hash, pub placed: Placed }
pub struct PlannedInstall { /* 名字、每一项的字节快照、哈希、依据、落点 —— 私有 */ }
impl PlannedInstall {
    pub fn name(&self) -> &str;
    pub fn hash(&self) -> &B3Hash;
    pub fn apply(self, register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>)
        -> Result<Installed, AxError>;
}
pub fn plan_install(city_root: &Path, slot: &Slot, package: &Path)
    -> Result<PlannedInstall, AxError>;
pub fn install(city_root: &Path, slot: &Slot, package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Installed, AxError>;
```

- **来源两个形状，落点随来源**：技能包是 `<name>/` 一个目录一件，内含 `SKILL.md`（与外部书架同一布局，`SKILL_FILE` 仍是它的唯一家），整包落成 `<shelf>/<section>/<name>/`：包里的每个目录与文件都照原相对路径落下，一项不丢——收下却不落的文件就是被静默丢掉的文件；居民自建技能是一份 `.md`，落成 `<shelf>/<section>/<name>.md`。两个落点都是 §8-8 扫描读的形状，`Installed::holding` 即扫描读回的那件持有（由同一个函数从盘上读出，不是第二个推导）。
- **整包哈希的字节序（规范串）**：头 `sprawling-skill-package/1
`，其后每一项按相对路径（段间用 `/`）的字节序排列，一项是：种类一字节（`d` 目录、`f` 文件）、路径长度（u64 小端）、路径字节；文件再跟内容长度（u64 小端）、内容字节。长度前缀让任何两个不同的包拼不出同一串；带头是为了一份恰好长得像规范串的文档在 CAS 里不会被读成一个包。包的哈希是这串字节的 BLAKE3，文档的哈希是正文的 BLAKE3——`PlannedInstall::hash` 与 `Installed::hash` 都是它，也是人批准的那一个。包的 `Installed::holding.hash` 仍是扫描给的 `SKILL.md` 的哈希（§8-8），两者答的是两个问题：前者是「装下的是哪一整份」，后者是「catalog 那一行变了没有」；扫描不为每次读 catalog 去读包里每个文件。
- **包的大小有上限**：一件包的文件字节合计不超过 `PACKAGE_BYTES_LIMIT`（32 MiB，`precheck::walk`）。预检把整包字节握在内存里、plan 与 apply 各读一遍，CAS 把整包存成一个 blob；一件技能是文本和几份小脚本，上限比它的本分宽得多，又远低于一台机器能握两份的量。判法按句柄报出的长度在读之前判，读时再以剩余额度加一截断，读的时候变大的文件同样被拒；超限＝`E_INVALID_ARGS`，拒词点出越线的那一项。
- **不执行代码**：本模块只读字节、判形状——包里的任何内容都不被执行、编译或解释。预检是静态的，这是它全部的含义。
- **禁符号链接，恒拒**：来源本身与书架上已有的那一件经 `symlink_metadata` 判形，包里每一层的每一项由下面的句柄逐级走判形，出现链接即拒，**不论它指向哪里、藏在第几层**——经链接读到的字节不属于这个包，且落位前后可以指向不同的东西。这条规则只在 `precheck` 里。**包按打开的目录句柄逐级走**（`cap-std`／`cap-fs-ext`）：包根由它的父目录句柄不跟随链接地打开，此后每一项只按名字、相对于列出它的那个目录句柄打开——子目录经 `open_dir_nofollow`，文件经不跟随链接的只读打开——打开后再由句柄自身的元数据判形，是链接、不是普通目录或文件、或（Windows 上）带任何重解析点（云端占位、去重文件不跟随打开时给出的是存根的字节）即拒。列举只贡献名字，列表里每一项的种类先判一次（是链接即拒）；读到的每个字节都经过一条从包根起、不含链接的句柄链，于是一个子目录在被判形之后换成链接，走读也到不了它指向的地方——换上的链接要么打不开，要么打开的是链接本身而被拒。Windows 上列举由句柄反查出的路径去列（`cap-std` 在 Windows 上的实现），一次被调包骗过的列举只会给出错的名字；这些名字仍相对真句柄打开，得到的是 NotFound 或包里自己的项，包外的字节进不来。居民自建技能的单份文档没有目录可走：`unlinked` 判路径本身的形状（来源与架上已有的同名持有都经它），`read_unlinked` 在打开的那一刻再判一次——Windows 不跟随链接地打开并拒一切重解析点，unix 比对打开前后的 inode。拒词指出是哪一项，并说出「重打包，只用普通文件与目录」。
- **名称冲突校验按「一个名字一个持有」判**：目标书架上 `<name>.md` 或 `<name>/` 任一在、而 section 不同即拒——§8-8 的扫描按名字建键，两格同名会按遍历序静默互盖。同格同名的已有持有按与来源同一个预检读出哈希（文档读正文、包算整包规范串）：同哈希是幂等（`Placed::AlreadyShelved`，一个字节不写）；异哈希即拒（与「二次出生恒拒」同形，覆写会把一件在用的技能悄悄换掉）——形状不同的同名持有哈希必不同，同样拒。同一格里 `<name>.md` 与 `<name>/` 并存也拒：那是一个名字两件持有，扫描会按遍历序留下其中一件，拿任何一件的哈希答「已在架上」都是只看了一半；拒词说出两件，恢复语让人先拿下其一。name 与 section 都必须是能落盘的单段名：非空、不以点开头、无分隔符——扫描会跳过空名与点开头的项，收下这样的名字等于装进一个 catalog 永远看不见的格子。
- **TOCTOU 复查在落位之前，落下的是快照**：`plan_install` 把来源的每一项读进内存（每项一份字节快照）并算出哈希，同时记下目标格当时有没有东西、哈希是什么；`apply` 落位前把来源整包重读一遍、重算哈希比对（哈希覆盖每一项的路径、种类、长度与内容，于是任何一项被改、增、删、换形都算「被换」），**不一致即整体拒收**（`E_VERSION_CONFLICT`）——盘上一个字节不动，`register` 不被调用。落位写的是快照里的字节而不是再去读来源，于是复查之后的一次调包也进不了书架：装上架的字节恒是来源某个完整状态的忠实映像，而落架不会盖掉一个缝里刚出现的同名持有。
- **staging 原子换入只在 `city::document` 里**：文档经 `document::replace`（暂存文件＋`rename`）；包经 `document::place_tree`——在同一 section 里一个点开头的暂存目录中写齐每一项（每个文件在暂存目录里经 `stage` 写入并 `sync_all`，不再各自暂存改名；扫描跳过点开头的项），再一次 `rename` 把整个目录换进 `<name>/`。目录换入的目标此刻不存在（存在就是 `AlreadyShelved` 或拒），所以读者看到的要么没有这件包、要么整包，没有第三种。上次崩溃留下的同名暂存目录先删掉再写：它从未被换入，没有读者见过它。
- **内容哈希入 CAS，登记先于换入**：`register` 由装配层供给（本 crate 依赖只有 `kernel`，CAS 归 `storage`；与 `Neighbourhood::scan` 的 `waiting` 同一口径），绑定的是 `storage::Cas::put`。登记的是哈希所算的那串字节——文档登记正文，包登记整包规范串（一个 blob，键就是人批准的哈希，历史能从它还原整包；逐项分别登记会留下一堆没有清单的 blob，整包哈希在 CAS 里指不到任何东西）。登记哈希与 `Installed::hash` 不符即拒（`E_CAS_CORRUPT`）：书架与 CAS 说的是同一份字节才叫有据可查。**来源记名即内容哈希**：盘上那份是现场，CAS 那份是历史（与 JOB.md 同一口径，§8-13 的 hash 答的正是「它变了没有」）。
- **失败码**：来源不在＝`E_PATH_NOT_FOUND`；来源不是包（缺 `SKILL.md`、既不是目录也不是 `.md` 文件）、包里有链接、包超过大小上限、名字或 section 不可用、名称冲突（含同格 `<name>.md` 与 `<name>/` 并存）＝`E_INVALID_ARGS`（与 `building::create` 的「名字已被占」同码同形）；依据被换＝`E_VERSION_CONFLICT`；登记哈希不符＝`E_CAS_CORRUPT`；落盘失败沿 `document::replace` 的 `E_STORAGE_FATAL`。每条拒因都带动作、主体与可执行的恢复语。
- **外部书架没有臂**：`Slot` 只有两个构造点，城库与楼架。外部书架是别人目录的只读挂载（§8-8），往那里落东西在类型上就拼不出来。

**本节测试**：`library::install::tests`——装上架的字节等于来源字节且扫描读回的 `Holding` 与 `Installed::holding` 整体相等；带子目录与多个文件的包整包落成 `<name>/`、逐项字节相等、登记的一个 blob 的哈希等于 `Installed::hash`；同哈希重装幂等（`AlreadyShelved`，盘上字节不变）；**plan 与 apply 之间来源被换即整体拒收**（`SKILL.md` 被改、多出一项、深层一项被改；拒后盘上无文件、登记零调用）；经符号链接的包恒拒，深层一项是链接也拒且拒词点出那一项；异哈希占名与跨 section 同名各拒一次；同格 `<name>.md` 与 `<name>/` 并存即拒；超过大小上限的包即拒（按句柄报出的长度，不读字节）。`document::tests` 另有一例：目标已在时 `place_tree` 拒收，目标旁不留暂存目录、目标原样。CAS 绑定的端到端一例在 `crates/sprawling/tests/skill_install.rs`（`Cas::put` 兑付 `register`，装上架的内容可按 `Installed::hash` 从 CAS 取回同一份字节）。

### 8-29 city::check：一座城的每份 TOML，逐份读一遍，错处落到行列（形状 1 判定）

`sprawling check <city>` 的判定半边。它不另写解析：每份文件交给运行时读它的那个解析器（`ConfigLayer::parse`、`policy::evaluate`、`Schedule::parse`、`Watch::parse`），那个解析器的结论就是结论；本模块只再做一件事——解析器拒了，就用同一个文件形状（`ConfigFile`、`Written`、`ScheduleFile`、`WatchFile`，`pub(crate)`）再读一次，取 `toml` 报的 span，折成行列。

```rust
// city::check
pub struct Position { pub line: usize, pub column: usize }   // 都从 1 数；列按字符数，不按字节
pub struct Finding { pub path: PathBuf, pub at: Option<Position>, pub error: AxError }
pub struct Report { pub read: usize, pub findings: Vec<Finding> }  // read：存在并读到的文件数
pub fn check(city_root: &Path) -> Result<Report, AxError>;
```

- **读哪些文件**：城层 `CONFIG.toml`；`building::all` 列出的每栋楼的楼层 `CONFIG.toml` 与 `RULES.toml`；`room::all` 列出的每个房间的居民层 `CONFIG.toml`；`SCHEDULE.toml`、`WATCH.toml`。不存在的文件不是错——城没写它就是默认。
- **每份文件至多一条 Finding**：解析器在第一处错停下，check 不比它多说。
- **`at` 为 `None`**：文件形状读得过、错出在形状之后的判定（一个不是 Address 的写前缀、一个空的 match），`toml` 没有 span 可给，于是不编一个位置。
- **失败**：列楼、列房间、读文件的 I/O 失败原样上传（`E_STORAGE_FATAL` 一族），因为那时「这座城有没有错」答不出来。

**决定**：位置从同一形状的二次读取来，而不是给 `AxError` 加一个位置字段——`AxError` 是全仓的错误形状，为一个只读动词给它加字段，每个构造它的地方都要多想一件事；二次读只在已经出错时发生，成功的读仍是一次。

### 8-30 这个目录有没有城的历史（`city::history`，形状 1 判定）

**接口**：`pub enum History { Absent, Present }`；`pub fn has_history(city_root: &Path) -> Result<History, AxError>`。账本目录由 `kernel::layout::CityLayout::ledger` 回答；目录不存在或为空是 `Absent`，有一个条目是 `Present`；目录在却列不出来是 `StorageFatal`，恢复提示「让账本目录可读，或换一个城目录」——把列不出来当 `Absent` 会让 `init` 在一座只是读不到的城上再写一次创世。

**决定**：这件事住在 `city`，与其余读城在盘上布局的函数（`buildings`、`survey`）同层。`init` 与 `up` 在装配点问它，doctor 也问它；doctor 是读面，读面不依赖装配点（sprawling-SPEC 8-92），而 `city` 是 doctor 本来就依赖的一层。**败给的方案**：放进 `kernel::layout`——kernel 不做文件 I/O；放进 `storage`——`storage` 读的是账本的行，这里只问目录里有没有东西，而问它的三个地方都已依赖 `city`。

### 8-31 `[clock]`：一层说结果多久带一次时钟行（`config_layers::clock`，形状 1 判定）

**接口**：`CONFIG.toml` 的 `[clock]` 表，一个键：

```toml
[clock]
stamp = "minute"   # "off" | "minute" | "five_minute" | "hour"
```

值的拼法是 `kernel::ClockStampGranularity` 的 serde 拼法，本模块不另写一份；`ConfigLayer::clock_stamp() -> Option<ClockStampGranularity>` 报这一层说了什么，`load` 用 `ladder.resolve(ConfigLayer::clock_stamp)` 把三级求成 `FrozenConfig.clock_stamp`，谁也没说时落到 `kernel::consts_policy::CLOCK_STAMP_DEFAULT`（`Minute`）。时区梯仍传空的 `LayeredValue`。

**拒什么**：`[clock]` 表 `deny_unknown_fields`。`zones` 与任何别的键、拼不出的值（`"minutes"`）都在解析时拒，走本模块既有的那一种拒法（`refuse::unreadable`）：主体是 serde 点名的键或值与它接受的集合，恢复语是「under `[clock]`, change the value the message names, or take that key out」。

**读者**：粒度只在 `runtime::clock::StampGate` 里起作用（`crates/runtime/Spec.lean` §8-10）；生产的每一跑由装配层按冻结下来的值造一个 `StampGate`（sprawling-SPEC 8-125）。

### 8-39 `[remote]`：城那一层选远程门的通路（`config_layers::remote`，形状 1 判定）

**接口**：

```rust
// city::config_layers::remote；对外 city::{remote_route, RemoteRoute, HostPermanence}
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    Cloudflare { tunnel: String, url: String, command: Option<String> },
    Command { command: String, args: Vec<String>, permanence: HostPermanence },
}
pub enum HostPermanence { Fixed, PerStart }       // 拼法 "fixed" | "per_start"
pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError>;
impl ConfigLayer { pub fn remote(&self) -> Option<&RemoteRoute>; }
```

```toml
[remote]
route = "cloudflare"
tunnel = "my-city"                  # `cloudflared tunnel create` 时起的名字
url = "https://city.example.org"    # 隧道的 DNS 路由指向的主机
# command = "C:/tools/cloudflared.exe"   # 不写时从 PATH 找 cloudflared

[remote]
route = "command"
command = "sh"
args = ["/path/to/tailscale-route.sh"]
permanence = "fixed"                # "fixed" | "per_start"
```

- **只有城那一层能写**：远程门开在整座城上，一栋楼或一个房间写下通路，就是一份下层的文件替全城决定外面从哪里进来。楼层、房间层写 `[remote]`，`ladder::stated` 读到它即拒（`E_CONFIG_INVALID`），恢复语指向城根的 `.sprawling/CONFIG.toml`。这与 `[skills] shelves`（§8-8）是同一条判定：`refuse::CityOnly` 有 `Shelves`、`Remote` 两臂，各带键名与理由，`ConfigLayer::city_only` 答一层写下的第一张这样的表，拒法只有 `refuse::below_city` 一处。
- **`route` 选臂**：`route` 是闭集（`cloudflare`｜`command`），每一臂只收自己的键。别的臂的键、不认的键、缺 `route`、缺本臂必需的键（`cloudflare` 的 `tunnel` 与 `url`，`command` 的 `command` 与 `permanence`）都由 serde 拒，主体点名那个键，拒法是本模块既有的 `refuse::unreadable`。`command`＋`args` 与 `[[mcp]]` 的同名两键同形，人认一次；`command` 为空串即拒。
- **值照写下的读进来**：隧道名合不合 `cloudflared` 的写法、`url` 是不是 `https://`、程序在不在，由 `remote_access` 的类型判（`TunnelName::parse`、`PublicUrl::parse`，`crates/remote_access/Spec.lean` §8-7 到 §8-9），判在装配层每次 `/remote open` 把这张表造成通路的那一刻（sprawling-SPEC 8-151）。本 crate 只见 `kernel`，理由与 `[resident] harness` 相同（§12.16）。
- **`permanence` 必写，没有缺省**：命令证明不了自己的主机名跨重启不变（`crates/remote_access/Spec.lean` §8-9）。缺省成 `fixed`，一条每次换主机名的通路就会被当成不换，设备的密钥在下次重启后读不到，而控制台没有提醒过人。`cloudflare` 一臂没有这个键：命名隧道的主机名是人用 DNS 路由钉住的，答 `Fixed`。
- **`remote_route` 只读城那一层，不爬梯子**：与 `city_shelves` 同一种读法（`ladder::stated(city_config, Layer::City)`）。文件不在、或文件里没有 `[remote]`，答 `E_CONFIG_INVALID`：主体是那份文件与「no `[remote]` table chooses a route」，恢复语写出两臂各自必需的键并指向 `docs/operating.md`。缺表对唯一的调用方 `/remote open` 就是拒绝，所以不答 `Option`。

**测试**（`config_layers::remote::tests`）：两臂各读回整个值；`cloudflare` 缺 `url` 即拒，主体点名 `url`；楼层写 `[remote]` 即拒，恢复语指向城那一层；城那一层没有 `[remote]` 时，`remote_route` 的拒绝点名这张表与两臂必需的键。

### 8-32 楼的写域是上限，一次派活的写入限制只收窄它（`city::policy`，形状 1 判定）

**接口**：不新增。`BuildingRules::write_domain()` 照旧从 `RULES.toml` 的 `prefixes` 与 `write` 两键给出 `kernel::WriteDomain`；一次 run 的写入限制 `kernel::WriteLimit` 由派活给出（kernel-SPEC §8-78），在写门上与写域各判一次。

- **两道判定都要过**：写域回答能不能写这个地址、写哪种文件；写入限制回答能不能改动已经存在的文件。`full` 不额外收窄，所以普通档永远放不宽楼的 `RULES.toml`；`create` 在 `write = "documents"` 的楼里仍只能新建 Markdown 文档、仍够不到计划文件；保留子树对两者都在写域之外。
- **`RULES.toml` 没有写入限制的键。** 一栋楼的规则说它允许什么，一次派活的限制说这一次要多小心；两者分在两处，「哪一处说了算」就不用规则来回答（§12.10）。
- 验收：写域一侧的判定由 kernel 的 `gate::domain` 测试与本 crate 的 `policy` 测试持有；两道判定的组合在 runtime 的 `an_existing_file_is_unchanged_under_create_by_edit_exec_and_link` 里由真实写路径观察。

### 8-33 `PREFERENCES.md` 与 `MAYOR.md` 的身份区：城怎么称呼人与主 Agent（`city::identity`，形状 2 值类型＋形状 1 判定）

```rust
pub const NAME_MAX_CHARS: usize = 64;
pub struct DisplayName(String);                 // 一行、去掉首尾空白后非空、无控制字符、不超过 NAME_MAX_CHARS 个字符
impl DisplayName { pub fn parse(raw: &str) -> Result<DisplayName, AxError>; pub fn as_str(&self) -> &str; }

#[derive(Serialize, Deserialize)]
pub struct Naming { /* person、imported_from、about、mayor */ }
impl Naming {
    pub fn read(city_root: &Path) -> Result<Naming, AxError>;        // 读两份文件的身份区
    pub fn person(&self) -> Option<&str>;
    pub fn imported_from(&self) -> Option<&str>;
    pub fn about(&self) -> &str;
    pub fn mayor(&self) -> Option<&str>;
    pub fn to_bytes(&self) -> Vec<u8>;                                // 冻进内容库的那份字节（JSON）
    pub fn from_bytes(bytes: &[u8]) -> Result<Naming, AxError>;
    pub fn version(&self) -> B3Hash;                                  // = B3Hash::digest(to_bytes())
    pub fn context(&self) -> Option<String>;                         // 接在 city 段之后的那一块；什么都没说时为 None
}
pub enum NamingEdit {
    Person { user_id: Option<String>, imported_from: Option<String>, about: Option<String> },
    Mayor { name: Option<String> },
}
pub struct Unreadable { pub document: Governed, pub line: u32, pub why: String }
pub fn read_naming(city_root: &Path) -> Result<Result<Naming, Unreadable>, AxError>;   // 页面读：读不出给出位置
pub fn write_naming(city_root: &Path, edit: &NamingEdit, base: &str) -> Result<Naming, AxError>;
pub fn write_governed(city_root: &Path, which: Governed, base: &str, body: &str) -> Result<PathBuf, AxError>;
pub fn persona(written: &[u8]) -> &[u8];                              // 身份区之后的正文
pub fn freeze_naming(city_root: &Path, room: &Address, version: B3Hash) -> Result<(), AxError>;
impl ConfigLayer { pub fn naming(&self) -> Option<B3Hash>; }          // 房间这一层冻下的身份版本
```

**身份区的语法。** 文件的第一行恰好是 `+++` 时，到下一行恰好是 `+++` 为止是一段 TOML，其后是正文；第一行不是 `+++` 的文件没有身份区，整份都是正文——旧文件照旧读。`PREFERENCES.md` 的身份区读 `user_id` 与 `imported_from` 两个键，正文是「关于你」；`MAYOR.md` 的身份区读 `name`，正文是这位主 Agent 是谁。身份区里别的键不读、也不丢：改写只动卡片上的那几个键。

**拒绝。** 只有一处读身份区，读不出时它说出原文位置：没有闭合的 `+++`（行号是开头那一行）、TOML 本身的错（重复的键、坏的值，行号从文件第一行数起）、一个键的值不是字符串、名字不合 `DisplayName` 的域。码是 `E_CONFIG_INVALID`，主语是文件名与行号，恢复语说改这一行或删掉身份区。读身份区的每个调用方都走这一处，所以开城、派活、页面、保存看到的是同一个判定：读不出时**不**回落成默认名字——那样下一次保存就会把人的正文连同读不出的那一行一起盖掉。

**写。** `write_naming` 在 `base` 上改写身份区：卡片上的键为 `Some` 时写入（先过 `DisplayName`），为 `None` 时删去；`about` 为 `Some` 时换掉正文；身份区删空了就连 `+++` 一起去掉，所以从没设过名字的文件改一次再改回来，逐字节回到原样。落盘经 `document::edit_against`：文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`，什么都不写。`write_governed` 是原文编辑器那扇门，同一道基线守卫；它在落盘之前对 `MAYOR.md` 与 `PREFERENCES.md` 的正文读一遍身份区，读不出就拒。

**进上下文。** `context()` 给出接在 city 段之后的一块：这个人叫什么、关于他的那段正文、主 Agent 叫什么并且就是 `hall/mayor`，最后一句说这些名字来自两份文件的身份区。什么都没说（没有名字、正文为空）时为 `None`，city 段的字节与从前一样。`hall/mayor` 自己的 resident 段以 `Your name: <name>` 开头，没有名字时照旧是地址末段；`MAYOR.md` 进 resident 段的是 `persona` 给出的正文，身份区不进去，因为名字已经由那一行说了，一个名字在一次请求里只有一个说法。

**一个 session 冻一版。** `Naming` 序列化成 JSON 放进内容库，摘要就是 `version()`；session 的第一次 run 把它写进房间自己那一层的 `[identity] version`（`freeze_naming`），之后这个 session 的每次 run 都从内容库读回那一版，所以改名不会在一个 session 中途改掉请求前缀。开新 session（`city::clear_session`、`forget_shape`）连同 `[model]` 一起删去 `[identity]`，下一次 run 冻下此刻的版本。只有身份区与正文进 `Naming`；`MAYOR.md` 正文进 resident 段的那份字节照旧在每次 run 现读。

**验收**：`identity` 的测试：没有身份区的旧文件照读；`a_stale_identity_save_is_refused_and_the_draft_survives`（`base` 过期被拒，文件仍是后写的那一份）；改一次再改回逐字节不变；重复的键与没有闭合的 `+++` 各拒一次并给出行号；未知键在改写后仍在。

### 8-34 页面写的两处：楼规带基线、城一层的两个键（`city::policy`、`city::config_layers::city_layer`，形状 4 adapter）

```rust
pub fn write_rules_against(city_root: &Path, addr: &Address, base: &str, text: &str) -> Result<BuildingRules, AxError>;
pub enum CitySetting { KeepWarm(KeepWarm), Effort(Effort) }
pub fn write_city_setting(city_root: &Path, setting: CitySetting) -> Result<(), AxError>;
```

- **楼规：先求值，再守基线。** `write_rules_against` 先 `evaluate(addr, text)`，拒了就什么都不写；通过之后经 `document::edit_against` 落盘，文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`。`write_rules` 照旧是 `rules` 工具的门：市长在一次 run 里提案，它读到的那一份就是它要换掉的那一份，没有页面上那种「打开之后别人改过」的窗口。两扇门共用同一个求值器与同一个落盘函数，所以「盘上的楼规永远读得懂」只有一条规则。
- **城一层：两个键各一臂。** `write_city_setting` 经 `config_layers::write` 那一个读改写入口改城自己的 `CONFIG.toml`：`KeepWarm` 写 `[cache] keep_warm`，`Effort` 写 `[model] effort`；同一文件里别的键原样留着，写出的字节先过 `ConfigLayer::parse` 才落盘。城一层不是 session 的记录处，所以这里写 `[model] effort` 不碰 `[model] name`。
- 验收：`config_layers::city_layer` 的 `a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it`；楼规两道判定的组合在 accounting 的 `a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing` 里经真实命令观察。

## 模板的写法：格式标注的是「该多小心」（`crates/city/templates/`）

**格式不是允许与否的门禁，是谨慎程度的标记**，而且不设门禁把它变红：想清楚了照样改。据此三类：

1. **机器要解析的 → 结构化**。`RULES.toml` 是现成范例（未知键拒识，于是拼错是一条消息而不是一个永远没到的许可）；`Roadmap.md` 的六列表格同理，`kernel::spine` 解析它，`plan` 是唯一写者。
2. **Agent 要写、写坏不影响运转的 → Markdown 外壳 + XML 小标题**。标签独占一行、正文不嵌套、未填的节内容只有括号行——**只标上下限并说明用途**，不做严谨标记语言，正则就能取。结构化字段放标签属性（如 `<finding id="1.1" area="view" stage="S3">`），自由散文放标签内容，于是定位引用、表格、列表都放得下且无转义问题。适用：`SPEC.md`、`URBANITE.md`、`Handoff.md`、`JOB.md`、`Roadmap.md` 的各节。
3. **`Memo.md` 完全自由**。它是「有东西要记、却没有别的规范可依」时的记事本，记法由写的人按需要选；有自己规范的东西走自己的文档（计划进 `Roadmap.md`，项目要站得住的决策进 `SPEC.md`）。

**没有备忘判形**：一个自由记事本不该被判 Malformed，所以 `kernel::spine` 不对 `Memo.md` 的节做强制。

**进度条只有一份数据：任务表。** 它是派生值，由脚本唯一写者重画，每格一个任务、顺序即落地顺序，半格表达进行中——不落盘手写，于是「表与条对不上」这件事构造上不可能。`blank.rs::is_blank_form` 的判空从行前缀（跳 `#`、`>`、括号行）改成看标签边界，于是一条以 `(` 开头的正文不再被误判为空。

**人设与纪律不同居**：`MAYOR.md`／`CLERK.md` 的人设留 Markdown（人随便写，想让市长学谁说话都行），纪律（工具清单、绝不做什么、何时停）编译进 `spine_files::hall`，与 `EPHEMERAL_SEGMENT` 同型——人设覆盖不到它，这正是「插得进去但毁不掉」。
