-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::exec

规定 `tools::exec`、`tools::exec::outcome`、`tools::exec::shell`、`tools::exec::confinement`、`tools::exec::yielding`（`crates/runtime/src/` 下同名的文件）。exec 的三臂、宿主进程沙箱、派出的命令降一级与环境声明。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

除末尾的亲和申请模型与 D95 的上限判定外，这一分部是说明文档，不是形式规格；它写下的接口形状与取舍由 Rust 的类型与 `tools::exec`、`backlog` 旁的测试守住（`crates/runtime/Spec.lean` §16）。
-/

/-!
### 8-13-2 runtime::tools::exec::confinement（宿主进程沙箱：按平台穷尽枚举＋每臂保证清单；形状 3＋2）


上面的 wasip1 面给的是 **guest** 的隔离；exec 的 program／shell 两臂跑的是**宿主进程**，它的隔离只能向平台买，而没有哪个平台把它全卖。故本模块的产物不是开关而是一份分类法：每一臂逐轴说出自己保什么、不保什么，跑在哪一臂上对人与 Agent 都可见（工具 disclosure＋doctor 回报），不是假定。

| 臂 | 保 | 不保 |
|---|---|---|
| `LinuxNamespaces { wrapper }` | 文件系统、网络、进程树、用户（强制 user namespace） | CPU／内存上限；整机对命令只读可见（`--ro-bind / /`），读不受限 |
| `WindowsJobObject` | 文件系统（副本及 profile storage）、网络（无 capability 的 AppContainer）、进程树、独立安全身份、CPU／aggregate 内存上限 | 源树与 host 权限不开放 |
| `MacosSeatbelt { wrapper }` | 文件写入只落副本、网络拒绝；读取不受限 | 进程树终止、独立用户、CPU／聚合内存硬上限；边界见 `Tools/Exec/NativeMacos.lean` D40 |
| `CopiedTree` | 文件系统（写入只落副本、源树只读） | 网络、进程树、用户、CPU／内存上限 |
| `Unavailable { missing }` | —— | 一切；`missing` 指名缺的是什么 |

