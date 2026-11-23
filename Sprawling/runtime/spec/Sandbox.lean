/-
Copyright (c) 2026 2youg1 and the sprawling contributors
SPDX-License-Identifier: MPL-2.0
-/

/-!
# Sandbox — retired wasip1 guest surface

wasip1／wasmtime 适配器已退役。Python 与命令执行都走 `runtime::tools::exec`：宿主进程，围栏由楼所选的臂（`copied_tree`／`native`／`container`／`none`）提供；`SandboxArm::Python` 只用宿主解释器（`SPRAWLING_PYTHON` 或 `PATH` 上的 `python3`／`python`）。

本文件保留编号入口，避免旧引用悬空；契约正文以 `Sprawling/runtime/spec/Tools/Exec.lean` 为准。

## §8-13 执行边界

生产面不再提供 `Sandbox` trait、`WasmtimeSandbox`、`Fuel`、`Mount` 或 guest 内存上限。Doctor 检查 `host-python`，不检查 wasm guest 或 `sandbox` feature。

## 验收

Python 臂经 `exec` 的 `-c` 跑宿主解释器；缺解释器时 `E_TOOL_UNAVAILABLE`，恢复语点出 `SPRAWLING_PYTHON`／安装 Python 3。围栏与 program 同界：`copied_tree` 不限制网络，保密楼上的外连缺口与 command 臂相同（`docs/operating.md`）。A10（fuel／preopen／wasip1）不再适用。
-/
