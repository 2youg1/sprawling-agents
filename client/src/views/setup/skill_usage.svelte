<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Every skill of the city and what became of it: the shelves that hold
  // it with the audit of what each holds now, the contents runs have
  // read, and every use. The skills nobody has used are listed apart,
  // because "never read" is the question a User asks before clearing a
  // shelf. A copy whose content changed since its last audit asks for a
  // new one in the alert tone (`SkillAudit::Stale`, wire D33).
  import type { Key } from "../../core/lang";
  import type { HeldSkill, SkillAudit, SkillUse } from "../../wire";
  import type { UseRow } from "../parts/usage_uses.svelte";

  function rows(uses: readonly SkillUse[]): UseRow[] {
    return uses.map((one) => ({ run: one.run, resident: one.resident ?? null, at: one.at, part: one.part, outcome: one.outcome }));
  }

  export function auditWord(audit: SkillAudit): Key {
    switch (audit.state) {
      case "unaudited":
        return "usage_audit_none";
      case "stale":
        return "usage_reaudit";
      case "audited":
        switch (audit.verdict) {
          case "pass":
            return "usage_audit_pass";
          case "warn":
            return "usage_audit_warn";
          case "fail":
            return "usage_audit_fail";
          case "unreachable":
            return "usage_audit_none";
        }
    }
  }

  export function asksAgain(held: HeldSkill): boolean {
    return held.audit.state === "stale";
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import UsageExport from "../parts/usage_export.svelte";
  import UsageUses from "../parts/usage_uses.svelte";

  const u = ui();
  const lang = u.lang;
  const question: Query = { skill_usage: { skill: null } };
  const asked = u.conn.asking.ask(question);
  const read = $derived(readAnswer($asked, (held) => ("skill_usage" in held ? held.skill_usage : undefined)));
  const used = $derived(read.kind === "held" ? read.value.skills.filter((skill) => skill.uses.length > 0) : []);
  const unused = $derived(read.kind === "held" ? read.value.skills.filter((skill) => skill.uses.length === 0) : []);
</script>

<section class="flex min-w-0 flex-col gap-base">
  <div class="flex items-baseline justify-between gap-base">
    <h2 class="text-note text-text-faint">{say($lang, "usage_skills_title")}</h2>
    <UsageExport what="skills" />
  </div>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if read.kind === "asking"}
    <p class="text-text-faint">…</p>
  {:else}
    <ul class="flex flex-col">
      {#each [...used, ...unused] as skill (skill.name)}
        <li class="flex min-w-0 flex-col gap-tight border-b border-edge py-snug">
          <span class="font-mono text-note text-text-quiet">{skill.name}</span>
          {#each skill.held as held, index (index)}
            <span class={asksAgain(held) ? "text-note text-alert" : "text-note text-text-faint"}>
              {say($lang, auditWord(held.audit))}
            </span>
          {/each}
          {#if skill.held.length === 0}
            <span class="text-note text-text-faint">{say($lang, "usage_off_shelf")}</span>
          {/if}
          {#each skill.versions as version (version.digest)}
            <span class="truncate font-mono text-note text-text-faint">
              {fill(say($lang, "usage_version"), { digest: version.digest.slice(0, 12), when: clock($lang, version.at), run: version.run })}
            </span>
          {/each}
          <UsageUses
            uses={rows(skill.uses)}
            days={skill.per_day}
          />
        </li>
      {/each}
    </ul>
    {#if unused.length > 0}
      <p class="text-note text-text-faint">
        {say($lang, "usage_never_list")}: {unused.map((skill) => skill.name).join(" · ")}
      </p>
    {/if}
  {/if}
</section>
