-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::assembly

规定 `crates/sprawling/src/assembly.rs` 与 `crates/sprawling/src/assembly/`：装配根，造生产的手、接上端口、起线程、开城（`bin::assembly`）；开城的次序在 `spec/Assembly/Listening.lean`，审计线程的结局在 `spec/Assembly/ChainWatch.lean`。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

本文件是描述，不是被证明的规格：它不含 Lean 定义与定理，它说的每一条由各节点名的测试判。城的唯一写者 `RunWorker` 住 `accounting` crate（`crates/accounting/src/worker/`），§8-7、§8-39、§8-51、§8-102、§8-165 与 D16 说的是那个写者被装配层用到的那一面；`accounting::worker::…` 这样的路径指的就是那个 crate。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::assembly::dropping`、`bin::assembly::remote_door` 旁的测试守住。
-/

/-!
## 8-7 ACP 入站

```rust
// accounting::worker::dispatching::agreeing
pub fn acp_dispatch(desk: &CommandDesk, body: &serde_json::Value, pairing: wire::Pairing)
    -> Result<wire::AcpProgress, AxError>;          // 外来请求 → 普通 Dispatch
```

- **令牌在门那侧判，判定在协议那侧措辞**：`wire` 持配对令牌，故常数时间比对住 `/acp` 路由；`pairing`（`wire::Pairing`，`Held` 或 `Absent`，不是 bool）这一位传进来，由 `agent_protocols::admit` 说拒词——未配对者只学到一位，这句话的权威只有一个。
- **入站不是第二个 control surface**：admit 之后就是人按派活条时走的同一条路（同一个 `CommandDesk`、同一个 `Command::Dispatch`）。回给编辑器的只有 progress 三字段，且 run id 是工人接单时才铸的，故此刻诚实的答案是「已受理、尚未完成」。
-/

/-!
## 8-15 装配层的公开面（`crates/sprawling/src/lib.rs`、`assembly.rs`）

```rust
// crates/sprawling/src/lib.rs —— 索引文件，只准声明（modmap 看守）
pub mod assembly; pub mod audience; pub mod console; pub mod doctor; pub mod firstrun;
pub mod monitor; pub mod release; pub mod serving; pub mod supervising;
mod browser_bidi; mod browser_tool; mod keying; mod outside; mod revealing;

