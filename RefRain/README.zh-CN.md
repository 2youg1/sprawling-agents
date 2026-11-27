# RefRain

[English](README.md)

**自己读、自己写，也能随时接手 Agent 的工作。** RefRain 一边是文档工作区，一边是 Sprawling Agent Harness 的 WebUI。读写文件不用先配置模型；要 Agent 帮忙时，再回到具体会话。

- **文档创作和阅读。** 在文件模式用「登记工作区」打开已有文件夹，无须复制。Markdown 自动保存会写回该文件夹；不想改动原件，可以先复制一份再登记。自动保存不清空撤销历史。PDF 支持缩放、翻页预览和续读；需要 Agent 看某一段时，可附上选区或原始像素区域。
- **以会话和 Git 管理 Agent 工作。** 看对话和工具调用，续接指定历史会话，或显式分支。在项目里查看 Git 提交和差异、从已有提交取回文件，也能从城市提交追到产生它的运行。
- **文件和工作过程放在一处。** 一侧读 Agent 写出的文件，另一侧看它怎么做到。文件版本、确认写入和城市状态由 Rust 主机负责；浏览器记住版式、阅读位置和草稿。

<p align="center">
  <a href="../docs/images/refrain-workspace.webp"><img alt="左侧是 Atlas 的任务对话与工具调用，右侧打开 Agent 创建的 README.md 文件" src="../docs/images/refrain-workspace.webp" width="100%"></a>
</p>

同一工作区打开了演示项目的对话和文件，可对照工具调用、答复与文件内容。

## 快速上手
在已安装的 Sprawling 中运行 `sprawling up ./cities/first`。页面关掉后，在主机终端按 Esc 进入 CLI，再输入 `/web`。试文档工作区：跳过模型设置，点击「登记工作区」，填入主机上的文件夹绝对路径，再打开 Markdown 或 PDF。城市里的「新建／导入工作区」是另一项操作，才会把主机文件夹复制进楼。试 Agent 工作区：接入模型后打开一个 Session，再对照对话和项目改动。

## 运行本地客户端

在仓库根目录构建生产客户端，再启动提供页面和离线资源的主机。

```sh
just build-web
cargo run --locked -p sprawling -- up ./cities/first
```

浏览器直接打开服务地址时，可用主机控制台显示的配对码完成配对。首次配置和模型连接见[上手指南](../docs/start/getting-started.zh-CN.md)。

## 开发与检查

在 `RefRain/` 下使用项目固定的 Bun 版本。Bun 运行 Vite 和类型感知的 Oxlint，并直接完成类型检查、格式化和测试。

```sh
bun install --frozen-lockfile
bun run dev
bun run lint
bun check
bun run test
bun run build
```

从仓库根目录运行 `bun fmt`，使用全仓库的格式化配置。

## 会话发送验收

脚本在浏览器打开旧会话，向上滚动后点击发送，再从原消息显式分支并发送新分支的第一条消息。它用城的账本核对续聊目标和分支起点，并确认两条回复各自显示在对应会话里。脚本复用 walk 的固定回复供应方，默认把临时数据放在 `target/send`。它与 `just walk` 和 `HotKeyE2E.ts` 共用端口，请分开运行。

```sh
just build-web
cargo build --package sprawling --bin sprawling --package citysim --bin provider --locked
cd RefRain
bun scripts/session_send_e2e.ts
```

## 技术栈与文档

Solid 2 和 TypeScript 构建响应式视图；Bun 管理依赖并运行检查；Vite 8 把前端打包供 Rust 主机嵌入。原生 CSS 提供网格、主题和层叠，Effect 负责解码通信数据。视图在 [`src/views/`](src/views/)，浏览器状态在 [`src/core/`](src/core/)，界面文案来自 [`src/lang.json`](src/lang.json)。[`src/wire/`](src/wire/) 的解码器由 `cargo xtask wire-ts` 生成，供浏览器读取[城市 wire](../docs/wire.md)。

[RefRain SPEC](Spec.lean) 定义界面、键盘和无障碍合同。[前端方法](../docs/maintain/frontend-method.md) 记录设计与验收流程；[界面设计方法](../docs/maintain/ui-design.md)解释网格与图层，[英文源码地图](README.md#where-code-goes)按目录标出维护入口。
