-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::assembly

规定 `crates/sprawling/src/assembly.rs` 与 `crates/sprawling/src/assembly/`：装配根，造生产的手、接上端口、起线程、开城（`bin::assembly`）；开城的次序在 `spec/Assembly/Listening.lean`，审计线程的结局在 `spec/Assembly/ChainWatch.lean`。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-7 ACP 入站

```rust
fn acp_dispatch(desk: &CommandDesk, body: wire::AcpBody, authentic: bool)
    -> Result<wire::AcpProgress, AxError>;          // 外来请求 → 普通 Dispatch
```

- **令牌在门那侧判，判定在协议那侧措辞**：`wire` 持配对令牌，故常数时间比对住 `/acp` 路由；`authentic` 这一位传进来，由 `agent_protocols::admit` 说拒词——未配对者只学到一位，这句话的权威只有一个。
- **入站不是第二个 control surface**：admit 之后就是人按派活条时走的同一条路（同一个 `CommandDesk`、同一个 `Command::Dispatch`）。回给编辑器的只有 progress 三字段，且 run id 是工人接单时才铸的，故此刻诚实的答案是「已受理、尚未完成」。
-/

/-!
## 8-15 装配层长出一扇门

```rust
// crates/sprawling/src/lib.rs —— 索引文件，只准声明（modmap 已看守）
pub mod assembly;
pub mod console;
pub mod firstrun;

// assembly：跨出 crate 的项，逐个放行
pub struct InitReport { pub ledger_dir, pub genesis, pub standing, pub adopted }
pub enum Adopt { Nothing, EveryFolder }
pub enum History { Absent, Present }   // 目录不存在或为空是 Absent；读不了是 Err，不是 Absent
pub fn has_history(&Path) -> Result<History, AxError>;   // StorageFatal：账本目录存在却列不出来
pub fn init_city(&Path) -> Result<InitReport, AxError>;
pub fn form_city(&Path, Adopt) -> Result<InitReport, AxError>;
pub fn open_vault() -> (gateway::Custodian, Option<Payload>);
pub struct Serving { /* 八个字段全 pub：调用方构造它 */ }
pub async fn listen(Serving) -> Result<Listening, AxError>;   // §8-88
impl Listening { pub async fn serve(self) -> Result<(), AxError>; }
pub struct ScanReport { pub waiting_approvals: usize /* lines、closed_calls 不跨出 */ }
impl ScanReport { pub fn summary(&self) -> String; }
pub struct RunWorker;
impl RunWorker {
    pub fn new(&Path, gateway::Custodian, Diagnostics) -> Result<Self, AxError>;
    pub fn handle(&mut self, wire::Command) -> Result<(), AxError>;
    pub fn startup_scan(&mut self) -> Result<ScanReport, AxError>;
    pub fn fork(&mut self, RunId, Seq, Option<Address>) -> Result<RunId, AxError>;
    pub fn adopt_building(&mut self, Address) -> Result<(), AxError>;   // 收楼即立基线 checkpoint（`crates/storage/Spec.lean` §8-8 base_checkpoint），进度写诊断
}

// console
pub struct Terminal { pub url: String, pub token: Option<String> }

// firstrun
pub enum FirstScreen { Start(PathBuf), Use(PathBuf), Quit }
pub fn ask<R: BufRead, W: Write>(&Path, &mut R, &mut W) -> std::io::Result<FirstScreen>;
pub fn default_city(&Path, Option<&Path>, bool) -> PathBuf;
pub fn is_writable(&Path) -> bool;
pub fn local_url(SocketAddr) -> String;
pub fn open_when_ready(SocketAddr, String);
```

