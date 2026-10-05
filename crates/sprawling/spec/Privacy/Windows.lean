-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 用户级 Windows privacy 平台契约
本节是尚未接线的平台接口说明，不是形式证明。
唯一候选是 PowerShell 7 的 POWERSHELL_TELEMETRY_OPTOUT，用户持久目标为
HKCU Environment；读取必须保留原类型/字节、键和值缺席、权限和 IO 失败。
真实 owner 由 OS SID 取得，管理来源 Unknown 拒写，不以可写 HKCU 或空 RSoP
判断 Local。REG_SZ/REG_EXPAND_SZ 的原字节无法精确重写时应用前拒绝。
新进程环境与持久值分开验证，不能声明已运行 pwsh 或 scrubbed exec 已生效。
当前没有生产平台 adapter；不得把此候选说明计作 OS 验收。
-/
