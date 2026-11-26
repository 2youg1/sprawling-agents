-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::firstrun

规定 `Sprawling/app/src/firstrun.rs`：首次运行与交付形态（`bin::firstrun`）。本文件是 `Sprawling/app/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `Sprawling/app/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::firstrun` 旁的测试守住。
-/

/-!
## 8-8 首次运行与交付形态

**原因**：release 里的 exe 是控制台程序。无参启动只向 stderr 写一行用法并退 2，从资源管理器双击即闪退——没有安装过程，也没有任何成败提示。从 exe 到 WebUI 之间还压着 `init`／`serve`／自行输入地址三步手工操作，而 `serve` 打印的是裸 socket 地址不是 URL。终端、双击、脚本三类到达方式被挤在同一个入口上。

**不猜启动方式**：判断「我是被双击的还是在终端里跑的」，可靠办法是 `GetConsoleProcessList`，需 `unsafe`——workspace lints 恒禁。故以**显式入口**取代探测：三扇门各自命名，背后共用同一段序列。

```rust
// bin::firstrun
pub(crate) enum FirstScreen { Start(PathBuf), Use(PathBuf), Quit }

// bin::router（二进制那一半）
fn default_city_location() -> Result<PathBuf, AxError>;   // 个人配置的 city，否则 Home::default_city()
fn name_city(city: &Path);                                // 第一屏选定后写进个人配置；失败只报告
pub(crate) fn ask<R: BufRead, W: Write>(city: &Path, input: &mut R, out: &mut W) -> io::Result<FirstScreen>;
pub(crate) fn open_paired(origins_url: &str, door: &wire::LocalDoor) -> Result<(), AxError>;
```

- **`up <dir>`＝序列的唯一定义**：目录里没有 ledger 就先 `init`，随后 `serve`。无参屏与 `start.cmd` 都落到它，`init`／`serve` 仍各自独立可用——一段序列一处权威。三扇门的序列相同，终端面不同（§8-11）：无参启动进 CLI，不开浏览器，人敲 `/web` 再开；`up` 与启动器进安静宿主并开浏览器，双击的人仍然一步到网页。
- **genesis 要人同意**：写 Ledger 第 0 行是全系统唯一一次不可撤销的语义写入，不因「有人双击了一个文件」而发生。无参屏只在默认位置还没有城时出现，是 CLI 的头两行：`sprawling  no city yet`，`start one at <路径>?   Enter yes   type a folder to use it   q quit`；按键**之前**最终路径已经在屏上，人按回车才开城；`q` 退出并打印命令表。默认位置已经是一座城时不再问，直接开它。
- **非交互 stdin 无此问**：`read_line` 得 EOF（管道、CI、无人值守）即 `Quit`，主流程打印命令表退 2。这条让该路径在没有 TTY 的地方也可测。
- 显式路径、个人配置 `city`、默认路径依次选择工作区。初次打开为这个工作区建立私有城。
- 第一屏把选定的工作区写入个人配置。`up <dir>` 只在本次启动使用该工作区。
- **默认路径只有 `accounting::home` 一处权威**：`~/sprawling/city` 由 `Home::default_city()` 给出，目录名 `city` 由 `home::CITY_DIR` 一处定义。
- **开浏览器恒非致命**：`open_paired` 失败只记一行，`serve` 照跑——URL 在这之前已经打印。命名不取 `browser`：`Sprawling/browser` 已占住「Agent 驱动真实浏览器」这个概念，一名一义。
- **开浏览器经跳转文件，不经命令行**：`open_paired` 向这台电脑上的这扇门要一个开页码（`Sprawling/wire/spec/Reception/Pairing.lean` §8-95），写 `<每用户运行目录>/sprawling/open-<pid>.html`（目录与钥匙文件同一个，只给本用户读），内容是一句 `<meta http-equiv="refresh">` 加一句 `location.replace`，指向 `<url>/#open=<码>`，再把这个文件的路径交给系统打开器。命令行里只有路径：交给 `cmd /C start`、`open`、`xdg-open` 的 URL 会留在浏览器主进程的 argv 里，Linux 上别的用户读得到 `/proc/<pid>/cmdline`。码兑掉或过期（120 s）时删掉这个文件；城被强制结束时来不及删，所以每次起城先删掉运行目录里比开页码寿命（`wire::OPEN_CODE_LIFETIME_MS`）更旧的 `open-*.html`，它们装的码都已过期。只删更旧的，不删别的进程号的全部：同一个用户可能同时开着两座城，另一座城刚写下、浏览器还没读的那一份不能删。WSL 里 `file:` 页开不了 Windows 一侧的浏览器，`WSL_DISTRO_NAME` 在场时只开裸地址，人在安静宿主按 C 生成一次性码，选中复制到浏览器。CLI 的 `/web`、安静宿主的 Enter 与 `up` 的开页都走 `open_paired`，三处同一条路。
- **打开器是城的子进程，照 `child::command` 的规则起**：环境去掉 `SPRAWLING_SECRET_*` 与 `SPRAWLING_PAIRING_TOKEN`，三路 stdio 全是 null，Windows 上不挂在城的控制台上，Unix 上在自己的进程组里。原因：冷启动时浏览器主进程由打开器起，继承打开器的环境，而且活得比城久，它的子进程也继承；stdio 继承下来，打开器与浏览器的输出就印进 CLI 或安静宿主的屏幕。Unix 的 `process_group(0)` 还让城退出时浏览器不随终端的 `SIGHUP` 结束。Windows 上 `cmd` 按绝对路径起，`<SystemRoot>\System32\cmd.exe`，`SystemRoot` 取 HKLM 的安装记录（`bin::privacy::windows::system32`），不按裸名经搜索顺序找：搜索顺序的第一个位置是二进制所在的目录，上一版的城默认就在它旁边，这个目录常常本用户可写。`open` 与 `xdg-open` 仍按 `PATH` 找：macOS 与 Linux 不先搜二进制所在的目录，`xdg-open` 的位置也因发行版而异。
- **横幅只给没有终端面的城**：`Headless`（含行控制台）印 city 目录、WebUI 的完整 URL、客户端完整与否、`Ctrl-C` 停城，四行，工装从标准输出读 URL。横幅不印配对码，只说怎样配对：有行控制台时敲 `/web`，它打开一个已经配好的页面；没有控制台时用 `--console` 或 `sprawling up` 起城，再用 `/web`。原因：没有终端面的城，标准输出通常是日志文件、journald 或服务管理器，读得到日志又能发出本机请求的人能在开页码有效时兑换。铸出的钥匙（`Keyed::Minted`）单独印一行，不拼进任何地址（`Sprawling/app/spec/Keying.lean` §8-22）。CLI 只印一行表头，安静宿主只印地址与配对码（§8-11）；安静宿主默认提示按 C 生成开页码。只有人在备用屏按 C 才生成并绘制，离开时清除这一行。URL 取 `wire::ListenerOrigins::url`，与 Host、Origin 名单同一处算出：bind 是未指定地址（`0.0.0.0`）时给回环形，因为那才是运行中的机器打得开的那一个。横幅与表头都在端口已经绑定、写者已经持锁之后才印（§8-88）。

**交付形态**：`just package` 产 `sprawling-<version>-<target>.zip`＝二进制＋`start.cmd`／`start.sh`＋`QUICKSTART.md`；裸 exe 不再单独作附件，双击的目标因此永远是启动器。`release.yml` 由 tag 触发，三平台各跑 `just dist`，`xtask budget` 在打包前拦下页壳客户端（`CLIENT_COMPLETE=false` 的二进制），通过后才附件。手工上传的产物来历不明，是本次全部症状的链头，这条把它关掉。

**本章测试**：个人配置 `city` 的读写与相对路径的拒在 `accounting::person` 旁；`ask` 空行得 `Start`、`q` 得 `Quit`、EOF 得 `Quit`；第一屏文本在返回前已含最终路径（证明「先示后写」）。URL 由 `wire::ListenerOrigins::url` 给出，它对未指定地址给回环形，测试在 `wire::reception::entry` 旁。
-/

/-! All city arguments to the executable select a workspace. Commands which read, serve
or change city state resolve the same private city through accounting::home::Home::workspace.
The first screen remembers the user's workspace selection and opens only its private city.
Project registration is explicit. Sessions enumerate only private state. -/
