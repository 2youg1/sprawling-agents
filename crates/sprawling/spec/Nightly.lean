-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Nightly 快照身份与发布推进

规定 `crates/sprawling/src/release/nightly.rs` 的快照身份，以及
`.github/workflows/nightly-release.yml` 的发布推进。模型只描述判定与保留规则；
GitHub、编译器、网络和文件系统的效果由回环安装验收与 workflow 检查。

nightly 是 main 的一份快照，不是 `kernel::Release` 的另一种 semver。tag 形如
`nightly-YYMMDD-fullSHA`，日期是首次进入发布流程的 UTC 日期；完整 SHA 同时进入 tag，
是身份、幂等和先后判定的唯一依据。基准正式 tag 只用于生成比较范围，
不参与 nightly 的版本排序。PR 列表由 GitHub generate-notes 以显式基准生成，
不另设 Direct commits 一节。
-/

namespace Sprawling.Nightly

/-- 发布通道与安装方式是两个维度；这里只有通道。 -/
inductive Channel where
  | release
  | nightly
  deriving Repr, DecidableEq

/-- 一个 tag 对通道选择可见的种类。 -/
inductive TagKind where
  | curated
  | nightly
  | other
  deriving Repr, DecidableEq

/-- 通道只接纳自己的 tag；`v*` 是 curated，`nightly-*` 是 nightly。 -/
def admits : Channel → TagKind → Bool
  | .release, .curated => true
  | .nightly, .nightly => true
  | _, _ => false

example : admits .release .curated = true := by decide
example : admits .nightly .nightly = true := by decide
example : admits .release .nightly = false := by decide
example : admits .nightly .curated = false := by decide
example : admits .release .other = false := by decide
example : admits .nightly .other = false := by decide

/-- 已公开快照的 source 字段来自 Release API 的 `target_commitish`，不是列表位置或展示文本。 -/
structure Snapshot where
  source : String
  deriving Repr, DecidableEq

/-- 发布候选的四个环境事实由调用者取证，模型不把它们当网络事实猜出来。 -/
structure Candidate where
  source : String
  ciGreen : Bool
  platformsReady : Bool
  extendsCurrent : Bool

inductive Decision where
  | skipSameSource
  | skipCi
  | skipStale
  | skipPlatforms
  | publish
  deriving Repr, DecidableEq

/-! D105 快照以完整 SHA 推进，失败不删除当前版本，公开后至多留两份

定时发布只在候选不同于当前快照、该 SHA 的 CI 已成功、它相对当前快照前进且三平台归档齐全时发生。
这些条件不满足是跳过，不是另造一个失败状态。旧版只有在新版公开之后才进入保留裁剪；
构建或公开失败不改变已公开历史。被否：按 25 小时猜 main 是否前进、强移固定 nightly tag、
任一平台成功就公开部分资产、先删旧版再构建，以及把 nightly 塞进正式 semver 比较。
重开参数：发布渠道改为每个平台独立推进，或需要保留两份以上供长期固定安装。 -/

/-- 发布判定；同 SHA 的重复运行首先短路，跨日也不制造另一份。 -/
def decide (current : Option Snapshot) (candidate : Candidate) : Decision :=
  if current.any (fun held => held.source == candidate.source) then
    .skipSameSource
  else if !candidate.ciGreen then
    .skipCi
  else if current.isSome && !candidate.extendsCurrent then
    .skipStale
  else if !candidate.platformsReady then
    .skipPlatforms
  else
    .publish

theorem publish_requires_ci (current : Option Snapshot) (candidate : Candidate)
    (h : decide current candidate = .publish) : candidate.ciGreen = true := by
  unfold decide at h
  split at h
  · cases h
  · split at h
    · simp_all
    · exact Bool.of_not_eq_false (by simp_all)

theorem same_source_is_never_published (current : Option Snapshot) (candidate : Candidate)
    (h : current.any (fun held => held.source == candidate.source) = true) :
    decide current candidate = .skipSameSource := by
  simp [decide, h]

theorem stale_candidate_is_never_published (current : Option Snapshot) (candidate : Candidate)
    (different : current.any (fun held => held.source == candidate.source) = false)
    (ci : candidate.ciGreen = true)
    (present : current.isSome = true)
    (stale : candidate.extendsCurrent = false) :
    decide current candidate = .skipStale := by
  simp [decide, different, ci, present, stale]

theorem platforms_must_be_ready (current : Option Snapshot) (candidate : Candidate)
    (h : decide current candidate = .publish) : candidate.platformsReady = true := by
  unfold decide at h
  split at h
  · cases h
  · split at h
    · simp_all
    · split at h
      · simp_all
      · exact Bool.of_not_eq_false (by simp_all)

/-- 公开历史按发布时间从新到旧；同一 source 只保留一项，公开后最多两项。 -/
def recordPublication (candidate : Snapshot) (history : List Snapshot) : List Snapshot :=
  (candidate :: history.filter (fun held => held.source != candidate.source)).take 2

/-- 发布事件之外还有构建失败；失败对公开历史恒等。 -/
inductive Event where
  | buildFailed
  | publishSucceeded (candidate : Snapshot)

structure ChannelState where
  published : List Snapshot
  deriving Repr, DecidableEq

def step (state : ChannelState) : Event → ChannelState
  | .buildFailed => state
  | .publishSucceeded candidate =>
    { published := recordPublication candidate state.published }

