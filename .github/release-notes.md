> **sprawling <!-- xtask:begin workspace_version -->0.17.1<!-- xtask:end -->.** Wire、账本与配置格式与 0.17.0 相同（wire 80 的迁移已在 0.17.0 完成）。升级前停止城市并验证备份可恢复。Windows 二进制仍未签名。
>
> Wire, the Ledger and the configuration formats are unchanged from 0.17.0, which moved the wire to 80. Stop the city and verify a restorable backup before upgrading. The Windows binary is unsigned.

---

欢迎来到 sprawling v0.17.1 261123

本版是一次修正发布。0.17.0 的 npm 与 crates.io 渠道只站住了一半，`v0.17.0-261123` 的 tag 还被重打过一次，而重打让源码验证在选择基线时红在一个已经退役的 tag 对象上。0.17.1 把这三件事一起收掉，npm 与 crates.io 从同一棵树发出。

**发布与验证 / Distribution and verification**

- npm 的八个包与 crates.io 的每个公开 crate 都从这一版的树发出；全仓共用一个 `0.17.1`。0.17.0 的 npm 缺 `@sprawling/mei-windows-x64` 的 trusted publisher 注册，crates.io 只发到二十个 crate 中的十四个。
- `v*` 发布 tag 不论新建还是重打，源码验证都以被 tag 提交的第一父作为改动范围，不再读取事件里的 `before`。重打 tag 因此不再因为拿不到已退役的 tag 对象而失败。

**Mei 与 Calyx**

- Mei 的终端改为锚定视口的整帧绘制：每个用户轮次都保留，输入行固定在底部，终端尺寸变化时重绘已写出的内容；城市控制台保持原来的增量主屏画面 (#367)。
- Python 调用保留源码与输出，耗时行与城市工具行一致；上下键只在草稿两端翻历史 (#367)。
- `@sprawling/mei` 同时携带无头核与交互面，支持 Windows x64、macOS arm64 和 Linux x64 musl；`bunx` 与 `npm exec` 走同一个 Bun 入口，TTY 打开 Calyx。

**设计规则**

- 仓库的设计规则写明事件驱动是默认形态：任务从唯一的 spawn 点启动，轮询只在确有必要压低延迟与开销时才出现 (#372)。

**下一版 / Next**

下一版主要是 Mei 的细节优化，以及 RefRain 面向文件编辑的模式。

**已知边界 / Known boundaries**

Mei 没有沙箱、权限确认或撤销。只使用可丢弃的项目副本和单独、可撤销且设好消费上限的 API key。部分既有诊断仍较笼统，部分工作区的 git 冲突仍可能阻止回复记录。

---

**In English / 英文版**

Welcome to sprawling v0.17.1 261123.

This is a correction release. 0.17.0's npm and crates.io channels only half stood up, and `v0.17.0-261123` was re-cut once, which turned source verification red while it selected a baseline that named the retired tag object. 0.17.1 settles all three, and npm and crates.io are published from one tree.

**Distribution and verification**

- All eight npm packages and every public crate are published from this release's tree, sharing `0.17.1`. 0.17.0's npm was missing the trusted publisher registration for `@sprawling/mei-windows-x64`, and crates.io had published fourteen of twenty crates.
- A `v*` release tag judges the tagged commit's first parent whether it is new or re-cut, and no longer reads the event's `before`. A re-cut tag can no longer fail on a retired tag object it cannot resolve.

**Mei and Calyx**

- Mei draws its terminal as one anchored viewport: every user turn survives, the composer stays fixed at the bottom, and a resize repaints what was already written, while the city console keeps its incremental main-screen frame (#367).
- A Python call keeps its source and output and shows the elapsed-time line the city tool row uses; Up and Down recall history only at the ends of the draft (#367).
- `@sprawling/mei` carries both the headless core and the interactive face for Windows x64, macOS arm64 and Linux x64 musl, and one Bun entry serves `bunx` and `npm exec`.

**Design rules**

- The repository's design rules now state the event-driven default: tasks start from the one spawn point, and polling appears only where it is necessary to keep latency and overhead down (#372).

This release carries three sprawling archives and three Mei archives, with npm and crates.io updated from the same source.

Next: Mei detail work, and a RefRain mode for file editing.

Mei still has no sandbox, permission prompts or undo. Use only disposable project copies and a separate, revocable, spending-limited API key.
