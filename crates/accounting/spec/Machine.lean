-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::machine

规定 `crates/accounting/src/machine.rs`：端口 `Machine`，以及唯一造得出 `Runnable` 的 `Recipe::command`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-4 accounting::machine（形状 3 端口）

```rust
pub trait Machine {
    /// Asks this machine every question the requirement table holds.
    fn report(&self) -> wire::DoctorAnswer;
    /// # Errors
    /// A program this machine cannot start, one that ended in failure,
    /// and one still running when its patience ran out.
    fn install(&self, item: &str, runnable: &Runnable<'_>) -> Result<(), AxError>;
}

pub enum Recipe {
    Command { program: &'static str, args: &'static [&'static str] },
    Print(&'static str),
    Manual(&'static str),
}
impl Recipe {
    pub fn spelled(&self) -> String;
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` for a printed recipe and a manual one, with
    /// what the person does instead.
    pub fn command(&self, item: &str) -> Result<Runnable<'_>, AxError>;
}

pub struct Runnable<'a> { /* private */ }
impl<'a> Runnable<'a> {
    pub fn program(&self) -> &'a str;
    pub fn args(&self) -> &'a [&'a str];
    pub fn spelled(&self) -> String;
}
```

```rust
// bin::doctor::probe（形状 4 适配器）
impl accounting::Machine for ThisMachine { /* doctor::answer(self) 与 doctor::running::run */ }

