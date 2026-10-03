-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::library::audit

规定 `library::audit`（`crates/city/src/library/audit.rs`）：书架上一件 skill 的审核状态怎么从账本读出来。本文件是 `crates/city/Spec.lean` 的一个分部，标签 §8-28b，决定引作 `city D19`。

模型放在 city 而不放在 accounting：「这件 skill 此刻的内容摘要是什么」是书架的事实（§8-8 的 `Holding::hash`、§8-28 的整包哈希），审核绑定的正是这个摘要；accounting 的折叠（wire D32）只读本模块给出的判定，不另写一份「过期」的规则。
-/

/-!
### 8-28b 技能审核：审核绑在内容摘要上，取不到审核不拦上架

```rust
// city::library::audit
pub enum AuditState { Unaudited, Audited { verdict: AuditVerdict, at: Seq }, Stale { audited: B3Hash } }
pub fn audit_state(content: &B3Hash, audits: &[(B3Hash, AuditVerdict, Seq)]) -> AuditState;
```

- **一件 skill 的审核状态是一个读法，不是存下的字段**：账本里有它的每一次 `skill_audited`（kernel D23，载荷带被审的那份内容的 `digest`），书架有它此刻的内容摘要；`audit_state` 取摘要与此刻相等的最新一条审核作答（`Audited`），一条都没有而曾审过别的摘要即 `Stale`，从没审过即 `Unaudited`。存一个「已审」的标志位就得在每一次改内容时记得清掉它，而改内容的人可以是在架外直接改文件的 User。
- **`Unreachable` 不是一条审核**：`skill_audited` 的 `verdict` 为 `Unreachable` 的行记下「这次没审成」与 skills.sh 上安全页的链接，`audit_state` 不把它算作对任何摘要的审核，于是取不到的那一次永远显示不出「已审」。
- **审核是建议，不是门**：上架只经 §8-28 的静态预检；审核在落位之后对落下的那份摘要进行，结论（含 `Fail`）显示在书架页与 skill 页上，准不准一件 skill 进阅览室仍是 User 在 `RULES.toml` 的 `reading_room` 里写下的（§8-8）。取不到 skills.sh、SkillSpector 不在，都不改变上架的结果（D89 第 3 条）。
- **三个平台相同**：判定只读摘要与账本行；取 skills.sh 是一次网络请求，SkillSpector 是 PATH 上的外部程序，两者在 Windows、macOS、Linux 上同一个调用。

模型是一件 skill 在一条事件轨迹上的演变。事件五种：通过预检的上架（带落下的摘要）、被预检拒收、内容在架上被改（带新摘要）、一次审核（带被审的摘要与结论）、一次取审核失败。下面证明，各在一条轨迹上量化：
1. 显示为「已审」时，轨迹里必有一次对**此刻这份内容摘要**的审核，结论就是显示的那一个（`audited_only_what_was_audited`）；于是内容改成一份从没被审过的字节之后，无论其后发生什么（只要内容不再变、也没有对它的审核），都不显示为已审（`changed_content_is_never_shown_audited`）。
2. 取审核失败不改变书架上的任何东西：从轨迹里删掉全部失败的取，书架状态逐字相同（`failed_fetches_change_nothing`）；于是一次通过预检的上架，不论之前失败过多少次取、审出过什么结论，都落下它的摘要（`a_failed_fetch_never_blocks_an_install`）。

**派生检查**（`library::audit::tests`）：一张轨迹向量表——上架、拒收、审、改、改回审过的字节、迟到的旧摘要审核、失败的取——逐行比对 `audit_state` 读出的状态与本模型 `shown` 在同一条轨迹上的答案；同一张表的每一行再各回放一遍上面的四条性质。表能咬住两种坏实现：「审核后只记布尔标志」（不看摘要）让向量表与两条已审性质变红，「把 `Unreachable` 当一条审核」让失败的取那条也变红。

- **现状：`audit_state` 还没有生产调用者**：它的调用者是 wire D32 的审核折叠与书架页上的审核标记，与 `InstallSkill` 的执行者同一次改动落地；在那之前它与 D4（没有生产调用者的公开面不留）不合，留着是因为折叠只读本模块的判定，不另写一份规则。
-/