// assembly：跨出 crate 的项，逐个放行
pub struct SystemClock;                                   // 墙钟只在这里取样（确定性规则 2）
pub fn hands(vault: gateway::Custodian) -> accounting::worker::Hands;  // 生产的手只在这里造
pub fn init_city(city_root: &Path) -> Result<InitReport, AxError>;     // = form_city(.., Adopt::Nothing)
pub fn form_city(city_root: &Path, adopt: Adopt) -> Result<InitReport, AxError>;
pub async fn listen(serving: serving::Serving) -> Result<Listening, AxError>;   // §8-88
impl Listening {
    pub fn local_addr(&self) -> SocketAddr;                // wire D16
    pub async fn serve(self, console: Option<console::Terminal>) -> Result<(), AxError>;
}
```

`InitReport`、`Adopt` 住 `accounting::worker::genesis`，`RunWorker`、`ScanReport` 住 `accounting::worker`，`History`／`has_history` 住 `city::history`，`open_vault` 与 `Serving` 住 `bin::serving`；装配层只把它们接起来。

- **为什么有 lib target**：只有 `src/main.rs` 时，工作区里没有任何东西能依赖装配层，citysim 与 `tests/` 都够不到它。`crates/sprawling/tests/assembly_door.rs` 走的正是这扇门：`init_city` 造城，再读 `InitReport.ledger_dir` 下的账本。
- **`pub mod` 而非扁平 facade**：模块表以 `bin::assembly`／`bin::console`／`bin::firstrun` 命名模块，模块名本身是已记录的架构事实；本 crate `publish = false`，C-REEXPORT 要替第三方省的那段路径没有受益人。**取窄的地方在项，不在模块**：只有跨出 crate 的项是 `pub`，其余留 `pub(crate)` 或 `pub(super)`。
- **`install` 与 `wire_client` 留在 bin**：前者把二进制放上 PATH，后者从终端连一座已服务的城并从 stdin 读 enrolment——两者都是关于命令行的，不是关于城的。
- **`Cargo.toml` 不分 target 配置**：Cargo 对同一 package 自动发现 `src/lib.rs` 与 `src/main.rs` 两个 target，OUT_DIR 对两者相同，`include!(client_embed.rs)` 与 `DEPENDENCIES` 因此留在 `main.rs`。
- **门禁**：`header` 要求 `lib.rs` 带 MPL 通告；`modmap` 对 `*/lib.rs` 按索引文件判定，只准 `mod`／`use`／`pub use`／注释／属性。
- **citysim 不是第二个装配层**：`tools/citysim/Cargo.toml` 不依赖 sprawling，`run_scenario` 手工构造 `RunPlan`，够到的最高层是 `runtime::run::drive`；装配层可被依赖，但 citysim 不依赖它。
-/

/-!
## 8-39 写者是一棵模块树（`accounting::worker::*`）

`RunWorker` 的定义在 `crates/accounting/src/worker.rs`，它的方法按字段簇散进 `worker/` 下的子模块（`credentials`、`workbench`、`settling`、`reviewing`、`commanding`、`driving`、`dispatching`、`waking`、`genesis`、`lifetime` 等），每个文件守 `xtask length` 的上限。下面说的是它的切法，不是一份文件清单；文件清单是 ARCHITECTURE 的模块表。

- **子模块，不是兄弟模块**：子模块看得见父模块的私有项（Rust Reference，*Visibility and Privacy*："If an item is private, it may be accessed by the current module and its descendants"），所以 `RunWorker` 的字段保持私有，跨文件调用收成 `pub(in crate::worker)`，`worker` 之外看到的仍是同一张脸。
- **缝取自字段簇，不取行数**：`city_root`、`ledger`、`governance` 被多数方法碰；其余字段成簇不交叉——凭据（`vault`）落 `credentials`，工作台与收尾（`inboxes`／`joins`／`requests`／`goals`）落 `workbench` 与 `settling`，追求（`pursuits`／`delegator`）落 `plans`，打断与观察落 `driving`，敲门（`knocks`）落 `waking`。`settle_requests` 回答「一个不能直接写的楼怎么收下这次改动」，所以住 `reviewing`；`create_building`／`adopt_building`／`startup_scan` 回答一座城怎么长出楼、重启后看见什么，所以住 `genesis`；`configure_building` 是人发的一个动词，住 `commanding::configure`。
- **总在一起走的值是一个有名字的值**：`Assignment`（一次派活：地址、模式、天花板、谁交下来）、`Given`（这一轮活被给了什么）、`Driving`／`Driven`（一次 drive 跑在什么上、以什么结束）、`Ending`、`Sweep`、`Reach`、`Entered`（一个人为接一个 endpoint 输入的东西，凭证是 `Credential` 枚举）、`Stated`（人为一个模型声明的东西：`Option<Window>` 上下文、`Option<Ceiling>` 最大输出与输入梯子的第一档；缺席即「没人声明过」，§8-71）。它们让函数守 4 个参数的上限，而不是压制 lint；`checkpoint_scope` 是 `Site` 上的方法，「检查点落在楼上还是落在房间上」只有一个答案。
- **簇还不是对象**：每个簇的方法都经 `self.record(...)` 写账本，把簇变成真对象要先分开「判定」与「记账」（ARCHITECTURE §5 的 invert the model seam），这件事还没做。
- **测试跟着它咬的那个模块，夹具一份**：按脚本作答的 OpenAI 服务器与 `worker_with_provider` 这一簇夹具住 `accounting::worker::fixture`（`#[cfg(test)] mod`，子模块从 `super` 够得到）。`#[cfg(test)]` 的东西到不了 `tests/`，`tests/` 的东西到不了 `src/`，分家就要养两份同名夹具，所以写者的测试留在 `src/`，`Views`／`Standing`／`CommandDesk` 不为测试变 `pub`。

### 交接探针（`accounting::worker::probing::probe`，形状 2 值类型）

探针的唯一生产调用点是 `accounting::worker::probing`，所以它住在调用者之下，不另占产品拓扑的一个单元（仪器与探针分家的理由见 citysim D6）。

