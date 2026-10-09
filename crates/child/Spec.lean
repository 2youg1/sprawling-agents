-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # child 的规格

`sprawling-child`（库名 `child`，目录 `crates/child`，不依赖任何 crate）是城起一个子进程时唯一的构造点：`child::command(program)` 交回一个 `std::process::Command`，它已经脱离城的控制台、放进自己的进程组，并去掉了城的秘密所在的环境变量。本文件是 crate 的规格入口；它没有状态机，十七节都是说明，不是证明，接口由 Rust 的类型与 `child::environment::tests` 守住。决定写作 `D<n>`，别处引作 `child D<n>`。
-/

/-! ## 1 需求分解

两个单元，与模块一一对应：

- 构造（`command`）：一个 `Command`，在 Windows 上带 `CREATE_NO_WINDOW`，在 Unix 上带 `process_group(0)`，环境按 `environment` 的规则去掉秘密（D1、D2）。
- 环境（`environment`）：哪些环境变量是城的秘密，以及去掉它们的规则：键以 `SPRAWLING_SECRET_` 起头的每一个，和 `SPRAWLING_PAIRING_TOKEN`（D3）。前缀在这里只定义一次，`gateway::credential::vault` 从这里读它。

本 crate 只配置，不起进程：`spawn` 仍由每个调用点自己做，stdio 与参数也是调用点的事。
-/

/-! ## 2 验收标准

| 验收 | 完成的定义 |
|---|---|
| 秘密不进子进程的环境 | `environment::tests::every_secret_key_is_removed_and_nothing_else`：一组继承来的键里，`SPRAWLING_SECRET_` 起头的每一个和 `SPRAWLING_PAIRING_TOKEN` 被去掉，别的键不动；在 Windows 上大小写不同的同名键也被去掉 |
| 城的子进程只从 `command` 起 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` 在三个平台上没有发现：`clippy.toml` 的 `disallowed-methods` 拒绝 `std::process::Command::new`，本 crate 之外每一个调用都带写明理由的 `#[expect(clippy::disallowed_methods, reason = "…")]`，属于 D4 的某一类 |
-/

/-! ## 3 假设与歧义

- **假设**：城的秘密只经这两种环境变量进到城的进程里。别的路（文件、命令行参数）不归本 crate。
- **例外**：城的子进程里只有一类不走 `command`：守护进程起的那个子进程（它就是城，有意留在同一个控制台上），理由写在 `crates/sprawling/spec/Supervising.lean` 的「子进程的那一行」：子进程继承这个终端，终端的面属于它。CLI 交出终端给某个 agent 的登录流程还没有建成（`crates/agent_protocols/Spec.lean` §3）；建成时它是第二类，因为登录期间那个程序占着终端。浏览器与文件管理器不是例外：它们比城活得久，正因为如此才要去掉城的秘密，所以打开器与文件管理器都经 `command` 起，调用点再把三路 stdio 设成 null（`crates/sprawling/spec/Firstrun.lean` §8-8）。
- **不是城的子进程的起点**：本二进制的 `gauge` 动词、`tools/` 下的开发工具、构建脚本与测试代码起的进程都不是城的子进程，D4 说它们为什么不走 `command`。`clippy.toml` 的禁令对它们同样生效，所以每一个这样的起点都在调用处看得见。
-/

/-! ## 4 现状分析

本 crate 之前，城里十几个起子进程的地方各自决定平台标志、stdio 与环境：`creation_flags` 全仓只出现在 runtime exec 的一处，ACP harness、MCP stdio、浏览器引擎、cloudflared、doctor 的子进程都附着在城的控制台上，并继承城的全部环境。Windows 上附着在同一个控制台上的进程能经 `CONOUT$` 读到屏上的配对码，经 `CONIN$` 读到人敲进 CLI 的键；继承的环境里有 `SPRAWLING_SECRET_*` 的明文。
-/

/-! ## 5 权威信源

- Windows 进程创建标志：https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags （`CREATE_NO_WINDOW` = `0x08000000`：「The process is a console application that is being run without a console window」）。
- 控制台继承：https://learn.microsoft.com/en-us/windows/console/creation-of-a-console （「By default, a console process inherits its parent's console」）。
- `std::os::windows::process::CommandExt::creation_flags`（稳定，1.16.0 起）与 `std::os::unix::process::CommandExt::process_group`（稳定，1.64.0 起）。
- Windows 的环境变量名不分大小写：https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables 。
-/