```rust
pub enum Guarantee { Filesystem, Network, ProcessTree, User, Resources }
pub enum Kept { Yes, No }
pub struct Assurances { pub filesystem: Kept, pub network: Kept, pub process_tree: Kept, pub user: Kept, pub resources: Kept }
pub enum Missing { ScratchDirectory }
pub enum Confinement { LinuxNamespaces { wrapper: PathBuf }, WindowsJobObject, MacosSeatbelt { wrapper: PathBuf }, CopiedTree, Unavailable { missing: Missing } }
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
- **`LinuxNamespaces` 的用户隔离是起动条件**：`namespaced()` 在 `--unshare-all` 之后显式加 `--unshare-user`；bubblewrap 手册（https://github.com/containers/bubblewrap/blob/main/bwrap.xml ）规定前者包含可跳过的 `--unshare-user-try`，后者要求创建 user namespace。`place()` 在同步副本之前用同一包装参数运行 `/bin/true`，绑定的是工作目录自身；探测没有写入，成功后才构造目标命令。探测起动失败或退出非零时返回 `E_SANDBOX_DENIED`，主语说明这一臂需要未提权的 user namespace，并保留系统错误或 wrapper 的 stderr；恢复语给出换到允许未提权 user namespace 的 Linux 主机或由人选择 `where: host`。探测不缓存，因为内核与 LSM 的许可可能在两条命令之间变化；目标命令仍带强制参数，所以探测之后许可被撤回时也不会少一轴运行。`statement()` 保证文件系统、网络、进程树与用户，资源上限不保，并说明未提权 user namespace 是必需条件。`--ro-bind / /` 让整机对命令只读可见：文件系统保证写入只落副本，不限制读取。
- **选择是纯函数**：`choose` 取 `Offerings`，scratch 不可用即 `Unavailable`；有 namespace wrapper 得 `LinuxNamespaces`，否则得 `CopiedTree`；Windows 的 `WindowsJobObject` 在 SB1 第 3 条的 conformance 通过之前只由 `[sandbox] arm = "native"` 明写构造（D32 表的「SB1 第 3 条落地之前解出 `copied_tree`」）。明写的 native 创建若失败，拒绝命令，不退到普通进程。
- **逐平台的臂**：Linux 读 `bwrap`，Windows 明写 `native` 时使用本构建的 AppContainer/Job leaf，macOS 缺省仍为 CopiedTree，显式 MacosSeatbelt 由 D40 固定策略探测，缺席或拒绝返回 E_SANDBOX_DENIED，不执行裸目标；无 scratch 的机器都拒绝。所有保证来自同一 `Assurances`，工具与 doctor 不另算。
- **Windows native 的实际执行**：`crates/runtime/spec/Tools/Exec/NativeWindows.lean` 规定挂起起动、command job 与 run job 在第一条指令之前的 assignment、无网络 capability、scratch ACL、CPU hard cap、aggregate committed-memory 与所有权清理。`ExecTool::through_the_backlog` 把 Windows sandbox 交给 `Backlog::run_native`，host 仍用普通命令；两者共享同一轮询与收场。native process 从表中消失前终止仍存活的后代。AppContainer 是独立 package security principal，base user SID 不改变，不宣称建立新 Windows 账户。
- **副本按工具一份，每条命令之前同步成工作目录此刻的样子，有界。** `place()` 取这个 `Confined` 留着的副本（没有，或留着的那份抄的是别的目录，就在 scratch 根下新建一个），把它同步成 `workdir` 此刻的样子，命令在副本里跑；`settled()` 在等待结束时把副本留给下一条命令（已留着一份时删掉这份）；交给 backlog 的后台命令由 `handed(id, …)` 记名，其成员报结时 `reaped(&[Finished])` 同样把副本留下或删掉；`Drop` 删掉留着的与未报结的副本。同步的规则：源侧跟随链接（指向树外的链接带进来的是内容，而不是通向人那棵树的入口）；副本侧不跟随链接，因为副本里的东西是上一条命令写的。副本里的一项与源不同类（命令把文件换成了链接、把目录换成了文件），或同名文件内容不同，就先删掉这一项再从源复制，落成一个新的目录项——命令可能在副本里造了指向别处的硬链接，就地改写会写穿到那一头；同名同类同长的文件逐字节比较，相同就不动；副本里源没有的项删掉。于是每条命令开始时，副本与工作目录逐文件相同，上一条命令写下的东西不会留给下一条。逐字节比较而不比 mtime 与长度：同一个时间戳刻度里的等长改写比不出来，副本就会为一个它没有带上的版本担保；比较只读两边的文件，不新建，而实时扫描等的是新建的文件。`Placed::work()` 报这次同步的 `storage::FileWork`（`crates/storage/Spec.lean` §8-31）：第一次放置 `created` 是树里的文件数，一次什么都没变的再放置 `created`、`rewritten`、`removed` 都是 0，`walked` 是两侧读过的目录项；`confinement::tests` 的 `a_sandbox_copy_is_synced_rather_than_made_again` 在 N 与 2N 个文件的树上断言这些数。**删不掉不把命令判成失败**（与 `backlog/member.rs`、`collect()` 同一条判断：命令的收场是调用方应得的事实，一个临时目录只值磁盘）。界：`MAX_FILES = 100_000`、`MAX_BYTES = 256 MiB`、`MAX_DEPTH = 64`，按源侧计；越界**拒**并报出越过的那一对数字，已同步一半的副本随之删掉——半份副本会为一堆没带上的文件担保，而本模块的全部理由是防这个。`MAX_DEPTH` 同时终结自指链接造成的无底走查。
- **`Mount`／`Fuel` 不沿用**：`Fuel` 是 wasmtime 指令计量、`Mount.guest` 是 guest 路径别名，二者 wasip1 专属。本模块保留的是**判断**（能力面＝能到达的路径集）而不是词形。宿主环境照旧不继承（exec 的 env allowlist 未动）；`SandboxJob.env` 的显式注入属 guest 面。
- **placement**：调用参数 `where: sandbox|host`，缺省 `sandbox`。`host` 是「在原地跑」——它才是碰得到人那棵树的那一臂，故必须由调用方按名说出，也正是与 A-8 同一条纪律（默认引导先在沙箱里做，出沙箱才需要审批）里「需要审批」的那个动作。python 臂无 host 形（它是 wasip1 guest）：要宿主解释器走 program 臂。
- **公开路径经 `runtime::tools`**：`confinement` 住 `tools/exec/`，doctor 的依赖回报与工具自己的 disclosure 都从 `runtime::tools::{Confinement, Guarantee, Kept, Missing}` 读这一份定义。
- 证据：`crates/runtime/src/tools/exec/tests.rs` 的 `a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone`（真命令、真树：沙箱里写得到、人那棵树不动；同一命令 `where: host` 则写进原树——此对拍使「没动」是能力判定而非命令没写）；`crates/runtime/src/tools/exec/confinement/tests.rs` 的 `the_sandbox_arm_a_machine_gets_is_chosen_from_what_it_has`、`every_sandbox_arm_states_what_it_does_not_hold`、`a_sandbox_refuses_a_tree_deeper_than_its_walk_can_end`、`a_sandbox_copy_is_synced_rather_than_made_again`、`a_sandbox_copy_goes_with_the_tool`；Linux 的 `a_namespaced_command_requires_a_user_namespace` 经 `Confined::place()` 放置命令，以成功的 wrapper 替身通过探测，断言返回命令的强制参数紧跟 `--unshare-all`，`a_namespace_setup_failure_refuses_before_copying` 以失败和缺席的 wrapper 验证拒绝与未创建副本。

Windows native 的资源与起动顺序由 `NativeWindows.lean` D53/D54 规定；Job 单独不提供身份或网络隔离。

**SB1 未落地的工作**（D32 选定的臂；以下保留各项原编号）：

2. **`[sandbox] arm` 的解析与解出**：键与五个拼写已由 `crates/wire/spec/Answer/Doctor.lean` D26 定，缺省按平台取那里的表；`kernel::config::SandboxLimits` 加 `arm`（规格的所有者 `crates/kernel/spec/Config.lean` §8-22），未知拼写 `E_CONFIG_INVALID`。`Offerings` 加各机制的采样（Linux 的 `bwrap`、Landlock ABI；Windows 的 AppContainer 与 job 叶子是否在本构建里；三个平台上的容器能力由 `crates/runtime/spec/Tools/Exec/Container.lean` 的 daemon admission 契约决定，CLI version 或 PATH 可见性不构成准入证据），`choose` 改为取（名字，`Offerings`）给出 `Confinement`：缺省名字的机制缺席时退到 `CopiedTree`，并在 `statement()` 与 doctor 行里说出「缺省的 native 不可用：缺 X」；User 明写的名字的机制缺席时答 `Unavailable { missing }`，`place()` 拒。`Missing` 随之加各机制的缺项（`NamespaceWrapper`、`UserNamespace`、`JobLeaf`、`ContainerRuntime`），每一项一句 `phrase` 与一句 `recovery`，要另装的给安装指引。
3. **Windows 的 `native`**：接口与性质统一由 `NativeWindows.lean` 规定；其 conformance 在该模块的 Rust tests 与 disposable Windows acceptance 中验证。缺省改为 `native` 之前，下列 Windows 回归必须在明写 `native` 的臂里通过，它们在 windows-latest 上实际失败过：`cmd /C` 的参数经 packet 编码后被 cmd 读成未闭合的引号（`the_program_arm_runs_a_real_child_with_a_scrubbed_environment`、`a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone`、`a_child_sees_the_names_its_building_declared_and_no_others`、accounting 的 `a_command_output_over_the_floor_reaches_the_model_sieved_with_the_way_back`）；Windows PowerShell (`powershell.exe`) 在容器内初始化失败（`System.Net.ServicePointManager` 的类型初始化抛出，`a_dispatched_command_runs_below_the_core`）；建筑声明的 `RUSTUP_HOME`/`CARGO_HOME` 在用户目录下，容器 SID 读不到（os error 183，`exec_builds` 的 `a_building_that_declares_the_names_can_build_a_rust_program`），D32 调研表已预计这些目录要授 ACL。`native-windows-acceptance.yml` 通过 `SPRAWLING_DISPOSABLE_NATIVE=1` 在 disposable runner 上运行这六项，测试 fixture 调用 `ExecTool::confined(Confined::with_arm(WindowsJobObject, …))`，accounting fixture 在临时建筑配置明写 `arm = "native"`；普通测试和生产选择不读取这个变量。步骤检查 runtime 恰好选中五项、accounting 恰好选中一项，并在失败后继续记录其余结果。`exec_builds` 用 `cargo build --offline --target-dir target` 把子构建输出限定在 disposable copy，不继承 harness 的 `CARGO_TARGET_DIR`，避免争用 harness 的构建锁。priority 回归的现有 probe 是 Windows PowerShell (`powershell.exe`)，不能以它代证 PowerShell 7 (`pwsh.exe`) 的初始化；`native_windows_disposable_pwsh_initializes_network_types` 另经 native backlog 检查 `ServicePointManager` 类型初始化与 BelowNormal priority，不替换旧回归。cmd source 的形成权威在 `NativeWindows.lean` D56；PowerShell 初始化与声明工具链目录的最小只读授权及清理仍须由真实 native acceptance 判定，六项通过之前保持 `copied_tree` 缺省。子进程自开 NUL 设备的平台前提与 acceptance runner 的对齐由 `NativeWindows.lean` D59 规定。
4. **`container` 臂**：admission 与直接 create argv 由 `crates/runtime/spec/Tools/Exec/Container.lean` D39 规定，输入值由 kernel Config 的 container 契约规定。冻结配置经 `ExecTool::with_container` 传到 exec，daemon 身份与取消归 Backlog，起动前 inspect 核对限额与挂载；guardian 处理父进程 EOF 与迟到 create，五轴的实际范围与实测入口由同一分部 §3／§16 规定，不能将非 root UID 声称为独立宿主安全主体。`docker`／`podman` 二者取先通过 daemon admission 的一个；镜像必须是 User 自行准备的本地固定 ID，无默认镜像且不隐式拉取。安装指引指向 D32 引的安装页。
5. **设置的「沙箱」控件**：列出这座城所在的电脑解得出的每个名字与它的五轴清单（读 doctor 的同一份 `DoctorSandbox`，不另算），缺的给 `Missing` 的那一句与安装指引；选中写 `[sandbox] arm`（经设置的那扇门）。
6. **doctor 每臂一行**：`crates/sprawling/spec/Doctor.lean` 的 doctor 表为 `native`、`container`、`python` 各加一行，读 `Offerings` 的同一次采样。
7. **每条保证的测试**：每个构造得出的臂，五轴各一个真命令的对拍——写工作目录（副本里有、源树没有）、连回环上一个监听的端口（开网的臂连得上、关网的臂连不上）、起一个比命令活得久的孙进程（进程树轴保时命令结束后它不在）、读自己的身份（用户轴保时与 harness 不同）、分配超过上限的内存（资源轴保时失败）；与 `every_sandbox_arm_states_what_it_does_not_hold` 并列，清单与对拍不一致即红。
8. **要另装的臂实跑一次**：`container` 在 GitHub 的 ubuntu runner（自带 Docker）上跑第 7 条的对拍；Linux 的 Landlock 回退在 D32 的未决解开之前不构造；macOS 的 Seatbelt 以 `Tools/Exec/NativeMacos.lean` D40 为接口与策略权威，已测生产文件与网络拒绝、初始化拒绝与直接主进程生命周期，有限平台结果不证明其它系统支持或同 RunId 聚合硬内存。

**未决（§3 口径）**：同步仍按命令读两侧的每个目录项，并逐字节比较同长的文件，代价随工作树的大小长；一棵带大构建缓存的工作树每条命令要读两遍缓存。判定它的证据是一棵真实 room 的每条命令同步耗时（毫秒）与其文件数的读数；若读数显示读取成了主项，再比较「按 mtime 与长度跳过、只对同一时间戳刻度里的文件逐字节比较」。
-/

/-! D38 Linux 放置前探测强制 user namespace（§8-13-2）

**决定**：强制参数与放置前探测由 `namespaced()` 的同一命令构造器提供；探测成功后才同步副本，探测失败以 `E_SANDBOX_DENIED` 拒绝。

**理由**：`place()` 返回的是尚未起动的命令，无法直接把目标 wrapper 的退出码变成类型化拒绝。每次探测避免两条命令之间内核或 LSM 的许可变化被缓存掩盖，目标命令仍带强制参数。

**被否**：仅加参数而让缺能力落成普通命令退出；许可不足时静默退到 `CopiedTree`。

**代价与重开参数**：每次 Linux 放置多起一个短命 wrapper；backlog 提供能区分 wrapper 起动失败与目标命令失败的起动协议时，可在同一次起动里报拒绝。
-/

/-!
### 8-13-3 runtime::tools::exec::yielding（派出的命令降一级；形状 4 adapter）


agent 派出去的构建、测试与 sprawling 的记账、视图、socket 服务抢同一批核；控制面必须赢，所以 exec 派出的每一条宿主命令都以低于核心的优先级起动，而不是继承核心的优先级。本模块只做「把一条 `Command` 改成降一级起动」这一件事，不决定哪条命令要降——凡经 `through_the_backlog` 的命令一律降，那是全部宿主子进程的唯一产地（§8-14），故降级只有这一处权威。

```rust
pub(super) fn one_level_down(Command, Shares) -> Result<Command, AxError>;
#[cfg(unix)] pub(super) fn require_executable(&Command) -> Result<(), AxError>;
pub(super) fn Confined::prepare(&mut self, Command, &Path, Shares) -> Result<(Command, Placed), AxError>;
```

- **Windows**：`BELOW_NORMAL_PRIORITY_CLASS`（`0x0000_4000`，`WinBase.h`），经标准库的安全接口 `std::os::windows::process::CommandExt::creation_flags` 在创建时给出，不需要管理员，也不需要 `unsafe`。是绝对档位而非相对档位：核心在正常档时，子进程低一档。
- **Unix**：包一层 `nice -n 10 -- <program> <args>`（`--` 结束 `nice` 的选项，名字以连字符开头的程序才不会被读成选项），工作目录与环境变量逐项搬到外层命令上。`nice` 是 POSIX 规定的工具；增量是相对的，所以子进程总比核心低 10 个 nice 单位，不需要 `CAP_SYS_NICE`（降优先级从不需要特权）。Linux 上核心的 `PATH` 里有 `ionice` 时，再在外面包一层 `ionice -c 2 -n 7`：IO 优先级取 best-effort 类里最低的一级，而不是 idle 类——idle 类在磁盘忙时可以让一条构建一个字节也读不到，best-effort 最低级只是排在核心之后。没有 `ionice`（例如不带 util-linux 的系统）时只包 `nice`：IO 这一半是两半里较轻的一半，缺了它不该让每条命令都起动失败。macOS 上 `/usr/sbin/taskpolicy` 在时，再在外面包一层 `taskpolicy -c utility`，把命令与它的后代的 QoS 压到 utility，系统先把它们放到效率核上（D29）；不在时只包 `nice`，理由同 `ionice`。
- **次序**：Confined::prepare 统一选择放置与降级的顺序：MacosSeatbelt 在 nice/taskpolicy 降级之后进入策略，避免策略拒绝调度调用；其他臂的沙箱包装仍包住降过级的命令。`env_clear` 与环境白名单在所有包装完成后落在实际起动的最外层命令上；优先级沿进程树继承，所以 wrapper 下的真命令同样低一档。
- 失败：Unix 上 `nice` 自己总能起动，找不到的程序只会变成退出码 127 与 stderr 里的一行字，所以本模块在包 `nice` 之前先按 `execvp` 的找法（带分隔符的名字相对工作目录，裸名字沿核心的 `PATH`）确认程序是一个可执行文件，不是就返回 `E_TOOL_UNAVAILABLE`，动作与恢复同 backlog 的 spawn 失败（「check the program name, or use the shell arm」）。找不到 `nice` 本身时，spawn 在 backlog 里以同一个码报出。Windows 上不包外层，找不到程序仍在 spawn 处失败。Sandbox 放置下这一查找发生在宿主上，沙箱里看见的 `PATH` 若不同，结果以沙箱里的起动为准。
- 证据：`crates/runtime/src/tools/exec/tests/yielding.rs` 的 `a_dispatched_command_runs_below_the_core`——同一条读自身优先级的命令，直接起动一次、经 exec 起动一次，断言后者的档位严格低于前者；Linux 臂 `a_dispatched_command_reads_and_writes_at_the_lowest_best_effort_io_level`——经 exec 起动的 `ionice` 读回自己的 IO 档位是 `best-effort: prio 7`。这一条只在 Linux 上编译与运行，先红与转绿都在合并火车的 Linux 任务里看到。

**决定（平台调用的取法）**：子进程的 CPU 优先级取第一档「安全 Rust」——`creation_flags` 与 `nice` 都是对外只给安全接口的现成路，不需要 Zig 叶子，也不需要 Lean 证明边界。**被否**：①起动后再对子进程调 `SetPriorityClass`／`setpriority`——要 FFI（`unsafe` 或 Zig 叶子），且子进程在改档之前已经以正常档跑了一段；②Unix 上用 `CommandExt::pre_exec` 调 `nice(2)`——`pre_exec` 本身是 `unsafe`。**重开参数**：Unix 主机上出现不带 `nice` 的受支持平台，或测得多包一层 `nice` 的起动开销占到一条命令墙钟时间的可见比例。

**逐 run 的 Job Object（`runtime::backlog::jobs`，形状 4 adapter）**：派出的命令起动之后，谁在吃内存要能归到派出它的 run，而一条 `cargo test` 真正吃内存的是它起的 `rustc` 与测试进程，不是 `cargo` 自己。所以 Windows 上每个 run 一个匿名 Job Object：`Backlog::run` 起动的子进程在登记进表的同一时刻装进它 owner 的 job（第一次装时创建），job 里的进程再起的进程由系统自动装进同一个 job，于是 job 的进程表就是这个 run 的整棵进程树。`Backlog::release(owner)` 丢掉这只 job 的句柄；job 不设 kill-on-close，丢句柄不杀进程，杀进程仍只归 `release` 与 `halt`。macOS 与 Linux 上没有 Job Object：两者都只读到命令自己的 pid（下面的 `unfollowed` 计每一条），因为进程组与 cgroup 的进程表都还没有接成读数；按 run 的 CPU 份额在 macOS 上是 `taskpolicy -c utility`，在 Linux 上是 cgroup v2 的 `cpu.weight`（没有委派时只有 `nice` 一档），分别在 D29 与 D33。

```rust
pub enum RunAffinity { Os, Mask(NonZeroUsize) } // 同一处理器组内非零掩码；缺省 Os
pub struct RunProcesses { pub pids: BTreeSet<u32>, pub unfollowed: u32, pub share: Shares, pub affinity: RunAffinity }
pub enum Shares { Unset, Cpu, CpuAndMemory { limit: NonZeroU64 } } // 每个 run 的份额（D29）；缺省 Unset
impl Backlog {
    pub fn with_shares(self, shares: Shares) -> Backlog;   // 这张表起动的每个 run 要的份额；不调时 Unset
    pub fn shares(&self) -> Shares;
    pub fn with_affinity(self, affinity: RunAffinity) -> Backlog;
    pub fn processes(&self) -> Result<BTreeMap<RunId, RunProcesses>, AxError>;
}
```

- `Shares` 是人的配置 `[core] placement` 那一臂里属于子进程的一半，由 `bin::serving::placement::run_shares` 定（`crates/sprawling/spec/Serving/Placement.lean` D47），经 `accounting::worker::hands::Hands.shares` 交进车队的表；runtime 不读人的配置。一张没人交过份额的表是 `Unset`：测试与不起 run 命令的地方都是它。
- `RunAffinity` 只收值：placement D49 在 pin 核心前取可用掩码减计划掩码，经 `Hands.affinity` 与车队交给 `Backlog::with_affinity`，runtime 不读配置、不算补集。Windows 创建 run 的 job 后先设置份额，再查询现有扩展限额、用 `win32job` 2.0.3 的 `limit_affinity` 加亲和、写回，保留已有内存限额。成功时 `RunProcesses.affinity` 为请求掩码；拒绝、未请求或非 Windows 时为 `Os`，份额报告仍只说明成功的份额。创建或加入 job 失败仍计 `unfollowed`，申请失败不改命令退出结果；后续命令复用第一条命令的 job 与限额。接口只约束当前组，未覆盖跨组调度与起动到加入 job 的窗口。
- `RunProcesses.share` 是这个 run 此刻实际拿到的份额：要了而平台给了，就是要的那一个；要了而平台拒绝（叶子调用失败、cgroup 不可写）或这个平台没有份额，就是 `Unset`。
- `processes` 按 run 给出它此刻在表里的进程：owner 是这个 run（窗口内或已转后台）的每条命令自己的 pid，并上这个 run 的 job 的进程表（job 只列还活着的进程）。已经结束、还没被 `harvest` 的命令仍列出自己的 pid，读数的一方在它后面读不到计数。已 `release` 的命令（owner 为 nobody）不归任何 run。
- `unfollowed` 是这个 run 的命令里有几条只读到了命令本身、没读到它起的进程：Windows 上创建 job、装进 job 或读 job 的进程表失败的那几条（这一次读数里整个 run 的命令都算），Unix 上是每一条，因为 Unix 上没有 Job Object（进程组是它的对应物，尚未接入）。装不进 job 不让命令起动失败：job 只服务于读数，为读数让一条构建失败是把代价付错了地方，失败落在 `unfollowed` 里给读数的人看。
- `share`：Windows 上 job 在创建的同一刻经 `desktop_ffi::cpu::job_share` 设权重 `RUN_CPU_WEIGHT`（5）与 `Shares::CpuAndMemory { limit }` 要的内存上限，设上了这个 run 读到的就是要的那一个值；job 拒了时 job 照旧跟进程树，这个 run 读作 `Unset`。Linux 上 harness 自己的 cgroup 可写时，这个 run 的 `cpu.weight` 与（要了内存上限时的）`memory.max` 就是它读到的份额（`runtime::backlog::cgroup`，D33）；不可写或收不下时读作 `Unset`。macOS 上恒为 `Unset`：`taskpolicy` 是命令外面的一层包装，没有 job 或 cgroup 可以读回（D29）。
- 失败：表够不着时 `E_STORAGE_FATAL`，与 `Backlog` 的其他读法相同。
- 每个进程的内存与 CPU 不在这里读：本 crate 不读平台计数（见下），由 sprawling 的 `bin::monitor` 按这里给出的 pid 去读。
- 证据：`crates/runtime/src/backlog/tests.rs` 的 `a_run_owns_the_processes_its_commands_started`——一条经 `run` 转后台的命令，其 pid 出现在它 owner 的那一项里，别的 run 那一项里没有；Windows 上 `unfollowed` 为 0、`share` 是要的那一个，Linux 上 cgroup 委派时同样、macOS 上与不可写的 Linux 上是 `Unset`；`release` 之后这个 run 不再出现。

**决定（job 的取法）**：Job Object 经 `win32job` 2.0.3（`Job::create`、`assign_process`、`query_process_id_list`，对外只给安全接口），本 crate 不写 `unsafe`；子进程的句柄经标准库的 `AsRawHandle` 取得。**被否**：①一整座城一只 job——分不出 run，而分解到 run 正是要它的原因；②只记直接子进程的 pid——`cargo`、`npm`、`sh -c` 这类命令自己几乎不占内存，读数会把一条吃掉几 GiB 的构建报成几 MiB；③`CREATE_SUSPENDED` 起动再装 job 再恢复——恢复线程要 FFI。代价：子进程从起动到装进 job 之间有一小段时间，那一段里它再起的进程不在 job 里（`cmd /C` 这类命令的第一个孙进程在这一段里起动的可能很小，但不为零）。**重开参数**：一个对外只给安全接口的 crate 能以挂起态起动子进程并在恢复前装进 job，或者读数显示 `unfollowed` 之外还有漏掉的孙进程。

**未决（§3 口径）**：核心线程升到正常档之上一级与空转安全阀在 `crates/sprawling/Spec.lean` §8-93；派出进程的 CPU 份额在 Windows 上已经按 D29 落地（每个 run 的 job 带权重 5，经 `crates/desktop/ffi` 的 Zig 叶子），作业级内存上限的叶子函数也已在（`desktop_ffi::cpu::job_share` 的 `memory`），要不要它由人的 `[core] placement` 那一臂定（`crates/sprawling/spec/Serving/Placement.lean` D47）；macOS 上派出的命令已包在 `/usr/sbin/taskpolicy -c utility` 里（`yielding` 的 `start_below_the_core`，找不到它时只包 `nice`），Linux 的 cgroup 委派由 `runtime::backlog::cgroup` 读（D33）；这条路不在第一档，是因为 `win32job` 2.0.3 的安全接口够得着 Job Object 本身（上文「job 的取法」），够不着这两项限额：它对外给出的内存限额只有按进程的工作集上限（`limit_working_memory`，即 `JOB_OBJECT_LIMIT_WORKINGSET`），限的是常驻页而不是提交量，不兑现「资源」一轴，且设它要不要特权随账户令牌而变（§8-13-2 记着两次读数：经 `win32job` 得 os error 1314，直接调 `SetInformationJobObject` 得成功），所以它不是任何一项限额的取法。作业级提交上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`）不要特权，但设它的字段在 `win32job` 里是 crate 私有的，`process-wrap` 10.0.1 与 `windows-spawn` 0.1.0 也不设它。一个对外只给安全接口的 crate 公开 `JOB_OBJECT_LIMIT_JOB_MEMORY` 与 CPU 速率控制时，D29 的叶子函数换成它。重命令共用的额度池尚未落地：池的大小由测得的核数与可用内存推出，哪些命令算重由城配置给默认表；可用内存的读数只在 sprawling 的 `bin::monitor::memory` 里读（`crates/sprawling/Spec.lean` §8-94），由 `bin::assembly` 交给本 crate，本 crate 不读平台。内存紧时新 run 排队已在 `crates/sprawling/Spec.lean` §8-46-3。
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