```rust
pub(crate) struct ProbeId { pub(crate) name: String, pub(crate) version: u32 }
pub(crate) struct Probe { /* id、questions —— 私有 */ }
impl Probe {
    pub(crate) fn new(id: ProbeId, questions: Vec<String>) -> Result<Probe, AxError>;
    pub(crate) fn answered(&self, answers: Vec<String>) -> Result<Answers, AxError>;  // 数目对不上即拒
}
pub(crate) struct Comparison { pub(crate) kept: u32, pub(crate) lost: Vec<u32> }
pub(crate) fn compare(before: &Answers, after: &Answers) -> Result<Comparison, AxError>;
pub(crate) fn handoff_probe() -> Result<Probe, AxError>;   // 名 handoff、版本 1、固定四问
```

- **跨版本比较恒拒**：问题改过的探针是另一件仪器，混算测的是仪器不是被测物。`handoff_probe` 是数据不是判定：问题改了就是版本 2。
- **报位置不报分数**：`lost` 是问题的序号，人自己去读那两个答案——一个摘要在这里正好会掩盖它要报告的那类损失。
- **探针不去采集**：问问题的是 `probing` 驱动的一个 Run；`probe` 只持问题与比较，恒不在它所测量的那条回路里。
-/

/-!
## 8-165 Assembly 显式化：接线是一处，判定住折叠，搬运是值

- **接线是一处**：把一个选择变成一个模型适配器只在 `gateway::adapter_for`（`crates/gateway/src/endpoint/adapter.rs`，与 `Endpoint::new` 同簇）发生，凭据由调用方以 `resolver` 递入；`accounting::worker::credentials` 只出 `resolver`（赎回闭包是凭据的形状），不在 `RunWorker` 上挂装配线。值参而非方法，因为 `adapter_for` 读的只有选择与 `resolver`，挂在 `RunWorker` 上等于说「装配需要整座城」。
- **停摆判定只有一处**：`halted_by`（`accounting::worker::commanding::governing`，`pub(in crate::worker)`）读本进程的 `governance` 折叠（`HALTED`／`RELEASED`），每个调用点走它。它不进 kernel：它答的是折叠状态，不是纯函数能回答的问题，搬进 kernel 等于把 `Governance` 一起搬过去。
- **适配器随值来去**：`Driving` 拥有它的适配器（`accounting::worker::keeping_warm::Door`），drive 结束时经 `Driven.adapter` 交回，调用方拆开归位；`&mut` 出借会把「谁拥有适配器」悬在一次调用上。`Site` 一侧的 `adapter` 是 `Option<Door>`，只是这次搬运的载具，跨过调用时两端皆为 `Some`；`None` 分支以 `E_CONFIG_INVALID` 拒绝而不 panic（§8-40 的先例）。
- **定时只有一个 `last_*`**：`last_tick` 的读写只在 `commanding::routing` 的 `tick`（读表→判断到期→推进）；它是写者状态而不是散装轮询，删它等于删掉「开机不补跑昨日」这条产品语义（§8-6）。
- **LOADING／UNLOADING 有名有主**：LOADING 是 `RunWorker::over` 里的 `Standing::fold`（一次验证、三折叠），UNLOADING 是 `close_city` 写 handoff；不给它们另起名字，因为给已存在的东西改名是第二个权威。
- **看守它的测试**：`a_loopback_endpoint_with_a_credential_sends_it_on_every_call` 与 `a_dispatch_without_a_provider_fails_saying_what_to_configure` 咬的正是这条装配线。
-/

/-!
## 8-51 城的创世哈希一座城读一次（`bin::assembly` 与 `accounting::worker::workbench::standing`）

`storage::Provenance::city_of` 读的是账本首段的第一行，而 `provenance()` 每造一次署名就读一次：一次波里的每个检查点、每次落地、每次合并各读一次盘。**这个事实在一座城的一生里恒定不变**——创世行写下就不再改，改了那也不是同一座城。

**记在 `RunWorker` 上，用 `OnceLock` 惰性读一次**：不在 `over()` 里急读，因为一个刚被造出来、账本还空着的 worker 是合法状态（`RunWorker::new` 在一座尚未 init 的城上就是这样被测试用的），急读会把「还没有创世行」变成造不出 worker。

**缓存一份副本在这里不构成第二权威**：第二权威的危险来自**会变的**事实被抄了一份；创世哈希不会变。真正被消掉的风险是相反的一个——每次重读都可能读出不同的答案（有人换了账本），而一次运行里换了城的身份是比陈旧副本坏得多的事。

