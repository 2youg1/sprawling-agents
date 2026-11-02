-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::connectors

规定 `crates/accounting/src/connectors.rs` 的端口 `Connectors`、它的生产适配器 `crates/accounting/src/worker/mcp.rs`，以及经这个端口接外部搜索服务的工作台工具 `web_search`（§8-35）。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住；常驻连接的身份与失效由 `crates/accounting/src/worker/mcp/tests/credentials.rs` 与同目录的 `reconnect.rs` 守住。`web_search` 的换号与重发是 kernel 的 `account_recovery` 状态机的性质，它的证明住 kernel 的规格，本分部只写这件工具怎样执行那台状态机交出的每一步。
-/

/-!
### 8-2 accounting::connectors（形状 3 端口）

```rust
pub trait Connectors {
    /// # Errors
    /// Whatever starting the server, its handshake or its listing
    /// refuses.
    fn connect(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError>;

    /// Drops every resident connection whose declaration carries
    /// `reference`, so the next `connect` redeems the reference again.
    fn invalidate(&self, reference: &kernel::SecretRef);
}

pub enum Reached {
    Connected(agent_protocols::Handshake),   // 这次调用启动或打开、并握过手
    Resident,                         // 早先一次调用连上、仍在运行
}
```

```rust
// accounting::worker::mcp（形状 4 适配器）
#[derive(Default)]
pub(crate) struct Residents;              // 生产：McpLink::open + agent_protocols::handshake + tools/list，连接在 run 之间保持（`crates/sprawling/Spec.lean` §8-4）
impl RunWorker {
    pub fn with_connectors(self, connectors: Box<dyn accounting::Connectors + Send + Sync>) -> RunWorker;
}
```

- **失败**：原样传 `McpLink::open`、`agent_protocols::handshake` 与 `agent_protocols::tools_from` 的 `AxError`。worker 把失败写进 diagnostics、把这个 server 留在外面，run 照常开始；端口不另造错误码。
- **`confidential` 原样传给 `McpTool::new`**：那是工具层的权威。worker 在机密楼里一个 server 都不启动，所以生产路径上它总是 `false`；它仍在签名里，是为了任何实现都不能造出一个绕过工具层拒绝的工具。
- **端口有状态，可被几条线程同时问**：生产适配器把连上的 server 按声明与 run root 留在表里，下一次 dispatch 直接拿它的工具；`McpLink::has_ended` 判定子进程已退出或 HTTP 会话已失效的那一行在这里被丢掉，经 `McpLink::open`、唯一 `handshake` 与 `tools/list` 重连，原调用不在这里重发；HTTP 失效共享于工具与表持有的链接克隆，无 session id 的 404 与效果未知的答复不因此重连。`connect` 取 `&self`、实现是 `Sync`，因为准备派活的 lane 各自问同一张表（`crates/sprawling/Spec.lean` §8-113），表自己按键上锁（`crates/sprawling/Spec.lean` §8-4）。
- **换了 Key 的连接不再被复用**：一条常驻连接在打开时兑付 header 里的引用，之后一直带着那一刻的值。`PutSecret` 在 vault 成功写入之后、写 `secret_captured` 之前对这个引用调 `invalidate`，账本那一步被拒也照样已经丢掉旧连接；同一个引用换了值、账号 id 不变，所以 Session 的账号亲和不受影响。`ForgetSecret` 删成之后同样对这个引用调 `invalidate`（`crates/accounting/spec/Worker.lean` §8-37）。
- **固定值**：`RunWorker::new` 与 `over` 装上 `Residents::default()`；`with_connectors` 是唯一换掉它的门。
- **依赖**：本 crate 因此依赖 `agent_protocols`（ARCHITECTURE.md §3 的 `depmap`）。
-/

/-!
### 8-35 `web_search`：城里的网络搜索工具（`workbench::tools::search`，形状 4 适配器）

```rust
// accounting::worker::workbench::tools::search
impl Laying {
    // 机密楼与 `Off` 答 None；其余对 `site.config.search` 调一次 city 的 `search_supplier`
    pub(super) fn search_tool(&self, site: &Site) -> Result<Option<WebSearch>, AxError>;
}
pub(super) struct WebSearch { /* supplier: SearchSupplier、meta、root、vault、connectors —— 私有 */ }
// kernel::Tool。名字固定为 web_search；参数 { query, objective?, num_results? }；
// 答远端 tools/call 交回的 content，原样
```

