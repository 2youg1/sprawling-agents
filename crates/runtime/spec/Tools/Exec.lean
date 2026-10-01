-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::exec

规定 `tools::exec`、`tools::exec::outcome`、`tools::exec::confinement`、`tools::exec::yielding`（`crates/runtime/src/` 下同名的文件）。exec 的三臂、宿主进程沙箱、派出的命令降一级与环境声明。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-13-2 runtime::tools::exec::confinement（宿主进程沙箱：按平台穷尽枚举＋每臂保证清单；形状 3＋2）


上面的 wasip1 面给的是 **guest** 的隔离；exec 的 program／shell 两臂跑的是**宿主进程**，它的隔离只能向平台买，而没有哪个平台把它全卖。故本模块的产物不是开关而是一份分类法：每一臂逐轴说出自己保什么、不保什么，跑在哪一臂上对人与 Agent 都可见（工具 disclosure＋doctor 回报），不是假定。

| 臂 | 保 | 不保 |
|---|---|---|
| `LinuxNamespaces { wrapper }` | 文件系统、网络、进程树、用户 | CPU／内存上限 |
| `WindowsJobObject` | 文件系统（工作目录为副本）、进程树、CPU／内存上限 | **网络**（作业对象不隔离网络）、用户 |
| `CopiedTree` | 文件系统（写入只落副本、源树只读） | 网络、进程树、用户、CPU／内存上限 |
| `Unavailable { missing }` | —— | 一切；`missing` 指名缺的是什么 |

```rust
pub enum Guarantee { Filesystem, Network, ProcessTree, User, Resources }
pub enum Kept { Yes, No }
pub struct Assurances { pub filesystem: Kept, pub network: Kept, pub process_tree: Kept, pub user: Kept, pub resources: Kept }
pub enum Missing { ScratchDirectory }
pub enum Confinement { LinuxNamespaces { wrapper: PathBuf }, WindowsJobObject, CopiedTree, Unavailable { missing: Missing } }
pub struct Offerings { pub namespace_tool: Option<PathBuf>, pub scratch: Option<PathBuf> }
impl Guarantee { pub const ALL: [Guarantee; 5]; pub fn phrase(self) -> &'static str; pub fn unkept(self) -> &'static str; }
impl Missing { pub fn phrase(self) -> &'static str; pub fn recovery(self) -> &'static str; }
impl Assurances { pub fn of(self, axis: Guarantee) -> Kept; }
impl Offerings { pub fn this_machine() -> Offerings; }   // 唯一的采样点；两个 detect 都只读它
impl Confinement { pub fn detect() -> Confinement; pub fn choose(&Offerings) -> Confinement;
                   pub fn assurances(&self) -> Assurances; pub fn name(&self) -> &'static str;
                   pub fn statement(&self) -> String; }
pub struct Confined { /* arm＋scratch＋idle（留给下一条命令的副本）＋outstanding（后台命令的副本） */ }
impl Confined { pub fn detect() -> Confined; pub fn with_arm(Confinement, Option<PathBuf>) -> Confined;
                pub fn arm(&self) -> &Confinement; pub fn statement(&self) -> String;
                pub fn place(&mut self, Command, &Path) -> Result<(Command, Placed), AxError>;
                pub fn settled(&mut self, Placed); pub fn handed(&mut self, BacklogId, Placed);
                pub fn reaped(&mut self, &[Finished]); }
impl Drop for Confined { /* 删留着的副本与未报结命令的副本 */ }
pub struct Placed { /* copy、work —— 私有 */ }
impl Placed { pub fn work(&self) -> storage::FileWork; }   // 这次同步的文件操作计数
pub enum Placement { Sandbox, Host }   // 调用参数 `where`；缺省 Sandbox
pub fn parse_placement(&Map<String, Value>) -> Result<Placement, AxError>;
```