- **接口随之改形**：`workbench::standing::provenance` 与 `Site::provenance` 收 `city: B3Hash` 而不再收 `&Path`，于是「谁去读盘」这件事只剩 `RunWorker::city_hash` 一个答案，每个调用点都从它取。
- **作证方式**：城的创世哈希被读过一次之后，把账本首段从盘上删掉，`city_hash` 仍答同一个值——记住了才可能如此。
-/

/-!
## 8-134 页面读到历史证明到哪里（`accounting::views::city`、`bin::assembly::listening`、`bin::assembly::attending`；`crates/wire/Spec.lean` §8-63）

服务中的城在后台证明整条链（§8-90、§8-122），证明完成之前写者拒绝每一次追加。页面需要在同一刻知道这件事，否则一次被拒的命令看起来是坏了。`CityAnswer.proved` 回答它。

- **一个句柄，两个读者。** 写者线程挂上的 `storage::ChainHalt` 是判定的唯一一处：写者用它决定接不接一行，视图用它决定答不答「已证明」。`bin::assembly::listening` 在起写者之前造这只 halt，经 `Views::watch_proof` 交给重建的视图（备用的那份由 `twin` 从它复制），`bin::assembly::attending` 经 `RunWorker::chain_under_audit(halt)` 把同一只挂给写者。
- **答什么。** `halt.proved()` 为真时答视图此刻的头（`Some(head)`）：证明完成时写者还没写过一行，之后的每一行都由这个已证明的写者接上；为假（还在证明，或证明发现链断了）答 `None`。不经 halt 起步的视图（一次性查询、测试）起步前已同步证明过整条链，答它们的头。
- **不进账本。** 证明是这台主机对这份账本的一次核对，不是城的历史；它记在 `the history is proved` 那一行诊断里（§8-121），页面读的是此刻的判定。
- 验收：`accounting::views::city` 的 `a_city_answer_says_the_history_is_proved_only_once_the_halt_says_so`。
-/

/-!
## 8-56 说出来的那句话：录音进城，一行字出来（`accounting::views::hearing`、`bin::assembly::listening::hearing`；`crates/wire/Spec.lean` §8-27）

`gateway::transcribe` 是转写的适配器；本节是从 `/transcribe` 走到它的那条路。

- **人填 URL 与 key 走既有的 attach 表单**。「哪个 endpoint、哪个 model 答这一类活」已有机制——`ModelTag`。第三个 tag `Transcribe` 因此是全部的新增面：第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。
- **`Views::transcriber`**（`accounting::views::hearing`）：从视图的一份快照读出选择、造出 `Transcriber`，放开快照之后才发请求。一次转写是数秒，而一份被握着的旧快照会让折叠迟迟收不回它。录音到达的是城一级的门、身上没有地址，读不出任何一座楼的规矩，故这里**写明** `BuildingPolicy::new(false)` 而不是取默认值——把「口述按普通楼出门」这件事摆在读者眼前。上传带上它所属的那座楼之后，这个值同样从楼规来。
- **`assembly::listening::hearing`**：把发布的视图与金库收成一条 `TranscribeSink`。金库是**工人开的那一把**（`Arc<Mutex<Custodian>>`），经启动握手那条通道交出来（`Started.vault`），经 `accounting::held_vault::resolving` 赎回凭据——第二个 `Custodian` 会是同一批机密的第二扇门。
- **容器从请求头读**：`AudioType::of_media_type` fail closed，拒词列出这座城发得出去的五种。浏览器录进它手上有的容器，而只有它知道是哪一个。
- **页面**：`core/speaking.ts` 管录音与上传，composer 多一个按钮，**转写结果落进输入框而不是直接发出去**——机器听错的那一句必须能改，否则它会花掉一次 run。没有 `transcribe` 选择的城不画这个按钮（`useHearing`）。
- **同一个选择也给 run 一件工具**：`transcribe`，有没有它读的是同一次 `select`，只是按 run 所在那座楼的楼规读（§8-131）。
- **验收**：`wire` 的 `/transcribe` 路由在没有 content-type 时按名拒绝；`gateway::transcribe::recording` 的 `of_media_type` 认得五种容器、拒第六种并列出前五种。
-/

/-!
## 8-102 开城时修过什么，要说给人（`accounting::worker::lifetime`、`accounting::worker::genesis`、`bin::assembly::attending`）