theorem failed_build_preserves_published (state : ChannelState) :
    step state .buildFailed = state := by
  rfl

theorem publication_retains_at_most_two (candidate : Snapshot) (history : List Snapshot) :
    (recordPublication candidate history).length ≤ 2 := by
  simp only [recordPublication, List.length_cons, List.length_take]
  exact Nat.min_le_left _ _

theorem new_publication_keeps_the_previous_snapshot (candidate previous : Snapshot)
    (rest : List Snapshot) (different : previous.source != candidate.source) :
    recordPublication candidate (previous :: rest) = [candidate, previous] := by
  simp [recordPublication, different]

example : decide (some { source := "old" })
    { source := "old", ciGreen := true, platformsReady := true, extendsCurrent := true } =
    .skipSameSource := by decide
example : decide (some { source := "old" })
    { source := "new", ciGreen := false, platformsReady := true, extendsCurrent := true } =
    .skipCi := by decide
example : decide (some { source := "old" })
    { source := "new", ciGreen := true, platformsReady := true, extendsCurrent := false } =
    .skipStale := by decide
example : decide (some { source := "old" })
    { source := "new", ciGreen := true, platformsReady := false, extendsCurrent := true } =
    .skipPlatforms := by decide
example : decide (some { source := "old" })
    { source := "new", ciGreen := true, platformsReady := true, extendsCurrent := true } =
    .publish := by decide
example : recordPublication { source := "new" } [{ source := "old" }, { source := "older" }] =
    [{ source := "new" }, { source := "old" }] := by decide

end Sprawling.Nightly

/-!
## 生产接口与一致性检查

```rust
pub enum Built { Released(Release), Nightly(NightlyBuild), FromSource }
pub struct NightlyBuild { /* 私有字段：tag 与完整 SHA */ }
impl NightlyBuild {
    pub fn new(tag: &str, source_sha: &str) -> Result<NightlyBuild, AxError>;
    pub fn tag(&self) -> &str;
    pub fn source_sha(&self) -> &str;
}
```

`SPRAWLING_NIGHTLY_TAG` 与 `SPRAWLING_SOURCE_SHA` 成对出现，并与正式
`SPRAWLING_RELEASE_TAG` 互斥；build script 声明三个环境输入的重跑条件。
tag 只接受上述形状，日期为六位数字、月日范围有效，tag 末尾的 40 位小写十六进制 SHA
与 Release API 的 `target_commitish` 逐字相等；字段缺失或不符均拒绝。发布创建时以完整 SHA
传给 `target`，因此 GitHub 的字段就是机器读数，不在 release notes 另造 source 标记。

正式 GitHub 查询跳过 nightly，nightly 查询按 `published_at` 选择最新有效快照：身份或
`published_at` 读不出的条目被跳过，一份坏条目不能让更新的好快照查不到；整张列表都没有
有效快照时才拒绝。
更新检查以最新快照为 base、正在运行的这份构建的 source 为 head 调用 GitHub compare：
`identical` 为 Current，`behind` 为 Behind，`ahead` 为 Ahead，`diverged` 为 Unconfirmed；
网络或解码失败为 Refused，不能猜版本顺序。base 与 head 的顺序由 verdict 的含义决定
（`Behind` 说的是已安装的这份），所以换序会把更新方向整条读反。
发布说明在比较范围之前以中英两种文字写出同一份固定警告：顶部一句标题句中英对照成对给出，两行之间
不空行；空一行后是正文，先英文一段，再中文一段，关键处加粗。警告说：这是自动发布的实验快照，
不稳定、未经充分测试，行为不受任何保障，不保证与任何其他版本兼容，协议与存储格式可能随时改变；
下载前先备份城市，下载后按发布页公布在每个归档上的 SHA256 摘要校验归档，回退二进制不保证旧版能
读新版迁移的数据；nightly 让每天合并的代码被构建、运行和测试，有时已含正式发行还没有的优化与
修复；欢迎下载 nightly 帮助测试可用性与兼容性，遇到问题开 issue 详尽描述执行了什么、期望什么、
实际发生了什么以及 tag 中的完整 SHA。警告还注明 tag 的日期差别：nightly 用 UTC 日期，正式发行 tag 用切版当天的 UTC+8 日期。
自动构建与 CI 通过不构成这类保证。
nightly 只经 Archive 更新，命令显式携带 `SPRAWLING_RELEASE_CHANNEL=nightly`。
`status` 显示 Cargo 基准版本与 nightly tag，不把它说成正式 release。

`.github/workflows/nightly-release.yml` 还遵守：canonical main 才允许自动公开；
手动其他 ref 与 fork 仅 build-only；CI pending/red 不重造红色发布 run；
同名 published 不重建，未公开 draft 可重建；发布失败后的重跑可只补 retention；
GitHub Release 的创建通知与每日一次节奏保留，不发 npm、crates.io 或 AUR。
归档只从身份命名的那个提交构建（checkout 的 `HEAD` 必须等于 `SOURCE_SHA`），
因为 tag 与 `target_commitish` 说的是源码而字节来自工作树。
公开前核对同名 tag 指向的提交：`gh release create --target` 不移动已存在的 tag，
指向别处时归档会挂到另一棵树上，所以要拒绝而不是改写。
-/
