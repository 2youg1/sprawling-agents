-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::connectors

规定 `crates/accounting/src/connectors.rs` 的端口 `Connectors`、它的生产适配器 `crates/accounting/src/worker/mcp.rs`，以及经这个端口接外部搜索服务的工作台工具 `web_search`（§8-35）。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`accounting::connectors` 旁还没有测试。`web_search` 的换号与重发是 kernel 的 `account_recovery` 状态机的性质，它的证明住 kernel 的规格，本分部只写这件工具怎样执行那台状态机交出的每一步。
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
- **换了 Key 的连接不再被复用**：一条常驻连接在打开时兑付 header 里的引用，之后一直带着那一刻的值。`PutSecret` 在 vault 成功写入之后、写 `secret_captured` 之前对这个引用调 `invalidate`，账本那一步被拒也照样已经丢掉旧连接；同一个引用换了值、账号 id 不变，所以 Session 的账号亲和不受影响。
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
- **结果未知的请求只在原账号上重发**：`tools/call` 已发出而答复丢了，在预算内只在同一账号上再发，从不换号，规则与模型请求相同；搜索是只读的，重复的代价是计费，换号会让另一个账号也计一次费，而丢失答复不证明是账号的问题。
- **不做 Session 亲和**：每次调用都从列表第一个可兑付的账号开始一轮，不记 Session 绑定。理由：搜索没有按账号隔离的 prompt cache，不出错时按顺序选号本来就总落在同一账号；记录绑定要在 Ledger 里多一个字段，而它换不来任何收益。
- **连接的身份含所选账号的引用**：`Residents` 按声明与 run root 留连接，声明里带着 header 的引用，所以两个账号恒不共用一条连接，换号就是换一条连接；同一个引用换了 Key 由 §8-2 的 `invalidate` 丢掉旧连接。
- **不回落**：所选那一家连不上、`tools/list` 里没有 `remote`、调用失败，都以失败的工具结果交回，不换到缺省那一家或列表里的另一家（city D24）。
- **Ledger 里没有 Key**：这件工具的调用与结果照任何 Connector 工具入账；错误的 subject 只写供应方 id 与账号 id，不写 Key，也不写引用。
-/

/-! D52 `web_search` 只执行 `account_recovery` 交回的步骤，自己不选号、不重发

**决定**：`web_search` 的账号选择、同账号重发、换号与停止全由 kernel 的 `account_recovery` 决定，本工具只把每次失败交给它、照它交回的 `Step` 做；等待用 Watchdog 的退避表。搜索账号不做 Session 亲和。

**理由**：模型请求与搜索请求守同一组由人定下的规则：可恢复错误在原账号重试一到两次再换号，账号错误直接换号，请求错误不重发，结果未知不换号。两条路径各写一个循环，就有两处可以分开漂移的规则；kernel 是 runtime 与本 crate 唯一共同的内层，规则放在那里，两条路径各自只驱动它。亲和是为了保住按账号隔离的 prompt cache，搜索没有这种 cache。

**被否**：①工具自己挑「第一个可兑付的账号」、对 `Retry::Yes` 自己再发一次：这是第二处选号和第二个重试循环；②搜索也记 Session 绑定：要多一个 Ledger 字段，换不来 cache 收益。

**重开参数**：某个搜索供应方按账号缓存或按账号计配额，使得在不出错时留在同一账号有可度量的收益。
-/

/-! D4 `Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `agent_protocols::Outbound`

理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `agent_protocols`，不归 worker。
-/

/-! D5 `Connectors::connect` 取 `&self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`

理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。取 `&self` 而不是 `&mut self`，是因为 MCP 缺表时的连接在 lane 里做，几条 lane 借同一张表的 `Arc`；`&mut self` 会把所有 lane 的握手排成一队。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
-/