**原因**：`JsonlLedger::open` 在断尾恢复时截掉撑裂的尾行，并返回 `OpenReport`（`crates/storage/Spec.lean` §8-1）。账上虽然多了一行 `log_truncated`，但页面不画它，CLI 也不读它；报告若被丢掉，人就不知道上一次进程死时丢了几个字节。

**形状**：值（形状 2）。`RunWorker` 持有 `LedgerOpening`，它是 `OpenReport` 在本 crate 的类型化状态，只由 `LedgerOpening::from(OpenReport)` 生成：

```rust
pub enum LedgerOpening { Intact, TailDropped { bytes: u64 } }
impl From<storage::OpenReport> for LedgerOpening { … }
impl LedgerOpening {
    /// 给人看的一句：截掉了什么、为什么、怎么恢复；`Intact` 答 `None`。
    pub fn notice(self) -> Option<String>;
}
impl RunWorker {
    // 第四个参数就是 JsonlLedger::open 返回的那一对：账与它开时修过什么一起到，调用方没法只交一半。
    pub fn over(&Path, Diagnostics, Hands, (JsonlLedger, OpenReport)) -> Result<Self, AxError>;
    pub fn opening(&self) -> LedgerOpening;
}
```

- **到人的两条路**：`sprawling resume` 的 `ScanReport::summary()` 在断尾时多一句 `notice()`；`sprawling serve` 的写者线程在开账之后、`open_for_service` 之前把同一句印到 stderr，先于横幅出现。两处读的都是同一个 `notice()`，措辞只有这一个来源。
- **`form_city` 不丢报告**：它只在目录没有账本时开账，报告因此恒为 `Intact`；它照样把 `open` 返回的那一对原样交给 `over`，让「worker 知道自己的账是怎么开的」对每条构造路径都成立，而不是靠一句注释说这里不会发生。
- **恢复**：截掉的是进程死时没写完的那一行，它之前的每一行都已按链校验。人要做的是确认最后一次动作是否需要重做；要逐字节看原状，就在再次打开之前从备份拷回 `.ledger`。
- **仍未到页面**：页面对 `log_truncated` 什么也不画（`client/src/core/belief.ts` 把它归进不显示的一组）。让城页说出这件事，需要一个视图字段与客户端的一个位置，这是本节接口尚未覆盖的一半。
- **`pub` 是因为读者在另一个 crate**：`LedgerOpening`、`opening`、`notice` 住 `accounting`，而印出那一句的 `bin::assembly::attending` 在 `sprawling`；`sprawling resume` 仍只经 `ScanReport::summary()` 读它。
- **被否：只把 `notice` 写进 `Diagnostics`**。`serve` 默认不开日志，`resume` 用的是 `Diagnostics::off()`，写进去就等于没说。

**本节测试**：`accounting::worker::lifetime::tests::a_torn_tail_is_told_in_the_startup_scan`：写一座城，在账尾追加半行，`RunWorker::new` 后 `startup_scan().summary()` 必须说出截掉的字节数。
-/

/-!
## 8-119 拖进对话框的文件存进城里（`bin::assembly::dropping`，形状：adapter）

浏览器不把被拖进页面的文件的绝对路径告诉页面。拖放携带 `file://` URI（`text/uri-list`）时页面直接用那条本地路径，不经过城；否则页面把字节经本页同源的 `POST /drop`（`crates/wire/Spec.lean` §8-49）交给城，城存下它，答出它的绝对路径，页面把这条路径插进对话框。本节是城这一半。

**接口。**

```rust
pub(super) const DROPPED_DIR: &str = "dropped";
pub(super) fn keep_dropped(city_root: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, AxError>;
pub(super) fn dropping(city_root: PathBuf) -> wire::DropSink;   // listening 把它装进 ServeConfig
```

- 存放处是 `<city>/hall/dropped/<hash>/<name>`：`<hash>` 是字节的 blake3 前 16 个十六进制字符，`<name>` 是浏览器给的文件名的最后一段。答出的是 `std::path::absolute` 给出的绝对路径。
- 名字由地址文法判：`hall/dropped/<hash>/<name>` 必须能被 `kernel::Address::parse` 读出来，否则 `E_INVALID_ARGS`，recovery 说改个名字再拖。空名字、`..`、带路径分隔符的名字都在这一步被拒。
- 同样的字节、同样的名字再拖一次，落在同一个文件上并答出同一条路径，文件只写一次；同名而字节不同的两个文件落在两个目录里，互不覆盖。
- 写失败是 `E_STORAGE_FATAL`，subject 是那条路径。
- **三个平台同一条规则**：地址文法在 Windows、macOS、Linux 上都拒反斜杠与 `:`（盘符与 NTFS 流），所以在 Linux 上本来合法的、带反斜杠的文件名在这里同样被拒，同一个名字在三个平台上得到同一个答复；答出的路径用本平台的分隔符（`std::path::absolute`）。

