# Mei 的提示词与工具接口

[English](TOOL-CONTRACT.md) · [项目首页](README.zh-CN.md)

[Agent SPEC](spec/Agent.lean) 定义调用参数与验证规则。[`src/launch.rs`](src/launch.rs) 中的 `python_tool` 构造 JSON `input_schema`。下方的固定样例记录了一次请求的内容。

`Mei/src/MEI.md` 在构建时进入 `system`；唯一的 python 定义位于 `tools[0]`。下表使用 Python `3.12.13`、uv `0.12.3` 的固定文档夹具；实际版本属性来自 doctor。字节数不含消息信封和 provider 自行添加的内容。

| 部分 | UTF-8 字节 | Token（`o200k_base`） |
|---|---:|---:|
| `system` | 297 | 57 |
| 完整紧凑 `tools[0]` JSON | 1489 | 332 |
| 合计 | 1786 | 389 |

Token 列用 OpenAI 的 `o200k_base` 分词器（GPT-4o/4.1/5 家族）对同一文本计数，不含聊天信封或 provider 追加内容；换成其他分词器数值会不同。以下 JSON 为便于阅读而格式化；字节数和 Token 数按紧凑线上格式计算。

### `system`

```text
<agent-harness>
You are operating inside Mei.
No additional documents or environment capabilities are injected or exposed; fetch what you need.
Workflow: Sense → Act → Verify → Adapt…
Aligned with context, fulfill user intent thoroughly and resourcefully and stay scoped.
</agent-harness>
```

### `tools[0]`

```json
{
  "description": "<python version=\"3.12.13\" uv_version=\"0.12.3\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. Expressions do not auto-echo, so explicitly print any values you need to inspect. If combined UTF-8 stdout/stderr is up to 16 KiB, the tool returns it inline. Larger or non-UTF-8 output returns a notice and the full path, or a Python expression that losslessly reconstructs a non-Unicode native path; filter that file, or re-run with narrower output, in the next python call. Run shell commands and CLIs through subprocess, capture their output, and check return codes. Start child interpreters with sys.executable, so they inherit the active environment. Call show_image(path) to attach a PNG, JPEG, GIF, or WebP picture as visual tool-result content; a picture is not text output. Attach at most 2 pictures per call and at most 5 MiB per picture, on the call's own thread; a picture from another thread or a later call is refused with how to read it. Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python>",
  "input_schema": {
    "properties": {
      "code": {
        "type": "string"
      }
    },
    "required": [
      "code"
    ],
    "type": "object"
  },
  "name": "python"
}
```

Mei 只保留一个 provider-neutral schema，并在 agent loop 中校验每一波工具调用；不声明 provider 专有的 `strict` 模式。