- **有没有这件工具**：机密楼不注册它，因为一次搜索把 run 写下的文字送出楼外，而机密楼的数据可入不可出（`crates/city/spec/Policy.lean` 的 confidential 四条）；`FrozenConfig.search` 是 `Off` 时也不注册。`search_supplier` 拒了（冻结值里的 `selected` 不在列表中，只有绕过解析造出的值会这样）时写进 diagnostics、这次 run 没有这件工具，与一台起不来的 MCP server 同一种处理。供应方怎么选、缺省是哪一家，是 `crates/city/spec/ConfigLayers.lean` §8-4c 的答案，本工具不另判。
- **元信息**：`effect = Connector { label: supplier.id }`，出网门按这个 label 判；`cost_tier = Heavy`；`timeout = EXTERNAL_CALL_PATIENCE`；`temporal = Timestamped`；`render = Generic`。参数表在 Run 起点由冻结的那一家推出，run 内不变：`query`（非空 string）恒必填；`objective`（非空 string）只在这一家映射了 `objective_field` 时出现，且必填；`num_results`（正整数）只在映射了 `count_field` 时出现，可缺。模型因此看不到一个这家收不了的参数。
- **一次调用**：从 kernel 的 `account_recovery` 取当前账号；把这家写成一条 `McpServer { label: supplier.id, transport: Http { url, headers } }`，带引用的账号给一对 `(account.header, reference)`，匿名账号不给 header；经 `Connectors::connect` 取工具；在 `tools/list` 里找 `remote`。找不到，或映射过不了远端的 `inputSchema`（映射的每个参数都要在 `properties` 里，远端 `required` 的每个参数都要有映射），在发 `tools/call` 之前以 `E_TOOL_UNAVAILABLE` 拒，恢复语指向设置页里这一家的映射；这一步与设置页保存时的校验是同一个函数。过了就按映射改名参数、发一次 `tools/call`，结果原样交回，本工具不声称能从结果里抽出引用。
- **账号选择与恢复只有一处权威**：`supplier.accounts` 的顺序就是优先级，一轮从第一个凭据可兑付的账号开始；每次失败之后，本工具把失败交给 `account_recovery` 问下一步，只执行它交回的那一步：`Resend` 在同一账号上再发，等待取 Watchdog 同一张退避表并尊重 `retry-after`，受这次工具调用的期限与取消约束；`Switch { to }` 换成那个账号重写 header；`Stop` 把对应的错误作为失败的工具结果交给模型——全部账号用尽时是 `E_PROVIDER_ACCOUNTS_EXHAUSTED`，请求错误与效果未知时是原错误（效果未知的标记保留），vault 锁定或不可用时是 vault 的错误。每个账号的重发预算取 `account_retries` 的那一个缺省值，搜索不另设一个数。一次失败要不要换号，由构造这个错误的那一处标在 `AxError` 上（`crates/gateway/spec/Endpoint/Failure.lean` 的分类），本工具不重读状态码。
- **一轮受这次工具调用的期限约束，不受 `Halt` 约束**：工具层没有调用中的取消——`Halt` 在一个 wave 的两次工具调用之间生效（`runtime::turn` 的 `execute_concurrent` 每次调用之前问一次），够不着一次在飞的调用。所以一轮从开轮起以元信息的 `timeout`（`EXTERNAL_CALL_PATIENCE`）为期限：`Resend` 的等待若会越过期限就不等，把要求等待的那次失败作为结果交回；`Switch` 不等，期限已过时同样停下。已经发出的一次尝试受它自己的期限约束，所以一轮至多约两个期限长，`Halt` 至多晚这么久生效，与任何一次 MCP 调用同一量级。等待时长取 `runtime::backoff_ms`（`crates/runtime/spec/Watchdog.lean` §8-9 的同一张表，含对端的 `retry-after`），以这个 run 的 id 为种子，连续计数在换号时归零。搜索没有人填的 `request_max_retries`，一轮的上限取 `UntilHalted`：多账号时止住它的是 n·(1＋k) 那道界，单账号时是期限。
- **哪些失败换号、哪些原号再发，由传输标在错误上**（`crates/agent_protocols/Spec.lean` §8-17 的失败码）：HTTP 与 SSE 答 401 是这个账号的 Key 被拒，标 `Advance`；408、429、502、503、504 是对端这次没有处理，标可重试（`Yes`）；答复丢失标效果未知（`Unknown`）；其余状态、连不上、`tools/list` 里没有 `remote` 与映射不合都是 `No` 且 `Keep`，一轮以 `Refused` 停下。开轮时凭据不可兑付的账号不在可选之列（kernel D55），全部不可兑付时一次也不发，以 `E_PROVIDER_ACCOUNTS_EXHAUSTED` 交回。
- **结果未知的请求只在原账号上重发**：`tools/call` 已发出而答复丢了，在预算内只在同一账号上再发，从不换号，规则与模型请求相同；搜索是只读的，重复的代价是计费，换号会让另一个账号也计一次费，而丢失答复不证明是账号的问题。
- **不做 Session 亲和**：每次调用都从列表第一个可兑付的账号开始一轮，不记 Session 绑定。理由：搜索没有按账号隔离的 prompt cache，不出错时按顺序选号本来就总落在同一账号；记录绑定要在 Ledger 里多一个字段，而它换不来任何收益。
- **连接的身份含所选账号的引用**：`Residents` 按声明与 run root 留连接，声明里带着 header 的引用，所以两个账号恒不共用一条连接，换号就是换一条连接；同一个引用换了 Key 由 §8-2 的 `invalidate` 丢掉旧连接。
- **不回落**：所选那一家连不上、`tools/list` 里没有 `remote`、调用失败，都以失败的工具结果交回，不换到缺省那一家或列表里的另一家（city D25）。
- **Ledger 里没有 Key**：这件工具的调用与结果照任何 Connector 工具入账；错误的 subject 只写供应方 id 与账号 id，不写 Key，也不写引用。
-/