- **为什么需要一个 lib target**：`crates/sprawling` 至今只有 `src/main.rs`，`mod assembly` 是私有模块，于是工作区里**没有任何东西能依赖它**——4377 行生产代码（含 1058 行的 `dispatch_in`）只由同文件内的 66 个测试看守，citysim 与任何 `tests/` 都够不到。加一个 lib target 让它可被依赖。
- **`pub mod` 而非扁平 facade**：§12 模块表以 `bin::assembly`／`bin::console`／`bin::firstrun` 命名模块，模块名本身是已记录的架构事实；折成 `sprawling::init_city` 会抹掉这层限定，而本 crate `publish = false`，C-REEXPORT 要替第三方省的那段路径没有受益人。**取窄的地方在项，不在模块**：只有跨出 crate 的项改 `pub`，其余留 `pub(crate)`——公开面因此是逐项决定的，不是逐模块授予的。
- **`install` 与 `wire_client` 留在 bin**：前者把二进制放上 PATH，后者从终端连一座已服务的城并从 stdin 读 enrolment——两者都是关于命令行的，不是关于城的，且除 `main` 外零引用。留在 bin 让公开面少六项。
- **`handle` 进公开面不是为测试拓宽**：AGENTS.md 写着「Tests use the same doors as production code」。`handle` 正是服务中的 worker 循环走的那扇门，把它命名出来是承认已有的门。反过来，那 66 个内部测试**不搬去 `tests/`**：它们触及 `Views::rebuild`／`read_building`／`run_id_for` 这类内部项，搬迁会为测试拓宽公开面，正是同一条规矩禁止的事。本 crate 的文件长度因此不变——它变短要等拆 `dispatch_in` 时把生产代码连同其测试一起搬走。
- **`ScanReport` 只放行一个字段**：`main` 读 `waiting_approvals` 决定是否多印一行，`lines` 与 `closed_calls` 只进 `summary()`。按需放行而非按结构对齐——`InitReport` 四个字段全跨出，是因为 `report_standing` 四个全读。
- **零行为变更**：`main.rs` 只改开头的声明块（七行 `mod` → 两行 `mod` ＋ 一行 `use sprawling::{assembly, console, firstrun}`），其余调用点逐字节不变。`Cargo.toml` 不改：Cargo 对同一 package 自动发现 `src/lib.rs` 与 `src/main.rs` 两个 target，OUT_DIR 对两者相同，`include!(client_embed.rs)` 与 `DEPENDENCIES` 因此留在 `main.rs` 原地。
- **红**：`crates/sprawling/tests/assembly_door.rs` 走 `init_city → RunWorker::new → handle(Command::CreateBuilding) → 读 InitReport.ledger_dir 下的账本`，断言 `building_created` 落账。改动之前它连编译都过不去（`sprawling` 这个 crate 名不存在），这就是「这条测试咬得动」的证据。
- **门禁连带**：`header` 要求 `lib.rs` 与新测试文件各带三行 MPL 通告；`modmap` 对 `*/lib.rs` 自动按索引文件判定，只准 `mod`／`use`／`pub use`／注释／属性——facade 因此只能是声明，正是要的形状。
- **一处文档更正**：ARCHITECTURE.md §3 写着「citysim is a second assembly layer: the same code with simulated adapters」。此句与现实不符——`tools/citysim/Cargo.toml` 依赖 kernel／storage／runtime／gateway，其中没有 sprawling；`run_scenario` 手工构造 `RunPlan`，够到的最高层是 `runtime::run::drive`。这次改动使 assembly **可被依赖**，但没有让 citysim 依赖它：模型适配器仍由 `adapter_for` 从 `EndpointBook` 内部构造，那条缝要不要倒置是另一个决定。按 AGENTS.md「reality wins and the document is corrected first, with its reason」，先把这句改成现实。
-/

/-!
## 8-39 装配点成为一棵模块树，十五条签名被消掉（`bin::assembly::*`）

`bin::assembly` 10,695 → 一棵树，每个文件在 1000 行以内，`[file_length.predating]` 的最后一行被划掉。

### 为什么是子模块，不是兄弟模块

`RunWorker` 22 个私有字段。**子模块看得见父模块的私有项**——Rust Reference 的 *Visibility and Privacy*：
"If an item is private, it may be accessed by the current module and its descendants"。
于是 `RunWorker` 的定义留在 `assembly.rs`，`impl RunWorker` 的方法散进 `assembly/*.rs`，
**可见性一个字不动**：crate 里 `assembly` 之外的任何模块看到的仍是今天那张脸。
兄弟模块做不到这件事，§8-37 与 §8-38 因此付了 `pub(crate)` 的价；这里不付。

### 缝在哪：先量再切

