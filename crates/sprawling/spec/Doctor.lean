-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::doctor

规定 `crates/sprawling/src/doctor.rs` 与 `crates/sprawling/src/doctor/`：运行中的机器有什么、这座城要什么，以及 CLI 自己的样子（`bin::doctor`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-146 Windows 上构建这个二进制要有 Zig（`bin::doctor::table::toolchain` 的 `zig` 一行）

`sprawling` 按路径链接 `crates/desktop/`，`crates/desktop/` 在 Windows 上链接它的 FFI 缝 `crates/desktop/ffi`，而那个包的构建脚本用 `zig build-lib` 编一片 Zig 叶子（`crates/desktop/Spec.lean` §8-12）。所以 Windows 上编这个二进制、跑 `just check` 都要一个 Zig，且是 `crates/desktop/ffi/zig-version` 钉住的那一版。

```rust
// bin::doctor::table::toolchain（形状 6 数据）
pub(super) const ZIG: Requirement;   // develop 层，Required；探测 `zig version` 以钉子开头的一行
const ZIG_PIN: &str = crate::doctor::pin::ZIG_VERSION;   // 构建脚本读到的 crates/desktop/ffi/zig-version，去掉行尾（§8-157）
```

- **钉子只在一处**：`ZIG_PIN` 是构建脚本从 `crates/desktop/ffi/zig-version` 读进来的那一行，与 `LEAN_PIN` 读 `lean-toolchain` 同一种写法（§8-58、§8-157）；探测是 `zig version` 的输出以钉子开头，Windows 的装法是 `winget install --id zig.zig -e --version <钉子> --scope user`，macOS 是 `brew install zig`，Linux 印出官方下载页。构建脚本与 CI 的安装步骤读同一个文件。
- **`Required` 而不是 `Optional`**：Windows 上没有它，`just check` 编不出这个二进制；在别的平台上 Zig 叶子不编，有它也不多花什么，而 `Need` 不按平台分（§8-58），两害取其轻是把它列为必需。
- **位置**：表里 `lean` 之后、`uv` 之前：它与 Rust、Lean 同属编译这份代码要的工具，装法不依赖表里更早的任何一行。

**验收**：`prereqs.tsv` 与表渲染出的文本逐字相等（§8-58 的现有测试）；`zig` 一行的探测与装法读的是钉子文件里的那一版。
-/

/-!
## 8-166 运行中的机器有什么，这座城要什么（`bin::doctor`）

**原因**：README 说「别 `cargo install` 这个东西」，`just check` 要 `just` 与 `cargo-nextest`，客户端要一个版本与 crate 版本相等的 `wasm-bindgen` CLI，exec 工具的 python 臂要一个 CPython-WASI 组件，而这些要求今天散在四份文档里。一个人装不全的时候，得到的是某一条命令的失败信息，而不是一句「运行中的机器缺什么」。

```rust
// bin::doctor（形状 1 decision：表与判定；驱动只读写它拿到的那两个句柄）
pub(crate) enum Tier { Use, Develop }                  // 两层：用得起来，改得动
pub(crate) enum Need { Required, OneOf(Group), Optional }   // 一组任一即可，见 §8-57
pub enum Platform { Windows, MacOs, Linux }            // `bin::install` 经它进入，遵 lib.rs 的约定：二进制进入即公开
pub(crate) struct PerPlatform<T> { windows: T, macos: T, linux: T }
pub(crate) enum Detection { Program { program, version_arg, places }, Environment { variable } }
pub(crate) struct Requirement { name, tier, need, enables, detect, homepage, recipe }   // recipe: PerPlatform<accounting::Recipe>（`crates/accounting/Spec.lean` §8-4）
pub(crate) enum Presence { Present(String), Absent }
pub(crate) struct Finding { requirement: &'static Requirement, presence: Presence }
pub(crate) fn examine(machine: &dyn Machine) -> Vec<Finding>;
pub(crate) fn finding_line(finding: &Finding) -> String;      // present <v> | absent | optional-absent
pub(crate) fn verdict(findings: &[Finding], tier: Tier) -> Verdict;
pub(crate) fn verdict_line(tier: Tier, verdict: &Verdict) -> String;

// bin::doctor::table（形状 6 data：编辑它就是编辑行为，无分支）
pub(crate) const REQUIREMENTS: &[Requirement];
pub(crate) const WASM_BINDGEN_VERSION: &str;                  // 与工作区 Cargo.toml 的钉子相等，由测试守住

// bin::doctor::screen（形状 4 adapter：屏幕上的那份报告与那一问）
pub(crate) struct Asked { install: bool }
pub(crate) fn asked(args: &[String]) -> Asked;
pub(crate) fn run<R: BufRead, W: Write>(asked: &Asked, machine: &dyn Machine, input: &mut R, out: &mut W) -> io::Result<bool>;
pub(crate) fn verb(args: &[String]) -> ExitCode;              // 必需项有缺则退 1

// bin::doctor::probe（形状 4 adapter：运行中的机器）
pub(crate) trait Machine: accounting::Machine { fn look(&self, r: &Requirement) -> Presence; }   // 安装经父 trait 的 install（`crates/accounting/Spec.lean` §8-4）
pub(crate) trait Machine { fn look(&self, r: &Requirement) -> Presence; fn install(&self, name: &str, recipe: &Recipe) -> Result<(), AxError>;
                           fn core_standing(&self) -> Result<Standing, AxError>; } // §8-93 的档位，由平台实际给出
pub(crate) struct ThisMachine { platform: Option<Platform>, patience: Duration }
pub(crate) fn answer(machine: &dyn Machine) -> wire::DoctorAnswer;   // bin::doctor::report：一页答案，ThisMachine 的 report 就是它
pub(super) fn names_of(program: &str) -> Vec<String>;         // Windows 上 .exe／.cmd／.bat 在先，无扩展名在后
```

- **两层而不是一张清单**：一个只想让这座城跑起来的人与一个要改这份代码的人，缺的不是同一批东西。把 Firefox 与 `cargo-nextest` 摆进同一张「缺失」清单，等于告诉前者他缺一个他永远不会用的测试跑器——**判定因此按层给两句话**（`ready to use`／`ready to develop`），而不是一句总分。
- **逐项征求同意，而不是一次总同意**：`--install` 对每一个缺项先印出**运行中的机器上要跑的那条命令**，再在 stdin 上问 `y/N`，默认是 N。一次总同意会让人对一串他没读过的命令点头，而这些命令改的是他自己的机器。**没被问到的东西恒不安装**。
- **恒不提权**：这里的每条命令都是用户级的（`winget`／`brew`／`cargo install`／`rustup`），`sudo`／`apt` 那一支落在 `Recipe::Print`，人自己贴。一个默认会请求管理员权限的 doctor，是把「检查」变成了「让我动你的系统」，与 §8-9 的 install 同一条理由：**只碰这个人 profile 里的东西**。
- **curl 脚本只印不跑（被否决的备选：`curl | sh` 自动安装）**：bun 在没有包管理器的平台上的官方装法是把一段脚本管进 shell。跑它意味着这座城代替人接受了一份它没读过、也无法在此刻校验的远端代码——**被否决**。那一支是 `Recipe::Print`：命令印在屏幕上，人自己决定。
- **探测是「在不在 PATH 上」加「`--version` 说什么」，且带时限**：一个装坏了的工具会挂在启动上，而 doctor 挂住等于比不装还糟。子进程的读法沿用 `agent_protocols::mcp::stdio` 的形状——读在一个线程里，等在一个带 deadline 的 channel 上，超时就杀掉子进程；`patience` 是参数，**不在这里采时钟**。浏览器另加平台标准安装路径与 Windows 的两个注册表键，因为 Windows 与 macOS 上它常常不在 PATH 上（§8-57）；**浏览器的版本不问它本人**，读它旁边的文件（§8-80），于是这条带时限的子进程路径只剩驱动与命令行工具在走。
- **四个文件而不是两个，理由是尺寸与形状**：`doctor.rs` 只留判定（表的形状、`finding_line`、`verdict`），表落 `table.rs`，屏幕与那一问落 `screen.rs`，跑子进程的落 `probe.rs`。判定与驱动同住一个文件时 `doctor.rs` 是 399 行——`xtask length` 的 400 之下一行，即下一次编辑必红。**这不是把文件切碎，是把「判断」与「跟人说话」分开**，两者本就不是一件事。
- **Windows 上先找带扩展名的那个文件**：`bun` 若由 npm 装出来，同一目录下既有无扩展名的 shell 脚本 `bun`（Windows 起不动）又有 `bun.cmd`。先取无扩展名的那个，报出来的是「装了但不说版本」——一个装好的工具被报成半坏的。故 `names_of` 在 Windows 上按 `.exe`／`.cmd`／`.bat`／无扩展名的次序找，这条次序有它自己的测试。
- **一项是环境变量而不是程序**：exec 工具 python 臂要的 CPython-WASI 组件由 `PYTHON_WASM_ENV` 指路（`accounting::worker::workbench::tools`），故它的探测是「那个变量指的文件在不在」，安装那一栏是 `Manual`——没有包管理器发它。它是 `Optional`，行尾说明它开启的是什么。
- **终端里的词是英文，这不违反 wording 门**：AGENTS.md 的语言表把词表的管辖写在客户端上，`xtask wording` 扫的目录是 `client/src`（见 `tools/xtask/src/wording.rs` 的 `CLIENT` 常量）。控制台是操作者的，与 `install`／`console`／`firstrun` 同一口径。
- **机器面是一条缝，而不是一个假想缝**：`Machine` 有两个实现——`ThisMachine`（真跑子进程）与测试里的 `ScriptedMachine`（一张 name→Presence 的表，外加它经 `accounting::Machine::install` 记下的安装请求）。终端与 worker 的安装走同一个 `accounting::Machine::install`，所以测试记下的就是终端真正会启动的那一次。判定因此不需要测试机上真装着什么就能被咬。
- **核心线程实际站在哪一档（§8-93）**：报告在各层的判定之后多一段 `priority`，一行 `core threads`：`one step above normal`；`normal, as config.toml [core] priority asks`；`normal, the platform refused: <原因>`（Unix 上没有 `CAP_SYS_NICE`）；读不出设置、或问档位的那条临时线程起不来或没答话就结束时是 `unknown: <错误>`（线程 panic 时只报它没答话，不带 panic 的内容）。`ThisMachine` 读人的设置，在一条临时线程上调 `raise_this_thread` 得到这一档，线程随即结束，所以 doctor 自己的线程不换档。报的是运行 doctor 的主机此刻会给核心的档位，而不是某座正在跑的城的线程被安全阀降回之后的档位。派出的命令总是低一档（`crates/runtime/Spec.lean` §8-13-3，降档从不被拒），这一段不重复它。证据：`crates/sprawling/src/doctor/tests/reading.rs` 的 `the_doctor_says_where_the_platform_lets_the_core_stand`（平台拒绝升档的机器，报告里是那一行与平台给的原因）。页面读同一读数：`report::fold` 从同一个 `Machine::core_standing` 折出 `DoctorAnswer::core`（`crates/wire/Spec.lean` §8-25），词由页面选；证据是 `doctor::report::tests` 的 `the_page_is_told_where_the_core_stands`。
- **机器面是一条缝，而不是一个假想缝**：`Machine` 有两个实现——`ThisMachine`（真跑子进程）与测试里的 `ScriptedMachine`（一张 name→Presence 的表，外加它记下的安装请求）。判定因此不需要测试机上真装着什么就能被咬。

**本章测试**：表的完整性（每一项都有探测方法，且三个平台各自要么给出命令要么明说 `manual`；每一个 `Optional` 项都说出它开启什么）；`verdict` 对「全在」「缺一个必需项」「只缺一个可选项」三类输入给出正确的穷尽枚举（可选项缺失不拖垮该层）；`finding_line` 的三种写法；`--install` 在答 `n` 时**一件也不装**、答 `y` 时只装被问的那一件（由 `ScriptedMachine` 记账）。

**本章验收**：`cargo run -p sprawling -- doctor`，输出逐项与两句判定，必需项有缺则退 1。
-/

/-!
### 8-166 之一 城目录前的实时扫描（`bin::doctor::scanning`、`bin::doctor::asking`，形状 4 adapter）

**原因**：一座城的目录承受大量小写入——Ledger 的追加、楼的 worktree、构建产物。Windows 上 Defender 的实时扫描在每次写入落下之前同步检查，这是小写入密集时变慢的已知来源。两种办法让扫描不挡在写入前面：把城放在受信任的 Dev Drive 上（Defender 对它改用异步的 performance mode），或把城目录加进 Defender 的排除项。doctor 是「运行中的机器有什么」的唯一权威（§8-47），所以由它说出城目录处在哪一种情况。

```rust
// bin::doctor::scanning（平台调用 + 纯解析 + 终端的词）
pub(crate) enum Scanning { DoesNotApply, Stopped, Read { city: PathBuf, drive: Drive, exclusion: Exclusion } }
pub(crate) enum Drive { Trusted, Untrusted { volume: String }, Not { volume: String, file_system: String }, Untold(Untold) }
pub(crate) enum Exclusion { Inside { under: String }, Outside, Untold(Untold) }
pub(crate) enum Untold {
    NoDisk, AdminOnly,
    Unread { command: &'static str }, Failed { command: &'static str, code: Option<i32> },
    Unstarted { command: &'static str }, Unanswered { command: &'static str, stopping: Option<String> },
}
pub(crate) fn read(platform: Option<Platform>, city: &Path) -> Scanning;            // ThisMachine::scanning 就是它
pub(super) fn drive_said(volume: &str, file_system: &str, ended: &Ended) -> Drive;   // 纯函数
pub(super) fn exclusion_said(city: &Path, ended: &Ended) -> Exclusion;               // 纯函数
pub(crate) fn lines(scanning: &Scanning) -> Vec<String>;                            // 终端的那一段
// bin::doctor::asking（问一个程序一次：计数的等待，读退出码与标准输出）
pub(super) enum Ended { Exited { code: Option<i32>, stdout: String }, Unstarted, Unanswered { stopping: Option<String> } }
pub(super) fn ask(command: &mut Command, knocks: u32) -> Ended;
// bin::doctor::probe：Machine::scanning(&self, city: &Path) -> Scanning
// bin::doctor::screen：Asked.scanned，命令行点名的城，没点名时是 `up` 会放城的那个目录
```

- **三个平台**：Windows 读下面两件事；macOS 与 Linux 答 `Scanning::DoesNotApply`，终端只有一行 `scanning  does not apply: Dev Drive and Defender's exclusions are Windows features`，不起任何进程。
- **判断的是哪个目录**：命令行点名了城就是那座城；没点名就是 `up` 会把城放在的地方，由路由的 `default_city_location` 交进 `screen::verb`——「没点名的城放哪」只有那一个权威。只在没点名时才调它，因为它要在二进制旁边试写一个空文件。路径先按 `monitor::volume::resolved` 规范成挂载点的写法，城还不存在时用它的绝对路径。
- **Dev Drive 分两步读，先读不要权限的那一步**：`monitor::volume::holding` 选出挂载点是城路径最长前缀的那块盘（与 `monitor::volume::space` 同一个函数），sysinfo 给出它的文件系统名。Dev Drive 恒是 ReFS，所以不是 ReFS 就确定不是，不再起进程。是 ReFS 时才问 `fsutil devdrv query <卷>`，取以 `This is` 开头的那一句：含 `not a developer volume` 是不是；含 `trusted` 且不含 `untrusted`／`not trusted` 是受信任；其余提到 `developer volume` 的是不受信任。fsutil 在普通用户下多半拒绝打开卷，且它的话随系统语言本地化——两种都读成 `Untold`，带上那条命令，不猜。
- **排除项只在不要管理员就能读的时候读**：Windows PowerShell 以 `-NoProfile -NonInteractive` 起动，先把输出编码设为 UTF-8、`$ErrorActionPreference='Stop'`，再取 `(Get-MpPreference).ExclusionPath`。每行一条排除路径；以 `N/A` 开头的那行读成 `Untold::AdminOnly`；空输出且退出 0 是没有排除项；退出非 0（Defender 关了或被第三方杀软替换）读成 `Failed`。一条排除路径覆盖城，当且仅当去掉末尾分隔符后不分大小写相等，或它加上 `\` 是城路径的前缀——`D:\Work` 不覆盖 `D:\Workshop\city`。含 `%` 或 `*` 的条目不展开、不算覆盖：展开要的是 Defender 自己的规则，第二份抄本会悄悄分叉。
- **等待是计数的，有上限，且从不等人**：`asking::ask` 把 stdin 设成空、stderr 丢弃，标准输出在一条读线程上读到底（Windows 的匿名管道缓冲只有几 KB，边轮询边不读会把子进程卡在写上），按 `TICK`（50 ms）敲 `try_wait`，敲满 `knocks` 下还没退出就经 `running::stop` 停掉。本节的上限是 `PATIENCE` 300 下（15 s）：冷起动的 Windows PowerShell 读 Defender 设置要几秒。`doctor::github` 问 `gh` 走同一个 `ask`，两处「问一次、读退出码」只有一份等法。
- **这一段是建议，从不是 doctor 的失败**：它不进任何一层的 verdict，不改退出码。报告在 `priority` 一段之后多一段 `scanning`：`city` 一行给出判断的路径，`dev drive` 与 `exclusion` 各一行说是、否或 `cannot tell: <原因>`；两者都不成立时给 `to speed it up`：建 Dev Drive 并把城挪上去，或在管理员 PowerShell 里跑 `Add-MpPreference -ExclusionPath '<城>'`（路径里的 `'` 写成 `''`）；是 Dev Drive 但不受信任时给 `fsutil devdrv trust <卷>`。
- **与各项探测同时问**：`screen::run` 在一个作用域线程上问 `Machine::scanning`，同时 `examine_each` 问表里各项，整份报告仍只等最慢的那一个（§8-59）。线程没答话就结束时这一段说 `Scanning::Stopped`，其余各段照常。
- **页面不显示这一段**：`wire::DoctorAnswer` 没有它的字段，加字段是一次 wire 变更（`WIRE_V`、`wire.ts`、`Door.lean`）。第三方杀毒软件的排除项读不到，那时 `exclusion` 说 `cannot tell`。

**测试**：`doctor::scanning::tests` 的 `fsutil_refusing_an_ordinary_user_reads_as_cannot_tell`、`the_statement_fsutil_prints_says_whether_the_volume_is_a_dev_drive`、`defender_hiding_its_exclusions_reads_as_cannot_tell`、`an_exclusion_covers_the_city_only_at_a_directory_boundary`、`the_advice_names_the_command_that_would_exclude_the_city`；`doctor::tests::reading` 的 `slow_scanning_is_advice_and_never_a_failure`（D33）。

**验收**：Windows 上 `cargo run -p sprawling -- doctor`，`scanning` 一段说出城目录所在卷的文件系统与排除项能否读到；`cargo nextest run -p sprawling -E 'test(scanning) | test(slow_scanning)'`。
-/

/-!
## 8-47 六处探测收成一处：doctor 是「运行中的机器有什么」的唯一权威（`bin::doctor::host`、`bin::doctor::presence`）

**原因**：运行中的机器被问了六次，每次一套读法——`SPRAWLING_PYTHON_WASM` 在 `accounting::worker::mcp` 与 `doctor::table` 各拼一次（§8-166 记下的债）；`host_shell()` 与 `execution_engine()` 住 `accounting::worker::workbench::engine`；Firefox 与 `chromedriver` 由 `bin::browser_bidi::lazy` 按名字盲起（Windows 上 Firefox 不在 PATH，于是 doctor 说「有」而浏览器工具说「没有」）；`ffmpeg` 在 `crates/desktop/` 里按名字起。六个答案各自漂，一个人看到的「缺什么」与 run 撞上的「缺什么」不是同一份。

**主机事实住机器层**（沿 `crates/kernel/Spec.lean` §8-22「主机事实不入城」）。doctor 装的东西落 `~/.sprawling/components/<item>/`，**恒不落进任何一座城**——一座城搬到另一台机器时不该带着运行中的机器的组件。城里 `CONFIG.toml` 仍只说能力位（`sandbox.shell`）与限额，不说路径。