/-! D19 审核状态是从账本与书架摘要读出的判定；审核由城在落位之后、对落下的那份摘要发起，不拦上架

**决定**：
- **何时审**：一件 skill 每一次落位（`InstallSkill`，wire D32；`PutShelved` 落地之后同一条路）之后，以及城在扫架（`Library::scan`，开城与每次落位后）时看见一件持有的摘要在 `audit_state` 下是 `Unaudited` 或 `Stale` 时，各发起一次；同一 `(skill, digest)` 在一个城进程里只发起一次，于是一份取不到的内容不会在每次扫架时再写一行。发起点在城的后台任务（唯一的 spawn 点），run 的工具波从不等审核：`describe` 读一件 `Stale` 的 skill 照常答。
- **审什么**：被审的是书架上落下的那份字节——包审整包规范串的摘要（§8-28），文档审正文的摘要；`skill_audited.digest` 就是 `audit_state` 比对的那个值，即 §8-28 的 `Installed::hash`，而不是 `Holding::hash`：包的 `Holding::hash` 只是 `SKILL.md` 的摘要，改了包里一份脚本它不变，而注入正可以藏在脚本里。扫架不为此读包里每个文件（§8-8）：比对在后台的审核任务里做，它用 §8-28 预检的同一个读（`precheck`，上限 `PACKAGE_BYTES_LIMIT`）算出架上此刻的整包摘要。审的不是来源：来源在落位之后可能已经变了。
- **谁来审，按序都试**：①skills.sh 的合作方审核（缺省，D89 第 3 条）：只在这件 skill 有一个 skills.sh 能认的来源时——来源是 skills.sh 名，或指向 `github.com/<owner>/<repo>` 的 git 地址——请求 `GET https://skills.sh/api/v1/skills/audit/<owner>/<repo>/<skill>`，不带令牌；每个合作方一行 `skill_audited`（`source = SkillsSh`，`scanner` 是合作方名，`verdict`／`risk`／`audited_at` 照录 `status`／`riskLevel`／`auditedAt`）；答 401、403、网络不通、超时（`SKILLS_SH_TIMEOUT`，5 s）或答复读不出时写一行 `verdict = Unreachable`，`link` 是 `https://skills.sh/<owner>/<repo>/<skill>` 这一页。②SkillSpector（可选）：doctor 在 PATH 上找到 `skillspector` 时，对落下的那份目录跑 `skillspector scan <path> --no-llm --format json`，退出码 0 读作 `Pass`、1 读作 `Fail`（它承诺稳定的只有这两个码与 JSON），其余退出码与读不出的输出读作 `Unreachable`；`scanner` 是它 `--version` 报出的串，`risk` 取 JSON 里的总风险等级原词。两者都不适用（本地路径或自带的 skill，且没有 SkillSpector）时不写行，状态就是 `Unaudited`：没人审过就不该有一行说审过。
- **结论只显示，不判准入**：`Fail` 在书架页与 skill 页上标红并给出链接；进不进阅览室仍由 User 写在 `RULES.toml` 的 `reading_room` 里（§8-8）。

**理由**：「已审」要和它审的那份字节绑在一起，才能在内容变了之后自己失效，所以状态从摘要读出，不存标志位（模型的 `changed_content_is_never_shown_audited`）。skills.sh 的无令牌接口今天对 audit 路径答 200、对同站的详情路径答 401，它随时可能关；把上架挂在它上面，书架会在某一天整个不能用，所以取不到只记一行 `Unreachable` 与安全页链接（`a_failed_fetch_never_blocks_an_install`）。审核在落位之后而不在之前，是因为被审的必须是书架上那份字节，而 §8-28 的 TOCTOU 复查保证落下的就是规划时读到的那份。

