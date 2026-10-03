-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::exec

规定 `tools::exec`、`tools::exec::outcome`、`tools::exec::shell`、`tools::exec::confinement`、`tools::exec::yielding`（`crates/runtime/src/` 下同名的文件）。exec 的三臂、宿主进程沙箱、派出的命令降一级与环境声明。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `tools::exec`、`backlog` 旁的测试守住（`crates/runtime/Spec.lean` §16）。
-/

/-!
### 8-13-2 runtime::tools::exec::confinement（宿主进程沙箱：按平台穷尽枚举＋每臂保证清单；形状 3＋2）


上面的 wasip1 面给的是 **guest** 的隔离；exec 的 program／shell 两臂跑的是**宿主进程**，它的隔离只能向平台买，而没有哪个平台把它全卖。故本模块的产物不是开关而是一份分类法：每一臂逐轴说出自己保什么、不保什么，跑在哪一臂上对人与 Agent 都可见（工具 disclosure＋doctor 回报），不是假定。

| 臂 | 保 | 不保 |
|---|---|---|
| `LinuxNamespaces { wrapper }` | 文件系统、网络、进程树、用户（见下文 `--unshare-all` 一条） | CPU／内存上限；整机对命令只读可见（`--ro-bind / /`），读不受限 |
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
- **`LinuxNamespaces` 的「用户」一轴今天是有条件的**：`namespaced()` 给 `bwrap` 的是 `--unshare-all`，而 bubblewrap 的手册写明它等于 `--unshare-user-try --unshare-ipc --unshare-pid --unshare-net --unshare-uts --unshare-cgroup-try`：内核不让未提权的进程建 user namespace、`bwrap` 又以 setuid 方式装着时（bubblewrap README 说它历史上支持这种方式），user namespace 静默不建，清单上的 `user: Yes` 就是对一个与 harness 同一身份的盒子说「有自己的身份」。规格的意图是清单只写兑现得了的轴，所以这是代码的缺陷，修法列在下文「SB1 未落地的工作」第 1 条。`--ro-bind / /` 让整机对命令只读可见：「文件系统」一轴的含义是写入只落副本（`Guarantee::Filesystem` 的文档），不是读不到别处，这一点 `statement()` 的句子与本表都照实写。
- **选择是纯函数**：`choose` 取 `Offerings`——有 wrapper 即 `LinuxNamespaces`，否则 `CopiedTree`；scratch 根不可用即 `Unavailable { missing: ScratchDirectory }`。采样只有 `Offerings::this_machine()` 一处（`PATH`＋`std::env::temp_dir()`），两个 `detect()` 都只读它；测试用 `Offerings` 陈述一台机器而不是借一台。wrapper 是按确切名字 `bwrap` 在 `PATH` 上找到的程序，只在 Linux 上找（`cfg!(target_os = "linux")`），别的平台 `namespace_tool` 恒为 `None`。
- **逐平台的臂**：Linux 上 `PATH` 里有 `bwrap` 得 `LinuxNamespaces`（副本加 wrapper 给的命名空间），没有得 `CopiedTree`；Windows 上恒得 `CopiedTree`（`WindowsJobObject` 不构造，见下条）；macOS 上今天没有任何平台隔离接入，恒得 `CopiedTree`。三个平台上 temp 目录不可用都得 `Unavailable { missing: ScratchDirectory }`，`place()` 拒。缺的是哪一轴，User 与 Agent 都从同一句 `statement()` 读到：它进 exec 工具的 disclosure 与 doctor 的回报，`CopiedTree` 的那一句逐字写出网络、进程树、用户与资源上限都不保。
- **`WindowsJobObject` 本构建不构造，且拒而不降级**：Job Object 本身已经可以不写 `unsafe` 地取得——`runtime::backlog::jobs` 经 `win32job` 2.0.3 的安全接口创建 job、装进进程、读进程表（下文「job 的取法」）。不构造的理由是这一臂的保证清单今天兑现不了两轴：①**资源**——`win32job` 2.0.3 的 `ExtendedLimitInfo` 对外只给按进程的工作集上限（`limit_working_memory`；经 `win32job` 设它在一台未提权账户上被系统以 os error 1314 拒绝，而 D32 的探测在另一台未提权的 Windows 11 上直接调 `SetInformationJobObject` 设 `JOB_OBJECT_LIMIT_WORKINGSET` 得到成功——要不要特权随账户令牌而变，推断是 `SeIncreaseWorkingSetPrivilege` 在不在令牌里；无论哪种，它限的是常驻页而不是提交量，不兑现「资源」一轴）、优先级档位（`limit_priority_class`）、调度级别（`limit_scheduling_class`）、亲和性（`limit_affinity`）与 kill-on-close、breakaway 两组开关，作业级提交上限的字段在 crate 私有的结构里，CPU 速率控制它根本不设（D29 说这两项改走哪一档），清单写 `resources: Yes` 就是对一个没有上限的盒子说「有上限」；②**进程树**——子进程起动之后才装进 job，中间一小段里起的孙进程不在 job 里（下文「job 的取法」的代价），`limit_kill_on_job_close` 收不到它。以 `CopiedTree` 冒充这一臂会对着一个开着网络的盒子回答「网络已关」的同类错误，故 `place()` 对它返 `E_SANDBOX_DENIED` 并给「改用 copied tree 且让命令离开网络」的 recovery。Windows 上 `detect()` 因此答 `CopiedTree`，其清单逐字写出网络未隔离——这一句就是 Agent 必须看见的那一句。
- **副本按工具一份，每条命令之前同步成工作目录此刻的样子，有界。** `place()` 取这个 `Confined` 留着的副本（没有，或留着的那份抄的是别的目录，就在 scratch 根下新建一个），把它同步成 `workdir` 此刻的样子，命令在副本里跑；`settled()` 在等待结束时把副本留给下一条命令（已留着一份时删掉这份）；交给 backlog 的后台命令由 `handed(id, …)` 记名，其成员报结时 `reaped(&[Finished])` 同样把副本留下或删掉；`Drop` 删掉留着的与未报结的副本。同步的规则：源侧跟随链接（指向树外的链接带进来的是内容，而不是通向人那棵树的入口）；副本侧不跟随链接，因为副本里的东西是上一条命令写的。副本里的一项与源不同类（命令把文件换成了链接、把目录换成了文件），或同名文件内容不同，就先删掉这一项再从源复制，落成一个新的目录项——命令可能在副本里造了指向别处的硬链接，就地改写会写穿到那一头；同名同类同长的文件逐字节比较，相同就不动；副本里源没有的项删掉。于是每条命令开始时，副本与工作目录逐文件相同，上一条命令写下的东西不会留给下一条。逐字节比较而不比 mtime 与长度：同一个时间戳刻度里的等长改写比不出来，副本就会为一个它没有带上的版本担保；比较只读两边的文件，不新建，而实时扫描等的是新建的文件。`Placed::work()` 报这次同步的 `storage::FileWork`（`crates/storage/Spec.lean` §8-31）：第一次放置 `created` 是树里的文件数，一次什么都没变的再放置 `created`、`rewritten`、`removed` 都是 0，`walked` 是两侧读过的目录项；`confinement::tests` 的 `a_sandbox_copy_is_synced_rather_than_made_again` 在 N 与 2N 个文件的树上断言这些数。**删不掉不把命令判成失败**（与 `backlog/member.rs`、`collect()` 同一条判断：命令的收场是调用方应得的事实，一个临时目录只值磁盘）。界：`MAX_FILES = 100_000`、`MAX_BYTES = 256 MiB`、`MAX_DEPTH = 64`，按源侧计；越界**拒**并报出越过的那一对数字，已同步一半的副本随之删掉——半份副本会为一堆没带上的文件担保，而本模块的全部理由是防这个。`MAX_DEPTH` 同时终结自指链接造成的无底走查。
- **`Mount`／`Fuel` 不沿用**：`Fuel` 是 wasmtime 指令计量、`Mount.guest` 是 guest 路径别名，二者 wasip1 专属。本模块保留的是**判断**（能力面＝能到达的路径集）而不是词形。宿主环境照旧不继承（exec 的 env allowlist 未动）；`SandboxJob.env` 的显式注入属 guest 面。
- **placement**：调用参数 `where: sandbox|host`，缺省 `sandbox`。`host` 是「在原地跑」——它才是碰得到人那棵树的那一臂，故必须由调用方按名说出，也正是与 A-8 同一条纪律（默认引导先在沙箱里做，出沙箱才需要审批）里「需要审批」的那个动作。python 臂无 host 形（它是 wasip1 guest）：要宿主解释器走 program 臂。
- **公开路径经 `runtime::tools`**：`confinement` 住 `tools/exec/`，doctor 的依赖回报与工具自己的 disclosure 都从 `runtime::tools::{Confinement, Guarantee, Kept, Missing}` 读这一份定义。
- 证据：`crates/runtime/src/tools/exec/tests.rs` 的 `a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone`（真命令、真树：沙箱里写得到、人那棵树不动；同一命令 `where: host` 则写进原树——此对拍使「没动」是能力判定而非命令没写）；`crates/runtime/src/tools/exec/confinement/tests.rs` 的 `the_sandbox_arm_a_machine_gets_is_chosen_from_what_it_has`、`every_sandbox_arm_states_what_it_does_not_hold`、`a_sandbox_refuses_a_tree_deeper_than_its_walk_can_end`、`a_sandbox_copy_is_synced_rather_than_made_again`、`a_sandbox_copy_goes_with_the_tool`。