```rust
// lib.rs：doctor 从二进制半边搬进 lib，装配层与浏览器层才够得着它
pub mod doctor;                                   // 公开面唯一新增：`pub use screen::verb`

// bin::doctor::presence（形状 2 value）：运行中的机器对一项东西的回答，三态而非两态
pub(crate) enum Presence {
    Present { at: PathBuf, version: Version },     // 在，且起得来
    Broken  { at: PathBuf, fault: Fault },         // 在，但用不了——对判定等于缺，对人必须说出为什么
    Absent(Absence),                               // 不在，且说出是哪一种不在
}
pub(crate) enum Version { Said(String), Silent, Unreadable, Late }
pub(crate) enum Fault   { WillNotStart(String), HalfWritten, Unreadable(String) }
pub(crate) enum Absence { NotOnSearchPath, VariableNamesNothing { variable, path },
                          NoComponent { dir }, NoHome, NotInThisBuild }
impl Presence { fn usable(&self) -> bool; fn at(&self) -> Option<&Path>; fn describe(&self) -> String; }

// bin::doctor（Detection 增三臂；表因此能说出四种「怎么找」）
pub(crate) enum Detection {
    Program     { program, version_arg, places },
    Component   { variable, file },                // 变量优先；否则 ~/.sprawling/components/<name>/<file>
    Interpreter { variable: PerPlatform<&str>, fallback: PerPlatform<&str> },   // COMSPEC／SHELL
    Built       { carried: bool },                 // 这份构建带不带（sandbox 引擎）
}

// bin::doctor::host（形状 4 adapter）：二进制里其他模块问运行中的机器的那一扇门
pub(crate) fn firefox() -> Presence;        // Gecko 族里运行中的机器有的那个牌子（§8-57）
pub(crate) fn chromedriver() -> Presence;   // chromedriver 或 msedgedriver，先答上来的那个
pub(crate) fn python_wasm() -> Presence;
pub(crate) fn shell() -> Presence;
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError>;   // 从 workbench::engine 搬来
pub(crate) const ENGINE_CARRIED: bool;                                             // cfg!(feature = "sandbox") 的唯一拼写
pub(crate) fn components_dir() -> Option<PathBuf>;                                 // ~/.sprawling/components

// accounting::worker::workbench::engine（仍是 adapter，但不再自己探测）
pub(super) struct MachineHalf { python_wasm: Option<PathBuf>, shell: Option<PathBuf>, engine: Box<dyn Sandbox> }
pub(super) fn machine_half(limits: &kernel::SandboxLimits) -> Result<MachineHalf, AxError>;

// bin::browser_bidi::engine（decision）：吃 doctor 的三态答案，拒绝时说出是哪一种
pub(crate) fn Engine::choose(firefox: &Presence, chromedriver: &Presence) -> Result<Engine, AxError>;
```

- **谁问谁**：`workbench::engine::machine_half` 问 `doctor::host` 三次（组件、shell、引擎），它自己只留一条判定——shell 只在冻结配置说 `shell = true` 时才递给 bench，组件缺席不拦派活（python 臂在被调用时才拒），引擎起不来则拒派活。`browser_bidi::lazy::start` 不再按名字盲起，改为 `Engine::choose(&host::firefox(), &host::chromedriver())` 后按路径起。`accounting::worker::mcp::PYTHON_WASM_ENV` 删除。
- **`SPRAWLING_PYTHON_WASM` 只拼一次，且保留为兼容读法**：拼写唯一处是 `doctor::table::PYTHON_WASM_VARIABLE`。**选的是「变量优先、组件目录次之」**：一个人显式指了一处，就该用那一处；指错了（变量设了但文件不在）报 `Absent(VariableNamesNothing)` 而**不悄悄落到组件目录**——被否决的备选是「目录优先、变量兜底」，它会让一个设错的变量永远没人发现。没设变量时看 `~/.sprawling/components/python-wasi/python.wasm`。测试遍历本 crate 的 `src/`，断言含该字面量的文件恰好一个。
- **`Broken` 是第三态，不是 `Absent` 的别名**：一个在 PATH 上却起不来的二进制（权限、坏文件、架构不符）、一个存在却没有那份文件的组件目录（下载中断）、一个读不了的目录（权限），三者对 verdict 都算缺，但每一个都带着自己的原因进报告行——**绝不以「absent」一词吞掉一个可以说清的故障**。`Version` 的四态同理：说了、没说、说的不是文本、超时没说；后三者仍算 Present（§8-166 已定：不说话的工具仍是装了的工具）。
- **本二进制起的每个子进程都由 `doctor::running::stop` 结束**：`ask_version` 读到第一行后杀掉子进程，用的是安装程序超时后走的同一段——杀不掉或收不了尸都不是可以丢掉的 `Result`，而是一句带进 `Fault::Unreadable` 的话，于是「本城起了一个它停不掉的进程」这件事排在它印出的版本号之前给人看。`Fault::Unreadable` 因此是「这台电脑不让本城把这一项做完」的那一态，它携带的那句话就是全部解释，`describe` 原样印出。
- **`Detection::Built` 的探测是真起一次引擎**，而不是读一个 cfg：一份声称带引擎却起不来的构建，doctor 必须报 `Broken { WillNotStart }`；`ENGINE_CARRIED` 是那个 cfg 的唯一拼写，表引用它。
- **`ffmpeg` 进表但 doctor 管不到 `crates/desktop/`**：`crates/desktop/` 跑在 `sprawling desktop` 这个子进程里，它在录制时按名字起 `ffmpeg`，与 doctor 的 `on_search_path` 走同一条 PATH，两个答案因此一致而非因此合一。doctor 报它（Optional，Use 层），`crates/desktop/` 不改——这是这里的边界，如实记。桌面 server 本身不进表：它是本二进制的一个动词（§8-4d），没有要装的东西。
- **`browser::profile` 没有探测可搬**：读 `crates/browser/Spec.lean` 的 D4 确认 profile 是「楼的登录态住城的保留区」这条纯判定，浏览器探测住 `bin::browser_bidi::lazy`，故不改 `crates/browser`。
- **doctor 进 lib 的公开面只多一行**：`pub use screen::verb`，二进制半边 `main/router.rs` 改调 `sprawling::doctor::verb`；`Machine` 仍是 `pub(crate) trait`，不上缝清单。

**探测可失败的路径，逐条**（本节与 §8-48 共用，测试点名「丑的那几条」）：

| 路径 | 答案 | 报告行 |
|---|---|---|
| 程序在 PATH 上却起不来 | `Broken { WillNotStart(err) }` | `firefox  broken at <path>: will not start: <err>` |
| 起来了但一行也不说／说的不是文本／超时 | `Present { version: Silent／Unreadable／Late }` | `present <path> (said nothing／unreadable version／no version within the deadline)` |
| 变量设了却指向不存在的文件 | `Absent(VariableNamesNothing)` | `absent: SPRAWLING_PYTHON_WASM names <path>, which is not there` |
| 组件目录在、文件不在（半写） | `Broken { HalfWritten }` | `broken at <dir>: half-written; delete it and install again` |
| 组件目录或文件读不了 | `Broken { Unreadable(err) }` | `broken at <dir>: <err>` |
| 找不到 home | `Absent(NoHome)` | `absent: neither USERPROFILE nor HOME is set` |
| 构建带引擎却起不来 | `Broken { WillNotStart }` | `sandbox-engine  broken at <exe>: will not start: <err>` |
| `--install` 的网络超时 | 包管理器自己的退出码 → `E_TOOL_UNAVAILABLE`，recovery「run the printed line yourself」 | doctor 自己不上网、不重试、不静默 |

**本章测试**：`the_python_variable_is_spelled_once`（红：今天两处）；`a_program_that_will_not_start_is_broken_not_absent`（临时目录里放一个不是可执行文件的 `broken.exe`／`broken`）；`a_variable_that_names_nothing_is_said_so`；`a_half_written_component_directory_is_broken`；`a_present_tool_that_says_nothing_is_still_present`；`a_broken_firefox_is_refused_by_its_fault_not_as_absent`（`Engine::choose`）。

**本章验收**：`cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings`、`cargo nextest run -p sprawling --locked --all-features` 绿；`cargo xtask modmap`／`length`／`header`／`specalign` 绿；`cargo run -p sprawling -- doctor` 仍报 Firefox 那一行。
-/

/-!
## 8-48 doctor 按城回答，按错误码解释（`bin::doctor::needs`、`bin::doctor::visit`、`bin::doctor::explain`）

**原因**：§8-166 的 doctor 回答的是「运行中的机器对这个仓库」，而一个人真正的问题是「我这座城跑得起来吗」：一栋写了 `browser = true` 的楼在没有 Firefox 的机器上，今天要等到 run 撞上 `E_BROWSER_UNAVAILABLE` 才知道。而那条拒绝的 `recovery` 是一句通用话，没有接到运行中的机器的事实上。

```rust
// bin::doctor::needs（形状 1 decision）：一栋楼的能力位要什么，运行中的机器给不给
pub(crate) struct Bits { pub browser: bool, pub shell: bool }
pub(crate) enum Capability { Browser, Shell }
impl Capability { pub(crate) fn any_of(self) -> &'static [&'static str]; pub(crate) fn as_str(self) -> &'static str; }
pub(crate) struct Lack { building: Address, capability: Capability, tried: Vec<(&'static str, Presence)> }
pub(crate) fn lacks(building: &Address, bits: &Bits, findings: &[Finding]) -> Vec<Lack>;
pub(crate) fn lack_line(lack: &Lack) -> String;

// bin::doctor::visit（形状 4 adapter）：走一遍城里的楼，读每一栋的位
pub(crate) enum Visited { Bits { building: Address, bits: Bits }, Unreadable { building: Address, err: AxError } }
pub(crate) fn visit(city_root: &Path) -> Result<Vec<Visited>, AxError>;

// bin::doctor::explain（形状 1 decision）：一个错误码接到运行中的机器
pub(crate) enum Explanation { NoSuchCode(String), NotAboutThisMachine(AxCode), Lines(Vec<String>) }
pub(crate) fn explain(code: &str, findings: &[Finding], platform: Option<Platform>) -> Explanation;

// bin::doctor::screen：`doctor [<city>] [--install] [--explain <code>]`
pub(crate) struct Asked { install: bool, city: Option<PathBuf>, explain: Option<String> }
```

- **能力位 → 项目，是一张穷尽表**：`Browser → [gecko, chromedriver, msedgedriver, webkit]`（任一即可，Gecko 在前，因为它不要驱动；见 §8-57）；`Shell → [shell]`。`browser` 读自 `RULES.toml`（`city::load`），`shell` 读自该楼冻结配置的 `sandbox.shell`（`city::load_config`）——两者合称「RULES.toml 的能力位」，实际住两份文件，这里如实记。`desktop` 不是能力位：它要的东西都在本二进制里（§8-4d）。一栋楼的一个位缺时，报告行点名**那栋楼**与它试过的每一项及各自的三态答案：`lab: browser: true, and this machine has no firefox (not on the search path) and no chromedriver (not on the search path)`。
- **读不了的楼是一行，不是沉默**：`RULES.toml` 解析失败或 `CONFIG.toml` 无效，那一栋报 `Visited::Unreadable`，屏幕上是 `lab: its rules will not read: <err>`；楼列表本身读不到（不是城）才是 `Err`。**doctor 永不静默**。
- **`--explain <code>` 是「错误码 → 主机项目」的一张表**：`E_TOOL_UNAVAILABLE → [sandbox-engine, python-wasi, shell, ffmpeg]`，`E_BROWSER_UNAVAILABLE → [gecko, chromium, chromedriver, msedgedriver, webkit]`。其它已知码答 `NotAboutThisMachine`（它由城里的判定决定，不由运行中的机器决定）；不认识的码答 `NoSuchCode`。每一行是**那一项的三态答案加这平台上的下一步**：`python-wasi  absent: no component at ~/.sprawling/components/python-wasi/python.wasm -> manual: put a CPython wasi build there`——一个人读完那一行就能动手。
- **边界**：doctor 不从源码构建、不 vendor、不静默。它探测一切，只安装有官方可验证来源的东西，并逐项先问。CPython-WASI 今天没有 python.org 发布的二进制，故它仍是 `Manual`，指向组件目录；这里不下载任何东西。`~/.sprawling/components/` 因此暂时只是 doctor 探测、人填入的约定——记在这里，免得下一步以为那里有个下载器。
- **退出码**：`doctor <city>` 在该城任一楼缺任一位时退 1，与 §8-166 的「必需项有缺退 1」同一口径；`--explain` 退 0（它是解释，不是判定），只有码本身不存在时退 1——一个拼错的码是一次问错，脚本该知道。

**本章测试**：`a_building_that_asks_for_a_browser_is_named_when_this_machine_has_none`（红：`lacks` 不存在）；`a_building_whose_rules_will_not_read_is_reported_not_skipped`（`visit` 在真目录上）；`explain_connects_a_refusal_code_to_what_this_machine_has`（`E_TOOL_UNAVAILABLE` 给出一行每项、含 recipe；未知码与无关码各自的答案）；`doctor_with_a_city_names_the_building_on_the_screen`（`run` 经 `ScriptedMachine`）。

**本章验收**：同 §8-47；另加 `cargo run -p sprawling -- doctor --explain E_TOOL_UNAVAILABLE` 一行一项。
-/

/-!
## 8-54 运行中的机器有什么，页面从城那里问（`bin::doctor::report`、`accounting::views::holding`；`crates/wire/Spec.lean` §8-25）

首跑屏的第一步原本只是一条可以复制的命令，没有任何办法知道它跑过没有、跑成了没有。本节让那一步答得出来。

- **`bin::doctor::report`**：`answer(machine: &dyn Machine) -> wire::DoctorAnswer` 问一次交给它的机器并折成答案。机器是参数，所以一个测试交一台假机器，就说出机器答了什么而不必有那样一台机器；生产的调用方是 `ThisMachine` 实现的 `accounting::Machine::report`。**它不判断任何事**——哪一项在这里、一个档次缺什么，权威在 `doctor` 与 `table`；这里只换一种说法。`screen` 把同一批 findings 折成一台机器的散文，两者从同一处折出。
- **`Views.machine: Option<wire::DoctorAnswer>`**，由 `found_on_this_machine` 从外面放进来，**不由任何记录折出**：这是本文件里唯一一个关于机器而非关于历史的答案，所以重建账本不碰它。`None` 答 `Unavailable`。
- **`Views.registry: Option<fn() -> wire::ReleaseAnswer>`**，由 `views::served` 的 `ask_the_registry_through` 从外面放进来，与 `machine`、`vault` 同形：serve 一座城时装配根交 `bin::release::answer`，`twin` 把它带到另一份。`NewestRelease` 在快照放开之后调它（它要出网）；`None` 是一份没人 serve 的 views（重建、测试），答 `Unavailable` 而不去问注册表。views 因此不直接碰 `bin::release`，搬进 `accounting` 时 `release` 留在 `sprawling`（`crates/accounting/Spec.lean` §7）。钉住它的测试是 `a_newest_release_is_asked_of_the_registry_the_views_were_handed`。
- **`Views.programs: Option<HarnessReach>`**（`HarnessReach { find, place }`，两个函数指针），由 `look_for_harnesses_through` 从外面放进来，与 `registry` 同形：serve 一座城时装配根交 `bin::doctor::host::find_program` 与 `bin::doctor::host::place_set_up`（后者读这家厂商文档写的变量与家目录，`crates/wire/spec/Answer/Harnesses.lean` D23）。`Query::Harnesses` 在快照放开之后调它；`None` 答 `Unavailable`（`crates/accounting/Spec.lean` §8-10、accounting D13）。
- **探测只由 `DoctorRefresh` 触发，服务一座城时一次也不跑**：表从 12 行长到 32 行，其中大半是起一个进程问它的版本（六件 cargo 子命令各起一次 cargo），windows-x86_64 暖缓存四核一档机器上量得 3.3–4.1 s。先前的决定把它放在开门之前，给出的参数是「12 项约 2 秒」，**两个数都已经移动**：项数翻了一倍有余，而问它的那一屏不再是第一屏（`#/` 是对话，机器那一屏在设置页的「依赖项安装」组里）。它当时否决后台探测的理由是「要多一条『还没答上来』的状态」，而那条状态今天已经存在、有夹具、也有它的动作（`MachineSkeleton` 与 `MachineUnchecked`）——那笔代价早已付过。服务因此不再为一个没人问的答案把套接字关着几秒。
- **客户端**：`client/src/views/machine.svelte` 画一份答案（`MachineReport`）与问一次（`Machine`）；首跑屏第一步换成它。**那一屏每次打开都发一次 `DoctorRefresh`**（与「重新检查」同一条命令，不是第二条路），每次打开至多一次；城里已有的答案先画出来，探完的那一份到了再换上（§8-120）。每一行是「状态词 + 名字 + 版本或装它的命令」，状态词取自 `lang.json`，版本与命令是城给的值——页面上没有句子。`#/gallery` 有一份夹具，三行各处于人会采取不同行动的三种状态。
- **不因事件失效**：这份答案说的是城启动时看到的那一眼，账本上没有任何记录能改变它，所以 `asking` 的 `staleBy` 对它落在 `default`（不失效）。
- **验收**：`doctor::report::tests`——没有任何浏览器引擎的假机器答出 `Absent { NotOnSearchPath }`、`use` 档 `missing == ["a browser engine"]` 而 `develop` 档为空；平台不明时每一项的 `install` 都是 `UnknownPlatform`。
-/

/-!
## 8-57 浏览器是一族引擎，不是一个牌子（`bin::doctor::family`、`family::gecko`／`chromium`／`webkit`、`bin::doctor::registry`）

**原因**：doctor 的 `firefox` 一行只认 PATH 上的 `firefox` 与两条固定路径，于是一台装了 Zen 的机器被判「运行层缺 firefox」，而 `browser_bidi::engine` 起浏览器只用 `--remote-debugging-port`、`-profile`、`--no-remote`、`-headless` 四个参数——**任何 Gecko 内核的浏览器都接受这四个参数**。表按牌子问，引擎按内核跑，两者本来就不是同一个问题；Chromium 一侧更反了一层：列的是 `chromedriver`，而人装的是浏览器。

```rust
// bin::doctor::family（形状 1 decision）：三族引擎，一族一行
pub(crate) enum Family { Gecko, Chromium, WebKit }
pub(crate) enum Confidence { Tried, NeedsConfirmation, Experimental }
pub(crate) struct Member { name, program, homepage, confidence, places: PerPlatform<&[&str]>, start_menu }
pub(crate) const BROWSER_VARIABLE: &str = "SPRAWLING_BROWSER";   // 本 crate 唯一拼写
pub(crate) const GECKO_ROW / CHROMIUM_ROW / WEBKIT_ROW: Requirement;
pub(crate) fn member_at(family: Family, at: &Path) -> Option<&'static Member>;
pub(super) fn look(family, platform, search_path) -> Presence;   // 版本读自文件，故无 patience（§8-80）

// bin::doctor（表因此能说出「任一即可」与「点进它自己的站」）
pub(crate) enum Need { Required, OneOf(Group), Optional }
pub(crate) enum Group { BrowserEngine }
pub(crate) enum Detection { …, Family(Family) }
pub(crate) struct Requirement { …, homepage: Option<&'static str>, … }

// bin::doctor::registry（形状 4 adapter）：Windows 记下的那两把钥匙
pub(super) fn installed_at(program: &str, start_menu: &str) -> Option<PathBuf>;
```

- **三族，而不是三个牌子**：Gecko（firefox、zen、librewolf、waterfox、floorp、firefox-developer、firefox-nightly、tor-browser）、Chromium（chrome、edge、brave、chromium、vivaldi）、WebKit（safari，经 `safaridriver`）。每个成员给出三平台的安装路径；一族的答案是**第一个答得上来的成员**。
- **驱动是另一行，不是这一行**：一个 Chromium 浏览器自己开不出会话，进得去的是它旁边那个主版本号相符的驱动。故 `chromium` 一行是 `Optional`（它回答的是「运行中的机器上有哪个 Chromium」），而 `chromedriver` 与 `msedgedriver` 各自成行并进 `BrowserEngine` 组——这与 `Engine::choose` 吃的两件东西一一对上，**不制造第二份「怎样才算有浏览器」的权威**。被否决的备选：让 `chromium` 一行在找到浏览器却没有驱动时报 `Broken`，那要借用 `Fault::HalfWritten` 的措辞（「删掉重装」），而它对一个装好的浏览器是假话。
- **`OneOf(Group)` 而不是四行各自 `Required`**：四条进得去的路，三条关着一条开着，对一个人是「有一个浏览器引擎」而不是「缺三样」。`verdict` 因此把一组折成一个名字（`a browser engine`），排在逐项点名的那些之后；`paint::count` 用同一条规则数总结行，于是总结行与判定行**恒不互相矛盾**。
- **`Need::OneOf` 在线上说 `Required`**：`wire::DoctorNeed` 只有两个词，而页面要的是「它挡不挡路」；「这一组满足没有」由该档次的 `missing` 回答。第三个词会让页面必须同时读两处才能给一行上色，而 `DoctorNeed` 加一个变体会让今天在跑的客户端解不出整份答案。
- **`SPRAWLING_BROWSER` 只拼一次，且是引擎读的同一处**：变量压过一切探测。路径按文件名主干认族；**认不出的归 Gecko**，因为无驱动的那条起法是任何分支都接受的那一条，而 xtask render 正是这样设它。被否决的备选是「认不出就当没有」：那会对着一个人正看着的文件报「不在 PATH 上」。
- **Windows 的两把钥匙经 `reg.exe` 读，不引新依赖**：per-user 安装落在带账户名的目录里，表里的字面路径找不到它；`App Paths\<程序>.exe` 与 `StartMenuInternet\<牌子>\shell\open\command` 两个键说得出。只 `reg query`，不写；找不到就换下一个成员。被否决的备选是加一个 registry crate——一个平台、一个问题，换一份人要下载的二进制里的第三方代码。
- **`enables` 不再说 WebUI 要某个浏览器**：WebUI 任何浏览器都打得开，要 Gecko／Chromium／WebKit 的只有 browser tool。
- **`Tried` 之外的成员排在最后并带一句说明**：Tor Browser 的启动器与代理会挡在会话前面（`, confirm it by hand once`），Safari 的 BiDi 支持是局部的（`, experimental`）。`member_at` 把找到的路径认回牌子，于是报告说 `zen` 而不只是 `gecko`，`DoctorItem.homepage` 给的也是**那个牌子自己的站**。