切缝取自一次 LCOM 测量（66 个方法对 21 个字段的接触矩阵），不取行数。读数：
`city_root` 被 21 个方法碰，`ledger` 与 `governance` 各 8，`inboxes` 5，`vault` 4，**其余 15 个字段各 ≤ 2 且成簇不交叉**：

| 簇 | 碰它的方法 | 落到 |
|---|---|---|
| `vault` | `put_secret`／`resolver` | `accounting::worker::credentials` |
| `inboxes`／`joins`／`requests`／`goals` | `lay_out_workbench`／`open_desks`／`settle_desks`／`settle`／`deliver_handback` | `accounting::worker::workbench`＋`accounting::worker::settling` |
| `pursuits`／`delegator` | `set_pursuit`／`pursue` | `accounting::worker::commanding` |
| `interrupts`／`watching` | `attach_interrupts`／`watch`／`drive_dispatch` | `assembly.rs`（装的两个钩子）＋`accounting::worker::driving` |
| `knocks` | `knock`／`answer_knocks` | `accounting::worker::dispatching` |

**这份读数说的是 `RunWorker` 是五个类**，而这里只把它们搬进各自的文件、让边界看得见；
把它们变成真的对象要先分开「判定」与「记账」（每个簇的方法都在 `self.record(...)` 写账本），那是 ARCHITECTURE §5 的
invert the model seam，仍未动手。**这里不假装做过它。**

### 门给这次拆分定的价：十五条签名必须被修好

`tools/xtask/src/length.rs` 的豁免键是 `路径::函数名`，而 `guard::strikes_only_exemptions` 的 rustdoc 写死了
"an over-long signature may be fixed or left alone, never relocated with its excuse"。
`assembly.rs` 里有十五条超标签名，**它们随文件搬家就失去豁免**，所以逐条消掉。
消法是同一条：**总在一起走、从不被单独选择的值，是一个还没有名字的值**（`Reporter` 的 doc 写下的先例）。

| 新值 | 它是什么 | 消掉了 |
|---|---|---|
| `Assignment` | 一次派活是什么：地址、模式、天花板、谁把它交下来（深度由它推出，不再第二次传） | `dispatch_in` 6→3、`lay_out_workbench` 7→4、`stand_up` 5→2、`settle` 5→3 |
| `Given` | 这一轮活被给了什么：brief、task、goal，与那份字节的 pin | `freeze_plan` 9→4 |
| `Driving` | 一次 drive 跑在什么上面：适配器、工作台、信号桌、写根、检查点域、身份 | `drive_dispatch` 9→3 |
| `Ending` | 一次 drive 以什么结束：结局、被抬起来的审批、代表桌 | `conclude` 10→3 |
| `Sweep` | drive 之后要收的东西：检查点、被抬起来的审批、job locator | `settle_desks` 11→4 |
| `Reach` | 这一轮活够得到谁：邻里与代表 | `status_tool` 7→4 |
| `Entered` | 一个人为接一个 endpoint 输入了什么：名字、base URL、兼容格式、凭证（`Credential` 枚举，不是「密钥＋鉴权头」两个 `Option`） | `endpoint_of` 5→1、`probe_endpoint` 5→1、`attach_endpoint` 6→2 |
| `Ceilings` | 一行模型声明的两个上限：上下文与最大输出（后者 `Option<Ceiling>`；人没填就沿用这个模型上一次登记的值，再退回目录行，见 §8-71） | `select_model` 5→4 |

`record_for` 的五参消得不需要新类型：`effect::Line` 已经装着 `who`／`addr`／`kind`／`data`，
调用点原本就在把它拆开再递进去，改成整份递。
`settle_requests` 与 `settle_desks` 另外收掉四个参数，因为 `who`／`run_id`／`write_root`／`building`
**本来就是 `Site` 的字段**，调用点在一个一个地从 `site` 里取出来递；`checkpoint_scope` 成为 `Site` 上的方法，
于是「检查点落在楼上还是落在房间上」在本模块只有一个答案。
三处 `#[expect(clippy::too_many_arguments)]` 随之报「这条压制没有被用到」而自己清掉——**修好之后压制自己消失，正是它该有的形状**。

### 十六个子模块，与两次为了行数之外的理由再切的缝