这一臂何时构造、构造时清单写什么，见 D29 的 (e)：资源一轴由 D29 的 job 限额兑现，进程树一轴要挂起态起动，两者都落在同一个 Zig 叶子上之后按原清单构造，不缩成一个只保一部分的臂。

**SB1 未落地的工作**（D32 选定的臂；W6b 的 SB1 实现，按这里的顺序）：

1. **`LinuxNamespaces` 的用户一轴改成强制**：`namespaced()` 在 `--unshare-all` 之后显式加 `--unshare-user`（bubblewrap 对同一命名空间的「try」与强制形取强制形；推断，需在 Linux runner 上以 `bwrap --unshare-all --unshare-user -- true` 核实组合被接受），建不成 user namespace 时 `bwrap` 失败，`place()` 把它报成 `E_SANDBOX_DENIED` 并说出缺的是未提权的 user namespace，而不是静默少一轴。有的发行版以 LSM 策略限制未提权的 user namespace（推断，本调研没有读到发行版的文档），GitHub 的 ubuntu runner 上这一臂能不能起是 CI 要读的第一件事。
2. **`[sandbox] arm` 的解析与解出**：键与五个拼写已由 `crates/wire/spec/Answer/Doctor.lean` D26 定，缺省按平台取那里的表；`kernel::config::SandboxLimits` 加 `arm`（规格的所有者 `crates/kernel/spec/Config.lean` §8-22），未知拼写 `E_CONFIG_INVALID`。`Offerings` 加各机制的采样（Linux 的 `bwrap`、Landlock ABI；Windows 的 AppContainer 与 job 叶子是否在本构建里；三个平台上 `docker`／`podman` 是否在 `PATH` 上且 `version` 应答），`choose` 改为取（名字，`Offerings`）给出 `Confinement`：缺省名字的机制缺席时退到 `CopiedTree`，并在 `statement()` 与 doctor 行里说出「缺省的 native 不可用：缺 X」；User 明写的名字的机制缺席时答 `Unavailable { missing }`，`place()` 拒。`Missing` 随之加各机制的缺项（`NamespaceWrapper`、`UserNamespace`、`JobLeaf`、`ContainerRuntime`），每一项一句 `phrase` 与一句 `recovery`，要另装的给安装指引。
3. **Windows 的 `native`**：D29 (e) 的挂起态起动与恢复落在 `crates/desktop/ffi` 的同一个 Zig 叶子上之后构造 `WindowsJobObject`；同一个叶子再加「以 AppContainer 身份起动」（`CreateProcessW` 带 `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`，零个 capability 即无网络），副本目录给这个容器的 SID 加上读写的 ACL。exec 的命令在两者之下：文件系统、网络、进程树、用户、资源五轴都保；harness 居民只进 job、不进 AppContainer（它要连 provider，网络必须开），清单照实写网络与用户不保。叶子的边界性质在 Lean 里证、与 Rust 参照做等价性质测试并两侧 fuzz（AGENTS.md 的平台调用档位）。
4. **`container` 臂**：`docker`／`podman` 二者取先应答的一个，`run --rm --network none --user <非零> --cpus --memory -v <副本>:<工作目录>`，镜像名由城配置给出、缺省不预拉；五轴都保。要另装 Docker Desktop、Podman Desktop 或 rootless Podman，安装指引指向 D32 引的安装页。
5. **设置的「沙箱」控件**：列出这座城所在的电脑解得出的每个名字与它的五轴清单（读 doctor 的同一份 `DoctorSandbox`，不另算），缺的给 `Missing` 的那一句与安装指引；选中写 `[sandbox] arm`（经设置的那扇门）。
6. **doctor 每臂一行**：`crates/sprawling/spec/Doctor.lean` 的 doctor 表为 `native`、`container`、`python` 各加一行，读 `Offerings` 的同一次采样。
7. **每条保证的测试**：每个构造得出的臂，五轴各一个真命令的对拍——写工作目录（副本里有、源树没有）、连回环上一个监听的端口（开网的臂连得上、关网的臂连不上）、起一个比命令活得久的孙进程（进程树轴保时命令结束后它不在）、读自己的身份（用户轴保时与 harness 不同）、分配超过上限的内存（资源轴保时失败）；与 `every_sandbox_arm_states_what_it_does_not_hold` 并列，清单与对拍不一致即红。
8. **要另装的臂实跑一次**：`container` 在 GitHub 的 ubuntu runner（自带 Docker）上跑第 7 条的对拍；Linux 的 Landlock 回退与 macOS 的 Seatbelt 在 D32 的未决解开之前不构造。
9. 本模块文档的链接 `[placing::copy](crate::tools::Confinement)` 指向了枚举而不是副本所在的模块，随第 2 条一起改正。