**本章测试**：`every_family_is_a_row_and_every_member_says_where_it_is_installed`；`a_found_browser_is_reported_by_the_brand_it_is`（Developer Edition 的程序文件也叫 `firefox`，只有目录分得开）；`a_member_that_needs_confirming_is_never_the_first_answer`；`a_verdict_counts_the_required_items_of_its_own_tier_only` 的三段——只有 Zen、只有 Edge 加驱动、四条路全关。

**本章验收**：只装 Zen 的 Windows 机器上 `cargo run -p sprawling -- doctor` 报 `gecko present … (zen)` 且 `ready to use`。
-/

/-!
## 8-58 一次装齐开发这份代码要的全部工具（`bin::doctor::table::toolchain`、`just prereqs`）

**原因**：一个人在一台新机器上想开发 sprawling，要的是 Git、`rust-toolchain.toml` 钉住的 Rust 工具链连同 rustfmt 与 clippy、just、cargo-nextest、cargo-deny、bun、`lean-toolchain` 钉住的 Lean（经 elan）、Python（经 uv），以及 render 门要的那个 Chromium 系浏览器。这份清单若在 doctor 的表与 `just prereqs` 里各写一份，一份加了 Lean 另一份没加，照着其中一份装齐的人照样在另一份面前红。

```rust
// bin::doctor::table::toolchain（形状 6 data）
pub(super) const DEVELOP: [Requirement; N];   // Develop 档的全部行，按安装先后排列
const LEAN_PIN: &str;                         // doctor::pin::LEAN_TOOLCHAIN：构建脚本读到的 lean-toolchain，去掉行尾
// bin::doctor（Detection 多一支）
Detection::Listed { program, args, line }     // 程序在，且它按 args 列出的某一行以 line 开头才算在
// bin::doctor::table
pub(crate) fn prereqs() -> String;            // Develop 档渲染成 prereqs.tsv 的全文
```

- **表是权威，`just prereqs` 读它的渲染**：`crates/sprawling/src/doctor/table/prereqs.tsv` 每行 `class<TAB>name<TAB>program<TAB>windows<TAB>macos<TAB>linux<TAB>purpose`，由 `table::prereqs()` 从 Develop 档渲染；测试 `the_prereqs_file_is_the_develop_tier_rendered` 要求文件逐字等于渲染结果，不等时把应有的全文印出来。`just prereqs` 在编译之前跑，只能读文件而不能问二进制，所以读的是这份渲染而不是另一张清单；`command -v <program>` 是它的探测，`program` 为 `-` 的行（浏览器家族）归 doctor 与 render 门自己去找。被否决的备选：justfile 当权威、doctor 在编译期读它——justfile 不写三平台的装法，doctor 就得再拼一遍。
- **`class` 就是 `Need`**：`required` 是 `just check` 离了它跑不起来的（git、bash、rustup、rust、rustfmt、clippy、just、cargo-nextest、bun、render 用的浏览器、elan、lean、zig），`optional` 是 `just check` 缺了会跳过、或只由人主动跑的 recipe 调用的（cargo-deny、uv、python，以及 cargo-mutants、cargo-fuzz、kani）。elan 与 lean 为什么必需，见 D9「Lean 是开发这份代码必需的工具」。
- **行序就是安装顺序**：后一行的装法用到前一行装出的程序——rustup 之后才有 `rustup component add` 与 `cargo install`，elan 之后才有 `elan toolchain install`，uv 之后才有 `uv python install`。页面的「全部安装」按表序逐项跑，所以顺序写在表里而不是写在页面上。
- **Lean 的版本只写在 `lean-toolchain`**：`LEAN_PIN` 是构建脚本读进来的那个文件（`str::trim_ascii_end` 在 const 里去掉换行，§8-157），装法是 `elan toolchain install <pin>`，探测是 `elan toolchain list` 里有以 pin 开头的一行。换 Lean 版本只改那一个文件。
- **Windows 上能用 winget 的都用 winget，且按用户装**：Git、just、bun、uv、ffmpeg 的 winget 清单都有用户级安装程序，配方带 `--scope user`，于是装的时候不弹 UAC——§8-166「恒不提权」在 winget 上就是这个参数。rustup 的清单没有 scope 字段，而 rustup-init 本来就只写这个人的 profile，所以不带（带了 winget 答「找不到适用的安装程序」）；Chrome 的清单只有机器级安装程序，而每台 Windows 都有 Edge，Chromium 一族在 Windows 上由 Edge 答上，Chrome 那条配方很少被走到。
- **elan 在 Windows 上由城跑它的官方安装脚本**：winget 上没有 elan，官方装法是 `elan-init.ps1`。理由：一台新机器应当一次走完，而让人把一行 PowerShell 贴进终端正是走不完的那一步；这一条由人定下，是 §8-166「脚本只印不跑」在 Windows 的 elan 上的例外。配方是 `powershell -NoProfile -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((irm <elan-init.ps1>))) -NoPrompt 1 -DefaultToolchain none"`：`-NoPrompt` 让它不问，`-DefaultToolchain none` 让 Lean 的版本仍只由 `lean` 一行按 `lean-toolchain` 装。它仍是 `Recipe::Command`，页面仍要人按一下才跑，所以「每一条会改动计算机的命令都先给人看过」这条不变；Linux 上的 `curl … | sh` 仍是 `Print`，例外只到 Windows 的 elan 为止。
- **cargo 工具是一包，包里有哪些由仓库自己点名的地方决定**：`Requirement.pack == Some(Pack::RustTools)` 的行是 cargo-nextest、cargo-deny、cargo-mutants、cargo-fuzz、kani。测试 `the_rust_tools_pack_is_every_cargo_tool_this_repository_calls` 读 `justfile`、`.github/` 下的工作流、`tools/xtask/src/` 与 `docs/`，把其中调用的 cargo 子命令（`cargo <sub>` 里 `<sub>` 不是 cargo 自带的那些，也不是文档写给 person 安装 sprawling 本身的途径 `cargo binstall`——没有哪个 recipe、工作流或检查运行它，贡献者用不着）与 install-action 的 `tool:` 行收成一个集合，要求它恰好等于这一包；多一个或少一个都点名。cargo-audit 与 cargo-llvm-cov 因此不在表里：仓库里没有任何 recipe、工作流或文档调用它们。线上每一项仍是自己的一行（`just prereqs` 与判定逐项读），页面把同一包的行画成一行，一个按钮按表序装完缺的那几项，每一项一份日志。
- **Rust 本身是一行**：`rust` 探 `rustc --version`，配方 `rustup default stable`（rustup 装好后给出一个在 PATH 上的 rustc；进仓库后 rustup 按 `rust-toolchain.toml` 自取钉住的那一版）。它与 `lean` 两行带 `Pin`：页面把钉住的版本与装着的、上游最新的并排画出（§8-120）。
- **探测与安装子进程看到同一条搜索路径**（`doctor::host::search_path`）：进程的 `PATH` 之后补上这个人的几个用户级 bin 目录——home 下 `.cargo`、`.elan`、`.local`、`.bun` 各自的 `bin`，Windows 上再加 `%LOCALAPPDATA%\Microsoft\WinGet\Links`——已在 `PATH` 上的不重复。安装程序改的是注册表或 shell 启动文件，本进程的 `PATH` 是启动时那一份；不补这几个目录，刚装好的 rustup 让下一行的 `cargo install` 找不到 cargo，刚装好的 elan 让 `elan toolchain install` 找不到 elan，整批装要重启城才能走完。
- **`same_command` 一处拼写三平台**：三平台装法相同的工具只写一次，三列各抄一遍只会让其中一列悄悄落后。
-/

/-!
## 8-130 `just` 跑配方的那个 bash 是 develop 层的一行（`bin::doctor::table::toolchain` 的 `BASH`）

**原因**：`justfile` 开头是 `set shell := ["bash", "-uc"]`，每一个配方都在 bash 里跑，而 develop 层没有这一行，于是一台照着 `just prereqs` 装齐的新 Windows 机器仍然在 `just check` 的第一步就红。Windows 上还多一层：`just` 按搜索路径的次序找 `bash`，而系统的 `PATH` 排在这个人的 `PATH` 之前，`C:\Windows\System32` 又在系统那一段里；装了 WSL 的机器上，`System32\bash.exe` 是 WSL 的启动器，没有发行版时它打一句错误就退出。实测：同一台机器上，`PATH` 里 System32 在 Git 的 `bin` 之前时，`just` 起的是这个启动器，配方失败；Git 的 `bin` 在前时，起的是 Git 的 bash，配方通过。

```rust
// bin::doctor::table::toolchain（形状 6 data）
pub(super) const BASH: Requirement;   // Develop 档，Required，排在 GIT 之后
```

- **探测问的是 `just` 会拿到的那一个 bash**：`Detection::Listed { program: "bash", args: ["--version"], line: "GNU bash" }`。探测按 `doctor::host::search_path` 的次序找程序，与 `just` 找 `bash` 是同一个次序（进程自己的 `PATH` 在前），WSL 启动器不打印以 `GNU bash` 开头的行，于是被报为缺，而那正是 `just` 会撞上的情形。`prereqs.tsv` 里这一行渲染成 `bash --version | grep -q '^GNU bash'`。
- **Windows 的装法是一句话，不是一条命令**：`Recipe::Manual`，要人在 Git Bash 里跑 `just`。Git for Windows（git 那一行装的）带着 bash，Git Bash 的终端把它自己的 bash 放在搜索路径最前。能让别处起的 `just` 找到它的另一条路，是把 Git 的 `bin` 放到 System32 之前，那要改系统的 `PATH`，要管理员权限，而 §8-166 恒不提权。macOS 自带 bash，配方 `brew install bash`；Linux 是 `Print("sudo apt install bash")`。被否：在 `justfile` 里用 `set windows-shell` 写 Git bash 的绝对路径——按用户装的 Git 在 `%LOCALAPPDATA%\Programs\Git`，按机器装的在 `C:\Program Files\Git`，写哪一个都有一半机器找不到。
- **测试从 `justfile` 读出 shell，不从表里抄**：`doctor::tests::the_shell_every_recipe_runs_in_is_a_required_develop_row` 读 `set shell` 那一行里的程序名，要求 Develop 档有一行 `Required` 的探测程序就是它。`justfile` 换了 shell 而表没跟上，测试红。

**本节接口的当前状态**：MSVC 的链接器不是一行。`x86_64-pc-windows-msvc` 编译要 `link.exe` 与 Windows SDK，rustc 经 Visual Studio 的实例清单找它们，不经搜索路径，所以现有的四种探测（搜索路径、已知位置、程序的清单、本二进制自己）一种都问不到；装 Build Tools 的安装程序又要提权。它缺不缺，要在一台干净的 Windows 上读：`Get-Command bash, link.exe -ErrorAction SilentlyContinue`；`winget install --id Git.Git -e --scope user` 之后再读一次 `Get-Command bash`；`winget install --id Rustlang.Rustup -e` 之后 `cargo build` 能否找到链接器；最后 `just prereqs`。读数说缺，而装法只能是一句话时，这一行按本节 bash 的形状加进来，探测是一种新的 `Detection`。
-/

/-!
## 8-59 CLI 自己的样子：一张表，四个状态词，一句下一步（`bin::doctor::paint`、`bin::doctor::screen`）

**原因**：`doctor` 的输出是左对齐的散文，版本整行贴出（`ffmpeg version N-125649-g8d3942 Copyright (c) 2000-2026 …`），没有分组也没有总结，一个人要逐行读完才找得到红的那一行。

```rust
// bin::doctor::paint（形状 3 projection：findings → 人读的行，不问机器、不读环境）
pub(crate) enum Ink { Colour, Plain }
impl Ink { pub(crate) fn or_plain(self, no_color: Option<OsString>) -> Ink; }
pub(crate) enum Status { Present, Missing, Broken, Optional }
pub(crate) enum Part { Required, Recommended }
pub(crate) fn row(finding: &Finding, ink: Ink) -> String;
pub(crate) fn count(findings: &[Finding], part: Part) -> Counted;   // 一组算一件
pub(crate) fn summary(findings: &[Finding], ink: Ink) -> Vec<String>;

// bin::doctor::screen
pub(crate) struct Asked { install, city, explain, ink }
pub(crate) fn asked(args: &[String], no_color: Option<OsString>) -> Asked;   // --no-color 与 NO_COLOR
```

- **定宽两列加一句细节**：名字列 18、状态列 9，状态是四个词之一——有／无／坏／可选。人读的是一列词，而不是一句句子。
- **版本只取第一段数字**：先找带点的十进制串（`133.0.3`、`2.43.0`），没有的话取第一个含数字的词并截到 12 个字符（ffmpeg 的 `N-125649-g8d`）。整行贴出会把其它列挤出屏幕，而 `Copyright … 2000-2026` 里的年份正是「取第一个数字」这条更笨的规则会取到的东西。
- **必备／推荐两段**：必备 = Use 档里挡路的项（`Required` 与 `OneOf`），其余全是推荐（Use 档的可选项加整个 Develop 档）。`Part::of` 是终端这条划分的唯一权威。页面不按这两段而按层分段（运行一座城／开发 sprawling），因为页面每段的进度条与「还差」读的是那一层的 verdict，而「推荐」一段混着两层，它的进度条对不上任何一句 verdict。
- **颜色是一份终端可以拒绝的提议**：`NO_COLOR`（无论它设成什么）与 `--no-color` 任一即可，且 `paint` 自己不读环境——ink 是 `screen` 决定后传进来的值，于是测试不必动运行中的机器上的变量就能要到两种答案。
- **先说话，再探测；探测并行**：标题行在第一项探测开始之前就写出并 flush，于是人面对的不是一块空屏；`examine` 为表里每一项各开一个作用域线程同时问，整份报告的等待是最慢那一项而不是所有项之和（逐项串行时首行要等 2.5 秒）。线程数就是表的行数，不按机器调：每项的成本是等一个子进程回答，不是占一个核。`Machine: Sync` 因此是 trait 的一部分。
- **每一行在它能写的那一刻写出**：`examine_each` 把每项的答复按答到的次序交给调用者，屏幕按屏幕次序（先「required」各项、再「recommended」各项，各自保持表序）缓存行文，某一行连同它前面的所有行都答了，就立刻写出并 flush。于是第一行在屏幕次序里第一项答到时就出现，而不是等最慢那一项。不按答到的次序直接写，是因为行归在两个标题之下，按答到的次序写会让行落到错的标题下，并让同一台机器的两次报告排法不同；代价是一项慢的会压住它后面的行，而这份等待由 `running` 的 patience 封顶。
- **总结与下一步**：两段各一行 `n / m ready`（一组算一件），末行是从这里往下的那一条命令——必备齐了是 `sprawling up`，不齐是 `sprawling doctor --install`。

**本章测试**：`one_row_per_item_carries_one_of_four_status_words`、`no_color_is_honoured_from_the_environment_and_from_the_flag`、`a_version_is_the_number_out_of_whatever_the_tool_printed`、`the_report_is_grouped_into_required_and_recommended`、`a_family_of_browsers_counts_once_in_the_summary`。

### 8-60 `bin::revealing`：把一条路径交给人自己的文件管理器（形状 4 适配器）

```rust
pub(crate) fn reveal(city_root: &Path, at: &Address) -> Result<(), AxError>;
// accounting::worker::RunWorker 的字段：打开时装上 revealing::reveal
reveal: fn(&Path, &Address) -> Result<(), AxError>,
```

- **worker 经它被交到的 `fn` 指针碰这里**：生产的 `Hands`（`bin::assembly::production::hands`）装上 `revealing::reveal`，经 `Hands.reveal` 交给 worker 的构造器，`Command::Reveal` 调的是那个字段，不直接调本函数。本模块启动主机的一个程序，worker 搬进 `accounting` 时它留在 `sprawling`（`crates/accounting/Spec.lean` §7、accounting D10）；脚本场景交一个自己的 `fn`，就不会在宿主上起文件管理器。钉住它的测试是 `a_reveal_reaches_the_file_manager_the_worker_was_handed`。

- **为什么是一条命令而不是一个链接**：浏览器打不开 `file://` 之外的东西，而 `file://` 打开的是一个目录列表而不是人平时用的那个窗口。城代为执行，于是「在文件管理器里指出来」这件事在三个平台上各自用它们自己的办法完成：Windows `explorer /select,<路径>`、macOS `open -R <路径>`、其余 `xdg-open <父目录>`。
- **文法即闸**：入参是 `Address`，它在语法上爬不出城，所以「请求城外的一个路径」这句话在线上拼不出来。此处不再加第二道路径检查——那会是同一条规则的第二个权威。
- **`/select,` 后面没有空格**：有空格时 `explorer` 把它解析成两个参数，打开的是人的主目录，退出码是 0。这条只有跑起来才会现形，所以钉在测试里。
- **两种拒绝各说各的**：地址在盘上不存在是 `E_PATH_NOT_FOUND`（页面比树旧）；文件管理器起不来是 `E_TOOL_UNAVAILABLE`（这台桌面没有处理程序）。两者都不是「城坏了」，所以都带可执行的恢复语。
- **Linux 只开父目录**：`xdg-open` 没有选中参数，而各文件管理器的选中写法互不相同——那会是一张这座城得跟着上游改的表。父目录是所有桌面都能兑现的承诺。

**本章测试**：`a_path_this_city_does_not_hold_is_refused_rather_than_opened`、`the_selected_path_travels_as_one_argument`。

### 8-62 `accounting::worker::credentials::probing`：一次 probe 答的是读数（形状 4 适配器）

```rust
pub(super) struct Probing { pub reach: kernel::Reach, pub served: Result<Vec<gateway::ModelFacts>, AxError> }
pub(super) fn reach_of(base_url: &str, proxying: Proxying, monotonic: fn() -> Instant) -> Result<kernel::Reach, AxError>;
pub(super) fn probed_payload(name: &str, base_url: &str, found: Probing) -> Result<Payload, AxError>;   // 经 Payload::of(&kernel::event::record::EndpointProbed)
pub(super) fn tuning_of(wire: wire::EndpointTuning) -> Result<gateway::EndpointTuning, AxError>;  // accounting::worker::credentials
```

- **`probe_endpoint` 不再因读不出模型表而拒绝**。它记一条 `endpoint_probed`，里面是分段读数（`gateway::reach` 量出，用时是调用方以 `Hands.monotonic` 前后两读之差，§8-129-2）、模型表、以及读不出时那条拒绝自己的 code 与 subject。理由是这四段对填表的人是四个不同的下一步，而作为一次拒绝返回时它们在界面上塌成传输库的一句话。**它仍会拒绝的两件事**：凭据引用拼不出来、载荷账本不收——两者都没走到发请求那一步，因此没有读数可报。
- **`attach_endpoint` 的拒绝语义一个字没改**：probe 失败而人没点名任何模型，仍然是拒绝，因为那样的城连一个可调用的模型 id 都没有。
- **probe 按调用时的那套头与期限发出**：一个需要自定义请求头的网关，在 probe 不带那个头时答 401，人于是读到「密钥无效」，而那把密钥是好的。`request_max_retries` 在这里被兑现一次——设置页上有人正在等这一个请求；模型调用的那一份由同一个数走另一条路兑现：`dispatching::agreeing` 在选定端点处把这个 `kernel::Retries` 原样冻进 `RunPlan`，`runtime::run::drive` 据此决定一次可重试的失败之后还有没有下一次。
- **`tuning_of` 是线上词汇与 gateway 词汇之间唯一的翻译点**：零读成缺省（清空一个数字框到达线上是 `Some(0)`，而没有请求能在 0 ms 内完成），空名字的头与不以 `/` 开头的 pointer 被丢掉（表单在人打字时留着空行），头的值读起来像凭据却不是 vault 引用时整次拒绝（`E_CONFIG_INVALID`，subject 是那个头的名字）；`stream_idle_timeout_ms` 两边同名，进 gateway 时仍是闲置界，只经同一条「零读成缺省」。
- **`EndpointsAnswer` 的每一行带 `label`**，取 `AttachedEndpoint::label()`，缺省即 name。