`assembly.rs` 756 行，十六个子模块各在 1000 行以内。其中两次是重新分配时切的，而切缝仍取自形状：
`settling` 里 `settle_requests` 回答的是「一个不能直接写的楼怎么收下这次改动」，那是评审与合并，成 `reviewing`；
`dispatching` 里 `wake`／`knock`／`answer_knocks` 回答的是「一个没在干活的居民怎么被叫起来」，成 `waking`。
`configure_building`／`create_building`／`adopt_building`／`startup_scan` 从动词表挪进 `genesis`：
**`form_city` 本来就在调 `adopt_building`**，一座城怎么长出楼、重启后看见什么，和一个人发一个动词不是一件事。

### 测试跟着它咬的那个模块，一份夹具留在父模块

`mod tests` 5,576 行，一百个测试函数。**先量后放**：3,490 行只用这个 crate 已经公开的面，
本来可以按 `assembly_door.rs` 的先例去 `crates/sprawling/tests/`。**没有那样做，理由是夹具。**
`fake_openai`（一台按脚本作答的 OpenAI 服务器）、`worker_with_provider`、`completion` 这一簇 443 行，
被两边同时需要：`what_a_worker_holds_is_what_a_restart_rebuilds` 要用它造一段历史再去核 `Standing::fold`，
而 `tests/` 里的验收测试也要用它。**`#[cfg(test)]` 的东西到不了 `tests/`，`tests/` 的东西到不了 `src/`**——
分家就要养两份同名夹具，那是一个夹具两个权威。

所以整套留在 `src/`：夹具成为 `accounting::worker::fixture`（父模块下的 `#[cfg(test)] mod`，十六个子模块都从 `super` 够得到），
每个测试搬到**它咬的那个模块**旁边。**crate 的公开面因此一个条目都没有增加**——
`Views`／`Standing`／`CommandDesk` 全部仍是 `pub(crate)`；`RunWorker` 的方法写在三个文件的三个 impl 块里，公开拼写不变。

### 验收

`cargo xtask length` 里 `assembly.rs` 的钉子被划掉而不是被调小；`[argument_count.predating]` 少十五行。
两者都是纯删除，所以 `guard::strikes_only_exemptions` 放行，不需要 `Verdict:` trailer；
budgets.toml 里那两段已经失真的注释单独一枚提交改，因为改注释会让豁免形状判定失效。

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
## 8-165 落点三 · Assembly 显式化：接线是一处，判定住 kernel，搬运是值

§8-28 留下的依据是「凡调用方仍在做区间比较、仍在记 `last_*` 的，皆是
epoch 机器要收走的东西」。这里把它收走，并连带回答那三句话。
**三句话各是一处代码动作，不多不少**：

### 接线留——`adapter_for` 搬出 `credentials.rs`

今天 `agree_to_work`（`dispatching.rs:252`）与 `name_the_work`
（`dispatching.rs:497`）各调一次 `self.adapter_for(&chosen)`，而
`adapter_for` 住在 `credentials.rs:581`——凭据簇里住着一条装配线。
搬家：`adapter_for(chosen, resolver)` 成为 `gateway` 的自由函数
（`endpoint/adapter.rs`，与 `Endpoint::new` 同簇），`resolver` 由调用方传入。
`credentials.rs` 留下 `resolver`（赎回闭包是凭据的形状），`dispatching`
的两个调用点各多传一个 `self.resolver()`。

**为什么是值参不是方法**：`adapter_for` 读的只有 `chosen` 与 `resolver`，
`self` 的其余 21 个字段与它无关；挂在 `RunWorker` 上等于说「装配需要整座城」。
搬出去后 `credentials.rs` 少一个 `impl RunWorker` 方法，多一个跨 crate 调用——
接线只有一处（`gateway::endpoint::adapter`），这就是「接线留」。

### 判定进 kernel——`halted_by` 的归属不变，调用点收敛

