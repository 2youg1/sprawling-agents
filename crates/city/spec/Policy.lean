-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Delegation
import crates.kernel.spec.Layout

/-!
# city::policy

规定 `policy`、`policy::cache`、`policy::evaluate`、`policy::reach`、`policy::user_browser`、`policy::desktop`、`config_layers::city_layer`（`crates/city/src/` 下同名的文件）。这栋楼里允许什么：`RULES.toml` 求值成规则、读界、写域、楼的治理文件落在哪、页面写楼规的那扇门。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
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
pub struct WorkspaceModelPolicy { /* allowed、max_effort、recommendations —— 私有 */ }
pub struct ModelRecommendation { /* model: ModelRef、effort: Effort —— 私有 */ }
pub struct WorkspaceRecommendations { /* easy、medium、hard —— 私有 */ }
impl WorkspaceModelPolicy {
    pub fn recommendation(&self, difficulty: Difficulty) -> &ModelRecommendation;
    pub fn digest(&self) -> Result<B3Hash, AxError>;       // 规范 JSON 字节的 BLAKE3，run_started 钉这一版
}
impl ModelRecommendation {
    pub fn model(&self) -> &ModelRef;
    pub fn effort(&self) -> Effort;
}
pub struct BuildingRules { /* addr、policy、write_prefixes、reach、egress、review、browser、usersbrowser、desktop、reading_room、harness_minutes、workspace_models、checkpoint_entropy_allow —— 私有 */ }
impl BuildingRules {
    pub fn workspace_models(&self) -> Option<&WorkspaceModelPolicy>; // 没写这一节即 None，旧路径不变
    pub fn policy(&self) -> &BuildingPolicy;          // 随每次模型调用出行
    pub fn addr(&self) -> &Address;
    pub fn egress(&self) -> Option<&EgressAllowlist>; // `None`：楼没写这一键，不限主机（D27）；confidential 楼恒 `Some(空表)`，它声明的表在 evaluate 处即拒
    pub fn review(&self) -> bool;                     // 缺这一行读作 false：run 的写由别的居民合并后才可见
    pub fn browser(&self) -> bool;                    // 缺这一行读作 false；confidential 楼恒 false
    pub fn reach(&self) -> DomainReach;               // Building 分部
    pub fn reading_room(&self) -> &[String];          // AGENTS.md 的 `## Reading room`，Library 分部
    pub fn model_pool(&self) -> ModelPool;
    pub fn usersbrowser(&self) -> Option<&UserBrowser>;
    pub fn write_domain(&self) -> Result<WriteDomain, AxError>;
    pub fn harness_minutes(&self) -> u32;             // harness run 的墙钟上限；缺这一行读作 60
    pub fn checkpoint_entropy_allow(&self) -> &EntropyAllowlist; // 精确的仓库相对路径；缺省空
}
pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError>;
pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError>;
// 楼规唯一的写门 write_rules_against 见 §8-34
pub fn rules_path(city_root: &Path, addr: &Address) -> PathBuf;
pub struct RulesCache { /* city_root、按楼存的 (mtime, len) 与规则 —— 私有 */ }
impl RulesCache {
    pub fn new(city_root: &Path) -> RulesCache;
    pub fn load(&self, addr: &Address) -> Result<Arc<BuildingRules>, AxError>;
}
```

**`harness_minutes`**：一次 harness run 在这栋楼里最多跑多少分钟（`crates/sprawling/Spec.lean` §8-124），正整数；缺这一行读作 `HARNESS_MINUTES_DEFAULT`（60）。`0` 由 serde 在解析时拒（`NonZeroU32`）：一个到点即停的上限是笔误。它只管 harness：模型 run 的每一回合都经城自己的安全点，停一件事是 `cancel`、停一片是 `halt`，而 harness 的回合在城之外走完，一个既不说话也不结束的 harness 没有别的路让出车道（D8 (b)）。

`RulesCache`（`policy/cache.rs`，形状 1 判定）：一个 run 一份，读界的闭包持有它。`load` 先对 `RULES.toml` 做一次 stat，(mtime, len) 与上次读到的相同就交回留着的规则，不同或第一次就走 `load` 读盘求值并按这次 stat 的戳留下。文件不存在或 stat 失败时不留任何东西、每次都走 `load`，于是「没有 RULES.toml」与「被替代的旧文档」两条的答案与不缓存时逐字相同。锁中毒时同样退回 `load`。失败与 `load` 相同，失败不留。决定见 D3。

- **规则是一份 TOML，散文没有另起一份文件**：若是 Markdown，读者就得在**任意一行**上匹配 `confidential:`／`write:`／`review:`／`browser:`／`usersbrowser:`／`desktop:`，于是「How work is done here」里一句以 `desktop = true` 开头的话就授予了宿主机的桌面，而一栋没写 `write:` 的楼落到 `Everything`。两处都朝宽松的一侧失败，那是权限读者唯一不许失败的方向。改成 TOML 之后键只在文法给出键的位置成立，`deny_unknown_fields` 让拼错成为一条消息而不是一次静默缺席，`confidential` 与 `write` 都不再有缺省。**散文留在同一份文件里**，作 `does` 与 `conventions` 两个键：拆成两份文档同样能关掉撞键，代价是一栋楼有两种说法且可以互相矛盾。居民拿到的就是这份文件本身的字节，所以城判定的与 agent 读到的是同一串。

- **confidential 四条各有其守处**：模型池锁本地由 `gateway::endpoint` **在会泄漏的那一端**拒（`req.policy.confidential` 即拒，携三段式）；写域止于本楼子树由 `write_domain()` 在构造点拒；数据可入不可出归出网门；**楼里的字节楼外读不到**，归读界（下一条）。**把兜底放在会出事的那一层**，路由错了仍然拦得住。
- **读界：本楼全开，他楼非机密可读，机密楼对楼外全关**。判定只有一处：`kernel::address::may_read(reader_building, target, rules) -> ReadVerdict`（`crates/kernel/Spec.lean` §8-2），三臂 `Open`／`Confidential`／`RulesUnreadable(AxError)`。问它的是模型选路的唯一判定处 `runtime::tools::chosen_path`（`crates/runtime/Spec.lean` §8-30-1）：`read` 的路径、`search` 的起点、`search` 不带路径时在城根下走进的每一栋楼，以及经这两件工具读到的 transcript（`crates/runtime/Spec.lean` §8-32），都先过它。`rules` 是目标所在楼此刻的规则：装配层把 `city::Building::of(target)` 与 `policy::load` 接成一个闭包交给 `may_read`，**只在目标出了本楼时才调用**——本楼的读不读盘，他楼的读每次现读规则。规则读不出＝`RulesUnreadable`，一样关：读不出的那份规则可能写着 `confidential = true`，而隐私设置不得朝宽松的一侧失败（本节「没有 RULES.toml」那条同理）。**读界只管模型选的路径**：catalog 名是人在阅览室里准入的（`crates/runtime/Spec.lean` §8-29 起首），不经它；`exec` 在宿主机上跑的命令读得到盘上任何文件，那道墙要 OS sandbox，与「出网」那条的缺口是同一个缺口。
- **没有 RULES.toml 是普通楼；有而不声明是错误**：把隐私设置的默认值悄悄取成宽松的那一边，正是这整个面存在的理由。拼写不是 `true`／`false` 同样拒——读起来像笔误的隐私设置不得解析成许可。
- **confidential 楼声明越界前缀＝拒而不裁剪**：静默裁剪会让文件说一套、城做另一套；拒绝会指出该改哪一行。
- **无声明写域时默认只写本楼**：一栋楼至少能写自己，且不多。`prefixes` 里一条读不出的地址**传播而不跳过**——在读它的地方丢掉，一栋楼就写得比人授予的少，而这件事没有任何一处说出来。
- **`review = true` 是楼级开关**：开则每个 Run 得一棵自己的 worktree，写的东西在别人检查并 merge 之前对楼不可见。**默认关**，与 confidential 的「不声明即错」相反——隐私的默认值不得惄悄取宽，而审查纪律的默认值不得惄悄取严：一个人派一个 Agent 去改一行字并盯着看，应当看得到文件变化。拼写不是 `true`／`false` 同样拒。
- **`egress` 是可选的域名表**：不写这一键，楼不限主机；写了表，城的浏览器与人的浏览器只导航到表中域名及其子域；空表是不到任何公网主机（D27）。表随 run 冻结，经 `limit_egress` 交给 `runtime::bench::ToolBench`，由它在每次调用前以 `kernel::gate::egress::egress_target` 判定。类型经 `kernel::EgressAllowlist` 重导出，住哪一簇文件是 kernel 内政（`gate::egress`，公共拼写不变）。**confidential 楼同时列域名＝矛盾，拒**——「数据可入不可出」是那个设置的含义，域名表写在它下面会逼读者自己去调和两句话。
- **`checkpoint_entropy_allow` 是 User 给低置信度扫描留的出路**：值是仓库相对文件路径的数组，每项先过 `Address::parse`，匹配整项而不是前缀或 glob。它只豁免这些文件里的熵命中，已知厂商形状照样拒绝；不含命中正文，也不写摘要。文件住在楼自己的保留子树，agent 只能读不能改（D1）；装配层把同一份 `EntropyAllowlist` 交给 model lane、bench 的预测网、harness、adoption 与 landing 的 checkpoint。缺省为空，旧城行为不需要迁移。处置与验收的权威是 `crates/storage/spec/Checkpoint.lean` D95，这里只规定配置从哪里来、谁能改。
- **执行点与缺口要分清**：provider 路径由 `endpoint` 的 confidential 拒守住；Agent 自己发起的出网是否被拦，取决于 exec 所在的那一臂 `runtime::tools::exec::Confinement` 是否承诺关网（`Guarantee::Network`，每一臂在工具描述里说出自己不承诺什么），浏览器工具一次调用点名的页面由 bench 按表判，页面里的子资源、重定向与 `exec` 发起的连接不经这张表。判定在这里，拦截只到调用点名的那个主机——不承诺关网的地方不要说「出网已管住」。
- **`usersbrowser` 一键同时是开关与地址**：值 `"ws://127.0.0.1:<port>/session"` 启用并声明地址，`true` 启用而地址未定（工具每次调用都得到门的问题），absent／`false` 即无此工具。**confidential 楼写这一键即拒**（`E_CONFIG_INVALID`）——附着读的是那个人浏览器里全部登录态，本楼的隔离在那一刻失效。地址的**语法**（`ws://`、主机形状）在此读一次并把 `url`／`host` 一起交出；**loopback 与否是 `kernel::gate::attach` 的政策**，语法不替政策作答。
- **`browser` 与 `usersbrowser` 是两个键**：前者是城自己拉起的浏览器（profile 按楼隔离），后者是人已经开着的那个（人的真 profile）。`browser` 不是 `usersbrowser` 的前缀截断——TOML 的键是文法给出的整体，两个设置因此互不误读。
- **`write_rules_against` 先求值再落盘**：一份写到一半就不再求值的治理文档会把它那栋楼一起带走。且**整份文档才是单位**：confidential 楼不得列域名，故两行可以各自合法而合在一起非法。
-/

