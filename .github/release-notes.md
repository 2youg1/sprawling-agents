> **sprawling <!-- xtask:begin workspace_version -->0.17.2<!-- xtask:end -->.** Wire、账本与配置格式与 0.17.0 相同（wire 80 的迁移已在 0.17.0 完成）。升级前停止城市并验证备份可恢复。Windows 二进制仍未签名。
>
> Wire, the Ledger and the configuration formats are unchanged from 0.17.0, which moved the wire to 80. Stop the city and verify a restorable backup before upgrading. The Windows binary is unsigned.

---

欢迎来到 sprawling v0.17.2 261124

本版修复 Mei 在 Windows 控制台中的输入处理，并修正命令菜单替换草稿时的光标状态。Wire、账本与配置格式保持不变。

**Mei 与 Calyx**

- Windows 输入改用控制台原生事件记录，停用 Quick Edit 与虚拟终端输入，并启用鼠标和窗口事件；不再向 Windows 控制台请求 Kitty 键盘、括号粘贴或 SGR 鼠标协议。
- 命令菜单替换输入草稿前重置光标，选择 `/quit` 不再因旧光标状态 panic。

---

**In English / 英文版**

Welcome to sprawling v0.17.2 261124.

This release fixes Mei's Windows console input and the cursor state when the command menu replaces a draft. The wire, Ledger and configuration formats are unchanged.

**Mei and Calyx**

- Windows input uses native console records, disables Quick Edit and virtual-terminal input, and enables mouse and window events. The Windows console no longer receives Kitty keyboard, bracketed-paste or SGR-mouse requests.
- Replacing the input draft from the command menu resets the cursor, so selecting `/quit` no longer panics on stale editor state.