`halted_by` 住在 `commanding/governing.rs:45`（`pub(in crate::assembly)`），
读的是 `governance` 折叠（`HALTED`／`RELEASED`）。「判定进 kernel」的
含义经核对后收窄：停摆判定读的是**本进程的折叠状态**（`self.governance`），
不是纯函数能回答的问题；硬搬进 kernel 等于把 `Governance` 也搬过去，
那是另一步的事（`folds.rs` 957 行）。这里只做收敛：`halted_by` 的两个调用点
（`dispatching.rs:229` 与 gate 面）确认走同一函数——量过，只有一处定义，
调用点已收敛，**本句的验收是「无代码变更」，理由记在这里而不是被含糊过去**。

### 搬运下沉 adapter——`Driving.adapter` 由 `&mut dyn Model` 改为拥有值

今天 `Driving<'a>`（`driving.rs:28`）的 `adapter` 字段是 `&'a mut dyn Model`，
由 `dispatching.rs:350` 的 `site.adapter.as_mut()` 出借。`Agreed.adapter`
与 `Site.adapter` 是 `Box<dyn Model + Send>` 拥有值，`Driving` 是唯一的
出借点。搬运下沉：`Driving` 改为拥有 `Box<dyn Model + Send>`（调用点 `move`），
`drive_dispatch` 结束时把 `adapter` 还回——还法是 `Driven` 多一个字段
`adapter: Box<dyn Model + Send>`，调用方拆开归位（`Site` 字段名不增不减，
`adapter` 的类型由 `Box` 变为 `Option<Box>`——`Option` 是这次搬运的载具而非新状态，
跨过调用时两侧皆为 `Some`，`None` 不可观察；`None` 分支以 `E_CONFIG_INVALID` 拒绝告之而非 panic，
§8-40 的先例）。

**为什么**：`&mut` 出借把「谁拥有 adapter」这个问题悬在一次调用上；
拥有值随 `Driven` 回来，适配器的来去在类型上闭合——这就是「搬运下沉」，
与 §8-40 `Standing` 四样东西「拆开归位」的同一条道理。

### epoch 机器——`last_tick` 的区间比较收归一处

§8-28 的依据点名 `last_*`。量过：`last_tick` 是全仓唯一的 `last_*`
（`grep last_` 全仓仅 `assembly.rs:185` 定义＋`routing.rs` 读写＋测试）。
收走：`tick` 的「读表→判断→推进 `last_tick`」三步收成
`RunWorker::tick_after(now)` 仍三步，但 `last_tick` 的读写只在此一函数——
今天已是如此（`routing.rs` 的 `tick` 是唯一读写点），**本句的验收同样是
「无代码变更」**：epoch 机器的第一条轨道（到期判断下沉 `city`）已在上一步
落定，剩下的 `last_tick` 字段本身是 worker 状态而非散装轮询，
删它等于把「开机不补跑昨日」这个产品语义（§8-6）一并删掉，不删的理由在此。

### LOADING / UNLOADING 在哪

LOADING／UNLOADING 落在 `RunWorker::over`（`assembly.rs:239`）：
`Standing::fold` 即全量 LOADING（一次验证、三折叠，一句注释已写明），
而 UNLOADING 是 `close_city` 写 handoff（`assembly.rs:394`）。
两者皆已有名有主，不给它们改名——**给已存在的东西改名是第二权威，
§8-39 的教训**。这里只在 `over` 的 doc 上加一句：「此即 LOADING；
UNLOADING 见 `close_city`」，让设计里的词与代码的名在文档里相遇。

### `RunWorker` 立面只减不增

`adapter_for` 搬出后，`impl RunWorker` 方法数减一；`halted_by`／`tick`／
`last_tick` 零增；`Driving`／`Driven` 的字段变化是 `driving.rs` 内部形状，
不进立面：`sprawling` 的公开项一项不增；`commanding/governing` 与 `commanding/routing`
各多一个 `impl RunWorker` 块，是住处，不是接口。

### 验收

1. `gateway::endpoint::adapter::adapter_for` 新建，`dispatching` 两调用点
   传 `self.resolver()`；`credentials.rs` 的 `adapter_for` 删除。
   既有测试 `a_loopback_endpoint_with_a_credential_sends_it_on_every_call`
   与 `a_dispatch_without_a_provider_fails_saying_what_to_configure`
   逐字绿（它们咬的正是这条装配线）。
2. `Driving` 拥有 adapter，`Driven` 带回 adapter；`dispatching.rs:350`
   处拆开归位。`sprawling` 全绿。