/-!
### 8-11 楼的治理字节搬进它自己的保留子树（沉淀一处路径权威）

```rust
pub fn rules_path(city_root, addr) -> PathBuf;      // <building>/.sprawling/RULES.toml
pub fn agents_path(city_root, addr) -> PathBuf;     // <building>/AGENTS.md（项目的，故在保留区之外）
pub fn city_agents_path(city_root) -> PathBuf;      // <city>/AGENTS.md：城围起来的工作区自己的那份
pub fn room_agents_paths(city_root, building, room) -> Result<Vec<(Address, PathBuf)>, AxError>;  // 楼根以下到 room 的每一级 <level>/AGENTS.md，近楼的在前；楼根那份不在其中
pub fn config_layers::path(city_root, addr, layer) -> Result<PathBuf, AxError>;
// 三层都是 <scope>/.sprawling/CONFIG.toml；city 层与另两层同一个式子
```

- **三层一个表达式**：`path()` 先算出 scope 目录（城根、楼根、房间目录），再一律 `.join(RESERVED_PREFIX).join(CONFIG_FILE)`。只有 City 层在保留区里、另两层不在，就是一个 Agent 写域够得着自己配置的洞口。
- **旧城必须报错，不得静默降级**：`policy::load` 把「规则不存在」当作默认策略（`confidential = false`）。于是一座旧城的楼会从「机密」静默变成「不机密」——所以 `RULES.toml` 缺失而一份**本版不再读的文件**仍在盘上时**拒绝**，`E_CONFIG_INVALID`，recovery 直接拿出要执行的那一步。这不是兼容适配层（它不读旧文件），是一道不让静默降级发生的门。它问的是一个问题而不是每次搬家加一道守卫：`superseded` 同时看楼根与保留子树下的旧名，因为两次搬家（改位置、改格式）的后果完全相同。
- **楼页仍然看得见规则**：`assembly` 组楼页答案时跳过一切点头目录（房间枚举因此也不会把 `.sprawling` 当房间），故 `RULES.toml` **按路径显式读一次**再入档。人在界面上看得到、改得了；楼里的 agent 读得到、写不了。
- **默认写域仍是整栋楼，这是故意的**：一次 Run 为它那栋楼产出一份楼级产物是正常的；把默认收紧到房间会把那件事一并禁掉，而洞口在于治理文件的位置，不在于写域的宽窄。
-/

