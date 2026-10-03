-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

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

**`harness_minutes`**：一次 harness run 在这栋楼里最多跑多少分钟（`crates/sprawling/Spec.lean` §8-124），正整数；缺这一行读作 `HARNESS_MINUTES_DEFAULT`（60）。`0` 由 serde 在解析时拒（`NonZeroU32`）：一个到点即停的上限是笔误。它只管 harness：模型 run 的每一回合都经城自己的安全点，停一件事是 `cancel`、停一片是 `halt`，而 harness 的回合在城之外走完，一个既不说话也不结束的 harness 没有别的路让出车道（D8 (b)）。

`RulesCache`（`policy/cache.rs`，形状 1 判定）：一个 run 一份，读界的闭包持有它。`load` 先对 `RULES.toml` 做一次 stat，(mtime, len) 与上次读到的相同就交回留着的规则，不同或第一次就走 `load` 读盘求值并按这次 stat 的戳留下。文件不存在或 stat 失败时不留任何东西、每次都走 `load`，于是「没有 RULES.toml」与「被替代的旧文档」两条的答案与不缓存时逐字相同。锁中毒时同样退回 `load`。失败与 `load` 相同，失败不留。决定见 D3。

- **规则是一份 TOML，散文没有另起一份文件**：若是 Markdown，读者就得在**任意一行**上匹配 `confidential:`／`write:`／`review:`／`browser:`／`usersbrowser:`／`desktop:`，于是「How work is done here」里一句以 `desktop = true` 开头的话就授予了宿主机的桌面，而一栋没写 `write:` 的楼落到 `Everything`。两处都朝宽松的一侧失败，那是权限读者唯一不许失败的方向。改成 TOML 之后键只在文法给出键的位置成立，`deny_unknown_fields` 让拼错成为一条消息而不是一次静默缺席，`confidential` 与 `write` 都不再有缺省。**散文留在同一份文件里**，作 `does` 与 `conventions` 两个键：拆成两份文档同样能关掉撞键，代价是一栋楼有两种说法且可以互相矛盾。居民拿到的就是这份文件本身的字节，所以城判定的与 agent 读到的是同一串。

- **confidential 四条各有其守处**：模型池锁本地由 `gateway::endpoint` **在会泄漏的那一端**拒（`req.policy.confidential` 即拒，携三段式）；写域止于本楼子树由 `write_domain()` 在构造点拒；数据可入不可出归出网门；**楼里的字节楼外读不到**，归读界（下一条）。**把兜底放在会出事的那一层**，路由错了仍然拦得住。
- **读界：本楼全开，他楼非机密可读，机密楼对楼外全关**。判定只有一处：`kernel::address::may_read(reader_building, target, rules) -> ReadVerdict`（`crates/kernel/Spec.lean` §8-2），三臂 `Open`／`Confidential`／`RulesUnreadable(AxError)`。问它的是模型选路的唯一判定处 `runtime::tools::chosen_path`（`crates/runtime/Spec.lean` §8-30-1）：`read` 的路径、`search` 的起点、`search` 不带路径时在城根下走进的每一栋楼，以及经这两件工具读到的 transcript（`crates/runtime/Spec.lean` §8-32），都先过它。`rules` 是目标所在楼此刻的规则：装配层把 `city::Building::of(target)` 与 `policy::load` 接成一个闭包交给 `may_read`，**只在目标出了本楼时才调用**——本楼的读不读盘，他楼的读每次现读规则。规则读不出＝`RulesUnreadable`，一样关：读不出的那份规则可能写着 `confidential = true`，而隐私设置不得朝宽松的一侧失败（本节「没有 RULES.toml」那条同理）。**读界只管模型选的路径**：catalog 名是人在阅览室里准入的（`crates/runtime/Spec.lean` §8-29 起首），不经它；`exec` 在宿主机上跑的命令读得到盘上任何文件，那道墙要 OS sandbox，与「出网」那条的缺口是同一个缺口。
- **没有 RULES.toml 是普通楼；有而不声明是错误**：把隐私设置的默认值悄悄取成宽松的那一边，正是这整个面存在的理由。拼写不是 `true`／`false` 同样拒——读起来像笔误的隐私设置不得解析成许可。
- **confidential 楼声明越界前缀＝拒而不裁剪**：静默裁剪会让文件说一套、城做另一套；拒绝会指出该改哪一行。
- **无声明写域时默认只写本楼**：一栋楼至少能写自己，且不多。`prefixes` 里一条读不出的地址**传播而不跳过**——在读它的地方丢掉，一栋楼就写得比人授予的少，而这件事没有任何一处说出来。
- **`review = true` 是楼级开关**：开则每个 Run 得一棵自己的 worktree，写的东西在别人检查并 merge 之前对楼不可见。**默认关**，与 confidential 的「不声明即错」相反——隐私的默认值不得惄悄取宽，而审查纪律的默认值不得惄悄取严：一个人派一个 Agent 去改一行字并盯着看，应当看得到文件变化。拼写不是 `true`／`false` 同样拒。
- **`## Egress` 列可达域名**：`BuildingRules::egress()` 交 `kernel::gate::egress::egress_target` 判定。类型经 `kernel::EgressAllowlist` 重导出，住哪一簇文件是 kernel 内政（`gate::egress`，公共拼写不变）。**confidential 楼同时列域名＝矛盾，拒**——「数据可入不可出」是那个设置的含义，域名表写在它下面会逼读者自己去调和两句话。
- **执行点与缺口要分清**：provider 路径由 `endpoint` 的 confidential 拒守住；Agent 自己发起的出网是否被拦，取决于 exec 所在的那一臂 `runtime::tools::exec::Confinement` 是否承诺关网（`Guarantee::Network`，每一臂在工具描述里说出自己不承诺什么），浏览器则没有拦截处。判定在这里，拦截不在——不承诺关网的地方不要说「出网已管住」。
- **`usersbrowser` 一键同时是开关与地址**：值 `"ws://127.0.0.1:<port>/session"` 启用并声明地址，`true` 启用而地址未定（工具每次调用都得到门的问题），absent／`false` 即无此工具。**confidential 楼写这一键即拒**（`E_CONFIG_INVALID`）——附着读的是那个人浏览器里全部登录态，本楼的隔离在那一刻失效。地址的**语法**（`ws://`、主机形状）在此读一次并把 `url`／`host` 一起交出；**loopback 与否是 `kernel::gate::attach` 的政策**，语法不替政策作答。
- **`browser` 与 `usersbrowser` 是两个键**：前者是城自己拉起的浏览器（profile 按楼隔离），后者是人已经开着的那个（人的真 profile）。`browser` 不是 `usersbrowser` 的前缀截断——TOML 的键是文法给出的整体，两个设置因此互不误读。
- **`write_rules` 先求值再落盘**：一份写到一半就不再求值的治理文档会把它那栋楼一起带走。且**整份文档才是单位**：confidential 楼不得列域名，故两行可以各自合法而合在一起非法。
-/