// accounting::worker::commanding::machine
impl RunWorker {
    pub fn with_machine(self, machine: Box<dyn accounting::Machine + Send>) -> RunWorker;
}
```

- **只有 `Recipe::command` 造得出 `Runnable`，它证明的是配方的种类，不是许可。** `Runnable` 的构造函数在本 crate 之外不可见，所以持有一个 `Runnable` 只证明它来自一个 `Command` 配方：打印的配方与手动的配方在 `Recipe::command` 被拒。`Recipe::Command` 的字段是 `pub`，任何 crate 都能拼出一个装任意程序的配方，所以挡住表外程序的是 `doctor_install`（`crates/accounting/src/worker/commanding/machine.rs`）先在 requirement 表里查这个名字：表里没有的名字以 `InvalidArgs` 被拒，端口根本不会被调用。
- **失败**：`install` 原样传 `bin::doctor::running` 的 `AxError`；`Recipe::command` 的拒绝是 `E_TOOL_UNAVAILABLE`，恢复说明人该做什么。端口不另造错误码。
- **worker 读的两处都经 `RunWorker.machine`**：`doctor_install` 的安装与它之后的重看，以及 `DoctorRefresh` 的 `look_at_this_machine`。需求表里没有的名字与没有配方的平台由 worker 交到的 `recipe_for`（生产是 `bin::doctor::recipe_for`）拒绝，这一步在端口被问到之前。
- **一扇安装的门**：终端的 `sprawling doctor --install` 与 worker 的 `doctor_install` 都经 `accounting::Machine::install` 启动安装程序。`bin::doctor::Machine` 是它的子 trait，只多一个逐项的 `look`，自己不声明 `install`，所以一个装东西的实现只有一处要写，也只有一处能被脚本换掉。
- **固定值**：生产的 `Hands`（`bin::assembly::production::hands`）装上 `ThisMachine`，经 `Hands.machine` 交给构造器；构造之后 `with_machine` 是唯一换掉它的门。
- **依赖**：`report` 交回线上的 `wire::DoctorAnswer`，所以本 crate 依赖 `wire`（ARCHITECTURE.md §3 的 `depmap`）。
-/

/-! ## 模型：只有命令配方造得出 `Runnable`，只有表里的名字问得到端口

`Recipe` 与 `Recipe.command` 照 Rust 的三臂写；`Runnable` 在 Rust 里构造函数私有，这里由 `command` 唯一产出。`doctorInstall` 是 `accounting::worker::commanding::machine` 里 `doctor_install` 的前两步：先在需求表里查这个名字（`Hands.recipe_for`，这里是参数 `recipeFor`），再要一个 `Runnable`，两步都过才问端口。拒绝码是 Rust 的 `AxCode` 拼写。本模型不是 Rust 实现的证明，对应由 `crates/sprawling/tests/machine.rs` 与 `accounting::worker::commanding` 的测试检查（§16）。
-/

namespace Accounting.Machine

/-- 一条安装配方的三种。 -/
inductive Recipe where
  | Command (program : String) (args : List String)
  | Print (text : String)
  | Manual (text : String)
  deriving Repr, DecidableEq

/-- 一条能跑的命令；只有 `Recipe.command` 造它。 -/
structure Runnable where
  program : String
  args : List String
  deriving Repr, DecidableEq

/-- `Recipe::command`：打印的与手动的配方以 `E_TOOL_UNAVAILABLE` 拒绝。 -/
def Recipe.command : Recipe → Except String Runnable
  | .Command p a => .ok ⟨p, a⟩
  | .Print _ => .error "E_TOOL_UNAVAILABLE"
  | .Manual _ => .error "E_TOOL_UNAVAILABLE"

/-- 持有一个 `Runnable` 只证明它来自一个命令配方，程序与参数原样。 -/
theorem a_runnable_comes_from_a_command (r : Recipe) (x : Runnable) (h : r.command = .ok x) :
    r = .Command x.program x.args := by
  cases r with
  | Command p a => simp [Recipe.command] at h; subst h; rfl
  | Print _ => simp [Recipe.command] at h
  | Manual _ => simp [Recipe.command] at h

/-- `doctor_install` 的结局：拒绝（带码），或带着那条命令去问端口。 -/
inductive Install where
  | Refused (code : String)
  | Asked (runnable : Runnable)
  deriving Repr, DecidableEq

def doctorInstall (recipeFor : String → Option Recipe) (item : String) : Install :=
  match recipeFor item with
  | none => .Refused "E_INVALID_ARGS"
  | some r =>
    match r.command with
    | .ok x => .Asked x
    | .error c => .Refused c

/-- 端口只被问到表里有、且配方是命令的名字：`Recipe::Command` 的字段公开，挡住表外程序的是这一次查表。 -/
theorem the_port_is_asked_only_for_a_listed_command (recipeFor : String → Option Recipe)
    (item : String) (x : Runnable) (h : doctorInstall recipeFor item = .Asked x) :
    recipeFor item = some (.Command x.program x.args) := by
  unfold doctorInstall at h
  cases hr : recipeFor item with
  | none => simp [hr] at h
  | some r =>
    simp only [hr] at h
    cases hc : r.command with
    | error c => simp [hc] at h
    | ok y =>
      simp [hc] at h; subst h
      rw [a_runnable_comes_from_a_command r y hc]

/-- 正常路径可实现：表里一条命令配方问得到端口；表外的名字与打印的配方都在端口之前被拒。 -/
theorem a_listed_command_is_asked_and_the_rest_refused :
    let table : String → Option Recipe := fun i =>
      if i = "git" then some (.Command "winget" ["install", "Git.Git"])
      else if i = "zig" then some (.Print "see ziglang.org") else none
    doctorInstall table "git" = .Asked ⟨"winget", ["install", "Git.Git"]⟩ ∧
      doctorInstall table "zig" = .Refused "E_TOOL_UNAVAILABLE" ∧
      doctorInstall table "anything" = .Refused "E_INVALID_ARGS" := by
  decide

end Accounting.Machine

/-! D6 `Machine` 回答整页，而不是逐项回答 `look(&Requirement) -> Presence`

理由：worker 要的是一页答案与一次安装；逐项的端口要把 `Requirement`、`Detection`、`Family`、`PerPlatform`、`Platform` 与 `Presence` 整个搬进本 crate，而且沙箱与凭据保管这两项机器级的读仍然绕过端口直接碰主机，脚本也就换不掉它们。被否决的做法：逐项端口——搬走 doctor 的整个模型，却仍留两条通向主机的路。`bin::doctor::Machine` 是本 trait 的子 trait，给 doctor 自己逐项判定时加一个 `look`，它的第二实现在 doctor 的测试里；它不另设 `install`，安装只有本 trait 这一扇门。
-/

/-! D7 `install` 收 `Runnable`，不收程序名加参数；`Recipe` 与 `Runnable` 因此一起住在本 crate

理由：`Runnable` 证明配方是 `Command`，只有与它同住一个 crate 的 `Recipe::command` 能造它，所以一个 `Machine` 实现不会被递到一条打印的或手动的配方。哪些程序可以跑由 `doctor_install` 对 requirement 表的查找决定，不由这个类型决定。被否决的做法：收 `&str` 与 `&[&str]`——拒绝打印配方的规则就只剩每个调用方的自觉。让 `Recipe` 的字段私有、只让表能构造，可以把许可也放进类型，但表住在 `sprawling`、类型住在本 crate，没有一种 crate 布局能便宜地做到。
-/