/-!
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
-/

/-!
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

**线上它走 `ConfigureBuilding` 的第四个可选字段**（`crates/wire/Spec.lean` §8-40）：那条帧问的就是「这栋楼的 runs 按什么规矩来」，沙箱、外部服务器、运行中的机器上的窗口与第二级提醒落哪一层是同一个问题的四面。`configured_payload` 因此收一个 `Written { sandbox, mcp, desktop, context }` 而不是四个裸布尔——一个调用点写 `(true, false, true, false)` 说不出哪一位是哪一面。
-/

/-!
### 8-32 楼的写域是上限，一次派活的写入限制只收窄它（`city::policy`，形状 1 判定）

**接口**：不新增。`BuildingRules::write_domain()` 照旧从 `RULES.toml` 的 `prefixes` 与 `write` 两键给出 `kernel::WriteDomain`；一次 run 的写入限制 `kernel::WriteLimit` 由派活给出（`crates/kernel/Spec.lean` §8-78），在写门上与写域各判一次。

- **两道判定都要过**：写域回答能不能写这个地址、写哪种文件；写入限制回答能不能改动已经存在的文件。`full` 不额外收窄，所以普通档永远放不宽楼的 `RULES.toml`；`create` 在 `write = "documents"` 的楼里仍只能新建 Markdown 文档、仍够不到计划文件；保留子树对两者都在写域之外。
- **`RULES.toml` 没有写入限制的键。** 一栋楼的规则说它允许什么，一次派活的限制说这一次要多小心；两者分在两处，「哪一处说了算」就不用规则来回答（D10）。
- 验收：写域一侧的判定由 kernel 的 `gate::domain` 测试与本 crate 的 `policy` 测试持有；两道判定的组合在 runtime 的 `an_existing_file_is_unchanged_under_create_by_edit_exec_and_link` 里由真实写路径观察。
-/

