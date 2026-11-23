> **sprawling <!-- xtask:begin workspace_version -->0.17.0<!-- xtask:end -->.** Wire 从 79 移至 80；账本与配置格式不变。升级前停止城市并验证备份可恢复。Windows 二进制仍未签名。
>
> Wire moves from 79 to 80; the Ledger and configuration formats are unchanged. Stop the city and verify a restorable backup before upgrading. The Windows binary is unsigned.

---

欢迎来到 sprawling v0.17.0 261123

本版迁移版本协议，并交付上一版之后的 Mei、Calyx 与仓库结构变化。

**版本协议 / Version protocol**

三段数字依次表示：

1. **成熟度**；
2. **全仓版本**；
3. **单独包的版本**。

全仓升版时，所有公开包共用同一个版本号。单包更新只使用第三位，不制造另一次全仓发布。当前全仓基线为 `0.17.0`，新 tag 的形状是 `v<version>-<YYMMDD>`。旧 tag 与 Release 全部保留。

**Mei 与 Calyx**

- Mei 的持久 Python3 工具在主机解释器上运行，小结果直接回显，更大或非 UTF-8 输出完整落盘；表达式不自动回显，异常后会话状态保留。
- 完整旧日志可恢复；`bunx` 与 `npm exec` 使用同一个 Bun 入口，TTY 打开 Calyx。
- Calyx 是独立的 Zig 交互面。`@sprawling/mei` 同时携带无头核与交互面，支持 Windows x64、macOS arm64 和 Linux x64 musl。
- Calyx 可通过 OSC 7501 报告工作状态；`SPRAWLING_PROGRAM_STATUS=never` 可关闭。

**仓库与交付**

产品目录归入 `Sprawling/`、`Calyx/`、`Mei/` 与 `RefRain/`，开发工具归入 `Tools/`。本版发布三份 sprawling 与三份 Mei 归档，并同步更新 npm 与 crates.io。

**下一版 / Next**

下一版主要是 Mei 的细节优化，以及 RefRain 面向文件编辑的模式。

**已知边界 / Known boundaries**

Mei 没有沙箱、权限确认或撤销。只使用可丢弃的项目副本和单独、可撤销且设好消费上限的 API key。部分既有诊断仍较笼统，部分工作区的 git 冲突仍可能阻止回复记录。

---

**In English / 英文版**

Welcome to sprawling v0.17.0 261123.

This release migrates the version protocol and carries the Mei, Calyx and repository-structure changes landed since the previous release.

The three numeric components mean, in order: maturity; the whole-repository release; an individual package release. A whole-repository release gives every public package one shared version. An individual package update uses the third component without creating another whole-repository release. The current shared baseline is `0.17.0`, and new tags read `v<version>-<YYMMDD>`. Existing tags and releases remain available.

Mei's Python tool runs on the host interpreter, returns small results inline, preserves session state across exceptions, and shares one Bun entry with Calyx on a TTY. Calyx is the Zig interactive face with optional OSC 7501 status. This release carries three sprawling archives and three Mei archives, with npm and crates.io updated from the same source.

Next: Mei detail work, and a RefRain mode for file editing.

Mei still has no sandbox, permission prompts or undo. Use only disposable project copies and a separate, revocable, spending-limited API key.