**未决（§3 口径）**：同步仍按命令读两侧的每个目录项，并逐字节比较同长的文件，代价随工作树的大小长；一棵带大构建缓存的工作树每条命令要读两遍缓存。判定它的证据是一棵真实 room 的每条命令同步耗时（毫秒）与其文件数的读数；若读数显示读取成了主项，再比较「按 mtime 与长度跳过、只对同一时间戳刻度里的文件逐字节比较」。
-/

/-!
### 8-13-3 runtime::tools::exec::yielding（派出的命令降一级；形状 4 adapter）


agent 派出去的构建、测试与 sprawling 的记账、视图、socket 服务抢同一批核；控制面必须赢，所以 exec 派出的每一条宿主命令都以低于核心的优先级起动，而不是继承核心的优先级。本模块只做「把一条 `Command` 改成降一级起动」这一件事，不决定哪条命令要降——凡经 `through_the_backlog` 的命令一律降，那是全部宿主子进程的唯一产地（§8-14），故降级只有这一处权威。

```rust
pub(super) fn one_level_down(Command) -> Command;
```

- **Windows**：`BELOW_NORMAL_PRIORITY_CLASS`（`0x0000_4000`，`WinBase.h`），经标准库的安全接口 `std::os::windows::process::CommandExt::creation_flags` 在创建时给出，不需要管理员，也不需要 `unsafe`。是绝对档位而非相对档位：核心在正常档时，子进程低一档。
- **Unix**：包一层 `nice -n 10 -- <program> <args>`（`--` 结束 `nice` 的选项，名字以连字符开头的程序才不会被读成选项），工作目录与环境变量逐项搬到外层命令上。`nice` 是 POSIX 规定的工具；增量是相对的，所以子进程总比核心低 10 个 nice 单位，不需要 `CAP_SYS_NICE`（降优先级从不需要特权）。Linux 上核心的 `PATH` 里有 `ionice` 时，再在外面包一层 `ionice -c 2 -n 7`：IO 优先级取 best-effort 类里最低的一级，而不是 idle 类——idle 类在磁盘忙时可以让一条构建一个字节也读不到，best-effort 最低级只是排在核心之后。没有 `ionice`（例如不带 util-linux 的系统）时只包 `nice`：IO 这一半是两半里较轻的一半，缺了它不该让每条命令都起动失败。macOS 上 `/usr/sbin/taskpolicy` 在时，再在外面包一层 `taskpolicy -c utility`，把命令与它的后代的 QoS 压到 utility，系统先把它们放到效率核上（D29）；不在时只包 `nice`，理由同 `ionice`。
- **次序**：降级在清环境与放置之前做，于是 `env_clear` 与环境白名单落在最外层命令上，沙箱的包装（`LinuxNamespaces` 的 wrapper）再包住降过级的命令；优先级沿进程树继承，所以 wrapper 下的真命令同样低一档。
- 失败：Unix 上 `nice` 自己总能起动，找不到的程序只会变成退出码 127 与 stderr 里的一行字，所以本模块在包 `nice` 之前先按 `execvp` 的找法（带分隔符的名字相对工作目录，裸名字沿核心的 `PATH`）确认程序是一个可执行文件，不是就返回 `E_TOOL_UNAVAILABLE`，动作与恢复同 backlog 的 spawn 失败（「check the program name, or use the shell arm」）。找不到 `nice` 本身时，spawn 在 backlog 里以同一个码报出。Windows 上不包外层，找不到程序仍在 spawn 处失败。Sandbox 放置下这一查找发生在宿主上，沙箱里看见的 `PATH` 若不同，结果以沙箱里的起动为准。
- 证据：`crates/runtime/src/tools/exec/tests/yielding.rs` 的 `a_dispatched_command_runs_below_the_core`——同一条读自身优先级的命令，直接起动一次、经 exec 起动一次，断言后者的档位严格低于前者；Linux 臂 `a_dispatched_command_reads_and_writes_at_the_lowest_best_effort_io_level`——经 exec 起动的 `ionice` 读回自己的 IO 档位是 `best-effort: prio 7`。这一条只在 Linux 上编译与运行，先红与转绿都在合并火车的 Linux 任务里看到。

**决定（平台调用的取法）**：子进程的 CPU 优先级取第一档「安全 Rust」——`creation_flags` 与 `nice` 都是对外只给安全接口的现成路，不需要 Zig 叶子，也不需要 Lean 证明边界。**被否**：①起动后再对子进程调 `SetPriorityClass`／`setpriority`——要 FFI（`unsafe` 或 Zig 叶子），且子进程在改档之前已经以正常档跑了一段；②Unix 上用 `CommandExt::pre_exec` 调 `nice(2)`——`pre_exec` 本身是 `unsafe`。**重开参数**：Unix 主机上出现不带 `nice` 的受支持平台，或测得多包一层 `nice` 的起动开销占到一条命令墙钟时间的可见比例。

**逐 run 的 Job Object（`runtime::backlog::jobs`，形状 4 adapter）**：派出的命令起动之后，谁在吃内存要能归到派出它的 run，而一条 `cargo test` 真正吃内存的是它起的 `rustc` 与测试进程，不是 `cargo` 自己。所以 Windows 上每个 run 一个匿名 Job Object：`Backlog::run` 起动的子进程在登记进表的同一时刻装进它 owner 的 job（第一次装时创建），job 里的进程再起的进程由系统自动装进同一个 job，于是 job 的进程表就是这个 run 的整棵进程树。`Backlog::release(owner)` 丢掉这只 job 的句柄；job 不设 kill-on-close，丢句柄不杀进程，杀进程仍只归 `release` 与 `halt`。macOS 与 Linux 上没有 Job Object：今天两者都只读到命令自己的 pid（下面的 `unfollowed` 计每一条）；按 run 的 CPU 份额在 macOS 上是 `taskpolicy -c utility`，在 Linux 上是 cgroup v2 的 `cpu.weight`，都在 D29，进程组作为读数的对应物尚未接入。

```rust
pub struct RunProcesses { pub pids: BTreeSet<u32>, pub unfollowed: u32, pub share: CpuShare }
pub enum CpuShare { Weighted, Unset }   // 这个 run 的 job 是否带着每个 run 都一样的 CPU 权重（D29）；缺省 Unset
impl Backlog { pub fn processes(&self) -> Result<BTreeMap<RunId, RunProcesses>, AxError>; }
```