**被否**：①落位之前先审、`Fail` 即拒收：审核方取不到时只能二选一——拦住（书架不可用）或放行（审核形同虚设），且审的是来源而不是落下的字节；②在 `Holding` 上存 `audited: bool`：User 在架外改文件时没人清它，正是模型排除的那个状态；③每次扫架都重试取不到的审核：每次开城给同一份内容写一行 `Unreachable`。

**重开参数**：skills.sh 给出带令牌的稳定接口、或 User 要求按 `Fail` 拒收时，重议「只显示」；`SKILLS_SH_TIMEOUT` 是推断值，一个常量。

**三个平台**：skills.sh 是同一次 HTTPS 请求；SkillSpector 要 Python ≥ 3.12，doctor 在 Windows（`skillspector.exe` 或 `.cmd` 垫片，经 `PATHEXT`）、macOS 与 Linux 上都只按 PATH 找，不打包、不安装。
-/

namespace City.Library.Audit

/-- 一份内容的摘要（`B3Hash`）。模型只需要它能判等。 -/
abbrev Digest := Nat

/-- 一次审成的结论（`AuditVerdict` 去掉 `Unreachable` 的三臂）。 -/
inductive Verdict where
  | pass
  | warn
  | fail
  deriving DecidableEq, Repr

/-- 一件 skill 的轨迹上的一步。 -/
inductive Event where
  /-- 通过 §8-28 的预检、落下摘要为 `d` 的内容。 -/
  | install (d : Digest)
  /-- 预检拒收：书架一个字节不动。 -/
  | refused
  /-- 内容在架上被改成摘要为 `d` 的字节（经 `PutShelved`，或 User 在架外直接改文件）。 -/
  | edit (d : Digest)
  /-- 一行 `skill_audited`，对摘要 `d` 审出结论 `v`。 -/
  | audit (d : Digest) (v : Verdict)
  /-- 一行 `verdict = Unreachable` 的 `skill_audited`：skills.sh 取不到、SkillSpector 不在。 -/
  | fetchFailed (d : Digest)
  deriving DecidableEq, Repr

/-- 书架上的一件 skill 与它的审核历史（新的在前）。 -/
structure Shelved where
  content : Option Digest
  audits : List (Digest × Verdict)
  deriving DecidableEq, Repr

/-- 还没有这件 skill、也没有任何审核。 -/
def empty : Shelved := ⟨none, []⟩

def step (s : Shelved) : Event → Shelved
  | .install d => { s with content := some d }
  | .refused => s
  | .edit d => { s with content := some d }
  | .audit d v => { s with audits := (d, v) :: s.audits }
  | .fetchFailed _ => s

def run (s : Shelved) (trace : List Event) : Shelved := trace.foldl step s

/-- 对摘要 `c` 的最新一次审核的结论。 -/
def latestFor (audits : List (Digest × Verdict)) (c : Digest) : Option Verdict :=
  (audits.find? (fun p => p.1 == c)).map (·.2)

/-- 页面看到的审核状态（`AuditState`，加上「不在架上」）。 -/
inductive Shown where
  | absent
  | unaudited
  | stale
  | audited (v : Verdict)
  deriving DecidableEq, Repr

def shown (s : Shelved) : Shown :=
  match s.content with
  | none => .absent
  | some c =>
    match latestFor s.audits c with
    | some v => .audited v
    | none => if s.audits.isEmpty then .unaudited else .stale

theorem run_cons (s : Shelved) (e : Event) (es : List Event) :
    run s (e :: es) = run (step s e) es := rfl

theorem run_append (s : Shelved) (a b : List Event) :
    run s (a ++ b) = run (run s a) b := by
  simp [run, List.foldl_append]

/-- 结束时的每一条审核，要么开始时就有，要么是轨迹里的一次审核。 -/
theorem audits_come_from_the_trace (s : Shelved) (trace : List Event) :
    ∀ p ∈ (run s trace).audits, p ∈ s.audits ∨ Event.audit p.1 p.2 ∈ trace := by
  induction trace generalizing s with
  | nil => intro p hp; exact Or.inl hp
  | cons e es ih =>
    intro p hp
    rw [run_cons] at hp
    rcases ih (step s e) p hp with h | h
    · cases e with
      | audit d v =>
        simp only [step, List.mem_cons] at h
        rcases h with h | h
        · subst h; exact Or.inr (by simp)
        · exact Or.inl h
      | install d => exact Or.inl h
      | refused => exact Or.inl h
      | edit d => exact Or.inl h
      | fetchFailed d => exact Or.inl h
    · exact Or.inr (List.mem_cons_of_mem _ h)