- **保证清单在类型上**：`Assurances` 逐轴五字段，每一臂的 `assurances()` 必须写满五轴，故新增一轴即四臂同时编译红——任何一臂都不会留下一个没人问过它的旧答案。`statement()` 由 `Assurances` 与 `Guarantee::phrase()`／`unkept()` 派生而非另写一段话，句子与类型因此不可能分家。
- **选择是纯函数**：`choose` 取 `Offerings`——有 wrapper 即 `LinuxNamespaces`，否则 `CopiedTree`；scratch 根不可用即 `Unavailable { missing: ScratchDirectory }`。采样只有 `Offerings::this_machine()` 一处（`PATH`＋`std::env::temp_dir()`），两个 `detect()` 都只读它；测试用 `Offerings` 陈述一台机器而不是借一台。
- **`WindowsJobObject` 本构建不构造，且拒而不降级**：`CreateJobObject` 是 workspace `unsafe_code = forbid` 禁止的 FFI，唯一放开它的是桌面 server 的 FFI 缝（`crates/desktop/Spec.lean` D14），而那里只放 Zig 叶子的调用。以 `CopiedTree` 冒充它会对着一个开着网络的盒子回答「网络已关」，正是本模块存在的理由的镜像，故 `place()` 对它返 `E_SANDBOX_DENIED` 并给「改用 copied tree 且让命令离开网络」的 recovery。Windows 上 `detect()` 因此答 `CopiedTree`，其清单逐字写出网络未隔离——这一句就是 Agent 必须看见的那一句。
- **副本按工具一份，每条命令之前同步成工作目录此刻的样子，有界。** `place()` 取这个 `Confined` 留着的副本（没有，或留着的那份抄的是别的目录，就在 scratch 根下新建一个），把它同步成 `workdir` 此刻的样子，命令在副本里跑；`settled()` 在等待结束时把副本留给下一条命令（已留着一份时删掉这份）；交给 backlog 的后台命令由 `handed(id, …)` 记名，其成员报结时 `reaped(&[Finished])` 同样把副本留下或删掉；`Drop` 删掉留着的与未报结的副本。同步的规则：源侧跟随链接（指向树外的链接带进来的是内容，而不是通向人那棵树的入口）；副本侧不跟随链接，因为副本里的东西是上一条命令写的。副本里的一项与源不同类（命令把文件换成了链接、把目录换成了文件），或同名文件内容不同，就先删掉这一项再从源复制，落成一个新的目录项——命令可能在副本里造了指向别处的硬链接，就地改写会写穿到那一头；同名同类同长的文件逐字节比较，相同就不动；副本里源没有的项删掉。于是每条命令开始时，副本与工作目录逐文件相同，上一条命令写下的东西不会留给下一条。逐字节比较而不比 mtime 与长度：同一个时间戳刻度里的等长改写比不出来，副本就会为一个它没有带上的版本担保；比较只读两边的文件，不新建，而实时扫描等的是新建的文件。`Placed::work()` 报这次同步的 `storage::FileWork`（`crates/storage/Spec.lean` §8-31）：第一次放置 `created` 是树里的文件数，一次什么都没变的再放置 `created`、`rewritten`、`removed` 都是 0，`walked` 是两侧读过的目录项；`confinement::tests` 的 `a_sandbox_copy_is_synced_rather_than_made_again` 在 N 与 2N 个文件的树上断言这些数。**删不掉不把命令判成失败**（与 `backlog/member.rs`、`collect()` 同一条判断：命令的收场是调用方应得的事实，一个临时目录只值磁盘）。界：`MAX_FILES = 100_000`、`MAX_BYTES = 256 MiB`、`MAX_DEPTH = 64`，按源侧计；越界**拒**并报出越过的那一对数字，已同步一半的副本随之删掉——半份副本会为一堆没带上的文件担保，而本模块的全部理由是防这个。`MAX_DEPTH` 同时终结自指链接造成的无底走查。
- **`Mount`／`Fuel` 不沿用**：`Fuel` 是 wasmtime 指令计量、`Mount.guest` 是 guest 路径别名，二者 wasip1 专属。本模块保留的是**判断**（能力面＝能到达的路径集）而不是词形。宿主环境照旧不继承（exec 的 env allowlist 未动）；`SandboxJob.env` 的显式注入属 guest 面。
- **placement**：调用参数 `where: sandbox|host`，缺省 `sandbox`。`host` 是「在原地跑」——它才是碰得到人那棵树的那一臂，故必须由调用方按名说出，也正是与 A-8 同一条纪律（默认引导先在沙箱里做，出沙箱才需要审批）里「需要审批」的那个动作。python 臂无 host 形（它是 wasip1 guest）：要宿主解释器走 program 臂。
- **公开路径经 `runtime::tools`**：`confinement` 住 `tools/exec/`，doctor 的依赖回报与工具自己的 disclosure 都从 `runtime::tools::{Confinement, Guarantee, Kept, Missing}` 读这一份定义。
- 证据：`crates/runtime/src/tools/exec/tests.rs` 的 `a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone`（真命令、真树：沙箱里写得到、人那棵树不动；同一命令 `where: host` 则写进原树——此对拍使「没动」是能力判定而非命令没写）；`crates/runtime/src/tools/exec/confinement/tests.rs` 的 `the_sandbox_arm_a_machine_gets_is_chosen_from_what_it_has`、`every_sandbox_arm_states_what_it_does_not_hold`、`a_sandbox_refuses_a_tree_deeper_than_its_walk_can_end`、`a_sandbox_copy_is_synced_rather_than_made_again`、`a_sandbox_copy_goes_with_the_tool`。

