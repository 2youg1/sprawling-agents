-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::release

规定 `crates/wire/src/answer/release.rs` 的帧，以及 `client/src/views/release/answer.svelte`
手动选择更新命令时的性质。帧形状由 Rust 类型、schema hash 和 wire 合约检查守住；
下面的选择模型证明候选限制在任意操作轨迹上保持，环境中的注册表 HTTP 与进程来源识别
由 `crates/sprawling/src/release.rs` 的回环回归和真实安装验收检查，不属于该证明。
-/

/-!
### 8-36 这座城的版本与原安装渠道：`Query::NewestRelease`

```rust
pub struct ReleaseLine { pub version: String, pub released: String }
pub enum ReleaseAnswer {
    Stands { mine: ReleaseLine, newest: ReleaseLine, registries: Vec<RegistryNewest>, verdict: ReleaseVerdict, update: UpdateHint },
    Unreleased { registries: Vec<RegistryNewest>, update: UpdateHint },
    Unconfirmed { mine: ReleaseLine, registries: Vec<RegistryNewest>, update: UpdateHint },
    Refused { refusal: AxError },
}
pub struct RegistryNewest { pub registry: Registry, pub reading: RegistryReading }
pub enum Registry { Npm, CratesIo, Github }
pub enum RegistryReading { Read { newest: ReleaseLine }, Refused { refusal: AxError } }
pub enum InstallChannel { Npm, Bun, Binstall, Unknown, Package, CargoOrBinstall, Cargo, Archive, Homebrew, Aur, Source }
pub struct UpdateHint { pub channel: InstallChannel, pub command: Option<String>, pub alternatives: Vec<String> }
```

人按下检查按钮或运行 `status --check` 才查询，没有定时、连接时查询或自动更新。
`Stands.newest` 是 Rust 判定实际使用的版本，终端与页面直接画它，不再另挑注册表行。
`registries` 另外显示每个来源的成功或拒绝；判定来源无法读取时答 `Refused`。
Cargo 的裸版本号由 `Version` 判，不与 npm 的预发布串直接比较；日期未知时为空，
未带发行 tag 的 Cargo 源码安装仍比较已编译版本，不制造发布日期。
开发构建答 `Unreleased`，保留实际已知安装来源与手动命令但不给 verdict；来源未知的发行二进制答 `Unconfirmed`，不猜一个 verdict。
更新命令只供人选择、复制和运行，查询与选择不下载或覆写二进制。归档更新命令将实际发布 tag 同时固定到脚本源码 ref 和 SPRAWLING_VERSION，不从 Release 的当前成熟度重造历史 tag。
-/

/-!
### 8-47f 查询按它回答的东西命名：`Query::NewestRelease`

查询线上拼作 `newest_release`，答复仍为 `Answer::Release`。
查询名与释放关停范围的 `Command::Release` 区分，保留一条查询拼法。
字段换形保持查询名时按 wire D1 升 `WIRE_V`，schema hash 与生成客户端同步。
-/

/-! D24 来源、比较版本与更新命令由服务端一处决定

npm/Bun/Package 读 npm latest；Cargo/Binstall/CargoOrBinstall 读 crates.io；Archive/Homebrew/Aur
读 GitHub 发布列表首个非 draft 的发布，与脚本安装器的列表来源一致，包含 prerelease。
GitHub 的 latest 排除 prerelease，不能用于本项目归档检查；没有可用发布时拒绝判定。
两种历史成熟度的 tag 通过 `Release::from_published_tag` 的同一 tag 解码器读取。
GitHub API、归档脚本与发行页面的 repository 身份取自根清单并经 Cargo 的
`CARGO_PKG_REPOSITORY` 进入 release 模块，不在 HTTP URL 中另写 owner/repo。

包入口只把明确的 npm/Bun 选择或 `.bun`、`_npx` 缓存来源交给二进制，不能把运行时
当安装器；通用包目录答 Package，需要在 npm/Bun 命令中选择。
Cargo bin 目录答 CargoOrBinstall，需要在 Cargo 编译与 binstall 下载中选择。
归档安装器复制二进制后写来源标记，卸载移除标记；标记名只由
`sprawling::release::ARCHIVE_ORIGIN_FILE` 定义，旁边仍有 skills 的解压归档同属 Archive。
明确的 `SPRAWLING_INSTALL_CHANNEL` 手动选择接受 npm/bun/cargo/binstall/archive/homebrew/aur。
Homebrew/Aur 的识别与安装布局由 sprawling Install D52 规定；GitHub 比较不保证外部包定义已同步。

`command` 只有来源明确时才给出；`alternatives` 给出不明确来源允许的精确命令。
页面选择必须是候选之一，清空或收到新答复后没有默认选择，Copy 只接收明确命令
或已确认的候选。被否：Cargo 路径自动选源码编译、Bun 运行时自动认作 Bun 安装器、
客户端另设版本选择与排序规则，这些都会让安装方式或显示版本漂离实际比较来源。
重开条件是新增安装器或注册表，此时扩展枚举、来源适配与相应生产验收。
-/

namespace Wire.ReleaseChoice

structure Selection where
  candidates : List String
  chosen : Option String

inductive Action where
  | choose (command : String)
  | refresh (candidates : List String)
  | clear

def valid (state : Selection) : Prop :=
  ∀ command, state.chosen = some command → command ∈ state.candidates

def step (state : Selection) : Action → Selection
  | .choose command => { state with chosen := if command ∈ state.candidates then some command else none }
  | .refresh candidates => ⟨candidates, none⟩
  | .clear => { state with chosen := none }

def trace (state : Selection) : List Action → Selection
  | [] => state
  | action :: rest => trace (step state action) rest

private theorem step_valid (state : Selection) (action : Action) : valid (step state action) := by
  cases action with
  | choose command =>
    simp only [step]
    split
    · rename_i member
      intro selected same
      cases same
      exact member
    · intro selected same
      cases same
  | refresh candidates =>
    intro selected same
    cases same
  | clear =>
    intro selected same
    cases same

/-- 任意选择、刷新与清空轨迹之后，可复制的命令仍来自当前服务端候选。 -/
theorem every_trace_keeps_the_current_candidates (state : Selection) (actions : List Action)
    (initial : valid state) : valid (trace state actions) := by
  induction actions generalizing state with
  | nil => exact initial
  | cons action rest ih => exact ih (step state action) (step_valid state action)

end Wire.ReleaseChoice