/-! ## 6 命名统一

**child process**（城起的子进程）、**secret**（`docs/glossary.md`）。`command` 与标准库的 `Command` 同义：它交回的就是一个 `Command`。
-/

/-! ## 7 模块边界

`child::command`（构造）与 `child::environment`（秘密的键与去掉它们的规则），`lib.rs` 只做再导出。本 crate 不依赖任何 crate，所以 ARCHITECTURE.md §3 的 `depmap` 里它的一行是空的，每一个有起点的 crate 都可以依赖它。
-/

/-! ## 8 接口先行

```rust
// child::command
pub fn command(program: impl AsRef<OsStr>) -> std::process::Command;
#[cfg(windows)] pub const NO_WINDOW: u32 = 0x0800_0000;   // 调用点另设 creation_flags 时与它按位或

// child::environment
pub const SECRET_PREFIX: &str = "SPRAWLING_SECRET_";
pub const PAIRING_TOKEN: &str = "SPRAWLING_PAIRING_TOKEN";
```

- `command` 不失败：它只配置，读的是本进程此刻的环境键，不读值。
- **`creation_flags` 是覆盖，不是累加**：一个调用点另要优先级类（runtime exec 的 `BELOW_NORMAL_PRIORITY_CLASS`）时写 `creation_flags(child::NO_WINDOW | BELOW_NORMAL)`，否则会把 `NO_WINDOW` 盖掉。

D1 Windows 上用 `CREATE_NO_WINDOW`，不用 `DETACHED_PROCESS`：`DETACHED_PROCESS` 的子进程再起一个控制台程序（shell 里的 `cargo`）时，系统会为它新开一个可见的控制台窗口；`CREATE_NO_WINDOW` 给子进程一个没有窗口的控制台，子孙进程继承那一个。`CREATE_NEW_PROCESS_GROUP` 不需要：子进程不在城的控制台上之后，城收到的 Ctrl+C 本来就到不了它。被否：`DETACHED_PROCESS`（上面的弹窗）；不设标志（附着的进程读得到屏幕缓冲区与键盘输入）。

D2 Unix 上用 `process_group(0)`：终端产生的 `SIGINT` 与 `SIGHUP` 不再发给子进程，它也读不走终端的输入。`setsid` 还不稳定，`pre_exec` 要 `unsafe`，工作区的 `unsafe_code` 是 `forbid`，所以用安全 Rust 做不到完全脱离控制终端。代价：城异常退出时子进程不随终端的 `SIGHUP` 结束，会成为孤儿；一个结束孤儿的机制推迟到以后。

D3 环境按键名去掉，不清空再放白名单：调用点各自需要的环境不同（`PATH`、代理、语言），一份白名单会成为每个调用点的第二个配置面；城的秘密只有两种键，按名去掉就够。前缀在 Windows 上按 ASCII 不分大小写比较，因为 Windows 的环境变量名不分大小写，`sprawling_secret_acme_key` 与 `SPRAWLING_SECRET_ACME_KEY` 是同一个变量，vault 读的也是它；Unix 上按字节比较。被否：清空再放白名单（每个调用点一份白名单）；只去掉 vault 此刻用到的那几个键（vault 不知道人在环境里放了哪些）。

D4 `clippy.toml` 的 `disallowed-methods` 拒绝 `std::process::Command::new`，本 crate 的 `command` 是唯一不带例外的调用者，它自己带 `#[expect(clippy::disallowed_methods, reason = "the one constructor every other spawn point is sent to")]`。本 crate 之外每一个调用带一条 `#[expect(clippy::disallowed_methods, reason = "…")]`，放在最窄的那一项上（函数，或整个都在起夹具进程的测试模块），理由说它属于下面哪一类：