### 8-61 已经在等的那一批，合成一道屏障

`RelayGate::serve` 醒来后先把队列里等着的请求**全部取空**，再一次 `Ledger::append_all` 交下去，按位回信。

- **理由是屏障的价钱与记录条数无关**：实测（`durability_barrier`，windows-x86_64 NVMe 一档机器）一条一屏障 585.2 µs／条，五十条一屏障 13.2 µs／条，而其中真正的写只有约 2.5 µs。四条车道同时在跑、每一行都要越到这一条记账线程上来，所以一次排空手里常常不止一件；一件一件交下去，交的是同样的字节，付的是四倍的屏障。
- **等待的语义一个字没改**：回信仍然在落盘之后才发出，因为「`Ok` 即已落盘」正是 `EventRef` 之所以是一条已存在历史的引用（`crates/storage/Spec.lean` §8-1）。否决「给端口加一个显式屏障动作、`append` 只写不同步」：那会让一条已经发出的 `EventRef` 指向一条可能还不存在的历史。
- **整波失败即整波拒**：`append_all` 的第一条拒绝结束整波，每个在等的调用方都收到同一条拒绝——与单条 `append` 在它后面那条失败时给出的承诺相同。
- **计数店是证据**：`everything_already_waiting_reaches_the_store_in_one_wave` 用一家数波数的店断言四条同时到达的 draft 不花四次波，这是端口早就允许的第二实现，而不是为这条断言新开的洞。

### 8-63 bin::serving::journal（形状 4 适配器）：一行诊断离开本进程的唯一出口

**日志给人看，而「人」不止坐在终端前。** `docs/logging.md` 把日志与账本分得干净，但没有说日志不许给浏览器看；记录页第四个透镜先前是空的，因为线上没有帧携得动一行。

```rust
pub type Clock = std::sync::Arc<dyn accounting::Clock + Send + Sync>;
pub struct Journal { /* lines: broadcast::Sender<wire::LogLine>, clock: Clock —— 私有 */ }
impl Journal {
    pub fn new(clock: Clock) -> Journal;                 // 调用方交 `assembly::SystemClock`
    pub fn sink(&self) -> runtime::diagnostics::Sink;  // 终端一份，看的人一份
    pub(crate) fn lines(&self) -> broadcast::Sender<wire::LogLine>;
}
```

- **一个 sink 两张嘴，不是两份日志**：页面读到的是终端读到的同一条 entry、同一个层底、同一个 sink。本文件不把任何一行读回来，所以「判定与恢复逻辑不读日志」照旧成立。
- **时钟在构造时交进来**：`docs/logging.md` §8 认可装配层是唯一可以采样的地方，写 entry 的库不许有第二个时间源，本模块也不许；所以 `Journal::new` 收下 `assembly::SystemClock`，每一行问它一次，而 serving 不写出 assembly 的名字（§8-92）。读不到钟即 `t` 缺席，而不是把这一行丢掉——锚是 `seq`，为一个时间戳丢诊断是把代价付错了地方。
- **`Journal` 先于 `Diagnostics` 存在**：sink 是往通道里写的那一半，所以它必须先有；`Serving` 因此同时携 `log` 与 `journal`，在 `assembly::listen` 里取出 `lines()` 交给 `ServeConfig::logs`。
- **窗口 512 行**：比增量通道宽、比事件通道窄。`wire` 层底的一座城写得比人读得快，而这里丢掉的是一条诊断而不是一段历史。
- **`Level` 五个名字的映射住在这里**：`wire` 依赖图上够不到 `runtime`，一条测试把 `LogLevel` 的五个 serde 名与 `Level::as_str()` 逐个钉成相等。`Level` 现已是闭枚举（G-22），所以第六级会在这张映射表上编译失败，而不是悄悄落到某一档——归错一档与看不见的一行都不再可能。

### 8-64 机器上的两个动词：`DoctorInstall` 与 `DoctorRefresh`（形状 4 适配器）

`Query::Doctor` 答的是开城那一刻的快照（§8-53）。于是机器页只能把一行命令复制到终端，装完还要重启城才看得见结果。两条命令补上这段，执行点是 `accounting::worker::commanding::machine`。

```rust
// accounting::machine（Recipe 的唯一拒绝语；Runnable 只由它造，`crates/accounting/Spec.lean` §8-4）
impl Recipe {
    pub fn command(&self, item: &str) -> Result<Runnable<'_>, AxError>;
}
pub struct Runnable<'a> { /* 私有：program、args */ }
// bin::doctor::running（本二进制起安装程序的唯一一处）
pub(crate) const PATIENCE: u32 = 3_600; // knocks, TICK apart
pub(crate) fn run(item: &str, runnable: &accounting::Runnable, patience: u32, log: &Path) -> Result<(), AxError>;
// bin::doctor::host：一项安装的输出写到哪个文件
pub(crate) fn install_log(item: &str) -> PathBuf; // <系统临时目录>/sprawling-install/<item>.log
// accounting::worker::commanding::machine：worker 经 RunWorker.machine（accounting::Machine）探与装，生产实现是 doctor::ThisMachine；终端的 --install 经同一个 accounting::Machine::install
impl RunWorker {
    pub(in crate::assembly) fn doctor_install(&mut self, item: &str) -> Result<(), AxError>;
    pub(in crate::assembly) fn look_at_this_machine(&mut self);
}
// bin::doctor::table：一个名字在这个平台上的配方；RunWorker.recipe_for 在打开时装上它
pub(crate) fn recipe_for(item: &str) -> Result<&'static accounting::Recipe, AxError>; // InvalidArgs：表里没有；ToolUnavailable：这个平台没有配方
```

- **查表与取平台在表旁边的 `doctor::recipe_for`，写进度行在 `doctor_install`**：worker 搬进 `accounting` 时需求表留在 `sprawling`（`crates/accounting/Spec.lean` §7），所以 worker 经打开时交给它的 `RunWorker.recipe_for` 这个 `fn` 指针拿配方，而不是自己读 `REQUIREMENTS`。它不是一层穿透：它是表的查法，与表同住 `doctor::table`，一个名字要不要被拒只在那里回答。进度行仍由 `doctor_install` 写，写的时刻因此就是安装到达的时刻。钉住这条的测试是 `an_install_takes_its_recipe_from_the_table_the_worker_was_handed`。
- **只跑 `Recipe::Command`，走的是终端那条 `Machine::install`**，不是第二个安装器。`Print` 与 `Manual` 各自带着「人自己去做什么」被拒：管道进 shell 的脚本是没人读过的代码，这条纪律不因请求来自页面而松一格。需求表里没有的名字在起任何进程之前就被拒，因为页面问的是这份构建不认识的东西。
- **「这条配方这座城可不可以跑」只有 `Recipe::command` 一个家**（H-12）。它要么给出 `Runnable`，要么给出那句带恢复语的拒绝；终端（`screen`）、页面（`commanding::machine`）与机器适配器（`probe`）三处都问它，所以同一条打印配方在三扇门后读到的是同一句话。`Machine::install` 收的是 `Runnable` 而不是 `Recipe`，于是「不可跑的配方」在这一层已经不可表达，`runnable()` 与 `probe` 里那第二段措辞随之删除。
- **`bin::doctor::running` 是本二进制起安装程序的唯一一处，等待有上限**（B-25／F-10）。三件事一起成立：`stdin` 是 `Stdio::null()`，于是要人同意源协议、要人输密码的包管理器立刻读到输入结束而不是坐在一台没有人的终端前；等待是 `try_wait` 的**计数敲门**，而不是 `Command::status()` 那种没有尽头的阻塞；敲完即杀掉子进程并带着 `E_TIMEOUT` 返回，恢复语是「自己在终端里跑这一行」。**上限用敲门次数而不是墙钟，因为本二进制读时钟的地方只有 `bin::assembly` 一处**（ARCHITECTURE §10 第 4 条）；这同时让上限可断言——测试要三次敲门就得到三次，而对着墙钟的断言问的是它跑在哪台机器上。上限由 `running::patience_for(runnable)` 一处决定：`cargo install` 从源码编译，给 `BUILD_PATIENCE = 24_000` 次 × `TICK = 50ms` = 20 分钟，因为 cargo-mutants、cargo-deny 这一类在四核机器上冷编译要 5–12 分钟，3 分钟的上限让它们每次都被杀在半路；其余的包管理器下载现成的包，仍是 `PATIENCE = 3_600` 次 = 180 秒。杀不掉或收不了尸都写进那条错误的主题——本城起的一个停不掉的进程是人必须知道的事实。
- **每一项的输出一份文件**：`stdout` 与 `stderr` 写进 `host::install_log(item)`，每次安装从空文件写起。安装失败时，人要的是包管理器自己说了什么；这份文件的路径写进开始那一行日志，也写进失败的恢复语。放在系统临时目录而不是 `~/.sprawling/components/` 下，因为 components 下一个名字对应的目录本身就是 `Detection::Component` 的探测对象。
- **进度就是日志行**（`bin::doctor` 模块名）。安装是本城起的一个进程并等它，值得报告的两件事——将要跑什么、怎么结束——正好是一行日志的形状；第二条进度通道会是同一件事的第二个权威。
- **`doctor_install` 装完自己再探一遍，探不到就拒绝**：装完仍答启动快照的城，会告诉人他刚装的东西还是没有。包管理器报成功而这座城仍找不到这一项（最常见的是它装进了本进程启动之后才加进 PATH 的目录），`doctor_install` 在交出新答案之后以 `E_TOOL_UNAVAILABLE` 拒绝，主题以 `<item>:` 开头，恢复语是重启这座城。于是每一次安装恰好以两种方式之一结束——新答案里这一项在，或者一条点名这一项的拒绝——页面的「全部安装」按这两种结局逐项往下走，而不必读日志行的措辞。钉住它的测试是 `an_install_that_leaves_the_item_absent_is_refused_by_name`。
- **答案不入账本**：机器有什么不是这座城里发生的事——它在本进程之外被改变，写进历史就是写进一份会错的历史。它沿 `RunWorker::examine` 这个 sink 交给服务层的 views，与开城那一次写进去的是同一处。没有 sink 的 worker（命令行逐条驱动的那种）照样探、照样写那一行日志：一个行为取决于有没有人在看的动词，是两个动词。
- **跑在写线程上，代价有上限**：探测是十几个程序各被起一次的几秒钟，安装是一个包管理器，两者都占住其他命令排的那条队。但另一条路更糟——让读去起进程，会拿着 views 的锁把其他每一次读都堵住，而且没人要求它这么做。**此条先前接受的是「占住写线程」本身，那在无期限时等于永远**：一个等在输入上的安装程序会让 `Halt` 与 `Cancel` 都进不来，而那正是人最需要它们的一刻。故代价此后由 `patience_for` 封顶（上一条）：写线程最多被一次下载占住 180 秒、被一次 `cargo install` 占住 20 分钟。后者是明知的代价：页面在这段时间里把那一项画成「正在装」，人看得见城在等什么；把安装搬出写线程要一条把结局送回页面的新通道（今天结局是命令的回答），那是剩下的一步，不在本节。

### 8-65 `agent_protocols::mcp::sse`：一台在流上应答的 server（形状 4 适配器；实现 `agent_protocols::Outbound`）

第三种 transport，也是唯一一种「请求与它的答不是同一次交换」的。

```rust
pub(crate) struct SseServer { /* 私有：消息端点、已兑付的 headers、client、事件接收端 */ }
impl SseServer {
    pub(crate) fn open(url: &str, headers: &[(String, String)],
                       resolve: &gateway::SecretResolver) -> Result<SseServer, AxError>;
}
impl agent_protocols::Outbound for SseServer { /* call：先 POST 再等流；notify：只 POST */ }
```

- **先开流，再说话**：规范让 server 把消息端点作为第一个事件播出来，故在流开口之前无处可投。开流因此自带期限（15 s），一台始终不播端点的 server 被拒，而不是被投到一个猜出来的路径上。
- **15 s 只管「开口并播出端点」，不管流本身**：reqwest 的请求级 `timeout` 从连接一直算到响应体读完，而这条流的响应体就是整段对话，挂在 GET 上它会在 15 s 后把一条好好的流掐断，此后每次调用都读到「流已结束」。故 GET 不带请求级期限，连同发送一起放进读取线程；读取线程送出的第一条要么是开流失败（`Err`，原样是 `unreachable`／`refused` 的那条错误），要么是播出的端点，这一侧对这第一条带 15 s 期限等——连不上、不答头、不播端点三种挂法都落在同一个期限里，而流一旦开口就只由每次调用自己的 `patience` 约束。阻塞 client 自带 30 s 的整请求默认期限，同理对这条流致命，故 client 以 `timeout(None)` 建成，每次 POST 各带自己的 `patience`。播出的多半是一条路径而不是整条地址，故按流自己的地址解析——一台在反向代理后面的 server 只知道它自己那条路径。
- **读取线程与 `agent_protocols::mcp::stdio` 同形、同理由**：读流没有自己的期限，故一条线程把阻塞读变成这一侧可以带期限等的通道；丢掉最后一个句柄即丢掉接收端，下一次发送结束读取线程。
- **先开流，再说话**：规范让 server 把消息端点作为第一个事件播出来，故在流开口之前无处可投。开流因此自带期限（15 s），一台始终不播端点的 server 被拒，而不是被投到一个猜出来的路径上。播出的多半是一条路径而不是整条地址，故按流自己的地址解析——一台在反向代理后面的 server 只知道它自己那条路径。
- **读取线程与 `agent_protocols::mcp::stdio` 同形、同理由**：读流没有自己的期限，故一条线程把阻塞读变成这一侧可以带期限等的通道；丢掉最后一个句柄即丢掉接收端，下一次发送结束读取线程。
- **一次 POST 不是一个答**：对侧用 202 收下并一言不发，答随后作为事件到达。故 `call` 先投再等流，`notify` 投完即止。
- **克隆共用同一条流**：一台 server 是一次对话，无论一次 Run 握着它几件工具；两条流会让一次调用的答落在调用方没有在读的那一条上。
- **401／403 抬 `E_CREDENTIAL_MISSING`**，与 `agent_protocols::mcp::http` 同一条口径：server 在、也听懂了，缺的是一个账号，而人接下来要做的是登录而不是检查地址。健康视图（§8-66）据此把它画成「认证中」而不是「失败」。

### 8-66 `accounting::views::mcp_health`：一个地址够得到的每台 server 此刻站在哪（形状 7 投影）

```rust
impl LiveAsk {
    pub(super) fn mcp_health_answer(&self, addr: &Address) -> wire::McpHealthAnswer;
}
```

- **现问现握手，恒不折自账本**。一台 server 通不通是关于此刻的事实——一个起得来的程序、一台应答的主机、一个仍然有效的账号——记下来的那一份会在它停掉一小时后仍说它在。读配置的方式与 `Query::Document`／`Listing` 读这棵树的方式相同。
- **用的就是 Run 起点那一次握手**（`agent_protocols::handshake` ＋ `tools/list`，经 `McpLink`），故人读到的与模型拿到的不可能不一致。三种状态的判断无一处要猜：`E_CREDENTIAL_MISSING` 即 `Authenticating`，其余拒绝整条上线。
- **这是唯一一条按秒计的读，代价写在这里**：它拿着 views 的锁，每台 server 一次握手。页面因此只在人打开 MCP 页或加完一台 server 时问一次，恒不上定时器、也恒不由记录触发（`staleBy` 对它答 false）。
- **金库是借来的，不是这里开的**（`Views::lend_the_vault`，开城时交一次，与 `found_on_this_machine` 同形）：同一批凭证上的第二个句柄就是第二扇门。没有金库的 `Views`（重建、测试）把要凭证的那台 server 报成兑付不了，而不是把引用当成值发出去。

### 8-67 四段全部入库，以及读回它们的三个投影

**装配点。** `freeze_plan` 把四段全部 `intern` 进内容仓库，而不再只落 must-read 名单上那几份。先前 city 段与 JOB 段有对应对象、building 与 resident 两段没有，于是 `prompt_assembled` 里四个哈希有两个在 `cas/` 里找不到——一个人拿着哈希读不回原文。内容寻址天然去重：一栋楼的规则无论被多少次 run 冻结，仓库里都只有一份。

city 段与 building 段同时携上它们的来源文档（`Assembled`：字节与 `Vec<SegmentSource>` 恒同行），地址由 `city::building_path`／`city::agents_path` 反算而来，**不在这里重述一栋楼的文件布局**。resident 段与 run 段不携来源：前者由身份与目录拼成，后者由 brief 与交接文本拼成，都不是可被打开的文档。一条位于城内却拼不成 `Address` 的路径是失败而不是猜测——一行读者点不开的来源，比一次回绝更糟。

**三个投影。**

```rust
// views::prefix
fn prefix_answer(&mut self, run: RunId) -> Option<wire::PrefixAnswer>;
fn content_answer(city_root: &Path, locator: &Locator) -> Option<wire::ContentAnswer>; // 锁外：读内容仓库（§8-100）
// views::skills
type SkillPins = BTreeMap<(String, B3Hash), Vec<RunId>>;
fn skills_answer(city_root: &Path, building: &Address, pins: &SkillPins) -> Option<wire::SkillsAnswer>; // 锁外：扫书架（§8-100）
// views::git_status
fn git_status_ask(&self, building: &Address) -> GitStatusAsk; // 锁内：城根、楼、最近一次检查点
impl GitStatusAsk { fn read(self) -> wire::Answer; } // 锁外：读工作树（§8-100）
```

**五条口径：**

1. **`prefix_answer` 取该 run 最早的一条 `prompt_assembled`。** prefix 一次冻结管一次 run 的一生，之后每一轮记的是同样四个哈希；取最早的那一条，一次没走过第一轮的 run 也仍有答案。
2. **字节的可读性判定只有一处。** `views::document` 的 `read_bytes` 同时服务树上的文件与仓库里的对象——什么样的字节算文本，不取决于它被存在哪里。
3. **技能的书架在被问的那一刻扫盘，而 pin 出自历史。** `city::Library` 是书架的权威，旁边再留一份索引就是磁盘说法的第二份副本；而「哪些 run 用过」折自 `run_started` 里那张 `skills` 表（`Views::skill_pins`，键为名字与哈希成对），不是第二次扫盘。pin 表在锁内复制出来，扫盘在锁外对着这份副本做（§8-100）。
4. **`git_status_ask` 的比较基准取自历史而不是 HEAD。** 该楼最近一条 `checkpoint_committed`／`pr_merged` 就是基准，它由 `commits_answer(Some(building), None, 1)` 给出——变更栏旁边显示的那一行，正是提交列表打开时的第一行。
5. **仓库句柄按次打开。** 这是投影里唯一一处伸向它不拥有的目录的读；跨重建留着的句柄会活得比开它的那座城还长。

### 8-68 `bin::release`：这是哪一版，以及唯一一次去问注册表（形状 4 适配器）

```rust
pub enum Built { Released(Release), FromSource }
pub fn built() -> Result<Built, AxError>;      // option_env!("SPRAWLING_RELEASE_TAG")
pub fn newest() -> Result<Release, AxError>;   // GET registry.npmjs.org/sprawling/latest
pub fn answer() -> ReleaseAnswer;              // 两读合判，恒不失败
```

**五条口径：**