**未决（§3 口径）**：同步仍按命令读两侧的每个目录项，并逐字节比较同长的文件，代价随工作树的大小长；一棵带大构建缓存的工作树每条命令要读两遍缓存。判定它的证据是一棵真实 room 的每条命令同步耗时（毫秒）与其文件数的读数；若读数显示读取成了主项，再比较「按 mtime 与长度跳过、只对同一时间戳刻度里的文件逐字节比较」。
-/

/-!
### 8-13-3 runtime::tools::exec::yielding（派出的命令降一级；形状 4 adapter）


agent 派出去的构建、测试与 sprawling 的记账、视图、socket 服务抢同一批核；控制面必须赢，所以 exec 派出的每一条宿主命令都以低于核心的优先级起动，而不是继承核心的优先级。本模块只做「把一条 `Command` 改成降一级起动」这一件事，不决定哪条命令要降——凡经 `through_the_backlog` 的命令一律降，那是全部宿主子进程的唯一产地（§8-14），故降级只有这一处权威。

```rust
pub(super) fn one_level_down(Command) -> Command;
```

- **Windows**：`BELOW_NORMAL_PRIORITY_CLASS`（`0x0000_4000`，`WinBase.h`），经标准库的安全接口 `std::os::windows::process::CommandExt::creation_flags` 在创建时给出，不需要管理员，也不需要 `unsafe`。是绝对档位而非相对档位：核心在正常档时，子进程低一档。
- **Unix**：包一层 `nice -n 10 -- <program> <args>`（`--` 结束 `nice` 的选项，名字以连字符开头的程序才不会被读成选项），工作目录与环境变量逐项搬到外层命令上。`nice` 是 POSIX 规定的工具；增量是相对的，所以子进程总比核心低 10 个 nice 单位，不需要 `CAP_SYS_NICE`（降优先级从不需要特权）。Linux 上核心的 `PATH` 里有 `ionice` 时，再在外面包一层 `ionice -c 2 -n 7`：IO 优先级取 best-effort 类里最低的一级，而不是 idle 类——idle 类在磁盘忙时可以让一条构建一个字节也读不到，best-effort 最低级只是排在核心之后。没有 `ionice`（例如不带 util-linux 的系统）时只包 `nice`：IO 这一半是两半里较轻的一半，缺了它不该让每条命令都起动失败。
- **次序**：降级在清环境与放置之前做，于是 `env_clear` 与环境白名单落在最外层命令上，沙箱的包装（`LinuxNamespaces` 的 wrapper）再包住降过级的命令；优先级沿进程树继承，所以 wrapper 下的真命令同样低一档。
- 失败：Unix 上 `nice` 自己总能起动，找不到的程序只会变成退出码 127 与 stderr 里的一行字，所以本模块在包 `nice` 之前先按 `execvp` 的找法（带分隔符的名字相对工作目录，裸名字沿核心的 `PATH`）确认程序是一个可执行文件，不是就返回 `E_TOOL_UNAVAILABLE`，动作与恢复同 backlog 的 spawn 失败（「check the program name, or use the shell arm」）。找不到 `nice` 本身时，spawn 在 backlog 里以同一个码报出。Windows 上不包外层，找不到程序仍在 spawn 处失败。Sandbox 放置下这一查找发生在宿主上，沙箱里看见的 `PATH` 若不同，结果以沙箱里的起动为准。
- 证据：`crates/runtime/src/tools/exec/tests/yielding.rs` 的 `a_dispatched_command_runs_below_the_core`——同一条读自身优先级的命令，直接起动一次、经 exec 起动一次，断言后者的档位严格低于前者；Linux 臂 `a_dispatched_command_reads_and_writes_at_the_lowest_best_effort_io_level`——经 exec 起动的 `ionice` 读回自己的 IO 档位是 `best-effort: prio 7`。这一条只在 Linux 上编译与运行，先红与转绿都在合并火车的 Linux 任务里看到。