- **哪一臂打开哪一项**：下面每一项都随人的配置 `[core] placement` 那一臂开关，开关表只在 `crates/sprawling/spec/Serving/Placement.lean` D47 一处；本 crate 只收一个值 `Shares`（`Backlog::with_shares`）：`Unset` 什么也不设（`"none"`），`Cpu` 设 CPU 份额（缺省 `"soft"`），`CpuAndMemory { limit }` 再设内存上限（`"soft_shares"`）。
- **Windows (d)**：`runtime::backlog::jobs` 创建一个 run 的 job 时，按 `Shares` 给它设至多两项：CPU 速率控制 `JOBOBJECT_CPU_RATE_CONTROL_INFORMATION { ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_WEIGHT_BASED, Weight: 5 }`（`SetInformationJobObject`，信息类 `JobObjectCpuRateControlInformation`），与作业级提交上限 `JOB_OBJECT_LIMIT_JOB_MEMORY`（读出 `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`、置上这一位与 `JobMemoryLimit`、再写回，于是别处已经设的位不丢）。接口档位：`win32job` 2.0.3 把扩展限额的结构放在 crate 私有字段里、不设 CPU 速率控制，`process-wrap` 10.0.1 也不设，所以走第二档 Zig 叶子，放在 `crates/desktop/ffi`（与 `crates/sprawling/spec/Serving/Standing.lean` D40 同一个叶子）：job 的句柄由 `win32job::Job::handle` 交出（公开），两项限额作为一个定长记录过 `(ptr, len)` 边界。两项都不要特权（推断：文档都没有要求特权，而工作集上限要不要特权随账户令牌而变，见 §8-13-2；派生的 Rust 检查在未提权的 Windows runner 上设好再读回，先红后绿）。
  - **权重**：`Shares` 不是 `Unset` 时设；每个 run 一样，取 5（范围 1–9），常量 `RUN_CPU_WEIGHT`。要的是按 run 公平：不设时一条起 16 个进程的构建按线程分到 16 份，对面只有一个进程的 run 分到 1 份；权重相同，每个 run 各得一份，一个 session 的编译饿不死别的 session。harness 不进任何 job，它的热线程本来就站在正常档之上（§8-93）。
  - **内存上限**：`Shares::CpuAndMemory { limit }` 时设，`limit` 是 User 在 `[core] memory_bytes` 明确填写的非零字节数；`bin::serving::placement::run_shares` 把该值放进份额，本 crate 不读平台。超过时是这个 run 的进程树里的分配失败（编译器报内存不足），城与别的 run 照常。只有 `"soft_shares"` 打开它（D47），缺席即不设，没有自动计算的缺省值。
  - **撞到上限与没落地的上限**：每条命令的结果带上这条上限在它运行期间落到哪一种，见 D95；单凭非零退出码不把失败归给上限。
  - **退路**：叶子调用失败时 job 不设份额、不设上限，命令照常起动（与装不进 job 同一条判断：为读数或份额让一条构建失败是把代价付错了地方），这个 run 的 `RunProcesses.share` 读作 `Unset`；User 填了上限时，这个 run 的每条命令的结果说上限没有落地（D95），而不是静默地不设。