1. **人问才发生。** 没有定时器，没有首次运行时的探测，也不搭另一条命令的车：`status` 只读编译进来的东西、一个套接字都不碰，只有 `status --check` 会出网。`QUICKSTART.md` 的开场承诺是「什么都没装、没注册服务、删掉文件夹就干净」，一个按自己时间表去够注册表的二进制，是在拿那句承诺换一个没人问过的问题。
2. **什么都不更新。** 二进制住在哪，归当初装它的人管——归档路径归 `sprawling install`，npm 路径归 npm（`tools/xtask/src/channel/shim.js` 已立此规）。故本模块只报告然后停下，答案里印的是该跑的命令，选哪条仍由选了安装渠道的那个人决定。
3. **`Built` 两态而不是 `Option<Release>`。** 缺席不是一个缺失的值，而是关于这次构建的一个事实：从工作树构建出来的二进制没有可比的对象，把它报成「过期」是在回答另一个二进制的问题。tag 由 `release.yml` 经 `SPRAWLING_RELEASE_TAG` 传入，`build.rs` 声明该变量（`cargo::rerun-if-env-changed`），否则 cargo 会拿上一个 tag 编出来的二进制顶数，而它的每一份都会报错版本。
4. **问 npm，不问 GitHub。** 本项目每一次发布都是 pre-release，而 `GET /repos/{owner}/{repo}/releases/latest` 按设计排除 pre-release，对本仓库答 404。npm 的 `latest` dist-tag 才是 `bunx sprawling` 真正解析的东西，问它才是问人真正有的那个问题。
5. **失败说清停在哪一阶段。** 只在失败路径上多发一次 `gateway::reach`，把 `kernel::reach` 已定义的分阶段读数——名字没解析、连不上、握手失败、对方答了什么状态——放进 recovery。「它没成功」不是一个人能据以行动的答案，而这条路径上多一次请求换一句能行动的话是划算的。退出码报的是问题有没有被回答，而不是答案是什么：版本过期是消息不是故障，而读不到注册表会让人以为自己查过了。

### 8-69 真端点验收闸 `just e2e`（`crates/sprawling/tests/e2e.rs`）

这个仓库的其余检查都在回答一个它自己写的 provider。这一条把一个人粘贴的 base URL、key 与模型名拿来，attach → 选模型 → 派活，再回头读账本——于是「key 填了，从来没跑通」是一次红，而不是一份报告。

**六条口径：**

1. **缺席就一行说明并通过。** 轮次是一张表：`gateway::known_hosts()`（由 `PRESETS` 算出）的每个主机的每个 face 各一轮，那个主机的 key 与模型名在 `SPRAWLING_E2E_KEY_<主机>`／`SPRAWLING_E2E_MODEL_<主机>` 里（主机名大写、非字母数字换成 `_`，如 `SPRAWLING_E2E_KEY_API_DEEPSEEK_COM`），一个 key 用于这个主机的全部 face；再加一轮「人粘贴的那个端点」，由四个环境变量 `SPRAWLING_E2E_BASE_URL`／`SPRAWLING_E2E_KEY`／`SPRAWLING_E2E_MODEL`／`SPRAWLING_E2E_DIALECT` 给出，接表外的中转或回环地址上的服务。每一轮缺什么由测试自己印出来，justfile 不抄第二份；一个主机只给了 key 与模型名中的一个是写坏的配置，同样印出。表不手写，加一行预置主机就多一组轮次（`crates/gateway/Spec.lean` §8-32 是同一张表的白盒一半）。每个测试对每一轮各立一座城，失败时点名是哪一轮。这与 `just adversary` 是同一个诚实形状：静默跳过的闸比没有闸更糟，而在每台没有 key 的机器上都红的闸没有人会跑。持有凭据的作业设 `SPRAWLING_E2E_REQUIRED=1`：此时粘贴端点那一轮的四个变量缺任何一个都判红，按主机的一轮只给了一半也判红，并印出同一行缺了什么——凭据在那里本该齐全，缺席是配置坏了，而不是所在机器正当地没有 key。只认 `1`；其他值与未设同义，免得一个拼错的开关悄悄换回跳过。
2. **不进 `just check`。** 它花的是别人的钱与别人的网络；有没有 key 是一台机器可以正当地没有的东西。它也不进任何定时工作流：凭据只在人自己的机器上，由持有 key 的人在自己的机器上跑 `just e2e`，仓库不存 key，也不存为它准备的变量（人的定规：需要人的 key 的定时作业删掉）。设了 `SPRAWLING_E2E_BASE_URL` 时其余三个就是欠着的，`SPRAWLING_E2E_REQUIRED=1` 让缺哪个就红哪个。
3. **从 `RunWorker::handle` 进城，不另起进程。** 本闸要判的真端点是 provider 的那一个；`CARGO_BIN_EXE` 与套接字那一侧归 `tools/adversary/`（xtask boundary 闸：白盒 Rust、黑盒 Lean）。Roadmap §20.6 写的是「经真二进制 HTTP 面」，此处按既有的边界规则改为「经 `wire::server` 递帧的那扇门」——多起一个进程只会多付一道边界成本，而断言仍然共享产品的类型，正是那道闸判为两头不讨好的形状。
4. **兼容格式随它的端点走，不做命令行开关。** §24.3 写的是 `just e2e --dialect messages --relay`。兼容格式是「你指向的那个端点说哪种线」的属性，与 base URL、key 同源，故与它们并列为环境变量；旗标是这个事实的第二个家，且能与另外三个变量互相矛盾。字面拼法由 `DialectKind` 的 serde 命名给出（`anthropic`／`open_ai`），测试不另列一张表。
5. **三个断言分三个测试，因为它们的波次不同。** ①`model_called` 出现且账本不带 `E_CONFIG_INVALID`／`E_WIRE_MISMATCH`（W1 关门）；②`model_called` 说得出输出上限来自哪一环（`ceiling_from` 非空）——**这一条在 1.1 上限链条落地前是红的，它是钉在那片叶子前面的桩**，与 ① 合并就成了两个事实一个判决；③ 故意断 key 必红，且 recovery 非空、错误码不是 `E_CONFIG_INVALID`——被拒的凭据是端点的答复，不是一份写坏的配置。
6. **时长由两次 cargo 调用与两个调参守住。** 编译不占额度：`--no-run` 先付编译（一台 windows-msvc 机器上冷构建实测 2m18s），`timeout 180` 只罩测试进程。`request_max_retries = 0` 让 gateway 的退避一次都不睡，`timeout_ms = 60000` 让一次请求封顶一分钟（Roadmap §0.0：单次 `sleep` ≤ 10 秒、单个测试进程 `timeout` ≤ 180 秒）。三个测试各把每一轮跑一遍，所以 180 秒罩住的是「轮数 × 三次派活」：一次填进的主机多到装不下时，分几次跑，每次只导出其中几家的变量。

**attach 就是一次真调用。** `admit` 为空表示「这个端点服务什么就收什么」，于是登记当场去问它的模型清单——一把被拒的 key 在 attach 处就被回绝，走不到派活。故三个用例都把 attach 与派活串成一个 `Result` 来判，而不是假定拒绝只会在最后一步出现。

### 8-71 空着的上限不是被抹掉的上限（`credentials::endpoints::select_model`）

- **缺陷**：设置页每次选模型都把整行发上来，于是一个人重选自己已经登记过的模型，就把当初填的上限用一个空框覆盖掉了；下一次 messages 兼容格式的调用因为写不出 `max_tokens` 被拒（A 章 B-01 的第二段）。`None` 从此表示「这次没说」，而不是「这次要清空」。
- **权威从高到低**：人这次填的 → 这个 endpoint 与这个 model id 上一次登记的 → 钉版目录行。**按 endpoint 与 model id 读上一次，而不是只按 tag**：把一个 tag 指向另一个模型时，旧模型的上限不得跟过去。上一次登记从 `book.choices()` 读回——书是「这座城登记了什么」的唯一陈述，在它旁边另存一份就是第二个权威。
- **`context_tokens` 同理**，`0` 是「这次没说」；两个数字读法一致，因为它们来自同一个空表单。窗口梯是人这次填的 → 上一次登记的 → 钉版目录行 → `gateway::provider::preset::window_for`（`crates/gateway/Spec.lean` §8-17）；四档都沉默时窗口为 `0`，上下文提醒随之不响。
- **输出上限的其余几档住 gateway**（`provider::ceiling`，`crates/gateway/Spec.lean` §8-17）：上游 `/v1/models` 的陈述、预设表与策略缺省 `OUTPUT_CEILING_DEFAULT`，以及 chat 与 responses 两面在人与上游都沉默时的 `ProviderDefault`。装配层不复写那条规则，只把人层、书里的值与这个端点的兼容格式交给它——一条规则两个家，漂开的总是没人看的那个。
- **来源入账**：`model_selected` 带 `ceiling_from: person | upstream | preset | policy | provider`，拼写取 `OutputCeiling::word`，载荷由 `gateway::router::payload` 一处写。`provider` 行的 `max_output_tokens` 缺席：请求里没有这个字段。账本里看得见来源，因此一次被截断的跑是读出来的，不是猜出来的。

### 8-73 run 标识直接取自摘要，而停不下来的疑问算「停」（`accounting::worker::dispatching::agreeing::run_id_for`、`accounting::worker::driving::lane`）

- **缺陷**：`run_id_for` 把摘要印成十六进制再逐对解回字节，两步各带一个 `unwrap_or`——`from_utf8` 失败取 `"00"`，`from_str_radix` 失败取 `0`。一次解不出的摘要于是变成全零的 run id，而两条不同的活会得到同一个标识。
- **改法**：`B3Hash::as_bytes()` 的前十六字节即标识，解析这一步整个消失。字节与旧写法逐位相同（印出来的十六进制正是这些字节），故账本与 replay 的字节不变。
- **`Interrupting::ask` 的同一类默认**：`backlog.stopping(id)` 的 `Err` 此前读作「没停」。读不到那张表的城答不出这个作用域还开着，于是改答为「停」——一次多余的取消看得见，一次漏掉的取消让 run 跑在人已经关掉的作用域里。
- **`SignalDesk::take_steer` 的拒绝不折平**：desk 返回 `Result<Option<Steer>, AxError>`——`Ok(None)` 是空队列，`Err` 是一件已离队、却读不成插队信的信（非 steer 型，或载荷里没有文字）。`ask` 对 `Err` 答 `Interrupt::None`，与 `accounting::worker::desk` 给人那一侧一个空 steer 的答案同字：安全点不是为一封读不懂的信停下来，而没有文字的信也没有内容可以交给这一跑。区别不在答案而在不折平——desk 把每一件取走的信都记成 `Consumed`，于是它不再在两个出口之间消失。

**本章测试**：`two_jobs_at_one_millisecond_get_two_run_ids`（`accounting::worker::dispatching::tests`）。

### 8-74 折叠读不懂的那一行就说出来，而「关」与「开」只有一种拼法（`accounting::worker::folds`、`views::holding`）

```rust
fn scope_of(scope: &wire::HaltScope) -> kernel::event::Scope;   // accounting::worker::naming：唯一的翻译
impl Governance { pub(crate) fn absorb(..) -> Result<(), AxError>;   // views/governance.rs
                  halted: BTreeSet<kernel::event::Scope> }
impl RunWorker { fn halted_by(&self, addr: &Address) -> Option<kernel::event::Scope>; }
```

- **`absorb` 改 `Result`**：读不懂的审批项与缺字段的 `run_started` 此前被静默丢掉，于是「历史里有一条这个 build 读不懂的线」表现为开城后少了一批待答项与一段活的说明。两处改为 `E_WIRE_MISMATCH`，恢复语指向写下这段历史的那个 build。
- **「关／开」与「谁来答」的拼法都搬进了 kernel**：`Admittance`、`Scope`、`autonomy_word` 住 `kernel::event`，两个折叠与写方经 `Payload::of`／`Payload::read` 读同一个结构。此前 `Governance` 把不认识的状态词读成「开」、`Views` 忽略整行，而 `read_autonomy` 把读不懂的委派静静读成「人自己答」——三处默认都没了，读不懂的行是一次 `E_WIRE_MISMATCH`。
- **`halted` 存 `Scope` 而不是字符串**：`halted_by` 用 `Scope::covers` 问包含关系，`split_once(':')` 连同它解不出地址时的静默跳过一并消失；`CityAnswer.halted` 在答的边界上用 `Display` 拼回同样的词。
- **写的一侧同源**：`set_admission` 收 `kernel::event::record::Admittance`，载荷由该枚举的 serde 拼写写出。

**本章测试**：`an_unreadable_approval_item_stops_the_fold`（`accounting::worker::folds::tests`）、`an_appointment_this_build_cannot_read_is_refused_rather_than_defaulted`（`kernel::event::record::governance`）。

### 8-76 哪些记录会动计划，是一张穷尽表（`accounting::plan_view::reach::may_move_plan`；`crates/accounting/Spec.lean` §8-6）

```rust
enum PlanReach { Untouched, Stale, NodeFreed, NodeStopped }
fn may_move_plan(kind: EventKind) -> PlanReach;
```

- **失效判定是按 `EventKind` 的穷尽表，没有通配臂**：加一种事件，就要在这里说它动不动计划。
- **没有地址的记录**：只要它的类别会动计划，就清掉每一份解析；一条没有地址的 `checkpoint_committed` 因此不会让每栋楼继续报旧的表。
- **`pr_merged` 读作 `Untouched`**：它同样会把文件落进楼里，改成 `Stale` 要连着改 `views::commits` 的期望，那是这张表之外的一件事。

**本章测试**：`a_record_with_no_address_stales_every_plan_it_could_have_moved`、`every_event_kind_has_a_reach`（`accounting::plan_view::tests`）。

### 8-78 一次派活在会计线程上花了多久，城自己说出来（`accounting::worker::dispatching::running`、`accounting::worker::workbench::servers`）

```rust
// stage_dispatch：进出各读一次城钟，Level::Trace 报出
"prepare_dispatch took {ms} ms for {addr}"
// mcp_tools：同法，单独一行
"mcp_tools took {ms} ms over {n} declared server(s), offering {k} tool(s)"
```

- **两行而不是一行**：`mcp_tools` 是常驻连接表（F-13）唯一能省掉的那一段，混进总数就说不清省了多少。总数含协商、开房间、写简报、立 run、铺工作台与冻结计划；这一切都在会计线程上，期间没有任何车道的追加被服务。
- **量在先，门在后**：`tools/xtask/budgets.toml [prepare_dispatch]` 只记读数与机器，不设上限——上限若写在读数之前，要么形同虚设，要么挡住正是要修它的那次改动。
- **读钟读不出不丢工具**：`mcp_tools` 无法报出 `Result`，因此钟失败时只是不报这一行；连接是工作，读数是诊断。
- **本章测试**：`a_dispatch_says_what_it_spent_before_the_drive`（`accounting::worker::dispatching::tests`）盯住「读数被说出来」这一件事，不断言数值——墙钟数值属于跑它的那台机器，属于 `budgets.toml` 的那一行。

### 8-79 会话冻下的形状只选一次（`accounting::worker::dispatching::session_shape`、`accounting::worker::dispatching::running`）

```rust
impl RunWorker {
    /// 记下这次派活冻下的形状，或拒掉一次会移动会话已冻下形状的派活。
    pub(super) fn choose_shape(&mut self, at: &Assignment, model: &gateway::ModelEntry)
        -> Result<(), AxError>;
}
```

- **不变量一句话**：一个会话的调用形状选一次。房间的 `CONFIG.toml` 在它的第一个 Run 写下 `[model] name`（人选了强度就一并写下 `[model] effort`），此后每次派活读回来与自己要冻下的形状对拍，不等即拒。
- **为什么住在派活面**：模型、输出上限与 effort 都上供应方的线，中途任何一个移动都会让会话此后每一轮为一段字节从未变过的 prompt 付全价。这道对拍在写简报之前，因此被拒的派活不落一个字节、不动会话一行记录——`stage_dispatch` 的序幕顺序（§8-166）就是这条拒绝的位置理由。
- **对拍本身是 runtime 的**：`runtime::turn::CallShape::verified_against` 拥有比较与三句拒绝语，恢复语指向人现在就做得到的两件事——**把动过的那一项改回去，或换一个地址派这件活**（`crates/runtime/Spec.lean` §8-4-1；`/new` 上线后改的是那一个常量）。派活面只负责造出两个形状：会话记下的那个，与这次派活将要冻下的那个（模型来自 endpoint book，effort 来自梯子）。
- **effort 比的是梯子在这里解析出的值**，不是房间那一行：`[model] effort` 一个键都没写过的会话，冻下的是「让供应方决定」，而从这一档挪到任何一档同样是形状移动。
- **模型比的是登记面现在的答案**：`SelectModel` 改的是城级 tag→model 登记，在那里拦会把正常配置一起禁掉；被挡的是它落到一个已开会话上的那一步。会话记下的模型仍在 endpoint book 里时，上限与窗口从那一行读（它们是模型的属性，抄一份进房间的配置就是同一个事实的第二个家）；模型已不在登记面时，模型那一臂先拒。
- **`[model] name` 不是「这个 Run 用哪个模型」的权威**：Run 的模型仍由 `EndpointBook::select` 选，会话记录只说它当初从哪个模型开始。两者不一致时这次派活被拒，而不是两个家各说各话（`crates/city/Spec.lean` §8-14、§8-4 的 `own_layer`）。
- **地址就是楼时，地址自己就是会话**：那种地址的「自己那份 `CONFIG.toml`」就是楼的 `CONFIG.toml`（`Layer::Resident` 与 `Layer::Building` 同一文件，`crates/city/Spec.lean` §8-4）。
- **本章测试**：`accounting::worker::dispatching::session_shape::tests::a_session_keeps_the_shape_it_froze_and_refuses_a_different_effort`——同一房间连续两次派活，四条前缀逐槽位哈希相同且 CAS 里的字节相等；第三次改 effort 被拒（`E_CONFIG_INVALID`，恢复语给出两条出路），会话的 effort 一行未动、没有多出一条 `prompt_assembled`；第四次相同请求照跑并再次冻下同一批字节。`a_model_chosen_after_a_session_opened_does_not_reach_it`——已开会话之后改城级登记，往该会话的派活被拒，房间记录仍是原来的模型，而新会话拿到新模型。

### 8-80 浏览器的版本读自它旁边的文件，而不是跑它一次（`bin::doctor::version_file`）

**原因**：`family::look` 对找到的每个浏览器跑一次 `ask_version(path, "--version")`。Windows 上 Chromium 系的浏览器收到 `--version` 不打印版本，而是**开一个窗口**；浏览器已在运行时，新进程把请求交给已有实例后自己退出，于是 `running::stop` 杀掉的是那个壳，窗口留在屏幕上。这条探测在每次服务城时跑一遍（§8-54），设置页的「重新检查」再跑一遍，于是一台装了两个 Chromium 牌子的机器每次启动都被弹出两个窗口。**一次健康检查的副作用不该是替人开浏览器**。

```rust
// bin::doctor::version_file（形状 4 适配器）：装它的那个程序在它旁边写下的版本号
pub(super) fn beside(program: &Path) -> Version;

// bin::doctor::family：一族的答案不再经过任何子进程
pub(super) fn look(family, platform, search_path) -> Presence;
```