**决定。**

1. **存在 hall 的 `dropped/` 下，而不是保留子树 `.sprawling/` 或城根下自己的目录。** 居民的 `read` 只收城内相对地址，并且不进保留子树（`crates/runtime/Spec.lean` `tools::chosen_path`）；放进 `.sprawling/` 的文件人能看见，市长却读不到。城根下每个顶层目录都是一栋楼（`city::buildings`），放在城根下的 `dropped/` 会作为一栋楼出现在城里和工作区的菜单里。hall 是每座城都有的楼（`kernel::consts_policy::HALL_BUILDING`），市长在自己的楼里读它，其他楼按 `kernel::address::may_read` 读它（除非 hall 的规则把它标为机密），而绝对路径的末尾就是那条相对地址。
2. **目录按内容命名。** 用时间或序号命名，同一个文件拖两次就是两份；用名字命名，两个同名的文件互相覆盖。内容哈希让两件事都不发生，也不需要任何状态。
3. **名字交给地址文法判，不另写一份清洗规则。** 一条存得下却不能被读成地址的路径，居民拿到了也打不开；地址文法已经是「城里什么路径合法」的唯一回答。

**测试。** `assembly::dropping::tests`：一个文件存下后答出的路径在 hall 的 `dropped/` 之下、内容与拖进来的字节相同；同样的字节再存一次答出同一条路径；`..` 与带分隔符的名字被拒为 `E_INVALID_ARGS`，城里什么也没有写。
-/

/-!
## 8-151 `/remote open` 按城的 `[remote]` 选通路（`bin::assembly::remote_door`，形状：adapter；`crates/city/Spec.lean` §8-39，crates/remote_access/Spec.lean §8-7 到 §8-9、D22）

§8-139 的门有了看守、监听与中继，但没有一条通路能把它从这台电脑之外够到。这一节把城的 `[remote]` 表接到 `remote_access::route` 这条缝上。

```rust
// bin::outside::keeper
pub(crate) type Choosing = Box<dyn FnMut() -> Result<Box<dyn Route + Send>, AxError> + Send>;
// bin::assembly::remote_door
const CLOUDFLARED: &str = "cloudflared";                 // 不写 command 时从 PATH 找的程序
const ROUTE_PATIENCE: TimeoutMs = TimeoutMs(30_000);     // 一次 open 最长等多久
fn chosen(city_root: &Path) -> Result<Box<dyn Route + Send>, AxError>;
```

- **一处选择**：`chosen` 读 `city::remote_route`（`crates/city/Spec.lean` §8-39），对 `RemoteRoute` 穷尽匹配：`Cloudflare` 造 `NamedTunnel`，`tunnel` 经 `TunnelName::parse`、`url` 经 `PublicUrl::parse`，程序是 `command`，不写时是 `cloudflared`；`Command` 造 `CommandRoute`，`command` 是程序，`args` 照写，`HostPermanence` 一对一换成 `Permanence`。生产里造通路的只有这一处；`Outdoors::keep` 把一个读城根的闭包交给 `Keeping.choose`，门在每次 `open` 时调它（§8-139）。
- **拒绝原样交出**：没配 `[remote]` → city 的那句拒绝（`E_CONFIG_INVALID`，主体是城那一层的文件，恢复语写出两种通路必需的键）；隧道名、地址写错 → `remote_access` 的拒绝（`E_CONFIG_INVALID`）；程序起不来 → `E_TOOL_UNAVAILABLE`，主体带程序名；到时没就绪 → `E_TIMEOUT`。每一种都发生在门打开之前：门关着，通路没开，账上没有一行。
- **耐心 30 秒**：`cloudflared` 连上边缘通常要一到数秒（crates/remote_access/Spec.lean §14），一条包着 `tailscale serve` 的命令更快。`open` 在控制台的线程上等，门的锁这段时间握着，所以等太久的代价是控制台看起来像停了。30 秒够一次慢网络；改它就是改人敲下 `/remote open` 之后最长等多久。
- **绑定之后的地址**：`Outdoors` 与另外三个读者——`bin::main::city` 印的横幅、控制台 `/serving` 印的地址、打开浏览器探的地址——读的都是绑定之后的地址：`wire::Bound::local_addr`（`crates/wire/Spec.lean` §8-46，wire D16）在绑定时读一次，`assembly::listening` 经 `Listening::local_addr` 把它交给 `Outdoors::new` 与这三个读者。城在 `:0` 上服务时，中继连的是系统分到的那个端口，端到端测试因此在 `127.0.0.1:0` 上起城。**被否**：`listening` 先在 `:0` 上试绑一次拿到端口、放掉，再把具体端口交给 `wire::bind`（试绑与真绑之间别的进程可以把端口拿走，`crates/wire/Spec.lean` §8-46 已否这一做法）。

