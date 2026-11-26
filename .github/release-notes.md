sprawling <!-- xtask:begin workspace_version -->0.18.3<!-- xtask:end --> 通过 HTTP/3 提供远程 RefRain，配对和会话走 WebTransport。RefRain 改进了 Markdown 编辑和主题预览，计划会记录任务依赖并显示分阶段进度。Mei 缩短了打开长会话和启动 Python 工具的等待。Cargo 可从分别发布的 Calyx 和渲染器包构建终端壳。

Wire 从 82 升到 83，升级后请刷新已打开的 RefRain 页面。账本格式仍为 2，配置格式仍为 1。已有 `route = "h3"` 配置需移除 `command` 字段。

---

sprawling 0.18.3 serves remote RefRain over HTTP/3, with WebTransport for pairing and sessions. RefRain improves Markdown editing and theme previews. Plans retain task dependencies and show progress by stage. Mei reduces waits when opening long sessions and starting Python tools. Cargo builds the terminal shell from the separately published Calyx and renderer crates.

Wire moves from 82 to 83. Reload open RefRain tabs after upgrading. The ledger format stays at 2 and the configuration format stays at 1. Existing `route = "h3"` configurations remove the `command` field.