3. `over` 的 doc 增 LOADING／`close_city` 互指一句；`halted_by`／`tick`／
   `last_tick` 零代码变更（本节即其理由）。
4. `just check` 绿；`sprawling` 基线按口径同集改写（只增减 impl 行）。

### 文档同步

本节；`ARCHITECTURE.md` §6（`gateway::endpoint::adapter` 新行；
`commanding::governing` 职责减一句）；gateway 规格的 endpoint 节（`crates/gateway/spec/Endpoint.lean`）记
`adapter_for` 的归属理由（装配线住适配器簇，凭据只出 `resolver`）。
-/

/-!
## 8-51 城的创世哈希一座城读一次（`bin::assembly` 与 `accounting::worker::workbench::standing`）

`storage::Provenance::city_of` 读的是账本首段的第一行，而 `provenance()` 每造一次署名就读一次：一次波里的每个检查点、每次落地、每次合并各读一次盘。**这个事实在一座城的一生里恒定不变**——创世行写下就不再改，改了那也不是同一座城。

**记在 `RunWorker` 上，用 `OnceLock` 惰性读一次**：不在 `over()` 里急读，因为一个刚被造出来、账本还空着的 worker 是合法状态（`RunWorker::new` 在一座尚未 init 的城上就是这样被测试用的），急读会把「还没有创世行」变成造不出 worker。

**缓存一份副本在这里不构成第二权威**：第二权威的危险来自**会变的**事实被抄了一份；创世哈希不会变。真正被消掉的风险是相反的一个——每次重读都可能读出不同的答案（有人换了账本），而一次运行里换了城的身份是比陈旧副本坏得多的事。

- **接口随之改形**：`workbench::standing::provenance` 与 `Site::provenance` 收 `city: B3Hash` 而不再收 `&Path`，于是「谁去读盘」这件事只剩 `RunWorker::city_hash` 一个答案，四个调用点都从它取。
- **作证方式**：城的创世哈希被读过一次之后，把账本首段从盘上删掉，`city_hash` 仍答同一个值——记住了才可能如此。
-/

/-!
## 8-134 页面读到历史证明到哪里（`accounting::views::city`、`bin::assembly::attending`；`crates/wire/Spec.lean` §8-63）

服务中的城在后台证明整条链（§8-90、§8-122），证明完成之前写者拒绝每一次追加。页面需要在同一刻知道这件事，否则一次被拒的命令看起来是坏了。`CityAnswer.proved` 回答它。

- **一个句柄，两个读者。** 写者线程挂上的 `storage::ChainHalt` 是判定的唯一一处：写者用它决定接不接一行，视图用它决定答不答「已证明」。`bin::assembly::attending` 在起写者之前造这只 halt，经 `RunWorker::chain_under_audit(halt)` 挂给写者，同一只的克隆经 `Views::watch_proof` 交给发布与备用两份视图。
- **答什么。** `halt.proved()` 为真时答视图此刻的头（`Some(head)`）：证明完成时写者还没写过一行，之后的每一行都由这个已证明的写者接上；为假（还在证明，或证明发现链断了）答 `None`。不经 halt 起步的视图（一次性查询、测试）起步前已同步证明过整条链，答它们的头。
- **不进账本。** 证明是这台主机对这份账本的一次核对，不是城的历史；它记在 `the history is proved` 那一行诊断里（§8-121），页面读的是此刻的判定。
- 验收：`accounting::views::city` 的 `a_city_answer_says_the_history_is_proved_only_once_the_halt_says_so`。
-/

/-!
## 8-56 说出来的那句话：录音进城，一行字出来（`accounting::views::hearing`、`bin::assembly::listening::hearing`；`crates/wire/Spec.lean` §8-27）

服务端此前只有 `gateway::transcribe` 这个适配器：一件没有任何路可以走到的东西。本节把路修通。