**测试**：

- `assembly::remote_door::tests::a_city_with_no_remote_table_is_refused_naming_the_table`：城那一层没有 `[remote]`，`chosen` 答 `E_CONFIG_INVALID`，恢复语里有 `route`、`tunnel`、`url`、`command`、`permanence`。
- `assembly::remote_door::tests::the_route_the_table_names_is_the_one_that_opens`：`command` 一臂写一个不存在的程序，打开时的拒绝是 `E_TOOL_UNAVAILABLE` 且主体带那个程序名；`cloudflare` 一臂写同样不存在的程序，拒绝的恢复语说装上 `cloudflared`；`cloudflare` 一臂的 `url` 写成 `http://`，造的那一刻就以 `E_CONFIG_INVALID` 拒。
- `crates/sprawling/tests/remote_door.rs::a_device_reaches_the_city_through_the_door_over_real_sockets`：`assembly::init_city` 造城，城那一层写 `[remote] route = "command"`，命令是这个测试程序自己（`route_stand_in`，一个只在被当作通路时做事的被忽略的测试：它从 `SPRAWLING_REMOTE_LOCAL` 读到回环地址，印出 `{"url": "https://<那个地址>"}` 后等着被结束），于是「外面」就是回环本身。起 `sprawling serve <城> 127.0.0.1:0 --console`，在它的标准输入上打 `/remote open`、`/remote pair phone`，从印出的链接读出配对码与指纹；设备一侧用 `remote_access` 在 `/remote/pair` 上配对、在 `/remote/session` 上建会话；会话里发线协议的 `Hello`、一个 `Read` 帧（`Ask` `Metrics`，答回 `Answered`）、一个 `Act` 帧（`Halt` 全城，设备收到 `city_halted` 的事件帧）与一个 `LocalOnly` 帧（`Reveal`，答回 `E_GATE_DENIED` 的 `Refusal`）；再打 `/remote close`。结束城之后读账：远程门的四行与 `city_halted` 依次是 `remote_opened`、`device_paired`、`remote_session_started`、`city_halted`、`remote_closed`。
-/

/-! D16 远程门的五行经 worker 的 relay 写，不经命令桌

远程门在装配层，而城的唯一写者在 `accounting::worker`；本城给 `RunWorker` 一个公开的 `relay()`，交出驾驶线程用的那一种 `kernel::Ledger`，门的看守拿它写 `remote_opened` 等五行（§8-139）。理由：relay 本就是「一个不是写者的线程把一行交给写者并等它落盘」的那扇门，答回来时这一行已经持久，门的下一步可以靠它；城仍只有一个写者。**被否**：①给 `wire::Command` 加五个没有线上形式的命令（如 `PutSecret`），由 `run_command` 落账：每一个都要进 `COMMAND_NAMES`、§19-2 的 reach 与 class、控制台的投影与客户端的生成类型，五个只为写一行而存在的动词散进六处；②让门的看守自己开一个账本写者：一座城就有了两个写者。**重开参数**：门需要等一行落盘之外的答复（例如写者先判这一步合不合规矩）时，改成一条桌上的进程内命令。
-/

/-! D17 通路在每次 `/remote open` 时按 `[remote]` 造出，随门开、随门关（§8-151）

