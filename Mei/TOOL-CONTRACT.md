# Mei prompt and tool contract

[简体中文](TOOL-CONTRACT.zh-CN.md) · [Project home](README.md)

The [Agent SPEC](spec/Agent.lean) defines the call arguments and validation rules. The `python_tool` function in [`src/launch.rs`](src/launch.rs) builds the JSON `input_schema`. The fixture below records one example.

`Mei/src/MEI.md` is compiled into `system`; the sole python definition occupies `tools[0]`. The table below uses a fixed documentation fixture with Python `3.12.13` and uv `0.12.3`; actual version attributes come from doctor. Byte counts exclude message envelopes and anything a provider adds itself.

| Part | UTF-8 bytes | Tokens (`o200k_base`) |
|---|---:|---:|
| `system` | 297 | 57 |
| Complete compact `tools[0]` JSON | 1489 | 332 |
| Total | 1786 | 389 |

Tokens use OpenAI's `o200k_base` tokenizer (the GPT-4o/4.1/5 family) on the same text, with no chat envelope or provider additions; another tokenizer gives another count. The JSON below is pretty-printed for reading, while the byte and token counts use its compact wire form.

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

Mei keeps one provider-neutral schema and validates every tool wave in the agent loop; it does not declare provider-specific `strict` modes.