- **人填 URL 与 key 走既有的 attach 表单**。「哪个 endpoint、哪个 model 答这一类活」已有机制——`ModelTag`。第三个 tag `Transcribe` 因此是全部的新增面：第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。
- **`Views::transcriber`**（`views::hearing`）：锁内读出选择、造出 `Transcriber`，锁外发请求。一次转写是数秒，而那把锁是全部读的答案所在。录音到达的是城一级的门、身上没有地址，读不出任何一座楼的规矩，故这里**写明** `BuildingPolicy::new(false)` 而不是取默认值——把「口述按普通楼出门」这件事摆在读者眼前。上传带上它所属的那座楼之后，这个值同样从楼规来。
- **`assembly::listening::hearing`**：把 views 与金库收成一条 `TranscribeSink`。金库是**工人开的那一把**，经启动握手那条通道交出来（`Started.vault`）——第二个 `Custodian` 会是同一批机密的第二扇门。
- **容器从请求头读**：`AudioType::of_media_type` fail closed，拒词列出这座城发得出去的五种。浏览器录进它手上有的容器，而只有它知道是哪一个。
- **页面**：`core/speaking.ts` 管录音与上传，composer 多一个按钮，**转写结果落进输入框而不是直接发出去**——机器听错的那一句必须能改，否则它会花掉一次 run。没有 `transcribe` 选择的城不画这个按钮（`useHearing`）。
- **同一个选择也给 run 一件工具**：`transcribe`，有没有它读的是同一次 `select`，只是按 run 所在那座楼的楼规读（§8-131）。
- **验收**：`wire` 的 `/transcribe` 路由在没有 content-type 时按名拒绝；`gateway::transcribe::recording` 的 `of_media_type` 认得五种容器、拒第六种并列出前五种。
-/

/-!
## 8-102 开城时修过什么，要说给人（`accounting::worker::lifetime`、`accounting::worker::genesis`、`bin::assembly::attending`）

**原因**：`JsonlLedger::open` 在断尾恢复时截掉撑裂的尾行，并返回 `OpenReport`（`crates/storage/Spec.lean` §8-1）。账上虽然多了一行 `log_truncated`，但页面不画它，CLI 也不读它；`RunWorker::new` 与 `form_city` 把报告丢掉，人于是不知道上一次进程死时丢了几个字节。

**形状**：值（形状 2）。`RunWorker` 持有 `LedgerOpening`，它是 `OpenReport` 在本 crate 的类型化状态，只由 `LedgerOpening::from(OpenReport)` 生成：

```rust
pub(crate) enum LedgerOpening { Intact, TailDropped { bytes: u64 } }
impl From<storage::OpenReport> for LedgerOpening { … }
impl LedgerOpening {
    /// 给人看的一句：截掉了什么、为什么、怎么恢复；`Intact` 答 `None`。
    pub(crate) fn notice(self) -> Option<String>;
}
impl RunWorker {
    // 第四个参数就是 JsonlLedger::open 返回的那一对：账与它开时修过什么一起到，调用方没法只交一半。
    pub(crate) fn over(&Path, Custodian, Diagnostics, (JsonlLedger, OpenReport)) -> Result<Self, AxError>;
    pub(crate) fn opening(&self) -> LedgerOpening;
}
```

- **到人的两条路**：`sprawling resume` 的 `ScanReport::summary()` 在断尾时多一句 `notice()`；`sprawling serve` 的写者线程在开账之后、`open_for_service` 之前把同一句印到 stderr，先于横幅出现。两处读的都是同一个 `notice()`，措辞只有这一个来源。
- **`form_city` 不丢报告**：它只在目录没有账本时开账，报告因此恒为 `Intact`；它照样把 `open` 返回的那一对原样交给 `over`，让「worker 知道自己的账是怎么开的」对每条构造路径都成立，而不是靠一句注释说这里不会发生。
- **恢复**：截掉的是进程死时没写完的那一行，它之前的每一行都已按链校验。人要做的是确认最后一次动作是否需要重做；要逐字节看原状，就在再次打开之前从备份拷回 `.ledger`。
- **仍未到页面**：页面对 `log_truncated` 什么也不画（`client/src/core/belief.ts` 把它归进不显示的一组）。让城页说出这件事，需要一个视图字段与客户端的一个位置，这是本节接口尚未覆盖的一半。
- **不进公开面**：`ScanReport::summary()` 是跨出 crate 的唯一读法，`LedgerOpening` 因此留在 `pub(crate)`。
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