theorem latest_is_a_member (audits : List (Digest × Verdict)) (c : Digest) (v : Verdict)
    (h : latestFor audits c = some v) : (c, v) ∈ audits := by
  unfold latestFor at h
  cases hf : audits.find? (fun p => p.1 == c) with
  | none => simp [hf] at h
  | some p =>
    simp [hf] at h
    have hm := List.mem_of_find?_eq_some hf
    have hc := List.find?_some hf
    simp at hc
    have : p = (c, v) := by
      cases p with
      | mk a b => simp_all
    rw [← this]; exact hm

/-- 显示为「已审」时，轨迹里有一次对此刻这份内容摘要的、结论相同的审核。 -/
theorem audited_only_what_was_audited (trace : List Event) (v : Verdict)
    (h : shown (run empty trace) = .audited v) :
    ∃ c, (run empty trace).content = some c ∧ Event.audit c v ∈ trace := by
  unfold shown at h
  cases hc : (run empty trace).content with
  | none => simp [hc] at h
  | some c =>
    rw [hc] at h
    cases hl : latestFor (run empty trace).audits c with
    | none =>
      simp only [hl] at h
      split at h <;> simp at h
    | some w =>
      simp only [hl, Shown.audited.injEq] at h
      subst h
      refine ⟨c, rfl, ?_⟩
      rcases audits_come_from_the_trace empty trace (c, w) (latest_is_a_member _ c w hl) with
        hm | hm
      · simp [empty] at hm
      · exact hm

/-- 一步不改内容的事件。 -/
def KeepsContent : Event → Prop
  | .install _ => False
  | .edit _ => False
  | _ => True

theorem content_kept (s : Shelved) (trace : List Event) (h : ∀ e ∈ trace, KeepsContent e) :
    (run s trace).content = s.content := by
  induction trace generalizing s with
  | nil => rfl
  | cons e es ih =>
    rw [run_cons, ih (step s e) (fun x hx => h x (List.mem_cons_of_mem _ hx))]
    have he := h e (by simp)
    cases e <;> simp_all [KeepsContent, step]

/-- 内容改成一份从没被审过的字节之后，只要内容不再变，就不显示为已审。 -/
theorem changed_content_is_never_shown_audited (before after : List Event) (d : Digest)
    (v : Verdict) (hkeep : ∀ e ∈ after, KeepsContent e)
    (hnever : ∀ w, Event.audit d w ∉ before ++ Event.edit d :: after) :
    shown (run empty (before ++ Event.edit d :: after)) ≠ .audited v := by
  intro h
  obtain ⟨c, hc, hm⟩ := audited_only_what_was_audited _ v h
  have hd : (run empty (before ++ Event.edit d :: after)).content = some d := by
    rw [run_append, run_cons, content_kept _ after hkeep]
    rfl
  rw [hd] at hc
  cases hc
  exact hnever v hm

def IsFetchFailed : Event → Bool
  | .fetchFailed _ => true
  | _ => false

/-- 删掉轨迹里全部失败的取，书架状态逐字相同。 -/
theorem failed_fetches_change_nothing (s : Shelved) (trace : List Event) :
    run s (trace.filter (fun e => !IsFetchFailed e)) = run s trace := by
  induction trace generalizing s with
  | nil => rfl
  | cons e es ih =>
    cases e <;> exact ih _

/-- 一次通过预检的上架，不论之前发生过什么，都落下它的摘要。 -/
theorem a_failed_fetch_never_blocks_an_install (s : Shelved) (before : List Event)
    (d : Digest) : (run s (before ++ [Event.install d])).content = some d := by
  rw [run_append]; rfl

end City.Library.Audit