/-! D52 `web_search` 只执行 `account_recovery` 交回的步骤，自己不选号、不重发

**决定**：`web_search` 的账号选择、同账号重发、换号与停止全由 kernel 的 `account_recovery` 决定，本工具只把每次失败交给它、照它交回的 `Step` 做；等待用 Watchdog 的退避表。搜索账号不做 Session 亲和。

**理由**：模型请求与搜索请求守同一组由人定下的规则：可恢复错误在原账号重试一到两次再换号，账号错误直接换号，请求错误不重发，结果未知不换号。两条路径各写一个循环，就有两处可以分开漂移的规则；kernel 是 runtime 与本 crate 唯一共同的内层，规则放在那里，两条路径各自只驱动它。亲和是为了保住按账号隔离的 prompt cache，搜索没有这种 cache。

**被否**：①工具自己挑「第一个可兑付的账号」、对 `Retry::Yes` 自己再发一次：这是第二处选号和第二个重试循环；②搜索也记 Session 绑定：要多一个 Ledger 字段，换不来 cache 收益。

**重开参数**：某个搜索供应方按账号缓存或按账号计配额，使得在不出错时留在同一账号有可度量的收益。
-/

/-! D116 lane 在注册前比较 pin，漂移经 `digest_invalidated` 去重入账，页面不主动握手

**决定**：`Laying` 在 lane 连接服务器并取得 `tools/list` 后，先对定义与工具做 digest 比较，再把相等工具交给 `runtime::catalog`。首次迁移的旧引用把当时配置与列表写成初始 pin；已有 pin 的新工具、改工具与消失工具不注册。skill 从 pin 的 CAS manifest 读取，外部候选只在初次准入或 Accept 时复制。漂移先经共享 `DriftBook` 判重，再以 `digest_invalidated` 追加；book 在开城时从历史重建，追加成功才登记，失败不吞掉下一次记录机会。

**理由**：握手只能在 lane 做，避免阻塞单写者；pin 持久化在配置而不是只在连接里，常驻连接复用旧列表时仍须与批准版本比较。页面继续读最新 `prompt_assembled`、漂移事件与 CAS 定义，不为回答 `Query::Catalog` 启动服务器或浏览器。

**被否**：每次 run 都追加漂移——同一未处理变更会淹没 Ledger；只在内存去重——重启后重复；查询时主动刷新——只读页面会产生外部副作用。重开参数：需要表示“服务器已恢复、同一漂移重新出现”的完整状态机时，将活动漂移与历史漂移分列，而不是删除历史去重键。
-/

/-! D119 保存拒绝无人能读的外部 pin：不等于保存时刻扫描值的 pin，其字节必须已在 CAS

**决定**：`ConfigureBuilding` 写 `[skills.pins]` 之前，对每个不等于保存时刻扫描值的 pin 做一次 `get` 全读校验；对象不在 store 则以 `E_VERSION_CONFLICT` 拒绝整次保存，pins 与其余各面都不写。等于扫描值的外部 pin 仍由准入复制当场存入并以 hash 复核；内部架的 pin 由安装的 CAS 不变量覆盖，不做这次读取。

**理由**：页面 Accept 的 candidate 来自回答那次扫描。外部目录在页面读取与保存之间被改写（或整个被撤走）时，写下的 pin 指向从未准入 store 的版本：下一次 `Query::Catalog` 读 pinned 文档、下一个 run 经 `pinned.or(found)` 准入 Reading Room，都在缺失对象上失败，整面板不可用且 UI 没有修复路径。校验把「留住已准入的旧版本」（keep，字节已在 CAS，照常保存）与「钉住没人准入过的版本」（页面读到的旧 candidate，拒绝）分开：前者是 D116 的承诺，后者是本决定拒掉的缺陷。

