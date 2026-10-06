<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { putPreferences } from "../../core/commands";
  import { say, type Key } from "../../core/lang";
  import { ui } from "../../ui";
  import { CorePlacement, CorePriority, type CorePreferences } from "../../wire";
  import Field from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";
  import Card from "./card.svelte";
  import { HELD, RECEIPT_MS, edited, refused, sent, waited, type Saving } from "./saving";

  const PLACEMENT: Record<CorePlacement, readonly [Key, Key]> = {
    none: ["performance_none", "performance_none_note"],
    soft: ["performance_soft", "performance_soft_note"],
    soft_shares: ["performance_soft_shares", "performance_soft_shares_note"],
    pinned: ["performance_pinned", "performance_pinned_note"],
  };
  const PRIORITY: Record<CorePriority, readonly [Key, Key]> = {
    raised: ["performance_raised", "performance_raised_note"],
    normal: ["performance_normal", "performance_normal_note"],
  };
  const u = ui();
  const { lang } = u;
  const asked = u.conn.asking.ask(QUERIES.preferences);
  const belief = u.conn.belief;
  const core = $derived.by(() => {
    const answer = $asked;
    return answer !== undefined && "preferences" in answer ? answer.preferences.core : undefined;
  });
  let placement = $state<CorePlacement>("soft");
  let priority = $state<CorePriority>("raised");
  let memory = $state("");
  let saving = $state.raw<Saving>(HELD);
  let awaiting = $state.raw<CorePreferences | null>(null);
  const bytes = $derived(memory.trim() === "" ? null : Number(memory));
  const valid = $derived(bytes === null || (/^[1-9][0-9]*$/.test(memory.trim()) && Number.isSafeInteger(bytes)));

  $effect(() => {
    if (core === undefined) return;
    if (awaiting !== null && core.placement === awaiting.placement && core.priority === awaiting.priority
        && (core.memory_bytes ?? null) === (awaiting.memory_bytes ?? null)) {
      saving = { kind: "saved" };
      awaiting = null;
    }
    if (saving.kind === "held" || saving.kind === "saved") {
      placement = core.placement ?? "soft";
      priority = core.priority ?? "raised";
      memory = core.memory_bytes === undefined || core.memory_bytes === null ? "" : String(core.memory_bytes);
    }
  });
  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    saving = refused(saving, error);
  });

  function moved(): void {
    saving = edited(core !== undefined && (placement !== core.placement || priority !== core.priority
      || bytes !== (core.memory_bytes ?? null) || !valid));
  }

  function save(): void {
    if (core === undefined || !valid) return;
    awaiting = { placement, priority, memory_bytes: bytes };
    if (!u.send(putPreferences({ core_placement: placement }))
        || !u.send(putPreferences({ core_priority: priority }))
        || !u.send(putPreferences({ run_memory: bytes }))) {
      awaiting = null;
      return;
    }
    const mine = sent(JSON.stringify(core));
    saving = mine;
    u.conn.asking.refresh(QUERIES.preferences);
    setTimeout(function recheck() {
      if (saving !== mine) return;
      u.conn.asking.refresh(QUERIES.preferences);
      setTimeout(recheck, 1_000);
    }, 1_000);
    setTimeout(() => {
      if (saving === mine) saving = waited(mine);
    }, RECEIPT_MS);
  }
</script>

{#if core !== undefined}
  <Card title="setup_group_performance" note="setup_group_hint_performance" {saving} settled="performance_restart" onSave={save}>
    <Segmented
      label={say($lang, "performance_placement")}
      held={placement}
      options={CorePlacement.literals.map((value) => ({ value, label: say($lang, PLACEMENT[value][0]) }))}
      onPick={(value) => { placement = value; moved(); }}
    />
    <p class="text-note text-text-quiet">{say($lang, PLACEMENT[placement][1])}</p>
    <Segmented
      label={say($lang, "performance_priority")}
      held={priority}
      options={CorePriority.literals.map((value) => ({ value, label: say($lang, PRIORITY[value][0]) }))}
      onPick={(value) => { priority = value; moved(); }}
    />
    <p class="text-note text-text-quiet">{say($lang, PRIORITY[priority][1])}</p>
    <Field label={say($lang, "performance_memory")} value={memory} help={say($lang, "performance_memory_note")}
      {...(valid ? {} : { error: say($lang, "performance_memory_invalid") })}
      onInput={(value) => { memory = value; moved(); }} mono />
  </Card>
{/if}