**决定（平台调用的取法）**：子进程的 CPU 优先级取第一档「安全 Rust」——`creation_flags` 与 `nice` 都是对外只给安全接口的现成路，不需要 Zig 叶子，也不需要 Lean 证明边界。**被否**：①起动后再对子进程调 `SetPriorityClass`／`setpriority`——要 FFI（`unsafe` 或 Zig 叶子），且子进程在改档之前已经以正常档跑了一段；②Unix 上用 `CommandExt::pre_exec` 调 `nice(2)`——`pre_exec` 本身是 `unsafe`。**重开参数**：Unix 主机上出现不带 `nice` 的受支持平台，或测得多包一层 `nice` 的起动开销占到一条命令墙钟时间的可见比例。

**逐 run 的 Job Object（`runtime::backlog::jobs`，形状 4 adapter）**：派出的命令起动之后，谁在吃内存要能归到派出它的 run，而一条 `cargo test` 真正吃内存的是它起的 `rustc` 与测试进程，不是 `cargo` 自己。所以 Windows 上每个 run 一个匿名 Job Object：`Backlog::run` 起动的子进程在登记进表的同一时刻装进它 owner 的 job（第一次装时创建），job 里的进程再起的进程由系统自动装进同一个 job，于是 job 的进程表就是这个 run 的整棵进程树。`Backlog::release(owner)` 丢掉这只 job 的句柄；job 不设 kill-on-close，丢句柄不杀进程，杀进程仍只归 `release` 与 `halt`。

```rust
pub struct RunProcesses { pub pids: BTreeSet<u32>, pub unfollowed: u32 }
impl Backlog { pub fn processes(&self) -> Result<BTreeMap<RunId, RunProcesses>, AxError>; }
```

- `processes` 按 run 给出它此刻在表里的进程：owner 是这个 run（窗口内或已转后台）的每条命令自己的 pid，并上这个 run 的 job 的进程表（job 只列还活着的进程）。已经结束、还没被 `harvest` 的命令仍列出自己的 pid，读数的一方在它后面读不到计数。已 `release` 的命令（owner 为 nobody）不归任何 run。
- `unfollowed` 是这个 run 的命令里有几条只读到了命令本身、没读到它起的进程：Windows 上创建 job、装进 job 或读 job 的进程表失败的那几条（这一次读数里整个 run 的命令都算），Unix 上是每一条，因为 Unix 上没有 Job Object（进程组是它的对应物，尚未接入）。装不进 job 不让命令起动失败：job 只服务于读数，为读数让一条构建失败是把代价付错了地方，失败落在 `unfollowed` 里给读数的人看。
- 失败：表够不着时 `E_STORAGE_FATAL`，与 `Backlog` 的其他读法相同。
- 每个进程的内存与 CPU 不在这里读：本 crate 不读平台计数（见下），由 sprawling 的 `bin::monitor` 按这里给出的 pid 去读。
- 证据：`crates/runtime/src/backlog/tests.rs` 的 `a_run_owns_the_processes_its_commands_started`——一条经 `run` 转后台的命令，其 pid 出现在它 owner 的那一项里，别的 run 那一项里没有；Windows 上 `unfollowed` 为 0；`release` 之后这个 run 不再出现。

**决定（job 的取法）**：Job Object 经 `win32job` 2.0.3（`Job::create`、`assign_process`、`query_process_id_list`，对外只给安全接口），本 crate 不写 `unsafe`；子进程的句柄经标准库的 `AsRawHandle` 取得。**被否**：①一整座城一只 job——分不出 run，而分解到 run 正是要它的原因；②只记直接子进程的 pid——`cargo`、`npm`、`sh -c` 这类命令自己几乎不占内存，读数会把一条吃掉几 GiB 的构建报成几 MiB；③`CREATE_SUSPENDED` 起动再装 job 再恢复——恢复线程要 FFI。代价：子进程从起动到装进 job 之间有一小段时间，那一段里它再起的进程不在 job 里（`cmd /C` 这类命令的第一个孙进程在这一段里起动的可能很小，但不为零）。**重开参数**：一个对外只给安全接口的 crate 能以挂起态起动子进程并在恢复前装进 job，或者读数显示 `unfollowed` 之外还有漏掉的孙进程。