- 守护进程的子进程（§3 的例外）：`bin::supervising::children`。
- `gauge` 量的命令：它在人的终端前台跑人给的命令，像 shell 那样。`command` 会把它放进自己的进程组、给它一个没有窗口的控制台，于是人按的 Ctrl+C 到不了它，`gauge` 退出后它成了孤儿。它不在城里跑，环境是人自己的。
- `tools/` 下的开发工具（xtask、citysim）：它们起的 cargo、git、浏览器与被测的二进制属于开发者，不属于城，不持有城的秘密；开发工具要能用终端的 Ctrl+C 结束它们，要按进程树杀掉浏览器。`tools/` 不依赖本 crate。
- 构建脚本（`crates/desktop/ffi/build.rs` 起 `zig`）：它在 cargo 里跑，不在城里。
- 测试代码：测试起的是它自己的夹具进程。经生产入口（如 `runtime::Backlog`）交进去的 `Command` 也是夹具，生产调用点交进去的由 `command` 造。

被否：给 `tools/` 一份自己的 `clippy.toml`（clippy 只读离 crate 最近的一份，于是时钟与 HTTP 客户端的禁令要复制一份）；crate 级的 `expect`（同一个 lint 名也管 `Instant::now` 与 `reqwest` 的禁令，crate 级的一条会把它们一起盖住）；测试也改走 `command`（Unix 上的进程组与 Windows 上的无窗口控制台会改变夹具进程与测试之间的控制台和信号关系，一个测试要的是它自己写下的那组标志）。重开条件：clippy 能按路径给出禁令的作用范围，或者测试需要验证的正是 `command` 的标志。
-/

/-! ## 9 工作流程

调用点写 `child::command(program)`，再加参数、stdio 与它自己要的环境，然后 `spawn`。`command` 在构造时读一次本进程的环境键，对每一个秘密键调 `env_remove`；调用点之后再 `env` 一个同名键，以调用点为准（那是它有意交给子进程的值）。
-/

/-! ## 10 实现逻辑

`environment::scrubbed(command, keys)` 对 `keys` 里每一个是秘密的键调 `env_remove`；`command` 以 `std::env::vars_os()` 的键调它。判定是 `environment::is_secret(key)`：键等于 `PAIRING_TOKEN`，或以 `SECRET_PREFIX` 起头；Windows 上两者都按 ASCII 不分大小写。
-/

/-! ## 11 边界枚举

键恰为 `SPRAWLING_SECRET_`（以前缀起头，去掉）；`SPRAWLING_SECRETS`（不以带下划线的前缀起头，留下）；小写的秘密键（Windows 上去掉，Unix 上留下）；不是 Unicode 的键（按字节比较，不会因为读不成字符串而漏掉）；空环境。
-/

/-! ## 12 错误处理

没有错误：本 crate 只配置一个 `Command`。起进程失败是 `spawn` 的 `io::Error`，归调用点。
-/

/-! ## 13 依赖选型

不依赖任何 crate：标志值是一个常量，两个 `CommandExt` 方法在标准库里。为一个常量引入 `windows` 或 `winsafe` 会让每一个起点都多一条平台依赖。
-/

/-! ## 14 硬编码声明

`NO_WINDOW = 0x0800_0000`（Windows 的 `CREATE_NO_WINDOW`，§5）；`SECRET_PREFIX = "SPRAWLING_SECRET_"`；`PAIRING_TOKEN = "SPRAWLING_PAIRING_TOKEN"`。
-/

/-! ## 15 影响面

改 `is_secret` 改的是每一个子进程看得到的环境。改 `command` 的平台标志改的是每一个子进程与城的控制台、终端信号的关系；runtime exec 的优先级类与 `NO_WINDOW` 按位或，改标志时一起看。改 `SECRET_PREFIX` 同时改 vault 从环境读 Key 的键名（`gateway::credential::vault`），人已经设好的环境变量会失效。
-/

/-! ## 16 测试与约束

`cargo nextest run -p sprawling-child`：`environment::tests` 判 §2 的第一行。没有 Lean 模型：本 crate 没有状态机，只有一个判定与一个构造。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md §3 的 `depmap`（`child:` 一行），`architecture.toml` 的 `[family.child]` 与两个模块行。
- `crates/gateway/spec/Credential.lean`（环境变量是 vault 的只读来源，键形 `SPRAWLING_SECRET_<REALM>_<NAME>`，前缀读自本 crate）。
- `clippy.toml` 的 `disallowed-methods`（D4）。
-/