- **三种读法一条顺序，且不按族分支**：`application.ini` 的 `[App] Version`（Gecko 在 Windows 与 Linux 把它写在程序旁，在 macOS 写在 `../Resources/`）→ 程序旁以版本号命名的目录（Chromium 在 Windows 的 `Application\<x.y.z.w>\`，在 macOS 的 `Contents/Frameworks/*.framework/Versions/`）→ 程序上方 bundle 的 `Contents/Info.plist` 里的 `CFBundleShortVersionString`（macOS 三族通用，Safari 只有这一条）。**族不是参数**：「这个牌子属于哪一族」的权威是 `family::claims` 与 `member_at`，在这里再判一次就是同一个事实的第二个家。
- **版本目录按数字逐段比，不按字典序**：升级过的 `Application\` 下常常同时留着两个版本目录，而 `154.0.4258.9` 与 `154.0.4258.32` 的字典序把旧的那个排在后面，于是报出的是已被换掉的版本。
- **读不出版本仍然是 Present，`Version::Silent` 的意思随之扩成一句**：「版本号读不出来」——一个印了空行的程序与一个旁边没有版本文件的浏览器，对报告是同一件事，而两者都不使这一项缺席（§8-166 已定：不说话的工具仍是装了的工具）。`describe` 因此是 `no version` 而不是 `said nothing`：后者对一个从未被问过版本的浏览器是假话。
- **失去的那一件事如实记**：`ask_version` 顺带证明了「这个程序起得来」，读文件不证明。一个在盘上却起不来的浏览器此后报 Present，而真相在 `browser_bidi::lazy` 起它时以 `E_BROWSER_UNAVAILABLE` 出现。这是用「每次服务都开窗」换「探测期发现起不来」：前者每次启动都发生，后者只在真要用浏览器那一次才要紧。
- **驱动仍然问**：`chromedriver`／`msedgedriver` 是 `Detection::Program`，它们真的打印版本且不开窗，`ask_version` 与它的 deadline 因此留在那条路上，只是不再有浏览器走它。
- **被否决的备选**：① 只在 Windows 上改读法——同一个事实（浏览器的版本）会有两个家，而另外两个平台上的 spawn 同样是几百毫秒与一个别人的进程；② 读 `HKCU\Software\<牌子>\BLBeacon\version`——每个牌子一把钥匙、每个人一份注册表，而版本目录一条规则答全五个 Chromium 牌子，且它是装它的程序刚写下的那一个；③ 给 `Version` 加一个「来源」变体上线——`DoctorVersion` 的四个词回答的是「版本是什么」，来源是城的内政，上线会让今天在跑的客户端解不出整份答案（§8-57 对 `DoctorNeed` 的同一条理由）。

**本章测试**：`version_file::tests::a_browser_says_its_version_through_the_files_beside_it`（三种布局各一份夹具目录：Gecko 的 `application.ini`、Chromium 的两个版本目录取数字大的那个、macOS bundle 的 `Info.plist`；旁边什么都没有的程序答 `Silent`）；`no_browser_is_started_to_learn_its_version`（`family.rs` 的正文里没有 `ask_version` 这个拼写，于是这条不变量在唯一能破它的那个文件上被钉住）。

**本章验收**：装了 Edge 与 Zen 的 Windows 机器上 `sprawling up` 与设置页「重新检查」期间不出现任何新的浏览器窗口；`cargo run -p sprawling -- doctor` 仍报 `gecko present … (zen)` 并带版本号。

### 8-81 空着的密钥框不是删除（`accounting::worker::credentials::endpoints::kept_credential`）

**原因**：供应方表单把 vault 答复的引用只存在组件实例里（`setup/providers/form.svelte` 的 `held`），页面一卸载就没了；`EndpointSummary` 上线只带 `has_credential: bool`，不带引用，所以表单也无从向城要回来。于是第二次打开设置页按「看看」或「接上」时，帧里 `secret: None`：城读成「没有凭据」，探测不带任何 `Authorization` 发出（人读到的是 401「密钥无效」，而那把密钥是好的），接上则把已归档的引用覆盖成空——**静默丢 key**。

```rust
// accounting::worker::credentials（形状 2 value）
pub(super) enum Credential { Absent { header: Option<String> }, Key { .. } }

// accounting::worker::credentials::endpoints（形状 1 decision）
fn kept_credential(&self, name: &str, dialect: DialectKind, header: Option<String>) -> gateway::AuthSpec;
```

- **一条规则一个家**：`endpoint_of` 是 probe 与 attach 共用的那道门，空引用的读法因此只有它一处，`ProbeEndpoint` 与 `AttachEndpoint` 不可能对同一个空框给出两种答案。
- **空不是删，删是另一个动词**：拿掉一个端点的凭据要一个自己的命令，现在还没有（见 §8-46-11 撤销 attach 那一格）；空着的框永远不承担这个意思。一个既能表示「不改」又能表示「删掉」的字段，会让每一次不相干的编辑都带着删除凭据的风险。
- **留引用、重算头**：归档的是 `AuthSpec`（头 + 引用），沿用的只是引用，头按这次进来的接口形态重算——同一把 key 从 chat 面挪到 messages 面要从 `Authorization: Bearer` 变成 `x-api-key`，照抄旧头会对一把好 key 答 401。人自己命名的头仍然压过推导，`Credential::Absent` 因此带着 `header`。
- **头由接口形态一处推出**：`Absent` 与 `Key` 两条路都经 `gateway::AuthSpec::for_dialect`，人自己命名的头压过推导。被否决的备选：按归档的头原样沿用——那会让 API key 在换面时带着旧头 401。
- **前端说的话此后是真话**：`lang.json` 的 `setup_key_stored`（「此名下已有密钥，留空则沿用」）先前只在表单自己还记得引用时出现，而城当时并不沿用。现在城沿用，那句话改为在**城说这个 id 有凭据**时出现——一句话一个家，不新增第二个键。
- **被否决的备选**：① 把 `secret:realm/name` 放进 `EndpointSummary` 让表单送回来——凭据引用是城的内政，上线只为让页面把它原样送回，等于给同一个事实开第二个家，还多一条泄露面；② 让表单按约定重新拼出引用（`referenceOf(id)`）——那只对这张表单自己登记过的 key 成立，`import` 与环境变量来的端点引用不同名，会把别人的引用送进这一个端点。

**本章测试**：`credentials::tests::kept::an_empty_key_keeps_the_credential_this_city_has_archived`——带 key 接上后，再一次空框 probe 与空框 attach，三次模型表请求都带 `authorization: Bearer sk-archived`，且端点的 `auth` 仍是原引用。

- **`attach_endpoint` 不以探测为准入条件（`crates/gateway/Spec.lean` §8-10 是权威，这里只记装配侧）**：鉴权头由 `gateway::AuthSpec::for_dialect` 按兼容格式产出（人填的头优先），于是 Anthropic 兼容端点拿到的是 `x-api-key` 而不是必然 401 的 `Authorization: Bearer`。探测失败时，若 `admit` 非空则按人报的型号登记（`probed: false`，另写一条 `effect` 级诊断点名探测的错），`admit` 为空才拒，恢复语是「把要用的 model id 报上来，再登记一次」。落选的是「探测失败即拒、让人先修好 `/models`」：多数兼容端点根本不服务这个接口，那条路等于让人去修一个对端从未承诺过的东西。
- **`Credential` 是穷举枚举**：`Absent { header }`／`Key { reference, header }`。「这次没填」与「填了一把 key」是两件事，前者保留城已有的凭证，两个 `Option` 拼不出这个区别。`Credential::entered` 是线上命令的唯一入口。

### 8-82 同一个地址上的新一段：`/new`（`accounting::worker::commanding::sessions`、`Command::OpenSession`、`EventKind::SessionOpened`）

**原因**：房间的第一个 run 把模型与强度冻进它自己的 `CONFIG.toml`，此后形状不同的派活都被 `E_CONFIG_INVALID` 拒（§8-79）。换过主模型的人因此再也派不出去，而拒绝的恢复语原先指向两件做不到的事（§8-4-1 已改成诚实的那一条）。设计本身没错——缓存前缀不能中途换模型——错在拒绝之后没有出口，这一章给的就是那个出口。

```rust
// wire::command（形状 2 value）
pub enum Carry { Nothing, Handoff }        // Nothing 是第一个变体，即默认
Command::OpenSession { addr: Address, carry: Carry, from: Option<Origin>, idem: IdemKey }
// Origin = kernel::Origin { run, at_seq }，由 kernel 拥有，wire 与 runtime 都读它

// kernel（形状 2 value；事件表逐变体登记）
EventKind::SessionOpened                   // payload：{ carried: bool }；地址在记录自己的 addr 字段

// accounting::worker::commanding::sessions（形状 1 decision + 一次写）
fn open_session(&mut self, addr: &Address, carry: Carry) -> Result<(), AxError>;
```

- **行李里只有行李。** `carried` 在 payload 里，房间在记录自己的 `addr` 字段里（`record_at`）——一条属于某个地址的线本来就在信封上说了地址，payload 再抄一份就是同一个地址的第二个家（`crates/kernel/Spec.lean` §8-4）。
- **一段会话是房间上的一段，不是房间本身**：`/new` 不换地址、不换身份。市长身份按 `hall/mayor` 精确匹配（`city::spine_files::hall`），换到 `hall/mayor-2` 就把身份丢了。
- **继承进的是 window，不是 prefix**（S2 改写了路线图早先那句话，理由记在这里）。四段 prefix 各是一份**文档**（`PrefixPlan` 的 `SourceDoc`），由人写、可编辑、逐字节哈希；而一段对话是模型说了什么、工具答了什么，`Window` 自己的文档就写着「frozen prefix 的字节永不落在这里」。把历史塞进 prefix 要有第五个槽位（而 prefix 是四段的类型），会让 `prompt_assembled` 声称历史属于它并不属于的那一段，还会让被缓存的前缀每回合都长。**所以 `RunPlan.inherited` 是 window 的材料**（`run/lifecycle.rs` 在开场任务之前推入），而它可重建的证据不是段哈希而是 `run_forked { from, at_seq }` 加上母亲自己的那些行（runtime §8-2 的 `inherited`）。
- **一份继承只属于开这一段的那一跑。** 房间的当前一段从 `session_opened` 带上来的 `from` 落在折叠里（`accounting::worker::folds::session`），第一次派活取走它并写下 `run_forked`（这一行同时也把「用掉了」记进折叠）；同一段里的第二次派活不再继承。一个分支是一个开头，而开头的那一跑就是继承的那一跑。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。这一验只读 `from` 指的那一行：worker 常驻一份 `storage::LedgerIndex`，验之前 `refresh`（只读上次之后追加的字节），再 `line_at(at_seq)` 取那一行、解出它属于哪条 run。不走 `runtime::replay::verify_ledger_dir`，因为那一步把整本历史读进内存、逐行验链再解析——94 MB 的账本上是一次 +67 MiB 的瞬时内存和几十毫秒，只为回答一行的归属；而 worker 是这本账唯一的写者，打开时已经验过它（`JsonlLedger::open` 的尾部恢复），整链的校验属于 replay 与 `verify`。被否决的备选：每次验前重建索引——它仍然扫全部段。
- **`--carry` 带摘要，也带上一跑对话的地址。** 摘要说找到了什么，只有 transcript 说是怎么找到的；接手的 run 本来就在 run 槽位里读到 `Predecessor transcript: <room>/<run>.jsonl`，带过来的一段的第一跑同样读到，指向这个房间里最后开始的那一跑。这份「欠着的前任」与 `from` 同住 `accounting::worker::folds::session`：折叠按 `run_started` 记下每个房间最后开始的 run（重建时读这一行，在世的 worker 在 `freeze_plan` 冻好一跑时自己记，因为 runtime 写的那一行它看不见），`session_opened { carried: true }` 把那一跑记为欠着的前任，`carried: false` 清掉它，这一段的第一跑开始即用掉。不往 `session_opened` 里加字段：前任是哪一跑已由历史里的 `run_started` 决定，payload 再写一份就是第二个家。被否决的备选：`/new` 时把整本账重验一遍去找最后一跑——一次人按下的命令，其代价随城的历史线性增长。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。这一验只读 `from` 指的那一行：worker 常驻一份 `storage::LedgerIndex`，验之前 `refresh`（只读上次之后追加的字节），再 `line_at(at_seq)` 取那一行、解出它属于哪条 run。不走 `runtime::replay::verify_ledger_dir`，因为那一步把整本历史读进内存、逐行验链再解析——94 MB 的账本上是一次 +67 MiB 的瞬时内存和几十毫秒，只为回答一行的归属；而 worker 是这本账唯一的写者，打开时已经验过它（`JsonlLedger::open` 的尾部恢复），整链的校验属于 replay 与 `verify`。被否决的备选：每次验前重建索引——它仍然扫全部段。两条测试（`checking_a_branch_origin_does_not_verify_the_history` 与分支重建的 `inheriting_a_branch_does_not_verify_the_history`）以一本 genesis 之后断了链的账本作证：`verify_ledger_dir` 拒它，这一验与这次重建照常作答。被否决的读数：拿一次验与一次 verify 的墙钟之比——它量的是跑测试那台机器的负载，并行跑时会翻转。
- **`Carry::Nothing` 必须真的清掉 `Handoff.md` 的槽位**：`accounting::worker::freezing` 无条件读 `city::handoff(root, room)` 并把它折进下一个 run 的 prompt，只清配置而留着文件，新一段仍会继承上一段的摘要，于是开关不起作用（`crates/city/Spec.lean` §8-14b 拥有那一步）。
- **默认不带，理由是 `/new` 对人意味着什么** ——「在这个工作区新开一个会话」，：`/new` 对人意味着「在这个工作区新开一个会话」，带上上一段的摘要是需要说出来的例外；**没有交接时 `--carry` 不弹问、不拒绝**，事件里如实写 `carried: false`——一段新会话就是人要的那件事，没有理由因为交接槽位空着而拒他。被否决的备选：默认带、`--fresh` 不带（路线图早先的建议）——它把例外当成了常态，而且换模型后的新一段仍受旧摘要影响。
- **拒绝只有一条**：地址上有活跃 run → `E_BUSY`，恢复语「先 `/stop`，再 `/new`」。一次派活正在写这一段的形状时把它换掉，等于让两个 run 各自以为冻的是同一份前缀。
- **分叉是同一件事多一个起点**：S2 把它做成 `OpenSession { from: Option<Origin> }`，而不是第二个动词；`Fork` 帧的退休随 S2 一起落地。
- **客户端走同一条路**：`core/commands.ts` 的 `openSession(addr, carry)`；`core/slash.ts` 的 `/new [--carry]`；composer 上方那行的「新对话」按钮；以及拒绝通知上的动作（§8-4-1 的那句话从此指向一个真存在的动词）。

**本章测试**（路线图原先写的 citysim `session_rotates.toml` 没有落点：citysim 的场景是 Rust 结构体，不是 toml，而且这条链路——派活被拒、`/new`、再派活——属于动词所在的 `sprawling`，不属于 runtime 的回合循环；改记在这里）：
- `accounting::worker::dispatching::session_shape::tests::a_new_session_lets_the_room_use_the_model_chosen_since`：dispatch（模型 A）→ `select_model`（B）→ dispatch 被 `E_CONFIG_INVALID` 拒 → `open_session` → dispatch 成功，且房间这次冻的是 B。
- `accounting::worker::commanding::sessions::tests` 的五条：房间里有 run 工作时 `E_BUSY` 且房间一字未动；`Nothing` 清形状也清槽位、事件写 `carried: false`；`Handoff` 留摘要、照样清形状、事件写 `carried: true`；没有摘要时 `--carry` 不拒也不撒谎；带过来的一段的第一跑读到上一跑 transcript 的地址（`a_carried_session_names_the_previous_runs_transcript`）。

**本章验收**：`cargo nextest run -p sprawling -p sprawling-city -p sprawling-wire -p sprawling-kernel` 绿；`cargo xtask wire-ts`、`wiring`、`specalign` 绿。

### 8-141 分支的第一个请求接着母 run 最后一个请求的字节（`accounting::worker::freezing`、`runtime::fork`；`crates/runtime/Spec.lean` §8-58）

**要什么。** provider 只为逐字节相同的前缀复用缓存：分支的第一个请求要以母 run 最后一个请求的字节开头——冻结前缀的四段，经 gateway 按兼容格式渲染之后的整份请求体，再到母 run 发过的每一条消息。消息那一半由 runtime 按 `run_started.opening` 重建（`crates/runtime/Spec.lean` §8-58）；前缀那一半由 `accounting::worker::freezing` 为分支重新组装，本节判两半合在一起、经真实渲染之后的字节。

**同一间房、`Carry::Nothing`、两次都是与人交谈的派活时，字节相同。** city 段、building 段、resident 段读的是同一组文件与同一版冻下的身份；`Carry::Nothing` 清掉了房间的 `Handoff.md`，没有 `--carry` 的前任，所以 run 段只剩 `city::RunBrief::Principal` 那一句，两次相同；模型、上限与工具表也相同。

**设计上就不同的三种，不改，理由在各自的规则里。**

1. 母 run 以 `FromJob` 开篇：第一条消息改写，理由在 `crates/runtime/Spec.lean` §8-58。
2. 分支开在另一间房：building 段以房间地址开头（`freezing::building_segment`），房间换了，从这一段起就不同；分支在哪间房是人的选择。
3. `Carry::Handoff` 或派活带了目标：run 段带上交接或分支自己的 JOB.md，run 段不同；这一段本来就不进缓存断点（`BreakpointPlan::marks_edge`），它的字节是这一次 run 自己的任务。

- 验收：`accounting::worker::freezing::tests::lineage` 的 `a_branch_first_request_carries_the_bytes_of_the_mothers_last`：一座真城、一个记下每个请求体的假 provider；母 run 在 `lab/room1` 答一句，`OpenSession { carry: Nothing, from: 母 run 的最后一行 }` 之后同一间房再派一次活；分支第一个请求体去掉 `messages` 之外的每个键与母 run 最后一个请求体相同，`messages` 以母 run 的 `messages` 开头。

### 8-83 客户端包落在 `sprawling` 包里的 `web-dist`，与 cargo 的输出目录无关（`build.rs` 的 `BUNDLE_DIR`）

**原因**：「客户端包在哪」有三个读者：`client/vite.config.ts` 写它，`build.rs` 嵌入它，`xtask::bundle::dist` 交给 `render`、`budget` 与 `shots` 去开。三者各自推导位置时，只要一个跟着 `CARGO_TARGET_DIR` 走、另一个不跟，`just build-web` 写出的真包就没人嵌入，二进制带着占位页通过构建，只留一条 cargo warning。而 crates.io 上的 `.crate` 只装这个包目录里的文件，包体在工作区的 `target/` 下时，从 crates.io 构建的每一份二进制都只有占位页（§8-157）。

- **权威是 `build.rs` 的 `const BUNDLE_DIR: &str = "web-dist"`，值是相对本包目录、以 `/` 分段的路径。** `build.rs` 按 `CARGO_MANIFEST_DIR` 接上它，在检出里、在 `target/package/` 的验证目录里、在 registry 解开的包里都指向同一个相对位置。`xtask::bundle` 用 `syn` 读这个常量，再接上 `build.rs` 所在的目录，得出仓库相对的整条路径 `crates/sprawling/web-dist`（tools/xtask/Spec.lean §8-18）；`artifact` 门要求 `client/vite.config.ts` 与 `justfile` 拼出这条路径。
- **`build.rs` 不读 `CARGO_TARGET_DIR`。** 客户端包是 bun 的产物，不是 cargo 的产物；它的位置由写它的那一步决定，与 cargo 把编译产物放在哪无关。
- **`.gitignore` 挡着它，清单的 `include` 把它收进包。** 有 `include` 时 cargo 按这张表遍历包目录，不再问 git，所以被忽略的 `web-dist/` 照样进 `.crate`；没有 `include` 时 cargo 跳过 git 忽略的文件，包体进不了包。
- **生成的 `client_embed.rs` 带常量 `CLIENT_BUNDLE_DIR`**，值取自 `BUNDLE_DIR`。`serve` 在只有占位页时提示 `--web-dir`，说这个目录在 `sprawling` 包里叫什么，不再手写一份。
- **被否决的备选**：让 vite 读 `CARGO_TARGET_DIR`，由 justfile 注入。那样每个读者都要复刻 cargo 解析目标目录的规则：环境变量、`.cargo/config.toml` 的 `build.target-dir`、相对路径按当前目录解析，而这条规则会在 TypeScript、`build.rs`、`xtask` 里各有一份。重开条件：客户端包改由 cargo 自己构建（例如 wasm 客户端在 `build.rs` 里编译），那时它才真是 cargo 的产物。

**本章测试**：`main::tests::the_embedded_client_is_the_bundle_the_workspace_built`——本包 `web-dist` 下有完整的包时，嵌入表的路径集合与盘上的文件集合相等，且 `CLIENT_COMPLETE` 为真；没有完整的包时，`CLIENT_COMPLETE` 为假。这条测试只在 `CARGO_TARGET_DIR` 指向工作区以外时才能区分对错。

### 8-157 从 crates.io 构建的二进制与发行归档是同一个东西（`build.rs`、`bin::doctor::pin`、各包清单）

**原因**：发行渠道有三条：GitHub 的归档、npm 包、crates.io。前两条装的是 `release.yml` 编出的同一份二进制；第三条在装的人自己的机器上从 `.crate` 编译，而 `.crate` 里只有一个包目录。凡按层数往上数到包目录之外去找的东西——锁、包体、模板、工具链文件——在 `cargo package` 的验证目录与 registry 解开的目录里都不存在：找锁落空让构建失败，找包体落空只嵌入占位页，`include_str!` 落空则编译失败；而不是默认 feature 的执行引擎，`cargo install` 不会带上，装出来的二进制拒绝每一次 exec。

```rust
// build.rs
const BUNDLE_DIR: &str = "web-dist";                           // §8-83
/// 从本包目录起往上，第一个含 `Cargo.lock` 的目录：检出里是工作区根，打出来的包里是包自己。
fn checkout_root(manifest: &Path) -> Result<PathBuf, String>;
/// 开发这份代码所钉的三份文件，相对检出的根；常量名是 OUT_DIR 下 pins.rs 里的名字。
const PINS: [(&str, &str); 3] = [
    ("RUST_TOOLCHAIN_FILE", "rust-toolchain.toml"),
    ("LEAN_TOOLCHAIN_FILE", "lean-toolchain"),
    ("ZIG_VERSION_FILE", "crates/desktop/ffi/zig-version"),
];
// OUT_DIR 下的 pins.rs（build.rs 写，bin::doctor::pin 用 include! 读）
pub(crate) const RUST_TOOLCHAIN_FILE: &str;   // 那份文件的全文；构建没找到它时为空串
pub(crate) const LEAN_TOOLCHAIN_FILE: &str;
pub(crate) const ZIG_VERSION_FILE: &str;
// bin::doctor::pin
pub(crate) const LEAN_TOOLCHAIN: &str;        // LEAN_TOOLCHAIN_FILE 去掉行尾
pub(crate) const ZIG_VERSION: &str;           // ZIG_VERSION_FILE 去掉行尾
pub(crate) fn pinned(pin: Pin) -> Option<String>;   // 文件为空即 None，与 Pin::Unpinned 同答
```

- **锁按 cargo 的规则找。** cargo 把 `Cargo.lock` 写在工作区根，`cargo package` 把它放进包的根（cargo-package 文档：「Cargo.lock is always included」）。所以「往上第一个含 `Cargo.lock` 的目录」在检出里是工作区根，在 `target/package/sprawling-<版本>/` 与 registry 解开的目录里是包自己，用不着按层数往上数。整条链上都没有锁时 `build.rs` 以 `cargo::error` 失败，与此前读不到锁时一样：没有锁的构建说不出自己由哪些包组成。
- **包体进包**：见 §8-83。`.crate` 的上限是 10 MB（cargo 的 publishing 文档），包体压缩前约 1.5 MB。
- **城写下的文档模板归 city。** 模板与 `City.md` 住在 `crates/city/templates/`：它们是城立城、建楼、开会话时写下的第一批字节，`city::spine_files` 与 `city::building` 按包内路径 `include_str!` 它们；accounting 立城时写的 `City.md` 读 `city::CITY_TEMPLATE`，不伸手到别的包目录里（`crates/city/Spec.lean` §8-41）。
- **工具链钉子由构建脚本找，找不到就不钉。** `doctor` 的 develop 层报「钉住的版本」，读的是检出根上的 `rust-toolchain.toml`、`lean-toolchain` 与 `crates/desktop/ffi/zig-version`。这三份文件不在本包里，从包里构建时它们不存在，所以 `build.rs` 在 `checkout_root` 下找它们，把全文写进 `OUT_DIR` 下的 `pins.rs`；只有「不存在」读成空串，别的读失败仍是 `cargo::error`。空串即不钉：`pinned` 答 `None`，页面不画钉住的版本，探测按空前缀接受任何一版。
- **`packaged` 门守这条线**（tools/xtask/Spec.lean §8-49）：可发布的包的生产代码里，`include!`、`include_str!`、`include_bytes!` 只指向包目录之内或 `OUT_DIR`。
- **清单**：`[workspace.package]` 写 `repository`、`homepage`，`publish = true`；每个包写自己的 `description`，本包另写 `readme`、`keywords`、`categories` 与 `include`；`xtask` 与 `citysim` 写 `publish = false`。工作区自己的包在 `[workspace.dependencies]` 里各钉 `version = "=<工作区版本>"`，`guard` 判它们等于 `[workspace.package] version`（tools/xtask/Spec.lean §8-49）。`sprawling-remote-access` 被二进制链接，随之可发布。`sprawling-desktop-ffi` 也可发布：`sprawling-desktop` 在 Windows 上依赖它，而 crates.io 要求依赖的每个包都在 registry 上；它的包里带着 Zig 叶子的源码与 `zig-version`，构建脚本只读包内的文件，所以从 crates.io 在 Windows 上装这个二进制要先装钉住的那一版 Zig（§8-146），别的平台不编叶子。
- **`sandbox` 是默认 feature。** `cargo install sprawling` 不写 `--features` 时也带执行引擎，与归档一致；不要引擎的构建写 `--no-default-features`，`just features` 编译这一份，因为别的命令都不再编它。
- **发布次序**：`release.yml` 的 `crates` job 在 `channel`（npm）之后跑，也就排在 GitHub release 之后。它用 `rust-lang/crates-io-auth-action` 把这次运行的 OIDC 令牌换成 crates.io 的短期令牌（Trusted Publishing，每个可发布的包在 crates.io 上登记了仓库 `2youg1/sprawling-agents` 与工作流 `release.yml`），然后 `cargo publish --workspace --locked --no-verify --allow-dirty`；cargo 按依赖次序逐个发布，desktop 与它的叶子都是工作区成员，不再单独发。仓库里不存长期令牌。crates.io 的版本不能覆盖，而 `release.yml` 允许同一个 tag 重新发版，所以 crates.io 排在最后；本包这个版本已在 sparse index 上时，job 不再发布、只留一行说明。`--no-verify`：同一棵树已经过 `verify` 与 `packaged` 门（D10），验证构建只是重编一遍，还会耗掉短期令牌的时效。`--allow-dirty`：job 改写了 binstall 的下载地址（D32）。
- **`cargo binstall sprawling` 取发行归档**：本包清单的 `[package.metadata.binstall]` 按目标三元组各写一条 `pkg-url` 与 `bin-dir`，指向 `release.yml` 打出的三份归档（`x86_64-pc-windows-msvc` → `-windows-x86_64.zip`，`aarch64-apple-darwin` → `-macos-aarch64.zip`，`x86_64-unknown-linux-musl` → `-x86_64-unknown-linux-musl.zip`，归档名的后缀表在 `tools/xtask/src/platform.rs`），`pkg-fmt = "zip"`，可执行文件在归档里的 `sprawling-<版本>-<后缀>/` 目录下。glibc 的 Linux 上 binstall 自己退到 musl 那一份；没有归档的目标，binstall 退回从 `.crate` 编译。仓库里的地址用 `v{ version }` 作 tag，`crates` job 发布前把它换成这次的 tag（D32）。

**本节接口的当前状态**：从 crates.io 构建的二进制仍有两处与归档不同。`[profile.release]` 写在工作区清单里，`cargo package` 不把它带进包，`cargo install` 按 cargo 的默认 release profile 编（`opt-level = 3`，不做 fat LTO，不剥符号）；`SPRAWLING_RELEASE_TAG` 只有 `release.yml` 设，所以 `status` 如实自称 built from source，成熟度照样从 `kernel::release::MATURITY` 读（§8-162）。工具链钉子不在包里时，develop 层的 `lean` 与 `zig` 两行装不钉的版本（§8-162）。

**本章测试**：`doctor::pin::tests` 读出的钉子与检出里的文件相等，空文件读成不钉；`cargo package -p sprawling --list --allow-dirty` 的列表里有包体的 `index.html`（在 `web-dist` 下）与 `Cargo.lock`；`cargo xtask gates packaged guard` 为绿；`cargo publish --workspace --dry-run --locked` 走完打包与验证构建；把清单里的 `v{ version }` 换成一个已发布的 tag 之后，`cargo binstall --dry-run --manifest-path crates/sprawling/Cargo.toml sprawling` 解析到那个 tag 下真实存在的归档地址。

### 8-162 成熟度只有一处，不钉的构建装不钉的版本（`main::version`、`bin::doctor::table::toolchain`）

**原因**：`status` 的第一行曾自己写着 `(pre-alpha)`，tag 的中缀、README 两份与 CHANGELOG 又各写一遍；进 alpha 时漏掉的那一处会照旧说 pre-alpha，而没有一道门看得见。成熟度现在只写在 `kernel::release::MATURITY`（kernel D18），本 crate 是它的一个读者。另一半是 §8-157 留下的：从 `.crate` 构建时三份钉子文件都不在，`lean` 与 `zig` 两行的装法拼出一个空参数，elan 与 winget 都会拒绝这条命令。

```rust
// main::version
/// `sprawling 0.0.8 (pre-alpha), built from source`：`status` 打印的第一行。
/// 生产路径传 `kernel::release::MATURITY`；参数让一条测试看得见成熟度换成 alpha 时这一行跟着变。
pub(super) fn headline(maturity: kernel::Maturity) -> String;

// bin::doctor::table::toolchain
/// 交给 `elan toolchain install` 的那一个：钉住的工具链，构建没找到 `lean-toolchain` 时是 `stable`。
const fn lean_install(pin: &'static str) -> &'static str;
/// winget 装 Zig 的参数：钉子非空时带 `--version <钉子>`，为空时不带这两个参数。
macro_rules! winget_zig { ($pin:expr) => { /* &'static [&'static str] */ } }
```

- **`status` 读常量，不写字面。** 版本行是 `sprawling <CARGO_PKG_VERSION> (<MATURITY.word()>)<发布日期或 built from source>`；tag 的中缀与文档的字样是同一个常量的另外几个读者（kernel D18、tools/xtask/Spec.lean §8-16 的 `maturity` 事实）。
- **钉子为空时装不钉的版本。** `lean` 装 `stable`，那是 elan 认的通道名；`zig` 在 Windows 上去掉 `--version` 与它的值，winget 装它给的那一版；macOS 与 Linux 两列本来就不带版本，不变。探测不变：空前缀接受任何一版（§8-157）。
- **winget 的参数用宏写，不用 `const fn`。** 一条装法的参数是 `&'static [&'static str]`，`const fn` 拿自己的参数造不出一段 `'static` 的切片；宏在 `const` 项里展开，钉子是常量，切片照常提升。测试用同一个宏展开空钉子，所以它判的是生产那一行的写法，而不是一份抄本。