- **macOS (d)**：没有 Job Object。派出的命令在 `nice` 外面再包一层 `/usr/sbin/taskpolicy -c utility`，把它与它的后代的 QoS 压到 utility，系统于是先把它们放到效率核上（外部命令，第一档，与 `nice` 同一种做法，§8-13-3）；这一层是 macOS 的 CPU 份额一项，`Shares::Unset` 时不包；找不到 `taskpolicy` 时只包 `nice`。本实现没有按 run 整棵进程树汇总的内存上限，`CpuAndMemory` 在 macOS 上只兑现 CPU 一半，`RunProcesses.share` 为 `Unset`，doctor 照实说。
  - **机制与强制范围**：Apple 的 [XNU `bsd/kern/kern_resource.c`](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c) 在 `dosetrlimit` 的 `RLIMIT_AS` 分支调用 `vm_map_set_size_limit(current_map(), newrlim->rlim_cur)`，因此不能笼统说 macOS 不执行地址空间限额；支持情况须按目标系统核实。shell 的 `ulimit` 包装可以绕过 Rust 的 `pre_exec`，所以 `unsafe` 也不是排除包装的理由。但 `current_map()` 限的是单个进程的虚拟地址空间，不是同一 run 的所有进程合计提交量；后代即使继承相同额度，N 个进程仍能各用一份，一个 run 的多条命令也不共享额度。它既不是 job 的提交量口径，也不是 cgroup 的树级口径。
  - **决定与未决**：不以逐进程 `ulimit` 冒充按 run 的总内存上限，不为本 crate 新增改变命令起动语义的 shell 包装。尚缺可安全调用、无需额外权限、按 run 汇总整棵进程树、后代不能退出额度的 macOS 机制及真实树级超限验证；找到满足这些条件的接口或允许外部受控执行服务时重开。申请路径未接入，所以没有虚构的设置失败处理；当前明确退到 CPU 包装、内存不强制，命令按原来的退出结果返回。