- `processes` 按 run 给出它此刻在表里的进程：owner 是这个 run（窗口内或已转后台）的每条命令自己的 pid，并上这个 run 的 job 的进程表（job 只列还活着的进程）。已经结束、还没被 `harvest` 的命令仍列出自己的 pid，读数的一方在它后面读不到计数。已 `release` 的命令（owner 为 nobody）不归任何 run。
- `unfollowed` 是这个 run 的命令里有几条只读到了命令本身、没读到它起的进程：Windows 上创建 job、装进 job 或读 job 的进程表失败的那几条（这一次读数里整个 run 的命令都算），Unix 上是每一条，因为 Unix 上没有 Job Object（进程组是它的对应物，尚未接入）。装不进 job 不让命令起动失败：job 只服务于读数，为读数让一条构建失败是把代价付错了地方，失败落在 `unfollowed` 里给读数的人看。
- `share`：Windows 上 job 在创建的同一刻经 `desktop_ffi::cpu::job_share` 设权重 `RUN_CPU_WEIGHT`（5），设上了是 `Weighted`；job 拒了权重时 job 照旧跟进程树，这个 run 读作 `Unset`。macOS 与 Linux 上恒为 `Unset`：macOS 没有按 run 的份额（D29），Linux 的 cgroup 份额还没有接上。作业级内存上限只属四臂对照的 ③ 臂，本构建传 0，即不设。
- 失败：表够不着时 `E_STORAGE_FATAL`，与 `Backlog` 的其他读法相同。
- 每个进程的内存与 CPU 不在这里读：本 crate 不读平台计数（见下），由 sprawling 的 `bin::monitor` 按这里给出的 pid 去读。
- 证据：`crates/runtime/src/backlog/tests.rs` 的 `a_run_owns_the_processes_its_commands_started`——一条经 `run` 转后台的命令，其 pid 出现在它 owner 的那一项里，别的 run 那一项里没有；Windows 上 `unfollowed` 为 0、`share` 为 `Weighted`，别处为 `Unset`；`release` 之后这个 run 不再出现。

**决定（job 的取法）**：Job Object 经 `win32job` 2.0.3（`Job::create`、`assign_process`、`query_process_id_list`，对外只给安全接口），本 crate 不写 `unsafe`；子进程的句柄经标准库的 `AsRawHandle` 取得。**被否**：①一整座城一只 job——分不出 run，而分解到 run 正是要它的原因；②只记直接子进程的 pid——`cargo`、`npm`、`sh -c` 这类命令自己几乎不占内存，读数会把一条吃掉几 GiB 的构建报成几 MiB；③`CREATE_SUSPENDED` 起动再装 job 再恢复——恢复线程要 FFI。代价：子进程从起动到装进 job 之间有一小段时间，那一段里它再起的进程不在 job 里（`cmd /C` 这类命令的第一个孙进程在这一段里起动的可能很小，但不为零）。**重开参数**：一个对外只给安全接口的 crate 能以挂起态起动子进程并在恢复前装进 job，或者读数显示 `unfollowed` 之外还有漏掉的孙进程。

**未决（§3 口径）**：核心线程升到正常档之上一级与空转安全阀在 `crates/sprawling/Spec.lean` §8-93；派出进程的 CPU 份额在 Windows 上已经按 D29 落地（每个 run 的 job 带权重 5，经 `crates/desktop/ffi` 的 Zig 叶子），作业级内存上限的叶子函数也已在（`desktop_ffi::cpu::job_share` 的 `memory`），但只属四臂对照的 ③ 臂，物理内存的读数还没有经 `bin::assembly` 交进本 crate，所以本构建不设；macOS 上派出的命令已包在 `/usr/sbin/taskpolicy -c utility` 里（`yielding` 的 `start_below_the_core`，找不到它时只包 `nice`），Linux 的 cgroup `cpu.weight` 尚未落地；这条路不在第一档，是因为 `win32job` 2.0.3 的安全接口够得着 Job Object 本身（上文「job 的取法」），够不着这两项限额：它对外给出的内存限额只有按进程的工作集上限（`limit_working_memory`，即 `JOB_OBJECT_LIMIT_WORKINGSET`），限的是常驻页而不是提交量，不兑现「资源」一轴，且设它要不要特权随账户令牌而变（§8-13-2 记着两次读数：经 `win32job` 得 os error 1314，直接调 `SetInformationJobObject` 得成功），所以它不是任何一项限额的取法。作业级提交上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`）不要特权，但设它的字段在 `win32job` 里是 crate 私有的，`process-wrap` 10.0.1 与 `windows-spawn` 0.1.0 也不设它。一个对外只给安全接口的 crate 公开 `JOB_OBJECT_LIMIT_JOB_MEMORY` 与 CPU 速率控制时，D29 的叶子函数换成它。重命令共用的额度池尚未落地：池的大小由测得的核数与可用内存推出，哪些命令算重由城配置给默认表；可用内存的读数只在 sprawling 的 `bin::monitor::memory` 里读（`crates/sprawling/Spec.lean` §8-94），由 `bin::assembly` 交给本 crate，本 crate 不读平台。内存紧时新 run 排队已在 `crates/sprawling/Spec.lean` §8-46-3。
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


> 权威在 `crates/kernel/Spec.lean` §8-22 的「11.1 增」段与 `crates/city/Spec.lean` §8-4 的「11.1 增」段；本节只说 exec 这一侧怎么用它，以及构造面因此怎么变。

**只放四个名字的后果**：`ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"]` 加 `env_clear()` 时，城里的 resident 跑不动 `cargo build`——MSVC 链接器读不到 `%ProgramFiles(x86)%`，退回裸 `link.exe`，撞上 PATH 上 Git 那个 coreutils `link`。

**改法**：`ENV_ALLOWLIST` 原样留着，它是**每一栋楼无条件继承的地板**；楼在 `[sandbox] env_passthrough` 里逐名声明的是**地板之上加的那几个**。两份清单合并后按名取值，取不到的名字不进（一个没设过的变量不该变成空串——空串与未设置在 Windows 上是两件事）。

**构造面**：总在一起走的那几个值是一个有名字的值，`ExecTool::new` 因此只收三个参数：