人改了城的 `[remote]`，下一次 `/remote open` 就用新的通路，不必重启城：城的签名密钥只活在这个进程里（crates/remote_access/Spec.lean §3），重启一次，每台设备都要重新配对。选择只有 `remote_door::chosen` 一处，它对 `city::RemoteRoute` 穷尽匹配，`Doorway` 只认 `Route` 这条缝。**被否**：开城时读一次、把造好的通路放进 `Keeping`（改了配置要重启城，代价是全部设备重新配对；配错的 `[remote]` 还会让 `/remote devices`、`/remote revoke` 这些与通路无关的动词一起失效）；让 `remote_access` 的通路类型自己从 TOML 读出（第二个配置读者，梯子与拒法要再写一遍，remote_access D22）。**重开参数**：通路的打开变得昂贵到要在开城时预热，或一座城要同时开着几条通路。
-/

/-! D18 远程门的端到端测试用 Rust 写，起真二进制（§8-151）

黑盒检查按 `xtask boundary` 归 `tools/adversary/` 的 Lean，可设备那一半的配对握手、会话握手与封装（ML-KEM、ML-DSA、AES-GCM）只在 `remote_access` 里有实现，Lean 那一侧今天说不了远程门的话；在 Lean 里再实现一遍，就是第二份密码学。所以 `crates/sprawling/tests/remote_door.rs` 起 `sprawling serve`，以 `remote_access` 的设备一侧走真套接字，起二进制与开 WebSocket 的几行各带 `boundary-ok`。**被否**：在进程内驱动 `bin::outside`（`outside` 不是库的公开面，而且那样测不到控制台读 `/remote open`、装配层读 `[remote]`、命令通路起子进程这三段生产路径）。**重开参数**：`tools/adversary/` 有了一个能做混合签名与封装的设备，这条测试就搬过去，`boundary-ok` 随之删掉。
-/

/-!
## 技能审核后台（city D19；`bin::assembly::skill_audit`，adapter）

`attending` 在接观察者前启动审核线程，将新提交的 `skill_shelved` 与 `run_started` 送到该线程；观察者只投递，不等 HTTP、程序或落账。线程启动时用 `runtime::replay::fold_ledger_dir` 读来源和有效审核历史，扫城库与各楼架；随后每个通知更新来源，再扫一次。读楼架沿 `city::Neighbourhood::scan` 的建筑列表，home 沿 `accounting::home::Home`。后台摘要沿 city 的 `skill_digest`，不使用 SKILL.md 的单文件哈希代替包哈希。去重集合在后台线程内持有，按名字与摘要去重，寿命等于本次开城。关闭观察者即关闭通知通道，线程在当前审核结束后退出。

HTTP 调用用 gateway 的 client_for，期限用 city 的 SKILLS_SH_TIMEOUT，不带令牌、不重试；可选程序用 doctor 的 find_program 与 asking::ask。所有载荷用 Payload::of 编码，CITY run、City actor，时间用 SystemClock，经 Relay 写回唯一写者。取审核失败在载荷中记 Unreachable、原因报告到 stderr；读架失败报告并保留下一次通知的重扫机会；写账失败结束线程。启动线程失败报告后仍开城，审核不准改变上架或 Reading Room。
-/

/-!
审核后台的 `serve(root, ledger, receive, audit)` 保留生产重放与扫描循环，写入接缝是既有 `kernel::Ledger`（生产 Relay、测试持久 JSONL 与失败适配器），审核接缝是 `FnMut(&city::AuditRequest) -> city::AuditReport`（生产 clients::audit、离线脚本）。接口留在模块内，因为没有跨 crate 的调用者，不引入公开 trait。
来源表按 digest 保存首次来源，未知摘要用 Path 表达远端不适用，载荷编码由 `RecordedAudit` 携带展平的 SkillAudited 与可选 `local_only_reason`；该字段只说明本次后台为何跳过远端，既有审核字段保持各自语义，不改变 wire 帧。理由常量只有后台一处定义。
`clients::scan` 接收程序问答 `FnMut(&mut Command, Duration)`，生产使用 answered，第二适配器回放退出码与输出；版本非零、信号、超时、起不来均返回 Unreachable，scan 不调用。
验收由 `skill_audit::tests` 经同一 serve 入口覆盖 city D19 的去重性质、启动与通知扫描、成功不重审、Unreachable 跨进程重试、失败 Ledger 停止，以及同名不同摘要与未知摘要的来源回归；`clients` 的版本问答检查将程序失败送进 city::audit_skill，核对 Unreachable 载荷。
-/