- **Linux (d)**：harness 自己所在的 cgroup 是 `/proc/self/cgroup` 的 `0::` 行所指的那一个，挂在 `/sys/fs/cgroup` 下（`runtime::backlog::cgroup`，D33）。它可写时，harness 先把自己移进一个子 cgroup `core`（cgroup v2 规定有进程的 cgroup 不能再往下分资源，`core` 把父 cgroup 空出来），在父 cgroup 的 `cgroup.subtree_control` 打开 `cpu` 与 `memory`，然后每个 run 建一个子 cgroup `run-<RunId>`：`cpu.weight` 写 100（每个 run 一样），`memory.max` 在 `Shares::CpuAndMemory { limit }` 时写 `limit`，CPU-only 时写 `max` 以清除重用目录的旧上限（D47 的 `"soft_shares"` 臂给的是 User 明确填写的字节上限），命令起动后把它的 pid 写进这个子 cgroup 的 `cgroup.procs`（同一 run 的后续命令只写自己的 pid，份额在建 cgroup 时已经写下）。全是标准库读写文件，第一档。与 Windows 的 job 一样，起动到写进 cgroup 之间有一小段，那一段里起的孙进程留在 `core` 里。不可写时（没有 systemd 的委派，CI 主机与许多桌面都是这样）只靠 `nice 10`，doctor 说「runs' commands compete thread by thread and run below the core: the cgroup is not delegated」。cgroup 收不下一个 run 时（建目录或写文件失败）这个 run 读作 `Unset`，命令照常起动：与 Windows 的叶子失败同一条判断；一条命令的 pid 写不进 `cgroup.procs` 时它不在上限之内。两种情形下 User 填了上限，结果都说上限没有落地（D95）。委派与否由 `runtime::platform_shares` 一处读出，doctor 与接线读同一个答案。
- **(e) `WindowsJobObject` 臂**：`NativeWindows.lean` D53/D54 规定无 capability AppContainer 与两层 job；叶子拥有主线程句柄并在两个 job 的 assignment 成功之后恢复，不用 `std::process::Child` 先起动再装 native job。
- **为什么不缩清单**：一只只保文件系统与「起动之后的进程树」的 job 臂，对 Agent 来说与 `CopiedTree` 几乎一样，多出的只是 kill-on-close；多一臂只多一句要读的话，不多一项保证。