**未决（§3 口径）**：核心线程升到正常档之上一级与空转安全阀在 sprawling-SPEC §8-93；派出进程的内存上限尚未落地，卡在一个事实上：`win32job` 2.0.3 对外只公开 job 的工作集上限（`limit_working_memory`，即 `JOB_OBJECT_LIMIT_WORKINGSET`，限的是常驻而不是提交量，按进程计），而在未提权的账户下设这一项被系统拒绝——`SetInformationJobObject` 返回 `ERROR_PRIVILEGE_NOT_HELD`（os error 1314，「客户端没有所需的特权」），普通账户的令牌里没有这一项要的特权，启用特权要 `AdjustTokenPrivileges`，是 FFI。于是这条路在普通账户上让每条派出命令都起动失败，或者静默不设上限，两者都不可取。作业级提交上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`）不要特权，但设它的字段在 `win32job` 里是 crate 私有的，`process-wrap` 10.0.1 与 `windows-spawn` 0.1.0 也不设它。重开参数：一个对外只给安全接口的 crate 公开 `JOB_OBJECT_LIMIT_JOB_MEMORY` 或 `JOB_OBJECT_LIMIT_PROCESS_MEMORY`，或者这一处系统调用改走平台调用规则的第二档（Zig 叶子）。重命令共用的额度池尚未落地：池的大小由测得的核数与可用内存推出，哪些命令算重由城配置给默认表；可用内存的读数只在 sprawling 的 `bin::monitor::memory` 里读（sprawling-SPEC §8-94），由 `bin::assembly` 交给本 crate，本 crate 不读平台。内存紧时新 run 排队已在 sprawling-SPEC §8-46-3。
-/

/-!
### 8-26 runtime::tools::exec 目录化


**「发生了什么」与「怎么写下来」分家。** 三条臂判定发生了什么，结果载荷的键名（`arm`／`stdout`／`stderr`／`exit_code`／`outcome`／`handle`／`what`／`detail`／`background`／`env`）是另一件事：它们必须一处定义，否则一条臂可以把结果拼得跟邻居不一样。两者同处一个文件时 `exec.rs` 越过了 400 行上限。`ExecTool::new` 的 7 参豁免键 `crates/runtime/src/tools/exec.rs::new` 仍指着它原来的文件。

| 文件 | 管什么 |
|---|---|
| `crates/runtime/src/tools/exec.rs` | 三臂本身：`ExecTool`（`new` 与 `run_program`／`run_python`／`run_shell`／`through_the_backlog`／`inherited_environment`）、环境白名单 `ENV_ALLOWLIST`、臂解析 `parse_arm`，以及 `impl Tool for ExecTool` 的路由。它声明 `mod outcome;` 并 `use outcome::{backgrounded, exceptional, settled, with_backlog, with_environment};` |
| `crates/runtime/src/tools/exec/outcome.rs` | 这把工具能给出的每一种回答的形状：`settled`（原名 `outcome`，模块名占了这个词故改用动词过去式）、`backgrounded`、`exceptional` 三种载荷，以及 `with_backlog`／`with_environment` 两条尾巴。全部 `pub(super)`，不出 `tools::exec` |
| `crates/runtime/src/tools/exec/tests.rs` | 每条臂对调用方的承诺：缺件时点名替代方案而拒绝、python 臂在沙盒里跑并报自己的退出码、燃料耗尽与 trap 原样抵达、无法识别的臂只拒不猜、program 臂真起子进程且环境被洗|
-/

/-!
### 8-31 runtime::tools::exec 的环境声明


> 权威在 kernel-SPEC §8-22 的「11.1 增」段与 city-SPEC §8-4 的「11.1 增」段；本节只说 exec 这一侧怎么用它，以及构造面因此怎么变。

**只放四个名字的后果**：`ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"]` 加 `env_clear()` 时，城里的 resident 跑不动 `cargo build`——MSVC 链接器读不到 `%ProgramFiles(x86)%`，退回裸 `link.exe`，撞上 PATH 上 Git 那个 coreutils `link`。