**本章测试**：`main::version::tests::the_status_line_says_the_maturity_it_is_given`——给 `Maturity::Alpha` 时版本行说 `(alpha)`；kernel 的 `release::tests::an_alpha_build_cuts_and_reads_alpha_tags` 是同一次挪动在 tag 一侧的读者。`doctor::table::toolchain::tests::a_build_without_pins_installs_lean_stable_and_any_zig`——空钉子时 `lean_install` 给 `stable`，`winget_zig!` 不带 `--version`，也没有空参数；钉子非空时两者都带着钉子。

### 8-84 记账线程的循环是一个有名字的函数，两件仪表直接驱动它（`accounting::worker::attend::attend`、`accounting::worker::driving::tests::instruments`）

```rust
// accounting::worker::attend —— shape: adapter
/// 记账线程的主循环：relay 请求、至多一轮回家的活、desk，按这个次序（§8-42-4），直到 desk 关门。
pub(crate) fn attend(worker: &mut RunWorker, desk: &CommandDesk);

// accounting::worker::driving::flight —— 只在测试构建里有
#[cfg(test)]
pub(in crate::assembly) fn measuring_relay(&self) -> Relay;   // 与车道同一个 gate 发出的写面
```

**为什么把循环从闭包里拿出来**：仪表要驱动的是生产在跑的那个循环本身。一份抄来的 relay 形状，只要记账侧与生产的等法有一处不同，量出的就是抄件：生产的循环在两个定时等待之间轮询时，抄件量出 5 µs 一次往返，同一次往返在服务中的城里是 31.7 ms。所以 citysim 不再留那份抄件，多 run 并行这一类负载就由 `instrument_relay_round_trip` 量（citysim D18）。循环只要还有第二份写法，仪表就会量错对象。`spawn_worker` 装好 `Serving` 与观察者之后调用 `attend`，这是它唯一的生产调用者。

**两件仪表**，都在 `accounting::worker::driving::tests::instruments`，都标 `#[ignore]`：它们量墙钟，一次要跑十几秒，不属于 `just check`；`just bench` 在 citysim 那一行之后跑它们（`cargo nextest run -p sprawling --release --run-ignored only -E 'test(/::instrument_/)' --no-capture`）。两件都经过同一套生产部件：`attend` 跑在自己的线程上，命令经 `CommandDesk::post` 进门，模型是回环上的假 provider（`fixture::provider`，带一个 `pace` 钩子决定何时作答）。

| 仪表 | 场景 | 读数 |
|---|---|---|
| `instrument_relay_round_trip` | 一轮活停在它的第一次模型调用上（provider 不作答），于是循环处在「有车道在跑」的那个形状里；另一条线程拿 `measuring_relay` 连续追加 200 条，逐条计时 | `store=disk`：城自己的 `JsonlLedger`，每条一道屏障；`store=memory`：同一个 `JsonlLedger` 开在 `storage::FaultFs` 上，屏障是一次内存拷贝；另给 `store=memory` 不过河时自己的追加耗时，两者之差就是过河本身 |
| `instrument_dispatch_gap` | 移植自 perf-latency 的双派活场景：run A 在 `lab/east` 跑 30 个 `status` 回合；A 的第五次模型调用到达时，往楼 `lab`（不带房间，按 §8-86 的规则取名）派 run B；provider 把任何含取名提示的调用压 3 s，所以一旦派活重新向模型要名字，读数会显示出来 | A 相邻两条记录 `t` 之差的最大值与中位数，单位 ms |

`instrument_relay_round_trip` 另断言内存存储过河的 p50 不超过 1 ms（`ROUND_TRIP_P50`）：那条往返里除了一次内存拷贝全是 harness，所以它量的就是 harness。磁盘存储只报读数不断言：它的中位数是设备的 fsync，这是物理下限，因机器而异，一个写死的毫秒数只对一类机器成立；它与内存那一行之差才是磁盘的份额。目标还有一条没写成断言：内存存储过河的 p50 不超过 10 µs，它只在 `--release` 下有意义，而 `crossing=none` 那一行在调试构建里已是 20 µs 量级。

读数行由仪表模块自己渲染，一行一个读数：`<仪表> <键=值>… machine=<os>-<arch>, <n> core(s)`，每行都带 `samples`、`floor_us`、`p50_us`（空档那一行是 `max_ms`、`median_ms`）。它不用 citysim 的 `perf load=…` 文法：那份文法属于 citysim 的 bench Main，本 crate 够不到它。`instrument_relay_round_trip` 是四个负载场景里多 run 并行那一个的读数；`instrument_dispatch_gap` 不属于四个负载场景。

**决定**：仪表放在 crate 内的测试里，而不是给 citysim 开一扇公共门。relay、`serve_flight` 与 desk 都是 `pub(crate)`；为量它们而开的公共面没有生产调用者。**败给的方案**：citysim 经 `RunWorker::handle(Dispatch)` 从外面驱动，再用 provider 两次请求之间的空隙推算 relay 往返。那个空隙里还有检查点（每波 20–90 ms）与工具，推算出来的是每回合剩余，不是一次往返。

**重开参数**：两件仪表只经过 desk、relay 与 provider 三个面；`attend` 的等法再怎么改，只要这三个面不变，仪表就不用改。

### 8-85 每个模型一段追加提示词（`accounting::worker::freezing::model_note`，形状 4 适配器）

同一套城规、楼规与身份，交给不同的模型时常常要补一两句只对那个模型说的话（例如某个模型爱省略测试输出，某个模型需要被提醒先读再改）。这段话放在城里一处、按 provider 与模型分文件：

```rust
pub(super) const MODELS_DIR: &str = "models";            // <city>/.sprawling/models/<provider>/<model>.md
pub(super) struct ModelNote { pub(super) at: Address, pub(super) bytes: Vec<u8> }
/// 这次派活选中的 endpoint 名与模型 id 下的那段话；文件不在即 None。
pub(super) fn model_note(city_root: &Path, provider: &str, model: &str) -> Result<Option<ModelNote>, AxError>;
```