**被否**：①`win32job` 的调度级别（`limit_scheduling_class`，安全接口）当作 CPU 份额——它只改同一优先级类里各 job 线程的时间片长短，每个 run 的级别都一样时什么也没分；②工作集上限（`limit_working_memory`）——限的是常驻页，不是提交量，而且要不要特权随账户令牌而变（§8-13-2）；③硬的 CPU 速率上限（`JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`）——机器空着时也让核闲着，与「忙了立刻换下一个核」相反；④每条命令一只 job——份额要按 session 分，不是按命令；⑤Linux 上用 `systemd-run --user --scope -p CPUWeight=…` 包每条命令——每条命令多一次 D-Bus 往返与一个 scope 的起动，而且要有用户级 systemd，不是每台机器都有；直接写委派的 cgroup 文件更少依赖。

**重开参数**：四臂对照里 ② 臂（缺省，含 CPU 份额）的 p99 与 p999 不优于 ① 臂，而把份额单独拆出来也无益——那就整个 (d) 都不做；内存上限只由显式用户配置决定，不由四臂对照决定；或者一个对外只给安全接口的 crate 公开 job 的 CPU 速率控制与作业级内存上限（叶子函数换成它）；或者 Rust 稳定版提供 `raw_attribute`（(e) 的恢复函数就不需要了）。
-/

/-! D95 一条命令的结果说出 User 的内存上限在它运行期间落到哪一种（设置覆盖；`runtime::backlog::ceiling`、`runtime::backlog::jobs`、`runtime::backlog::cgroup`、`runtime::tools::exec::outcome`）

**来源**：这是一条 ruling：run 的内存没有缺省上限，只有 User 填的上限；撞到它时，命令结果与界面都给出明确的错误；执行路径上失败而不报错的地方都要报出来。上限只在 `[core] placement = "soft_shares"` 且填了 `[core] memory_bytes` 时被要求（`crates/sprawling/spec/Serving/Placement.lean` D47），本决定只管被要求之后。

**接口**：

```rust
pub enum Ceiling {                                   // runtime 根上导出
    Hit { limit: NonZeroU64 },                       // 这条命令运行期间，这个 run 的分配在上限处被拒
    Unapplied { limit: NonZeroU64, why: Unapplied }, // 要了上限，这条命令却不在上限之内运行
    Unread { limit: NonZeroU64 },                    // 上限在，撞没撞读不出来
}
pub enum Unapplied { Platform, NotDelegated, Refused, Unjoined }
// Started::Settled 与 Finished 各多一个 ceiling: Option<Ceiling>；None 即没有要上限，或上限在而没撞到
```

命令入表时记下一个 `ceiling::Mark`（下面 `CeilingMark` 的 Rust 面），收走时由 `ceiling::verdict` 用那一刻读到的计数给出 `Option<Ceiling>`；这两者与「哪种情形算没落地」只在 `runtime::backlog::ceiling` 一处，`jobs` 与 `cgroup` 只读平台的计数。

exec 结果多一个键 `memory_ceiling`，形如 `{"state": "hit" | "unapplied" | "unread", "limit_bytes": N, "detail": "…"}`；`unapplied` 另带 `"why": "platform" | "not_delegated" | "refused" | "unjoined"`。后台命令在 `background` 行里带同一个键。键名与拼写只在 `tools::exec::outcome` 一处；客户端从结果读它（`client/src/views/monitor/trace.ts` 的 `commandOf`），在终端行下面画出错误。

**证据**：撞到上限按命令运行期间这个 run 的撞限计数有没有增加来判，计数在命令入表时记一次、在命令被收走时再读一次（`settle` 与 `harvest` 是收走命令的两处）。入表时就读不出计数，记下的是 `unread`，收走时报 `Unread`。同一 run 的上限是所有命令共用的，所以计数增加时，这期间还在跑的每条命令都报 `Hit`：上限说的是这个 run，而不是哪一个进程超了。

- **Windows**：run 的 job 建成时先挂一个 I/O completion port（`desktop_ffi::cpu::JobWatch`，`crates/desktop/ffi/Spec.lean` D6），读的是 `JOB_OBJECT_MSG_JOB_MEMORY_LIMIT` 消息的条数；读一次就把消息从 port 上取走，所以 run 累计这些条数，一次读失败之后取走的条数已经丢了，这个 run 此后的读数一律是读不出；挂不上就不设内存上限，这个 run 的命令报 `Unapplied::Refused`，于是一个设下的上限总是读得到的上限。native 臂的 command job 不再另设同值的内存上限：它嵌在 run job 里，run job 的上限已经管住整棵树，而两只同值的 job 嵌套时，撞限消息落到内层 job 的 port，外层的 port 一条也收不到（在一台 Windows 11 机器上以两只各挂 port 的嵌套 job 实测：内层设同值上限时内层计 1、外层计 0，内层不设时外层计 1），只设外层就只有一个读处。Microsoft 文档说 job 消息的投递不保证；一条没投递的消息漏报一次撞限，这一缺口按文档原样承认。
- **Linux**：读 `run-<RunId>/memory.events` 的 `oom` 一行（cgroup v2：用量到了 `memory.max`、分配将要失败时加一）。不用 `max` 一行：它在回收成功、分配并未失败时也加，会把一次正常的回收报成撞限。读不出这个文件时报 `Unread`，而不是当作没撞。
- **macOS**：没有按 run 的上限（D29），要了上限的每条命令报 `Unapplied::Platform`。
- **container 臂**：命令跑在容器运行时的进程里，不在 run 的 job 或 cgroup 里，它的内存由 building 的 `[sandbox.container] memory_bytes` 管（`Container.lean`）；要了上限时报 `Unapplied::Unjoined`。

**没落地即报**：要了上限、这条命令却不在上限之内运行时，结果报 `Unapplied`：平台没有这一项（`Platform`）、Linux 的 cgroup 没有委派（`NotDelegated`）、job 或 cgroup 拒了上限或 port（`Refused`）、命令没进 run 的 job 或 cgroup（`Unjoined`）。这些情形下命令照常起动（D29 的退路），但 User 填下的上限没有生效不再是静默的。

**被否**：①按非零退出码与 stderr 里的 “out of memory” 字样归因：编译器与运行时的措辞各不相同，而且一条因别的原因失败的命令会被报成撞限；②读 job 的 `PeakJobMemoryUsed`：一次被拒的大分配不会把峰值推到上限，峰值说明不了撞没撞；③撞限时让命令失败或杀掉 run：分配被拒的进程自己决定怎么收场，城只报告；④只在第一条命令报一次 `Unapplied`：每条结果都是模型单独读的，后来的结果不报就等于说上限在。

**重开参数**：Windows 给出按 job 读出撞限次数的查询（不经 completion port）；Linux 的 cgroup 接口改了 `memory.events` 的语义；macOS 出现 D29 所说的按 run 汇总的机制。
-/

/-! D33 每个 Linux run 的 cgroup 由 `runtime::backlog::cgroup` 一个模块建，根是参数（D29）

**接口**（`crates/runtime/src/backlog/cgroup.rs`，形状 4 adapter）