/-!
### 8-11 楼的治理字节搬进它自己的保留子树（沉淀一处路径权威）

```rust
pub fn rules_path(city_root, addr) -> PathBuf;      // <building>/.sprawling/RULES.toml
pub fn agents_path(city_root, addr) -> PathBuf;     // <building>/AGENTS.md（项目的，故在保留区之外）
pub fn city_agents_path(city_root) -> PathBuf;      // <city>/AGENTS.md：城围起来的工作区自己的那份
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
### 8-34 页面写的两处：楼规带基线、城一层的两个键（`city::policy`、`city::config_layers::city_layer`，形状 4 adapter）

```rust
pub fn write_rules_against(city_root: &Path, addr: &Address, base: &str, text: &str) -> Result<BuildingRules, AxError>;
pub enum CitySetting { KeepWarm(KeepWarm), Effort(Effort) }
pub fn write_city_setting(city_root: &Path, setting: CitySetting) -> Result<(), AxError>;
```

- **楼规：先求值，再守基线。** `write_rules_against` 先 `evaluate(addr, text)`，拒了就什么都不写；通过之后经 `document::edit_against` 落盘，文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`。`write_rules` 不带基线，是 `rules` 工具 `propose` 一臂的写面；这一臂在效果层恒被拒（§8-2b，D1 定规），run 到不了它，所以楼规在生产里只经 `write_rules_against` 落盘（`accounting::worker::commanding::configure`，人经 `ConfigureBuilding`）。两扇门共用同一个求值器与同一个落盘函数，所以「盘上的楼规永远读得懂」只有一条规则。
- **城一层：两个键各一臂。** `write_city_setting` 经 `config_layers::write` 那一个读改写入口改城自己的 `CONFIG.toml`：`KeepWarm` 写 `[cache] keep_warm`，`Effort` 写 `[model] effort`；同一文件里别的键原样留着，写出的字节先过 `ConfigLayer::parse` 才落盘。城一层不是 session 的记录处，所以这里写 `[model] effort` 不碰 `[model] name`。
- 验收：`config_layers::city_layer` 的 `a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it`；楼规两道判定的组合在 accounting 的 `a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing` 里经真实命令观察。
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
  egress : List String
  browser : Bool
  usersbrowser : Granted
  desktop : Bool
  prefixes : List Address

/-- 求值通过的规则。 -/
structure BuildingRules where
  addr : Address
  confidential : Bool
  reach : DomainReach
  egress : List String
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
  !written.egress.isEmpty || written.browser || (usersbrowserOf written.usersbrowser).isSome ||
    written.desktop

/-- `evaluate`：缺键即拒；机密楼要任何一种出路即拒。 -/
def evaluate (addr : Address) (written : Written) : Except Refused BuildingRules :=
  match written.confidential, written.write with
  | some confidential, some reach =>
    match confidential && wayOut written with
    | true => .error .Contradiction
    | false => .ok ⟨addr, confidential, reach, written.egress, written.browser,
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
    rules.egress = [] ∧ rules.browser = false ∧ rules.usersbrowser = none ∧ rules.desktop = false := by
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
example : (evaluate ⟨["lab"]⟩ ⟨some false, some .Everything, ["example.org"], false, .Switch false,
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