- **位置只有一处，在城一级的保留区里**：`<city>/.sprawling/models/<provider>/<model>.md`，`<provider>` 是派活时选中的 endpoint 名（`AttachEndpoint` 的 `name`），`<model>` 是模型 id 原样。放在保留区而不是城根，是因为城根下的目录名就是楼的地址，一栋叫 `models` 的楼会与它撞名；保留区也是居民写不到的地方，一段约束模型的话不该由被约束的模型改写。楼一级不设第二处：两处就要回答「楼的那段覆盖还是追加城的那段」，而今天没有一个读者需要这个区分。
- **追加在 run 段末尾，因而在整段 system prompt 的末尾。** 与楼段接 `AGENTS.md` 走同一扇门（`Assembled::extend`：空一行接上，记一行来源）。它随 run 段一同冻结、一同入库，整个 run 内字节不动，所以每一回合的缓存前缀都盖得住它；`prompt_assembled` 的 run 段哈希盖住它的字节，来源行写出它的地址与长度，WebUI 的提示词视图（§8-67）因此照原样列出「这个 run 用的是哪个模型的哪一段」。重放读的是账本里那一行，不重新读文件，故逐字节一致。
- **文件不在，行为逐字节与没有这个功能时相同**：run 段不多一个字节，来源表不多一行。
- **文件在却读不出**（权限、目录占位、不是文件）是 `E_STORAGE_FATAL`，指名那条路径，派活被拒；不静默跳过，理由与 `city_segment` 相同：人写下的那段话静静躺在盘上没被读，而 run 照着缺了它的提示词干活，没有人会被告知。
- **地址先于路径。** 位置先按地址文法拼出（`Address::parse`），再落到盘上。模型 id 若含地址文法拒绝的字符（`:`、`\`、`.`／`..` 段），它在城里就没有可指名的那段话，按「没有文件」处理；先拼路径再读则会让一个含 `..` 的 id 读到保留区以外的文件。
- **被否决的备选**：把它做成第五段。四段恒四是 `FrozenPrefix` 的类型（`crates/runtime/Spec.lean` §8-4），`verified_system_hashes` 与 `rebuild_prefix` 都按四段对拍；为一段随模型而变、随 run 冻结的文字加一个槽位，要改的是三个 crate 的冻结不变量，换来的只是一个本来就能从来源行读出的边界。重开条件：有读者需要按模型单独缓存这段话（例如跨 run 的缓存断点放在它之前）。

**本章测试**：`freezing::tests::model_note::a_note_for_the_chosen_model_ends_the_system_prompt`——为 `house/m-local` 放一个文件，派活发出的请求里最后一条 system 消息以这段文字结尾；`freezing::tests::model_note::a_note_for_another_model_is_not_sent`——别的模型的那段话不出现。

### 8-86 房间名按规则取，起名不调用模型（`accounting::worker::dispatching::session`）

```rust
// accounting::worker::dispatching::session —— shape: decision（`session_for`、`rule_name`）
/// 人给了名字、或地址已含房间时原样返回；地址只有一段且没有名字时返回 `rule_name(task)`。不调用模型。
pub(super) fn session_for(addr: &Address, session: Option<SessionName>, task: &str) -> Result<Option<SessionName>, AxError>;
/// 任务原文里前四个 ASCII 字母数字词，小写，用 `-` 连起来；一个也没有、或拼出保留名时是 `work`。
/// 结果总是合法的 session 名；`SessionName::parse` 仍是判定者，所以签名带着它的错误。
pub(super) fn rule_name(task: &str) -> Result<SessionName, AxError>;
```

**规则**：派到一栋楼（地址只有一段、旁边没有 session）上的活，房间名由 `rule_name` 从任务原文里取：按非 ASCII 字母数字字符切词，丢掉空词，取前四个，转小写，用 `-` 连起来；每个词最多留前 15 个字符（`RULE_WORD_MAX`），所以四个词加三个连字符最长 63 个字符，总在 `SessionName::parse` 的上限 64 之内，规则名永远是合法的 session 名。一个 ASCII 词也没有、或这些词拼出城自己保留的名字而被 `SessionName::parse` 拒绝时，名字是 `work`。重名由 `city::open_room` 加序号（`work`、`work-2`……），规则本身不查盘。

**为什么**：模型的回答不受字符集约束，而 `SessionName::parse` 接受任何不含分隔符的文字：一个回了「收到。」的模型会让 run 落进人找不到的房间 `shop/收到。`。问模型要名字还挡在首字前面：派活要先等一次完整的非流式调用，再发出 run 的请求。规则名在记账线程上用几微秒算出，不读 book、不读 vault、不起线程，派活帧到达后发出的第一条 provider 请求就是 run 本身（`a_bare_building_is_named_by_rule_and_the_run_is_the_first_call`）。机密楼的任务原文也不会为了起名被发给任何模型。

**决定**：完全不调用模型起名。**败给的方案**：保留模型起名，放到 run 开始之后，再校验长度与字符集、失败时回落到规则名。那样房间要么在 run 开始后改名（房间是 dispatch 在盘上写的第一件事，ARCHITECTURE §5 第 3 步，run 的文件已经在里面），要么多出一个只为改名存在的线程与在飞计数；一个人要一个好找的名字时，发到 `building/name` 就有。只含非 ASCII 文字的任务都叫 `work`、`work-2`，这是这条规则的代价；让它重新值得调用模型的参数是：一个名字能在 run 开始前、不增加首字延迟地取到。

### 8-87 粘进派活里的 key 进 vault，文字里只留引用（`accounting::worker::dispatching::custody`）

**原因**：人在页面上把一把 provider key 粘进任务或目标时，这段文字原样进了三处：发给模型的请求、账本里的派活记录、房间里的 `JOB.md`。三处都能被城里的居民 `grep` 到，账本还会随城搬走；而 `kernel::secret::scan` 早已能认出这些形状，只是派活这道门从不问它。

```rust
// accounting::worker::dispatching::custody（形状 4 适配器）
impl RunWorker {
    /// 把文字里每一段 provider 形状的 key 存进 vault，原处换成它的 `secret:` 引用。
    pub(in crate::assembly) fn take_custody(&mut self, text: String) -> Result<String, AxError>;
}
```

- **一道门，一处权威**：调用点是 `stage_dispatch` 里 `agree_to_work` 之后、`session_for` 之前。人打的派活、外面敲门的唤醒、计划推进起的活都经过这里，所以三种入口不会各有一套规矩；放在同意之后，是因为存 key 是一次写入，而城不肯接的活什么都不写；放在命名之前，是因为命名要把任务文字发给摘要模型。
- **认什么由 `kernel::secret::scan` 定**：只换形状表命中的段（`SecretSpan::provider` 为 `Some`）。熵命中不换：提示里的提交哈希、校验和也是高熵串，换掉它们，居民就读不到它要处理的那个值，而形状表列出的前缀不会出现在这类值里。
- **存进现有的 `gateway::Custodian`**：它按机器选后端——系统的凭据服务，或用口令派生密钥、逐条 ChaCha20-Poly1305 加密的 `encrypted-file`（`crates/gateway/Spec.lean` §8-21）。另起一把 AES-GCM 密钥就是同一件事的第二个家，也会多出一处密钥要保管。
- **引用是 `secret:pasted/<provider>-<seq>`**，`<seq>` 是存这把 key 时账本的下一个序号。每存一把 key 都追加一条 `secret_captured`（`origin: "pasted"`），所以序号不会重复：同一毫秒的两次派活也不会让后一把覆盖前一把。名字里不放时间，也不放随机数，citysim 回放时仍逐字节一致；也不放 key 的哈希或尾巴，`secret_captured` 的规矩是记录行里不带明文，也不带哈希前缀。
- **失败**：vault 拒绝写入时整个派活失败，错误原样返回（`E_CONFIG_INVALID`，恢复语由 vault 给出）。此时房间和 `JOB.md` 都还没写；如果照原文继续派活，正是这一节要堵的泄露。
- **被否决的备选**：① 在页面上拦下粘贴，让人先去设置页登记——人粘 key 的时候，多半就是要居民用它，拦下来只会让人换个地方再粘一次；② 在请求出门时替换——账本和 `JOB.md` 在出门之前就已写下原文。

**居民手里的工具**：居民往城里写文字、读城里的文字，都经过工作台上的工具：`edit` 写文件，`exec` 的命令行可以写文件，`read`、`search`、`exec` 的输出回到模型。装配台把**每一件**工具登记上 bench 之前包上一层 `accounting::worker::workbench::tools::kept`：

```rust
// accounting::worker::workbench::tools::kept（形状 4 适配器）
pub(in crate::assembly) struct Keeper { /* vault, 摆台时账本的位置, 计数 */ }
pub(in crate::assembly) struct Kept { /* Box<dyn kernel::Tool>, Arc<Keeper> */ }
impl kernel::Tool for Kept {
    /* invoke: 参数里每一个字符串先交给 custody，再交给工具；
       工具的结果里每一个字符串交给 custody，再回到模型 */
}
```

- 同一段替换逻辑：派活文字、工具参数和工具结果都经过 `custody::kept_text`，所以认什么、怎么切、切歪了怎么报错只有一处。
- **参数**：`edit` 的 `new`、`exec` 的命令行，以及其它工具的每一个字符串参数，工具收到的都是引用。所以 `edit` 写进文件的、`exec` 命令行里 `echo … > f` 写进文件的，都只有引用；回给模型的 diff 里也只有引用。
- **结果**：`read` 读到的文件、`exec` 打印的输出里的 key，在回到模型之前换成引用，所以下一个发给模型的请求里没有原文。
- **引用是 `secret:written/<provider>-<pos>-<n>`（参数）和 `secret:output/<provider>-<pos>-<n>`（结果）**：`<pos>` 是装配台摆出时账本的位置，每次派活在摆台之前都写过记录，所以两次派活的 `<pos>` 不同；`<n>` 是这次派活里第几把不同的 key，一次派活的所有工具共用一份记录，所以两件工具不会把两把 key 存到同一个名字下。同一把 key 在同一侧再次出现时交回第一次的引用，不再写 vault：记录按 key 的 BLAKE3 摘要（`kernel::B3Hash::digest`）找到它的引用，所以 vault 随一次派活里不同 key 的个数增长，而不是随工具调用的次数增长；摘要只留在这次派活的内存里，不进账本也不进 vault，`<n>` 就是记录里已有的条数加一，不会绕回。工具运行在 drive 里，拿不到 `&mut RunWorker`，写不了账本，所以不能像派活那样用存 key 时的下一个序号。
- 没有命中的参数和结果原样交出，不复制。
- vault 拒绝写入时：参数里的 key 存不进去，这次调用失败，工具不运行，文件不动；结果里的 key 存不进去，工具的效果已经发生，所以调用照常成功，结果交给模型时那把 key 换成 `[key withheld: <错误码>]`（错误码是 vault 给的，比如 `E_STORAGE_FATAL`），原文与引用都不出现。回一个错误会告诉模型这次调用失败，它可能重做一件做过就收不回的事，比如又发一次请求、又写一次文件；换成标记，模型知道那里有一把 key，也知道它为什么拿不到引用。
- 账本里的 `tool_called`、`tool_result`、`model_returned` 本来就经过 `runtime::turn::ledger::Journal::append_redacted`，key 在写进账本之前已换成指纹标记；这一层管的是工具本身和模型看到的东西。
- **被否决的备选**：① 在 `runtime::EditTool`、`ExecTool` 里各自扫描——那要让 `runtime` 认得 vault，而 vault 属于装配层，也会让每件工具各有一份规矩；包一层就不必改 `runtime` 的公开面。② 只包 `edit`——`exec` 的命令行和每件工具的输出照样把 key 带进文件和请求。

**尚未覆盖**：① 工具存下的 key 没有 `secret_captured` 记录，因为工具拿不到账本；要补就得把引用放进一张 desk，drive 结束后由装配层补记。② `exec` 的命令自己取到的 key（比如从城外的文件复制进来）写进文件时不经过这一层，只有它打印出来的部分会被换掉。③ 模型回复里的工具参数原样回到后续请求：模型只可能写出它自己编出来的 key，因为它看到的都是引用。

**本章测试**：`accounting::worker::dispatching::custody::tests::a_pasted_key_reaches_the_vault_and_nothing_else`——任务文字里夹一把 `sk-ant-` 形状的 key 派活，断言：模型收到的每个请求、城目录下的每个文件（账本和 `JOB.md` 都在其中）都不含原文；请求里带着 `secret:pasted/anthropic-…` 引用；vault 按这个引用解出的正是原文。`accounting::worker::workbench::tools::kept::tests::a_written_key_reaches_the_vault_and_not_the_file`——模型用 `edit` 新建一个含 key 的文件，断言：文件里没有原文，只有 `secret:written/anthropic-…` 引用，vault 按这个引用解出原文。`accounting::worker::workbench::tools::kept::tests::a_key_a_tool_reads_reaches_the_vault_and_not_the_model`——城里的文件里有一把 key，模型用 `read` 读它，断言：之后发给模型的请求里没有原文，只有 `secret:output/anthropic-…` 引用，vault 按这个引用解出原文。`accounting::worker::workbench::tools::kept::tests::one_key_seen_twice_is_kept_under_one_reference`——同一把 key 两次交给工具、再交另一把，断言：前两次工具收到同一个引用，第三次收到下一个编号。
-/

/-!
## 8-120 每一项的三个版本：装着的、钉住的、上游最新的（`bin::doctor::upstream`、`bin::doctor::pin`，形状：adapter；`crates/wire/Spec.lean` §8-50）

一个人打开依赖页，要知道的不只是「有没有」，还有「是不是旧了」。装着的版本由探测读出（`DoctorVersion::Said`），钉住的版本写在仓库的两份文件里，上游最新的版本只有发布它的人知道。

```rust
// bin::doctor（Requirement 多三个字段，每一行都写明）
pub(crate) enum Pack { RustTools }                         // §8-58：同一包的行在页面上是一行
pub(crate) enum Pin { Unpinned, RustToolchain, LeanToolchain }
pub(crate) enum Upstream { Crate(&'static str), GitHub(&'static str), RustChannel, PythonOrg,
                           Unread(wire::DoctorUnread) }
// bin::doctor::pin：钉子读自仓库里的那一份文件，由构建脚本在编译时读进 OUT_DIR；构建没找到那份文件时，钉子降为不钉（§8-157）
pub(crate) fn pinned(pin: Pin) -> Option<String>;
// bin::doctor::upstream：一项的上游最新版本，出网
pub(crate) fn newest(item: &str) -> wire::DoctorUpstream;   // 不失败、不等：没读过的项起一条线程去读，先答 Asking
```

- **上游只问官方的来源**：cargo 工具与 just 问 crates.io（`/api/v1/crates/<crate>` 的 `crate.max_stable_version`），Rust 问 static.rust-lang.org 上 stable 发布通道的清单里 `[pkg.rust]` 的 `version`，Python 问 python.org 的 release API（取已发布、非预发布版本里最大的一个），其余问项目自己的 GitHub releases 的 `latest`（git-for-windows、rustup、bun、uv、elan、lean4）。版本号取答案里第一个点分数字，于是 `bun-v1.2.3` 与 `v2.55.0.windows.3` 都读成页面比得了的样子。
- **读不到的项说它读不到，并说为什么**：rustfmt 与 clippy 随工具链发布（`WithToolchain`），浏览器一族有好几个牌子各自发布（`ManyBrands`），驱动的版本跟着浏览器走（`MatchesBrowser`），sandbox 引擎是这份代码自己（`ThisProject`），shell、python-wasi 与 ffmpeg 没有一个官方的机读来源（`NoSource`）。网络上失败的一问答 `Refused { said }`，`said` 是停在哪一步。
- **经这座城自己的网络设置出去**：请求由 `gateway::client_for(Proxying::ExceptLocal, url)` 造，与发布检查、模型调用同一条代理规则。
- **页面打开依赖组时才问，每项一问，后台逐个填上，问题本身从不等网络**：一个会话的问题是一个接一个答的（wire 的 `server::socket`），三十来个出网的问题排成一队，会把这一页其余的每一个问题都堵在最慢的那一个后面。所以 `Query::UpstreamVersion { item }` 立即作答：头一次问到某一项时起一条线程去问它的发布者，答 `Asking`；线程把读数留在进程里，页面对还是 `Asking` 的项隔 1.5 秒再问一次，读到为止。线程的请求有 10 秒上限，所以轮询有尽头。这与 `NewestRelease` 的「只在人按下时问」是同一个原则的两种按法：打开依赖页就是人在问自己的工具旧没旧。一次成功的读数在本进程里记住，下一次打开不再出网；失败的读数交出去一次就忘掉，下次打开再问；GitHub 对不带凭据的请求每小时只给 60 次，每开一次页都重问会在一个小时里把它用完。记住的读数不按时间过期，因为本二进制读时钟只在 `bin::assembly` 一处（ARCHITECTURE §10）；城重启即重问。
- **钉子只在一处写**：`Pin::RustToolchain` 读 `rust-toolchain.toml` 的 `channel`，`Pin::LeanToolchain` 读 `lean-toolchain` 冒号之后的部分。改版本只改那一份文件。
- **比较在页面**：`installed < newest` 时那一行标「有更新」。比较按点分数字逐段比，读不出数字的一方不比。
- **打开依赖组即探一次**：页面每次打开这一组发一次 `DoctorRefresh`（§8-54 的同一条命令），不再等人先按「重新检查」；城里已有的答案先画出来，新的一份到了再换上。

**测试**：`doctor::pin::tests` 读出的两个钉子与两份文件相等，空文件读成不钉；`doctor::upstream::tests` 从固定的答案文本里读出版本（crates.io、通道清单、python.org、GitHub 各一份），以及每一行的 `Upstream` 与表对得上；`doctor::tests::the_rust_tools_pack_is_every_cargo_tool_this_repository_calls`。
-/

/-! D6 doctor 的「能做什么」各面自持：终端的英文住需求表，页面的两种语言住 `lang.json`，线上只携 id（`crates/wire/Spec.lean` §8-25）

`Requirement::enables` 是 `sprawling doctor` 那份终端报告的措辞，终端只说英文，这句话与它描述的那一行同住 `bin::doctor::table`，改一行的人在同一处看见它；页面按 `DoctorItem::name` 从 `client/src/lang.json` 的 `machine_enables_<name>`（name 里的连字符写成 `_`，因为词表的键一律 snake_case）取 en 与 zh，与 doctor 其余每一种状态同口径——终端的 `absent` 由 `paint` 拼，页面的 `machine_absent` 由 `lang.json` 给，两面各说各的，线上只携枚举与 id。两面的集合由 `bin::doctor::tests` 的 `every_item_has_the_page_clause_in_both_languages` 钉在一起：需求表多一行而 `lang.json` 没给它词，测试红。**被否掉的**：终端也从 `lang.json` 取英文（二进制在编译期读 `client/` 的一份源文件，每次 doctor 都要解析整张词表，而终端从不说第二种语言）；线上继续携英文句子（页面的词成了服务端的选择，中文读者看到的是英文）。条件变了就重议：终端要说第二种语言时，两面合用一张词表。
-/

/-! D9 Lean 是开发这份代码必需的工具（§8-58）

各 crate 的规格正从 `<crate>-SPEC.md` 迁成 `Spec.lean`（ARCHITECTURE.md §11「Specifications in Lean」），`just check` 里的 `models` 一步是这些规格在本地被证明过的唯一证据。Lean 列为可选时，没装 Lean 的机器上 `models` 静默通过，本地的绿就不再说明规格被证明过，只有 CI 知道。所以 `elan` 与 `lean` 两行是 `required`：缺了它们，`just prereqs` 在编译之前报出来并给出装法，`just models` 自己也报错而不是跳过。被否决的备选：保持可选、只靠 CI 的 `models` job 证明——那样每次本地验证都得另外说明「规格没有证过」，而这句话没有哪道门会替人说。
-/

/-! D10 包体与模板进拥有它们的包，不在发布时组装（§8-83、§8-157）

crates.io 上的 `.crate` 只是一个包目录，所以一个构建要读的每一个文件都放进拥有它的那个包：客户端包落在 `crates/sprawling/web-dist`，城写下的模板与 `City.md` 落在 `crates/city/templates/`，构建脚本按 cargo 找锁的规则找 `Cargo.lock`，开发工具链的钉子在包外，找不到就降为不钉。理由：发布的包与仓库里的包是同一份清单、同一组文件，验证构建就是工作区里那次构建换一个目录，`packaged` 门在每一次 `just check` 里就判出包外的引用，不必等到发布。**被否**：①发布时由 xtask 在临时目录里组装一份改过的包，把包体、模板与钉子复制进去（与 `xtask channel` 组装 npm 包同一种做法）——发布出去的清单与源码对不上，`.cargo_vcs_info.json` 指向的提交编不出那份包，而组装器本身要一套自己的测试；②把各 crate 折成一个 crate 再发布——拆掉 ARCHITECTURE 的 crate 拓扑与每个 `pub(crate)` 边界。**重开参数**：一个要发布的文件不能放进任何一个包（例如两个包都要读的一大份数据），那时由一个包交出常量，另一个包读它，`City.md` 交给 accounting 就是这样做的。
-/

/-! D11 Zig 是在 Windows 上开发这份代码必需的工具（§8-146）

桌面 server 没有准入安全接口的四组 Win32 调用经一片 Zig 叶子（`crates/desktop/Spec.lean` D12），叶子在构建时编译，所以 Windows 上没有 Zig 就编不出这个二进制。`zig` 一行因此是 `required`，版本只读 `crates/desktop/ffi/zig-version`。被否决的备选：把 Zig 叶子预编译成一个提交进树里的静态库——那是一份没人能从源码复现的二进制，`release` 的逐字节重建也就无从谈起。
-/

/-! D12 不钉的构建照列 `lean` 与 `zig`，装不钉的版本（§8-162、§8-157）

从 `.crate` 构建时钉子文件不在，两行的装法改为 `stable` 的 Lean 与 winget 给的那一版 Zig，而不是拼出一个空参数。理由：一条带空参数的命令被安装器拒绝时，人看到的是「装不上」，却不知道是因为构建没带钉子；装一个不钉的版本至少让 `just models` 与 Windows 上的编译跑得起来，版本差异由 doctor 照常报出。被否：①钉子为空时把两行整个藏起来——`Need` 不按构建来源分（§8-58），藏起来的是一个开发这份代码确实需要的工具，那个问题留在 §3；②把钉子文件也打进 `sprawling` 的包——它们属于仓库的开发工具链而不属于二进制，复制一份就是第二个钉子。重开参数：§3 那一条有了答案，或 crates.io 的包里有了钉子。
-/

/-! D31 `rust-toolchain.toml` 钉一个确切的 stable 版本，每个版本开工时升到最新 stable（§8-120、§8-58）

`channel` 写确切版本（现为 1.99.0），不写 `stable`。理由：四个读者要求代码树说出自己的编译器——CI 的每一份 Rust 缓存以这份文件为键，浮动通道下键不变而编译器在底下换了，上游新增一条 lint 就让 `main` 变红，却没有哪个提交可追；flake 的 `toolchain-version-is-derived` 在 `rustc --version` 里找通道名，而它从不印出 `stable`；doctor 按数字比较钉住的、装着的与上游最新的版本（本节「比较在页面」），`stable` 没有数字可比；发行与测量要绑定产出它的编译器，只有代码树写明的版本记得住。贡献者不因此多一步：rustup 在第一次用到时自取这份文件写的版本。被否：`channel = "stable"`——它让贡献者少下载一次，但上面四处各要第二套规则，CI 的红绿也不再只由提交决定。`[workspace.package]` 的 `rust-version` 是另一件事：发布出去的包允许的最低编译器，每个成员都继承它，clippy 的 `incompatible_msrv` 据它判标准库 API，所以它不随钉子一起升，只在代码确实要更新的版本时升。重开参数：CI 改成按发布日解析编译器并把实际版本写进缓存键与读数，或 rust-overlay 与 doctor 都能读浮动通道背后的版本。
-/

/-! D32 binstall 的下载地址在发布时填入 tag（§8-157）

发行的 tag 是 `v<版本>-<成熟度>-<YYMMDD>`，binstall 的模板只认得 `{ version }`、`{ target }` 这类变量，拼不出日期，所以仓库里的 `pkg-url` 写 `releases/download/v{ version }/`，`crates` job 在 `cargo publish` 之前把这一段换成 `${GITHUB_REF_NAME}`，并核对三条地址都换到了。tag 只有一个权威，就是这次运行的 ref。这是 D10 的一个例外：改的只是 cargo 构建时不读的 `[package.metadata]`，包里的源码与清单的其余部分仍与提交一致，`.cargo_vcs_info.json` 照实记下 `dirty`。**被否**：①把整个 tag 写进清单——每次发版要在打 tag 之前猜出日期，清单与 tag 成了同一个事实的两份；②发版时另推一个 `v<版本>` 的 tag 并挂同一组归档——一份归档挂在两个 release 上，`gh release delete` 重发时还要收拾两处；③指向 `releases/latest/download`——本项目的每个 release 都是 pre-release，GitHub 的 latest 不指向它们。**重开参数**：tag 改成 `v<版本>`，那时删掉这一步改写，模板原样可用。
-/

/-! D33 城目录前的扫描经随系统发行的工具读，读不出就说读不出，且只是建议（§8-166 之一）

文件系统名经 sysinfo 的 `Disks` 读（本 crate 已依赖它，对外全是安全接口），Dev Drive 与排除项经 `std::process::Command` 起动 `fsutil` 与 Windows PowerShell，按 AGENTS.md 平台调用次序的第一档。理由：这两样随 Windows 发行，不加依赖，也不要 `unsafe`；它们拒绝普通用户或说本地化的话时，`Untold` 带上那条命令说出读不出，人知道该以管理员再问还是不必在意。这一段从不进 verdict：慢不是坏，一台没有 Dev Drive 的机器照样跑得起这座城。**被否**：①调 `FSCTL_QUERY_PERSISTENT_VOLUME_STATE` 读 `PERSISTENT_VOLUME_STATE_DEV_VOLUME` 位——没有安全接口，要么 `unsafe` Rust 要么一片 Zig 叶子，而一次只传句柄与常量的调用没有 `(ptr, len)` 边界可放在 Zig 后面；②读 Defender 排除项的注册表键——同样要管理员，换不来任何多读到的东西。**重开参数**：一项测量表明在本地化或非管理员的机器上「说不出」占了多数答案，且 FSCTL 能在普通用户的目录句柄上答出 Dev Drive 位。
-/