**改法**：`ENV_ALLOWLIST` 原样留着，它是**每一栋楼无条件继承的地板**；楼在 `[sandbox] env_passthrough` 里逐名声明的是**地板之上加的那几个**。两份清单合并后按名取值，取不到的名字不进（一个没设过的变量不该变成空串——空串与未设置在 Windows 上是两件事）。

**构造面**：总在一起走的那几个值是一个有名字的值，`ExecTool::new` 因此只收三个参数：

```rust
pub struct ExecSetup {                 // 形状 2 值类型
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Option<PathBuf>,
    pub fuel: Fuel,
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: Address,
    pub run: RunId,                    // 这张工具台服务的 run：它起的后台命令只交还给它（§8-28-1 第 4 条）
}
pub fn new(setup: ExecSetup, sandbox: Box<dyn Sandbox>, backlog: Backlog) -> Result<ExecTool, AxError>;
```

三个参数，在函数参数上限之内。

**账本上留什么**：配置写入时不记事件——`CONFIG.toml` 是「一跑受什么治理」的权威，再记一条同事实的事件就是第二个权威。账本记的是**一跑拿它做了什么**：`exec` 的结果载荷增 `env` 字段，列出这次真正递给子进程的名字，按名排序。名字不是值——值恒不入账本。

**验收**：声明了名字的楼里，`exec` 的子进程恰好看到那几个（加地板四个）；没声明的楼里只看到地板四个；凭据形状的名字在配置解析处被拒。以及一次真实演示：声明了名字的楼里 `exec` 跑得动 `cargo build`。
-/

/-! D10 沙箱副本按工具一份、每条命令前同步，而不是每条命令新建一份

**决定**：`Confined` 留着一份副本，`place()` 把它同步成工作目录此刻的样子：不同的文件删掉再复制，多出来的项删掉，相同的文件不动（§8-13-2）。

**理由**：新建文件是实时扫描等待的地方。在 Windows x86_64 笔记本级、Defender 实时防护开的机器上，放置一棵 512 个 16 KB 文件的树 p50 约 0.9 s，约 1.8 ms/文件，磁盘空闲 98% 以上（这是工作树放置的读数；复制同样多的文件是同一种等待，这一点是推断）；每条沙箱命令复制整棵树，就是每条命令付这一次。同步之后，一条命令新建与改写的文件数等于上一条命令动过的文件加上工作目录变了的文件，与树的大小无关；树的大小只进目录项读取与同长文件的比较。

**被否**：①每条命令新建一份（原做法）：新建文件数是命令数乘以树的文件数；②用硬链接「复制」：命令在副本里的写入会写穿到人那棵树，共享可写文件的副本不是隔离；③按 mtime 与长度判定相同：同一个时间戳刻度里的等长改写判不出来，副本会为它没带上的版本担保；④就地改写不同的文件：命令可能把副本里的名字做成指向别处的硬链接或链接，就地写会写到那一头，所以不同的项先删掉再建。

**重开参数**：一个工具的两条命令需要同时持有同一份副本（今天后台命令各持一份，留着的只有一份）；或读数显示同步的读取成了每条命令的主项（§8-13-2 的未决）。
-/

/-! D11 「只新建」下 exec 只在副本里跑，不开放 host

**决定**：`WriteLimit::Create` 下，exec 的 `host` 放置在起进程之前被拒；`sandbox` 放置照常（§8-55）。

**理由**：host 上的一条命令能覆盖、删除、改名任何它够得到的文件，也能建硬链接，而不要管理员权限的前提下，没有一种手段能让已有文件对它只读；`Create` 承诺的是「已有文件不变」，给不出这个保证的路就不开放写入。副本放置本来就把写入留在副本里，所以它在 `Create` 下的行为与原来相同，什么都不必改。

**被否**：①在 host 上跑完再比对、改了就回滚：回滚发生在改动之后，期间读到这个文件的人与进程看到的是改过的字节，而且一个被删掉又建回来的文件已经不是原来那个目录项；②把命令新建的文件从副本搬回树上：要给副本里每个新文件再判一遍写域与「只新建」，这是第二条写路径，今天没有读者要它。

**重开参数**：出现一个能让已有文件只读的放置（例如 `LinuxNamespaces` 臂把工作目录只读挂进去）时，那个臂在 `Create` 下可以开放；出现要在 `Create` 下由命令产出新文件的场景时，重议第②条。
-/