```rust
pub enum PlatformShares { None, Cpu, CpuAndMemory }  // 运行中的机器给得了哪几半（D29）；根上导出
pub fn platform_shares() -> PlatformShares;          // 一处读委派：Linux 的 cgroup 可写给 CpuAndMemory，否则 None
struct Cgroups { ... }                                // 每个 run 一个子 cgroup，父 cgroup 由建它的那一方给
impl Cgroups {
    fn adopt(parent: &Path, pid: u32) -> Cgroups;     // 移进 core、打开 cpu 与 memory
    fn enter(&mut self, run: RunId, pid: u32, asked: Shares) -> Shares;  // 这个 run 现在拿到的份额
    fn held(&self, run: RunId) -> Shares;
}
```

**决定**：

- **根是参数**：`Cgroups::adopt` 收一个父 cgroup 路径，生产路径给的是 `/sys/fs/cgroup` 接上 `/proc/self/cgroup` 的 `0::` 行，测试给的是一个临时目录。于是这个模块的全部读写能在 Windows 上用一个仿 `/sys/fs/cgroup` 的目录树验证，不必等一台 Linux 机器：cgroup v2 的接口就是几个普通文件，读写它们用的也是普通文件读写。
- **可写性是干读**：`platform_shares` 与 `adopt` 都先看父 cgroup 的 `cgroup.procs` 与 `cgroup.subtree_control` 能不能以写方式打开；打开失败即没有委派，什么也不建。这是在不改动机器状态的前提下能问的问题，也是 doctor（另一个进程，只知道运行中的机器、不知道服务进程建过什么）读到的同一个答案。真正动状态的是 `adopt` 的三步：建 `core`、把自己的 pid 写进 `core/cgroup.procs`、把 `+cpu +memory` 写进父 cgroup 的 `cgroup.subtree_control`。
- **一个 run 一个孩子，名字是 `run-<RunId>`**：第一条命令进表时建目录并写下份额，之后同一条 run 的每条命令只把自己的 pid 写进 `cgroup.procs`——cgroup 是进程表，不是每个命令一张。第一条命令的 pid 与份额同一次写进，所以这个 run 从第一条命令起就在自己的份额里；写 pid 失败不让命令失败，这个 run 读作它已经拿到的份额（Windows 的 job 装不下时同样只落在 `unfollowed` 里）。
- **读回**：`RunProcesses.share` 读的是这个模块记下的、当时确实写下的值；`Shares::Unset` 一个字节也不写，一个目录也不建，所以四臂对照的 ① 臂在 Linux 上不留任何 cgroup。`release` 丢掉这个 run 的记录，不删目录：cgroup 里有活进程时目录删不掉，而停进程仍只归 `release` 与 `halt`。

**被否**：①`systemd-run --user --scope` 包每条命令：要用户级 systemd、每条命令一次 D-Bus 往返（D29 已否）；②对每条命令建一个 cgroup：份额按 run 分，不按命令分（与 job 同理）；③建之前用 `stat` 的权限位判断可写：cgroup 文件系统的权限位由内核按挂载参数报出，不以写方式打开一次就问不出「写下去会不会被拒」；④把父 cgroup 的位置编译进常量：容器与 systemd 的用户实例把 harness 放在哪一层各不相同，`/proc/self/cgroup` 是唯一的权威。

**重开参数**：Linux 的读数显示按权重分与 `nice` 相比没有差别（与 D29 的四臂对照同一个标准）；或者一个文件系统事件使 harness 的 cgroup 在进程中途变得可写——那时才需要重试 `adopt`，今天 `adopt` 只在第一条要份额的命令进表时试一次。
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
| `native` | 命名空间包装程序 `bwrap`（今天的臂） | Linux | 否，要内核允许未提权的 user namespace；不可创建时拒绝，即使 `bwrap` 以 setuid 装着 | 保（整机只读可见） | 保 | 保（`--die-with-parent`） | 保（强制 `--unshare-user`；不可创建时拒绝，§8-13-2） | 不保（D29 的 cgroup 另给 CPU 与内存） | 能，但居民要开网，开网时网络一轴不保 | 发行版的 `bubblewrap` 包 |
| `native` | Landlock 加 seccomp | Linux | 否：Landlock 让任何进程、包括未提权的进程限制自己；内核 5.13 起，且要编进内核并在启动时启用；TCP 规则从 ABI v4 起，UDP 从 v10 起；seccomp 过滤要先 `PR_SET_NO_NEW_PRIVS` | 保（还能限读） | 条件：按 ABI，只管 TCP（与 v10 起的 UDP） | 不保 | 不保 | 不保 | 能 | 要在子进程里自限：`pre_exec` 是 `unsafe`，所以只能由 harness 以自己的一个子命令重新起动、自限之后再 `exec` 目标命令；这一做法未核实，进未决 |
| `container` | rootless Podman／Docker | Linux | 装要包管理器；运行要 `newuidmap`、`newgidmap` 与 `/etc/subuid`、`/etc/subgid` 里至少 65536 个从属 id（官方页） | 保 | 保 | 保 | 保 | 保（cgroup v2 委派时） | 能 | 外部运行时与镜像；rootless Podman 的网络走 pasta |
| （`container` 的运行时） | gVisor `runsc` | Linux | 经 Docker、Kubernetes 或直接用 `runsc`（官方页） | 保 | 保 | 保 | 保 | 保 | 能 | 一个 OCI 运行时，在已有的容器臂下作为可选项，不另起名字 |
| （不收） | microVM（Firecracker） | Linux | 要 KVM（`/dev/kvm`） | 保 | 保 | 保 | 保 | 保 | 能 | 要内核镜像与根文件系统的供给；桌面与 CI 主机常没有 `/dev/kvm` |
| `native` | Seatbelt（`sandbox-exec` 配置） | macOS | macOS 26.6.2 runner 上无需提权；缺席或策略拒绝时拒开 | 保（写入只落副本；读取不受限） | 保（TCP／UDP，fork／exec 后代同受策略约束） | 不保 | 不保 | 不保 | 不适用：当前只放 exec，关网不能直接装要连 provider 的居民 | 系统 `sandbox-exec(1)` 已标 DEPRECATED；平台继承假设与支持边界见 `crates/runtime/spec/Tools/Exec/NativeMacos.lean` |
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
| Windows | `copied_tree`；缺省读取 wire D26，SB1 第 3 条六项 native 回归通过之前保持这一选择 | `native`（Job Object 加 AppContainer 的 exec，Job Object 的居民）、`container`（Docker Desktop 或 Podman Desktop，要另装）、`python`、`none` |
| macOS | `copied_tree`，缺省读取 wire D26 | `native`（Seatbelt，文件写入与网络；其余三轴不保，见 D40）、`container`（Docker Desktop 或 Podman Desktop，要另装）、`python`、`none` |
| Linux | `native`：`bwrap`；没有 `bwrap` 时解出 `copied_tree` 并照实说 | `copied_tree`、`container`（rootless Podman 或 Docker，要另装；gVisor 作为它的运行时）、`python`、`none` |

安装指引：Docker Desktop（https://docs.docker.com/desktop/setup/install/windows-install/ 、https://docs.docker.com/desktop/setup/install/mac-install/）、Podman Desktop（https://podman-desktop.io/docs/installation/windows-install）、rootless Podman（https://github.com/containers/podman/blob/main/docs/tutorials/rootless_tutorial.md）、rootless Docker（https://docs.docker.com/engine/security/rootless/）、bubblewrap（https://github.com/containers/bubblewrap）。

