-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy 的一次确认许可
本节是尚未接线的确认接口说明，不是形式证明。
确认行为由 crates.sprawling.spec.Privacy 的 Request、Step.answer、planValid
约束；许可绑定 control/definition/owner/current/target/operation/expiry。
错误、取消、期限已过和重放均不得写入；durablePrepared 和 writeStarted 都
必须重新检查期限和资格。时间由 assembly 的 SystemClock 取得，熵由 OS 提供。
console 才可显示 code，页面、模型、MCP/ACP 和远程请求不能直接取得许可。
当前没有 production confirmation adapter；不能把确认文字当安全授权。
-/