```rust
pub struct ExecSetup {                 // 形状 2 值类型
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Shell,                  // D30：缺席、要了而没有、或找到的那一个与它是哪种解释器
    pub fuel: Fuel,
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: Address,
    pub run: RunId,                    // 这张工具台服务的 run：它起的后台命令只交还给它（§8-28-1 第 4 条）
    pub policy: PolicyReader,          // 这个 run 的策略格（§8-55、§8-62）
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

/-! D29 每个 run 的 job 按权重分 CPU、设作业级内存上限；子进程在 macOS 降到 utility，在 Linux 用 cgroup v2 的 `cpu.weight` 或只用 nice（AF1 的 (d) 与 (e)，D88 第 3 条，D94，D20）

**决定**：

- **Windows (d)**：`runtime::backlog::jobs` 创建一个 run 的 job 时，同时给它设两项：CPU 速率控制 `JOBOBJECT_CPU_RATE_CONTROL_INFORMATION { ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_WEIGHT_BASED, Weight: 5 }`（`SetInformationJobObject`，信息类 `JobObjectCpuRateControlInformation`），与作业级提交上限 `JOB_OBJECT_LIMIT_JOB_MEMORY`（读出 `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`、置上这一位与 `JobMemoryLimit`、再写回，于是别处已经设的位不丢）。接口档位：`win32job` 2.0.3 把扩展限额的结构放在 crate 私有字段里、不设 CPU 速率控制，`process-wrap` 10.0.1 也不设，所以走第二档 Zig 叶子，放在 `crates/desktop/ffi`（与 `crates/sprawling/spec/Serving/Standing.lean` D40 同一个叶子）：job 的句柄由 `win32job::Job::handle` 交出（公开），两项限额作为一个定长记录过 `(ptr, len)` 边界。两项都不要特权（推断：文档都没有要求特权，而工作集上限要不要特权随账户令牌而变，见 §8-13-2；派生的 Rust 检查在未提权的 Windows runner 上设好再读回，先红后绿）。
  - **权重**：每个 run 一样，取 5（范围 1–9），常量 `RUN_CPU_WEIGHT`。要的是按 run 公平：不设时一条起 16 个进程的构建按线程分到 16 份，对面只有一个进程的 run 分到 1 份；权重相同，每个 run 各得一份，一个 session 的编译饿不死别的 session。harness 不进任何 job，它的热线程本来就站在正常档之上（§8-93）。
  - **内存上限**：物理内存的一半，常量 `RUN_JOB_MEMORY_SHARE`（1/2）；物理内存由 sprawling 的 `bin::monitor::memory` 在起动时读出、经 `bin::assembly` 交给本 crate，本 crate 不读平台。超过时是这个 run 的进程树里的分配失败（编译器报内存不足），城与别的 run 照常。四臂对照的 ③ 臂才打开它（`crates/sprawling/spec/Serving/Placement.lean`「四臂对照」），默认按读数定。
  - **退路**：叶子调用失败时 job 照今天的样子（不设份额、不设上限），命令照常起动（与装不进 job 同一条判断：为读数或份额让一条构建失败是把代价付错了地方），doctor 说出「每个 run 的 CPU 份额：未设」与原因。
- **macOS (d)**：没有 Job Object。派出的命令在 `nice` 外面再包一层 `/usr/sbin/taskpolicy -c utility`，把它与它的后代的 QoS 压到 utility，系统于是先把它们放到效率核上（外部命令，第一档，与 `nice` 同一种做法，§8-13-3）；找不到 `taskpolicy` 时只包 `nice`，并照实说。没有不要特权的内存上限：`setrlimit` 要在 `pre_exec` 里调，`pre_exec` 是 `unsafe`，而且 macOS 不执行 `RLIMIT_AS`；所以 macOS 上这一项不可用，doctor 照实说。按 run 的 CPU 份额同样没有。
- **Linux (d)**：harness 自己所在的 cgroup（`/proc/self/cgroup` 的 `0::` 行，挂在 `/sys/fs/cgroup` 下）可写时，harness 先把自己移进一个子 cgroup `core`（cgroup v2 规定有进程的 cgroup 不能再往下分资源），在父 cgroup 的 `cgroup.subtree_control` 打开 `cpu` 与 `memory`，每个 run 建一个子 cgroup，写 `cpu.weight`（每个 run 一样，100）与 `memory.max`（同 `RUN_JOB_MEMORY_SHARE`），子进程起动后把 pid 写进那个 cgroup 的 `cgroup.procs`；全是标准库读写文件，第一档。与 Windows 的 job 一样，起动到写进 cgroup 之间有一小段，那一段里起的孙进程留在 `core` 里。不可写时（没有 systemd 的委派，CI 主机与许多桌面都是这样）只靠 `nice 10`，doctor 说「每个 run 的 CPU 份额：只有 nice（cgroup v2 未委派）」。
- **(e) `WindowsJobObject` 臂**：今天不构造，理由按 `win32job` 2.0.3 今天公开的接口重新判过（§8-13-2），仍是两轴兑现不了。资源一轴由上面的叶子兑现；进程树一轴要「挂起态起动、装进 job、再恢复」：挂起态起动有安全接口（`CommandExt::creation_flags` 加 `CREATE_SUSPENDED`），装 job 有（`win32job::Job::assign_process`），恢复没有——`std::process::Child` 不交出主线程句柄，`ResumeThread` 或 `NtResumeProcess` 都要 FFI，而 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 要的 `CommandExt::raw_attribute` 既是 `unsafe` 又只在 nightly 上。于是同一个叶子再加一个「恢复这个进程」的函数之后，这一臂按原清单构造：文件系统（副本）、进程树、CPU 与内存上限都保，网络与用户不保；那时 Windows 上 `choose` 有了它就选它。
- **为什么不缩清单**：一只只保文件系统与「起动之后的进程树」的 job 臂，对 Agent 来说与 `CopiedTree` 几乎一样，多出的只是 kill-on-close；多一臂只多一句要读的话，不多一项保证。

**被否**：①`win32job` 的调度级别（`limit_scheduling_class`，安全接口）当作 CPU 份额——它只改同一优先级类里各 job 线程的时间片长短，每个 run 的级别都一样时什么也没分；②工作集上限（`limit_working_memory`）——限的是常驻页，不是提交量，而且要不要特权随账户令牌而变（§8-13-2）；③硬的 CPU 速率上限（`JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`）——机器空着时也让核闲着，与「忙了立刻换下一个核」相反；④每条命令一只 job——份额要按 session 分，不是按命令；⑤Linux 上用 `systemd-run --user --scope -p CPUWeight=…` 包每条命令——每条命令多一次 D-Bus 往返与一个 scope 的起动，而且要有用户级 systemd，不是每台机器都有；直接写委派的 cgroup 文件更少依赖。

**重开参数**：四臂对照里 ③ 臂（再加 (d)）的 p99 与 p999 不优于 ② 臂——那就只留 Windows 的权重或整个 (d) 都不做；或者一个对外只给安全接口的 crate 公开 job 的 CPU 速率控制与作业级内存上限（叶子函数换成它）；或者 Rust 稳定版提供 `raw_attribute`（(e) 的恢复函数就不需要了）。
-/

/-! D30 shell 默认仍是平台的 shell；一栋楼可以在 `CONFIG.toml` 里换成 pwsh 7；exec 的失败按 shell 分类，从账本折出（TF5，D88 第 1、7 条，D83 第 10 条，D94）

**决定**：

- **配置项**：楼的 `CONFIG.toml` 里 `[sandbox]` 一节加 `interpreter`，取 `"system"`（缺省）或 `"pwsh"`。它与 `shell` 是两件事：`shell` 决定 shell 臂给不给，`interpreter` 决定给的时候是哪一个。字段在 `kernel::config::SandboxLimits` 上，规格的所有者是 `crates/kernel/spec/Config.lean` §8-22，按层解析的规则与 `SandboxLimits` 的其余字段相同。写的是名字而不是路径：`SandboxLimits` 的文档说，哪个 shell 程序存在、装在哪里属于机器而不属于城，城搬到另一台机器上不能带着原来那台主机上的路径。
- **校验**：拼写只有这两种，别的拼写在解析处以 `E_CONFIG_INVALID` 拒绝，主语是这一个键，恢复语给出两种拼法；不猜，也不当作缺省。
- **在主机上解析**：`"system"` 照今天：Windows 上 `%COMSPEC%`（缺省 `cmd.exe`）加 `/C`，macOS 与 Linux 上 `$SHELL`（缺省 `/bin/sh`）加 `-c`（doctor 的 `shell` 行，`crates/sprawling/src/doctor/table.rs`）。`"pwsh"` 在三个平台上都是搜索路径上的 `pwsh`，加 `-NoLogo -NoProfile -NonInteractive -Command`；doctor 的 `pwsh` 行以 `pwsh --version` 探它（印 `PowerShell 7.4.6` 一类的一行），`doctor::host::usable_pwsh` 只在这一行的主版本不小于 7 时给出路径，主版本读不出也不给。机器那一半（`accounting::worker::workbench::engine::machine_half`）按 `interpreter` 问 `ExecHost` 的 `shell` 或 `pwsh`，交给 bench 的是一个值：

```rust
pub enum Shell {                                    // runtime::tools::exec::shell，ExecSetup.shell；根上导出
    Absent,                                         // 没有一层要 shell 臂
    Missing { asked: kernel::Interpreter },         // 楼要了这个解释器（含 System），所在的主机上没有能用的
    Found { program: PathBuf, interpreter: kernel::Interpreter },
}
```

  shell 臂可用（`Found`）时，exec 工具的说明在末尾多一句 `A shell line runs under <name>.`，`<name>` 与结果里的 `interpreter` 同一个拼写：模型写哪种语法取决于它，cmd、pwsh 与 sh 的同一行命令意思不同，不说出来就是让模型猜。`Absent` 与 `Missing` 不加这句，调用时的拒绝已经说清。
  `Missing { asked: Pwsh }` 在调用时以 `E_TOOL_UNAVAILABLE` 拒绝，主语点名 pwsh 7，恢复语是「install PowerShell 7, or set `[sandbox] interpreter = "system"` in this building's CONFIG.toml」。不退回 cmd：为 pwsh 写的命令行在 cmd 下是另一种语言，退回只会把一次清楚的拒绝换成一次看不懂的失败。三个平台行为相同，只有 `"system"` 指的程序随平台变。
- **账本上留什么**：shell 臂的结果载荷加 `interpreter` 字段，值是实际起动的解释器的程序名（去掉扩展名、小写：`cmd`、`pwsh`、`sh`、`bash`、`zsh`……），拼写只在 `tools::exec::outcome::interpreter_name` 一处。配置不入账本（§8-31），所以要按 shell 统计，这次跑的是哪一个只能记在结果上。加这个字段之前的记录没有它，按它们实际跑的那一个计：那时只有 `"system"`，折叠把它们计在 `system` 名下。转进后台的 shell 命令（`outcome: backgrounded`）当时没有退出码，不计。
- **失败类别**：`tools::exec::outcome` 的纯函数 `FailureClass::of(interpreter, exit_code, stdout, stderr)` 把一次 shell 臂的结果分进 `CommandNotFound`、`Syntax`、`Encoding` 或不分类（`None`）；按 shell 统计的折叠 `ShellTally::absorb(result)` 读一条 `ToolResult` 载荷里的 `result`。两者住 `outcome`，因为那个文件是 exec 结果键名（`arm`、`exit_code`、`stdout`、`stderr`、`interpreter`）的唯一主人，折叠读的正是这几个键；每个解释器打印什么也只在这一处。`runtime` 在根上导出 `FailureClass`、`ShellTally`。账本的视图（`sprawling view --shells`，`bin::main::view::shells`）读记录、把每条 `result` 交给 `ShellTally`，折出来的是每个解释器的调用数与每类失败数。规则：
  - `CommandNotFound`：cmd 退出码 9009；sh 系退出码 127；pwsh 的 stderr 带错误 id `CommandNotFoundException`。
  - `Syntax`：sh 系退出码 2 且 stderr 带 `syntax error`；pwsh 的 stderr 带 `ParserError`；cmd 的 stderr 带 `was unexpected at this time.` 或 `The syntax of the command is incorrect.`。
  - `Encoding`：`stdout` 或 `stderr` 里有 U+FFFD。载荷里的文字经 `String::from_utf8_lossy` 写下（`crates/runtime/src/tools/exec/outcome.rs`），所以替换字符就是一段不是 UTF-8 的字节——cmd 按控制台代码页输出，中文 Windows 上是 936。一个真的打印了 U+FFFD 的程序也会被计进来，这是多计的一侧。
  - 先看退出码，再看不随语言变的错误 id，最后才看英文文字：cmd 的提示随 Windows 的显示语言变，非英文系统上 cmd 的语法错误落进「不分类」，这一点照实写在视图的说明里。本仓不收一张各语言提示的表：那会是第二份 Microsoft 文字的权威。
  - 退出码为 0 的结果不分类：一条成功的命令即使输出里有 U+FFFD 也不算失败。
- **现状**：配置项、`Shell`、拒绝、`interpreter` 字段、分类与 `ShellTally` 已落地，测试在 `tools::exec::tests::shell`、`kernel::config::interpreter`、`crates/city/src/config_layers/write/tests.rs` 与 `doctor::tests::host`。读它的视图是 `sprawling view <city> --shells`（`crates/sprawling/Spec.lean` §8-105），每个解释器一行 JSON；同一份读数经线上的 `Query::Shells`（wire D48）画在页面工具页的一张表里；楼页的沙箱卡只把读到的 `interpreter` 原样送回，不给选择控件，改它今天靠手写 `CONFIG.toml`。
- **何时再定默认**：视图按解释器给出每类失败占 shell 臂调用的比例；pwsh 7 的成功率明显更高时，把读数交 User 再定缺省（D88 第 7 条读作这样，待 User 确认）。不做 pwsh 预热池：启动时间不是问题（D88 第 7 条）。
- **program 臂在 `CopiedTree` 下每条命令前同步副本的成本**：计数已经有了（`Placed::work()`，§8-13-2），读数归波后的 mid 读数，判定它的证据与重开参数写在 §8-13-2 的「未决」与 D10。

**被否**：①默认改成 pwsh——D88 第 1 条：保持 cmd；②把 `shell` 从布尔改成三值（关、system、pwsh）——每一层已写的 `shell = true` 都要迁移，而两件事（给不给、给哪一个）各有各的读者；③在配置里写 shell 的路径——城会带着一台机器的路径搬家；④从 stderr 的本地化文字分类——见上；⑤把失败类别作为一种新的账本事件在执行时写下——那是同一事实的第二个权威，账本上已经有退出码与输出，折叠就能读出。

**重开参数**：视图显示某类失败的「不分类」占比高到读不出差别（例如非英文 Windows 上 cmd 的语法错误）——那时再议是否以 cmd 的错误级别或别的不随语言变的信号补上；或者 User 按读数改了缺省。
-/

/-! D32 沙箱臂的调研表（SB0）与按平台的缺省臂、可选臂（D86，D20，D23，D94）

**调研表**。五轴的列序照 `Guarantee::ALL`：文件（写入只落副本）、网络、进程树、用户、资源；「保」与「不保」按 `Kept` 的意思，「条件」写在格里。「居民」一列问这一臂能不能同时装下 harness 居民（代理作为主进程）：居民要连 provider，所以一个关网的臂装居民时网络一轴必然不保。每一行的依据是行末列出的官方页面（本调研取过的页面）与本节末尾的探测；没有取到官方页面的格写「未核实」，并进下文的未决。

| 族（`SandboxArm`） | 机制 | 平台 | 要管理员或系统功能 | 文件 | 网络 | 进程树 | 用户 | 资源 | 居民 | 依赖与维护 |
|---|---|---|---|---|---|---|---|---|---|---|
| `copied_tree` | 复制工作树（今天的臂，§8-13-2） | 三个平台 | 否 | 保 | 不保 | 不保 | 不保 | 不保 | 不适用：只放 exec | 无外部依赖 |
| `native` | Job Object（D29 的叶子） | Windows | 否；探测里未提权的进程建 job、设 CPU 权重 5、设作业级提交上限与 kill-on-close 全部成功 | 保（副本） | 不保 | 保（要 D29 (e) 的挂起态起动） | 不保 | 保 | 能：job 装得下任意进程树 | `crates/desktop/ffi` 的 Zig 叶子（D29 已定） |
| `native` | AppContainer | Windows | 否；探测里未提权的进程建、删 AppContainer 配置文件都返回 `S_OK` | 保，且更强：容器 SID 只够得到 ACL 授给它的路径，副本目录要显式授权 | 保（零个 capability） | 不保（单独时） | 保（容器 SID，低完整性级别） | 不保（单独时） | 只能开网装：居民要 `internetClient` 一类 capability，网络一轴随之不保 | 起动要 `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`，稳定版 Rust 不给安全接口，进同一个 Zig 叶子；用户目录下装的工具（`cargo`、`bun`）默认不对容器可读，要授 ACL（推断，SB1 实跑核实） |
| `native` | 受限令牌 | Windows | 否；探测里 `CreateRestrictedToken`（去掉特权）成功 | 不保（单独时） | 不保 | 不保 | 部分：去特权、deny-only SID，仍是同一个用户 | 不保 | 能 | 起动要 `CreateProcessAsUserW`，FFI；它给的已被 AppContainer 覆盖，不单独成臂 |
| （不收） | Windows Sandbox | Windows | 是：只在 Pro、Enterprise、Education 版上，要打开可选功能；官方页写明 Home 版不支持 | 保 | 可配置 | 保 | 保 | 保 | 能，但整个是一台虚拟机 | 每条命令要一条宿主到虚拟机的通道，本调研没有读到受支持的命令行接口；探测机是 Home 版，`WindowsSandbox.exe` 不存在 |
| `container` | Docker Desktop（WSL 2 后端） | Windows、macOS | Windows：安装与更新不要管理员（官方页），但 WSL 2 功能要先打开，打开它要管理员；超过 250 人或 1000 万美元年收入的企业商用要付费订阅 | 保（只挂副本） | 保（`--network none`） | 保 | 保（`--user`） | 保（`--cpus`、`--memory`） | 能 | 外部运行时、镜像要拉取与更新；macOS 上支持当前与前两个大版本 |
| `container` | Podman Desktop | Windows、macOS | Windows：「只为我」安装不要管理员；打开 WSL 或 Hyper-V 功能要管理员（官方页） | 保 | 保 | 保 | 保 | 保 | 能 | 外部运行时与镜像 |
| （经 `container`） | WSL 2 | Windows | 是：`wsl --install` 要以管理员身份运行（官方页）；探测机上 WSL 与虚拟机平台两项功能已开、没有发行版 | —— | —— | —— | —— | —— | —— | 它本身不是一个臂：是 Windows 上两个 Desktop 容器运行时的后端，也是在 Windows 上得到 Linux 各臂的途径 |
| `native` | 命名空间包装程序 `bwrap`（今天的臂） | Linux | 否，要内核允许未提权的 user namespace；否则 `bwrap` 只能以 setuid 装 | 保（整机只读可见） | 保 | 保（`--die-with-parent`） | 条件：今天是 `--unshare-user-try`，见 §8-13-2 与 SB1 第 1 条 | 不保（D29 的 cgroup 另给 CPU 与内存） | 能，但居民要开网，开网时网络一轴不保 | 发行版的 `bubblewrap` 包 |
| `native` | Landlock 加 seccomp | Linux | 否：Landlock 让任何进程、包括未提权的进程限制自己；内核 5.13 起，且要编进内核并在启动时启用；TCP 规则从 ABI v4 起，UDP 从 v10 起；seccomp 过滤要先 `PR_SET_NO_NEW_PRIVS` | 保（还能限读） | 条件：按 ABI，只管 TCP（与 v10 起的 UDP） | 不保 | 不保 | 不保 | 能 | 要在子进程里自限：`pre_exec` 是 `unsafe`，所以只能由 harness 以自己的一个子命令重新起动、自限之后再 `exec` 目标命令；这一做法未核实，进未决 |
| `container` | rootless Podman／Docker | Linux | 装要包管理器；运行要 `newuidmap`、`newgidmap` 与 `/etc/subuid`、`/etc/subgid` 里至少 65536 个从属 id（官方页） | 保 | 保 | 保 | 保 | 保（cgroup v2 委派时） | 能 | 外部运行时与镜像；rootless Podman 的网络走 pasta |
| （`container` 的运行时） | gVisor `runsc` | Linux | 经 Docker、Kubernetes 或直接用 `runsc`（官方页） | 保 | 保 | 保 | 保 | 保 | 能 | 一个 OCI 运行时，在已有的容器臂下作为可选项，不另起名字 |
| （不收） | microVM（Firecracker） | Linux | 要 KVM（`/dev/kvm`） | 保 | 保 | 保 | 保 | 保 | 能 | 要内核镜像与根文件系统的供给；桌面与 CI 主机常没有 `/dev/kvm` |
| `native` | Seatbelt（`sandbox-exec` 配置） | macOS | 未核实 | 未核实 | 未核实 | 未核实 | 未核实 | 未核实 | 未核实 | 未核实：本调研没有取到 Apple 关于 `sandbox-exec` 的官方页（App Sandbox 的文档页靠脚本渲染，取回的正文是空的） |
| （不收） | NVIDIA OpenShell | 三个平台 | Roadmap §6 SB 记下的 Support Matrix：沙箱靠 Landlock 与 seccomp，网络按策略放行，要一个跑在 Docker、Podman、Kubernetes 或 MicroVM 上的 gateway；Windows 上只在 WSL 2 加 Docker Desktop 下，标为 Experimental | —— | —— | —— | —— | —— | —— | 它的隔离来自上面已经各成一行的机制，再加一个 gateway 进程与它自己的策略层；本调研没有重新取它的页面 |
| `python` | wasip1 里的 Python（§8-13） | 三个平台 | 否 | 保（只够得到 preopen） | 保（wasip1 没有获得 socket 的途径） | 保（wasip1 没有起进程的接口） | 不保 | 条件：CPU 由 fuel 限，内存没有另设上限 | 不能：只跑 Python | `wasm` feature 里的 wasmtime 48 |

**每条命令的起动开销**（暂定读数，取于一台负载很重的 Windows 11 x86_64 笔记本，别的构建同时在跑；登记的读数归 W7 之后的测量）：`cmd /c exit 0` 起动并等它退出，各 200 次——直接起动 p50 33.9 ms、p99 60.7 ms；起动后装进一个 job p50 32.0 ms、p99 55.2 ms，这个次数下看不出装 job 的开销。`copied_tree` 的开销是同步副本的开销，见 D10。其余的臂在取读数的那一类电脑上没有，读数归 SB1 第 8 条的实跑。

**探测**（未提权的 PowerShell，经 `Add-Type` 直接调 Win32；只列带出结论的那几行）：

```
elevated: False
CreateJobObject handle nonzero: True
CPU rate control weight=5: True
JOB_MEMORY 1 GiB + KILL_ON_JOB_CLOSE: True
WORKINGSET limit: True
CreateAppContainerProfile hr=0x00000000
DeleteAppContainerProfile hr=0x00000000
CreateRestrictedToken(DISABLE_MAX_PRIVILEGE): True
VirtualMachinePlatform InstallState=1
Microsoft-Windows-Subsystem-Linux InstallState=1
WindowsSandbox.exe present: False
```

`wsl --status` 答默认版本 2，`wsl -l -v` 答没有已安装的发行版；`docker` 与 `podman` 都不在 `PATH` 上。

**决定**：按平台的缺省臂与可选臂如下，缺省是 `crates/wire/spec/Answer/Doctor.lean` D26 的那一张表上的「缺省」一列，那里是唯一的定义处，User 以后改它就是改一个值（推断的选择，待 User 定，Roadmap §7 第 1 条）。

| 平台 | 缺省 | 可选 |
|---|---|---|
| Windows | `native`：Job Object 加 AppContainer（exec），Job Object（居民）；SB1 第 3 条落地之前解出 `copied_tree` 并照实说 | `copied_tree`、`container`（Docker Desktop 或 Podman Desktop，要另装）、`python`、`none` |
| macOS | `copied_tree`：Seatbelt 的未决解开之前，`native` 在 macOS 上答 `Unavailable` | `container`（Docker Desktop 或 Podman Desktop，要另装）、`python`、`none` |
| Linux | `native`：`bwrap`；没有 `bwrap` 时解出 `copied_tree` 并照实说 | `copied_tree`、`container`（rootless Podman 或 Docker，要另装；gVisor 作为它的运行时）、`python`、`none` |

安装指引：Docker Desktop（https://docs.docker.com/desktop/setup/install/windows-install/ 、https://docs.docker.com/desktop/setup/install/mac-install/）、Podman Desktop（https://podman-desktop.io/docs/installation/windows-install）、rootless Podman（https://github.com/containers/podman/blob/main/docs/tutorials/rootless_tutorial.md）、rootless Docker（https://docs.docker.com/engine/security/rootless/）、bubblewrap（https://github.com/containers/bubblewrap）。

**理由**：按 Roadmap §6 SB 列出的六项依次比较。①方便：缺省的臂不要另装，一台新机器开城就有；`container` 要装一个运行时与镜像，只作可选。②不要管理员（D20）：Windows 上探测过的不要管理员的三种机制里，Job Object 与 AppContainer 合起来保五轴（AppContainer 给网络与用户，job 给进程树与资源，副本给文件），受限令牌给的被 AppContainer 覆盖；Windows Sandbox 要专业版以上加管理员，WSL 2 的安装要管理员。③五轴：Windows 的 `native` 保五轴，Linux 的 `bwrap` 保四轴（资源由 D29 的 cgroup 另给），都多于 `copied_tree` 的一轴。④居民：job 装得下居民；关网的机制装居民时网络一轴不保，这一点写进清单而不是挑一个能关居民网络的臂——没有这样的臂。⑤依赖（D23）：缺省的臂只依赖平台本身与已有的 Zig 叶子（Windows）或发行版的一个小包（Linux）；容器运行时、gVisor、microVM 与 OpenShell 各是一个大的外部系统。⑥三个平台（D94）：三个平台都有 `copied_tree`、`container` 与 `python`，`native` 在 macOS 上缺的是证据而不是名字，答 `Unavailable` 并说明缺什么。缺省的 `native` 缺机制时退到 `copied_tree` 而不是拒：缺省是城替 User 选的，一条拒绝会让一台没装 `bwrap` 的机器上 exec 整个不能用；退的时候 `statement()` 与 doctor 说出退了、缺什么，所以不是静默变弱。User 明写的名字缺机制时拒，因为那是 User 要的那种盒子。

**被否**：①Windows 缺省 `copied_tree` 不变：它对 Agent 只保文件一轴，而不要管理员的机制能保五轴；②Windows Sandbox 作 Windows 的 `native`：要专业版以上与管理员，探测机就没有；③缺省 `container`：每台机器先要装一个运行时、拉一个镜像，与「方便」与 D23 都相反；④OpenShell 作缺省：它的隔离是 Landlock、seccomp 与容器，已各成一行，多出的是一个 gateway 与一套自己的策略层，Windows 上只是实验性的；⑤Linux 缺省换成 Landlock 加 seccomp：不保进程树与用户，网络只管 TCP，而且起动方式未核实；它留作 `bwrap` 缺席时 `native` 的第二种机制，等未决解开；⑥按产品给每个机制一个名字：D26 已否。

**重开参数**：Seatbelt 的未决解开（macOS 的 `native` 有了臂，macOS 缺省改 `native`）；Windows 的 Zig 叶子在 SB1 里证明做不出 AppContainer 起动（Windows 的 `native` 缩成 Job Object，清单照实写网络与用户不保）；User 按这张表另定缺省。

**未决（§3 口径）**：①macOS 的 Seatbelt：判定它的证据是 GitHub 的 macOS runner 上 `man sandbox-exec` 的原文（是否标为已弃用）、一个 `(version 1)(deny default)` 起头、只放开副本目录写入并 `(deny network*)` 的配置下跑写文件、连回环端口与起孙进程三条命令的结果，以及 Apple 的一页可取的官方文档；②Linux 的 Landlock 回退：判定它的证据是一个不写 `unsafe` 的起动方式（harness 以自己的子命令自限后 `exec`）在 Linux runner 上跑通，并读出 runner 内核的 Landlock ABI。
-/