**被否**：回答侧容忍缺失对象（只显示 drift、不显示 pinned 文本）——run 侧仍在同一缺失上失败，缺陷只是从保存门挪到读取门；保存时无条件重读全部 pin——等于扫描值的 pin 由准入路径复核，不需要第二遍。

**重开参数**：CAS 长出回收后，「pin 的内容被回收」需要与「pin 从未准入」不同的答复与恢复文案，校验口径随之改。
-/

/-! D120 定义漂移记录在案但不拦工具：信任边界是改城市定义的那只手

**决定**：definition pin 覆盖 `McpServer` 规范化 JSON（label、command/args、url、env 与 headers 的引用）的 BLAKE3。lane 比较出定义漂移时只记 `digest_invalidated`（subject `server`），该服务器的工具照常注册，工具级 pin 仍逐件比较；command/url 变了而工具 schema 未变时，引用它的楼会在新传输上继续调用这些工具。会话 `CATALOG.toml` 冻结的是引用与 pin（选择），定义本身在每次 run 按 label 解析到配置梯当前的一份；会话里的定义 pin 用于发现漂移，不用于选定义。

**理由**：工具 pin 挡的是模型可见面（名字、描述、schema）的静默变化；传输面（换 endpoint、改启动命令）由配置梯的所有者负责，他改城市定义时已经站在所有引用楼的信任根上。定义漂移做成拦截而没有对应的 Accept 面，会让一次城市级修改打断所有引用楼且无法在 UI 收敛——被否的正是这种不可恢复的拦截。

**被否**：定义不匹配即整服务器不注册——没有 Accept 面的拦截不可恢复；按 session 冻结的定义选版本——配置梯只有「当前」一份定义，存档第二份要另一个存放处与过期策略。

**重开参数**：目录页有了 server 定义行的 Changed 与 Accept 之后，拦截与放行可以按楼选择，本决定重开。
-/

/-! D121 目录一次保存过两道写门：每道文件级原子，半截组合都可读，下一次保存收敛

**决定**：统一目录保存的落盘顺序是 CONFIG.toml 的各面（`[catalog]`、`[[mcp_ref]]`、`[skills.pins]`）在先，RULES.toml 的 reading room 在后；每道门走 `document::edit`（整文件临时件加 rename，文件级原子），两道门之间没有事务。任意半截都可读且语义明确：pins 落了而 admission 未落，回答把该技能显示为钉住但不准入（Off），run 不读它；admission 落了而 pin 未落，run 按候选准入并以 `migrate_skill_pin` 补 pin。崩溃或 I/O 失败后的下一次保存以两门的当前内容为准收敛。

**理由**：reading room 的权威是 RULES.toml（政策面），pins 的权威是 CONFIG.toml（配置面），把任何一边搬进另一边都要迁移已发布的文件并制造第二个权威；两文件事务需要 journal，而半截状态都已可读、可判、可由下一次保存修复，复杂度换不来可观察的收益。

**被否**：把 reading room 并入 CONFIG.toml——政策的权威随之分裂；先写 RULES 再写 CONFIG——半截（准入了无 pin）同样存在，只是顺序换了；两文件写后互验回滚——回滚本身又是一次非原子的两文件操作。
-/

/-! D122 connector pin 的 digest 走 serde_json 确定性再序列化，不另设 canonical 方案

**决定**：定义与工具的 pin 都是 `serde_json::to_vec` 字节的 BLAKE3。workspace 不启用 serde_json 的 `preserve_order`：`Payload` 的 map 按键排序（BTreeMap），浮点按 ryu 最短往返格式，同一解析结构恒得同一序列化字节。同一份 schema 两次 listing 即使原文键序不同，也得出同一 digest，不产生伪漂移；页面 Accept 把回答里的 digest 原样传回，不参与再序列化。

**理由**：pin 只在同一条管线内比较（lane 计算、CAS 存取、页面传回），不跨实现交换；要钉住的是语义结构而不是某家服务器的排版。RFC 8785（JCS）给出的确定性与现状等价，却多一个依赖和一份键序/数字规则的第二实现，反而多一处可漂移的面。

**被否**：手写 JCS——第二份规范化实现会与 serde_json 各自漂移；对 listing 原文字节取 digest——把服务器的排版变化误判成工具变化，正是本决定要消掉的伪漂移。
-/

/-! D4 `Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `agent_protocols::Outbound`

理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `agent_protocols`，不归 worker。
-/

/-! D5 `Connectors::connect` 取 `&self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`

理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。取 `&self` 而不是 `&mut self`，是因为 MCP 缺表时的连接在 lane 里做，几条 lane 借同一张表的 `Arc`；`&mut self` 会把所有 lane 的握手排成一队。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
-/