/-!
### 8-34 页面写的两处：楼规带基线、城一层的配置键（`city::policy`、`city::config_layers::city_layer`，形状 4 adapter）

```rust
pub fn write_rules_against(city_root: &Path, addr: &Address, base: &str, text: &str) -> Result<BuildingRules, AxError>;
pub struct RulesEdit<'a> { pub base: &'a str, pub text: &'a str }   // 页面提议的正文与它先读到的那一版
pub fn write_rules_against_with(city_root: &Path, addr: &Address, edit: RulesEdit<'_>,
    validate: impl FnOnce(&BuildingRules) -> Result<(), AxError>) -> Result<BuildingRules, AxError>; // 同一次求值后先过端点簿，再守 base 并落盘
pub enum CitySetting<'a> { KeepWarm(KeepWarm), Effort(Effort), Catalog(&'a ExposureTable),
                           SkillShelves(&'a [String]) }   // [catalog] 见 ConfigLayers §8-4g；书架见 Library §8-8
pub fn write_city_setting(city_root: &Path, setting: CitySetting<'_>) -> Result<(), AxError>;
```

- **楼规：先求值，再守基线。** `write_rules_against` 先 `evaluate(addr, text)`，拒了就什么都不写；通过之后经 `document::edit_against` 落盘，文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`。它是楼规唯一的写门（`accounting::worker::commanding::configure`，人经 `ConfigureBuilding`）；`rules` 工具只读，没有写面（§8-2b，D23），所以「盘上的楼规永远读得懂」只有一条规则。
- **城一层：每个配置键各一臂。** `write_city_setting` 经 `config_layers::write` 那一个读改写入口改城自己的 `CONFIG.toml`：`KeepWarm` 写 `[cache] keep_warm`，`Effort` 写 `[model] effort`，`Catalog` 写 `[catalog]`，`SkillShelves` 整表写 `[skills] shelves`；同一文件里别的键原样留着，写出的字节先过 `ConfigLayer::parse` 才落盘。城一层不是 session 的记录处，所以这里写 `[model] effort` 不碰 `[model] name`。
- 验收：`config_layers::city_layer` 的 `a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it`；楼规两道判定的组合在 accounting 的 `a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing` 里经真实命令观察。
-/

/-! ### 8-35 `RULES.toml` 的工作区模型策略（`policy::workspace`，形状 1 判定＋形状 2 值类型）

真实场景：人在 `shop` 工作区允许本地、普通、强三个已登记模型，把 `easy / medium / hard` 分别指到普通＋low、普通＋medium、强＋high；父 Session 只给 `delegate.difficulty`，城市在子 Session 第一次派活时选型。没有 `[workspace_models]` 的楼完全走旧 `main` 路径。

```toml
[workspace_models]
allowed = [{ endpoint = "house", model = "ordinary" }, { endpoint = "house", model = "strong" }]
max_effort = "high"
[workspace_models.recommendations.easy]
endpoint = "house"
model = "ordinary"
effort = "low"
[workspace_models.recommendations.medium]
endpoint = "house"
model = "ordinary"
effort = "medium"
[workspace_models.recommendations.hard]
endpoint = "house"
model = "strong"
effort = "high"
```

- **一个 ModelRef 同时携 `(endpoint, model)`**，同名模型跨 provider 不合并。`allowed` 非空；三档推荐都必须在其中；`max_effort` 与三档 `effort` 都是真实档位，旧账本兼容词 `none` 在这里拒绝；每档都不高于 `max_effort`。这些结构不变量在 `evaluate` 一次性判完，失败不落任何字节。
- **页面保存还要过端点簿**：`PutRules` 与携自定义规则的 `CreateBuilding` 在现有写门之前，用同一本 `EndpointBook` 拒掉未接入、未登记或不存在的引用，以及已知不提供推荐 effort 的模型。文件已有并发改动仍由 `base` 拒；任一拒绝都不留下半份策略。
- **策略只管城市原生模型居民**：harness 自己选模型，`agree_to_work` 的 harness 臂不读这张表。confidential 的本地边界仍由 `EndpointBook::select` 在选型时判，策略不能放宽它。
- **只在新 Session 的委派路径上推荐**：已有房间记录时忽略本次难度，继续走冻结形状；新子 Session 有难度取对应推荐，没难度取 `medium`。人显式给出的 `Dispatch.model`／`effort` 优先，但仍受 allowed／max effort 与 provider offer 约束。推荐端点被摘除、模型不再登记、effort offer 改变或 confidential 不允许时拒绝，不降档、不另换模型。
- **版本随子 run 入账**：策略的规范 JSON 字节摘要进入 `run_started.routing.policy`；实际 `ModelRef`、请求难度与选择来源同在这条记录，实际 effort 沿用已有的 `run_started.effort`，不写第二份。重放只读这些行，不重新评估任务。
- **新楼的模板带一段注释占位**：`crates/city/templates/RULES.toml` 末尾把 `allowed`、`max_effort` 与三档推荐写成注释，每档给 `endpoint`（provider 名）、`model`、`effort` 三栏。注释即不启用，人揭开并改掉示例值就生效；占位符本身由 `the_template_hands_a_building_a_working_routing_placeholder` 钉住——模板照读是 `None`，揭开后是本读者接受的策略。
- **本版不加结构化表单**：另立一帧与一棵 TOML 写者，会让同一个键有两种写法，而现有规则页已经带着基线、求值与端点簿校验三道守卫；占位符解决“不知道写什么”，结构化编辑器留给真有批量改法需求的那天（D90 的重开参数）。

验收：city 的解析与结构拒绝测试、Lean 的 `route_existing_session`／`route_without_policy`／`route_recommendation_allowed`／`route_recommendation_within_max`；accounting 经真实 `delegate` 覆盖三档、无难度缺省、已有 Session 不换模型、无策略旧行为与开始前拒绝。
-/

/-! D116 checkpoint 的熵允许清单写在楼规里，且只收短路径

**决定**：`checkpoint_entropy_allow` 是 `RULES.toml` 的可选键，元素是仓库相对路径；不把命中正文或摘要写进清单。它只豁免熵命中，形状拒绝没有对应的放宽键。

**理由**：这份文件由 User 写、agent 改不到，并且已有整份求值与基线写门；把豁免放在普通工作文档里会让被扫描的 run 自己放宽判定。路径足以指出误报所在的文件，又避免把长触发串复制进进历史的治理文件。易触发的长例句留在本就不进历史的 `Memo.md`、`Roadmap.md` 或 `JOB.md`；`SPEC.md` 与 `RULES.toml` 只用短名引用它们。

**被否**：允许任意字符串（把真实未知密钥永久豁免且把正文写进历史）；路径前缀或 glob（一个短目录名会豁免过宽）；另建一份可写配置（同一规则出现两个所有者）。

**重开参数**：出现必须按单个命中而非单文件豁免的真实场景，并且能用一个不含正文也不永久保存秘密指纹的身份表达它时。
-/

/-! D90 工作区策略追加进 RULES.toml，而不是另建模型默认值

**决定**：`[workspace_models]` 是楼规的可选末节，复用整份楼规的求值、基线写门与治理摘要；不把推荐写进 `[model] name`，也不改全城 `main`。

**理由**：工作区允许什么与三档推荐是一张必须原子校验、原子保存的策略；楼规已有这条写路径。另建文件会让楼规与策略各自留下版本和失败路径，改全城 `main` 则会把一次子任务的选择泄漏给无关 Session。

**被否**：给 `delegate` 再加 model／effort 参数——父 Session 就会同时回答难度、模型与价格，和城市策略形成三套可能矛盾的答案；房间级策略——第一版没有房间级场景，Session 冻结与人工 `Dispatch` 已覆盖单次指定；为这张表另立结构化帧与第二棵 TOML 写者——同一个键有了两种写法，而模板里的注释占位已经回答了“不知道写什么”。

**重开参数**：人要求在页面上一次改三档而不碰其余楼规文本，且规则页的基于求值的拒词已不足以给出下一步时，再议结构化帧，且它只能经 `write_rules_against` 这一扇门落盘。
-/

/-! D2 定规：读界在调用时按目标所在楼现读规则

**决定**：读界的规则不在派活时冻结。`may_read` 的第三个参数是一个闭包，只在目标出了读者本楼时调用，调用时读目标所在楼的 `RULES.toml`；一次 `search` 不带路径时，城根下每走进一栋他楼调一次。

**理由**：两条轴都站在这一边。延迟：派活时冻结一张全城表要为每一栋楼读一次规则，代价随楼数线性增长，而绝大多数 run 从不读他楼；现读把代价放到真的跨楼的那一次读上，本楼的读零次读盘。安全：派活之后才建起来的机密楼、派活之后才改成 `confidential = true` 的楼，现读立刻关上；冻结表在 run 冻结之前一直按旧答案放行，而那正是朝宽松一侧失败。

**被否**：派活时冻结一张「哪些楼机密」的表，与写域、工具表一同冻结。它让 `may_read` 成为只吃值的纯函数，代价是上面两条；若冻结的是「哪些楼开放」，新楼会被关上，但派活代价不变。

**重开参数**：一次跨楼读的规则读取（一个小文件加一次 TOML 求值）相对该次 `read` 本身的读盘不再可忽略时——例如城的规则搬进一张随账本折叠的内存表，读规则不再触盘——冻结与现读的延迟差消失，可以重议；安全那一条不随它消失，重议时须给出新楼与改规则两种情形下仍然关上的办法。
-/

/-! D3 定规：一个 run 内按 (mtime, len) 留住他楼的规则

**决定**：读界的闭包持有一份 `RulesCache`，一个 run 一份。同一栋他楼的规则，文件的 mtime 与长度都没变时不再读盘求值；任一变了就重读。

**理由**：一次不带路径的 `search` 在城根下走进每一栋他楼，一个 run 反复读同一栋楼，每次都读一个小文件再做一次 TOML 求值；stat 一次的代价低于读加求值。D2 的安全一条仍然成立：改规则就改了文件，mtime 随之前进，下一次 stat 就重读；把 `confidential = false` 改成 `true` 连长度也变了，所以即便 mtime 的粒度粗到同一刻内写两次，这一次翻转也逃不过长度。

**被否**：按内容哈希作键——要先读完整个文件，省下的只剩求值；run 之间共享一份——run 的寿命是缓存失效的天然边界，跨 run 的表要自己回答何时丢，而多出来的只有第一次读。

**重开参数**：若某个文件系统的 mtime 粒度粗到同一刻内能写出等长而意义不同的规则并且真有人这样改，键要加上内容哈希或 inode 变更计数。
-/

/-! D10 写入限制随派活走，不进 `RULES.toml`

**决定**：「只读可新建」是一次派活的选择（`RunPolicy.write`），楼的 `RULES.toml` 不加对应的键；楼的写域是一次 run 的上限，派活只能收窄它（§8-32）。

**理由**：今天要「只新建」的是人对某一次活的谨慎——让一个 agent 起草新文件而不碰已有的——不是一栋楼的性质。若楼与派活各有一个限制，就要再定一条「两者取更严」的规则，并让页面、状态工具与回放都说清楚一次 run 的限制来自哪一处；只有一个来源时，账上的 `run_started.policy` 就是全部答案。

**被否**：`RULES.toml` 加 `limit = "full" | "create"`，与派活取更严的一个：一个事实有两个来源，而今天没有一栋楼需要它。

**重开参数**：出现一栋每次 run 都必须只新建的楼（例如只收稿件的投稿楼），且人要求不依赖每次派活都选对时。
-/

/-! D27 `egress` 不写是不限主机，写了才判

**决定**：`RULES.toml` 的 `egress` 是可选键。不写，楼不受域名表约束，浏览器到哪里只由 `browser`／`usersbrowser` 两个开关决定；写了，城在每次浏览器调用前按表判它点名的主机，表外的主机以 `E_GATE_DENIED` 作工具结果回流；写成空表，楼不到任何公网主机。机密楼不读这一键，恒取空表。新楼的模板不写这一键，只留一行注释给出写法。

**理由**：域名表是人选择收紧一栋楼的手段，不是每栋楼都要回答的问题。缺省取「空表＝不出公网」时，每一栋开了 `browser = true` 却没列域名的楼都会在升级后失去浏览器，人得先读懂拒绝才知道去补哪一行；缺省取「不限」时，没写这一键的楼行为不变，写了表的楼得到它以为已有的约束。判定是对一张随 run 冻结的短表做后缀比对，不读盘、不解析域名、不多一次往返，所以加这道门不给任何调用加可测的延迟。

**被否**：沿用模板里的 `egress = []` 并把空表读作「不限」——空表读作放行就是隐私设置朝宽松一侧失败，与本文件其余各键的方向相反，也与 `kernel::EgressAllowlist` 写明的「空表＝不出公网」冲突。

**重开参数**：出现一种默认就该关网的楼（例如只读城内资料、从不需要公网的楼），且人要求新楼一律从「不出公网」开始时。
-/

/-!
## 模型：机密楼没有出路，写域止于本楼，治理文件在一切写域之外，留住的规则就是现读的规则

`evaluate` 读的是 `Written`（`RULES.toml` 的每个键一个字段）；模型只留下判定用到的字段：`confidential` 与 `write` 没有缺省（缺即拒），出网域名、`browser`、`usersbrowser`、`desktop` 四种出路，写前缀。TOML 的文法、`deny_unknown_fields` 与前缀的地址文法是 serde 与 `Address::parse` 的，不在模型里。

* 不说 `confidential` 或 `write` 的规则被拒（`a_rules_file_that_does_not_say_confidential_is_refused`），隐私设置与写域都不朝宽松一侧取缺省。
* 求值通过的机密楼四种出路一条也没有（`a_confidential_building_has_no_way_out`）；普通楼可以列出网域名（`example`），所以上面的拒绝不是拒绝一切换来的。
* 机密楼的写域里每个前缀都在本楼之内（`a_confidential_domain_stays_in_its_building`）；没写前缀的楼写域就是它自己（`no_prefix_means_the_building_alone`）。
* 楼的 `RULES.toml` 与 `DESKTOP.toml` 住在它的保留子树里，没有写域够得到（`the_rules_are_out_of_every_write_domain`），这条由 kernel 布局的 `governing_files_are_reserved` 给出，这里只把它用到这两份文件上。
* `RulesCache` 在文件的戳不变时交回留着的规则；只要同一个戳读出同一份规则，留着的答案与现读的答案相同，且失败从不被留下（`kept_rules_answer_what_a_read_would`，D3）。这条前提就是 D3 的重开参数：戳相同而字节不同的文件系统让它不成立。
-/

namespace City.Policy

open Kernel.Address
open Kernel.Layout

/-- `usersbrowser` 一键的三种读法。 -/
inductive Granted where
  | Switch (on : Bool)
  | At (address : String)
  deriving DecidableEq, Repr

/-- `DomainReach`：`write` 一键。 -/
inductive DomainReach where
  | Everything
  | Documents
  deriving DecidableEq, Repr

/-- `Written` 里判定用到的字段；`none` 是这一键没写。 -/
structure Written where
  confidential : Option Bool
  write : Option DomainReach
  egress : Option (List String)
  browser : Bool
  usersbrowser : Granted
  desktop : Bool
  prefixes : List Address

/-- 求值通过的规则。 -/
structure BuildingRules where
  addr : Address
  confidential : Bool
  reach : DomainReach
  egress : Option (List String)
  browser : Bool
  usersbrowser : Option Granted
  desktop : Bool
  prefixes : List Address

/-- 拒绝的种类：缺一个没有缺省的键，或两个各自合法的设置不能同时成立。 -/
inductive Refused where
  | Missing
  | Contradiction
  | Outside
  deriving DecidableEq, Repr

/-- `usersbrowser` 读成什么：`false` 是没有这件工具。 -/
def usersbrowserOf : Granted → Option Granted
  | .Switch false => none
  | granted => some granted

/-- 规则要了哪一种出路：出网域名、城的浏览器、人的浏览器、桌面。 -/
def wayOut (written : Written) : Bool :=
  !(written.egress.getD []).isEmpty || written.browser || (usersbrowserOf written.usersbrowser).isSome ||
    written.desktop

/-- `evaluate`：缺键即拒；机密楼要任何一种出路即拒。 -/
def evaluate (addr : Address) (written : Written) : Except Refused BuildingRules :=
  match written.confidential, written.write with
  | some confidential, some reach =>
    match confidential && wayOut written with
    | true => .error .Contradiction
    | false => .ok ⟨addr, confidential, reach, if confidential then some [] else written.egress,
      written.browser,
      usersbrowserOf written.usersbrowser, written.desktop, written.prefixes⟩
  | _, _ => .error .Missing

theorem a_rules_file_that_does_not_say_confidential_is_refused (addr : Address) (written : Written)
    (unsaid : written.confidential = none ∨ written.write = none) :
    evaluate addr written = .error .Missing := by
  rcases unsaid with unsaid | unsaid <;> simp [evaluate, unsaid] <;>
    cases written.confidential <;> rfl

theorem a_confidential_building_has_no_way_out (addr : Address) (written : Written)
    (rules : BuildingRules) (evaluated : evaluate addr written = .ok rules)
    (confidential : rules.confidential = true) :
    rules.egress = some [] ∧ rules.browser = false ∧ rules.usersbrowser = none ∧
      rules.desktop = false := by
  unfold evaluate at evaluated
  split at evaluated
  · rename_i c reach _ _
    split at evaluated
    · cases evaluated
    · rename_i fine
      cases evaluated
      simp only at confidential
      subst confidential
      simp_all [wayOut]
  · cases evaluated

/-- 一栋普通的楼可以列出网域名：机密楼的拒绝不是拒绝一切换来的。 -/
example : (evaluate ⟨["lab"]⟩ ⟨some false, some .Everything, some ["example.org"], false, .Switch false,
    false, []⟩).toBool = true := rfl

/-- `BuildingRules::write_domain` 的前缀：机密楼的前缀出了本楼即拒；一条都没写就是整栋楼。 -/
def writeDomain (rules : BuildingRules) : Except Refused (List Address) :=
  if rules.confidential && rules.prefixes.any (fun pre => !is_within pre rules.addr) then
    .error .Outside
  else if rules.prefixes.isEmpty then .ok [rules.addr]
  else .ok rules.prefixes

theorem a_confidential_domain_stays_in_its_building (rules : BuildingRules)
    (prefixes : List Address) (built : writeDomain rules = .ok prefixes)
    (confidential : rules.confidential = true) :
    ∀ pre ∈ prefixes, is_within pre rules.addr = true := by
  unfold writeDomain at built
  split at built
  · cases built
  · rename_i inside
    simp only [confidential, Bool.true_and, List.any_eq_true, Bool.not_eq_true',
      not_exists, not_and, Bool.not_eq_false] at inside
    split at built
    · cases built
      intro pre found
      simp only [List.mem_singleton] at found
      subst found
      exact is_within_refl _
    · cases built
      exact inside

theorem no_prefix_means_the_building_alone (rules : BuildingRules) (none_written : rules.prefixes = []) :
    writeDomain rules = .ok [rules.addr] := by
  simp [writeDomain, none_written]

theorem the_rules_are_out_of_every_write_domain (names : Names) (protected_name : String → Bool)
    (protects : protected_name names.reserved = true) (building : Address) (rules_file desktop_file : String) :
    is_reserved protected_name (relative building [names.reserved, rules_file]) = true ∧
      is_reserved protected_name (relative building [names.reserved, desktop_file]) = true :=
  ⟨governing_files_are_reserved names protected_name protects building rules_file,
    governing_files_are_reserved names protected_name protects building desktop_file⟩

/-- `RulesCache` 为一栋楼留着的东西：上次读到的戳与那次的规则。 -/
abbrev Kept (S R : Type) := Option (S × R)

/-- `RulesCache::load`：没有戳就现读、什么都不留；戳与留着的相同就交回留着的；否则现读，读成了才按这次的戳留下，失败不留。 -/
def load {S R E : Type} [DecidableEq S] (kept : Kept S R) (stamp : Option S) (fresh : Except E R) :
    Except E R × Kept S R :=
  match stamp, kept with
  | none, _ => (fresh, kept)
  | some now, some (held, rules) =>
    if held = now then (.ok rules, kept)
    else match fresh with
      | .ok rules => (.ok rules, some (now, rules))
      | .error e => (.error e, kept)
  | some now, none =>
    match fresh with
    | .ok rules => (.ok rules, some (now, rules))
    | .error e => (.error e, kept)

/-- 留着的规则是它那个戳现读会得到的规则。 -/
def Sound {S R E : Type} (readAt : S → Except E R) (kept : Kept S R) : Prop :=
  ∀ held rules, kept = some (held, rules) → readAt held = .ok rules

theorem kept_rules_answer_what_a_read_would {S R E : Type} [DecidableEq S] (readAt : S → Except E R)
    (kept : Kept S R) (sound : Sound readAt kept) (now : S) :
    (load kept (some now) (readAt now)).1 = readAt now ∧ Sound readAt (load kept (some now) (readAt now)).2 := by
  cases kept with
  | none =>
    cases fresh : readAt now <;> simp [load, fresh, Sound]
  | some entry =>
    obtain ⟨held, rules⟩ := entry
    by_cases same : held = now
    · subst same
      have kept_is_read := sound held rules rfl
      simp [load, kept_is_read, sound]
    · cases fresh : readAt now <;> simp [load, same, fresh, Sound]
      all_goals exact sound held rules rfl

end City.Policy

/-! 工作区路由的纯模型；Rust 的 `policy::workspace` 与 `agree_to_work` 共用这里的判定语义。 -/
namespace City.Policy.Workspace

open Kernel.Delegation

/-- 同一 provider 上的同一模型；两个字段合起来才是身份。 -/
structure ModelRef where
  endpoint : String
  model : String
  deriving DecidableEq, Repr

/-- 一档推荐；`effort` 在模型中是可比较的档位序号，不包含旧账本兼容词 `none`。 -/
structure Recommendation where
  model : ModelRef
  effort : Nat
  deriving Repr

/-- 三档齐全的一张工作区策略。 -/
structure Policy where
  allowed : List ModelRef
  maxEffort : Nat
  easy : Recommendation
  medium : Recommendation
  hard : Recommendation
  deriving Repr

/-- 父 Session 给出的难度命中哪一档；未给难度是 `medium`，不是另一次判断。 -/
def recommendation (policy : Policy) (difficulty : Difficulty) : Recommendation :=
  match difficulty with
  | .Easy => policy.easy
  | .Medium => policy.medium
  | .Hard => policy.hard

/-- `evaluate` 与端点簿校验共同建立、路由依赖的结构前提。 -/
def Valid (policy : Policy) : Prop :=
  policy.allowed ≠ [] ∧ ∀ difficulty,
    (recommendation policy difficulty).model ∈ policy.allowed ∧
      (recommendation policy difficulty).effort ≤ policy.maxEffort

/-- `none` 是继续走旧冻结／`main` 路径；`some` 才是城市替一次新子 Session 作出的推荐。 -/
def route (existingSession : Bool) (policy : Option Policy) (requested : Option Difficulty) :
    Option Recommendation :=
  match existingSession, policy with
  | true, _ | _, none => none
  | false, some policy => some (recommendation policy (requested.getD .Medium))

/-- 已有 Session 不受这次难度影响。 -/
theorem route_existing_session (policy : Option Policy) (requested : Option Difficulty) :
    route true policy requested = none := by
  cases policy <;> rfl

/-- 无策略时线上行为保持原样。 -/
theorem route_without_policy (requested : Option Difficulty) :
    route false none requested = none :=
  rfl

/-- 有效策略选出的模型一定在 allowed 中。 -/
theorem route_recommendation_allowed (policy : Policy) (requested : Option Difficulty)
    (valid : Valid policy) :
    ∃ selected, route false (some policy) requested = some selected ∧
      selected.model ∈ policy.allowed := by
  refine ⟨recommendation policy (requested.getD .Medium), rfl, ?_⟩
  exact (valid.2 (requested.getD .Medium)).1

/-- 有效策略选出的 effort 一定不高于 max_effort。 -/
theorem route_recommendation_within_max (policy : Policy) (requested : Option Difficulty)
    (valid : Valid policy) :
    ∃ selected, route false (some policy) requested = some selected ∧
      selected.effort ≤ policy.maxEffort := by
  refine ⟨recommendation policy (requested.getD .Medium), rfl, ?_⟩
  exact (valid.2 (requested.getD .Medium)).2

/-- 不写难度不是空路由：配置了策略的新子 Session 落到 medium 推荐。 -/
theorem route_defaults_to_medium (policy : Policy) :
    route false (some policy) none = some policy.medium :=
  rfl

end City.Policy.Workspace