**理由**：按 Roadmap §6 SB 列出的六项依次比较。①方便：缺省的臂不要另装，一台新机器开城就有；`container` 要装一个运行时与镜像，只作可选。②不要管理员（D20）：Windows 上探测过的不要管理员的三种机制里，Job Object 与 AppContainer 合起来保五轴（AppContainer 给网络与用户，job 给进程树与资源，副本给文件），受限令牌给的被 AppContainer 覆盖；Windows Sandbox 要专业版以上加管理员，WSL 2 的安装要管理员。③五轴：Windows 的 `native` 保五轴，Linux 的 `bwrap` 保四轴（资源由 D29 的 cgroup 另给），都多于 `copied_tree` 的一轴。④居民：job 装得下居民；关网的机制装居民时网络一轴不保，这一点写进清单而不是挑一个能关居民网络的臂——没有这样的臂。⑤依赖（D23）：缺省的臂只依赖平台本身与已有的 Zig 叶子（Windows）或发行版的一个小包（Linux）；容器运行时、gVisor、microVM 与 OpenShell 各是一个大的外部系统。⑥三个平台（D94）：三个平台都有 `copied_tree`、`container` 与 `python`，`native` 在 macOS 上的显式臂由 D40 给出文件写入与网络两项保证，初始化失败照实拒绝；其余三轴不保，不以此关闭聚合内存的未决。缺省的 `native` 缺机制时退到 `copied_tree` 而不是拒：缺省是城替 User 选的，一条拒绝会让一台没装 `bwrap` 的机器上 exec 整个不能用；退的时候 `statement()` 与 doctor 说出退了、缺什么，所以不是静默变弱。User 明写的名字缺机制时拒，因为那是 User 要的那种盒子。

**被否**：①Windows 缺省 `copied_tree` 不变：它对 Agent 只保文件一轴，而不要管理员的机制能保五轴；②Windows Sandbox 作 Windows 的 `native`：要专业版以上与管理员，探测机就没有；③缺省 `container`：每台机器先要装一个运行时、拉一个镜像，与「方便」与 D23 都相反；④OpenShell 作缺省：它的隔离是 Landlock、seccomp 与容器，已各成一行，多出的是一个 gateway 与一套自己的策略层，Windows 上只是实验性的；⑤Linux 缺省换成 Landlock 加 seccomp：不保进程树与用户，网络只管 TCP，而且起动方式未核实；它留作 `bwrap` 缺席时 `native` 的第二种机制，等未决解开；⑥按产品给每个机制一个名字：D26 已否。

**重开参数**：macOS 的已弃用接口在拟支持的系统范围中仍能稳定保持 D40 策略，并完成真实前后台生命周期检查时，重新论证缺省选择并修改唯一权威 wire D26；Windows 的 Zig 叶子证明做不出 AppContainer 起动时，缩小实际保证而不保留错误清单；User 按有效接口另定缺省时，更新该权威。

**未决（§3 口径）**：①macOS 的受支持系统范围、已弃用 Seatbelt 的维护边界与后台生命周期：D40 的显式臂保持文件写入与网络策略，真实 runner 手册标为 deprecated，有限命令对拍不证明其它系统或整棵后台进程树的终止；判定证据是每个拟支持系统的手册、固定 deny-default 策略下的生产 ExecTool 行为，以及后台/halt/release 的实际结果；聚合硬内存仍由 D29 定义，D40 不提供它；②Linux 的 Landlock 回退：判定它的证据是一个不写 `unsafe` 的起动方式（harness 以自己的子命令自限后 `exec`）在 Linux runner 上跑通，并读出 runner 内核的 Landlock ABI。
-/

namespace Runtime.Tools.Exec

/-- 有效的申请值；平台拒绝与不申请均没有实际掩码。 -/
inductive AffinityAttempt where
  | unrequested
  | accepted (mask : Nat) (nonzero : mask > 0)
  | refused
  deriving Repr

def heldAffinity : AffinityAttempt → Option Nat
  | .unrequested => none
  | .accepted mask _ => some mask
  | .refused => none

/-- 未建 job 与已建 job 分开：首条命令之后不重新申请限额。 -/
inductive RunJobState where
  | absent
  | created (affinity : Option Nat)
  deriving Repr

/-- 每条命令尝试入表；建 job 失败则下一条仍可重试。 -/
inductive JobEntry where
  | creationRefused
  | created (attempt : AffinityAttempt)
  deriving Repr

def enterJob : RunJobState → JobEntry → RunJobState
  | .created mask, _ => .created mask
  | .absent, .creationRefused => .absent
  | .absent, .created attempt => .created (heldAffinity attempt)

def enterTrace (state : RunJobState) (entries : List JobEntry) : RunJobState :=
  entries.foldl enterJob state

/-- 无论后续命令申请什么，run 都保留首次建成的 job 限额。 -/
theorem created_job_keeps_affinity_on_every_trace (mask : Option Nat)
    (entries : List JobEntry) : enterTrace (.created mask) entries = .created mask := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
    simpa [enterTrace, List.foldl, enterJob] using ih

/-! ### D95 的判定：一条命令收走时报哪一种

`CeilingMark` 是命令入表时记下的：没要上限、要了却没落地、入表时计数就读不出、或上限在并记下当时的撞限计数。
`ceilingAt` 用收走时读到的计数（读不出是 `none`）给出报告。Rust 的 `ceiling::verdict` 是同一个
函数，`ceiling::tests` 的 proptest 在全部输入上检查下面两条定理所说的性质。 -/

inductive CeilingMark where
  | notAsked
  | unapplied
  | unread
  | watching (before : Nat)
  deriving Repr, DecidableEq

inductive CeilingReport where
  | silent
  | hit
  | unapplied
  | unread
  deriving Repr, DecidableEq

def ceilingAt : CeilingMark → Option Nat → CeilingReport
  | .notAsked, _ => .silent
  | .unapplied, _ => .unapplied
  | .unread, _ => .unread
  | .watching _, none => .unread
  | .watching before, some after => if before < after then .hit else .silent

/-- 撞限只在上限在、计数读得出且增加时报。 -/
theorem hit_only_when_the_count_moved (mark : CeilingMark) (read : Option Nat) :
    ceilingAt mark read = .hit ↔ ∃ before after, mark = .watching before ∧ read = some after ∧ before < after := by
  cases mark with
  | notAsked => simp [ceilingAt]
  | unapplied => simp [ceilingAt]
  | unread => simp [ceilingAt]
  | watching before =>
    cases read with
    | none => simp [ceilingAt]
    | some after =>
      by_cases moved : before < after <;> simp [ceilingAt, moved]

/-- 要了上限的命令只有在上限在、计数读得出且没动时才不报：没落地与读不出都说出来。 -/
theorem an_asked_ceiling_is_silent_only_when_held_and_unhit (mark : CeilingMark) (read : Option Nat)
    (asked : mark ≠ .notAsked) (silent : ceilingAt mark read = .silent) :
    ∃ before after, mark = .watching before ∧ read = some after ∧ after ≤ before := by
  cases mark with
  | notAsked => exact absurd rfl asked
  | unapplied => simp [ceilingAt] at silent
  | unread => simp [ceilingAt] at silent
  | watching before =>
    cases read with
    | none => simp [ceilingAt] at silent
    | some after =>
      by_cases moved : before < after
      · simp [ceilingAt, moved] at silent
      · exact ⟨before, after, rfl, rfl, Nat.le_of_not_lt moved⟩

end Runtime.Tools.Exec
