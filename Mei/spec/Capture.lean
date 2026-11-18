-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 的输出捕获所有权模型

这一部是**描述**，不是证明：捕获的所有权由 `Mei/src/capture.rs` 的 `Store` 与
`Calyx/shell/mei_session.zig` 的运行目录决定，这里只写下两者共用的分类。行为由
`Mei/test/capture.rs` 经生产门（真实进程、真实文件计数）与 `Calyx/shell` 的
`zig build test` 判定：commit 保留 spilled、非 commit 回收整个目录、解释器退出不删除
session/caller 所有的文件。模型不覆盖宿主崩溃后的物理删除。
-/

namespace Mei.Capture

inductive Kind where
  | inline | spilled
  deriving DecidableEq, Repr

inductive Surface where
  | interactive | headless
  deriving DecidableEq, Repr

inductive Owner where
  | run | session | caller
  deriving DecidableEq, Repr

def owner : Kind → Surface → Owner
  | .inline, _ => .run
  | .spilled, .interactive => .session
  | .spilled, .headless => .caller

def fileSurvivesInterpreterExit (kind : Kind) (surface : Surface) : Bool :=
  owner kind surface != .run

end Mei.Capture
